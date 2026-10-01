use super::*;
use crate::outgoing_message::OutgoingEnvelope;
use crate::outgoing_message::OutgoingMessage;
use codex_analytics::AnalyticsEventsClient;
use codex_app_server_protocol::ServerNotification;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::timeout;

fn processor() -> (SearchRequestProcessor, mpsc::Receiver<OutgoingEnvelope>) {
    let (sender, receiver) = mpsc::channel(16);
    let outgoing = Arc::new(OutgoingMessageSender::new(sender, AnalyticsEventsClient::disabled()));
    (SearchRequestProcessor::new(outgoing), receiver)
}

fn start(roots: &std::path::Path) -> FuzzyFileSearchSessionStartParams {
    FuzzyFileSearchSessionStartParams { session_id: "shared-id".into(), roots: vec![roots.to_string_lossy().into_owned()] }
}

fn update(query: &str) -> FuzzyFileSearchSessionUpdateParams {
    FuzzyFileSearchSessionUpdateParams { session_id: "shared-id".into(), query: query.into() }
}

#[tokio::test]
async fn same_ids_are_connection_scoped_and_disconnect_keeps_other_search_alive() {
    let (processor, mut receiver) = processor();
    let first = Arc::new(SearchConnectionState::default());
    let second = Arc::new(SearchConnectionState::default());
    let first_root = tempfile::tempdir().expect("first root");
    let second_root = tempfile::tempdir().expect("second root");
    std::fs::write(first_root.path().join("alpha-first.txt"), "first").expect("first file");
    std::fs::write(second_root.path().join("alpha-second.txt"), "second").expect("second file");
    processor.fuzzy_file_search_session_start_response(ConnectionId(1), Arc::clone(&first), start(first_root.path())).await.expect("first start");
    processor.fuzzy_file_search_session_start_response(ConnectionId(2), Arc::clone(&second), start(second_root.path())).await.expect("second start");
    processor.fuzzy_file_search_session_update_response(ConnectionId(1), Arc::clone(&first), update("alpha")).await.expect("first update");
    processor.fuzzy_file_search_session_update_response(ConnectionId(2), Arc::clone(&second), update("alpha")).await.expect("second update");
    let mut found = HashMap::new();
    timeout(Duration::from_secs(5), async {
        while found.len() != 2 {
            let OutgoingEnvelope::ToConnection { connection_id, message: OutgoingMessage::AppServerNotification(envelope), .. } = receiver.recv().await.expect("notification") else {
                panic!("search notifications must never broadcast");
            };
            if let ServerNotification::FuzzyFileSearchSessionUpdated(update) = envelope.notification
                && !update.files.is_empty()
            {
                found.insert(connection_id, update.files.into_iter().map(|file| file.path).collect::<Vec<_>>());
            }
        }
    }).await.expect("both native searches complete");
    assert_eq!(found, HashMap::from([(ConnectionId(1), vec!["alpha-first.txt".into()]), (ConnectionId(2), vec!["alpha-second.txt".into()])]));
    first.request_shutdown();
    processor.connection_closed(ConnectionId(1), &first).await;
    assert!(first.publishers.is_empty());
    assert!(processor.fuzzy_file_search_session_start_response(ConnectionId(1), Arc::clone(&first), start(first_root.path())).await.is_err());
    processor.fuzzy_file_search_session_update_response(ConnectionId(2), Arc::clone(&second), update("second")).await.expect("other connection remains active");
    processor.fuzzy_file_search_session_stop(ConnectionId(2), Arc::clone(&second), FuzzyFileSearchSessionStopParams { session_id: "shared-id".into() }).await.expect("joined stop");
    assert!(second.state.lock().expect("state").sessions.is_empty());
    processor.shutdown().await.expect("global cleanup");
}

#[tokio::test]
async fn abandoned_start_waiter_releases_reserved_startup_and_publisher() {
    let (processor, _receiver) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    {
        let request = processor.fuzzy_file_search_session_start_response(ConnectionId(1), Arc::clone(&connection), start(root.path()));
        tokio::pin!(request);
        assert!(futures::poll!(&mut request).is_pending());
        assert_eq!(connection.state.lock().expect("state").sessions.len(), 1);
        // Dropping the reply waiter must not drop ownership of accepted startup.
    }
    timeout(Duration::from_secs(5), processor.shutdown()).await.expect("shutdown deadline").expect("joined shutdown");
    assert!(connection.state.lock().expect("state").sessions.is_empty());
    assert!(connection.startups.is_empty());
    assert!(connection.publishers.is_empty());
    connection.native.shutdown().await.expect("native owner remains joined");
}

#[tokio::test]
async fn stop_before_start_acknowledgement_joins_startup_and_can_restart() {
    let (processor, _receiver) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    let request = processor.fuzzy_file_search_session_start_response(ConnectionId(1), Arc::clone(&connection), start(root.path()));
    tokio::pin!(request);
    assert!(futures::poll!(&mut request).is_pending());
    timeout(Duration::from_secs(5), processor.fuzzy_file_search_session_stop(ConnectionId(1), Arc::clone(&connection), FuzzyFileSearchSessionStopParams { session_id: "shared-id".into() }))
        .await.expect("stop deadline").expect("joined startup stop");
    assert!(request.await.is_err(), "released startup must not return a usable session");
    processor.fuzzy_file_search_session_start_response(ConnectionId(1), Arc::clone(&connection), start(root.path())).await.expect("restart after joined stop");
    processor.shutdown().await.expect("joined shutdown");
}

#[tokio::test]
async fn same_token_cancels_only_its_connection_and_old_guard_cannot_remove_new_owner() {
    let (processor, _receiver) = processor();
    let first = Arc::new(SearchConnectionState::default());
    let second = Arc::new(SearchConnectionState::default());
    // Retain admitted predecessor flags explicitly so cancellation identity does
    // not depend on whether a tiny filesystem traversal wins a timing race.
    let mut guards = Vec::new();
    for connection in [&first, &second] {
        let flag = Arc::new(AtomicBool::new(false));
        let id = {
            let mut state = connection.state.lock().expect("state");
            let id = state.admit().expect("admission");
            state.tokens.insert("same-token".into(), id);
            state.one_shots.insert(id, Arc::clone(&flag));
            id
        };
        guards.push(OneShotGuard { connection: Arc::clone(connection), id, token: Some("same-token".into()), cancellation: flag });
    }
    let response = processor.fuzzy_file_search(ConnectionId(1), Arc::clone(&first), FuzzyFileSearchParams { query: String::new(), roots: Vec::new(), cancellation_token: Some("same-token".into()) }).await.expect("empty search cancels its token without another slot");
    assert_eq!(response, FuzzyFileSearchResponse { files: Vec::new() });
    assert!(guards[0].cancellation.load(Ordering::Acquire));
    assert!(!guards[1].cancellation.load(Ordering::Acquire));
    let successor = {
        let mut state = first.state.lock().expect("state");
        let id = state.admit().expect("successor admission");
        state.tokens.insert("same-token".into(), id);
        state.one_shots.insert(id, Arc::new(AtomicBool::new(false)));
        id
    };
    drop(guards.remove(0));
    assert_eq!(first.state.lock().expect("state").tokens.get("same-token"), Some(&successor));
    drop(guards);
    processor.shutdown().await.expect("cleanup");
    second.shutdown().await.expect("unregistered fixture owner cleanup");
}

#[tokio::test]
async fn stopped_native_owner_returns_error_instead_of_empty_success() {
    let (processor, _receiver) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    connection.native.request_shutdown();
    let error = processor.fuzzy_file_search(ConnectionId(1), Arc::clone(&connection), FuzzyFileSearchParams { query: "alpha".into(), roots: vec![root.path().to_string_lossy().into_owned()], cancellation_token: None }).await.expect_err("native failure must propagate");
    assert!(error.message.contains("file-search owner is shutting down"));
    assert!(connection.state.lock().expect("state").one_shots.is_empty());
    processor.shutdown().await.expect("cleanup");
}

#[tokio::test]
async fn failed_native_start_joins_publisher_before_reply_and_does_not_leave_session() {
    let (processor, _receiver) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    connection.native.request_shutdown();
    let request = processor.fuzzy_file_search_session_start_response(ConnectionId(1), Arc::clone(&connection), start(root.path()));
    tokio::pin!(request);
    assert!(futures::poll!(&mut request).is_pending());
    processor.fuzzy_file_search_session_stop(ConnectionId(1), Arc::clone(&connection), FuzzyFileSearchSessionStopParams { session_id: "shared-id".into() }).await.expect("failed start cleanup joined");
    assert!(request.await.is_err());
    assert!(connection.publishers.is_empty());
    assert!(connection.state.lock().expect("state").sessions.is_empty());
    processor.shutdown().await.expect("cleanup");
}

#[tokio::test]
async fn cancellation_remains_available_at_capacity_without_releasing_existing_leases() {
    let (processor, _receiver) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let predecessor = Arc::new(AtomicBool::new(false));
    let unrelated = Arc::new(AtomicBool::new(false));
    let (first_id, count) = {
        let mut state = connection.state.lock().expect("state");
        let first_id = state.admit().expect("predecessor admission");
        state.tokens.insert("same-token".into(), first_id);
        state.one_shots.insert(first_id, Arc::clone(&predecessor));
        let unrelated_id = state.admit().expect("unrelated admission");
        state.tokens.insert("other-token".into(), unrelated_id);
        state.one_shots.insert(unrelated_id, Arc::clone(&unrelated));
        while let Ok(id) = state.admit() {
            state.one_shots.insert(id, Arc::new(AtomicBool::new(false)));
        }
        (first_id, state.one_shots.len())
    };
    let error = processor.fuzzy_file_search(ConnectionId(1), Arc::clone(&connection), FuzzyFileSearchParams {
        query: "alpha".into(), roots: vec!["/fixture".into()], cancellation_token: Some("same-token".into()),
    }).await.expect_err("capacity must reject new work but still request cancellation");
    assert!(error.message.contains("capacity"));
    assert!(predecessor.load(Ordering::Acquire));
    assert!(!unrelated.load(Ordering::Acquire));
    {
        let state = connection.state.lock().expect("state");
        assert_eq!((state.tokens.get("same-token"), state.one_shots.len()), (Some(&first_id), count));
    }
    predecessor.store(false, Ordering::Release);
    let response = processor.fuzzy_file_search(ConnectionId(1), Arc::clone(&connection), FuzzyFileSearchParams {
        query: String::new(), roots: vec!["/fixture".into()], cancellation_token: Some("same-token".into()),
    }).await.expect("empty-query cancellation needs no free slot");
    assert_eq!(response, FuzzyFileSearchResponse { files: Vec::new() });
    assert!(predecessor.load(Ordering::Acquire));
    assert!(!unrelated.load(Ordering::Acquire));
    assert_eq!(connection.state.lock().expect("state").one_shots.len(), count);
    processor.request_shutdown();
    assert!(predecessor.load(Ordering::Acquire));
    processor.shutdown().await.expect("shutdown remains available at capacity");
}

#[tokio::test]
async fn publisher_failure_survives_disconnect_and_fails_global_cleanup() {
    let (processor, receiver) = processor();
    let connection = Arc::new(SearchConnectionState::default());
    let root = tempfile::tempdir().expect("root");
    std::fs::write(root.path().join("alpha.txt"), "alpha").expect("search file");
    processor.fuzzy_file_search_session_start_response(ConnectionId(1), Arc::clone(&connection), start(root.path())).await.expect("start");
    drop(receiver);
    processor.fuzzy_file_search_session_update_response(ConnectionId(1), Arc::clone(&connection), update("alpha")).await.expect("update admitted");
    timeout(Duration::from_secs(5), async {
        while connection.failures.result().is_ok() { tokio::task::yield_now().await; }
    }).await.expect("publisher reports the closed queue");
    processor.connection_closed(ConnectionId(1), &connection).await;
    assert!(connection.publishers.is_empty());
    assert!(processor.state.lock().expect("processor state").connections.is_empty());
    let error = processor.shutdown().await.expect_err("disconnect must retain cleanup failure after removing connection");
    assert!(format!("{error:#}").contains("queue is closed"));
}
