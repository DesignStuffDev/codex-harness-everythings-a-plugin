//! Real native normal-query regression plus cancellation-admission fences.
use super::ingress_tests::enqueue;
use super::ingress_tests::response;
use super::preparing_fixture::*;
use super::*;
use crate::connection_rpc_gate::ConnectionRpcGate;
use crate::outgoing_message::OutgoingEnvelope;
use crate::outgoing_message::OutgoingMessage;
use crate::request_serialization::RequestSerializationQueues;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn ordinary_start_update_and_later_restart_still_use_fifo_and_real_native_search() {
    let (processor, mut outgoing, provider) = super::tests::processor();
    let root = tempfile::tempdir().expect("native search root");
    std::fs::write(root.path().join("alpha-file.txt"), "alpha").expect("real searchable file");
    let connection = Arc::new(SearchConnectionState::default());
    let gate = Arc::new(ConnectionRpcGate::new());
    let queues = RequestSerializationQueues::default();
    for generation in 0..2 {
        let opened = enqueue(
            &queues,
            &processor,
            ConnectionId(1),
            &connection,
            &gate,
            ClientRequest::FuzzyFileSearchSessionStart {
                request_id: RequestId::Integer(generation * 3),
                params: FuzzyFileSearchSessionStartParams {
                    session_id: "native".into(),
                    roots: vec![root.path().to_string_lossy().into_owned()],
                },
            },
        )
        .await;
        // Queue Update before awaiting Start; no early update or replacement.
        let updated = enqueue(
            &queues,
            &processor,
            ConnectionId(1),
            &connection,
            &gate,
            ClientRequest::FuzzyFileSearchSessionUpdate {
                request_id: RequestId::Integer(generation * 3 + 1),
                params: FuzzyFileSearchSessionUpdateParams {
                    session_id: "native".into(),
                    query: "alpha".into(),
                },
            },
        )
        .await;
        response(opened).await.expect("ordered real native startup");
        response(updated)
            .await
            .expect("ordered real query acknowledgement");
        let files = timeout(Duration::from_secs(5), async {
            loop {
                let OutgoingEnvelope::ToConnection {
                    connection_id,
                    message: OutgoingMessage::AppServerNotification(envelope),
                    ..
                } = outgoing.recv().await.expect("native notification")
                else {
                    panic!("native search must remain connection scoped");
                };
                assert_eq!(connection_id, ConnectionId(1));
                if let ServerNotification::FuzzyFileSearchSessionUpdated(update) =
                    envelope.notification
                    && !update.files.is_empty()
                {
                    assert_eq!(update.session_id, "native");
                    break update
                        .files
                        .into_iter()
                        .map(|file| file.path)
                        .collect::<Vec<_>>();
                }
            }
        })
        .await
        .expect("actual native match");
        assert_eq!(files, vec!["alpha-file.txt".to_owned()]);
        let stopped = enqueue(
            &queues,
            &processor,
            ConnectionId(1),
            &connection,
            &gate,
            ClientRequest::FuzzyFileSearchSessionStop {
                request_id: RequestId::Integer(generation * 3 + 2),
                params: FuzzyFileSearchSessionStopParams {
                    session_id: "native".into(),
                },
            },
        )
        .await;
        response(stopped)
            .await
            .expect("actual native joined cleanup");
        // No previous-generation notification may satisfy the next assertion.
        while outgoing.try_recv().is_ok() {}
    }
    gate.shutdown().await;
    processor.shutdown().await.expect("native consumer drained");
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[tokio::test]
async fn closed_gate_and_wrong_connection_identity_cannot_issue_stop_intent() {
    let constructor = Start::new(Ok(()));
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
        ClientRequest::FuzzyFileSearchSessionStart {
            request_id: RequestId::Integer(1),
            params: FuzzyFileSearchSessionStartParams {
                session_id: "held".into(),
                roots: vec!["/fixture".into()],
            },
        },
    )
    .await;
    entered(&constructor).await;
    let stop = ClientRequest::FuzzyFileSearchSessionStop {
        request_id: RequestId::Integer(2),
        params: FuzzyFileSearchSessionStopParams {
            session_id: "held".into(),
        },
    };
    assert!(
        processor
            .admit_search_request(ConnectionId(2), &connection, &stop)
            .err()
            .expect("identity refusal")
            .message
            .contains("identity does not match")
    );
    assert!(!constructor.cancelled.is_cancelled());
    gate.close().await;
    let mut admission = None;
    gate.run(async {
        admission = Some(processor.admit_search_request(ConnectionId(1), &connection, &stop));
    })
    .await;
    assert!(
        admission.is_none(),
        "closed authenticated gate never invokes cancellation admission"
    );
    assert!(!constructor.cancelled.is_cancelled());
    // Explicit owning connection shutdown supplies cancellation for final drain.
    connection.request_shutdown();
    cancelled(&constructor).await;
    constructor.release.add_permits(1);
    assert!(response(opened).await.is_err());
    gate.shutdown().await;
    processor
        .shutdown()
        .await
        .expect("shutdown retains the accepted request");
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}
