//! Host-side process ownership for explicitly negotiated dependency sessions.

use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use serde_json::json;
use tokio::io::BufReader;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use tokio::sync::Notify;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::time::sleep_until;
use tokio::time::timeout;
use tokio::time::timeout_at;

use crate::ComponentBinding;
use crate::broker_api::BROKER_CAPACITY;
use crate::broker_completion::ParentCompletions;
use crate::broker_dispatch::BrokerDispatcher;
use crate::broker_host::BrokerHost;
use crate::broker_host::HostDependencyRegistry;
use crate::broker_wire::BrokerIncoming;
use crate::broker_wire::BrokerMessageReader;
use crate::broker_writer::BrokerWriterControl;
use crate::broker_writer::ScopedOutgoing;
use crate::broker_writer::write_broker_messages;
use crate::process::ChildGuard;
use crate::process::plugin_command;
use crate::process::read_frame;
use crate::process::write_frame;
use crate::session::CALL_SLOTS;
use crate::session::ComponentSession;
use crate::session::PendingState;
use crate::session::SessionInner;
use crate::session::Shared;
use crate::session::ensure_persistent_call_allowed;
use crate::session_wire::Header;
use crate::session_wire::Outgoing;
use crate::session_wire::SessionComponent;

impl ComponentBinding {
    /// Start a persistent component with only this binding's explicitly registered
    /// host services. A peer must acknowledge the broker before any request is sent.
    pub async fn connect_with_broker(
        &self,
        registry: HostDependencyRegistry,
    ) -> Result<ComponentSession> {
        ensure_persistent_call_allowed()?;
        let deadline = Duration::from_millis(self.timeout_ms);
        timeout(deadline, tokio::fs::create_dir_all(&self.state_dir)).await??;
        let mut command = plugin_command(self);
        command
            .args(&self.args)
            .current_dir(&self.package_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .stderr(
                if matches!(self.spec.kind.as_str(), "auth" | "attachment_store") {
                    Stdio::null()
                } else {
                    Stdio::inherit()
                },
            );
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .spawn()
            .context("start persistent dependency component")?;
        let mut stdin = child
            .stdin
            .take()
            .context("persistent component stdin missing")?;
        let mut stdout = BufReader::new(
            child
                .stdout
                .take()
                .context("persistent component stdout missing")?,
        );
        let guard = ChildGuard(Some(child));
        let broker = BrokerHost::new(registry);
        timeout(deadline, async {
            write_frame(
                &mut stdin,
                json!({
                    "type":"initialize", "api_version":1, "plugin_id":self.plugin_id,
                    "config":self.config, "state_dir":self.state_dir,
                    "session":{"mode":"multiplexed","version":1},
                    "extensions":{"dependency_broker":broker.offer()},
                }),
            )
            .await?;
            let ready = read_frame(&mut stdout).await?;
            ensure!(
                ready["type"] == "ready"
                    && ready["api_version"] == 1
                    && ready["session"] == json!({"mode":"multiplexed","version":1}),
                "component does not support multiplexed session version 1"
            );
            let acknowledgement =
                serde_json::from_value(ready["extensions"]["dependency_broker"].clone()).map_err(
                    |_| anyhow::anyhow!("component does not support dependency broker version 1"),
                )?;
            broker.acknowledge(acknowledgement)?;
            Ok::<(), anyhow::Error>(())
        })
        .await
        .context("dependency component handshake timed out")??;

        let (regular, regular_rx) = mpsc::channel(CALL_SLOTS);
        let (control, control_rx) = mpsc::channel(CALL_SLOTS);
        let (shutdown, shutdown_rx) = watch::channel(false);
        let (completion, _) = watch::channel(None);
        let shared = Arc::new(Shared {
            pending: Mutex::new(PendingState::default()),
            regular_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
            control_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
            closing: AtomicBool::new(false),
            changed: Notify::new(),
            completion,
        });
        let supervisor_shared = Arc::clone(&shared);
        let supervisor_regular = regular.clone();
        let supervisor_broker = broker.clone();
        let supervisor = tokio::spawn(async move {
            supervise(
                guard,
                stdin,
                stdout,
                regular_rx,
                control_rx,
                supervisor_regular,
                shutdown_rx,
                supervisor_shared,
                supervisor_broker,
                deadline,
            )
            .await
        });
        let completion_shared = Arc::clone(&shared);
        tokio::spawn(async move {
            let outcome = match supervisor.await {
                Ok(result) => result.map_err(|_| "dependency component connection failed; accepted operation outcomes may be unknown".to_owned()),
                Err(_) => Err("dependency component supervisor failed; accepted operation outcomes are unknown".to_owned()),
            };
            completion_shared.finish(outcome);
        });
        Ok(ComponentSession {
            inner: Arc::new(SessionInner {
                component: SessionComponent {
                    kind: self.spec.kind.clone(),
                    name: self.spec.name.clone(),
                },
                regular,
                control,
                shared,
                shutdown,
                timeout: deadline,
                broker: Some(broker),
            }),
        })
    }
}

struct Tasks {
    reader: Option<JoinHandle<Result<()>>>,
    writer: Option<JoinHandle<Result<()>>>,
    scoped: Option<JoinHandle<Result<()>>>,
    dispatcher: BrokerDispatcher,
    parents: ParentCompletions,
    writer_control: BrokerWriterControl,
}

impl Drop for Tasks {
    fn drop(&mut self) {
        // A forced outer abort cannot claim a graceful join. Cancellation keeps
        // decoder/handler ownership visible until their monitor observes a join.
        self.dispatcher.cancel_all();
        self.writer_control.cancel();
        for task in [&self.reader, &self.writer, &self.scoped]
            .into_iter()
            .flatten()
        {
            task.abort();
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn supervise(
    mut guard: ChildGuard,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    regular_rx: mpsc::Receiver<Outgoing>,
    control_rx: mpsc::Receiver<Outgoing>,
    regular: mpsc::Sender<Outgoing>,
    mut shutdown: watch::Receiver<bool>,
    shared: Arc<Shared>,
    broker: BrokerHost,
    grace: Duration,
) -> Result<()> {
    let (scoped_tx, scoped_rx) = mpsc::channel(CALL_SLOTS);
    let (responses, response_rx) = mpsc::channel(BROKER_CAPACITY);
    let (requests, request_rx) = mpsc::channel(BROKER_CAPACITY);
    let (cancellations, cancellation_rx) = mpsc::channel(BROKER_CAPACITY);
    drop((requests, cancellations));
    let dispatcher = BrokerDispatcher::new(broker.clone(), responses);
    let parents = ParentCompletions::new();
    let reader_dispatcher = dispatcher.clone();
    let reader_parents = parents.clone();
    let reader_shared = Arc::clone(&shared);
    let writer_control = BrokerWriterControl::new();
    let mut tasks = Tasks {
        reader: Some(tokio::spawn(async move {
            receive(stdout, reader_shared, reader_dispatcher, reader_parents).await
        })),
        writer: Some(tokio::spawn(write_broker_messages(
            stdin,
            scoped_rx,
            control_rx,
            request_rx,
            response_rx,
            cancellation_rx,
            writer_control.clone(),
        ))),
        scoped: Some(tokio::spawn(scope_messages(regular_rx, scoped_tx, broker))),
        dispatcher,
        parents,
        writer_control,
    };
    let outcome = drive(
        &mut tasks,
        &mut guard,
        &regular,
        &mut shutdown,
        &shared,
        grace,
    )
    .await;
    shared.stop_admitting();
    tasks.dispatcher.cancel_all();
    tasks.writer_control.cancel();
    let teardown = async {
        let handles = [tasks.reader.take(), tasks.scoped.take()];
        for task in handles.iter().flatten() {
            task.abort();
        }
        for task in handles.into_iter().flatten() {
            let _ = task.await;
        }
        // A controlled writer stop joins any uncapped serializer/spool read and
        // resolves response flush acknowledgements before parent slots release.
        let writer_outcome = if let Some(writer) = tasks.writer.take() {
            writer
                .await
                .context("dependency writer cleanup task failed")
                .and_then(|result| result)
        } else {
            Ok(())
        };
        guard.terminate_and_reap().await?;
        tasks.dispatcher.drained().await;
        tasks.parents.drained().await;
        tasks.dispatcher.release_all();
        writer_outcome
            .context("dependency writer cleanup was not confirmed; accepted outcomes are unknown")
    };
    timeout(Duration::from_secs(2), teardown).await
        .context("dependency teardown did not confirm child reaping and handler joins; accepted outcomes are unknown")??;
    outcome
}

async fn drive(
    tasks: &mut Tasks,
    guard: &mut ChildGuard,
    regular: &mpsc::Sender<Outgoing>,
    shutdown: &mut watch::Receiver<bool>,
    shared: &Arc<Shared>,
    grace: Duration,
) -> Result<()> {
    let mut closing = false;
    let mut shutdown_sent = false;
    let mut deadline = Instant::now() + Duration::from_secs(365 * 24 * 60 * 60);
    loop {
        let empty = shared
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .calls
            .is_empty();
        if closing
            && empty
            && !shutdown_sent
            && tasks.parents.is_idle()
            && tasks.dispatcher.stop_if_idle()
        {
            timeout_at(
                deadline,
                regular.send(Outgoing {
                    header: Header::Shutdown,
                    value: serde_json::Value::Null,
                    after_sent: None,
                    sent: None,
                }),
            )
            .await
            .context("component shutdown admission timed out")?
            .context("component shutdown writer closed")?;
            shutdown_sent = true;
        }
        tokio::select! {
            result = async { tasks.reader.as_mut().context("dependency reader missing")?.await.context("dependency reader task failed")? } => {
                tasks.reader.take();
                result?;
                ensure!(shutdown_sent, "component acknowledged unsolicited shutdown");
                let child = guard.0.as_mut().context("component child missing")?;
                let status = timeout_at(deadline, child.wait()).await.context("dependency component exit timed out")??;
                guard.0.take();
                ensure!(status.success(), "dependency component exited unsuccessfully");
                return Ok(());
            }
            result = async { tasks.writer.as_mut().context("dependency writer missing")?.await.context("dependency writer task failed")? } => {
                tasks.writer.take();
                result?;
                bail!("dependency request writer stopped unexpectedly");
            }
            result = async { tasks.scoped.as_mut().context("scoped writer missing")?.await.context("scoped writer task failed")? } => {
                tasks.scoped.take();
                result?;
                bail!("scoped request writer stopped unexpectedly");
            }
            _ = shutdown.changed(), if !closing => {
                closing = true;
                shared.stop_admitting();
                deadline = Instant::now() + grace;
            }
            _ = shared.changed.notified() => {},
            _ = tasks.dispatcher.drained(), if closing && !tasks.dispatcher.is_idle() => {},
            _ = tasks.parents.drained(), if closing && !tasks.parents.is_idle() => {},
            _ = sleep_until(deadline), if closing => bail!("dependency shutdown timed out; accepted outcomes are unknown"),
        }
    }
}

async fn scope_messages(
    mut incoming: mpsc::Receiver<Outgoing>,
    outgoing: mpsc::Sender<ScopedOutgoing>,
    broker: BrokerHost,
) -> Result<()> {
    while let Some(message) = incoming.recv().await {
        let scope = match &message.header {
            Header::Request { id, .. } => broker.scope_for_parent(*id),
            Header::Result { .. }
            | Header::Error { .. }
            | Header::Shutdown
            | Header::ShutdownComplete => None,
        };
        outgoing
            .send(ScopedOutgoing { message, scope })
            .await
            .context("scoped writer closed")?;
    }
    Ok(())
}

async fn receive(
    stdout: BufReader<ChildStdout>,
    shared: Arc<Shared>,
    dispatcher: BrokerDispatcher,
    parents: ParentCompletions,
) -> Result<()> {
    let mut reader = BrokerMessageReader::new(stdout, true);
    while let Some(message) = reader.next().await? {
        let (id, result) = match message {
            BrokerIncoming::Ordinary {
                message,
                scope: None,
            } => match message.header {
                Header::Result { id } => {
                    (id, Ok(message.payload.context("component result missing")?))
                }
                Header::Error { id, .. } => {
                    (id, Err("dependency component request failed".to_owned()))
                }
                Header::ShutdownComplete => {
                    ensure!(
                        shared
                            .pending
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .calls
                            .is_empty()
                            && dispatcher.is_idle()
                            && parents.is_idle(),
                        "component shutdown with pending dependency work"
                    );
                    return Ok(());
                }
                Header::Request { .. } | Header::Shutdown => {
                    bail!("unexpected dependency peer frame")
                }
            },
            BrokerIncoming::Request { header, payload } => {
                dispatcher.dispatch(header, payload)?;
                continue;
            }
            BrokerIncoming::Cancel { id } => {
                dispatcher.cancel(id);
                continue;
            }
            BrokerIncoming::Ordinary { scope: Some(_), .. }
            | BrokerIncoming::Result { .. }
            | BrokerIncoming::Error { .. } => bail!("unexpected dependency frame direction"),
        };
        parents.complete(id, result, Arc::clone(&shared), dispatcher.clone())?;
    }
    bail!("dependency component disconnected before shutdown acknowledgement")
}
