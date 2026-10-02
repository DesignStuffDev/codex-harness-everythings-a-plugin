//! Launch MCP stdio servers and return the transport rmcp should use.
//!
//! This module owns the "where does the server process run?" decision:
//!
//! - [`LocalStdioServerLauncher`] starts the configured command as a child of
//!   the orchestrator process.
//! - [`ExecutorStdioServerLauncher`] starts the configured command through the
//!   executor process API.
//!
//! Both paths return [`StdioServerTransport`], so `RmcpClient` can hand the
//! resulting byte stream to rmcp without knowing where the process lives. The
//! executor-specific byte adaptation lives in `executor_process_transport`.
//! Unix local servers use the stdio-only descriptor policy.

use std::collections::HashMap;
use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

use anyhow::Result;
use anyhow::anyhow;
use codex_async_utils::RetainedTask;
use codex_config::types::McpServerEnvVar;
use codex_exec_server::ExecBackend;
use codex_exec_server::ExecEnvPolicy;
use codex_exec_server::ExecParams;
use codex_exec_server::ExecProcess;
use codex_protocol::config_types::ShellEnvironmentPolicyInherit;
use codex_utils_path_uri::LegacyAppPathString;
use codex_utils_path_uri::PathUri;
use codex_utils_pty::Command;
#[cfg(unix)]
use codex_utils_pty::DescriptorPolicy;
use codex_utils_pty::ProcessMode;
use futures::FutureExt;
use futures::future::BoxFuture;
use rmcp::service::RoleClient;
use rmcp::service::RxJsonRpcMessage;
use rmcp::service::TxJsonRpcMessage;
use rmcp::transport::Transport;
#[cfg(windows)]
use tracing::warn;

use crate::executor_process_transport::ExecutorProcessTransport;
use crate::local_process_owner::LocalProcessOwner;
#[cfg(windows)]
use crate::local_process_owner::LocalProcessTerminator;
use crate::local_stdio_transport::LocalStdioTransport;
use crate::program_resolver;
use crate::protocol_mode::McpProtocolMode;
use crate::utils::create_env_for_mcp_server;
use crate::utils::create_env_overlay_for_remote_mcp_server;
use crate::utils::remote_mcp_env_var_names;

// General purpose public code.

/// Launches an MCP stdio server and returns the transport for rmcp.
///
/// This trait is the boundary between MCP lifecycle code and process placement.
/// `RmcpClient` owns MCP operations such as `initialize` and `tools/list`; the
/// launcher owns starting the configured command and producing an rmcp
/// [`Transport`] over the server's stdin/stdout bytes.
pub trait StdioServerLauncher: private::Sealed + Send + Sync {
    /// Start the configured stdio server and return its rmcp-facing transport.
    fn launch(
        &self,
        command: StdioServerCommand,
    ) -> BoxFuture<'static, io::Result<StdioServerTransport>>;
}

/// Command-line process shape shared by stdio server launchers.
#[derive(Clone)]
pub struct StdioServerCommand {
    program: OsString,
    args: Vec<OsString>,
    env: Option<HashMap<OsString, OsString>>,
    env_vars: Vec<McpServerEnvVar>,
    cwd: Option<String>,
    protocol_mode: McpProtocolMode,
    local_process_observer: Option<Arc<dyn Fn(LocalProcessOwner) + Send + Sync>>,
}

/// Client-side rmcp transport for a launched MCP stdio server.
///
/// The concrete process placement stays private to this module. `RmcpClient`
/// only sees the standard rmcp transport abstraction and can pass this value
/// directly to `rmcp::service::serve_client`.
pub struct StdioServerTransport {
    inner: StdioServerTransportInner,
    process: StdioServerProcessHandle,
}

enum StdioServerTransportInner {
    Local(LocalStdioTransport),
    Executor(ExecutorProcessTransport),
}

impl Transport<RoleClient> for StdioServerTransport {
    type Error = io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = std::result::Result<(), Self::Error>> + Send + 'static {
        // Both variants already implement rmcp's transport contract. This
        // wrapper keeps process placement private while leaving rmcp's send
        // semantics unchanged.
        match &mut self.inner {
            StdioServerTransportInner::Local(transport) => transport.send(item).boxed(),
            StdioServerTransportInner::Executor(transport) => transport.send(item).boxed(),
        }
    }

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<RoleClient>>> + Send {
        // rmcp reads from the same transport shape for both placements. The
        // executor variant turns pushed process-output events back into the
        // line-delimited JSON stream expected by rmcp.
        match &mut self.inner {
            StdioServerTransportInner::Local(transport) => transport.receive().boxed(),
            StdioServerTransportInner::Executor(transport) => transport.receive().boxed(),
        }
    }

    async fn close(&mut self) -> std::result::Result<(), Self::Error> {
        self.process.begin_shutdown();
        let transport = match &mut self.inner {
            StdioServerTransportInner::Local(transport) => transport.close().await,
            StdioServerTransportInner::Executor(transport) => transport.close().await,
        };
        let process = self.process.terminate().await;
        match (transport, process) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(transport), Err(process)) => Err(io::Error::other(StdioCloseErrors {
                transport,
                process,
            })),
        }
    }
}

impl StdioServerTransport {
    pub(crate) fn process_handle(&self) -> StdioServerProcessHandle {
        self.process.clone()
    }
}

impl StdioServerCommand {
    /// Build the stdio process parameters before choosing where the process
    /// runs.
    pub(super) fn new(
        program: OsString,
        args: Vec<OsString>,
        env: Option<HashMap<OsString, OsString>>,
        env_vars: Vec<McpServerEnvVar>,
        cwd: Option<String>,
        protocol_mode: McpProtocolMode,
    ) -> Self {
        Self {
            program,
            args,
            env,
            env_vars,
            cwd,
            protocol_mode,
            local_process_observer: None,
        }
    }

    pub(super) fn with_local_process_observer(
        mut self,
        observer: Arc<dyn Fn(LocalProcessOwner) + Send + Sync>,
    ) -> Self {
        self.local_process_observer = Some(observer);
        self
    }
}

// Local public implementation.

/// Starts MCP stdio servers as local child processes.
///
/// This is the existing behavior for local MCP servers: the orchestrator
/// process spawns the configured command and rmcp talks to the child's local
/// stdin/stdout pipes directly.
#[derive(Clone)]
pub struct LocalStdioServerLauncher {
    fallback_cwd: PathBuf,
}

impl LocalStdioServerLauncher {
    /// Creates a local stdio launcher.
    ///
    /// `fallback_cwd` is used when the MCP server config omits `cwd`, so
    /// relative commands resolve from the caller's runtime working directory.
    pub fn new(fallback_cwd: PathBuf) -> Self {
        Self { fallback_cwd }
    }
}

impl StdioServerLauncher for LocalStdioServerLauncher {
    fn launch(
        &self,
        command: StdioServerCommand,
    ) -> BoxFuture<'static, io::Result<StdioServerTransport>> {
        let fallback_cwd = self.fallback_cwd.clone();
        async move {
            // Keep synchronous program resolution and process creation from blocking the
            // caller's startup deadline.
            tokio::task::spawn_blocking(move || Self::launch_server(command, fallback_cwd))
                .await
                .map_err(io::Error::other)?
        }
        .boxed()
    }
}

// Local private implementation.

#[derive(Clone)]
pub(crate) struct StdioServerProcessHandle {
    inner: Arc<StdioServerProcessHandleInner>,
}

struct StdioServerProcessHandleInner {
    kind: StdioServerProcessKind,
    executor_shutdown: OnceLock<Option<Arc<RetainedTask<io::Result<()>>>>>,
}

enum StdioServerProcessKind {
    Local(LocalProcessOwner),
    Executor(Arc<dyn ExecProcess>),
}

#[derive(Debug)]
struct StdioCloseErrors {
    transport: io::Error,
    process: io::Error,
}

impl std::fmt::Display for StdioCloseErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("MCP transport close and process termination both failed")
    }
}

impl std::error::Error for StdioCloseErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Both originals remain owned; the public message contains no diagnostics.
        let _ = &self.process;
        Some(&self.transport)
    }
}

mod private {
    pub trait Sealed {}
}

impl private::Sealed for LocalStdioServerLauncher {}

impl LocalStdioServerLauncher {
    fn launch_server(
        command: StdioServerCommand,
        fallback_cwd: PathBuf,
    ) -> io::Result<StdioServerTransport> {
        let StdioServerCommand {
            program,
            args,
            env,
            env_vars,
            cwd,
            protocol_mode,
            local_process_observer,
        } = command;
        let program_name = program.to_string_lossy().into_owned();
        let envs = create_env_for_mcp_server(env, &env_vars).map_err(io::Error::other)?;
        let cwd = cwd.map(PathBuf::from).unwrap_or(fallback_cwd);
        let resolved_program =
            program_resolver::resolve(program, &envs, &cwd).map_err(io::Error::other)?;

        let build_command = || {
            let mut command = Command::new(&resolved_program);
            command.current_dir(&cwd).envs(&envs).args(&args);
            command.process_mode(ProcessMode::NewGroup);
            // MCP uses only stdio; select Explicit to exclude unrelated
            // orchestrator descriptors from the server and commands it launches.
            // Descriptor allowlisting is Unix-only. Windows can still inherit unrelated
            // handles and needs a handle allowlist in the shared spawn backend.
            #[cfg(unix)]
            command.descriptor_policy(DescriptorPolicy::Explicit);
            command
        };
        #[cfg(windows)]
        let mut command = build_command();
        #[cfg(not(windows))]
        let command = build_command();
        #[cfg(windows)]
        let job = match codex_utils_pty::JobObject::create_without_breakaway() {
            Ok(job) => {
                command.prepare_suspended_spawn(&job);
                Some(job)
            }
            Err(error) => {
                warn!("Windows MCP process job containment unavailable: {error}");
                None
            }
        };

        let spawn_transport = |command: Command| {
            LocalStdioTransport::spawn(
                command, program_name.clone(), protocol_mode, local_process_observer.clone(),
            )
        };
        let transport = spawn_transport(command)?;
        let process_id = transport.id();
        #[cfg(windows)]
        let (transport, process_id, job) = match job {
            Some(job) => match process_id
                .ok_or_else(|| io::Error::other("missing suspended MCP server process id"))
                .and_then(|process_id| job.assign_and_resume_process(process_id))
            {
                Ok(true) => (transport, process_id, Some(job)),
                Ok(false) => (transport, process_id, None),
                Err(error) => {
                    warn!(
                        "Windows MCP process job containment failed; retrying without it: {error}"
                    );
                    // This function runs on spawn_blocking. Join the rejected
                    // generation before publishing a replacement, so a successful
                    // fallback cannot hide its child or an earlier cleanup failure.
                    transport.process_owner().set_terminator(Some(LocalProcessTerminator::Job(job)));
                    let mut transport = transport;
                    tokio::runtime::Handle::current().block_on(transport.close())?;
                    let transport = spawn_transport(build_command())?;
                    let process_id = transport.id();
                    (transport, process_id, None)
                }
            },
            None => (transport, process_id, None),
        };
        #[cfg(windows)]
        let terminator = match job {
            Some(job) => Some(LocalProcessTerminator::Job(job)),
            None => process_id.and_then(|process_id| {
                match codex_utils_pty::JobObject::open_process_handle(process_id) {
                    Ok(handle) => Some(LocalProcessTerminator::Process(handle)),
                    Err(error) => {
                        warn!("Windows MCP process handle unavailable: {error}");
                        None
                    }
                }
            }),
        };
        let owner = transport.process_owner();
        #[cfg(windows)]
        owner.set_terminator(terminator);
        #[cfg(not(windows))]
        let _ = process_id;
        let process = StdioServerProcessHandle::local(owner);
        Ok(StdioServerTransport {
            inner: StdioServerTransportInner::Local(transport),
            process,
        })
    }
}

impl StdioServerProcessHandle {
    fn local(process: LocalProcessOwner) -> Self {
        Self {
            inner: Arc::new(StdioServerProcessHandleInner {
                kind: StdioServerProcessKind::Local(process),
                executor_shutdown: OnceLock::new(),
            }),
        }
    }

    pub(crate) fn executor(_program_name: String, process: Arc<dyn ExecProcess>) -> Self {
        Self {
            inner: Arc::new(StdioServerProcessHandleInner {
                kind: StdioServerProcessKind::Executor(process),
                executor_shutdown: OnceLock::new(),
            }),
        }
    }

    pub(crate) fn begin_shutdown(&self) {
        self.inner.begin_shutdown();
    }

    pub(crate) fn is_local(&self) -> bool {
        matches!(&self.inner.kind, StdioServerProcessKind::Local(_))
    }

    /// Observe the retained termination operation. Remote acknowledgment alone
    /// does not prove transport closure; callers needing that use wait_closed.
    pub(crate) async fn terminate(&self) -> io::Result<()> {
        self.begin_shutdown();
        match &self.inner.kind {
            StdioServerProcessKind::Local(process) => process.terminate().await,
            StdioServerProcessKind::Executor(_) => {
                let task = self.inner.executor_shutdown.get().and_then(Option::as_ref)
                    .ok_or_else(|| io::Error::other("MCP executor cleanup runtime unavailable"))?;
                match task.wait().await {
                    Ok(result) if result.is_ok() => Ok(()),
                    Ok(_) => Err(io::Error::other("MCP executor termination failed")),
                    Err(_) => Err(io::Error::other("MCP executor termination task failed")),
                }
            }
        }
    }

    pub(crate) async fn wait_closed(&self) -> io::Result<()> {
        match &self.inner.kind {
            StdioServerProcessKind::Local(process) => process.wait_closed().await,
            StdioServerProcessKind::Executor(_) => {
                self.terminate().await?;
                Err(io::Error::other("MCP executor process closure remains unconfirmed"))
            }
        }
    }
}

impl StdioServerProcessHandleInner {
    fn begin_shutdown(&self) {
        match &self.kind {
            StdioServerProcessKind::Local(process) => process.begin_shutdown(),
            StdioServerProcessKind::Executor(process) => {
                self.executor_shutdown.get_or_init(|| {
                    let runtime = tokio::runtime::Handle::try_current().ok()?;
                    let process = Arc::clone(process);
                    Some(RetainedTask::spawn(&runtime, async move {
                        process.terminate().await.map_err(io::Error::other)
                    }))
                });
            }
        }
    }
}

impl Drop for StdioServerProcessHandleInner {
    fn drop(&mut self) {
        self.begin_shutdown();
    }
}

// Remote public implementation.

/// Starts MCP stdio servers through the executor process API.
///
/// MCP framing still runs in the orchestrator. The executor only owns the
/// child process and transports raw stdin/stdout/stderr bytes, so it does not
/// need to know about MCP methods such as `initialize` or `tools/list`.
///
/// Windows executor-backed servers retain the executor's normal descendant
/// lifetime. MCP-specific containment requires negotiated process ownership:
/// caller-controlled process IDs cannot safely select a destructive policy,
/// and a wrapper may exit while its descendants continue serving requests.
#[derive(Clone)]
pub struct ExecutorStdioServerLauncher {
    exec_backend: Arc<dyn ExecBackend>,
}

impl ExecutorStdioServerLauncher {
    /// Creates a stdio server launcher backed by the executor process API.
    pub fn new(exec_backend: Arc<dyn ExecBackend>) -> Self {
        Self { exec_backend }
    }
}

impl StdioServerLauncher for ExecutorStdioServerLauncher {
    fn launch(
        &self,
        command: StdioServerCommand,
    ) -> BoxFuture<'static, io::Result<StdioServerTransport>> {
        let exec_backend = Arc::clone(&self.exec_backend);
        async move { Self::launch_server(command, exec_backend).await }.boxed()
    }
}

// Remote private implementation.

impl private::Sealed for ExecutorStdioServerLauncher {}

impl ExecutorStdioServerLauncher {
    async fn launch_server(
        command: StdioServerCommand,
        exec_backend: Arc<dyn ExecBackend>,
    ) -> io::Result<StdioServerTransport> {
        let StdioServerCommand {
            program,
            args,
            env,
            env_vars,
            cwd,
            protocol_mode: _,
            local_process_observer: _,
        } = command;
        let Some(cwd) = cwd else {
            return Err(io::Error::other(
                "executor stdio server requires an explicit cwd",
            ));
        };
        let cwd: PathUri = LegacyAppPathString::from_path(Path::new(&cwd))
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let program_name = program.to_string_lossy().into_owned();
        let envs = create_env_overlay_for_remote_mcp_server(env, &env_vars);
        let remote_env_vars = remote_mcp_env_var_names(&env_vars);
        // The executor protocol carries argv/env as UTF-8 strings. Local stdio can
        // accept arbitrary OsString values because it calls the OS directly; remote
        // stdio must reject non-Unicode command, argument, or environment data
        // before sending an executor request.
        let argv = Self::process_api_argv(&program, &args).map_err(io::Error::other)?;
        let env = Self::process_api_env(envs).map_err(io::Error::other)?;
        let process_id = ExecutorProcessTransport::next_process_id();
        // Start the MCP server process on the executor with raw pipes. `tty=false`
        // keeps stdout as a clean protocol stream, while `pipe_stdin=true` lets
        // rmcp write JSON-RPC requests after the process starts.
        let started = exec_backend
            .start(ExecParams {
                metadata: Default::default(),
                process_id,
                argv,
                cwd,
                shell_snapshot: None,
                env_policy: Some(Self::remote_env_policy(&remote_env_vars)),
                env,
                tty: false,
                pipe_stdin: true,
                arg0: None,
                sandbox: None,
                enforce_managed_network: false,
                managed_network: None,
                network_proxy: None,
            })
            .await
            .map_err(io::Error::other)?;

        let process =
            StdioServerProcessHandle::executor(program_name.clone(), Arc::clone(&started.process));
        Ok(StdioServerTransport {
            inner: StdioServerTransportInner::Executor(ExecutorProcessTransport::new(
                started.process,
                program_name,
            )),
            process,
        })
    }

    fn process_api_argv(program: &OsString, args: &[OsString]) -> Result<Vec<String>> {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(Self::os_string_to_process_api_string(
            program.clone(),
            "command",
        )?);
        for arg in args {
            argv.push(Self::os_string_to_process_api_string(
                arg.clone(),
                "argument",
            )?);
        }
        Ok(argv)
    }

    fn process_api_env(env: HashMap<OsString, OsString>) -> Result<HashMap<String, String>> {
        env.into_iter()
            .map(|(key, value)| {
                Ok((
                    Self::os_string_to_process_api_string(key, "environment variable name")?,
                    Self::os_string_to_process_api_string(value, "environment variable value")?,
                ))
            })
            .collect()
    }

    fn os_string_to_process_api_string(value: OsString, label: &str) -> Result<String> {
        value
            .into_string()
            .map_err(|_| anyhow!("{label} must be valid Unicode for remote MCP stdio"))
    }

    fn remote_env_policy(remote_env_vars: &[String]) -> ExecEnvPolicy {
        let include_only = if remote_env_vars.is_empty() {
            Vec::new()
        } else {
            // `source = "remote"` means the value is read from the executor's
            // environment, not copied from Codex. Start from `All` only so the
            // named remote variable is available to the filter below; the
            // effective child env is still limited by `include_only`.
            crate::utils::DEFAULT_ENV_VARS
                .iter()
                .map(|name| (*name).to_string())
                .chain(remote_env_vars.iter().cloned())
                .collect()
        };
        ExecEnvPolicy {
            inherit: if remote_env_vars.is_empty() {
                ShellEnvironmentPolicyInherit::Core
            } else {
                ShellEnvironmentPolicyInherit::All
            },
            ignore_default_excludes: true,
            exclude: Vec::new(),
            r#set: HashMap::new(),
            include_only,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::config_types::EnvironmentVariablePattern;
    use codex_protocol::config_types::ShellEnvironmentPolicy;
    use codex_protocol::shell_environment;

    #[test]
    fn remote_env_policy_uses_core_env_without_remote_source_vars() {
        let policy = ExecutorStdioServerLauncher::remote_env_policy(&[]);

        assert_eq!(policy.inherit, ShellEnvironmentPolicyInherit::Core);
        assert!(policy.include_only.is_empty());
    }

    #[test]
    fn remote_env_policy_includes_remote_source_vars_without_full_env() {
        let policy = ExecutorStdioServerLauncher::remote_env_policy(&["REMOTE_TOKEN".to_string()]);

        assert_eq!(policy.inherit, ShellEnvironmentPolicyInherit::All);
        assert!(
            policy.include_only.contains(&"REMOTE_TOKEN".to_string()),
            "remote source var should be included in executor env policy"
        );
        assert!(
            policy
                .include_only
                .contains(&crate::utils::DEFAULT_ENV_VARS[0].to_string()),
            "remote default env vars should remain available"
        );
    }

    #[test]
    fn remote_env_policy_effectively_filters_unrequested_vars() {
        let exec_policy =
            ExecutorStdioServerLauncher::remote_env_policy(&["REMOTE_TOKEN".to_string()]);
        let policy = ShellEnvironmentPolicy {
            inherit: exec_policy.inherit,
            ignore_default_excludes: exec_policy.ignore_default_excludes,
            exclude: exec_policy
                .exclude
                .iter()
                .map(|pattern| EnvironmentVariablePattern::new_case_insensitive(pattern))
                .collect(),
            r#set: exec_policy.r#set,
            include_only: exec_policy
                .include_only
                .iter()
                .map(|pattern| EnvironmentVariablePattern::new_case_insensitive(pattern))
                .collect(),
            use_profile: false,
        };

        let env = shell_environment::create_env_from_vars(
            [
                ("PATH".to_string(), "/remote/bin".to_string()),
                ("REMOTE_TOKEN".to_string(), "remote-secret".to_string()),
                (
                    "UNREQUESTED_SECRET".to_string(),
                    "must-not-pass".to_string(),
                ),
            ],
            &policy,
            /*thread_id*/ None,
        );

        assert_eq!(env.get("PATH").map(String::as_str), Some("/remote/bin"));
        assert_eq!(
            env.get("REMOTE_TOKEN").map(String::as_str),
            Some("remote-secret")
        );
        assert!(!env.contains_key("UNREQUESTED_SECRET"));
    }
}
