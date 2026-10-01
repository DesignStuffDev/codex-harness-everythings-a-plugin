//! Production service admission, cancellation publication and task-loss gates.
//! Controlled backends establish service ownership, not native worker behavior.

use pretty_assertions::assert_eq;
use std::sync::Mutex;

use super::*;

struct AnyFactory(Arc<dyn SearchBackend>);

impl SearchBackendFactory for AnyFactory {
    fn create(&self, _: BackendContext) -> BackendFactoryFuture<'_> {
        Box::pin(async { Ok(Arc::clone(&self.0)) })
    }
}

async fn configured(backend: Arc<dyn SearchBackend>) -> (Arc<SearchService>, ProviderIdentity) {
    let base_dir = std::env::current_dir().unwrap();
    let limits = ServiceLimits::new(SearchBudget {
        max_index_entries: NonZeroUsize::new(400).unwrap(),
        max_index_bytes: NonZeroUsize::new(400_000).unwrap(),
        max_worker_threads: NonZeroUsize::new(32).unwrap(),
    });
    let service = Arc::new(
        SearchService::new(
            Arc::new(AnyFactory(backend)),
            ServiceEnvironment {
                startup_dir: base_dir.clone(),
                plugin_config: serde_json::json!({}),
                state_dir: base_dir.clone(),
            },
            limits.clone(),
        )
        .unwrap(),
    );
    let identity = ProviderIdentity {
        provider_id: uuid::Uuid::new_v4().to_string(),
    };
    let response: WireReply<ProviderIdentity, InitializeResponse> = receive(
        service
            .admit(
                INITIALIZE_METHOD,
                ServiceLane::Ordinary,
                json(InitializeRequest {
                    contract_version: FILE_SEARCH_CONTRACT_VERSION,
                    identity: identity.clone(),
                    base_dir,
                    requested_limits: limits,
                }),
            )
            .unwrap(),
    )
    .await;
    assert!(matches!(response.reply, Reply::Ok { .. }));
    (service, identity)
}

struct RoutedBackend {
    backends: Vec<Arc<Backend>>,
    next: AtomicUsize,
}

impl SearchBackend for RoutedBackend {
    fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        self.backends[self.next.fetch_add(1, Ordering::SeqCst)].begin_open(request)
    }
    fn request_shutdown(&self) {
        for backend in &self.backends {
            backend.request_shutdown();
        }
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async {
            for backend in &self.backends {
                assert_eq!(backend.shutdown().await, joined());
            }
            joined()
        })
    }
}

async fn update(service: &SearchService, identity: &LeaseIdentity, epoch: u64) {
    let response: WireReply<LeaseIdentity, UpdateResponse> = receive(
        service
            .admit(
                UPDATE_METHOD,
                ServiceLane::Ordinary,
                json(UpdateRequest {
                    contract_version: FILE_SEARCH_CONTRACT_VERSION,
                    identity: identity.clone(),
                    query_epoch: WireU64(epoch),
                    query: format!("query {epoch}"),
                }),
            )
            .unwrap(),
    )
    .await;
    assert!(
        matches!(response.reply, Reply::Ok { result } if result.accepted_query_epoch == WireU64(epoch))
    );
}

#[tokio::test]
async fn preparing_cancel_reaches_one_control_and_keeps_live_sibling_usable() {
    let first = backend(joined(), 0);
    let sibling = backend(joined(), 1);
    let replacement = backend(joined(), 1);
    let router = Arc::new(RoutedBackend {
        backends: vec![
            Arc::clone(&first),
            Arc::clone(&sibling),
            Arc::clone(&replacement),
        ],
        next: AtomicUsize::new(0),
    });
    let (service, provider) = configured(router).await;
    let request = open(&provider, 1);
    let first_identity = request.identity.clone();
    let pending = service
        .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), first.entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    let request = open(&provider, 2);
    let sibling_identity = request.identity.clone();
    let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
            .unwrap(),
    )
    .await;
    assert!(matches!(response.reply, Reply::Ok { .. }));
    update(&service, &sibling_identity, 1).await;
    let releasing = release(&service, &first_identity);
    assert!(lock(&first.controls)[0].requests.load(Ordering::SeqCst) > 0);
    assert_eq!(
        lock(&sibling.controls)[0].requests.load(Ordering::SeqCst),
        0
    );
    assert_eq!(first.session.close_calls.load(Ordering::SeqCst), 0);
    assert_eq!(lock(&service.inner.state).leases.len(), 2);
    update(&service, &sibling_identity, 2).await;
    first.starts.add_permits(1);
    let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(pending).await;
    assert!(
        matches!(response.reply, Reply::Error { error } if error.cleanup == StartCleanup::Confirmed)
    );
    let response: WireReply<LeaseIdentity, WireCloseOutcome> = receive(releasing).await;
    assert!(
        matches!(response.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == joined())
    );
    assert_eq!(first.session.close_calls.load(Ordering::SeqCst), 1);
    assert_eq!(sibling.session.close_calls.load(Ordering::SeqCst), 0);
    update(&service, &sibling_identity, 3).await;
    let request = open(&provider, 3);
    let replacement_identity = request.identity.clone();
    let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
            .unwrap(),
    )
    .await;
    assert!(matches!(response.reply, Reply::Ok { .. }));
    let repeated: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &first_identity)).await;
    assert!(
        matches!(repeated.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == joined())
    );
    update(&service, &replacement_identity, 1).await;
    assert_eq!(service.shutdown().await, joined());
}

struct HookBackend {
    inner: Arc<Backend>,
    hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl SearchBackend for HookBackend {
    fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let hook = lock(&self.hook).take();
        if let Some(hook) = hook {
            hook();
        }
        self.inner.begin_open(request)
    }
    fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.inner.shutdown()
    }
}

#[tokio::test]
async fn release_during_begin_is_latched_before_control_publication() {
    let backend = backend(joined(), 0);
    let wrapper = Arc::new(HookBackend {
        inner: Arc::clone(&backend),
        hook: Mutex::new(None),
    });
    let (service, provider) = configured(wrapper.clone()).await;
    let request = open(&provider, 1);
    let identity = request.identity.clone();
    let weak = Arc::downgrade(&service);
    let reply = Arc::new(Mutex::new(None));
    let captured_reply = Arc::clone(&reply);
    *lock(&wrapper.hook) = Some(Box::new(move || {
        let service = weak.upgrade().unwrap();
        // Prove begin ran outside both locks before re-entering the real RELEASE
        // dispatch. try_lock makes an accidental lock violation fail promptly.
        let lease = {
            let state = service
                .inner
                .state
                .try_lock()
                .expect("no registry lock across begin");
            Arc::clone(state.leases.get(&identity.session_epoch.0).unwrap())
        };
        drop(lease.state.try_lock().expect("no lease lock across begin"));
        *lock(&captured_reply) = Some(release(&service, &identity));
    }));
    let pending = service
        .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), backend.entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    assert!(lock(&backend.controls)[0].requests.load(Ordering::SeqCst) > 0);
    backend.starts.add_permits(1);
    let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(pending).await;
    assert!(
        matches!(response.reply, Reply::Error { error } if error.cleanup == StartCleanup::Confirmed)
    );
    let releasing = lock(&reply).take().unwrap();
    let response: WireReply<LeaseIdentity, WireCloseOutcome> = receive(releasing).await;
    assert!(
        matches!(response.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == joined())
    );
    assert_eq!(backend.session.close_calls.load(Ordering::SeqCst), 1);
    assert_eq!(service.shutdown().await, joined());
}

#[tokio::test]
async fn unpolled_start_and_close_owner_loss_publish_quarantined_receipt() {
    let (service, lease, opening, releasing, backend) = tokio::task::spawn_blocking(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let backend = backend(joined(), 0);
        let (service, provider) = runtime.block_on(configured(backend.clone()));
        let request = open(&provider, 1);
        let identity = request.identity.clone();
        let (opening, releasing, lease) = {
            let _entered = runtime.enter();
            let opening = service
                .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
                .unwrap();
            let lease = Arc::clone(
                lock(&service.inner.state)
                    .leases
                    .get(&identity.session_epoch.0)
                    .unwrap(),
            );
            let releasing = release(&service, &identity);
            (opening, releasing, lease)
        };
        // Neither newly accepted task has been polled. Their guards must already
        // exist and publish failure before the task tracker tokens disappear.
        drop(runtime);
        (service, lease, opening, releasing, backend)
    })
    .await
    .unwrap();
    let lost = crate::service_lease::owner_lost();
    let expected = SearchCloseOutcome {
        operation: Err(lost.clone()),
        cleanup: CloseCleanup::Unconfirmed(lost.clone()),
    };
    assert!(opening.wait().await.is_err());
    let response: WireReply<LeaseIdentity, WireCloseOutcome> = receive(releasing).await;
    assert!(
        matches!(response.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == expected)
    );
    assert_eq!(lease.outcome().await, expected);
    assert_eq!(lease.outcome().await, expected);
    assert!(lock(&lease.state).start_finished);
    assert_eq!(
        lock(&lease.state).start_failure,
        Some(SearchStartError {
            operation: lost.clone(),
            cleanup: StartCleanup::Unconfirmed(lost),
        })
    );
    assert_eq!(backend.begin_calls.load(Ordering::SeqCst), 0);
    assert!(service.inner.tasks.is_empty());
    assert_eq!(lock(&service.inner.state).used, lease.allocation());
    assert_eq!(lock(&service.inner.state).leases.len(), 1);
    assert!(lock(&service.inner.state).closing);
    // Provider-wide teardown guards are a separate slice; this test observes the
    // exact lease receipts and does not claim a whole-provider shutdown join.
}

#[tokio::test]
async fn unpolled_close_after_ready_cannot_claim_joined_cleanup() {
    let (service, lease, releasing) = tokio::task::spawn_blocking(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let backend = backend(joined(), 1);
        let (service, provider) = runtime.block_on(configured(backend));
        let request = open(&provider, 1);
        let identity = request.identity.clone();
        runtime.block_on(async {
            let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
                service
                    .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
                    .unwrap(),
            )
            .await;
            assert!(matches!(response.reply, Reply::Ok { .. }));
        });
        let lease = Arc::clone(
            lock(&service.inner.state)
                .leases
                .get(&identity.session_epoch.0)
                .unwrap(),
        );
        let releasing = {
            let _entered = runtime.enter();
            release(&service, &identity)
        };
        drop(runtime);
        (service, lease, releasing)
    })
    .await
    .unwrap();
    let lost = crate::service_lease::owner_lost();
    let expected = SearchCloseOutcome {
        operation: Err(lost.clone()),
        cleanup: CloseCleanup::Unconfirmed(lost),
    };
    let response: WireReply<LeaseIdentity, WireCloseOutcome> = receive(releasing).await;
    assert!(
        matches!(response.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == expected)
    );
    assert_eq!(lease.outcome().await, expected);
    assert_eq!(lock(&service.inner.state).used, lease.allocation());
    assert_eq!(lock(&service.inner.state).leases.len(), 1);
    assert!(service.inner.tasks.is_empty());
}

struct PanickingControlBackend {
    inner: Arc<Backend>,
    panic_receipt: bool,
}

struct PanickingControl {
    inner: Arc<dyn SearchStartControl>,
    panic_receipt: bool,
}

impl SearchStartControl for PanickingControl {
    fn request_cancel(&self) {
        panic!("controlled request hook panic");
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        if self.panic_receipt {
            panic!("controlled receipt hook panic");
        }
        self.inner.cancel_and_wait()
    }
}

impl SearchBackend for PanickingControlBackend {
    fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let pending = self.inner.begin_open(request)?;
        let control = Arc::new(PanickingControl {
            inner: pending.control(),
            panic_receipt: self.panic_receipt,
        });
        Ok(PendingSearchStart::new(control, pending.finish()))
    }
    fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.inner.shutdown()
    }
}

#[tokio::test]
async fn independent_cancel_receipt_drains_after_request_hook_panics() {
    let backend = backend(joined(), 0);
    let (service, provider) = configured(Arc::new(PanickingControlBackend {
        inner: Arc::clone(&backend),
        panic_receipt: false,
    }))
    .await;
    let request = open(&provider, 1);
    let identity = request.identity.clone();
    let opening = service
        .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), backend.entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    let releasing = release(&service, &identity);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if lock(&backend.controls)[0].requests.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    backend.starts.add_permits(1);
    let expected = SearchCloseOutcome {
        operation: Err(crate::service::panic_error()),
        cleanup: CloseCleanup::Joined,
    };
    let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(opening).await;
    let expected_start = SearchStartError {
        operation: crate::service::panic_error(),
        cleanup: StartCleanup::Confirmed,
    };
    assert!(matches!(response.reply, Reply::Error { error } if error == expected_start));
    let response: WireReply<LeaseIdentity, WireCloseOutcome> = receive(releasing).await;
    assert!(
        matches!(response.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == expected)
    );
    assert_eq!(backend.session.close_calls.load(Ordering::SeqCst), 1);
    assert_eq!(lock(&service.inner.state).used, [0; 3]);
    assert_eq!(service.shutdown().await, expected);
}

#[tokio::test]
async fn failed_cancel_routes_publish_immutable_uncertainty_before_late_finish() {
    let backend = backend(joined(), 0);
    let (service, provider) = configured(Arc::new(PanickingControlBackend {
        inner: Arc::clone(&backend),
        panic_receipt: true,
    }))
    .await;
    let request = open(&provider, 1);
    let identity = request.identity.clone();
    let opening = service
        .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), backend.entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    let lease = Arc::clone(
        lock(&service.inner.state)
            .leases
            .get(&identity.session_epoch.0)
            .unwrap(),
    );
    let expected = SearchCloseOutcome {
        operation: Err(crate::service::panic_error()),
        cleanup: CloseCleanup::Unconfirmed(crate::service::panic_error()),
    };
    let response: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &identity)).await;
    assert!(
        matches!(response.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == expected)
    );
    assert!(!lock(&lease.state).start_finished);
    assert_eq!(lock(&service.inner.state).used, lease.allocation());
    assert_eq!(lock(&service.inner.state).leases.len(), 1);
    backend.starts.add_permits(1);
    let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(opening).await;
    assert!(
        matches!(response.reply, Reply::Error { error } if error.cleanup == StartCleanup::Unconfirmed(crate::service::panic_error()))
    );
    assert_eq!(lease.outcome().await, expected);
    // Separate whole-provider recovery may join after retained work drains. It
    // must never rewrite the lease uncertainty that the RELEASE caller observed.
    let provider_outcome = service.shutdown().await;
    assert_eq!(
        provider_outcome.operation,
        Err(crate::service::panic_error())
    );
    assert_eq!(lease.outcome().await, expected);
}

struct LaterCleanupFailureBackend {
    inner: Arc<Backend>,
    entered: Arc<Semaphore>,
    proceed: Arc<Semaphore>,
}

struct LaterCleanupFailureControl {
    inner: Arc<dyn SearchStartControl>,
    entered: Arc<Semaphore>,
    proceed: Arc<Semaphore>,
}

impl SearchStartControl for LaterCleanupFailureControl {
    fn request_cancel(&self) {
        self.inner.request_cancel();
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        let observed = self.inner.cancel_and_wait();
        let entered = Arc::clone(&self.entered);
        let proceed = Arc::clone(&self.proceed);
        Box::pin(async move {
            // Drain the underlying retained owner, then emulate a distinct loss
            // of cancellation receipt proof after startup's cause is observable.
            let _ = observed.await;
            entered.add_permits(1);
            proceed.acquire().await.unwrap().forget();
            let error = SearchError::new(
                SearchErrorKind::TransportLost,
                "later cancellation receipt failed",
            );
            SearchStartCancellationOutcome {
                operation: Err(error.clone()),
                cleanup: StartCleanup::Unconfirmed(error),
            }
        })
    }
}

impl SearchBackend for LaterCleanupFailureBackend {
    fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let pending = self.inner.begin_open(request)?;
        let control = Arc::new(LaterCleanupFailureControl {
            inner: pending.control(),
            entered: Arc::clone(&self.entered),
            proceed: Arc::clone(&self.proceed),
        });
        Ok(PendingSearchStart::new(control, pending.finish()))
    }
    fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.inner.shutdown()
    }
}

#[tokio::test]
async fn known_startup_failure_precedes_later_cancellation_receipt_loss() {
    for kind in [
        SearchErrorKind::ResourceExhausted,
        SearchErrorKind::ClosedLease,
    ] {
        let operation = SearchError::new(kind, "original startup failure");
        let mut backend = backend(joined(), 1);
        Arc::get_mut(&mut backend).unwrap().start_plan = StartPlan::Fail(SearchStartError {
            operation: operation.clone(),
            cleanup: StartCleanup::Confirmed,
        });
        let entered = Arc::new(Semaphore::new(0));
        let proceed = Arc::new(Semaphore::new(0));
        let (service, provider) = configured(Arc::new(LaterCleanupFailureBackend {
            inner: backend,
            entered: Arc::clone(&entered),
            proceed: Arc::clone(&proceed),
        }))
        .await;
        let request = open(&provider, 1);
        let identity = request.identity.clone();
        let opening = service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), entered.acquire())
            .await
            .unwrap()
            .unwrap()
            .forget();
        let lease = Arc::clone(
            lock(&service.inner.state)
                .leases
                .get(&identity.session_epoch.0)
                .unwrap(),
        );
        assert_eq!(
            lock(&lease.state).start_failure,
            Some(SearchStartError {
                operation: operation.clone(),
                cleanup: StartCleanup::Confirmed,
            })
        );
        assert_eq!(lock(&lease.state).first_failure, Some(operation.clone()));
        proceed.add_permits(1);
        let cleanup_error = SearchError::new(
            SearchErrorKind::TransportLost,
            "later cancellation receipt failed",
        );
        let expected = SearchCloseOutcome {
            operation: Err(operation.clone()),
            cleanup: CloseCleanup::Unconfirmed(cleanup_error.clone()),
        };
        let response: WireReply<LeaseIdentity, OpenResponse, SearchStartError> =
            receive(opening).await;
        let expected_start = SearchStartError {
            operation: operation.clone(),
            cleanup: StartCleanup::Unconfirmed(cleanup_error),
        };
        assert!(matches!(response.reply, Reply::Error { error } if error == expected_start));
        assert_eq!(lease.outcome().await, expected);
        let response: WireReply<LeaseIdentity, WireCloseOutcome> =
            receive(release(&service, &identity)).await;
        assert!(
            matches!(response.reply, Reply::Ok { result } if SearchCloseOutcome::from(result.clone()) == expected)
        );
        assert_eq!(service.shutdown().await.operation, Err(operation));
        assert_eq!(lease.outcome().await, expected);
    }
}

#[tokio::test]
async fn close_owner_loss_keeps_already_observed_startup_cause() {
    for kind in [
        SearchErrorKind::ResourceExhausted,
        SearchErrorKind::ClosedLease,
    ] {
        let operation = SearchError::new(kind, "startup cause before runtime loss");
        let original = operation.clone();
        let (service, lease, opening) = tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let mut backend = backend(joined(), 1);
            Arc::get_mut(&mut backend).unwrap().start_plan = StartPlan::Fail(SearchStartError {
                operation: original,
                cleanup: StartCleanup::Confirmed,
            });
            let entered = Arc::new(Semaphore::new(0));
            let (service, provider) =
                runtime.block_on(configured(Arc::new(LaterCleanupFailureBackend {
                    inner: backend,
                    entered: Arc::clone(&entered),
                    proceed: Arc::new(Semaphore::new(0)),
                })));
            let request = open(&provider, 1);
            let identity = request.identity.clone();
            let opening = runtime.block_on(async {
                let opening = service
                    .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
                    .unwrap();
                tokio::time::timeout(Duration::from_secs(3), entered.acquire())
                    .await
                    .unwrap()
                    .unwrap()
                    .forget();
                opening
            });
            let lease = Arc::clone(
                lock(&service.inner.state)
                    .leases
                    .get(&identity.session_epoch.0)
                    .unwrap(),
            );
            assert!(lock(&lease.state).start_finished);
            drop(runtime);
            (service, lease, opening)
        })
        .await
        .unwrap();
        assert!(opening.wait().await.is_err());
        assert_eq!(
            lease.outcome().await,
            SearchCloseOutcome {
                operation: Err(operation.clone()),
                cleanup: CloseCleanup::Unconfirmed(crate::service_lease::owner_lost()),
            }
        );
        assert_eq!(
            lock(&lease.state).start_failure,
            Some(SearchStartError {
                operation,
                cleanup: StartCleanup::Confirmed,
            })
        );
        assert_eq!(lock(&service.inner.state).used, lease.allocation());
        assert!(service.inner.tasks.is_empty());
    }
}
