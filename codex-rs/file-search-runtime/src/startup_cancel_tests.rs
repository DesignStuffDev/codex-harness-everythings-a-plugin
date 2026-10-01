//! A close fence preceding a rejected startup is different from an unsolicited
//! backend failure. Exercise both through retained facade ownership.

use pretty_assertions::assert_eq;
use tokio::sync::Semaphore;

use crate::FileSearchProvider;

use super::*;

struct RejectedStart {
    entered: Semaphore,
    release: Semaphore,
    error: SearchStartError,
}

impl SearchBackend for RejectedStart {
    fn open(&self, _: SearchOpen) -> SearchStartFuture<'_> {
        Box::pin(async move {
            self.entered.add_permits(1);
            self.release.acquire().await.unwrap().forget();
            Err(self.error.clone())
        })
    }
    fn request_shutdown(&self) {}
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async { joined() })
    }
}

#[derive(Clone, Copy)]
enum Fence {
    BeforeResult,
    AfterResult,
}

async fn check_start(error: SearchStartError, fence: Fence, expected: SearchCloseOutcome) {
    let backend = Arc::new(RejectedStart {
        entered: Semaphore::new(0),
        release: Semaphore::new(0),
        error: error.clone(),
    });
    let provider = FileSearchProvider::from_backend(backend.clone(), policy()).unwrap();
    let scope = scope(&provider);
    let opening_scope = scope.clone();
    let opening = tokio::spawn(async move {
        opening_scope
            .open(request(), Arc::new(Reporter::default()))
            .await
    });
    bounded(backend.entered.acquire()).await.unwrap().forget();
    let lease = lock(&provider.inner.state)
        .scopes
        .values()
        .next()
        .unwrap()
        .leases
        .values()
        .next()
        .unwrap()
        .clone();
    assert_eq!(lock(&provider.inner.state).sessions, 1);
    if matches!(fence, Fence::BeforeResult) {
        scope.request_shutdown();
    }
    backend.release.add_permits(1);
    let rejected = bounded(opening).await.unwrap().err().unwrap();
    assert_eq!(rejected.operation, error.operation);
    assert_eq!(rejected.cleanup, error.cleanup);
    assert_eq!(
        bounded(crate::state::observe(lease.completed.subscribe())).await,
        expected
    );
    assert_eq!(
        bounded(provider.shutdown()).await,
        SearchCloseOutcome {
            operation: expected.operation,
            cleanup: CloseCleanup::Joined
        },
    );
    assert_eq!(lock(&provider.inner.state).sessions, 0);
}

#[tokio::test]
async fn explicit_cancel_before_start_rejection_does_not_poison_joined_provider() {
    for cleanup in [StartCleanup::NotAdmitted, StartCleanup::Confirmed] {
        check_start(
            SearchStartError {
                operation: crate::state::closed(),
                cleanup,
            },
            Fence::BeforeResult,
            joined(),
        )
        .await;
    }
}

#[tokio::test]
async fn automatic_error_close_cannot_hide_unsolicited_closed_startup() {
    let error = crate::state::closed();
    check_start(
        SearchStartError {
            operation: error.clone(),
            cleanup: StartCleanup::NotAdmitted,
        },
        Fence::AfterResult,
        SearchCloseOutcome {
            operation: Err(error),
            cleanup: CloseCleanup::Joined,
        },
    )
    .await;
}

#[tokio::test]
async fn startup_cancel_preserves_resource_failure_and_cleanup_uncertainty() {
    let resource = SearchError::new(
        SearchErrorKind::ResourceExhausted,
        "actual startup resource failure",
    );
    check_start(
        SearchStartError {
            operation: resource.clone(),
            cleanup: StartCleanup::Confirmed,
        },
        Fence::BeforeResult,
        SearchCloseOutcome {
            operation: Err(resource),
            cleanup: CloseCleanup::Joined,
        },
    )
    .await;
    let closed = crate::state::closed();
    let cleanup = SearchError::new(
        SearchErrorKind::TransportLost,
        "startup cleanup unconfirmed",
    );
    check_start(
        SearchStartError {
            operation: closed.clone(),
            cleanup: StartCleanup::Unconfirmed(cleanup.clone()),
        },
        Fence::BeforeResult,
        SearchCloseOutcome {
            operation: Err(closed),
            cleanup: CloseCleanup::Unconfirmed(cleanup),
        },
    )
    .await;
}
