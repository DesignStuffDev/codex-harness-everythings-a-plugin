//! Bounded shutdown observation with ownership independent of the waiting UI.

use std::fmt;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::io::Result as IoResult;
use std::time::Duration;

use codex_app_server::in_process::IN_PROCESS_SHUTDOWN_BUDGET;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::time::timeout_at;
use tracing::warn;

use crate::ClientCommand;
use crate::InProcessAppServerClient;

// One deadline covers command admission, the complete embedded drain, and the
// facade join. The margin is for forwarding and scheduling, not a second drain.
const FACADE_SHUTDOWN_BUDGET: Duration =
    IN_PROCESS_SHUTDOWN_BUDGET.saturating_add(Duration::from_secs(5));

/// Start on the captured owning runtime before returning the receipt future.
pub(super) fn start(
    client: InProcessAppServerClient,
) -> impl std::future::Future<Output = IoResult<()>> + Send {
    let InProcessAppServerClient {
        command_tx,
        event_rx,
        worker_handle,
        shutdown_runtime,
        file_search_factory: _,
    } = client;
    drop(event_rx);
    let (completed, completion) = oneshot::channel();
    let owner = WorkerOwner {
        handle: Some(worker_handle),
        runtime: shutdown_runtime.clone(),
    };
    let deadline = {
        // Use the owning runtime's clock even when called outside an entered
        // runtime (also preserves paused-clock behavior in lifecycle tests).
        let _entered = shutdown_runtime.enter();
        let local = Instant::now() + FACADE_SHUTDOWN_BUDGET;
        codex_utils_process::process_shutdown::current().map_or(local, |deadline| {
            local.min(Instant::from_std(deadline.graceful()))
        })
    };
    // This task intentionally outlives a cancelled receipt observer. WorkerOwner
    // also retains an abort-and-join fallback if the coordinator itself unwinds.
    shutdown_runtime.spawn(coordinate(command_tx, owner, deadline, completed));
    async move {
        completion.await.map_err(|_| IoError::other(
            "in-process shutdown coordinator stopped; worker and storage cleanup are unconfirmed"
        ))?
    }
}

struct WorkerOwner {
    handle: Option<JoinHandle<()>>,
    runtime: Handle,
}

impl Drop for WorkerOwner {
    fn drop(&mut self) {
        if let Some(worker) = self.handle.take() {
            worker.abort();
            self.runtime.spawn(async move {
                if let Err(error) = worker.await
                    && !error.is_cancelled()
                {
                    warn!("in-process shutdown fallback worker failed: {error}; runtime/storage cleanup is unconfirmed");
                }
            });
        }
    }
}

async fn coordinate(
    command_tx: mpsc::Sender<ClientCommand>,
    mut worker: WorkerOwner,
    deadline: Instant,
    completed: oneshot::Sender<IoResult<()>>,
) {
    let (response_tx, response_rx) = oneshot::channel();
    let acknowledged =
        timeout_at(deadline, async {
            command_tx.send(ClientCommand::Shutdown { response_tx }).await.map_err(|_| IoError::new(
            ErrorKind::BrokenPipe,
            "in-process shutdown command channel closed; runtime/storage cleanup is unconfirmed",
        ))?;
            response_rx.await.map_err(|_| IoError::new(
            ErrorKind::BrokenPipe,
            "in-process shutdown acknowledgement lost; runtime/storage cleanup is unconfirmed",
        ))?
        })
        .await;
    drop(command_tx);
    let mut failure = match acknowledged {
        Ok(result) => result.err(),
        Err(_) => Some(IoError::new(
            ErrorKind::TimedOut,
            "in-process shutdown deadline expired during command admission or acknowledgement; runtime/storage cleanup is unconfirmed",
        )),
    };

    // The owner always contains the worker here; keep it in the owner while
    // awaiting so coordinator cancellation cannot discard an unobserved task.
    let Some(handle) = worker.handle.as_mut() else {
        add_failure(
            &mut failure,
            IoError::other("in-process shutdown worker ownership lost; cleanup is unconfirmed"),
        );
        report(completed, failure);
        return;
    };
    match timeout_at(deadline, &mut *handle).await {
        Ok(result) => {
            worker.handle.take();
            if let Err(error) = result {
                add_failure(
                    &mut failure,
                    IoError::other(format!(
                        "in-process shutdown worker failed: {error}; runtime/storage cleanup is unconfirmed"
                    )),
                );
            }
            report(completed, failure);
        }
        Err(_) => {
            handle.abort();
            add_failure(
                &mut failure,
                IoError::new(
                    ErrorKind::TimedOut,
                    "in-process shutdown deadline expired; worker abort requested and runtime/storage cleanup is unconfirmed",
                ),
            );
            // Do not extend the public deadline. This is explicitly not a join
            // receipt. The coordinator continues owning and observing the task.
            report(completed, failure);
            let result = handle.await;
            worker.handle.take();
            if let Err(error) = result
                && !error.is_cancelled()
            {
                warn!(
                    "in-process worker failed after forced shutdown: {error}; runtime/storage cleanup is unconfirmed"
                );
            }
        }
    }
}

fn report(completed: oneshot::Sender<IoResult<()>>, failure: Option<IoError>) {
    if let Err(result) = completed.send(failure.map_or(Ok(()), Err))
        && let Err(error) = result
    {
        warn!("in-process shutdown failed after its observer was dropped: {error}");
    }
}

#[derive(Debug)]
struct ShutdownFailures {
    first: IoError,
    later: IoError,
}

impl fmt::Display for ShutdownFailures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}; additionally: {}", self.first, self.later)
    }
}

impl std::error::Error for ShutdownFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.first)
    }
}

fn add_failure(failure: &mut Option<IoError>, later: IoError) {
    *failure = Some(match failure.take() {
        Some(first) => IoError::new(first.kind(), ShutdownFailures { first, later }),
        None => later,
    });
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
