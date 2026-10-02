//! Session cleanup observations and sticky ownership of failed sessions.

use std::sync::Arc;
use std::time::Duration;

use codex_protocol::ThreadId;
use codex_protocol::error::CodexErrorDetails;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use futures::FutureExt;
use futures::StreamExt;
use futures::stream::FuturesUnordered;

use super::ThreadManager;
use super::ThreadManagerState;
use crate::codex_thread::CodexThread;
use crate::session::session::Session;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ThreadShutdownReport {
    pub completed: Vec<ThreadId>,
    pub submit_failed: Vec<ThreadId>,
    pub shutdown_failed: Vec<ThreadId>,
    pub timed_out: Vec<ThreadId>,
}

impl ThreadManagerState {
    pub(super) fn retain_removed_thread(&self, thread: &Arc<CodexThread>) {
        let mut removed = self.removed_threads.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !removed.iter().any(|existing| Arc::ptr_eq(existing, thread)) {
            removed.push(Arc::clone(thread));
        }
    }

    /// Poll already-completed exact owners without holding either registry lock.
    /// Pending and failed owners stay reachable; clean outputs can release full
    /// sessions during normal server operation instead of waiting for final exit.
    pub(crate) fn retire_completed_removed_threads(&self) {
        let snapshot = self.removed_threads.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        for thread in snapshot {
            if matches!(thread.io.session_loop_termination.clone().now_or_never(), Some(Ok(()))) {
                self.retire_removed_thread(&thread);
            }
        }
    }

    fn retire_removed_thread(&self, thread: &Arc<CodexThread>) {
        let retired = {
            let mut removed = self.removed_threads.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            removed.iter().position(|existing| Arc::ptr_eq(existing, thread))
                .map(|index| removed.swap_remove(index))
        };
        drop(retired);
    }

    pub(super) fn observe_removed_for_client(&self, thread_id: ThreadId) -> CodexResult<()> {
        let removed = self.removed_threads.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner).iter()
            .filter(|thread| thread.session.thread_id() == thread_id)
            .cloned().collect::<Vec<_>>();
        for thread in removed {
            match thread.io.session_loop_termination.clone().now_or_never() {
                Some(Ok(())) => self.retire_removed_thread(&thread),
                Some(Err(_)) | None => return Err(CodexErr::ShutdownFailed),
            }
        }
        Ok(())
    }

    pub(super) async fn record_failed_session(&self, session: Arc<Session>) {
        let mut failed = self.failed_shutdowns.write().await;
        if !failed.iter().any(|existing| Arc::ptr_eq(existing, &session)) {
            failed.push(Arc::clone(&session));
        }
        // Drop the argument and any user-owned resources after the registry lock.
        drop(failed);
    }
}

impl ThreadManager {
    /// Retains a failed session independently of live-map removal or replacement.
    /// Failure is sticky; a later successful observation cannot erase it.
    pub async fn record_shutdown_failure(&self, thread: &Arc<CodexThread>) {
        self.state.record_failed_session(Arc::clone(&thread.session)).await;
    }

    /// Observe all tracked sessions within the caller's allowance. A timeout
    /// cancels only this observer; exact session and MCP task custody persists.
    pub async fn shutdown_all_threads_bounded(&self, timeout: Duration) -> ThreadShutdownReport {
        let threads = {
            // Removal publishes custody while holding the live-map write lock.
            // This matching read order gives a complete live/removed snapshot.
            let live = self.state.threads.read().await;
            let mut snapshot = live.values().cloned().collect::<Vec<_>>();
            let removed = self.state.removed_threads.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for thread in removed.iter() {
                if !snapshot.iter().any(|existing| Arc::ptr_eq(existing, thread)) {
                    snapshot.push(Arc::clone(thread));
                }
            }
            snapshot
        };
        let mut shutdowns = threads.into_iter().map(|thread| async move {
            let result = tokio::time::timeout(timeout, thread.shutdown_and_wait()).await;
            (thread, result)
        }).collect::<FuturesUnordered<_>>();
        let mut report = ThreadShutdownReport::default();
        while let Some((thread, result)) = shutdowns.next().await {
            let thread_id = thread.session.thread_id();
            match result {
                Ok(Ok(())) => {
                    // A replacement must survive an older session's completion.
                    self.remove_thread_if_matches(&thread_id, &thread).await;
                    self.state.retire_removed_thread(&thread);
                    report.completed.push(thread_id);
                }
                Ok(Err(error)) => {
                    self.record_shutdown_failure(&thread).await;
                    if matches!(error.details(), CodexErrorDetails::ShutdownFailed) {
                        report.shutdown_failed.push(thread_id);
                    } else {
                        report.submit_failed.push(thread_id);
                    }
                }
                Err(_) => report.timed_out.push(thread_id),
            }
        }
        // A replacement admitted after the snapshot is still live; do not label
        // its ID complete merely because the older runtime joined successfully.
        {
            let live = self.state.threads.read().await;
            report.shutdown_failed.extend(report.completed.iter()
                .copied().filter(|id| live.contains_key(id)));
        }
        report.shutdown_failed.extend(self.state.failed_shutdowns.read().await.iter()
            .map(|session| session.thread_id()));
        report.shutdown_failed.sort_by_key(std::string::ToString::to_string);
        report.shutdown_failed.dedup();
        // Different runtimes can share an ID. A retained failure wins over a
        // successful replacement in the aggregate report.
        report.completed.retain(|id| !report.shutdown_failed.contains(id));
        report.completed.sort_by_key(std::string::ToString::to_string);
        report.submit_failed.sort_by_key(std::string::ToString::to_string);
        report.timed_out.sort_by_key(std::string::ToString::to_string);
        report
    }
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
