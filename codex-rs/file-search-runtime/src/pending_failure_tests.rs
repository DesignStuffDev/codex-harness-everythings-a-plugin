//! Controlled contract failures test facade causality, not native OS cleanup.
//! These fixtures admit no backend work: startup only returns an error value.
use super::*;
use pretty_assertions::assert_eq;
use tokio::sync::Semaphore;

struct ErrorControl {
    entered: Semaphore,
    release: Arc<Semaphore>,
    hook_panics: bool,
    cancelled: std::sync::atomic::AtomicBool,
    receipt: SearchStartCancellationOutcome,
}
impl SearchStartControl for ErrorControl {
    fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        assert!(!self.hook_panics, "deliberately broken cancellation hook");
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.cancelled.store(true, Ordering::SeqCst);
        self.entered.add_permits(1);
        // The independently owned receipt route remains functional even when
        // this fixture's nonblocking hook violates its no-panic contract.
        let receipt = self.receipt.clone();
        let permit = self.release.clone();
        Box::pin(async move {
            permit.acquire().await.unwrap().forget();
            receipt
        })
    }
}
struct ErrorBackend {
    original: SearchError,
    control: Arc<ErrorControl>,
}
impl SearchBackend for ErrorBackend {
    fn begin_open(&self, _: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let original = self.original.clone();
        Ok(PendingSearchStart::new(
            self.control.clone(),
            Box::pin(async move {
                Err(SearchStartError {
                    operation: original,
                    cleanup: StartCleanup::NotAdmitted,
                })
            }),
        ))
    }
    fn request_shutdown(&self) {}
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async { joined() })
    }
}
async fn preserves_original(hook_panics: bool, kind: SearchErrorKind) {
    let root = fixture();
    let original = SearchError::new(kind, "original startup failure");
    let cleanup_error =
        SearchError::new(SearchErrorKind::SearchFailed, "later cancellation failure");
    let control = Arc::new(ErrorControl {
        entered: Semaphore::new(0),
        release: Arc::new(Semaphore::new(0)),
        hook_panics,
        cancelled: std::sync::atomic::AtomicBool::new(false),
        receipt: SearchStartCancellationOutcome {
            operation: Err(cleanup_error),
            cleanup: StartCleanup::NotAdmitted,
        },
    });
    let backend = Arc::new(ErrorBackend {
        original: original.clone(),
        control: control.clone(),
    });
    let provider = FileSearchProvider::from_backend(
        backend,
        RuntimePolicy {
            provider: ProviderLimits {
                max_scopes: nz(1),
                max_sessions: nz(1),
                resources: budget(1),
            },
            max_query_bytes: nz(256),
            max_roots_options_bytes: nz(65536),
            max_matches: nz(10),
            max_frame_retained_bytes: nz(128 * 1024),
            poll_wait: Duration::from_millis(20),
        },
    )
    .unwrap();
    let scope = provider
        .scope_factory()
        .new_scope(ScopeLimits {
            max_sessions: nz(1),
        })
        .unwrap();
    let pending = scope
        .begin_open(request(root.path()), Arc::new(Reporter::default()))
        .unwrap();
    let observed = pending.control();
    timeout(WAIT, control.entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    assert!(control.cancelled.load(Ordering::SeqCst));
    assert_eq!(
        lock(&first_lease(&provider).state).first_error,
        Some(original.clone())
    );
    assert_eq!(
        lock(&provider.inner.state).first_error,
        Some(original.clone())
    );
    control.release.add_permits(1);
    let expected = SearchStartCancellationOutcome {
        operation: Err(original.clone()),
        cleanup: StartCleanup::NotAdmitted,
    };
    let error = timeout(WAIT, pending.finish())
        .await
        .unwrap()
        .err()
        .unwrap();
    assert_eq!(
        error,
        SearchStartError {
            operation: original.clone(),
            cleanup: StartCleanup::NotAdmitted
        }
    );
    assert_eq!(observed.cancel_and_wait().await, expected);
    assert_eq!(
        scope.shutdown().await,
        SearchCloseOutcome {
            operation: Err(original.clone()),
            cleanup: CloseCleanup::Joined
        }
    );
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Err(original),
            cleanup: CloseCleanup::Joined
        }
    );
}
#[tokio::test]
async fn spontaneous_closed_startup_survives_a_distinct_delayed_cancellation_failure() {
    preserves_original(false, SearchErrorKind::ClosedLease).await;
}
#[tokio::test]
async fn known_resource_startup_failure_precedes_panicking_hook_in_all_ledgers() {
    preserves_original(true, SearchErrorKind::ResourceExhausted).await;
}
