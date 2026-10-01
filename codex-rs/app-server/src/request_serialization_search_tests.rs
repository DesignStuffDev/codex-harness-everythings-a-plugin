use super::*;
use pretty_assertions::assert_eq;
use tokio::sync::oneshot;
use tokio::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn same_search_id_serializes_only_within_its_own_connection() {
    let queues = RequestSerializationQueues::default();
    let scope = || ClientRequestSerializationScope::FuzzyFileSearchSession { session_id: "shared".into() };
    let (first_key, access) = RequestSerializationQueueKey::from_scope(ConnectionId(1), scope());
    let (second_key, second_access) = RequestSerializationQueueKey::from_scope(ConnectionId(2), scope());
    assert_eq!(access, second_access);
    let first_gate = Arc::new(ConnectionRpcGate::new());
    let second_gate = Arc::new(ConnectionRpcGate::new());
    let (started, started_rx) = oneshot::channel();
    let (release, released) = oneshot::channel();
    queues.enqueue(first_key.clone(), access, QueuedInitializedRequest::new(Arc::clone(&first_gate), async move {
        started.send(()).expect("started observer");
        released.await.expect("release request");
    })).await;
    timeout(Duration::from_secs(2), started_rx).await.expect("first request starts").expect("started signal");
    let (same_done, mut same_observer) = oneshot::channel();
    queues.enqueue(first_key, access, QueuedInitializedRequest::new(Arc::clone(&first_gate), async move {
        same_done.send(()).expect("same connection observer");
    })).await;
    let (other_done, other_observer) = oneshot::channel();
    queues.enqueue(second_key, access, QueuedInitializedRequest::new(Arc::clone(&second_gate), async move {
        other_done.send(()).expect("other connection observer");
    })).await;
    timeout(Duration::from_secs(2), other_observer).await.expect("other connection is independent").expect("other completion");
    assert_eq!(same_observer.try_recv(), Err(oneshot::error::TryRecvError::Empty));
    release.send(()).expect("release blocked first request");
    timeout(Duration::from_secs(2), same_observer).await.expect("same connection drains").expect("same completion");
    first_gate.shutdown().await;
    second_gate.shutdown().await;
}
