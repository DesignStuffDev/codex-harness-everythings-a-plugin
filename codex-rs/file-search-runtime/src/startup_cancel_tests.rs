//! A close fence preceding a rejected startup is different from an unsolicited
//! backend failure. Exercise both through retained facade ownership.

use pretty_assertions::assert_eq;
use tokio::sync::Semaphore;

use crate::FileSearchProvider;

use super::*;

struct RejectedStart {
    entered: Semaphore,
    release: Arc<Semaphore>,
    error: SearchStartError,
    origin: crate::pending_fixture::FailureOrigin,
    start: crate::pending_fixture::SingleStart,
}

impl SearchBackend for RejectedStart {
    fn begin_open(&self, _: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let release = Arc::clone(&self.release);
        let error = self.error.clone();
        let pending = self.start.begin(
            Box::pin(async move {
                release.acquire().await.unwrap().forget();
                Err(error)
            }),
            self.origin,
            /*returned*/ None,
        )?;
        self.entered.add_permits(1);
        Ok(pending)
    }
    fn request_shutdown(&self) {
        self.start.request_cancel();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.request_shutdown();
        Box::pin(async {
            self.start.completed().await;
            // Existing explicit provider recovery receipt, independent of the
            // startup lease's immutable cleanup certainty.
            joined()
        })
    }
}

#[derive(Clone, Copy)]
enum Fence {
    BeforeResult,
    AfterResult,
}

async fn check_start(
    error: SearchStartError,
    fence: Fence,
    origin: crate::pending_fixture::FailureOrigin,
    expected: SearchCloseOutcome,
) {
    let backend = Arc::new(RejectedStart {
        entered: Semaphore::new(0),
        release: Arc::new(Semaphore::new(0)),
        error: error.clone(),
        origin,
        start: crate::pending_fixture::SingleStart::default(),
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
            crate::pending_fixture::FailureOrigin::ExplicitCancellation,
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
        crate::pending_fixture::FailureOrigin::Genuine,
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
        crate::pending_fixture::FailureOrigin::Genuine,
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
        crate::pending_fixture::FailureOrigin::Genuine,
        SearchCloseOutcome {
            operation: Err(closed),
            cleanup: CloseCleanup::Unconfirmed(cleanup),
        },
    )
    .await;
}

#[tokio::test]
async fn explicit_cancel_cannot_erase_a_genuine_closed_startup_failure() {
    for cleanup in [StartCleanup::NotAdmitted, StartCleanup::Confirmed] {
        let error = crate::state::closed();
        check_start(
            SearchStartError {
                operation: error.clone(),
                cleanup,
            },
            Fence::BeforeResult,
            crate::pending_fixture::FailureOrigin::Genuine,
            SearchCloseOutcome {
                operation: Err(error),
                cleanup: CloseCleanup::Joined,
            },
        )
        .await;
    }
}
