use std::time::Duration;

use codex_app_server_protocol::FuzzyFileSearchSessionError;
use codex_app_server_protocol::FuzzyFileSearchSessionErrorKind;
use codex_app_server_protocol::FuzzyFileSearchSessionFailedNotification;
use codex_app_server_protocol::ServerNotification;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::Delivery;
use super::InProcessServerEvent;
use super::forward;

#[tokio::test]
async fn terminal_search_failure_waits_for_queue_capacity_and_is_delivered_once() {
    let (sender, mut receiver) = mpsc::channel(1);
    sender
        .send(InProcessServerEvent::Lagged { skipped: 7 })
        .await
        .expect("occupy client queue");
    let failure = FuzzyFileSearchSessionFailedNotification {
        session_id: "active-search".to_string(),
        query: "alpha".to_string(),
        error: FuzzyFileSearchSessionError {
            kind: FuzzyFileSearchSessionErrorKind::SearchFailed,
            message: "selected provider failed".to_string(),
        },
    };
    let forwarding = forward(
        &sender,
        ServerNotification::FuzzyFileSearchSessionFailed(failure.clone()),
    );
    tokio::pin!(forwarding);
    assert!(futures::poll!(forwarding.as_mut()).is_pending());
    assert!(matches!(
        receiver.recv().await,
        Some(InProcessServerEvent::Lagged { skipped: 7 })
    ));
    assert_eq!(
        timeout(Duration::from_secs(2), forwarding)
            .await
            .expect("capacity now available"),
        Delivery::Delivered,
    );
    let Some(InProcessServerEvent::ServerNotification(notification)) = receiver.recv().await else {
        panic!("terminal failure must survive the full queue");
    };
    let ServerNotification::FuzzyFileSearchSessionFailed(delivered) = *notification else {
        panic!("received wrong notification");
    };
    assert_eq!(delivered, failure);
    assert!(receiver.try_recv().is_err());
}
