//! One retained cancellation observation, scoped to the original process lease.

use std::sync::Arc;

use codex_file_search_api::SearchStartCancellationFuture;
use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::SearchStartControl;
use codex_file_search_api::StartCleanup;

use super::state::Lease;
use super::state::lost;
use super::state::observe;

pub(super) struct ProcessStartControl {
    pub lease: Arc<Lease>,
}

impl SearchStartControl for ProcessStartControl {
    fn request_cancel(&self) {
        self.lease.request_close();
    }

    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        Box::pin(observe_start(Arc::clone(&self.lease)))
    }
}

pub(super) async fn observe_start(lease: Arc<Lease>) -> SearchStartCancellationOutcome {
    let outcome = observe(lease.completed.subscribe()).await;
    SearchStartCancellationOutcome {
        operation: outcome.operation,
        // finish publishes this immutable value before waking close observers.
        // If the owner vanished without it, never guess no work or joined work.
        cleanup: lease
            .start_cleanup
            .get()
            .cloned()
            .unwrap_or_else(|| StartCleanup::Unconfirmed(lost())),
    }
}
