use super::*;
use crate::outgoing_message::OutgoingEnvelope;
use crate::outgoing_message::OutgoingMessage;
use codex_analytics::AnalyticsEventsClient;
use codex_file_search::FileMatch;
use codex_file_search::MatchType;
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
        while receiver.is_empty() { tokio::task::yield_now().await; }
    }).await.expect("publisher must admit its first notification");
}

#[tokio::test]
async fn close_cancels_blocked_admission_and_preserves_prior_notification_order() {
    let (sender, mut receiver) = mpsc::channel(1);
    let outgoing = Arc::new(OutgoingMessageSender::new(sender, AnalyticsEventsClient::disabled()));
    let tasks = TaskTracker::new();
    let failures = PublisherFailures::default();
    let publisher = SearchPublisher::start(ConnectionId(7), "same-id".into(), Arc::clone(&outgoing), &tasks, failures.clone());
    let observer = Arc::clone(publisher.observer());
    let id = observer.set_query("alpha".into()).expect("query");
    observer.on_update(&snapshot(id, "alpha", "alpha.txt"));
    filled(&receiver).await;
    observer.on_complete_tagged(id);
    // Capacity stays occupied: joined close must cancel its blocked next send.
    timeout(Duration::from_secs(2), publisher.close()).await.expect("bounded close").expect("joined close");
    tasks.close();
    tasks.wait().await;
    failures.result().expect("no publisher failure");
    let (connection, first) = notification(receiver.recv().await.expect("admitted update"));
    assert_eq!(connection, ConnectionId(7));
    assert!(matches!(first, ServerNotification::FuzzyFileSearchSessionUpdated(_)));
    observer.on_update(&snapshot(id, "alpha", "late.txt"));
    observer.on_complete_tagged(id);
    assert!(receiver.try_recv().is_err(), "no completion or callback may enter after close");
    // The handler's response is enqueued only after this joined close, so the
    // admitted update necessarily precedes it on the connection FIFO.
}

#[tokio::test]
async fn repeated_query_text_and_bounded_coalescing_keep_only_current_generation() {
    let (sender, mut receiver) = mpsc::channel(1);
    let outgoing = Arc::new(OutgoingMessageSender::new(sender, AnalyticsEventsClient::disabled()));
    let tasks = TaskTracker::new();
    let publisher = SearchPublisher::start(ConnectionId(3), "search".into(), Arc::clone(&outgoing), &tasks, PublisherFailures::default());
    // Keep the publisher blocked while old/new callback generations accumulate.
    outgoing.reserve_server_notification_to_connection(ConnectionId(99)).await.expect("capacity").send(
        ServerNotification::FuzzyFileSearchSessionCompleted(FuzzyFileSearchSessionCompletedNotification { session_id: "sentinel".into() }),
    );
    let observer = publisher.observer();
    let old = observer.set_query("a".into()).expect("first query");
    observer.on_update(&snapshot(old, "a", "old.txt"));
    observer.set_query("b".into()).expect("second query");
    let current = observer.set_query("a".into()).expect("third query");
    for index in 0..1_000 {
        observer.on_update(&snapshot(current, "a", &format!("a-{index}.txt")));
    }
    observer.on_update(&snapshot(old, "a", "stale.txt"));
    observer.on_complete_tagged(old);
    observer.on_complete_tagged(current);
    receiver.recv().await.expect("sentinel");
    let (connection, update) = notification(timeout(Duration::from_secs(2), receiver.recv()).await.expect("update deadline").expect("update"));
    assert_eq!(connection, ConnectionId(3));
    let ServerNotification::FuzzyFileSearchSessionUpdated(update) = update else { panic!("update before completion"); };
    assert_eq!(update.query, "a");
    assert_eq!(update.files.iter().map(|file| file.path.as_str()).collect::<Vec<_>>(), vec!["a-999.txt"]);
    let (connection, completed) = notification(timeout(Duration::from_secs(2), receiver.recv()).await.expect("completion deadline").expect("completion"));
    assert_eq!(connection, ConnectionId(3));
    assert!(matches!(completed, ServerNotification::FuzzyFileSearchSessionCompleted(_)));
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
    let outgoing = Arc::new(OutgoingMessageSender::new(sender, AnalyticsEventsClient::disabled()));
    let tasks = TaskTracker::new();
    let failures = PublisherFailures::default();
    let publisher = SearchPublisher::start(ConnectionId(1), "search".into(), outgoing, &tasks, failures.clone());
    let id = publisher.observer().set_query("query".into()).expect("query");
    publisher.observer().on_update(&snapshot(id, "query", "query.txt"));
    timeout(Duration::from_secs(2), async {
        while failures.result().is_ok() { tokio::task::yield_now().await; }
    }).await.expect("closed queue failure must be retained");
    assert!(publisher.close().await.expect_err("close reports failure").to_string().contains("queue is closed"));
    tasks.close();
    tasks.wait().await;
    assert!(failures.result().is_err());
}
