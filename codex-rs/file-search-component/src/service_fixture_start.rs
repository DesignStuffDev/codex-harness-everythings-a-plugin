//! Retained synthetic constructor owners for service tests, not native joins.

use super::*;
use tokio::sync::oneshot;

const MAX_FIXTURE_STARTS: usize = 16;

pub(super) struct FixtureStart {
    pub(super) cancelled: watch::Sender<bool>,
    pub(super) requests: AtomicUsize,
    pub(super) receipt: watch::Sender<Option<SearchStartCancellationOutcome>>,
    pub(super) session: Arc<Session>,
}

impl SearchStartControl for FixtureStart {
    fn request_cancel(&self) {
        if !self.cancelled.send_replace(true) {
            self.requests.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        let mut receipt = self.receipt.subscribe();
        Box::pin(async move {
            loop {
                if let Some(outcome) = receipt.borrow_and_update().clone() {
                    return outcome;
                }
                if receipt.changed().await.is_err() {
                    return lost_receipt();
                }
            }
        })
    }
}

struct OwnerGuard(Option<Arc<FixtureStart>>);

impl Drop for OwnerGuard {
    fn drop(&mut self) {
        if let Some(control) = self.0.take() {
            // The guard exists before spawn and cannot convert task loss into
            // successful synthetic cleanup, even before the first owner poll.
            control.receipt.send_replace(Some(lost_receipt()));
        }
    }
}

fn lost_error() -> SearchError {
    SearchError::new(
        SearchErrorKind::TransportLost,
        "service fixture startup owner disappeared",
    )
}

fn lost_receipt() -> SearchStartCancellationOutcome {
    SearchStartCancellationOutcome {
        operation: Err(lost_error()),
        cleanup: StartCleanup::Unconfirmed(lost_error()),
    }
}

pub(super) fn begin(backend: &Backend) -> Result<PendingSearchStart, SearchStartError> {
    backend.begin_calls.fetch_add(1, Ordering::SeqCst);
    let control = {
        let mut controls = lock(&backend.controls);
        if controls.len() == MAX_FIXTURE_STARTS {
            return Err(SearchStartError {
                operation: SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "service fixture retained start ceiling reached",
                ),
                cleanup: StartCleanup::NotAdmitted,
            });
        }
        let session = if controls.is_empty() {
            Arc::clone(&backend.session)
        } else {
            Arc::new(Session::new(
                backend.session.outcome.clone(),
                /*polls*/ 0,
                backend.session.snapshot.clone(),
            ))
        };
        let control = Arc::new(FixtureStart {
            cancelled: watch::channel(false).0,
            requests: AtomicUsize::new(0),
            receipt: watch::channel(None).0,
            session,
        });
        controls.push(Arc::clone(&control));
        control
    };
    let mut guard = OwnerGuard(Some(Arc::clone(&control)));
    let starts = Arc::clone(&backend.starts);
    let session = Arc::clone(&control.session);
    let plan = backend.start_plan.clone();
    let (sender, receiver) = oneshot::channel();
    backend.entered.add_permits(1);
    tokio::spawn(async move {
        starts.acquire().await.expect("start gate").forget();
        let control = guard.0.as_ref().expect("armed owner");
        let failure = match plan {
            StartPlan::Ready => None,
            StartPlan::Fail(error) => Some((error, false)),
            StartPlan::ClosedByCancellation(error) => {
                let expected = *control.cancelled.borrow()
                    && matches!(
                        &error.cleanup,
                        StartCleanup::NotAdmitted | StartCleanup::Confirmed
                    );
                Some((error, expected))
            }
        };
        if let Some((error, expected_cancellation)) = failure {
            control
                .receipt
                .send_replace(Some(SearchStartCancellationOutcome {
                    operation: if expected_cancellation {
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

        let _ = sender.send(Ok(Arc::clone(&session) as Arc<dyn SearchBackendSession>));
        let mut cancelled = control.cancelled.subscribe();
        let mut closing = session.closing.subscribe();
        loop {
            let requested = *cancelled.borrow_and_update() || *closing.borrow_and_update();
            if requested {
                break;
            }
            tokio::select! {
                result = cancelled.changed() => result.expect("retained cancellation sender"),
                result = closing.changed() => result.expect("retained session sender"),
            }
        }
        let outcome = session.close().await;
        control
            .receipt
            .send_replace(Some(SearchStartCancellationOutcome {
                operation: outcome.operation,
                cleanup: outcome.cleanup.into(),
            }));
        drop(guard.0.take());
    });
    Ok(PendingSearchStart::new(
        control,
        Box::pin(async move {
            receiver.await.unwrap_or_else(|_| {
                Err(SearchStartError {
                    operation: lost_error(),
                    cleanup: StartCleanup::Unconfirmed(lost_error()),
                })
            })
        }),
    ))
}
