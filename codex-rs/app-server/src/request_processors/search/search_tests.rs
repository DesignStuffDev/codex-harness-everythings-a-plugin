use super::*;
use crate::fuzzy_file_search::close_result;
use crate::outgoing_message::OutgoingEnvelope;
use crate::outgoing_message::OutgoingMessage;
use codex_analytics::AnalyticsEventsClient;
use codex_app_server_protocol::ServerNotification;
use codex_file_search::NativeBackendLimits;
use codex_file_search::NativeSearchBackend;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::RuntimePolicy;
use pretty_assertions::assert_eq;
use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::timeout;

fn processor() -> (
    SearchRequestProcessor,
    mpsc::Receiver<OutgoingEnvelope>,
    FileSearchProvider,
) {
    let positive = |value| NonZeroUsize::new(value).expect("fixture bound");
    let resources = SearchBudget {
        max_index_entries: positive(1024),
        max_index_bytes: positive(256 * 1024 * 1024),
        max_worker_threads: positive(64),
    };
    let policy = RuntimePolicy {
        provider: ProviderLimits {
            max_scopes: positive(16),
            max_sessions: positive(16),
            resources,
        },
        max_query_bytes: positive(64 * 1024),
        max_roots_options_bytes: positive(256 * 1024),
        max_matches: positive(50),
        max_frame_retained_bytes: positive(16 * 1024 * 1024),
        poll_wait: Duration::from_millis(50),
    };
    let backend = NativeSearchBackend::new(
        std::env::current_dir().expect("process base"),
        NativeBackendLimits {
            max_sessions: policy.provider.max_sessions,
            resources,
            max_query_bytes: policy.max_query_bytes,
            max_roots_options_bytes: policy.max_roots_options_bytes,
            max_matches: policy.max_matches,
            max_snapshot_bytes: positive(16 * 1024 * 1024),
            max_poll_wait: policy.poll_wait,
        },
    )
    .expect("real native backend");
    let provider =
        FileSearchProvider::from_backend(Arc::new(backend), policy).expect("runtime facade");
    let mut options = crate::fuzzy_file_search::options();
    options.threads = NonZeroUsize::MIN;
    let context = SearchContext {
        shutdown_requested: tokio_util::sync::CancellationToken::new(),
        factory: provider.scope_factory(),
        scope_limits: ScopeLimits {
            max_sessions: positive(16),
        },
        budget: SearchBudget {
            max_index_entries: positive(64),
            max_index_bytes: positive(16 * 1024 * 1024),
            max_worker_threads: positive(4),
        },
        options,
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

fn start(roots: &std::path::Path) -> FuzzyFileSearchSessionStartParams {
    FuzzyFileSearchSessionStartParams {
        session_id: "shared-id".into(),
        roots: vec![roots.to_string_lossy().into_owned()],
    }
}

fn update(query: &str) -> FuzzyFileSearchSessionUpdateParams {
    FuzzyFileSearchSessionUpdateParams {
        session_id: "shared-id".into(),
        query: query.into(),
    }
}

#[tokio::test]
async fn same_ids_are_connection_scoped_and_disconnect_keeps_other_search_alive() {
    let (processor, mut receiver, _provider) = processor();
    let first = Arc::new(SearchConnectionState::default());
    let second = Arc::new(SearchConnectionState::default());
    let first_root = tempfile::tempdir().expect("first root");
    let second_root = tempfile::tempdir().expect("second root");
    std::fs::write(first_root.path().join("alpha-first.txt"), "first").expect("first file");
    std::fs::write(second_root.path().join("alpha-second.txt"), "second").expect("second file");
    processor
        .fuzzy_file_search_session_start_response(
            ConnectionId(1),
            Arc::clone(&first),
            start(first_root.path()),
        )
        .await
        .expect("first start");
    processor
        .fuzzy_file_search_session_start_response(
            ConnectionId(2),
            Arc::clone(&second),
            start(second_root.path()),
        )
        .await
        .expect("second start");
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(1),
            Arc::clone(&first),
            update("alpha"),
        )
        .await
        .expect("first update");
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(2),
            Arc::clone(&second),
            update("alpha"),
        )
        .await
        .expect("second update");
    let mut found = HashMap::new();
    timeout(Duration::from_secs(5), async {
        while found.len() != 2 {
            let OutgoingEnvelope::ToConnection {
                connection_id,
                message: OutgoingMessage::AppServerNotification(envelope),
                ..
            } = receiver.recv().await.expect("notification")
            else {
                panic!("search notifications must never broadcast");
            };
            if let ServerNotification::FuzzyFileSearchSessionUpdated(update) = envelope.notification
                && !update.files.is_empty()
            {
                found.insert(
                    connection_id,
                    update
                        .files
                        .into_iter()
                        .map(|file| file.path)
                        .collect::<Vec<_>>(),
                );
            }
        }
    })
    .await
    .expect("both native searches complete");
    assert_eq!(
        found,
        HashMap::from([
            (ConnectionId(1), vec!["alpha-first.txt".into()]),
            (ConnectionId(2), vec!["alpha-second.txt".into()])
        ])
    );
    first.request_shutdown();
    processor.connection_closed(ConnectionId(1), &first).await;
    assert!(first.publishers.is_empty());
    assert!(
        processor
            .fuzzy_file_search_session_start_response(
                ConnectionId(1),
                Arc::clone(&first),
                start(first_root.path())
            )
            .await
            .is_err()
    );
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(2),
            Arc::clone(&second),
            update("second"),
        )
        .await
        .expect("other connection remains active");
    processor
        .fuzzy_file_search_session_stop(
            ConnectionId(2),
            Arc::clone(&second),
            FuzzyFileSearchSessionStopParams {
                session_id: "shared-id".into(),
            },
        )
        .await
        .expect("joined stop");
    assert!(second.state.lock().expect("state").sessions.is_empty());
    processor.shutdown().await.expect("global cleanup");
}

#[tokio::test]
async fn abandoned_start_waiter_releases_reserved_startup_and_publisher() {
    let (processor, _receiver, _provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    {
        let request = processor.fuzzy_file_search_session_start_response(
            ConnectionId(1),
            Arc::clone(&connection),
            start(root.path()),
        );
        tokio::pin!(request);
        assert!(futures::poll!(&mut request).is_pending());
        assert_eq!(connection.state.lock().expect("state").sessions.len(), 1);
        // Dropping the reply waiter must not drop ownership of accepted startup.
    }
    timeout(Duration::from_secs(5), processor.shutdown())
        .await
        .expect("shutdown deadline")
        .expect("joined shutdown");
    assert!(connection.state.lock().expect("state").sessions.is_empty());
    assert!(connection.startups.is_empty());
    assert!(connection.publishers.is_empty());
    close_result(connection.scope.get().expect("scope").shutdown().await)
        .expect("runtime scope remains joined");
}

#[tokio::test]
async fn stop_before_start_acknowledgement_joins_startup_and_can_restart() {
    let (processor, _receiver, _provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    let request = processor.fuzzy_file_search_session_start_response(
        ConnectionId(1),
        Arc::clone(&connection),
        start(root.path()),
    );
    tokio::pin!(request);
    assert!(futures::poll!(&mut request).is_pending());
    timeout(
        Duration::from_secs(5),
        processor.fuzzy_file_search_session_stop(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionStopParams {
                session_id: "shared-id".into(),
            },
        ),
    )
    .await
    .expect("stop deadline")
    .expect("joined startup stop");
    assert!(
        request.await.is_err(),
        "released startup must not return a usable session"
    );
    processor
        .fuzzy_file_search_session_start_response(
            ConnectionId(1),
            Arc::clone(&connection),
            start(root.path()),
        )
        .await
        .expect("restart after joined stop");
    processor.shutdown().await.expect("joined shutdown");
}

#[tokio::test]
async fn same_token_cancels_only_its_connection_and_old_guard_cannot_remove_new_owner() {
    let (processor, _receiver, _provider) = processor();
    let first = Arc::new(SearchConnectionState::default());
    let second = Arc::new(SearchConnectionState::default());
    // Retain admitted predecessor flags explicitly so cancellation identity does
    // not depend on whether a tiny filesystem traversal wins a timing race.
    let mut guards = Vec::new();
    for connection in [&first, &second] {
        let flag = Arc::new(PendingSearchObserver::new(
            Arc::new(AtomicBool::new(false)),
            tokio_util::sync::CancellationToken::new(),
        ));
        let id = {
            let mut state = connection.state.lock().expect("state");
            let id = state.admit().expect("admission");
            state.tokens.insert("same-token".into(), id);
            state.one_shots.insert(id, Arc::clone(&flag));
            id
        };
        guards.push(OneShotGuard {
            connection: Arc::clone(connection),
            id,
            token: Some("same-token".into()),
            cancellation: flag,
        });
    }
    let response = processor
        .fuzzy_file_search(
            ConnectionId(1),
            Arc::clone(&first),
            FuzzyFileSearchParams {
                query: String::new(),
                roots: Vec::new(),
                cancellation_token: Some("same-token".into()),
            },
        )
        .await
        .expect("empty search cancels its token without another slot");
    assert_eq!(response, FuzzyFileSearchResponse { files: Vec::new() });
    assert!(guards[0].cancellation.cancellation_requested());
    assert!(!guards[1].cancellation.cancellation_requested());
    let successor = {
        let mut state = first.state.lock().expect("state");
        let id = state.admit().expect("successor admission");
        state.tokens.insert("same-token".into(), id);
        state.one_shots.insert(
            id,
            Arc::new(PendingSearchObserver::new(
                Arc::new(AtomicBool::new(false)),
                tokio_util::sync::CancellationToken::new(),
            )),
        );
        id
    };
    drop(guards.remove(0));
    assert_eq!(
        first.state.lock().expect("state").tokens.get("same-token"),
        Some(&successor)
    );
    drop(guards);
    processor.shutdown().await.expect("cleanup");
    second
        .shutdown()
        .await
        .expect("unregistered fixture owner cleanup");
}

#[tokio::test]
async fn stopped_runtime_scope_returns_error_instead_of_empty_success() {
    let (processor, _receiver, _provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    processor
        .register(ConnectionId(1), &connection)
        .expect("register");
    connection.scope.get().expect("scope").request_shutdown();
    let error = processor
        .fuzzy_file_search(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchParams {
                query: "alpha".into(),
                roots: vec![root.path().to_string_lossy().into_owned()],
                cancellation_token: None,
            },
        )
        .await
        .expect_err("native failure must propagate");
    assert_eq!(error.code, crate::error_code::INTERNAL_ERROR_CODE);
    assert!(
        error
            .message
            .contains("file-search runtime ownership is closing"),
        "unexpected search failure: {error:?}"
    );
    assert!(connection.state.lock().expect("state").one_shots.is_empty());
    processor.shutdown().await.expect("cleanup");
}

#[tokio::test]
async fn failed_native_start_joins_publisher_before_reply_and_does_not_leave_session() {
    let (processor, _receiver, _provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    processor
        .register(ConnectionId(1), &connection)
        .expect("register");
    connection.scope.get().expect("scope").request_shutdown();
    let request = processor.fuzzy_file_search_session_start_response(
        ConnectionId(1),
        Arc::clone(&connection),
        start(root.path()),
    );
    tokio::pin!(request);
    assert!(futures::poll!(&mut request).is_pending());
    processor
        .fuzzy_file_search_session_stop(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionStopParams {
                session_id: "shared-id".into(),
            },
        )
        .await
        .expect("failed start cleanup joined");
    assert!(request.await.is_err());
    assert!(connection.publishers.is_empty());
    assert!(connection.state.lock().expect("state").sessions.is_empty());
    processor.shutdown().await.expect("cleanup");
}

#[tokio::test]
async fn cancellation_remains_available_at_capacity_without_releasing_existing_leases() {
    let (processor, _receiver, _provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let predecessor = Arc::new(AtomicBool::new(false));
    let unrelated = Arc::new(AtomicBool::new(false));
    let (first_id, count) = {
        let mut state = connection.state.lock().expect("state");
        let first_id = state.admit().expect("predecessor admission");
        state.tokens.insert("same-token".into(), first_id);
        state.one_shots.insert(
            first_id,
            Arc::new(PendingSearchObserver::new(
                Arc::clone(&predecessor),
                tokio_util::sync::CancellationToken::new(),
            )),
        );
        let unrelated_id = state.admit().expect("unrelated admission");
        state.tokens.insert("other-token".into(), unrelated_id);
        state.one_shots.insert(
            unrelated_id,
            Arc::new(PendingSearchObserver::new(
                Arc::clone(&unrelated),
                tokio_util::sync::CancellationToken::new(),
            )),
        );
        while let Ok(id) = state.admit() {
            state.one_shots.insert(
                id,
                Arc::new(PendingSearchObserver::new(
                    Arc::new(AtomicBool::new(false)),
                    tokio_util::sync::CancellationToken::new(),
                )),
            );
        }
        (first_id, state.one_shots.len())
    };
    let error = processor
        .fuzzy_file_search(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchParams {
                query: "alpha".into(),
                roots: vec!["/fixture".into()],
                cancellation_token: Some("same-token".into()),
            },
        )
        .await
        .expect_err("capacity must reject new work but still request cancellation");
    assert!(error.message.contains("capacity"));
    assert!(predecessor.load(Ordering::Acquire));
    assert!(!unrelated.load(Ordering::Acquire));
    {
        let state = connection.state.lock().expect("state");
        assert_eq!(
            (state.tokens.get("same-token"), state.one_shots.len()),
            (Some(&first_id), count)
        );
    }
    predecessor.store(false, Ordering::Release);
    let response = processor
        .fuzzy_file_search(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchParams {
                query: String::new(),
                roots: vec!["/fixture".into()],
                cancellation_token: Some("same-token".into()),
            },
        )
        .await
        .expect("empty-query cancellation needs no free slot");
    assert_eq!(response, FuzzyFileSearchResponse { files: Vec::new() });
    assert!(predecessor.load(Ordering::Acquire));
    assert!(!unrelated.load(Ordering::Acquire));
    assert_eq!(
        connection.state.lock().expect("state").one_shots.len(),
        count
    );
    processor.request_shutdown();
    assert!(predecessor.load(Ordering::Acquire));
    processor
        .shutdown()
        .await
        .expect("shutdown remains available at capacity");
}

#[tokio::test]
async fn publisher_failure_survives_disconnect_and_fails_global_cleanup() {
    let (processor, receiver, _provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    std::fs::write(root.path().join("alpha.txt"), "alpha").expect("search file");
    processor
        .fuzzy_file_search_session_start_response(
            ConnectionId(1),
            Arc::clone(&connection),
            start(root.path()),
        )
        .await
        .expect("start");
    drop(receiver);
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(1),
            Arc::clone(&connection),
            update("alpha"),
        )
        .await
        .expect("update admitted");
    timeout(Duration::from_secs(5), async {
        while connection.failures.result().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("publisher reports the closed queue");
    processor
        .connection_closed(ConnectionId(1), &connection)
        .await;
    assert!(connection.publishers.is_empty());
    assert!(
        processor
            .state
            .lock()
            .expect("processor state")
            .connections
            .is_empty()
    );
    let error = processor
        .shutdown()
        .await
        .expect_err("disconnect must retain cleanup failure after removing connection");
    assert!(format!("{error:#}").contains("queue is closed"));
}

#[tokio::test]
async fn registration_rejects_foreign_provider_and_reused_connection_identity() {
    let (first, _first_receiver, _first_provider) = processor();
    let (second, _second_receiver, _second_provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    assert!(
        connection.scope.get().is_none(),
        "default does not initialize any provider"
    );
    first
        .register(ConnectionId(1), &connection)
        .expect("first registration");
    let other = Arc::new(SearchConnectionState::default());
    assert!(first.register(ConnectionId(1), &other).is_err());
    assert!(first.register(ConnectionId(2), &connection).is_err());
    assert!(second.register(ConnectionId(1), &connection).is_err());
    assert!(
        other.scope.get().is_none(),
        "rejected identity does not consume a scope"
    );
    first.shutdown().await.expect("first cleanup");
    second.shutdown().await.expect("second cleanup");
}

#[tokio::test]
async fn oversized_ids_and_queries_reject_before_retaining_search_state() {
    let (processor, _receiver, _provider) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let oversized = "é".repeat(129);
    let error = processor
        .fuzzy_file_search_session_start_response(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionStartParams {
                session_id: oversized.clone(),
                roots: vec!["/fixture".into()],
            },
        )
        .await
        .expect_err("UTF-8 byte bound");
    assert!(error.message.contains("256-byte"));
    assert!(connection.scope.get().is_none());
    processor
        .fuzzy_file_search_session_update_response(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionUpdateParams {
                session_id: oversized.clone(),
                query: "x".into(),
            },
        )
        .await
        .expect_err("update id bound");
    processor
        .fuzzy_file_search_session_stop(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchSessionStopParams {
                session_id: oversized.clone(),
            },
        )
        .await
        .expect_err("stop id bound");
    processor
        .fuzzy_file_search(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchParams {
                query: "x".into(),
                roots: vec!["/fixture".into()],
                cancellation_token: Some(oversized),
            },
        )
        .await
        .expect_err("token id bound");
    assert!(connection.scope.get().is_none());
    processor
        .fuzzy_file_search(
            ConnectionId(1),
            Arc::clone(&connection),
            FuzzyFileSearchParams {
                query: "x".repeat(processor.context.max_query_bytes.get() + 1),
                roots: vec!["/fixture".into()],
                cancellation_token: None,
            },
        )
        .await
        .expect_err("query bound");
    assert!(connection.state.lock().expect("state").one_shots.is_empty());
    processor.shutdown().await.expect("cleanup");
}
