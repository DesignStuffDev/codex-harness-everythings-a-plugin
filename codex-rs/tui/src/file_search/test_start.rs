//! Retained fixture cleanup: gates model owned work, never waiter lifetime.
use super::*;

pub(super) fn own_close(session: Arc<Session>, task: TaskTrackerToken) {
    let guard = CloseGuard {
        session: Some(session.clone()),
        _task: task,
    };
    tokio::spawn(async move {
        let mut guard = guard;
        session.closing.cancelled().await;
        session.close_entered.add_permits(1);
        let mut gate = session.close_ready.subscribe();
        while !*gate.borrow_and_update() {
            gate.changed().await.expect("retained fixture cleanup gate");
        }
        publish_close(&session.closed, session.outcome.clone());
        drop(guard.session.take());
    });
}

fn publish_close(sender: &watch::Sender<Option<SearchCloseOutcome>>, outcome: SearchCloseOutcome) {
    sender.send_if_modified(|retained| {
        if retained.is_some() {
            return false;
        }
        *retained = Some(outcome);
        true
    });
}

pub(super) async fn observe_close(
    mut receiver: watch::Receiver<Option<SearchCloseOutcome>>,
) -> SearchCloseOutcome {
    loop {
        if let Some(outcome) = receiver.borrow_and_update().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            let error = lost();
            return SearchCloseOutcome {
                operation: Err(error.clone()),
                cleanup: CloseCleanup::Unconfirmed(error),
            };
        }
    }
}

fn lost() -> SearchError {
    SearchError::new(
        SearchErrorKind::ForcedShutdown,
        "fixture lifecycle owner disappeared without a cleanup receipt",
    )
}

struct CloseGuard {
    session: Option<Arc<Session>>,
    _task: TaskTrackerToken,
}
impl Drop for CloseGuard {
    fn drop(&mut self) {
        if let Some(session) = &self.session {
            session.closing.cancel();
            let error = lost();
            publish_close(
                &session.closed,
                SearchCloseOutcome {
                    operation: Err(session
                        .outcome
                        .operation
                        .clone()
                        .err()
                        .unwrap_or_else(|| error.clone())),
                    cleanup: CloseCleanup::Unconfirmed(error),
                },
            );
        }
    }
}

// Only this public wrapper requests close on last public handle loss; retained
// actors and cancellation controls hold the Session state, not this wrapper.
struct Lease(Arc<Session>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.request_close();
    }
}
impl SearchBackendSession for Lease {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        self.0.update_query(query)
    }
    fn next_snapshot(&self, after: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        self.0.next_snapshot(after, wait)
    }
    fn request_close(&self) {
        self.0.request_close();
    }
    fn close(&self) -> SearchCloseFuture<'_> {
        self.0.close()
    }
}

struct Start {
    session: Arc<Session>,
    receipt: watch::Sender<Option<SearchStartCancellationOutcome>>,
}
impl Start {
    fn publish(&self, outcome: SearchStartCancellationOutcome) {
        self.receipt.send_if_modified(|retained| {
            if retained.is_some() {
                return false;
            }
            *retained = Some(outcome);
            true
        });
    }
}
impl SearchStartControl for Start {
    fn request_cancel(&self) {
        self.session.request_close();
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        Box::pin(observe_start(self.receipt.subscribe()))
    }
}
async fn observe_start(
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
struct StartGuard {
    start: Option<Arc<Start>>,
    _task: TaskTrackerToken,
}
impl Drop for StartGuard {
    fn drop(&mut self) {
        if let Some(start) = &self.start {
            start.session.request_close();
            let error = lost();
            start.publish(SearchStartCancellationOutcome {
                operation: Err(start
                    .session
                    .outcome
                    .operation
                    .clone()
                    .err()
                    .unwrap_or_else(|| error.clone())),
                cleanup: StartCleanup::Unconfirmed(error),
            });
        }
    }
}

pub(super) fn begin(
    session: Arc<Session>,
    mut ready: watch::Receiver<bool>,
    entered: Arc<Semaphore>,
    task: TaskTrackerToken,
) -> PendingSearchStart {
    let start = Arc::new(Start {
        session,
        receipt: watch::channel(None).0,
    });
    let owner = start.clone();
    let guard = StartGuard {
        start: Some(owner.clone()),
        _task: task,
    };
    let (sender, receiver) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let mut guard = guard;
        entered.add_permits(1);
        while !*ready.borrow_and_update() && !owner.session.closing.is_cancelled() {
            tokio::select! {
                biased;
                _ = owner.session.closing.cancelled() => {},
                changed = ready.changed() => { changed.expect("retained fixture open gate"); },
            }
        }
        let mut sender = Some(sender);
        if !owner.session.closing.is_cancelled() {
            let public: Arc<dyn SearchBackendSession> = Arc::new(Lease(owner.session.clone()));
            if sender
                .take()
                .expect("one ready reply")
                .send(Ok(public))
                .is_err()
            {
                owner.session.request_close();
            }
        }
        owner.session.closing.cancelled().await;
        let close = observe_close(owner.session.closed.subscribe()).await;
        let outcome = SearchStartCancellationOutcome {
            operation: close.operation,
            cleanup: close.cleanup.into(),
        };
        if let Some(sender) = sender {
            let _ = sender.send(Err(SearchStartError {
                operation: outcome.operation.clone().err().unwrap_or_else(|| {
                    SearchError::new(SearchErrorKind::ClosedLease, "fixture closed")
                }),
                cleanup: outcome.cleanup.clone(),
            }));
        }
        owner.publish(outcome);
        drop(guard.start.take());
    });
    let control: Arc<dyn SearchStartControl> = start.clone();
    PendingSearchStart::new(
        control,
        Box::pin(async move {
            match receiver.await {
                Ok(Ok(session)) if !start.session.closing.is_cancelled() => return Ok(session),
                Ok(Ok(_)) => {}
                Ok(Err(error)) => return Err(error),
                Err(_) => start.request_cancel(),
            }
            let outcome = observe_start(start.receipt.subscribe()).await;
            Err(SearchStartError {
                operation: outcome.operation.err().unwrap_or_else(|| {
                    SearchError::new(SearchErrorKind::ClosedLease, "fixture closed")
                }),
                cleanup: outcome.cleanup,
            })
        }),
    )
}
