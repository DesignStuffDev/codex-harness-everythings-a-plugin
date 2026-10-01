//! Gated service receipts, not native OS-worker cleanup evidence.

use pretty_assertions::assert_eq;

use super::*;

fn closed_start() -> SearchError {
    SearchError::new(
        SearchErrorKind::ClosedLease,
        "backend closed during startup",
    )
}

fn exhausted_start() -> SearchError {
    SearchError::new(
        SearchErrorKind::ResourceExhausted,
        "startup resource exhausted",
    )
}

fn failing_start(operation: SearchError, cleanup: StartCleanup) -> Arc<Backend> {
    let mut backend = backend(
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: match &cleanup {
                StartCleanup::NotAdmitted | StartCleanup::Confirmed => CloseCleanup::Joined,
                StartCleanup::Unconfirmed(error) => CloseCleanup::Unconfirmed(error.clone()),
            },
        },
        0,
    );
    Arc::get_mut(&mut backend)
        .expect("sole fixture owner")
        .start_plan = StartPlan::Fail(SearchStartError { operation, cleanup });
    backend
}

fn cancelled_start(cleanup: StartCleanup) -> Arc<Backend> {
    let mut backend = failing_start(closed_start(), cleanup.clone());
    Arc::get_mut(&mut backend)
        .expect("sole fixture owner")
        .start_plan = StartPlan::ClosedByCancellation(SearchStartError {
        operation: closed_start(),
        cleanup,
    });
    backend
}

async fn opening(
    service: &SearchService,
    provider: &ProviderIdentity,
    backend: &Backend,
) -> (PendingServiceReply, LeaseIdentity) {
    let request = open(provider, 1);
    let identity = request.identity.clone();
    let reply = service
        .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
        .expect("open");
    backend
        .entered
        .acquire()
        .await
        .expect("startup entered")
        .forget();
    (reply, identity)
}

async fn rejected(pending: PendingServiceReply) -> SearchStartError {
    let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(pending).await;
    match response.reply {
        Reply::Error { error } => error,
        Reply::Ok { .. } => panic!("closed startup was acknowledged ready"),
    }
}

async fn close_receipt(pending: PendingServiceReply) -> SearchCloseOutcome {
    let response: WireReply<LeaseIdentity, WireCloseOutcome> = receive(pending).await;
    match response.reply {
        Reply::Ok { result } => result.into(),
        Reply::Error { error } => panic!("release failed: {error}"),
    }
}

#[tokio::test]
async fn explicit_release_before_known_closed_start_has_successful_close_receipt() {
    for cleanup in [StartCleanup::NotAdmitted, StartCleanup::Confirmed] {
        let backend = cancelled_start(cleanup.clone());
        let (service, provider) = service(Arc::clone(&backend)).await;
        let (opening, identity) = opening(&service, &provider, &backend).await;
        let releasing = release(&service, &identity);
        backend.starts.add_permits(1);
        assert_eq!(
            rejected(opening).await,
            SearchStartError {
                operation: closed_start(),
                cleanup,
            }
        );
        assert_eq!(close_receipt(releasing).await, joined());
        assert_eq!(close_receipt(release(&service, &identity)).await, joined());
        assert_eq!(lock(&service.inner.state).used, [0; 3]);
        assert_eq!(service.shutdown().await, joined());
    }
}

#[tokio::test]
async fn explicit_shutdown_before_known_closed_start_does_not_fail_provider() {
    for cleanup in [StartCleanup::NotAdmitted, StartCleanup::Confirmed] {
        let backend = cancelled_start(cleanup.clone());
        let (service, provider) = service(Arc::clone(&backend)).await;
        let (opening, _) = opening(&service, &provider, &backend).await;
        service.request_shutdown();
        backend.starts.add_permits(1);
        assert_eq!(
            rejected(opening).await,
            SearchStartError {
                operation: closed_start(),
                cleanup,
            }
        );
        assert_eq!(service.shutdown().await, joined());
        assert_eq!(lock(&service.inner.state).used, [0; 3]);
    }
}

#[tokio::test]
async fn spontaneous_closed_start_is_not_normalized_by_automatic_error_close() {
    for cleanup in [StartCleanup::NotAdmitted, StartCleanup::Confirmed] {
        let backend = failing_start(closed_start(), cleanup);
        let (service, provider) = service(Arc::clone(&backend)).await;
        let (opening, identity) = opening(&service, &provider, &backend).await;
        backend.starts.add_permits(1);
        assert_eq!(rejected(opening).await.operation, closed_start());
        let expected = SearchCloseOutcome {
            operation: Err(closed_start()),
            cleanup: CloseCleanup::Joined,
        };
        assert_eq!(close_receipt(release(&service, &identity)).await, expected);
        assert_eq!(service.shutdown().await, expected);
    }
}

#[tokio::test]
async fn explicit_release_does_not_erase_genuine_closed_start() {
    for cleanup in [StartCleanup::NotAdmitted, StartCleanup::Confirmed] {
        let backend = failing_start(closed_start(), cleanup.clone());
        let (service, provider) = service(Arc::clone(&backend)).await;
        let (opening, identity) = opening(&service, &provider, &backend).await;
        let releasing = release(&service, &identity);
        assert!(*lock(&backend.controls)[0].cancelled.borrow());
        backend.starts.add_permits(1);
        assert_eq!(
            rejected(opening).await,
            SearchStartError {
                operation: closed_start(),
                cleanup,
            }
        );
        let expected = SearchCloseOutcome {
            operation: Err(closed_start()),
            cleanup: CloseCleanup::Joined,
        };
        assert_eq!(close_receipt(releasing).await, expected);
        assert_eq!(close_receipt(release(&service, &identity)).await, expected);
        assert_eq!(service.shutdown().await, expected);
        assert_eq!(lock(&service.inner.state).used, [0; 3]);
    }
}

#[tokio::test]
async fn explicit_release_does_not_erase_genuine_start_failure() {
    for cleanup in [StartCleanup::NotAdmitted, StartCleanup::Confirmed] {
        let backend = failing_start(exhausted_start(), cleanup.clone());
        let (service, provider) = service(Arc::clone(&backend)).await;
        let (opening, identity) = opening(&service, &provider, &backend).await;
        let releasing = release(&service, &identity);
        backend.starts.add_permits(1);
        assert_eq!(
            rejected(opening).await,
            SearchStartError {
                operation: exhausted_start(),
                cleanup,
            }
        );
        let expected = SearchCloseOutcome {
            operation: Err(exhausted_start()),
            cleanup: CloseCleanup::Joined,
        };
        assert_eq!(close_receipt(releasing).await, expected);
        assert_eq!(service.shutdown().await, expected);
    }
}

#[tokio::test]
async fn explicit_release_does_not_turn_unconfirmed_closed_start_into_success() {
    let cleanup = CloseCleanup::Unconfirmed(failure());
    let backend = failing_start(closed_start(), StartCleanup::Unconfirmed(failure()));
    let (service, provider) = service(Arc::clone(&backend)).await;
    let (opening, identity) = opening(&service, &provider, &backend).await;
    let releasing = release(&service, &identity);
    backend.starts.add_permits(1);
    assert_eq!(
        rejected(opening).await,
        SearchStartError {
            operation: closed_start(),
            cleanup: StartCleanup::Unconfirmed(failure()),
        }
    );
    let expected = SearchCloseOutcome {
        operation: Err(closed_start()),
        cleanup,
    };
    assert_eq!(close_receipt(releasing).await, expected);
    assert_eq!(service.shutdown().await, expected);
    assert_eq!(lock(&service.inner.state).leases.len(), 1);
    assert_eq!(lock(&service.inner.state).used[0], 100);
}

#[tokio::test]
async fn cancelled_successful_start_reports_genuine_failure_found_during_close() {
    let expected = SearchCloseOutcome {
        operation: Err(exhausted_start()),
        cleanup: CloseCleanup::Joined,
    };
    let backend = backend(expected.clone(), 0);
    let (service, provider) = service(Arc::clone(&backend)).await;
    let (opening, identity) = opening(&service, &provider, &backend).await;
    let releasing = release(&service, &identity);
    backend.starts.add_permits(1);
    assert_eq!(
        rejected(opening).await,
        SearchStartError {
            operation: exhausted_start(),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(close_receipt(releasing).await, expected);
    assert_eq!(service.shutdown().await, expected);
}
