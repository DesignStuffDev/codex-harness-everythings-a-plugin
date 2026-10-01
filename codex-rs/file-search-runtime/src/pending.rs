//! Passive pending owner, separate from the retained startup and cleanup tasks.

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use codex_file_search_api::SearchStartCancellationFuture;
use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::SearchStartControl;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::FileSearchSession;
use crate::state::*;

pub type RuntimeSearchStartFuture<'a> =
    Pin<Box<dyn Future<Output = Result<FileSearchSession, SearchStartError>> + Send + 'a>>;

/// One pending public owner. Its control remains scoped to this exact lease
/// after readiness; neither a control clone nor a dropped observer owns cleanup.
#[must_use = "observe startup or drop this ticket to request cancellation"]
pub struct PendingFileSearchStart {
    control: Arc<dyn SearchStartControl>,
    completion: RuntimeSearchStartFuture<'static>,
    armed: bool,
}
impl PendingFileSearchStart {
    pub(crate) fn new(
        lease: Arc<Lease>,
        receiver: oneshot::Receiver<Result<(), SearchStartError>>,
    ) -> Self {
        let control = Arc::new(RuntimeStartControl(Arc::clone(&lease)));
        Self {
            control,
            completion: Box::pin(async move {
                let result = receiver.await;
                // Readiness notification alone is not transfer to a public owner.
                let handoff = matches!(result, Ok(Ok(()))) && !lock(&lease.state).closing;
                if handoff {
                    return Ok(FileSearchSession::new(lease));
                }
                lease.request_close();
                let receipt = observe_cancel(lease.cancelled.subscribe()).await;
                let original = result.ok().and_then(Result::err);
                Err(SearchStartError {
                    operation: receipt.operation.err().unwrap_or_else(|| {
                        original.map_or_else(
                            || match &receipt.cleanup {
                                StartCleanup::Unconfirmed(error) => error.clone(),
                                StartCleanup::NotAdmitted | StartCleanup::Confirmed => closed(),
                            },
                            |error| error.operation,
                        )
                    }),
                    cleanup: receipt.cleanup,
                })
            }),
            armed: true,
        }
    }
    pub fn control(&self) -> Arc<dyn SearchStartControl> {
        Arc::clone(&self.control)
    }
    pub fn finish(self) -> RuntimeSearchStartFuture<'static> {
        Box::pin(self)
    }
}
impl Future for PendingFileSearchStart {
    type Output = Result<FileSearchSession, SearchStartError>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let result = this.completion.as_mut().poll(context);
        if result.is_ready() {
            this.armed = false;
        }
        result
    }
}
impl Drop for PendingFileSearchStart {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| self.control.request_cancel()));
        }
    }
}
struct RuntimeStartControl(Arc<Lease>);
impl SearchStartControl for RuntimeStartControl {
    fn request_cancel(&self) {
        self.0.request_close();
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        Box::pin(observe_cancel(self.0.cancelled.subscribe()))
    }
}
pub(crate) async fn observe_cancel(
    mut receiver: watch::Receiver<Option<SearchStartCancellationOutcome>>,
) -> SearchStartCancellationOutcome {
    loop {
        if let Some(outcome) = receiver.borrow_and_update().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            let error = lost();
            return SearchStartCancellationOutcome {
                operation: Err(error.clone()),
                cleanup: StartCleanup::Unconfirmed(error),
            };
        }
    }
}

impl Lease {
    /// Start errors are retained independently of eventual cleanup certainty.
    pub(crate) fn retained_failure(&self) -> Option<codex_file_search_api::SearchError> {
        let state = lock(&self.state);
        state
            .first_error
            .clone()
            .or_else(|| {
                state.start_error.as_ref().and_then(|error| {
                    let clean_receipt =
                        state.backend_cancellation.as_ref().is_some_and(|receipt| {
                            receipt.operation.is_ok()
                                && !matches!(receipt.cleanup, StartCleanup::Unconfirmed(_))
                        });
                    let synthetic = state.skipped_before_backend
                        || (error.operation.kind()
                            == codex_file_search_api::SearchErrorKind::ClosedLease
                            && clean_receipt);
                    (!synthetic).then(|| error.operation.clone())
                })
            })
            .or_else(|| {
                state
                    .backend_cancellation
                    .as_ref()
                    .and_then(|outcome| outcome.operation.clone().err())
            })
    }

    /// A typed cancellation receipt, not the timing of a closing flag, is the
    /// authority for expected ClosedLease. A different real error always wins.
    pub(crate) fn retained_operation(
        &self,
        receipt: &SearchStartCancellationOutcome,
    ) -> Option<codex_file_search_api::SearchError> {
        let state = lock(&self.state);
        state
            .first_error
            .clone()
            .or_else(|| {
                state.start_error.as_ref().and_then(|error| {
                    let proven_cancel = state.skipped_before_backend
                        || (state.pending_start.is_some()
                            && error.operation.kind()
                                == codex_file_search_api::SearchErrorKind::ClosedLease
                            && receipt.operation.is_ok()
                            && !matches!(receipt.cleanup, StartCleanup::Unconfirmed(_)));
                    (!proven_cancel).then(|| error.operation.clone())
                })
            })
            .or_else(|| receipt.operation.clone().err())
    }

    /// Run exactly one owned cancellation observer as soon as control exists;
    /// do not wait for a potentially blocked startup result to request cleanup.
    pub(crate) async fn join_backend(&self) -> SearchStartCancellationOutcome {
        use futures::FutureExt;
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let (control, finished, skipped, error, uncertainty) = {
                let state = lock(&self.state);
                (
                    state.pending_start.clone(),
                    state.start_finished,
                    state.skipped_before_backend,
                    state.start_error.clone(),
                    state.cleanup_uncertain.clone(),
                )
            };
            if let Some(error) = uncertainty {
                return SearchStartCancellationOutcome {
                    operation: Err(self.retained_failure().unwrap_or_else(|| error.clone())),
                    cleanup: StartCleanup::Unconfirmed(error),
                };
            }
            if let Some(control) = control {
                let observed =
                    AssertUnwindSafe(async { control.cancel_and_wait().await }).catch_unwind();
                tokio::pin!(observed);
                loop {
                    let changed = self.changed.notified();
                    tokio::pin!(changed);
                    changed.as_mut().enable();
                    let uncertainty = lock(&self.state).cleanup_uncertain.clone();
                    if let Some(error) = uncertainty {
                        // Dropping a passive receipt observer does not abort the
                        // backend's retained cancellation owner or refund quota.
                        return SearchStartCancellationOutcome {
                            operation: Err(self
                                .retained_failure()
                                .unwrap_or_else(|| error.clone())),
                            cleanup: StartCleanup::Unconfirmed(error),
                        };
                    }
                    tokio::select! {
                        result = &mut observed => return match result {
                            Ok(outcome) => outcome,
                            Err(_) => {
                                let error = panicked();
                                SearchStartCancellationOutcome { operation: Err(error.clone()), cleanup: StartCleanup::Unconfirmed(error) }
                            }
                        },
                        _ = &mut changed => {},
                    }
                }
            }
            if finished {
                return if skipped {
                    SearchStartCancellationOutcome {
                        operation: Ok(()),
                        cleanup: StartCleanup::NotAdmitted,
                    }
                } else if let Some(error) = error {
                    SearchStartCancellationOutcome {
                        operation: Err(error.operation),
                        cleanup: error.cleanup,
                    }
                } else {
                    let error = lost();
                    SearchStartCancellationOutcome {
                        operation: Err(error.clone()),
                        cleanup: StartCleanup::Unconfirmed(error),
                    }
                };
            }
            changed.await;
        }
    }

    /// A backend claiming no admission cannot later hand us a ready session.
    /// Drain that actual session rather than releasing based on contradictory
    /// metadata; the protocol failure remains visible even if cleanup joins.
    pub(crate) async fn reconcile_start(
        &self,
        mut receipt: SearchStartCancellationOutcome,
    ) -> SearchStartCancellationOutcome {
        use futures::FutureExt;
        let backend = {
            let state = lock(&self.state);
            (receipt.cleanup == StartCleanup::NotAdmitted)
                .then(|| state.backend.clone())
                .flatten()
        };
        if let Some(backend) = backend {
            let error = crate::policy::invalid(
                "file-search backend returned a session after claiming no admission",
            );
            let outcome = match AssertUnwindSafe(async { backend.close().await })
                .catch_unwind()
                .await
            {
                Ok(outcome) => outcome,
                Err(_) => uncertain(panicked()),
            };
            receipt.operation = Err(receipt
                .operation
                .err()
                .or_else(|| outcome.operation.err())
                .unwrap_or(error));
            receipt.cleanup = outcome.cleanup.into();
        }
        receipt
    }

    /// Serialize the pair of public receipts through one immutable state value.
    /// Bookkeeping must already have released or quarantined this reservation.
    pub(crate) fn publish_completion(&self, receipt: SearchStartCancellationOutcome) {
        let retained = {
            let mut state = lock(&self.state);
            state.terminal.get_or_insert(receipt).clone()
        };
        self.cancelled.send_if_modified(|slot| {
            if slot.is_none() {
                *slot = Some(retained.clone());
                true
            } else {
                false
            }
        });
        let cleanup = match retained.cleanup {
            StartCleanup::NotAdmitted | StartCleanup::Confirmed => {
                codex_file_search_api::CloseCleanup::Joined
            }
            StartCleanup::Unconfirmed(error) => {
                codex_file_search_api::CloseCleanup::Unconfirmed(error)
            }
        };
        complete(
            &self.completed,
            codex_file_search_api::SearchCloseOutcome {
                operation: retained.operation,
                cleanup,
            },
        );
    }
}
