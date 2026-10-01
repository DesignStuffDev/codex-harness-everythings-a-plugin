//! Forward required terminal notifications through the bounded client queue.

use codex_app_server_protocol::ServerNotification;
use tokio::sync::mpsc;
use tracing::warn;

use super::InProcessServerEvent;
use super::server_notification_requires_delivery;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Delivery {
    Delivered,
    Dropped,
    Closed,
}

pub(super) async fn forward(
    event_tx: &mpsc::Sender<InProcessServerEvent>,
    notification: ServerNotification,
) -> Delivery {
    if server_notification_requires_delivery(&notification) {
        if event_tx
            .send(InProcessServerEvent::ServerNotification(Box::new(
                notification,
            )))
            .await
            .is_err()
        {
            Delivery::Closed
        } else {
            Delivery::Delivered
        }
    } else {
        match event_tx.try_send(InProcessServerEvent::ServerNotification(Box::new(
            notification,
        ))) {
            Ok(()) => Delivery::Delivered,
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!("dropping in-process server notification (queue full)");
                Delivery::Dropped
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Delivery::Closed,
        }
    }
}

#[cfg(test)]
#[path = "in_process_notification_tests.rs"]
mod tests;
