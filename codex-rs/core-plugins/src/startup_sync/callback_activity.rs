//! Non-closing, non-harvesting activity diagnostics for an owned callback scope.

use super::CuratedCallbackScope;
use super::Record;
use super::lock;
use std::sync::atomic::Ordering;

/// Activity of exact registered actions, without consuming any task result.
///
/// A finished-but-unjoined task may have succeeded, been suppressed, panicked,
/// or been cancelled. Only the existing final scope wait observes that outcome.
/// This snapshot is not a successful-completion or downstream MCP receipt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CuratedCallbackActivity {
    pub admission_closed: bool,
    pub awaiting_handle: usize,
    pub running: usize,
    pub finished_unjoined: usize,
    pub joining: usize,
    pub completed: usize,
    pub suppressed: usize,
    pub failed: usize,
    /// Unexpected records remain retained and always require failure reporting.
    pub unexpected: usize,
}

impl CuratedCallbackScope {
    /// Inspect scope state and exact handle readiness without closing, polling,
    /// taking/joining handles, dispatching actions or exposing captured payloads.
    /// New admission can occur after this snapshot; a caller requiring a stable
    /// generation boundary must separately establish its producer has finished.
    pub fn activity(&self) -> CuratedCallbackActivity {
        // Any last registry reference must drop outside the scope mutex.
        let registry = self.registry.upgrade();
        let state = lock(&self.state);
        let mut observed = CuratedCallbackActivity {
            admission_closed: state.closed
                || registry
                    .as_ref()
                    .is_none_or(|registry| registry.closing.load(Ordering::Acquire)),
            unexpected: state.unexpected.len(),
            ..Default::default()
        };
        for record in &state.records {
            match record {
                Record::AwaitingHandle => observed.awaiting_handle += 1,
                Record::Running(handle) if handle.is_finished() => observed.finished_unjoined += 1,
                Record::Running(_) => observed.running += 1,
                Record::Joining => observed.joining += 1,
                Record::Completed => observed.completed += 1,
                Record::Suppressed => observed.suppressed += 1,
                Record::Failed { .. } => observed.failed += 1,
            }
        }
        drop(state);
        drop(registry);
        observed
    }
}

#[cfg(test)]
#[path = "callback_activity_tests.rs"]
mod tests;
