//! Bounded, cancellable transport for independently installed component processes.

use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use codex_component_api::MAX_FRAME_BYTES;
use serde_json::Value;
use serde_json::json;
use tokio::io::AsyncBufRead;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::Instant;
use tokio::time::timeout_at;

use crate::ComponentBinding;

#[path = "process_control.rs"]
mod control;

pub use control::ComponentShutdown;
pub use control::ComponentStartup;
use control::ControlReceivers;
pub(crate) use control::FORCE_REAP_GRACE;
use control::InvocationPolicy;
use control::RunningTask;

const SHUTDOWN_GRACE: Duration = Duration::from_millis(500);

/// One incremental component event, or the final result after successful shutdown.
#[derive(Clone, Debug, PartialEq)]
pub enum StreamFrame {
    Event(Value),
    Done(Value),
}

/// A single invocation. Dropping it (or an in-progress `next`) cancels its child.
///
/// At most one unread frame is queued. Ordinary invocation deadlines remain
/// active while the consumer is idle. Service streams instead bound startup
/// and shutdown, with no execution deadline after readiness.
#[derive(Debug)]
pub struct ComponentStream {
    receiver: mpsc::Receiver<StreamFrame>,
    task: Option<RunningTask>,
    shutdown: ComponentShutdown,
}

impl ComponentBinding {
    /// Invoke a component, consuming any incremental events until its result.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let mut stream = self.stream(method, params).await?;
        loop {
            match stream.next().await? {
                StreamFrame::Event(_) => {}
                StreamFrame::Done(result) => return Ok(result),
            }
        }
    }

    /// Start a fresh plugin process and return after its handshake and request.
    /// Persistent component state belongs in the supplied plugin state directory.
    /// On Linux, the OS thread spawning the child must outlive it: parent-death
    /// termination follows that thread. Use a long-lived runtime worker.
    pub async fn stream(&self, method: &str, params: Value) -> Result<ComponentStream> {
        self.start_stream(method, params).ready().await
    }
}

impl ComponentStream {
    /// Clone this before polling `next`; retain the same pending `next` future
    /// across a graceful shutdown request. Dropping that future still force-kills.
    pub fn shutdown_handle(&self) -> ComponentShutdown {
        self.shutdown.clone()
    }

    pub async fn next(&mut self) -> Result<StreamFrame> {
        // Move the cancellation guard into this future, so cancelling a pending
        // read terminates the invocation even if the stream itself is retained.
        let task = self.task.take().context("component stream has finished")?;
        match self.receiver.recv().await {
            Some(StreamFrame::Event(event)) => {
                self.task = Some(task);
                Ok(StreamFrame::Event(event))
            }
            Some(StreamFrame::Done(result)) => {
                task.join().await?;
                Ok(StreamFrame::Done(result))
            }
            None => {
                task.join().await?;
                bail!("component closed its stream without a result")
            }
        }
    }
}

/// Own the child before the first await, including during the ready handshake.
pub(crate) struct ChildGuard(pub(crate) Option<Child>);

impl ChildGuard {
    pub(crate) async fn terminate_and_reap(&mut self) -> Result<()> {
        if let Some(child) = self.0.as_mut() {
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                // SAFETY: negation targets the process group we created.
                unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
            }
            // Always attempt wait, including if killing races a natural exit.
            let _ = child.start_kill();
            child.wait().await.context("reap terminated component")?;
            self.0.take();
        }
        Ok(())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                // The plugin is its own process-group leader. Cancel its native
                // subprocesses as well as the entrypoint (for example app-server).
                // SAFETY: kill accepts any pid; negation targets our child's group.
                unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
            }
            let _ = child.start_kill();
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    let _ = child.wait().await;
                });
            }
            // kill_on_drop still terminates when the runtime stops. This
            // fallback cannot acknowledge reaping before host exit; intentional
            // shutdown must await terminate_and_reap while the runtime is alive.
        }
    }
}

async fn supervise(
    binding: ComponentBinding,
    method: String,
    params: Value,
    started: oneshot::Sender<()>,
    sender: mpsc::Sender<StreamFrame>,
    control: ControlReceivers,
) -> Result<()> {
    let ControlReceivers {
        graceful: mut shutdown_requested,
        mut force,
        completed,
        policy,
    } = control;
    let deadline = Instant::now() + Duration::from_millis(binding.timeout_ms);
    let mut shutdown_timer = shutdown_requested.clone();
    // Protocol work may be cancelled independently. Keep the child owned here
    // until explicit termination and wait complete, including before readiness.
    let mut guard = ChildGuard(None);
    let (mut payload_owner, payload_client) =
        if binding.spec.kind == "model_transport" && binding.spec.contract_version == 2 {
            let (owner, client) = crate::payload_work::JsonWorkOwner::start();
            (Some(owner), Some(client))
        } else {
            (None, None)
        };
    let mut transport = crate::streaming_wire::InvocationTransport::new(&binding, payload_client);
    let invocation = async {
        let (mut stdin, mut stdout) = timeout_at(deadline, async {
            tokio::fs::create_dir_all(&binding.state_dir)
                .await
                .context("create component state directory")?;
            let mut command = plugin_command(&binding);
            // Auth plugins handle credential material; their diagnostics must not
            // bypass the adapter's sanitized error surface through inherited stderr.
            let stderr = if matches!(binding.spec.kind.as_str(), "auth" | "attachment_store") {
                Stdio::null()
            } else {
                Stdio::inherit()
            };
            command
                .args(&binding.args)
                .current_dir(&binding.package_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(stderr)
                .kill_on_drop(true);
            #[cfg(unix)]
            command.process_group(0);
            let child = command.spawn().with_context(|| {
                format!("start component plugin {}", binding.entrypoint.display())
            })?;
            guard.0 = Some(child);
            let child = guard.0.as_mut().context("component child missing")?;
            let mut stdin = child.stdin.take().context("component stdin missing")?;
            let mut stdout =
                BufReader::new(child.stdout.take().context("component stdout missing")?);

            transport
                .initialize(&binding, &mut stdin, &mut stdout)
                .await?;
            transport
                .send_request(
                    &mut stdin,
                    json!({
                        "type": "request",
                        "id": 1,
                        "component": {"kind": binding.spec.kind, "name": binding.spec.name},
                        "method": method,
                        "params": params,
                    }),
                )
                .await?;
            Ok::<_, anyhow::Error>((stdin, stdout))
        })
        .await
        .context("component startup timed out")??;
        started
            .send(())
            .map_err(|()| anyhow::anyhow!("component invocation cancelled"))?;

        let mut shutdown_sent = false;
        let result = loop {
            // Keep partially consumed frame bytes in the same read future when
            // a shutdown request arrives. Cancelling read_frame would lose them.
            let mut frame = {
                let reading = transport.receive(&mut stdout);
                tokio::pin!(reading);
                loop {
                    tokio::select! {
                        result = &mut reading => break result?,
                        changed = shutdown_requested.changed(), if !shutdown_sent => {
                            changed.context("component shutdown controller closed")?;
                            let requested = shutdown_requested.borrow().is_some();
                            if requested {
                                write_frame(&mut stdin, json!({"type":"shutdown"})).await?;
                                shutdown_sent = true;
                            }
                        }
                    }
                }
            };
            if frame.get("id").and_then(Value::as_u64) != Some(1) {
                bail!("component frame has unexpected request id; expected 1");
            }
            match frame.get("type").and_then(Value::as_str) {
                Some("event") => {
                    let event = frame.as_object_mut().and_then(|frame| frame.remove("event"))
                        .context("component event payload missing")?;
                    sender
                        .send(StreamFrame::Event(event))
                        .await
                        .context("component event consumer closed")?;
                }
                Some("result") => {
                    break frame.as_object_mut().and_then(|frame| frame.remove("result"))
                        .context("component result payload missing")?;
                }
                Some("error") => {
                    let message = frame
                        .get("message")
                        .and_then(Value::as_str)
                        .context("component error message missing")?;
                    bail!("component returned an error: {message}");
                }
                Some(kind) => bail!("unexpected component frame type: {kind}"),
                None => bail!("component frame type missing"),
            }
        };
        let shutdown = async {
            if !shutdown_sent {
                write_frame(&mut stdin, json!({"type": "shutdown"})).await?;
            }
            drop(stdin);
            let child = guard.0.as_mut().context("component child missing")?;
            let (status, ()) = tokio::try_join!(
                async { child.wait().await.context("wait for component shutdown") },
                transport.finish(&mut stdout)
            )?;
            guard.0.take();
            if !status.success() {
                bail!("component exited unsuccessfully: {status}");
            }
            Ok::<(), anyhow::Error>(())
        };
        let normal_exit_expired = async {
            if let InvocationPolicy::Service { exit_grace } = policy {
                // A normal service result begins a full cleanup phase. Its
                // deadline cannot be extended by a later graceful request.
                tokio::time::sleep(exit_grace).await;
                return;
            }
            let graceful_requested = shutdown_requested.borrow().is_some();
            if !graceful_requested {
                tokio::select! {
                    _ = tokio::time::sleep(SHUTDOWN_GRACE) => return,
                    _ = shutdown_requested.changed() => {}
                }
            }
            // A graceful request can arrive even after the result. Its shared
            // supervisor deadline replaces the ordinary short exit allowance.
            std::future::pending::<()>().await;
        };
        tokio::select! {
            result = shutdown => result?,
            _ = normal_exit_expired => bail!("component shutdown timed out; accepted writes may not have completed and durability is unknown"),
        }
        Ok::<Value, anyhow::Error>(result)
    };
    let supervised = async {
        let shutdown_expired = async {
            let deadline = match shutdown_timer.wait_for(Option::is_some).await {
                Ok(value) => *value,
                Err(_) => None,
            };
            if let Some(deadline) = deadline {
                tokio::time::sleep_until(deadline).await;
            } else {
                std::future::pending::<()>().await;
            }
        };
        tokio::select! {
            result = invocation => result,
            _ = shutdown_expired => bail!("component graceful shutdown timed out; accepted writes may not have completed and durability is unknown"),
            _ = async {
                if force.wait_for(|requested| *requested).await.is_err() {
                    std::future::pending::<()>().await;
                }
            } => bail!("component forced termination requested; accepted writes may not have completed and durability is unknown"),
        }
    };
    let outcome = match policy {
        InvocationPolicy::Bounded => timeout_at(deadline, supervised)
            .await
            .with_context(|| {
                format!(
                    "component invocation timed out after {} ms",
                    binding.timeout_ms
                )
            })
            .and_then(std::convert::identity),
        InvocationPolicy::Service { .. } => supervised.await,
    }
    .with_context(|| format!("plugin {} method {method}", binding.plugin_id));
    let cleaned = tokio::time::timeout(FORCE_REAP_GRACE, async {
        let work = async {
            if let Some(owner) = &mut payload_owner {
                owner.stop_and_join().await?;
            }
            Ok::<(), anyhow::Error>(())
        };
        let (reaped, joined) = tokio::join!(guard.terminate_and_reap(), work);
        reaped?;
        joined?;
        Ok::<(), anyhow::Error>(())
    })
    .await
    .context(
        "component termination did not confirm reaping and payload work cleanup within 2 seconds",
    )
    .and_then(|result| result);
    completed.send_replace(Some(
        cleaned
            .as_ref()
            .copied()
            .map_err(|error| format!("{error:#}")),
    ));
    match cleaned {
        Ok(()) => {
            let result = outcome?;
            sender
                .send(StreamFrame::Done(result))
                .await
                .context("component result consumer closed")?;
            Ok(())
        }
        Err(error) => Err(error).with_context(|| {
            let prior = outcome.err().map(|error| format!("{error:#}"));
            format!("component cleanup failed; durability is unknown; prior outcome: {prior:?}")
        }),
    }
}

pub(crate) fn plugin_command(binding: &ComponentBinding) -> Command {
    #[cfg(windows)]
    if binding
        .entrypoint
        .extension()
        .is_some_and(|extension| extension == "pyz")
    {
        let mut command = Command::new("python");
        command.arg(&binding.entrypoint);
        return command;
    }
    let command = Command::new(&binding.entrypoint);
    #[cfg(target_os = "linux")]
    let command = {
        let mut command = command;
        let parent_pid = std::process::id() as libc::pid_t;
        // The current engine spawns components on long-lived runtime workers.
        // Linux ties this signal to the creating thread, so callers must retain
        // that thread for the child's lifetime, even if its future moves threads.
        // SAFETY: after fork this hook uses only libc syscalls and constructs an
        // OS error on failure. It neither allocates application data nor locks.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(
                    libc::PR_SET_PDEATHSIG,
                    libc::SIGKILL as libc::c_ulong,
                    0 as libc::c_ulong,
                    0 as libc::c_ulong,
                    0 as libc::c_ulong,
                ) == -1
                {
                    return Err(std::io::Error::last_os_error());
                }
                // Cover death between fork and arming the signal. After this
                // check the kernel handles any later creator-thread death.
                if libc::getppid() != parent_pid {
                    libc::_exit(128 + libc::SIGKILL);
                }
                Ok(())
            });
        }
        command
    };
    command
}

pub(crate) async fn write_frame<W: AsyncWrite + Unpin>(stdin: &mut W, frame: Value) -> Result<()> {
    let mut buffer = FrameBuffer(Vec::new());
    serde_json::to_writer(&mut buffer, &frame).context("encode component frame")?;
    buffer.0.push(b'\n');
    stdin
        .write_all(&buffer.0)
        .await
        .context("write component frame")?;
    stdin.flush().await.context("flush component frame")
}

/// Refuse an oversized frame during serialization, before allocating its bytes.
struct FrameBuffer(Vec<u8>);

impl std::io::Write for FrameBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() >= MAX_FRAME_BYTES - self.0.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("component request exceeds {MAX_FRAME_BYTES} byte frame limit"),
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) async fn read_frame<R: AsyncBufRead + Unpin>(stdout: &mut R) -> Result<Value> {
    let mut bytes = Vec::new();
    loop {
        let available = stdout.fill_buf().await.context("read component frame")?;
        if available.is_empty() {
            bail!("unexpected EOF from component before a complete frame");
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if count > MAX_FRAME_BYTES - bytes.len() {
            bail!("component response exceeds {MAX_FRAME_BYTES} byte frame limit");
        }
        bytes.extend_from_slice(&available[..count]);
        stdout.consume(count);
        if newline.is_some() {
            return serde_json::from_slice(&bytes).context("invalid component JSON frame");
        }
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
