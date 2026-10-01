//! Controlled backend boundaries exercise the production App Server adapter.
//! These are lifecycle tests, not proof of an installed external provider.

use super::*;
use crate::outgoing_message::OutgoingEnvelope;
use crate::outgoing_message::OutgoingMessage;
use codex_analytics::AnalyticsEventsClient;
use codex_app_server_protocol::FuzzyFileSearchSessionErrorKind;
use codex_app_server_protocol::ServerNotification;
use codex_file_search_api::*;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::RuntimePolicy;
use pretty_assertions::assert_eq;
use std::num::NonZeroUsize;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

struct Backend(Arc<Session>);
impl SearchBackend for Backend {
    fn open(&self, _: SearchOpen) -> SearchStartFuture<'_> {
        let session = Arc::clone(&self.0);
        Box::pin(async move { Ok(session as Arc<dyn SearchBackendSession>) })
    }
    fn request_shutdown(&self) {
        self.0.request_close();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.0.close()
    }
}

struct Session {
    frames: watch::Sender<Option<SearchFrame>>,
    closing: CancellationToken,
    updates: Semaphore,
    entered_update: Semaphore,
    entered_close: Semaphore,
    close_gate: watch::Sender<bool>,
    close_outcome: SearchCloseOutcome,
}
impl Session {
    fn new(update_permits: usize, close_ready: bool) -> Arc<Self> {
        Arc::new(Self {
            frames: watch::channel(None).0,
            closing: CancellationToken::new(),
            updates: Semaphore::new(update_permits),
            entered_update: Semaphore::new(0),
            entered_close: Semaphore::new(0),
            close_gate: watch::channel(close_ready).0,
            close_outcome: SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined,
            },
        })
    }
}
impl SearchBackendSession for Session {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        Box::pin(async move {
            self.entered_update.add_permits(1);
            tokio::select! {
                biased;
                _ = self.closing.cancelled() => Err(SearchError::new(SearchErrorKind::ClosedLease, "fixture closed")),
                permit = self.updates.acquire() => {
                    permit.expect("update gate").forget();
                    Ok(QueryAccepted { id: query.id })
                }
            }
        })
    }
    fn next_snapshot(&self, after: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(async move {
            let mut frames = self.frames.subscribe();
            let deadline = tokio::time::Instant::now() + wait;
            loop {
                if let Some(frame) = frames.borrow_and_update().clone()
                    && frame.revision > after
                {
                    return Ok(SearchPoll::Changed(frame));
                }
                tokio::select! {
                    _ = self.closing.cancelled() => return Err(SearchError::new(SearchErrorKind::ClosedLease, "fixture closed")),
                    _ = frames.changed() => {},
                    _ = tokio::time::sleep_until(deadline) => return Ok(SearchPoll::Unchanged { revision: after }),
                }
            }
        })
    }
    fn request_close(&self) {
        self.closing.cancel();
    }
    fn close(&self) -> SearchCloseFuture<'_> {
        self.request_close();
        Box::pin(async move {
            self.entered_close.add_permits(1);
            let mut ready = self.close_gate.subscribe();
            while !*ready.borrow_and_update() {
                ready.changed().await.expect("retained close gate");
            }
            self.close_outcome.clone()
        })
    }
}

fn fixture(
    session: Arc<Session>,
) -> (
    SearchRequestProcessor,
    mpsc::Receiver<OutgoingEnvelope>,
    FileSearchProvider,
) {
    let positive = |value| NonZeroUsize::new(value).expect("fixture bound");
    let resources = SearchBudget {
        max_index_entries: positive(16),
        max_index_bytes: positive(1024 * 1024),
        max_worker_threads: positive(16),
    };
    let policy = RuntimePolicy {
        provider: ProviderLimits {
            max_scopes: positive(2),
            max_sessions: positive(16),
            resources,
        },
        max_query_bytes: positive(1024),
        max_roots_options_bytes: positive(1024),
        max_matches: positive(50),
        max_frame_retained_bytes: positive(64 * 1024),
        poll_wait: Duration::from_millis(20),
    };
    let provider =
        FileSearchProvider::from_backend(Arc::new(Backend(session)), policy).expect("runtime");
    let context = SearchContext {
        shutdown_requested: tokio_util::sync::CancellationToken::new(),
        factory: provider.scope_factory(),
        scope_limits: ScopeLimits {
            max_sessions: positive(16),
        },
        budget: SearchBudget {
            max_index_entries: NonZeroUsize::MIN,
            max_index_bytes: positive(1024),
            max_worker_threads: NonZeroUsize::MIN,
        },
        options: crate::fuzzy_file_search::options(),
        max_query_bytes: policy.max_query_bytes,
    };
    let (sender, receiver) = mpsc::channel(16);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        sender,
        AnalyticsEventsClient::disabled(),
    ));
    (
        SearchRequestProcessor::new(outgoing, context),
        receiver,
        provider,
    )
}

async fn started(processor: &SearchRequestProcessor, connection: &Arc<SearchConnectionState>) {
    processor
        .fuzzy_file_search_session_start_response(
            ConnectionId(1),
            Arc::clone(connection),
            FuzzyFileSearchSessionStartParams {
                session_id: "search".into(),
                roots: vec!["/fixture".into()],
            },
        )
        .await
        .expect("start");
}
fn update() -> FuzzyFileSearchSessionUpdateParams {
    FuzzyFileSearchSessionUpdateParams {
        session_id: "search".into(),
        query: "alpha".into(),
    }
}
async fn permit(semaphore: &Semaphore) {
    timeout(Duration::from_secs(2), semaphore.acquire())
        .await
        .expect("operation deadline")
        .expect("gate")
        .forget();
}

#[tokio::test]
async fn stop_during_pending_update_does_not_hold_connection_lock_and_joins_cleanup() {
    let session = Session::new(/*update_permits*/ 0, /*close_ready*/ false);
    let (processor, mut receiver, _provider) = fixture(Arc::clone(&session));
    let connection = Arc::new(SearchConnectionState::default());
    started(&processor, &connection).await;
    let updating_processor = processor.clone();
    let updating_connection = Arc::clone(&connection);
    let updating = tokio::spawn(async move {
        updating_processor
            .fuzzy_file_search_session_update_response(
                ConnectionId(1),
                updating_connection,
                update(),
            )
            .await
    });
    permit(&session.entered_update).await;
    let stopping = processor.fuzzy_file_search_session_stop(
        ConnectionId(1),
        Arc::clone(&connection),
        FuzzyFileSearchSessionStopParams {
            session_id: "search".into(),
        },
    );
    tokio::pin!(stopping);
    assert!(futures::poll!(&mut stopping).is_pending());
    permit(&session.entered_close).await;
    assert!(
        futures::poll!(&mut stopping).is_pending(),
        "stop must await joined cleanup"
    );
    session.close_gate.send_replace(true);
    timeout(Duration::from_secs(2), stopping)
        .await
        .expect("stop deadline")
        .expect("joined stop");
    assert!(updating.await.expect("update task").is_err());
    assert!(
        receiver.try_recv().is_err(),
        "explicit stop must suppress failure and completion"
    );
    processor.shutdown().await.expect("processor cleanup");
}

#[tokio::test]
async fn abandoned_one_shot_waiter_keeps_slot_until_cleanup_is_observed() {
    let session = Session::new(/*update_permits*/ 0, /*close_ready*/ false);
    let (processor, _receiver, _provider) = fixture(Arc::clone(&session));
    let connection = Arc::new(SearchConnectionState::default());
    let requesting_processor = processor.clone();
    let requesting_connection = Arc::clone(&connection);
    let request = tokio::spawn(async move {
        requesting_processor
            .fuzzy_file_search(
                ConnectionId(1),
                requesting_connection,
                FuzzyFileSearchParams {
                    query: "alpha".into(),
                    roots: vec!["/fixture".into()],
                    cancellation_token: Some("token".into()),
                },
            )
            .await
    });
    permit(&session.entered_update).await;
    request.abort();
    assert!(
        request
            .await
            .expect_err("request observer cancelled")
            .is_cancelled()
    );
    permit(&session.entered_close).await;
    assert_eq!(connection.state.lock().expect("state").one_shots.len(), 1);
    session.close_gate.send_replace(true);
    timeout(Duration::from_secs(2), async {
        loop {
            if connection.state.lock().expect("state").one_shots.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("retained request drains");
    processor.shutdown().await.expect("joined cleanup");
}

#[tokio::test]
async fn unsolicited_cancelled_terminal_emits_failed_without_completed() {
    let session = Session::new(/*update_permits*/ 1, /*close_ready*/ true);
    let (processor, mut receiver, _provider) = fixture(Arc::clone(&session));
    let connection = Arc::new(SearchConnectionState::default());
    started(&processor, &connection).await;
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(1),
            Arc::clone(&connection),
            update(),
        )
        .await
        .expect("actual ack");
    session.frames.send_replace(Some(SearchFrame {
        revision: 1,
        query_id: 1,
        query: "alpha".into(),
        snapshot: None,
        phase: SearchPhase::Cancelled,
    }));
    let envelope = timeout(Duration::from_secs(2), receiver.recv())
        .await
        .expect("failure deadline")
        .expect("failure notification");
    let OutgoingEnvelope::ToConnection {
        connection_id,
        message: OutgoingMessage::AppServerNotification(notification),
        ..
    } = envelope
    else {
        panic!("failure must target the original connection");
    };
    assert_eq!(connection_id, ConnectionId(1));
    let ServerNotification::FuzzyFileSearchSessionFailed(failure) = notification.notification
    else {
        panic!("unsolicited cancellation must not report completed");
    };
    assert_eq!(
        (failure.session_id, failure.query, failure.error.kind),
        (
            "search".into(),
            "alpha".into(),
            FuzzyFileSearchSessionErrorKind::ClosedLease
        )
    );
    assert!(
        processor
            .fuzzy_file_search_session_stop(
                ConnectionId(1),
                Arc::clone(&connection),
                FuzzyFileSearchSessionStopParams {
                    session_id: "search".into()
                }
            )
            .await
            .is_err()
    );
    assert!(receiver.try_recv().is_err());
    processor
        .shutdown()
        .await
        .expect_err("retained presentation failure survives stop");
}

#[tokio::test]
async fn one_shot_terminal_before_idle_returns_error_instead_of_hanging() {
    let session = Session::new(/*update_permits*/ 1, /*close_ready*/ true);
    let (processor, _receiver, _provider) = fixture(Arc::clone(&session));
    let connection = Arc::new(SearchConnectionState::default());
    let requesting_processor = processor.clone();
    let requesting_connection = Arc::clone(&connection);
    let request = tokio::spawn(async move {
        requesting_processor
            .fuzzy_file_search(
                ConnectionId(1),
                requesting_connection,
                FuzzyFileSearchParams {
                    query: "alpha".into(),
                    roots: vec!["/fixture".into()],
                    cancellation_token: None,
                },
            )
            .await
    });
    permit(&session.entered_update).await;
    session.frames.send_replace(Some(SearchFrame {
        revision: 1,
        query_id: 1,
        query: "alpha".into(),
        snapshot: None,
        phase: SearchPhase::Cancelled,
    }));
    timeout(Duration::from_secs(2), request)
        .await
        .expect("terminal wakeup deadline")
        .expect("request owner")
        .expect_err("terminal is not successful completion");
    assert!(connection.state.lock().expect("state").one_shots.is_empty());
    processor.shutdown().await.expect("joined cleanup");
}

#[tokio::test]
async fn startup_response_rechecks_generation_after_ready_was_stopped() {
    let session = Session::new(/*update_permits*/ 1, /*close_ready*/ true);
    let (processor, _receiver, _provider) = fixture(session);
    let connection = Arc::new(SearchConnectionState::default());
    let request = processor.fuzzy_file_search_session_start_response(
        ConnectionId(1),
        Arc::clone(&connection),
        FuzzyFileSearchSessionStartParams {
            session_id: "search".into(),
            roots: vec!["/fixture".into()],
        },
    );
    tokio::pin!(request);
    assert!(futures::poll!(&mut request).is_pending());
    timeout(Duration::from_secs(2), async {
        loop {
            if matches!(
                connection
                    .state
                    .lock()
                    .expect("state")
                    .sessions
                    .get("search"),
                Some(SessionEntry::Ready { .. })
            ) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("startup becomes ready without resuming response observer");
    processor
        .fuzzy_file_search_session_stop(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionStopParams {
                session_id: "search".into(),
            },
        )
        .await
        .expect("stop ready generation");
    request
        .await
        .expect_err("stale start response cannot acknowledge the stopped generation");
    processor.shutdown().await.expect("joined cleanup");
}

#[tokio::test]
async fn replacement_reserves_identity_and_stop_waits_for_predecessor_cleanup() {
    let session = Session::new(/*update_permits*/ 1, /*close_ready*/ false);
    let (processor, _receiver, _provider) = fixture(Arc::clone(&session));
    let connection = Arc::new(SearchConnectionState::default());
    started(&processor, &connection).await;
    let replacement = processor.fuzzy_file_search_session_start_response(
        ConnectionId(1),
        Arc::clone(&connection),
        FuzzyFileSearchSessionStartParams {
            session_id: "search".into(),
            roots: vec!["/fixture".into()],
        },
    );
    tokio::pin!(replacement);
    assert!(futures::poll!(&mut replacement).is_pending());
    permit(&session.entered_close).await;
    let duplicate = processor
        .fuzzy_file_search_session_start_response(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionStartParams {
                session_id: "search".into(),
                roots: vec!["/fixture".into()],
            },
        )
        .await
        .expect_err("concurrent replacement is bounded and explicit");
    assert!(duplicate.message.contains("already pending"));
    let stop = processor.fuzzy_file_search_session_stop(
        ConnectionId(1),
        Arc::clone(&connection),
        FuzzyFileSearchSessionStopParams {
            session_id: "search".into(),
        },
    );
    tokio::pin!(stop);
    assert!(futures::poll!(&mut stop).is_pending());
    assert_eq!(connection.state.lock().expect("state").pending.len(), 1);
    session.close_gate.send_replace(true);
    timeout(Duration::from_secs(2), stop)
        .await
        .expect("replacement cleanup deadline")
        .expect("joined stop");
    replacement
        .await
        .expect_err("stopped replacement cannot acknowledge start");
    assert!(connection.state.lock().expect("state").pending.is_empty());
    processor.shutdown().await.expect("joined cleanup");
}

#[cfg(test)]
#[path = "shutdown_race_tests.rs"]
mod shutdown_races;
