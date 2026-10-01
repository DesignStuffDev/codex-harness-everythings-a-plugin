use crate::SearchBackendSession;
use crate::SearchError;
use crate::SearchStartError;
use crate::SearchStartFuture;
use crate::StartCleanup;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

/// Retained cancellation receipt for exactly one pending start and its lease.
///
/// Cancellation intent, elapsed time and a dropped observer are not cleanup
/// proof. `NotAdmitted` means no backend work was accepted; `Confirmed` requires
/// actual drainage, including a ready session if cancellation raced its handoff.
/// `Unconfirmed` retains accounting until a separate recovery receipt. Keep the
/// first operation error independent of cleanup certainty. A clean cancellation
/// has `operation == Ok(())`; requesting cancellation is not itself a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "inspect cleanup ownership separately from the retained operation result"]
pub struct SearchStartCancellationOutcome {
    pub operation: Result<(), SearchError>,
    pub cleanup: StartCleanup,
}

/// Passive observation of the independently retained cancellation owner.
pub type SearchStartCancellationFuture<'a> =
    Pin<Box<dyn Future<Output = SearchStartCancellationOutcome> + Send + 'a>>;

/// Cancellation authority for one start, never its provider or sibling leases.
///
/// The implementation owns accepted work, bounded cancellation state, exactly
/// one cleanup completion, and its immutable receipt independently of observers.
/// Register that owner and an armed completion guard before publishing a ticket
/// or spawning work: dropping an unpolled task must not lose cleanup evidence.
/// Repeated observers share the receipt, not a new cleanup operation. The ticket
/// below does not implement or prove those backend ownership obligations.
pub trait SearchStartControl: Send + Sync {
    /// Synchronously latch cancellation of this start or its resulting lease.
    /// The ready-before-cancel race must still target this same lease only.
    ///
    /// This hook must be infallible, idempotent and bounded: do not block, await,
    /// run user callbacks, grow a registry or spawn a task on every call. It may
    /// wake an already-retained owner. In particular it must be safe in Drop and
    /// while unwinding. A panic violates the contract; it proves no cleanup.
    fn request_cancel(&self);

    /// Request cancellation synchronously, then observe the retained receipt.
    /// The request happens even if this returned future is never polled. Dropping
    /// this observer must not cancel the owner, release capacity or change the
    /// receipt seen by later observers. Like `request_cancel`, the synchronous
    /// part must be bounded and infallible; waiting belongs only in the future.
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static>;
}

/// A non-Clone observation ticket for an independently retained startup owner.
///
/// Dropping an unfinished ticket requests cancellation before dropping its
/// completion observer. A terminal result transfers the exact ready session or
/// startup error and disarms that abandonment request. Retaining a control clone
/// does not defer ticket-drop cancellation; it can also cancel the same lease
/// after a successful handoff. No executor, spawn or backend adapter lives here.
///
/// The cancellation hook must not panic. Drop catches its initial unwind without
/// manufacturing a receipt. This is best-effort containment: destructors (including
/// those of panic payloads) must also be infallible, and aborts cannot be caught.
/// Explicit calls on the returned control retain the implementation's behavior.
#[must_use = "observe startup or deliberately drop the ticket to request cancellation"]
pub struct PendingSearchStart {
    control: Arc<dyn SearchStartControl>,
    completion: SearchStartFuture<'static>,
    armed: bool,
}

impl PendingSearchStart {
    /// Construct from an already-retained owner's control and passive observer.
    ///
    /// Both must identify the same start. The owner must survive dropping this
    /// observer, publish uncertainty if lost, and retain cleanup after terminal
    /// errors. Wrapping a sole unowned backend future does not meet this contract.
    pub fn new(
        control: Arc<dyn SearchStartControl>,
        completion: SearchStartFuture<'static>,
    ) -> Self {
        Self {
            control,
            completion,
            armed: true,
        }
    }

    #[must_use]
    pub fn control(&self) -> Arc<dyn SearchStartControl> {
        Arc::clone(&self.control)
    }

    /// Consume the ticket into an observer with the same armed drop guard.
    /// Dropping it before its first poll still requests cancellation.
    pub fn finish(self) -> SearchStartFuture<'static> {
        Box::pin(self)
    }
}

impl Future for PendingSearchStart {
    type Output = Result<Arc<dyn SearchBackendSession>, SearchStartError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let result = this.completion.as_mut().poll(context);
        if result.is_ready() {
            this.armed = false;
        }
        result
    }
}

impl Drop for PendingSearchStart {
    fn drop(&mut self) {
        if self.armed {
            // Contain a contract-violating unwind only at this destructor
            // boundary. It must never fabricate a cancellation/cleanup receipt.
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| self.control.request_cancel()));
        }
    }
}
