//! Regression checks through the production facade, with backend gates fixing
//! which observer wins each race. None of these fixtures owns facade cleanup.

use std::future::Future;
use std::sync::atomic::AtomicUsize;
use std::task::Poll;

use pretty_assertions::assert_eq;
use tokio::sync::Semaphore;

use crate::FileSearchProvider;

use super::*;

fn startup_close_cases() -> Vec<(SearchCloseOutcome, SearchError)> {
    let operation = SearchError::new(
        SearchErrorKind::ResourceExhausted,
        "startup cleanup retained the backend resource failure",
    );
    let transport = SearchError::new(
        SearchErrorKind::TransportLost,
        "startup cleanup could not confirm the backend join",
    );
    vec![
        (
            SearchCloseOutcome {
                operation: Err(operation.clone()),
                cleanup: CloseCleanup::Joined,
            },
            operation,
        ),
        (
            SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Unconfirmed(transport.clone()),
            },
            transport,
        ),
        (joined(), crate::state::closed()),
    ]
}

#[tokio::test]
async fn close_during_pending_startup_preserves_the_cleanup_cause() {
    for (outcome, cause) in startup_close_cases() {
        let native = Session::new(/*closes*/ 1, outcome.clone());
        let backend = Backend::new(vec![native], /*starts*/ 0);
        let provider = provider(Arc::clone(&backend));
        let scope = scope(&provider);
        let opening_scope = scope.clone();
        let opening = tokio::spawn(async move {
            opening_scope
                .open(request(), Arc::new(Reporter::default()))
                .await
        });
        bounded(backend.opened.acquire()).await.unwrap().forget();
        scope.request_shutdown();
        backend.starts.add_permits(1);

        assert_eq!(
            bounded(opening).await.unwrap().err().unwrap(),
            SearchStartError {
                operation: cause,
                cleanup: outcome.cleanup.clone().into(),
            },
        );
        assert_eq!(bounded(scope.shutdown()).await, outcome);
        assert_eq!(
            bounded(provider.shutdown()).await,
            SearchCloseOutcome {
                operation: outcome.operation,
                cleanup: CloseCleanup::Joined
            },
        );
    }
}

/// Signals from the pending ticket immediately before returning the selected
/// session, with no later await in finish. On the current-thread executor, the
/// facade startup task publishes its result before the test resumes on this signal.
struct ReadyBackend {
    session: Arc<dyn SearchBackendSession>,
    returned: Arc<Semaphore>,
    start: crate::pending_fixture::SingleStart,
}

impl SearchBackend for ReadyBackend {
    fn begin_open(&self, _: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let session = Arc::clone(&self.session);
        self.start.begin(
            Box::pin(async move { Ok(session) }),
            crate::pending_fixture::FailureOrigin::Genuine,
            Some(Arc::clone(&self.returned)),
        )
    }

    fn request_shutdown(&self) {
        self.start.request_cancel();
        self.session.request_close();
    }

    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            self.request_shutdown();
            self.start.completed().await;
            joined()
        })
    }
}

#[tokio::test]
async fn close_before_startup_observer_resumes_preserves_the_cleanup_cause() {
    for (outcome, cause) in startup_close_cases() {
        let native = Session::new(/*closes*/ 1, outcome.clone());
        let backend = Arc::new(ReadyBackend {
            session: native,
            returned: Arc::new(Semaphore::new(0)),
            start: crate::pending_fixture::SingleStart::default(),
        });
        let provider = FileSearchProvider::from_backend(backend.clone(), policy()).unwrap();
        let scope = scope(&provider);
        let mut opening = Box::pin(scope.open(request(), Arc::new(Reporter::default())));
        std::future::poll_fn(|context| {
            assert!(opening.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        bounded(backend.returned.acquire()).await.unwrap().forget();

        // Leave the public open future unpolled until cleanup has completed.
        // Its retained startup task already sent Ok, selecting the second race.
        assert_eq!(bounded(scope.shutdown()).await, outcome);
        assert_eq!(
            bounded(opening).await.err().unwrap(),
            SearchStartError {
                operation: cause,
                cleanup: outcome.cleanup.clone().into(),
            },
        );
        assert_eq!(
            bounded(provider.shutdown()).await,
            SearchCloseOutcome {
                operation: outcome.operation,
                cleanup: CloseCleanup::Joined
            },
        );
    }
}

#[tokio::test]
async fn uncertain_close_winning_update_observer_preserves_cause_priority() {
    let operation = SearchError::new(SearchErrorKind::SearchFailed, "retained operation failure");
    let transport = SearchError::new(
        SearchErrorKind::TransportLost,
        "unconfirmed cleanup failure",
    );
    for (operation_result, cause) in [
        (Err(operation.clone()), operation),
        (Ok(()), transport.clone()),
    ] {
        let outcome = SearchCloseOutcome {
            operation: operation_result,
            cleanup: CloseCleanup::Unconfirmed(transport.clone()),
        };
        let native = Session::new(/*closes*/ 1, outcome.clone());
        native.updates.acquire_many(100).await.unwrap().forget();
        let backend = Backend::new(vec![Arc::clone(&native)], /*starts*/ 1);
        let provider = provider(backend);
        let scope = scope(&provider);
        let session = scope
            .open(request(), Arc::new(Reporter::default()))
            .await
            .unwrap();
        let updating = session.clone();
        let update = tokio::spawn(async move {
            updating.update_query(query(/*id*/ 1, "pending")).await
        });
        bounded(native.update_entered.acquire())
            .await
            .unwrap()
            .forget();

        assert_eq!(bounded(session.close()).await, outcome);
        assert_eq!(bounded(update).await.unwrap(), Err(cause));
        assert_eq!(lock(&provider.inner.state).sessions, 1);

        // The observer's error does not cancel the accepted update owner.
        native.updates.add_permits(1);
        assert_eq!(
            bounded(provider.shutdown()).await,
            SearchCloseOutcome {
                operation: outcome.operation,
                cleanup: CloseCleanup::Joined
            },
        );
        assert_eq!(lock(&provider.inner.state).sessions, 0);
    }
}

struct ImmediateUnchangedSession {
    native: Arc<Session>,
    polls: AtomicUsize,
    first_poll: Semaphore,
}

impl SearchBackendSession for ImmediateUnchangedSession {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        self.native.update_query(query)
    }

    fn next_snapshot(&self, after: u64, _: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(async move {
            let previous = self.polls.fetch_add(1, Ordering::SeqCst);
            if previous == 0 {
                self.first_poll.add_permits(1);
            }
            // A broken pump must fail this test rather than hang its executor.
            if previous >= 256 {
                return Err(SearchError::new(
                    SearchErrorKind::SearchFailed,
                    "no-progress poll pump exhausted the fairness safety limit",
                ));
            }
            Ok(SearchPoll::Unchanged { revision: after })
        })
    }

    fn request_close(&self) {
        self.native.request_close();
    }

    fn close(&self) -> SearchCloseFuture<'_> {
        self.native.close()
    }
}

#[tokio::test(start_paused = true)]
async fn immediate_unchanged_polls_allow_single_thread_updates_and_cancellation() {
    let native = Arc::new(ImmediateUnchangedSession {
        native: Session::new(/*closes*/ 1, joined()),
        polls: AtomicUsize::new(0),
        first_poll: Semaphore::new(0),
    });
    let backend = Arc::new(ReadyBackend {
        session: native.clone(),
        returned: Arc::new(Semaphore::new(0)),
        start: crate::pending_fixture::SingleStart::default(),
    });
    let provider = FileSearchProvider::from_backend(backend, policy()).unwrap();
    let scope = scope(&provider);
    let session = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    bounded(native.first_poll.acquire()).await.unwrap().forget();

    // Runnable unrelated work cannot cause a burst of immediate empty polls.
    // Paused time and these ready yields keep the pacing timer from expiring.
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    assert_eq!(native.polls.load(Ordering::SeqCst), 1);
    assert_eq!(
        bounded(session.update_query(query(/*id*/ 1, "still responsive"))).await,
        Ok(QueryAccepted {
            id: std::num::NonZeroU64::new(1).unwrap()
        }),
    );
    assert_eq!(bounded(session.close()).await, joined());
    assert_eq!(bounded(provider.shutdown()).await, joined());
}
