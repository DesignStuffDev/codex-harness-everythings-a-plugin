#![allow(clippy::unwrap_used, clippy::expect_used)]

//! These tests exercise the production service's admission/retention state with
//! controllable backend completion. They do not prove native OS worker cleanup.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_file_search_api::*;
use pretty_assertions::assert_eq;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::Semaphore;

use crate::service::lock;
use crate::*;

struct Backend {
    starts: Semaphore,
    entered: Semaphore,
    start_failure: Option<SearchStartError>,
    session: Arc<Session>,
    shutdown_cleanup: CloseCleanup,
}

impl SearchBackend for Backend {
    fn open(&self, _: SearchOpen) -> SearchStartFuture<'_> {
        Box::pin(async move {
            self.entered.add_permits(1);
            self.starts.acquire().await.expect("start gate").forget();
            if let Some(error) = &self.start_failure {
                return Err(error.clone());
            }
            Ok(Arc::clone(&self.session) as Arc<dyn SearchBackendSession>)
        })
    }
    fn request_shutdown(&self) {
        self.session.request_close();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            SearchCloseOutcome {
                operation: Ok(()),
                cleanup: self.shutdown_cleanup.clone(),
            }
        })
    }
}

struct Session {
    polls: Semaphore,
    entered: Semaphore,
    close_calls: AtomicUsize,
    outcome: SearchCloseOutcome,
    snapshot: Option<SearchFrame>,
}

impl SearchBackendSession for Session {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        Box::pin(async move { Ok(QueryAccepted { id: query.id }) })
    }
    fn next_snapshot(&self, after_revision: u64, _: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(async move {
            self.entered.add_permits(1);
            self.polls.acquire().await.expect("poll gate").forget();
            Ok(match &self.snapshot {
                Some(frame) => SearchPoll::Changed(frame.clone()),
                None => SearchPoll::Unchanged {
                    revision: after_revision,
                },
            })
        })
    }
    fn request_close(&self) {
        self.polls.add_permits(1);
    }
    fn close(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            self.close_calls.fetch_add(1, Ordering::SeqCst);
            self.outcome.clone()
        })
    }
}

struct Factory(Arc<Backend>);
impl SearchBackendFactory for Factory {
    fn create(&self, _: BackendContext) -> BackendFactoryFuture<'_> {
        Box::pin(async { Ok(Arc::clone(&self.0) as Arc<dyn SearchBackend>) })
    }
}

fn budget() -> SearchBudget {
    SearchBudget {
        max_index_entries: NonZeroUsize::new(100).expect("positive"),
        max_index_bytes: NonZeroUsize::new(100_000).expect("positive"),
        max_worker_threads: NonZeroUsize::new(8).expect("positive"),
    }
}

fn backend(outcome: SearchCloseOutcome, starts: usize) -> Arc<Backend> {
    Arc::new(Backend {
        starts: Semaphore::new(starts),
        entered: Semaphore::new(0),
        start_failure: None,
        shutdown_cleanup: outcome.cleanup.clone(),
        session: Arc::new(Session {
            polls: Semaphore::new(0),
            entered: Semaphore::new(0),
            close_calls: AtomicUsize::new(0),
            outcome,
            snapshot: None,
        }),
    })
}

fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}
fn failure() -> SearchError {
    SearchError::new(SearchErrorKind::SearchFailed, "controlled backend failure")
}

async fn service(backend: Arc<Backend>) -> (SearchService, ProviderIdentity) {
    let base_dir = std::env::current_dir().expect("cwd");
    let mut limits = ServiceLimits::new(budget());
    limits.max_leases = 1;
    let service = SearchService::new(
        Arc::new(Factory(backend)),
        ServiceEnvironment {
            startup_dir: base_dir.clone(),
            plugin_config: serde_json::json!({}),
            state_dir: base_dir.clone(),
        },
        limits.clone(),
    )
    .expect("service");
    let identity = ProviderIdentity {
        provider_id: uuid::Uuid::new_v4().to_string(),
    };
    let request = InitializeRequest {
        contract_version: FILE_SEARCH_CONTRACT_VERSION,
        identity: identity.clone(),
        base_dir,
        requested_limits: limits,
    };
    let response: WireReply<ProviderIdentity, InitializeResponse> = receive(
        service
            .admit(INITIALIZE_METHOD, ServiceLane::Ordinary, json(request))
            .expect("admit"),
    )
    .await;
    assert!(matches!(response.reply, Reply::Ok { .. }));
    (service, identity)
}

fn open(provider: &ProviderIdentity, epoch: u64) -> OpenRequest {
    OpenRequest::from_native(
        LeaseIdentity {
            provider_id: provider.provider_id.clone(),
            lease_id: uuid::Uuid::new_v4().to_string(),
            session_epoch: WireU64(epoch),
        },
        SearchOpen {
            roots: vec![std::path::PathBuf::from(".")],
            options: FileSearchOptions::default(),
            budget: budget(),
        },
    )
    .expect("open dto")
}

fn json(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).expect("encode")
}
async fn receive<T: DeserializeOwned>(pending: PendingServiceReply) -> T {
    let value = tokio::time::timeout(Duration::from_secs(3), pending.wait())
        .await
        .expect("response deadline")
        .expect("response");
    serde_json::from_value(value).expect("typed response")
}
fn release(service: &SearchService, identity: &LeaseIdentity) -> PendingServiceReply {
    service
        .admit(
            RELEASE_METHOD,
            ServiceLane::Control,
            json(LeaseRequest {
                contract_version: FILE_SEARCH_CONTRACT_VERSION,
                identity: identity.clone(),
            }),
        )
        .expect("release")
}

#[tokio::test]
async fn release_finds_preparing_before_backend_task_runs_and_joins_late_start() {
    let backend = backend(joined(), 0);
    let (service, provider) = service(Arc::clone(&backend)).await;
    let request = open(&provider, 1);
    let identity = request.identity.clone();
    let opening = service
        .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
        .expect("admit open");
    let releasing = release(&service, &identity);
    assert_eq!(lock(&service.inner.state).leases.len(), 1);
    backend.entered.acquire().await.expect("entered").forget();
    assert_eq!(backend.session.close_calls.load(Ordering::SeqCst), 0);
    backend.starts.add_permits(1);
    let opened: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(opening).await;
    match opened.reply {
        Reply::Error { error } => assert_eq!(error.cleanup, StartCleanup::Confirmed),
        Reply::Ok { .. } => panic!("closed startup was acknowledged ready"),
    }
    let closed: WireReply<LeaseIdentity, WireCloseOutcome> = receive(releasing).await;
    assert!(matches!(closed.reply, Reply::Ok { result } if result.cleanup == CloseCleanup::Joined));
    assert_eq!(backend.session.close_calls.load(Ordering::SeqCst), 1);
    assert_eq!(lock(&service.inner.state).leases.len(), 0);
    assert_eq!(service.shutdown().await.cleanup, CloseCleanup::Joined);
}

#[tokio::test]
async fn dropped_poll_observer_keeps_slot_until_owned_operation_completes() {
    let backend = backend(joined(), 1);
    let (service, provider) = service(Arc::clone(&backend)).await;
    let request = open(&provider, 1);
    let identity = request.identity.clone();
    let _: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
            .expect("open"),
    )
    .await;
    let poll = PollRequest {
        contract_version: FILE_SEARCH_CONTRACT_VERSION,
        identity: identity.clone(),
        after_revision: WireU64(0),
        wait_ms: 1,
    };
    let observing = service
        .admit(POLL_METHOD, ServiceLane::Ordinary, json(&poll))
        .expect("poll");
    drop(observing);
    backend
        .session
        .entered
        .acquire()
        .await
        .expect("entered")
        .forget();
    let response: WireReply<LeaseIdentity, WirePoll> = receive(
        service
            .admit(POLL_METHOD, ServiceLane::Ordinary, json(poll))
            .expect("second poll"),
    )
    .await;
    assert!(
        matches!(response.reply, Reply::Error { error } if error.kind() == SearchErrorKind::ResourceExhausted)
    );
    let closed: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &identity)).await;
    assert!(matches!(closed.reply, Reply::Ok { result } if result.cleanup == CloseCleanup::Joined));
    assert_eq!(backend.session.close_calls.load(Ordering::SeqCst), 1);
    assert_eq!(service.shutdown().await.cleanup, CloseCleanup::Joined);
}

#[tokio::test]
async fn joined_failed_operation_refunds_quota_and_retains_failure() {
    let backend = backend(
        SearchCloseOutcome {
            operation: Err(failure()),
            cleanup: CloseCleanup::Joined,
        },
        2,
    );
    let (service, provider) = service(backend).await;
    let first = open(&provider, 1);
    let identity = first.identity.clone();
    let _: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(first))
            .expect("first"),
    )
    .await;
    let closed: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &identity)).await;
    assert!(
        matches!(closed.reply, Reply::Ok { result } if result.cleanup == CloseCleanup::Joined && matches!(result.operation, OperationOutcome::Error { .. }))
    );
    let second: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(open(&provider, 2)))
            .expect("second"),
    )
    .await;
    assert!(matches!(second.reply, Reply::Ok { .. }));
    let outcome = service.shutdown().await;
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert_eq!(outcome.operation, Err(failure()));
}

#[tokio::test]
async fn unconfirmed_close_keeps_reservation_and_fences_provider() {
    let backend = backend(
        SearchCloseOutcome {
            operation: Err(failure()),
            cleanup: CloseCleanup::Unconfirmed(failure()),
        },
        1,
    );
    let (service, provider) = service(backend).await;
    let request = open(&provider, 1);
    let identity = request.identity.clone();
    let _: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
            .expect("open"),
    )
    .await;
    let closed: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &identity)).await;
    assert!(
        matches!(closed.reply, Reply::Ok { result } if matches!(result.cleanup, CloseCleanup::Unconfirmed(_)))
    );
    let outcome = service.shutdown().await;
    assert!(matches!(outcome.cleanup, CloseCleanup::Unconfirmed(_)));
    assert_eq!(lock(&service.inner.state).leases.len(), 1);
    assert_eq!(lock(&service.inner.state).used[0], 100);
    let rejected: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(open(&provider, 2)))
            .expect("rejected"),
    )
    .await;
    assert!(
        matches!(rejected.reply, Reply::Error { error } if error.operation.kind() == SearchErrorKind::ClosedLease)
    );
}

#[path = "service_boundary_tests.rs"]
mod boundaries;

#[path = "service_startup_tests.rs"]
mod startup;
