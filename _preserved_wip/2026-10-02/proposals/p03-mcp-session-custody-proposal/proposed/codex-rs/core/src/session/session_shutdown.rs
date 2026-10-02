//! Payload-free session cleanup results backed by an exact retained Tokio task.

use std::future::Future;

use codex_async_utils::RetainedTask;
use codex_async_utils::TaskJoinFailure;
use futures::FutureExt;

use super::SessionLoopTermination;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionShutdownFailure {
    McpPrewarm,
    McpRuntime,
    Persistence,
    Submission,
    LoopPanicked,
    LoopCancelled,
    LoopSpawnPanicked,
}

pub(crate) type SessionShutdownOutcome = Result<(), SessionShutdownFailure>;

pub(super) fn spawn_session_loop(
    future: impl Future<Output = SessionShutdownOutcome> + Send + 'static,
) -> SessionLoopTermination {
    // Registration precedes spawn. The shared future is only an observer; its
    // cancellation cannot discard the exact task or its original join failure.
    let owner = RetainedTask::spawn(&tokio::runtime::Handle::current(), future);
    async move {
        match owner.wait().await {
            Ok(outcome) => *outcome,
            Err(TaskJoinFailure::Panicked) => Err(SessionShutdownFailure::LoopPanicked),
            Err(TaskJoinFailure::Cancelled) => Err(SessionShutdownFailure::LoopCancelled),
            Err(TaskJoinFailure::SpawnPanicked) => Err(SessionShutdownFailure::LoopSpawnPanicked),
        }
    }
    .boxed()
    .shared()
}

#[cfg(test)]
#[path = "session_shutdown_tests.rs"]
mod tests;
