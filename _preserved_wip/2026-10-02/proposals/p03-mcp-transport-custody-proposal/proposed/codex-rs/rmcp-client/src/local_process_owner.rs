//! Exact local process and stderr custody independent of shutdown observers.
//!
//! The registry owns the resources separately from the worker's future. Even a
//! cancelled worker therefore leaves the original child and reader reachable.

use std::io;
#[cfg(windows)]
use std::os::windows::io::OwnedHandle;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::PoisonError;
use std::sync::Weak;
use std::time::Duration;

use codex_async_utils::RetainedTask;
use codex_async_utils::TaskJoinFailure;
use codex_utils_pty::Child;
#[cfg(unix)]
use codex_utils_pty::process_group::kill_process_group;
#[cfg(unix)]
use codex_utils_pty::process_group::terminate_process_group;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::Lines;
use tokio::process::ChildStderr;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use tokio::runtime::Handle;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

#[cfg(unix)]
const GROUP_GRACE: Duration = Duration::from_secs(/*secs*/ 2);
const CHILD_GRACE: Duration = Duration::from_secs(/*secs*/ 3);
const STDERR_GRACE: Duration = Duration::from_millis(/*millis*/ 250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseFailure {
    Signal,
    Wait,
    Kill,
    Framing,
    UnobservedFraming,
    UnconfirmedOwnership,
    RuntimeUnavailable,
    TaskPanicked,
    TaskCancelled,
    SpawnPanicked,
}

impl std::fmt::Display for CloseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Signal => "MCP process termination signal failed",
            Self::Wait => "MCP child exit observation failed",
            Self::Kill => "MCP child kill or reap failed",
            Self::Framing => "MCP stdio framing close failed",
            Self::UnobservedFraming => "MCP stdio framing close was not observed",
            Self::UnconfirmedOwnership => "MCP stdio resource ownership remains unconfirmed",
            Self::RuntimeUnavailable => "MCP process cleanup runtime unavailable",
            Self::TaskPanicked => "MCP process cleanup task panicked",
            Self::TaskCancelled => "MCP process cleanup task was cancelled",
            Self::SpawnPanicked => "MCP process cleanup task spawn panicked",
        })
    }
}

impl std::error::Error for CloseFailure {}

#[cfg(unix)]
pub(crate) struct LocalProcessTerminator {
    process_group_id: u32,
}

#[cfg(windows)]
pub(crate) enum LocalProcessTerminator {
    Job(codex_utils_pty::JobObject),
    Process(OwnedHandle),
}

#[cfg(not(any(unix, windows)))]
pub(crate) struct LocalProcessTerminator;

#[derive(Default)]
struct Registry {
    processes: Mutex<Vec<Arc<ProcessState>>>,
}

static PROCESSES: OnceLock<Arc<Registry>> = OnceLock::new();

type CloseTask = RetainedTask<Result<(), CloseFailure>>;

struct ProcessState {
    registry: Weak<Registry>,
    child: AsyncMutex<Option<Child>>,
    stderr: AsyncMutex<Option<Lines<BufReader<ChildStderr>>>>,
    terminator: Mutex<Option<LocalProcessTerminator>>,
    shutdown_at: OnceLock<Instant>,
    shutdown: CancellationToken,
    worker: OnceLock<Arc<CloseTask>>,
    worker_ready: CancellationToken,
    termination_ready: CancellationToken,
    framing: Mutex<Option<Result<(), CloseFailure>>>,
    // Retain original errors privately; public receipts have fixed classifications.
    errors: Mutex<Vec<io::Error>>,
    // Reading diagnostics can fail after ownership has been closed successfully.
    diagnostic_errors: Mutex<Vec<io::Error>>,
    program_name: String,
}

#[derive(Clone)]
pub(crate) struct LocalProcessOwner {
    state: Arc<ProcessState>,
    process_id: Option<u32>,
}

impl LocalProcessOwner {
    pub(crate) fn new(
        child: Child,
        program_name: String,
        observer: Option<Arc<dyn Fn(LocalProcessOwner) + Send + Sync>>,
    ) -> io::Result<(Self, ChildStdin, ChildStdout)> {
        Self::new_in(child, program_name, PROCESSES.get_or_init(Default::default), observer)
    }

    fn new_in(
        mut child: Child,
        program_name: String,
        registry: &Arc<Registry>,
        observer: Option<Arc<dyn Fn(LocalProcessOwner) + Send + Sync>>,
    ) -> io::Result<(Self, ChildStdin, ChildStdout)> {
        let process_id = child.id();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take().map(|stderr| BufReader::new(stderr).lines());
        #[cfg(unix)]
        let terminator = process_id.map(|process_group_id| LocalProcessTerminator {
            process_group_id,
        });
        #[cfg(not(unix))]
        let terminator = None;
        let state = Arc::new(ProcessState {
            registry: Arc::downgrade(registry),
            child: AsyncMutex::new(Some(child)),
            stderr: AsyncMutex::new(stderr),
            terminator: Mutex::new(terminator),
            shutdown_at: OnceLock::new(),
            shutdown: CancellationToken::new(),
            worker: OnceLock::new(),
            worker_ready: CancellationToken::new(),
            termination_ready: CancellationToken::new(),
            framing: Mutex::new(None),
            errors: Mutex::new(Vec::new()),
            diagnostic_errors: Mutex::new(Vec::new()),
            program_name,
        });
        registry.processes.lock().unwrap_or_else(PoisonError::into_inner)
            .push(Arc::clone(&state));
        let owner = Self { state, process_id };
        // Publish the actual child outside all owner/registry locks, before
        // worker creation or fallible stdio validation can end this generation.
        if let Some(observer) = observer {
            observer(owner.clone());
        }
        #[cfg(not(windows))]
        owner.state.termination_ready.cancel();
        if let Ok(runtime) = Handle::try_current() {
            let running = Arc::clone(&owner.state);
            let published = Arc::clone(&owner.state);
            let _ = RetainedTask::spawn_published(&runtime, async move {
                let (process, stderr) = tokio::join!(running.close_child(), running.drain_stderr());
                process.and(stderr)
            }, move |task| {
                let _ = published.worker.set(task);
            });
            owner.state.worker_ready.cancel();
        } else {
            owner.state.worker_ready.cancel();
            owner.begin_shutdown();
            return Err(io::Error::other(CloseFailure::RuntimeUnavailable));
        }
        match (stdin, stdout) {
            (Some(stdin), Some(stdout)) => Ok((owner, stdin, stdout)),
            _ => {
                owner.record_framing(Err(io::Error::other("MCP server stdio was not piped")));
                // No transport is returned for the launcher to attach its
                // Windows terminator; the exact child still needs reaping.
                owner.state.termination_ready.cancel();
                owner.begin_shutdown();
                Err(io::Error::other("MCP server stdio was not piped"))
            }
        }
    }

    pub(crate) fn id(&self) -> Option<u32> {
        self.process_id
    }

    #[cfg(windows)]
    pub(crate) fn set_terminator(&self, terminator: Option<LocalProcessTerminator>) {
        let previous = {
            let mut current = self.state.terminator.lock().unwrap_or_else(PoisonError::into_inner);
            std::mem::replace(&mut *current, terminator)
        };
        drop(previous);
        // A closed client roster can request shutdown during owner publication.
        // Its worker must observe this platform handle before sending signals.
        self.state.termination_ready.cancel();
    }

    pub(crate) fn begin_shutdown(&self) {
        self.state.shutdown_at.get_or_init(Instant::now);
        self.state.shutdown.cancel();
    }

    pub(crate) fn is_shutdown_started(&self) -> bool {
        self.state.shutdown.is_cancelled()
    }

    pub(crate) fn record_framing(&self, result: io::Result<()>) {
        let result = result.map_err(|error| self.state.record_error(CloseFailure::Framing, error));
        let mut framing = self.state.framing.lock().unwrap_or_else(PoisonError::into_inner);
        if !matches!(*framing, Some(Err(_))) {
            *framing = Some(result);
        }
    }

    pub(crate) fn transport_dropped(&self) {
        let mut framing = self.state.framing.lock().unwrap_or_else(PoisonError::into_inner);
        if framing.is_none() {
            *framing = Some(Err(CloseFailure::UnobservedFraming));
        }
        drop(framing);
        self.begin_shutdown();
    }

    /// Observe signal, direct-child reaping and stderr completion independently
    /// of framing. The service can request termination before closing its pipes.
    pub(crate) async fn terminate(&self) -> io::Result<()> {
        self.begin_shutdown();
        self.state.worker_ready.cancelled().await;
        let worker = self.state.worker.get()
            .ok_or_else(|| io::Error::other(CloseFailure::RuntimeUnavailable))?;
        match worker.wait().await {
            Ok(result) => (*result).map_err(io::Error::other),
            Err(error) => Err(io::Error::other(match error {
                TaskJoinFailure::Panicked => CloseFailure::TaskPanicked,
                TaskJoinFailure::Cancelled => CloseFailure::TaskCancelled,
                TaskJoinFailure::SpawnPanicked => CloseFailure::SpawnPanicked,
            })),
        }
    }

    /// Only a joined, reaped process and a successful framing observation retire
    /// the separate resource owner. Failure and cancelled observers retain it.
    pub(crate) async fn wait_closed(&self) -> io::Result<()> {
        let process = self.terminate().await;
        let framing = *self.state.framing.lock().unwrap_or_else(PoisonError::into_inner);
        let framing = framing.unwrap_or(Err(CloseFailure::UnobservedFraming));
        let direct_child_reaped = self.state.child.lock().await.is_none();
        let stderr_closed = self.state.stderr.lock().await.is_none();
        let diagnostic_errors = self.state.diagnostic_errors.lock()
            .unwrap_or_else(PoisonError::into_inner).len();
        let ownership_confirmed = process.is_ok() && framing.is_ok()
            && direct_child_reaped && stderr_closed;
        tracing::info!(target: "codex_rmcp_client::lifecycle",
            event = "mcp_local_process_shutdown_observed", receipt_version = 1_u64,
            direct_child_reaped, stderr_closed, diagnostic_errors, ownership_confirmed,
            process_group_completion = "unconfirmed",
            "MCP local process shutdown observed");
        process.and(framing.map_err(io::Error::other))?;
        if !ownership_confirmed {
            return Err(io::Error::other(CloseFailure::UnconfirmedOwnership));
        }
        if let Some(registry) = self.state.registry.upgrade() {
            let retired = {
                let mut processes = registry.processes.lock().unwrap_or_else(PoisonError::into_inner);
                processes.iter().position(|state| Arc::ptr_eq(state, &self.state))
                    .map(|index| processes.swap_remove(index))
            };
            drop(retired);
        }
        Ok(())
    }
}

impl ProcessState {
    fn record_error(&self, failure: CloseFailure, error: io::Error) -> CloseFailure {
        self.errors.lock().unwrap_or_else(PoisonError::into_inner).push(error);
        failure
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "borrow the registry-owned child across wait and kill so worker cancellation preserves custody"
    )]
    async fn close_child(&self) -> Result<(), CloseFailure> {
        self.shutdown.cancelled().await;
        self.termination_ready.cancelled().await;
        let started = *self.shutdown_at.get_or_init(Instant::now);
        let signal = self.signal_and_escalate(started).await;
        let mut child_slot = self.child.lock().await;
        let Some(child) = child_slot.as_mut() else { return signal };
        let mut failure = None;
        let reaped = match tokio::time::timeout_at(started + CHILD_GRACE, child.wait()).await {
            Ok(Ok(_)) => true,
            result => {
                if let Ok(Err(error)) = result {
                    failure = Some(self.record_error(CloseFailure::Wait, error));
                }
                match child.kill().await {
                    Ok(()) => true,
                    Err(error) => {
                        let kill_failure = self.record_error(CloseFailure::Kill, error);
                        failure.get_or_insert(kill_failure);
                        false
                    }
                }
            }
        };
        let reaped_child = if reaped { child_slot.take() } else { None };
        drop(child_slot);
        drop(reaped_child);
        signal.and(failure.map_or(Ok(()), Err))
    }

    async fn signal_and_escalate(&self, started: Instant) -> Result<(), CloseFailure> {
        #[cfg(unix)]
        {
            let signal = {
                let terminator = self.terminator.lock().unwrap_or_else(PoisonError::into_inner);
                match terminator.as_ref() {
                    Some(terminator) => terminate_process_group(terminator.process_group_id),
                    None => Ok(false),
                }
            };
            let mut failure = None;
            let escalate = match signal {
                Ok(exists) => exists,
                Err(error) => {
                    failure = Some(self.record_error(CloseFailure::Signal, error));
                    true
                }
            };
            if escalate {
                // Retain the unreaped leader until escalation finishes so its
                // process-group ID cannot be reused before this final signal.
                tokio::time::sleep_until(started + GROUP_GRACE).await;
                let result = {
                    let terminator = self.terminator.lock().unwrap_or_else(PoisonError::into_inner);
                    match terminator.as_ref() {
                        Some(terminator) => kill_process_group(terminator.process_group_id),
                        None => Ok(()),
                    }
                };
                if let Err(error) = result {
                    let signal_failure = self.record_error(CloseFailure::Signal, error);
                    failure.get_or_insert(signal_failure);
                }
            }
            failure.map_or(Ok(()), Err)
        }
        #[cfg(windows)]
        {
            let _ = started;
            let result = {
                let terminator = self.terminator.lock().unwrap_or_else(PoisonError::into_inner);
                match terminator.as_ref() {
                    Some(LocalProcessTerminator::Job(job)) => job.terminate(),
                    Some(LocalProcessTerminator::Process(handle)) => {
                        codex_utils_pty::JobObject::terminate_process_handle(handle)
                    }
                    None => Ok(()),
                }
            };
            result.map_err(|error| self.record_error(CloseFailure::Signal, error))
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = started;
            Ok(())
        }
    }

    #[expect(
        clippy::await_holding_invalid_type,
        reason = "borrow the registry-owned stderr reader so worker cancellation cannot drop its handle"
    )]
    async fn drain_stderr(&self) -> Result<(), CloseFailure> {
        let mut stderr = self.stderr.lock().await;
        let Some(reader) = stderr.as_mut() else { return Ok(()) };
        let mut drain_until = None;
        let outcome = loop {
            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(drain_until.unwrap_or_else(Instant::now)), if drain_until.is_some() => break Ok(()),
                _ = self.shutdown.cancelled(), if drain_until.is_none() => {
                    let started = *self.shutdown_at.get_or_init(Instant::now);
                    drain_until = Some(started + STDERR_GRACE);
                }
                line = reader.next_line() => match line {
                    Ok(Some(line)) => tracing::info!("MCP server stderr ({}): {line}", self.program_name),
                    Ok(None) => break Ok(()),
                    Err(error) => {
                        self.diagnostic_errors.lock().unwrap_or_else(PoisonError::into_inner)
                            .push(error);
                        break Ok(());
                    }
                },
            }
        };
        let completed_reader = stderr.take();
        drop(stderr);
        drop(completed_reader);
        outcome
    }
}

#[cfg(all(test, unix))]
#[path = "local_process_owner_tests.rs"]
mod tests;
