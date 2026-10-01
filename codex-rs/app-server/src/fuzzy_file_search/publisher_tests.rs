use super::*;
use crate::outgoing_message::OutgoingEnvelope;
use crate::outgoing_message::OutgoingMessage;
use codex_analytics::AnalyticsEventsClient;
use codex_file_search_api::FileMatch;
use codex_file_search_api::MatchType;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::timeout;

fn snapshot(query_id: u64, query: &str, path: &str) -> FileSearchSnapshot {
    FileSearchSnapshot {
        query_id,
        query: query.to_owned(),
        matches: vec![FileMatch {
            score: 10,
            path: path.into(),
            root: "/fixture".into(),
            match_type: MatchType::File,
            indices: Some(vec![0]),
        }],
        total_match_count: 1,
        scanned_file_count: 1,
        walk_complete: true,
    }
}

fn notification(envelope: OutgoingEnvelope) -> (ConnectionId, ServerNotification) {
    match envelope {
        OutgoingEnvelope::ToConnection {
            connection_id,
            message: OutgoingMessage::AppServerNotification(envelope),
            ..
        } => (connection_id, envelope.notification),
        _ => panic!("search must target one connection"),
    }
}

async fn filled(receiver: &mpsc::Receiver<OutgoingEnvelope>) {
    timeout(Duration::from_secs(2), async {
        while receiver.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("publisher must admit its first notification");
}

#[tokio::test]
async fn close_cancels_blocked_admission_and_preserves_prior_notification_order() {
    let (sender, mut receiver) = mpsc::channel(1);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        sender,
        AnalyticsEventsClient::disabled(),
    ));
    let tasks = TaskTracker::new();
    let failures = PublisherFailures::default();
    let publisher = SearchPublisher::start(
        ConnectionId(7),
        "same-id".into(),
        Arc::clone(&outgoing),
        &tasks,
        failures.clone(),
        CancellationToken::new(),
    );
    let observer = Arc::clone(publisher.observer());
    let id = observer
        .set_query("alpha".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("query");
    observer.on_update(&snapshot(id, "alpha", "alpha.txt"));
    filled(&receiver).await;
    observer.on_complete_tagged(id);
    // Capacity stays occupied: joined close must cancel its blocked next send.
    timeout(Duration::from_secs(2), publisher.close())
        .await
        .expect("bounded close")
        .expect("joined close");
    tasks.close();
    tasks.wait().await;
    failures.result().expect("no publisher failure");
    let (connection, first) = notification(receiver.recv().await.expect("admitted update"));
    assert_eq!(connection, ConnectionId(7));
    assert!(matches!(
        first,
        ServerNotification::FuzzyFileSearchSessionUpdated(_)
    ));
    observer.on_update(&snapshot(id, "alpha", "late.txt"));
    observer.on_complete_tagged(id);
    assert!(
        receiver.try_recv().is_err(),
        "no completion or callback may enter after close"
    );
    // The handler's response is enqueued only after this joined close, so the
    // admitted update necessarily precedes it on the connection FIFO.
}

#[tokio::test]
async fn repeated_query_text_and_bounded_coalescing_keep_only_current_generation() {
    let (sender, mut receiver) = mpsc::channel(1);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        sender,
        AnalyticsEventsClient::disabled(),
    ));
    let tasks = TaskTracker::new();
    let publisher = SearchPublisher::start(
        ConnectionId(3),
        "search".into(),
        Arc::clone(&outgoing),
        &tasks,
        PublisherFailures::default(),
        CancellationToken::new(),
    );
    // Keep the publisher blocked while old/new callback generations accumulate.
    outgoing
        .reserve_server_notification_to_connection(ConnectionId(99))
        .await
        .expect("capacity")
        .send(ServerNotification::FuzzyFileSearchSessionCompleted(
            FuzzyFileSearchSessionCompletedNotification {
                session_id: "sentinel".into(),
            },
        ));
    let observer = publisher.observer();
    let old = observer
        .set_query("a".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("first query");
    observer.on_update(&snapshot(old, "a", "old.txt"));
    observer
        .set_query("b".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("second query");
    let current = observer
        .set_query("a".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("third query");
    for index in 0..1_000 {
        observer.on_update(&snapshot(current, "a", &format!("a-{index}.txt")));
    }
    observer.on_update(&snapshot(old, "a", "stale.txt"));
    observer.on_complete_tagged(old);
    observer.on_complete_tagged(current);
    receiver.recv().await.expect("sentinel");
    let (connection, update) = notification(
        timeout(Duration::from_secs(2), receiver.recv())
            .await
            .expect("update deadline")
            .expect("update"),
    );
    assert_eq!(connection, ConnectionId(3));
    let ServerNotification::FuzzyFileSearchSessionUpdated(update) = update else {
        panic!("update before completion");
    };
    assert_eq!(update.query, "a");
    assert_eq!(
        update
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec!["a-999.txt"]
    );
    let (connection, completed) = notification(
        timeout(Duration::from_secs(2), receiver.recv())
            .await
            .expect("completion deadline")
            .expect("completion"),
    );
    assert_eq!(connection, ConnectionId(3));
    assert!(matches!(
        completed,
        ServerNotification::FuzzyFileSearchSessionCompleted(_)
    ));
    observer.on_complete_tagged(current);
    publisher.close().await.expect("joined close");
    assert!(receiver.try_recv().is_err());
    tasks.close();
    tasks.wait().await;
}

#[tokio::test]
async fn closed_outgoing_queue_is_reported_by_close_and_owner() {
    let (sender, receiver) = mpsc::channel(1);
    drop(receiver);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        sender,
        AnalyticsEventsClient::disabled(),
    ));
    let tasks = TaskTracker::new();
    let failures = PublisherFailures::default();
    let publisher = SearchPublisher::start(
        ConnectionId(1),
        "search".into(),
        outgoing,
        &tasks,
        failures.clone(),
        CancellationToken::new(),
    );
    let id = publisher
        .observer()
        .set_query("query".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("query");
    publisher
        .observer()
        .on_update(&snapshot(id, "query", "query.txt"));
    timeout(Duration::from_secs(2), async {
        while failures.result().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("closed queue failure must be retained");
    assert!(
        publisher
            .close()
            .await
            .expect_err("close reports failure")
            .to_string()
            .contains("queue is closed")
    );
    tasks.close();
    tasks.wait().await;
    assert!(failures.result().is_err());
}

#[tokio::test]
async fn observer_failure_wakes_waiters_and_survives_close() {
    let observer = SearchObserver::new(Arc::new(AtomicBool::new(false)), CancellationToken::new());
    let query_id = observer
        .set_query("query".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("query");
    let mut changed = observer.subscribe();
    let failure = SearchError::new(SearchErrorKind::SearchFailed, "é".repeat(2_000));
    observer.on_error(&failure);
    timeout(Duration::from_secs(2), changed.changed())
        .await
        .expect("failure wakes one-shot observers")
        .expect("observer still owns sender");
    observer.request_close();
    observer.on_error(&SearchError::new(
        SearchErrorKind::TransportLost,
        "later cleanup failure",
    ));
    observer.on_update(&snapshot(query_id, "query", "late.txt"));
    observer.on_complete_tagged(query_id);
    assert_eq!(observer.error(), Some(failure.clone()));
    assert!(failure.message().len() <= SearchError::MAX_MESSAGE_BYTES);
    assert!(!observer.is_current(query_id));
    assert!(!observer.is_complete());
    assert!(observer.files().is_empty());
    let rejected = observer
        .set_query("new".into(), NonZeroUsize::new(1_024).unwrap())
        .expect_err("closed observer retains the original operation error");
    assert_eq!(rejected.downcast_ref::<SearchError>(), Some(&failure));
}

#[test]
fn oversized_query_does_not_replace_the_current_generation() {
    let observer = SearchObserver::new(Arc::new(AtomicBool::new(false)), CancellationToken::new());
    let first = observer
        .set_query("a".into(), NonZeroUsize::new(3).unwrap())
        .expect("first query");
    observer.on_update(&snapshot(first, "a", "a.txt"));
    let files = observer.files();
    let oversized = observer
        .set_query("😀".into(), NonZeroUsize::new(3).unwrap())
        .expect_err("query limit counts UTF-8 bytes");
    assert_eq!(
        oversized.downcast_ref::<SearchError>(),
        Some(&SearchError::new(
            SearchErrorKind::ResourceExhausted,
            "file search query exceeds the negotiated UTF-8 byte limit",
        ))
    );
    assert!(observer.is_current(first));
    assert_eq!(observer.files(), files);
    assert_eq!(observer.error(), None);
    let next = observer
        .set_query("é".into(), NonZeroUsize::new(2).unwrap())
        .expect("query at the byte limit");
    assert_eq!(next, first + 1);
    assert!(!observer.is_current(first));
    assert!(observer.is_current(next));
    assert!(observer.files().is_empty());
}

#[tokio::test]
async fn explicit_stop_suppresses_capacity_blocked_failure_and_completion() {
    let (sender, mut receiver) = mpsc::channel(1);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        sender,
        AnalyticsEventsClient::disabled(),
    ));
    let tasks = TaskTracker::new();
    let failures = PublisherFailures::default();
    let publisher = SearchPublisher::start(
        ConnectionId(4),
        "failed-search".into(),
        outgoing,
        &tasks,
        failures.clone(),
        CancellationToken::new(),
    );
    let observer = Arc::clone(publisher.observer());
    let query_id = observer
        .set_query("query".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("query");
    observer.on_update(&snapshot(query_id, "query", "query.txt"));
    filled(&receiver).await;
    // The update occupies the only output slot. Explicit stop must suppress
    // both a queued completion and the failure that supersedes it.
    observer.on_complete_tagged(query_id);
    let failure = SearchError::new(SearchErrorKind::SearchFailed, "matcher failed");
    observer.on_error(&failure);
    observer.on_complete_tagged(query_id);
    observer.request_close();
    let closed = timeout(Duration::from_secs(2), publisher.close())
        .await
        .expect("failure cancels blocked admission")
        .expect_err("close retains operation failure");
    assert_eq!(closed.downcast_ref::<SearchError>(), Some(&failure));
    tasks.close();
    tasks.wait().await;
    assert!(failures.result().is_err());
    let (connection, update) = notification(receiver.recv().await.expect("prior update"));
    assert_eq!(connection, ConnectionId(4));
    assert!(matches!(
        update,
        ServerNotification::FuzzyFileSearchSessionUpdated(_)
    ));
    assert!(
        receiver.try_recv().is_err(),
        "explicit stop suppresses pending Failed and Completed"
    );
}

#[tokio::test]
async fn failure_delivered_after_close_remains_observable() {
    let observer = SearchObserver::new(Arc::new(AtomicBool::new(false)), CancellationToken::new());
    observer.request_close();
    let mut changed = observer.subscribe();
    let failure = SearchError::new(SearchErrorKind::SearchFailed, "accepted work failed");
    observer.on_error(&failure);
    timeout(Duration::from_secs(2), changed.changed())
        .await
        .expect("late operation failure wakes observers")
        .expect("observer still owns sender");
    assert_eq!(observer.error(), Some(failure));
    assert!(!observer.is_complete());
}

#[tokio::test]
async fn first_failure_is_published_once_for_current_query_without_completion() {
    let (sender, mut receiver) = mpsc::channel(1);
    let outgoing = Arc::new(OutgoingMessageSender::new(
        sender,
        AnalyticsEventsClient::disabled(),
    ));
    let tasks = TaskTracker::new();
    let failures = PublisherFailures::default();
    // Occupy the connection queue while updates and failure are coalesced.
    outgoing
        .reserve_server_notification_to_connection(ConnectionId(99))
        .await
        .expect("capacity")
        .send(ServerNotification::FuzzyFileSearchSessionCompleted(
            FuzzyFileSearchSessionCompletedNotification {
                session_id: "sentinel".into(),
            },
        ));
    let publisher = SearchPublisher::start(
        ConnectionId(5),
        "current-session".into(),
        outgoing,
        &tasks,
        failures.clone(),
        CancellationToken::new(),
    );
    let observer = Arc::clone(publisher.observer());
    let old = observer
        .set_query("old".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("first query");
    observer.on_update(&snapshot(old, "old", "stale.txt"));
    let current = observer
        .set_query("current".into(), NonZeroUsize::new(1_024).unwrap())
        .expect("current query");
    observer.on_update(&snapshot(current, "current", "current.txt"));
    observer.on_complete_tagged(current);
    let failure = SearchError::new(SearchErrorKind::ResourceExhausted, "é".repeat(2_000));
    observer.on_error(&failure);
    observer.on_error(&SearchError::new(
        SearchErrorKind::TransportLost,
        "later error must not replace the first",
    ));
    observer.on_complete_tagged(current);
    receiver.recv().await.expect("sentinel");
    let (connection, delivered) = notification(
        timeout(Duration::from_secs(2), receiver.recv())
            .await
            .expect("failure admission deadline")
            .expect("failure notification"),
    );
    assert_eq!(connection, ConnectionId(5));
    let ServerNotification::FuzzyFileSearchSessionFailed(delivered) = delivered else {
        panic!("a failed query must publish Failed instead of Updated or Completed");
    };
    assert_eq!(
        delivered,
        FuzzyFileSearchSessionFailedNotification {
            session_id: "current-session".into(),
            query: "current".into(),
            error: FuzzyFileSearchSessionError {
                kind: FuzzyFileSearchSessionErrorKind::ResourceExhausted,
                message: failure.message().to_owned(),
            },
        }
    );
    let closed = timeout(Duration::from_secs(2), publisher.close())
        .await
        .expect("publisher exits after terminal failure")
        .expect_err("close retains the same operation failure");
    assert_eq!(closed.downcast_ref::<SearchError>(), Some(&failure));
    tasks.close();
    tasks.wait().await;
    assert!(failures.result().is_err());
    assert!(
        receiver.try_recv().is_err(),
        "failure notification is terminal"
    );
}
