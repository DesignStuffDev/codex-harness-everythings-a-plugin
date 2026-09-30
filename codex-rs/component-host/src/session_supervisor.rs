use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use codex_component_api::COMPONENT_API_VERSION;
use serde_json::Value;
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
use crate::process::ChildGuard;
use crate::process::FORCE_REAP_GRACE;
use crate::process::plugin_command;
use crate::process::read_frame;
use crate::process::write_frame;
use crate::session::CALL_SLOTS;
use crate::session::ComponentSession;
use crate::session::PendingState;
use crate::session::SessionInner;
use crate::session::Shared;
use crate::session_wire::Header;
use crate::session_wire::MessageReader;
use crate::session_wire::Outgoing;
use crate::session_wire::SessionComponent;
use crate::session_wire::write_messages;

impl ComponentBinding {
    /// Connect a persistent service using an explicit, backwards-compatible
    /// handshake extension. A one-invocation plugin cannot accidentally opt in.
    /// On Linux the spawning OS thread must outlive this session's child; use a
    /// long-lived runtime worker because parent-death signalling follows that thread.
    /// Returned startup errors follow direct-child reaping, unless explicitly
    /// reported unconfirmed. Dropping this future signals its owned startup task
    /// to cancel and reap; it does not acknowledge cleanup to the absent waiter.
    pub async fn connect(&self) -> Result<ComponentSession> {
        let binding = self.clone();
        let (cancel, cancelled) = watch::channel(false);
        let _cancellation = StartupCancellation(cancel);
        tokio::spawn(async move { binding.connect_owned(cancelled).await })
            .await
            .context("persistent startup supervisor failed; direct-child reaping is unconfirmed")?
    }

    async fn connect_owned(
        &self,
        mut cancelled: watch::Receiver<bool>,
    ) -> Result<ComponentSession> {
        let deadline = Duration::from_millis(self.timeout_ms);
        // Keep this owner outside the cancellable startup future. Even a caller
        // abandoning connect leaves this task alive to kill and wait explicitly.
        let mut guard = ChildGuard(None);
        let initialized = tokio::select! {
            biased;
            _ = cancelled.wait_for(|cancelled| *cancelled) => {
                Err(anyhow::anyhow!("persistent component startup cancelled"))
            }
            initialized = async {
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
                guard.0 = Some(command.spawn().context("start persistent component")?);
                let child = guard.0.as_mut().context("persistent component child missing")?;
                let mut stdin = child.stdin.take().context("persistent component stdin missing")?;
                let mut stdout = BufReader::new(child.stdout.take().context("persistent component stdout missing")?);
                timeout(deadline, async {
                    write_frame(
                        &mut stdin,
                        json!({
                            "type":"initialize", "api_version": COMPONENT_API_VERSION,
                            "plugin_id":self.plugin_id, "config":self.config, "state_dir":self.state_dir,
                            "session":{"mode":"multiplexed","version":1},
                        }),
                    ).await?;
                    let ready = read_frame(&mut stdout).await?;
                    ensure!(
                        ready["type"] == "ready"
                            && ready["api_version"] == COMPONENT_API_VERSION
                            && ready["session"] == json!({"mode":"multiplexed","version":1}),
                        "component does not support multiplexed session version 1"
                    );
                    Ok::<(), anyhow::Error>(())
                }).await.context("persistent component handshake timed out")??;
                Ok((stdin, stdout))
            } => initialized,
        };
        let (stdin, stdout) = match initialized {
            Ok(pipes) => pipes,
            Err(error) => {
                let cleanup = timeout(FORCE_REAP_GRACE, guard.terminate_and_reap())
                    .await
                    .context("persistent startup cleanup exceeded 2 seconds")
                    .and_then(std::convert::identity);
                return Err(match cleanup {
                    Ok(()) => error,
                    Err(cleanup) => error.context(format!(
                        "persistent startup failed; direct-child reaping is unconfirmed: {cleanup:#}"
                    )),
                });
            }
        };

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
                deadline,
            )
            .await
        });
        let completion_shared = Arc::clone(&shared);
        tokio::spawn(async move {
            let outcome = match supervisor.await {
                Ok(result) => result.map_err(|error| format!("component connection failed; accepted operation outcomes may be unknown: {error:#}")),
                Err(error) => Err(format!("component supervisor failed; direct-child reaping is unconfirmed and accepted operation outcomes are unknown: {error}")),
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
            }),
        })
    }
}

struct StartupCancellation(watch::Sender<bool>);

impl Drop for StartupCancellation {
    fn drop(&mut self) {
        self.0.send_replace(true);
    }
}

struct Tasks {
    reader: Option<JoinHandle<Result<()>>>,
    writer: Option<JoinHandle<Result<()>>>,
}

impl Tasks {
    async fn stop_and_join(&mut self) -> Result<()> {
        for task in [&self.reader, &self.writer].into_iter().flatten() {
            task.abort();
        }
        let mut failure = None;
        for slot in [&mut self.reader, &mut self.writer] {
            if let Some(task) = slot.as_mut() {
                let result = task.await;
                slot.take();
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => failure = Some(error),
                    Err(error) => {
                        if !error.is_cancelled() {
                            failure = Some(anyhow::Error::new(error));
                        }
                    }
                }
            }
        }
        match failure {
            Some(error) => Err(error).context("persistent component worker failed during cleanup"),
            None => Ok(()),
        }
    }
}

impl Drop for Tasks {
    fn drop(&mut self) {
        for task in [&self.reader, &self.writer].into_iter().flatten() {
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
    shutdown: watch::Receiver<bool>,
    shared: Arc<Shared>,
    shutdown_grace: Duration,
) -> Result<()> {
    let reader_shared = Arc::clone(&shared);
    let mut tasks = Tasks {
        reader: Some(tokio::spawn(
            async move { receive(stdout, reader_shared).await },
        )),
        writer: Some(tokio::spawn(write_messages(stdin, regular_rx, control_rx))),
    };
    let outcome = drive(
        &mut guard,
        &mut tasks,
        regular,
        shutdown,
        &shared,
        shutdown_grace,
    )
    .await;
    shared.stop_admitting();
    // Errors and deadlines cancel protocol work, not child ownership. Kill and
    // wait while this supervisor is still alive, before publishing completion.
    let cleanup = timeout(FORCE_REAP_GRACE, async {
        let (reaped, joined) = tokio::join!(guard.terminate_and_reap(), tasks.stop_and_join());
        reaped?;
        joined
    })
    .await
    .context("persistent cleanup exceeded 2 seconds; direct-child reaping and worker joins are unconfirmed; accepted operation outcomes are unknown")
    .and_then(std::convert::identity);
    match cleanup {
        Ok(()) => outcome,
        Err(cleanup) => Err(cleanup).with_context(|| match outcome {
            Ok(()) => "persistent cleanup failed; direct-child reaping or worker joins are unconfirmed".to_owned(),
            Err(error) => format!("{error:#}; persistent cleanup failed; direct-child reaping or worker joins are unconfirmed"),
        }),
    }
}

async fn drive(
    guard: &mut ChildGuard,
    tasks: &mut Tasks,
    regular: mpsc::Sender<Outgoing>,
    mut shutdown: watch::Receiver<bool>,
    shared: &Shared,
    shutdown_grace: Duration,
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
        if closing && empty && !shutdown_sent {
            timeout_at(deadline, regular.send(Outgoing {
                    header: Header::Shutdown,
                    value: Value::Null,
                    after_sent: None,
                    sent: None,
                }))
                .await
                .context("persistent component shutdown enqueue timed out; accepted operation outcomes are unknown")?
                .context("component shutdown writer closed")?;
            shutdown_sent = true;
        }
        tokio::select! {
            result = tasks.reader.as_mut().context("component response reader missing")? => {
                tasks.reader.take();
                result.context("component response reader failed")??;
                ensure!(shutdown_sent, "component acknowledged an unsolicited shutdown");
                let child = guard.0.as_mut().context("component child missing")?;
                let status = timeout_at(deadline, child.wait()).await
                    .context("persistent component exit timed out")??;
                guard.0.take();
                ensure!(status.success(), "persistent component exited unsuccessfully: {status}");
                return Ok(());
            }
            result = tasks.writer.as_mut().context("component request writer missing")? => {
                tasks.writer.take();
                result.context("component request writer failed")??;
                bail!("persistent component request writer stopped unexpectedly");
            }
            _ = shutdown.changed(), if !closing => {
                closing = true;
                shared.stop_admitting();
                deadline = Instant::now() + shutdown_grace;
            }
            _ = shared.changed.notified() => {},
            _ = sleep_until(deadline), if closing => {
                bail!("persistent component shutdown timed out; accepted operation outcomes are unknown");
            }
        }
    }
}

async fn receive(stdout: BufReader<ChildStdout>, shared: Arc<Shared>) -> Result<()> {
    let mut reader = MessageReader::new(stdout);
    while let Some(message) = reader.next().await? {
        let (id, result) = match message.header {
            Header::Result { id } => (
                id,
                Ok(message
                    .payload
                    .context("component result payload missing")?),
            ),
            Header::Error { id, message } => (id, Err(message)),
            Header::ShutdownComplete => {
                ensure!(
                    shared
                        .pending
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .calls
                        .is_empty(),
                    "component shut down with pending requests"
                );
                return Ok(());
            }
            Header::Request { .. } | Header::Shutdown => {
                bail!("unexpected frame from persistent component")
            }
        };
        let pending = shared
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .calls
            .remove(&id)
            .with_context(|| format!("unexpected component response id {id}"))?;
        let _ = pending.response.send(result);
        shared.changed.notify_one();
    }
    bail!("persistent component disconnected before shutdown acknowledgement")
}

#[cfg(test)]
#[path = "session_supervisor_tests.rs"]
mod tests;
