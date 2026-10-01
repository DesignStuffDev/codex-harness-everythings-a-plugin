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
        session.entered_close.add_permits(1);
        let mut gate = session.close_gate.subscribe();
        while !*gate.borrow_and_update() {
            gate.changed().await.expect("retained fixture cleanup gate");
        }
        publish_close(&session.closed, session.close_outcome.clone());
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
                        .close_outcome
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

struct Control(Arc<Session>);
impl SearchStartControl for Control {
    fn request_cancel(&self) {
        self.0.request_close();
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        let receiver = self.0.closed.subscribe();
        Box::pin(async move {
            let outcome = observe_close(receiver).await;
            SearchStartCancellationOutcome {
                operation: outcome.operation,
                cleanup: outcome.cleanup.into(),
            }
        })
    }
}

pub(super) fn begin(session: Arc<Session>) -> PendingSearchStart {
    // This fixture admits a pre-created ready session synchronously. Its close
    // actor already owns cleanup; the future below only observes public handoff.
    let control = Arc::new(Control(session.clone()));
    let public: Arc<dyn SearchBackendSession> = Arc::new(Lease(session.clone()));
    PendingSearchStart::new(
        control,
        Box::pin(async move {
            if session.closing.is_cancelled() {
                let outcome = observe_close(session.closed.subscribe()).await;
                return Err(SearchStartError {
                    operation: outcome.operation.err().unwrap_or_else(|| {
                        SearchError::new(SearchErrorKind::ClosedLease, "fixture closed")
                    }),
                    cleanup: outcome.cleanup.into(),
                });
            }
            Ok(public)
        }),
    )
}
