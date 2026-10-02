//! Real same-process A-to-B acceptance, run once in a parent-owned fresh test ELF.
//! The parent owns Git/model/MCP fixtures and the finite child-process deadline.

use super::{InProcessClientSender, InProcessServerEvent, InProcessStartArgs, start};
use anyhow::{Context, Result, bail, ensure};
use codex_app_server_protocol::{
    ClientInfo, ClientRequest, InitializeParams, JSONRPCErrorError,
    McpServerStartupState, McpServerToolCallParams, McpServerToolCallResponse,
    PluginListParams, PluginListResponse, RequestId, ServerNotification,
    SkillsListParams, SkillsListResponse, ThreadStartParams, ThreadStartResponse,
    TurnInterruptParams, TurnInterruptResponse, TurnStartParams, TurnStartResponse,
    TurnStatus, UserInput,
};
use codex_arg0::Arg0DispatchPaths;
use codex_config::{CloudConfigBundleLoader, LoaderOverrides};
use codex_core::config::{ConfigBuilder, ConfigOverrides};
use codex_core_plugins::PluginsManager;
use codex_core_plugins::startup_sync::{
    CuratedCallbackActivity, CuratedCallbackObservation, CuratedCallbackScope,
    CuratedProcessShutdown, CuratedSyncLifecycleObservation,
    CuratedSyncNativeCompletion, CuratedSyncOperationDisposition,
};
use codex_exec_server::EnvironmentManager;
use codex_feedback::CodexFeedback;
use codex_protocol::protocol::SessionSource;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

const OBSERVE: Duration = Duration::from_secs(90);
const PLUGIN: &str = "replacement-probe@openai-api-curated";
const SKILL: &str = "replacement-probe:replacement-skill";
const TOOL: &str = "replacement_probe";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case { Pending, Replay, Race }

impl Case {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "replay" => Ok(Self::Replay),
            "race" => Ok(Self::Race),
            _ => bail!("unknown curated replacement case"),
        }
    }
    fn label(self) -> &'static str {
        match self { Self::Pending => "pending", Self::Replay => "replay", Self::Race => "race" }
    }
}

struct Control(BufReader<TcpStream>);

impl Control {
    async fn connect() -> Result<Self> {
        let address: SocketAddr = std::env::var("CODEX_TEST_CURATED_REPLACEMENT_CONTROL")?.parse()?;
        ensure!(address.ip().is_loopback() && address.port() != 0, "control must be a loopback fixture");
        Ok(Self(BufReader::new(timeout(OBSERVE, TcpStream::connect(address)).await??)))
    }

    async fn checkpoint(&mut self, event: &'static str) -> Result<()> {
        timeout(OBSERVE, async {
            let mut message = serde_json::to_vec(&json!({"event": event}))?;
            message.push(b'\n');
            self.0.get_mut().write_all(&message).await?;
            self.0.get_mut().flush().await?;
            let mut line = String::new();
            ensure!(self.0.read_line(&mut line).await? != 0, "control closed before acknowledgement");
            ensure!(line.len() <= 4096, "control acknowledgement exceeded fixture bound");
            let ack: Value = serde_json::from_str(&line)?;
            ensure!(ack.get("ack").and_then(Value::as_str) == Some(event), "control acknowledgement mismatch");
            Ok::<_, anyhow::Error>(())
        }).await.context("control checkpoint deadline")?
    }
}

#[derive(Debug)]
struct RpcFailure(JSONRPCErrorError);
impl fmt::Display for RpcFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "typed RPC failed with code {}", self.0.code)
    }
}
impl std::error::Error for RpcFailure {}

#[derive(Clone, Debug)]
struct EventFailure(InProcessServerEvent);
impl fmt::Display for EventFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let category = match &self.0 {
            InProcessServerEvent::Lagged { .. } => "lagged",
            InProcessServerEvent::ServerRequest(_) => "unexpected server request",
            InProcessServerEvent::ServerNotification(_) => "MCP startup failure",
        };
        write!(f, "replacement event observation failed: {category}")
    }
}
impl std::error::Error for EventFailure {}

#[derive(Debug)]
struct BothFailures { operation: anyhow::Error, cleanup: anyhow::Error }
impl fmt::Display for BothFailures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; cleanup also failed: {}", self.operation, self.cleanup)
    }
}
impl std::error::Error for BothFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { Some(self.operation.as_ref()) }
}
fn combine(first: Result<()>, second: Result<()>) -> Result<()> {
    match (first, second) {
        (Ok(()), result) | (result, Ok(())) => result,
        (Err(operation), Err(cleanup)) => Err(BothFailures { operation, cleanup }.into()),
    }
}

#[derive(Default)]
struct Events {
    ready: BTreeSet<(String, String)>,
    turns: HashMap<(String, String), TurnStatus>,
    failure: Option<EventFailure>,
}

impl Events {
    fn check(&self) -> Result<()> {
        if let Some(error) = &self.failure { return Err(error.clone().into()); }
        Ok(())
    }
    fn observe(&mut self, event: InProcessServerEvent) {
        match &event {
            InProcessServerEvent::ServerNotification(notification) => match notification.as_ref() {
                ServerNotification::McpServerStatusUpdated(update) => {
                    if update.status == McpServerStartupState::Ready {
                        if let Some(thread) = &update.thread_id {
                            self.ready.insert((thread.clone(), update.name.clone()));
                        }
                    } else if update.status == McpServerStartupState::Failed {
                        self.failure.get_or_insert(EventFailure(event));
                    }
                }
                ServerNotification::TurnCompleted(update) => {
                    self.turns.insert((update.thread_id.clone(), update.turn.id.clone()), update.turn.status.clone());
                }
                _ => {}
            },
            InProcessServerEvent::Lagged { .. } | InProcessServerEvent::ServerRequest(_) => {
                self.failure.get_or_insert(EventFailure(event));
            }
        }
    }
}

/// One continuously drained event stream and one exact task join per runtime.
struct Embedded {
    sender: InProcessClientSender,
    scope: Arc<CuratedCallbackScope>,
    events: Arc<Mutex<Events>>,
    stop: CancellationToken,
    task: Option<tokio::task::JoinHandle<Result<()>>>,
    next_id: i64,
}

impl Embedded {
    async fn launch(home: &Path) -> Result<Self> {
        let config = Arc::new(ConfigBuilder::default()
            .codex_home(home.to_path_buf())
            .harness_overrides(ConfigOverrides {
                cwd: Some(home.to_path_buf()), bypass_hook_trust: Some(true), ..Default::default()
            }).build().await?);
        ensure!(config.sqlite_config().home() == home,
            "resolved SQLite home differs from fixture home");
        let state_db = codex_rollout::state_db::try_init(config.as_ref()).await?;
        let mut client = start(InProcessStartArgs {
            arg0_paths: Arg0DispatchPaths::default(), config,
            cli_overrides: Vec::new(), loader_overrides: LoaderOverrides::default(),
            strict_config: false, cloud_config_bundle: CloudConfigBundleLoader::default(),
            embedded_network_policy: Default::default(),
            thread_config_loader: Arc::new(codex_config::NoopThreadConfigLoader),
            feedback: CodexFeedback::new(), log_db: None, state_db: Some(state_db),
            environment_manager: Arc::new(EnvironmentManager::default_for_tests()),
            config_warnings: Vec::new(), session_source: SessionSource::Cli,
            enable_codex_api_key_env: false,
            initialize: InitializeParams {
                client_info: ClientInfo { name: "curated-replacement-acceptance".into(), title: None, version: "1".into() },
                capabilities: None,
            },
            channel_capacity: 1024,
        }).await?;
        let scope = match client._curated_callback_lifecycle.as_ref().map(|guard| guard.scope()) {
            Some(scope) => scope,
            None => {
                let operation = anyhow::anyhow!("real startup did not install callback custody");
                return match client.shutdown().await {
                    Ok(()) => Err(operation),
                    Err(cleanup) => Err(BothFailures { operation, cleanup: cleanup.into() }.into()),
                };
            }
        };
        let sender = client.sender();
        let events = Arc::new(Mutex::new(Events::default()));
        let record = Arc::clone(&events);
        let stop = CancellationToken::new();
        let stopping = stop.clone();
        let task = tokio::spawn(async move {
            let mut observation = Ok(());
            loop {
                tokio::select! {
                    biased;
                    _ = stopping.cancelled() => break,
                    event = client.next_event() => match event {
                        Some(event) => {
                            if let InProcessServerEvent::ServerRequest(request) = &event {
                                if let Err(error) = client.fail_server_request(request.id().clone(), JSONRPCErrorError {
                                    code: -32603, message: "acceptance fixture cannot approve server requests".into(), data: None,
                                }) {
                                    record.lock().unwrap_or_else(PoisonError::into_inner).observe(event);
                                    observation = Err(error.into());
                                    break;
                                }
                            }
                            record.lock().unwrap_or_else(PoisonError::into_inner).observe(event);
                        }
                        None => { observation = Err(anyhow::anyhow!("runtime event stream closed before requested shutdown")); break; }
                    }
                }
            }
            combine(observation, client.shutdown().await.map_err(anyhow::Error::from))
        });
        Ok(Self { sender, scope, events, stop, task: Some(task), next_id: 1 })
    }

    async fn request<T: DeserializeOwned>(&mut self, make: impl FnOnce(RequestId) -> ClientRequest) -> Result<T> {
        self.events.lock().unwrap_or_else(PoisonError::into_inner).check()?;
        let id = self.next_id;
        self.next_id += 1;
        let response = timeout(OBSERVE, self.sender.request(make(RequestId::Integer(id)))).await??
            .map_err(RpcFailure)?;
        let parsed = serde_json::from_value(response)?;
        self.events.lock().unwrap_or_else(PoisonError::into_inner).check()?;
        Ok(parsed)
    }

    async fn thread(&mut self, home: &Path) -> Result<String> {
        let response: ThreadStartResponse = self.request(|request_id| ClientRequest::ThreadStart {
            request_id, params: ThreadStartParams {
                cwd: Some(home.to_string_lossy().into_owned()),
                config: Some(HashMap::from([("bypass_hook_trust".into(), json!(true))])),
                ..Default::default()
            },
        }).await?;
        Ok(response.thread.id)
    }

    async fn metadata(&mut self, home: &Path, version: &str) -> Result<()> {
        let plugins: PluginListResponse = self.request(|request_id| ClientRequest::PluginList {
            request_id, params: PluginListParams { cwds: None, marketplace_kinds: None, force_refetch: false },
        }).await?;
        ensure!(plugins.marketplace_load_errors.is_empty(), "typed plugin list contained load errors");
        let matching: Vec<_> = plugins.marketplaces.iter().flat_map(|market| &market.plugins)
            .filter(|plugin| plugin.id == PLUGIN).collect();
        ensure!(matching.len() == 1, "typed plugin identity was missing or duplicated");
        let plugin = matching[0];
        ensure!(plugin.installed && plugin.enabled, "fixture plugin is not installed and enabled");
        ensure!(plugin.local_version.as_deref() == Some(version), "typed installed plugin version mismatch");
        let skills: SkillsListResponse = self.request(|request_id| ClientRequest::SkillsList {
            request_id, params: SkillsListParams { cwds: vec![home.to_path_buf()], force_reload: false },
        }).await?;
        ensure!(skills.data.iter().all(|entry| entry.errors.is_empty()), "typed skills list contained load errors");
        let matching: Vec<_> = skills.data.iter().flat_map(|entry| &entry.skills)
            .filter(|skill| skill.name == SKILL).collect();
        ensure!(matching.len() == 1, "typed skill identity was missing or duplicated");
        ensure!(matching[0].enabled && matching[0].plugin_id.as_deref() == Some(PLUGIN), "typed skill ownership mismatch");
        ensure!(matching[0].description == version, "typed cached skill description mismatch");
        Ok(())
    }

    async fn ready(&self, thread: &str, server: &str) -> Result<()> {
        timeout(OBSERVE, async {
            loop {
                {
                    let events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
                    events.check()?;
                    if events.ready.contains(&(thread.to_owned(), server.to_owned())) { return Ok::<_, anyhow::Error>(()); }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.context("typed MCP Ready deadline")?
    }

    async fn tool(&mut self, thread: &str, server: &str, version: &str) -> Result<()> {
        // The caller must have observed this thread/server's Ready notification
        // before this direct request, which could otherwise drive dirty refresh.
        ensure!(self.events.lock().unwrap_or_else(PoisonError::into_inner).ready
            .contains(&(thread.to_owned(), server.to_owned())), "MCP tool attempted before typed Ready");
        let response: McpServerToolCallResponse = self.request(|request_id| ClientRequest::McpServerToolCall {
            request_id, params: McpServerToolCallParams {
                thread_id: thread.into(), server: server.into(), tool: TOOL.into(),
                arguments: Some(json!({})), meta: None,
            },
        }).await?;
        ensure!(response.is_error != Some(true), "probe tool returned an operation error");
        ensure!(response.content == vec![json!({"type": "text", "text": version})], "typed MCP probe version mismatch");
        Ok(())
    }

    async fn turn(&mut self, thread: &str) -> Result<String> {
        let response: TurnStartResponse = self.request(|request_id| ClientRequest::TurnStart {
            request_id, params: TurnStartParams {
                thread_id: thread.into(), input: vec![UserInput::Text {
                    text: "Hold this acceptance turn until explicitly interrupted.".into(), text_elements: Vec::new(),
                }], ..Default::default()
            },
        }).await?;
        Ok(response.turn.id)
    }

    async fn interrupt(&mut self, thread: &str, turn: &str) -> Result<()> {
        let _: TurnInterruptResponse = self.request(|request_id| ClientRequest::TurnInterrupt {
            request_id, params: TurnInterruptParams { thread_id: thread.into(), turn_id: turn.into() },
        }).await?;
        timeout(OBSERVE, async {
            loop {
                {
                    let events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
                    events.check()?;
                    if let Some(status) = events.turns.get(&(thread.into(), turn.into())) {
                        ensure!(*status == TurnStatus::Interrupted, "held turn completed without requested interruption");
                        return Ok::<_, anyhow::Error>(());
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.context("interrupted turn completion deadline")?
    }

    async fn close(&mut self) -> Result<()> {
        self.stop.cancel();
        if let Some(task) = self.task.take() { task.await??; }
        self.events.lock().unwrap_or_else(PoisonError::into_inner).check()
    }
}

fn native() -> Result<CuratedSyncLifecycleObservation> {
    PluginsManager::observe_curated_repo_sync().context("actual startup did not create a curated generation")
}

fn validate_generation(observed: CuratedSyncLifecycleObservation, generation: u64) -> Result<()> {
    ensure!(observed.generation == Some(generation), "curated worker generation changed");
    ensure!(!observed.admission_closed, "embedded replacement closed process admission");
    ensure!(!observed.quarantined && observed.unexpected_handles == 0, "native ownership is quarantined or unexpected");
    Ok(())
}

async fn native_finished(generation: u64) -> Result<()> {
    timeout(OBSERVE, async {
        loop {
            let observed = native()?;
            validate_generation(observed, generation)?;
            if let Some(disposition) = observed.operation {
                ensure!(disposition == CuratedSyncOperationDisposition::Succeeded, "real curated operation did not succeed");
            }
            if observed.operation == Some(CuratedSyncOperationDisposition::Succeeded)
                && observed.native_handle_finished == Some(true)
            {
                ensure!(observed.native == CuratedSyncNativeCompletion::Running, "passive native observation unexpectedly joined the handle");
                return Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.context("real curated native completion deadline")?
}

fn activity_value(value: CuratedCallbackActivity) -> Value {
    json!({"admission_closed": value.admission_closed, "awaiting_handle": value.awaiting_handle,
        "running": value.running, "finished_unjoined": value.finished_unjoined, "joining": value.joining,
        "completed": value.completed, "suppressed": value.suppressed, "failed": value.failed,
        "unexpected": value.unexpected})
}

async fn callback_finished(scope: &CuratedCallbackScope) -> Result<CuratedCallbackActivity> {
    timeout(OBSERVE, async {
        loop {
            let state = scope.activity();
            ensure!(!state.admission_closed, "B callback scope closed during behavioral proof");
            ensure!(state.completed == 0 && state.suppressed == 0 && state.failed == 0 && state.unexpected == 0,
                "B callback result was consumed, suppressed, failed or unexpected before final wait");
            ensure!(state.finished_unjoined <= 1, "B admitted duplicate callback tasks");
            if state.awaiting_handle == 0 && state.running == 0 && state.joining == 0 && state.finished_unjoined == 1 {
                return Ok::<_, anyhow::Error>(state);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.context("B callback activity deadline")?
}

fn counts(value: CuratedCallbackObservation) -> Value {
    json!({"pending": value.pending, "completed": value.completed, "suppressed": value.suppressed, "failed": value.failed})
}
fn exact_counts(value: CuratedCallbackObservation, completed: usize) -> Result<()> {
    ensure!(value.pending == 0 && value.completed == completed && value.suppressed == 0 && value.failed == 0,
        "final callback counts did not match exact successful actions");
    Ok(())
}

#[derive(Default)]
struct Trial { a: Option<Embedded>, b: Option<Embedded>, generation: Option<u64>, receipt: Value }

impl Trial {
    async fn exercise(&mut self, home: &Path, case: Case, control: &mut Control) -> Result<()> {
        ensure!(!home.join("hook.log").exists() || tokio::fs::read(home.join("hook.log")).await?.is_empty(), "hook log was not initially empty");
        self.receipt = json!({"receipt_version": 1, "case": case.label(), "existing_b_before_state": case == Case::Pending});
        ensure!(PluginsManager::observe_curated_repo_sync().is_none(), "fresh process already had a curated gate");
        ensure!(PluginsManager::observe_curated_repo_sync().is_none(), "passive observation initialized a curated gate");
        self.receipt["passive_observation_did_not_initialize"] = json!(true);
        self.a = Some(Embedded::launch(home).await?);
        control.checkpoint("a_started").await?;
        let before = native()?;
        let generation = before.generation.context("native worker has no generation")?;
        self.generation = Some(generation);
        validate_generation(before, generation)?;
        ensure!(before.native == CuratedSyncNativeCompletion::Running && before.native_handle_finished == Some(false)
            && before.operation.is_none(), "A did not observe actual held native work");
        self.receipt["generation"] = json!(generation);
        let a = self.a.as_mut().context("A missing")?;
        let a_thread = a.thread(home).await?;
        a.metadata(home, "version1").await?;
        a.ready(&a_thread, "probe-old").await?;
        a.tool(&a_thread, "probe-old", "version1").await?;
        self.receipt["a_old_typed_metadata_skill_mcp"] = json!(true);
        a.close().await?;
        exact_counts(a.scope.wait().await, 0)?;
        validate_generation(native()?, generation)?;
        ensure!(native()?.native_handle_finished == Some(false), "A local shutdown did not leave held native generation alive");
        control.checkpoint("a_closed").await?;

        if case == Case::Replay { native_finished(generation).await?; }
        if case == Case::Race { control.checkpoint("race_start").await?; }
        self.b = Some(Embedded::launch(home).await?);
        let b = self.b.as_mut().context("B missing")?;
        let (thread, turn) = if case == Case::Pending {
            let thread = b.thread(home).await?;
            b.metadata(home, "version1").await?;
            b.ready(&thread, "probe-old").await?;
            b.tool(&thread, "probe-old", "version1").await?;
            let turn = b.turn(&thread).await?;
            control.checkpoint("b_ready").await?;
            (thread, Some(turn))
        } else { (String::new(), None) };
        native_finished(generation).await?;
        let activity = callback_finished(&b.scope).await?;
        self.receipt["b_before_final_wait"] = activity_value(activity);
        let thread = if case == Case::Pending { thread } else { b.thread(home).await? };
        // Ready precedes requests that can independently drive MCP refresh.
        b.ready(&thread, "probe-new").await?;
        b.metadata(home, "version2").await?;
        b.tool(&thread, "probe-new", "version2").await?;
        let turn = match turn {
            Some(turn) => turn,
            None => {
                let turn = b.turn(&thread).await?;
                control.checkpoint("b_ready").await?;
                turn
            }
        };
        b.interrupt(&thread, &turn).await?;
        let hook = timeout(OBSERVE, async {
            loop {
                match tokio::fs::read_to_string(home.join("hook.log")).await {
                    Ok(contents) if !contents.is_empty() => return Ok::<_, anyhow::Error>(contents),
                    Ok(_) => {},
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                    Err(error) => return Err(error.into()),
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.context("B Interrupt hook deadline")??;
        ensure!(hook.lines().collect::<Vec<_>>() == ["version2"], "hook invocation was old, duplicated, or unexpected");
        self.receipt["b_thread"] = json!(thread);
        self.receipt["b_turn"] = json!(turn);
        self.receipt["b_new_typed_metadata_skill_mcp"] = json!(true);
        self.receipt["b_interrupt_hook_version"] = json!("version2");
        self.receipt["hook_invocations"] = json!(1);
        // This releases the held model fixture; final success still requires
        // local shutdown, exact joins, the receipt write, and `complete` below.
        control.checkpoint("behavior_verified").await?;
        Ok(())
    }

    async fn finish(&mut self) -> Result<()> {
        let mut result = Ok(());
        if let Some(a) = &mut self.a { result = combine(result, a.close().await); }
        if let Some(b) = &mut self.b { result = combine(result, b.close().await); }
        if let Some(a) = &self.a {
            let observed = a.scope.wait().await;
            self.receipt["a_final"] = counts(observed);
            result = combine(result, exact_counts(observed, 0));
        }
        if let Some(b) = &self.b {
            let observed = b.scope.wait().await;
            self.receipt["b_final"] = counts(observed);
            result = combine(result, exact_counts(observed, 1));
        }
        let deadline = Instant::now() + Duration::from_secs(30);
        let observed = CuratedProcessShutdown::begin(deadline).wait_until(deadline).await;
        let clean = observed.is_complete() && observed.native.native == CuratedSyncNativeCompletion::Joined
            && observed.native.sync_succeeded() && observed.native.generation == self.generation
            && !observed.native.quarantined && observed.native.unexpected_handles == 0;
        self.receipt["curated_process_final"] = json!({"clean": clean,
            "native_joined": observed.native.native == CuratedSyncNativeCompletion::Joined,
            "sync_succeeded": observed.native.sync_succeeded(), "generation": observed.native.generation,
            "quarantined": observed.native.quarantined, "unexpected_handles": observed.native.unexpected_handles,
            "remaining_registered_callbacks": counts(observed.callbacks)});
        // Local waits retired clean A/B scopes; their own receipts above retain
        // exact action counts. The global drain only sees remaining scopes.
        result = combine(result, if clean { exact_counts(observed.callbacks, 0) }
            else { Err(anyhow::anyhow!("final curated native/callback proof was incomplete")) });
        result
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires one fresh parent-owned child ELF and real Git/model/MCP fixtures"]
async fn production_replacement_child() -> Result<()> {
    let home = PathBuf::from(std::env::var_os("CODEX_TEST_CURATED_REPLACEMENT_HOME")
        .context("curated replacement child sentinel missing")?);
    ensure!(home.is_absolute() && home.is_dir(), "fixture home must be an existing absolute directory");
    let case = Case::parse(&std::env::var("CODEX_TEST_CURATED_REPLACEMENT_CASE")?)?;
    let mut control = Control::connect().await?;
    let mut trial = Trial::default();
    let operation = trial.exercise(&home, case, &mut control).await;
    let cleanup = trial.finish().await;
    trial.receipt["operation_ok"] = json!(operation.is_ok());
    trial.receipt["cleanup_ok"] = json!(cleanup.is_ok());
    trial.receipt["passed"] = json!(operation.is_ok() && cleanup.is_ok());
    let receipt = async {
        let mut serialized = serde_json::to_vec_pretty(&trial.receipt)?;
        serialized.push(b'\n');
        tokio::fs::write(home.join("curated-replacement-child.json"), serialized).await?;
        Ok::<_, anyhow::Error>(())
    }.await;
    combine(combine(operation, cleanup), receipt)?;
    control.checkpoint("complete").await
}
