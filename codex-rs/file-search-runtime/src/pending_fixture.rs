//! Retained synthetic constructor/close owners; these do not prove OS joins.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use codex_file_search_api::*;
use futures::FutureExt;
use tokio::sync::Semaphore;
use tokio::sync::oneshot;
use tokio::sync::watch;

#[derive(Clone, Copy)]
pub(crate) enum FailureOrigin {
    Genuine,
    ExplicitCancellation,
}

pub(crate) struct Control {
    cancelled: watch::Sender<bool>,
    receipt: watch::Sender<Option<SearchStartCancellationOutcome>>,
}

impl SearchStartControl for Control {
    fn request_cancel(&self) {
        self.cancelled.send_replace(true);
    }

    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        Box::pin(observe(self.receipt.subscribe()))
    }
}

impl Control {
    pub(crate) async fn completed(&self) -> SearchStartCancellationOutcome {
        observe(self.receipt.subscribe()).await
    }
}

#[derive(Default)]
pub(crate) struct SingleStart(Mutex<Option<Arc<Control>>>);

impl SingleStart {
    pub(crate) fn begin(
        &self,
        constructor: SearchStartFuture<'static>,
        origin: FailureOrigin,
        returned: Option<Arc<Semaphore>>,
    ) -> Result<PendingSearchStart, SearchStartError> {
        let mut stored = self.0.lock().unwrap();
        if stored.is_some() {
            return Err(SearchStartError {
                operation: SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "single-start fixture already admitted its constructor",
                ),
                cleanup: StartCleanup::NotAdmitted,
            });
        }
        let (pending, control) = begin(constructor, origin, returned);
        *stored = Some(control);
        Ok(pending)
    }

    pub(crate) fn request_cancel(&self) {
        let control = self.0.lock().unwrap().clone();
        if let Some(control) = control {
            control.request_cancel();
        }
    }

    pub(crate) async fn completed(&self) {
        let control = self.0.lock().unwrap().clone();
        if let Some(control) = control {
            let _ = control.completed().await;
        }
    }
}

struct OwnerGuard(Option<Arc<Control>>);

impl Drop for OwnerGuard {
    fn drop(&mut self) {
        if let Some(control) = self.0.take() {
            control.receipt.send_if_modified(|receipt| {
                if receipt.is_some() {
                    return false;
                }
                *receipt = Some(uncertain(lost()));
                true
            });
        }
    }
}

fn lost() -> SearchError {
    SearchError::new(
        SearchErrorKind::TransportLost,
        "retained runtime fixture owner disappeared",
    )
}

fn uncertain(error: SearchError) -> SearchStartCancellationOutcome {
    SearchStartCancellationOutcome {
        operation: Err(error.clone()),
        cleanup: StartCleanup::Unconfirmed(error),
    }
}

async fn observe(
    mut receiver: watch::Receiver<Option<SearchStartCancellationOutcome>>,
) -> SearchStartCancellationOutcome {
    loop {
        if let Some(outcome) = receiver.borrow_and_update().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            return uncertain(lost());
        }
    }
}

/// Register an owned constructor before returning its passive ticket. After a
/// successful handoff the same task retains the sole underlying close operation.
/// Caller registries must bound accepted starts and retain controls for shutdown.
pub(crate) fn begin(
    constructor: SearchStartFuture<'static>,
    origin: FailureOrigin,
    returned: Option<Arc<Semaphore>>,
) -> (PendingSearchStart, Arc<Control>) {
    let control = Arc::new(Control {
        cancelled: watch::channel(false).0,
        receipt: watch::channel(None).0,
    });
    let mut guard = OwnerGuard(Some(Arc::clone(&control)));
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let result = match AssertUnwindSafe(constructor).catch_unwind().await {
            Ok(result) => result,
            Err(_) => Err(SearchStartError {
                operation: lost(),
                cleanup: StartCleanup::Unconfirmed(lost()),
            }),
        };
        let control = guard.0.as_ref().unwrap();
        let session = match result {
            Ok(session) => session,
            Err(error) => {
                let expected = matches!(origin, FailureOrigin::ExplicitCancellation)
                    && *control.cancelled.borrow()
                    && error.operation.kind() == SearchErrorKind::ClosedLease
                    && matches!(
                        &error.cleanup,
                        StartCleanup::NotAdmitted | StartCleanup::Confirmed
                    );
                control
                    .receipt
                    .send_replace(Some(SearchStartCancellationOutcome {
                        operation: if expected {
                            Ok(())
                        } else {
                            Err(error.operation.clone())
                        },
                        cleanup: error.cleanup.clone(),
                    }));
                drop(guard.0.take());
                let _ = sender.send(Err(error));
                return;
            }
        };
        let handoff = Arc::new(OwnedSession {
            native: Arc::clone(&session),
            control: Arc::clone(control),
        }) as Arc<dyn SearchBackendSession>;
        let _ = sender.send(Ok(handoff));
        let mut cancelled = control.cancelled.subscribe();
        loop {
            let requested = *cancelled.borrow_and_update();
            if requested {
                break;
            }
            cancelled.changed().await.unwrap();
        }
        let receipt = match AssertUnwindSafe(async { session.close().await })
            .catch_unwind()
            .await
        {
            Ok(outcome) => SearchStartCancellationOutcome {
                operation: outcome.operation,
                cleanup: outcome.cleanup.into(),
            },
            Err(_) => uncertain(lost()),
        };
        control.receipt.send_replace(Some(receipt));
        drop(guard.0.take());
    });
    let ticket = PendingSearchStart::new(
        control.clone(),
        Box::pin(async move {
            let result = receiver.await.unwrap_or_else(|_| {
                Err(SearchStartError {
                    operation: lost(),
                    cleanup: StartCleanup::Unconfirmed(lost()),
                })
            });
            // Signal from the passive finish immediately before returning, as
            // the old ReadyBackend::open did. Its facade task has no extra await
            // before publishing readiness on a current-thread executor.
            if let Some(returned) = returned {
                returned.add_permits(1);
            }
            result
        }),
    );
    (ticket, control)
}

struct OwnedSession {
    native: Arc<dyn SearchBackendSession>,
    control: Arc<Control>,
}

impl Drop for OwnedSession {
    fn drop(&mut self) {
        self.control.request_cancel();
    }
}

impl SearchBackendSession for OwnedSession {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        self.native.update_query(query)
    }

    fn next_snapshot(&self, after: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        self.native.next_snapshot(after, wait)
    }

    fn request_close(&self) {
        self.control.request_cancel();
        // Keep the deliberately panicking native hook visible to the production
        // facade's recovery test. The retained close owner is already signalled.
        self.native.request_close();
    }

    fn close(&self) -> SearchCloseFuture<'_> {
        let receipt = self.control.cancel_and_wait();
        Box::pin(async move {
            let receipt = receipt.await;
            SearchCloseOutcome {
                operation: receipt.operation,
                cleanup: match receipt.cleanup {
                    StartCleanup::NotAdmitted | StartCleanup::Confirmed => CloseCleanup::Joined,
                    StartCleanup::Unconfirmed(error) => CloseCleanup::Unconfirmed(error),
                },
            }
        })
    }
}
