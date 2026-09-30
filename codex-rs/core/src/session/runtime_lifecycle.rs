//! Owned, idempotent cleanup for constructed sessions and their submission loops.
//!
//! Completion carries no Session or submission sender. Accepted cleanup is driven
//! independently of its waiters on the runtime that owns the session services.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use futures::FutureExt;
use tokio::runtime::Handle;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use super::SessionLoopTermination;
use super::session::Session;

pub(super) type ShutdownOutcome = Result<(), Arc<str>>;

/// One supervised operation with a retained result for all current/future waiters.
pub(super) struct ShutdownTask {
    started: AtomicBool,
    completion: watch::Sender<Option<ShutdownOutcome>>,
}

impl Default for ShutdownTask {
    fn default() -> Self {
        Self {
            started: AtomicBool::new(false),
            completion: watch::channel(None).0,
        }
    }
}

impl ShutdownTask {
    pub(super) fn start(
        &self,
        runtime: &Handle,
        operation: impl Future<Output = ShutdownOutcome> + Send + 'static,
    ) -> ShutdownWait {
        let wait = ShutdownWait(self.completion.subscribe());
        if !self.started.swap(true, Ordering::AcqRel) {
            let task = runtime.spawn(operation);
            let completion = self.completion.clone();
            runtime.spawn(async move {
                let outcome = task
                    .await
                    .unwrap_or_else(|_| Err(Arc::from("session cleanup worker failed")));
                completion.send_replace(Some(outcome));
            });
        }
        wait
    }
}

pub(super) struct ShutdownWait(watch::Receiver<Option<ShutdownOutcome>>);

impl ShutdownWait {
    pub(super) async fn wait(mut self) -> ShutdownOutcome {
        loop {
            let outcome = self.0.borrow_and_update().clone();
            if let Some(outcome) = outcome {
                return outcome;
            }
            self.0
                .changed()
                .await
                .map_err(|_| Arc::from("session cleanup completion was lost"))?;
        }
    }
}

#[derive(Default)]
pub(super) struct SessionRuntimeLifecycle {
    runtime: ShutdownTask,
    persistence: ShutdownTask,
    terminal: ShutdownTask,
}

pub(super) fn shutdown_runtime(session: &Arc<Session>) -> ShutdownWait {
    let owned = Arc::clone(session);
    session
        .runtime_lifecycle
        .runtime
        .start(&session.services.runtime_handle, async move {
            super::runtime_cleanup::run(&owned).await
        })
}

pub(super) fn shutdown_persistence(session: &Arc<Session>) -> ShutdownWait {
    let live_thread = session.live_thread().cloned();
    session
        .runtime_lifecycle
        .persistence
        .start(&session.services.runtime_handle, async move {
            if let Some(live_thread) = live_thread {
                live_thread
                    .shutdown()
                    .await
                    .map_err(|error| Arc::from(error.to_string()))?;
            }
            Ok(())
        })
}

fn finish_session(session: &Arc<Session>) -> ShutdownWait {
    let owned = Arc::clone(session);
    session
        .runtime_lifecycle
        .terminal
        .start(&session.services.runtime_handle, async move {
            let runtime = shutdown_runtime(&owned).wait().await;
            // A runtime cleanup error must not abandon the writer.
            let persistence = shutdown_persistence(&owned).wait().await;
            match (runtime, persistence) {
                (Ok(()), Ok(())) => Ok(()),
                (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
                (Err(runtime), Err(persistence)) => {
                    Err(Arc::from(format!("{runtime}; {persistence}")))
                }
            }
        })
}

/// Terminal lifetime transferred from construction to the loop supervisor.
/// It deliberately owns neither SessionIo nor the startup holder containing it.
pub(crate) struct SessionRuntimeOwner {
    session: Option<Arc<Session>>,
}

impl SessionRuntimeOwner {
    pub(super) fn new(session: Arc<Session>) -> Self {
        Self {
            session: Some(session),
        }
    }

    pub(super) fn close(mut self) -> ShutdownWait {
        let session = self.session.take().expect("runtime owner is armed");
        finish_session(&session)
    }
}

impl Drop for SessionRuntimeOwner {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = finish_session(&session);
        }
    }
}

pub(super) fn supervise_session_loop(
    handle: JoinHandle<()>,
    owner: SessionRuntimeOwner,
) -> SessionLoopTermination {
    let runtime = owner
        .session
        .as_ref()
        .expect("runtime owner is armed")
        .services
        .runtime_handle
        .clone();
    let (completion, receiver) = watch::channel(None);
    runtime.spawn(async move {
        let loop_result = handle.await;
        let cleanup = owner.close().wait().await;
        let outcome = match (loop_result, cleanup) {
            (Ok(()), outcome) => outcome,
            (Err(_), Ok(())) => Err(Arc::from("session submission loop failed")),
            (Err(_), Err(cleanup)) => Err(Arc::from(format!(
                "session submission loop failed; {cleanup}"
            ))),
        };
        if let Err(error) = &outcome {
            tracing::warn!(%error, "session terminated with cleanup errors");
        }
        completion.send_replace(Some(outcome));
    });
    async move {
        // Preserve the public unit-returning termination API. The independently
        // driven supervisor retains and reports failures before publishing it.
        let _ = ShutdownWait(receiver).wait().await;
    }
    .boxed()
    .shared()
}

/// A panicking extension must not prevent later mandatory cleanup stages.
pub(super) async fn cleanup_stage(
    name: &'static str,
    future: impl Future<Output = ()>,
    failures: &mut Vec<&'static str>,
) {
    if std::panic::AssertUnwindSafe(future)
        .catch_unwind()
        .await
        .is_err()
    {
        failures.push(name);
    }
}
