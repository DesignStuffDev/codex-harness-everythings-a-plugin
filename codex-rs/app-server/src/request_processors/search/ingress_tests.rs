//! Admission/queue contract tests. Full JSON-RPC transport/worker proof is a
//! separate real-host gate; these tests use the actual serialization queue.
use super::preparing_fixture::*;
use super::*;
use crate::connection_rpc_gate::ConnectionRpcGate;
use crate::request_serialization::QueuedInitializedRequest;
use crate::request_serialization::RequestSerializationQueueKey;
use crate::request_serialization::RequestSerializationQueues;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::time::timeout;

fn start(id: i64) -> ClientRequest {
    ClientRequest::FuzzyFileSearchSessionStart {
        request_id: RequestId::Integer(id),
        params: FuzzyFileSearchSessionStartParams {
            session_id: "same".into(),
            roots: vec!["/fixture".into()],
        },
    }
}
fn stop(id: i64) -> ClientRequest {
    ClientRequest::FuzzyFileSearchSessionStop {
        request_id: RequestId::Integer(id),
        params: FuzzyFileSearchSessionStopParams {
            session_id: "same".into(),
        },
    }
}
pub(super) async fn enqueue(
    queues: &RequestSerializationQueues,
    processor: &SearchRequestProcessor,
    connection_id: ConnectionId,
    connection: &Arc<SearchConnectionState>,
    gate: &Arc<ConnectionRpcGate>,
    request: ClientRequest,
) -> oneshot::Receiver<Result<(), JSONRPCErrorError>> {
    let mut admitted = None;
    gate.run(async {
        admitted = Some(processor.admit_search_request(connection_id, connection, &request));
    })
    .await;
    let admitted = admitted
        .expect("open ingress gate")
        .expect("reader admission");
    let (key, access) = RequestSerializationQueueKey::from_scope(
        connection_id,
        request.serialization_scope().expect("still serialized"),
    );
    let processor = processor.clone();
    let connection = connection.clone();
    let (reply, response) = oneshot::channel();
    queues
        .enqueue(
            key,
            access,
            QueuedInitializedRequest::new(gate.clone(), async move {
                let result = match request {
                    ClientRequest::FuzzyFileSearchSessionStart { params, .. } => processor
                        .start_search_admitted(connection_id, connection, params, admitted)
                        .await
                        .map(|_| ()),
                    ClientRequest::FuzzyFileSearchSessionStop { params, .. } => processor
                        .stop_search_admitted(connection_id, connection, params, admitted)
                        .await
                        .map(|_| ()),
                    ClientRequest::FuzzyFileSearchSessionUpdate { params, .. } => processor
                        .fuzzy_file_search_session_update_response(
                            connection_id,
                            connection,
                            params,
                        )
                        .await
                        .map(|_| ()),
                    _ => panic!("test only queues search session requests"),
                };
                let _ = reply.send(result);
            }),
        )
        .await;
    response
}
pub(super) async fn response(
    receiver: oneshot::Receiver<Result<(), JSONRPCErrorError>>,
) -> Result<(), JSONRPCErrorError> {
    timeout(Duration::from_secs(2), receiver)
        .await
        .expect("ordered reply deadline")
        .expect("request owner reply")
}

#[tokio::test]
async fn stop_intent_crosses_held_start_queue_but_its_reply_waits_for_real_cleanup() {
    let first = Start::new(Ok(()));
    let sibling = Start::new(Ok(()));
    let (processor, provider, _outgoing) = fixture(vec![first.clone(), sibling.clone()]);
    let first_connection = Arc::new(SearchConnectionState::default());
    let other_connection = Arc::new(SearchConnectionState::default());
    first.check_connection(&first_connection);
    sibling.check_connection(&other_connection);
    let gate = Arc::new(ConnectionRpcGate::new());
    let other_gate = Arc::new(ConnectionRpcGate::new());
    let queues = RequestSerializationQueues::default();
    let opened = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &first_connection,
        &gate,
        start(1),
    )
    .await;
    entered(&first).await;
    let other_opened = enqueue(
        &queues,
        &processor,
        ConnectionId(2),
        &other_connection,
        &other_gate,
        start(2),
    )
    .await;
    entered(&sibling).await;
    let mut stopped = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &first_connection,
        &gate,
        stop(3),
    )
    .await;
    cancelled(&first).await;
    assert!(
        !sibling.cancelled.is_cancelled(),
        "same ID on another connection is untouched"
    );
    assert!(
        !first
            .hook_under_fence
            .load(std::sync::atomic::Ordering::Acquire)
    );
    assert_eq!(stopped.try_recv(), Err(oneshot::error::TryRecvError::Empty));
    assert!(
        first_connection
            .state
            .lock()
            .expect("state")
            .pending
            .contains_key("same")
    );
    first.release.add_permits(1);
    assert!(
        response(opened)
            .await
            .expect_err("cancelled start")
            .message
            .contains("released before")
    );
    response(stopped).await.expect("actual joined stop");
    assert!(!sibling.cancelled.is_cancelled());
    let other_stopped = enqueue(
        &queues,
        &processor,
        ConnectionId(2),
        &other_connection,
        &other_gate,
        stop(4),
    )
    .await;
    cancelled(&sibling).await;
    sibling.release.add_permits(1);
    assert!(response(other_opened).await.is_err());
    response(other_stopped).await.expect("sibling cleanup");
    gate.shutdown().await;
    other_gate.shutdown().await;
    processor.shutdown().await.expect("all requests drained");
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[tokio::test]
async fn stop_latches_all_prior_queued_starts_but_never_a_later_start() {
    let constructor = Start::new(Ok(()));
    let (processor, provider, _outgoing) = fixture(vec![constructor.clone()]);
    let connection = Arc::new(SearchConnectionState::default());
    let gate = Arc::new(ConnectionRpcGate::new());
    let queues = RequestSerializationQueues::default();
    let (key, access) = RequestSerializationQueueKey::from_scope(
        ConnectionId(1),
        start(1).serialization_scope().expect("scope"),
    );
    let (hold, held) = oneshot::channel();
    queues
        .enqueue(
            key,
            access,
            QueuedInitializedRequest::new(gate.clone(), async move {
                held.await.expect("queue barrier");
            }),
        )
        .await;
    let first = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        start(1),
    )
    .await;
    let update = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        ClientRequest::FuzzyFileSearchSessionUpdate {
            request_id: RequestId::Integer(6),
            params: FuzzyFileSearchSessionUpdateParams {
                session_id: "same".into(),
                query: "alpha".into(),
            },
        },
    )
    .await;
    let second = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        start(2),
    )
    .await;
    let stopped = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        stop(3),
    )
    .await;
    let later = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        start(4),
    )
    .await;
    hold.send(()).expect("release only the queue barrier");
    assert!(
        response(first)
            .await
            .expect_err("first queued start retired")
            .message
            .contains("released before")
    );
    assert!(
        response(update)
            .await
            .expect_err("queued update cannot acquire retired session")
            .message
            .contains("not found")
    );
    assert!(
        response(second)
            .await
            .expect_err("second queued start retired")
            .message
            .contains("released before")
    );
    response(stopped)
        .await
        .expect("no admitted constructor before stop");
    entered(&constructor).await;
    assert!(
        !constructor.cancelled.is_cancelled(),
        "earlier stop is not a future tombstone"
    );
    let last = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        stop(5),
    )
    .await;
    cancelled(&constructor).await;
    constructor.release.add_permits(1);
    assert!(response(later).await.is_err());
    response(last).await.expect("later cleanup");
    gate.shutdown().await;
    processor.shutdown().await.expect("drained");
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[tokio::test]
async fn intent_capacity_refunds_unpolled_requests_and_stop_remains_admissible() {
    let (processor, provider, _outgoing) = fixture(Vec::new());
    let connection = Arc::new(SearchConnectionState::default());
    let mut tickets = Vec::new();
    for id in 0..connection::MAX_SEARCHES_PER_CONNECTION {
        tickets.push(
            processor
                .admit_search_request(ConnectionId(1), &connection, &start(id as i64))
                .expect("bounded intent"),
        );
    }
    assert!(
        processor
            .admit_search_request(ConnectionId(1), &connection, &start(17))
            .err()
            .expect("capacity error")
            .message
            .contains("queued-start capacity")
    );
    let stopped = processor
        .admit_search_request(ConnectionId(1), &connection, &stop(18))
        .expect("stop admission at capacity");
    drop(tickets);
    // The Stop bundle still retains passive receipt Arcs, so this checks actual
    // completion-based pruning rather than waiting for the last Arc to vanish.
    let next = processor
        .admit_search_request(ConnectionId(1), &connection, &start(19))
        .expect("unpolled owner drop refunds metadata");
    processor
        .stop_search_admitted(
            ConnectionId(1),
            connection.clone(),
            FuzzyFileSearchSessionStopParams {
                session_id: "same".into(),
            },
            stopped,
        )
        .await
        .expect("known unadmitted cleanup");
    drop(next);
    processor.shutdown().await.expect("drained no-work fixture");
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[tokio::test]
async fn queued_stop_retains_original_failed_start_receipt_after_entry_removal() {
    let failure = codex_file_search_api::SearchError::new(
        codex_file_search_api::SearchErrorKind::SearchFailed,
        "constructor failed before joined stop",
    );
    let constructor = Start::new(Err(failure.clone()));
    let (processor, provider, _outgoing) = fixture(vec![constructor.clone()]);
    let connection = Arc::new(SearchConnectionState::default());
    let gate = Arc::new(ConnectionRpcGate::new());
    let queues = RequestSerializationQueues::default();
    let opened = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        start(1),
    )
    .await;
    entered(&constructor).await;
    let stopped = enqueue(
        &queues,
        &processor,
        ConnectionId(1),
        &connection,
        &gate,
        stop(2),
    )
    .await;
    cancelled(&constructor).await;
    constructor.release.add_permits(1);
    assert!(
        response(opened)
            .await
            .expect_err("real startup failure")
            .message
            .contains(failure.message())
    );
    assert!(connection.state.lock().expect("state").sessions.is_empty());
    assert!(
        response(stopped)
            .await
            .expect_err("missing entry cannot erase its retained failure")
            .message
            .contains(failure.message())
    );
    gate.shutdown().await;
    assert!(processor.shutdown().await.is_err());
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Err(failure),
            cleanup: CloseCleanup::Joined
        }
    );
}
