//! Real stdio lifecycle regression against an adversarial protocol peer. These
//! receipts validate the adapter, not the independently built native worker.

use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use pretty_assertions::assert_eq;

use super::*;

fn exhausted() -> SearchError {
    SearchError::new(
        SearchErrorKind::ResourceExhausted,
        "original resource exhaustion",
    )
}

#[tokio::test]
async fn cancelled_startup_preserves_genuine_release_failure_with_joined_cleanup() {
    let fixture = Fixture::new(json!({"hold_open":true,"hold_release":true,"resource_error":true}));
    let backend = fixture.connect().await;
    let owner = Arc::clone(&backend);
    let request = fixture.request();
    let opening = tokio::spawn(async move { owner.open(request).await });
    fixture.wait("open", 1).await;
    backend.request_shutdown();
    fixture.wait("release_started", 1).await;
    fixture.gate("release_gate");
    assert_eq!(
        opening.await.unwrap().err().unwrap(),
        SearchStartError {
            operation: exhausted(),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(
        backend.shutdown().await,
        SearchCloseOutcome {
            operation: Err(exhausted()),
            cleanup: CloseCleanup::Joined,
        }
    );
    assert_eq!(lock(&backend.inner.state).leases.len(), 0);
    fixture.assert_reaped();
}

#[tokio::test]
async fn cancelled_startup_preserves_genuine_release_failure_and_cleanup_uncertainty() {
    let fixture = Fixture::new(
        json!({"hold_open":true,"hold_release":true,"resource_error":true,"unconfirmed":true}),
    );
    let backend = fixture.connect().await;
    let owner = Arc::clone(&backend);
    let request = fixture.request();
    let opening = tokio::spawn(async move { owner.open(request).await });
    fixture.wait("open", 1).await;
    backend.request_shutdown();
    fixture.wait("release_started", 1).await;
    fixture.gate("release_gate");
    let uncertainty = SearchError::new(SearchErrorKind::TransportLost, "cleanup proof unavailable");
    assert_eq!(
        opening.await.unwrap().err().unwrap(),
        SearchStartError {
            operation: exhausted(),
            cleanup: StartCleanup::Unconfirmed(uncertainty.clone()),
        }
    );
    assert_eq!(
        backend.shutdown().await,
        SearchCloseOutcome {
            operation: Err(exhausted()),
            cleanup: CloseCleanup::Unconfirmed(uncertainty),
        }
    );
    assert_eq!(lock(&backend.inner.state).leases.len(), 1);
    fixture.assert_reaped();
}

#[tokio::test]
async fn earlier_genuine_start_failure_is_not_replaced_by_release_failure() {
    let fixture = Fixture::new(json!({"open_error":true,"resource_error":true}));
    let backend = fixture.connect().await;
    let earlier = SearchError::new(SearchErrorKind::SearchFailed, "earlier startup failure");
    assert_eq!(
        backend.open(fixture.request()).await.err().unwrap(),
        SearchStartError {
            operation: earlier.clone(),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(
        backend.shutdown().await,
        SearchCloseOutcome {
            operation: Err(earlier),
            cleanup: CloseCleanup::Joined,
        }
    );
    fixture.assert_reaped();
}
