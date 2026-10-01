//! Retained ownership for a single provider connection. Admission is synchronous:
//! a received open is visible to release before its asynchronous backend starts.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use serde_json::Value;
use tokio::sync::watch;
use tokio_util::task::TaskTracker;

use crate::LeaseIdentity;
use crate::ProviderIdentity;
use crate::ServiceLimits;
use crate::contract::invalid;
use crate::service_lease::Lease;

/// Factory inputs are explicit and fixed for the process lifetime. Configuration
/// can contain secrets, so neither context nor environment implements Debug.
#[derive(Clone)]
pub struct BackendContext {
    pub base_dir: PathBuf,
    pub plugin_config: Value,
    pub state_dir: PathBuf,
    pub limits: ServiceLimits,
}

pub type BackendFactoryFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Arc<dyn SearchBackend>, SearchStartError>> + Send + 'a>>;

/// Constructs the chosen backend once from explicit composition policy. A
/// failed factory must retain accepted cleanup independently of dropped waiters
/// and report whether startup admitted work, joined it, or lost cleanup proof.
pub trait SearchBackendFactory: Send + Sync {
    fn create(&self, context: BackendContext) -> BackendFactoryFuture<'_>;
}

/// `startup_dir` is captured once by the runner, never changed per lease. The
/// service refuses a different initialization base instead of normalizing roots.
pub struct ServiceEnvironment {
    pub startup_dir: PathBuf,
    pub plugin_config: Value,
    pub state_dir: PathBuf,
}

/// A response observer. Dropping it does not cancel admitted backend work.
pub struct PendingServiceReply {
    pub(crate) future: Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>,
}

impl PendingServiceReply {
    pub async fn wait(self) -> Result<Value, String> {
        self.future.await
    }

    pub(crate) fn ready(value: Result<Value, String>) -> Self {
        Self {
            future: Box::pin(async move { value }),
        }
    }
}

/// Public connection owner. Tasks retain `Inner`, never this owner, so dropping
/// the connection always fences admission even when response observers survive.
pub struct SearchService {
    pub(crate) inner: Arc<Inner>,
}

impl SearchService {
    /// Must be created inside a Tokio runtime; owned cleanup uses that runtime.
    pub fn new(
        factory: Arc<dyn SearchBackendFactory>,
        environment: ServiceEnvironment,
        ceilings: ServiceLimits,
    ) -> Result<Self, SearchError> {
        ceilings.validate()?;
        if !environment.startup_dir.is_absolute() {
            return Err(invalid("file-search startup directory must be absolute"));
        }
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| invalid("file-search service requires an active runtime"))?;
        let (closed, _) = watch::channel(None);
        Ok(Self {
            inner: Arc::new(Inner {
                factory,
                environment,
                ceilings,
                runtime,
                state: Mutex::new(Registry::default()),
                tasks: TaskTracker::new(),
                closed,
            }),
        })
    }

    /// Fences new work without waiting for accepted operations or startup.
    pub fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }

    /// Repeated observers receive the retained operation and cleanup outcome.
    pub async fn shutdown(&self) -> SearchCloseOutcome {
        self.request_shutdown();
        observe_close(self.inner.closed.subscribe()).await
    }
}

impl Drop for SearchService {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

pub(crate) struct Inner {
    pub(crate) factory: Arc<dyn SearchBackendFactory>,
    pub(crate) environment: ServiceEnvironment,
    pub(crate) ceilings: ServiceLimits,
    pub(crate) runtime: tokio::runtime::Handle,
    pub(crate) state: Mutex<Registry>,
    pub(crate) tasks: TaskTracker,
    pub(crate) closed: watch::Sender<Option<SearchCloseOutcome>>,
}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) identity: Option<ProviderIdentity>,
    pub(crate) limits: Option<ServiceLimits>,
    pub(crate) backend: Option<Arc<dyn SearchBackend>>,
    pub(crate) initialization_cleanup: Option<CloseCleanup>,
    pub(crate) initialized: bool,
    pub(crate) closing: bool,
    pub(crate) shutdown_started: bool,
    pub(crate) last_epoch: u64,
    pub(crate) leases: HashMap<u64, Arc<Lease>>,
    pub(crate) retired: VecDeque<(LeaseIdentity, SearchCloseOutcome)>,
    pub(crate) used: [u64; 3],
    pub(crate) first_failure: Option<SearchError>,
    pub(crate) omitted_failures: u64,
}

pub(crate) const RETAINED_RECEIPTS: usize = 64;

impl Registry {
    pub(crate) fn remember_failure(&mut self, error: SearchError) {
        if self.first_failure.is_none() {
            self.first_failure = Some(error);
        } else {
            self.omitted_failures = self.omitted_failures.saturating_add(1);
        }
    }

    pub(crate) fn remember_receipt(
        &mut self,
        identity: LeaseIdentity,
        outcome: SearchCloseOutcome,
    ) {
        if self.retired.len() == RETAINED_RECEIPTS {
            self.retired.pop_front();
        }
        self.retired.push_back((identity, outcome));
    }
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn closed_error() -> SearchError {
    SearchError::new(
        SearchErrorKind::ClosedLease,
        "file-search admission is closed",
    )
}

pub(crate) fn panic_error() -> SearchError {
    SearchError::new(
        SearchErrorKind::SearchFailed,
        "file-search backend operation panicked",
    )
}

pub(crate) fn uncertain(error: SearchError) -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Err(error.clone()),
        cleanup: CloseCleanup::Unconfirmed(error),
    }
}

pub(crate) async fn observe_close(
    mut receiver: watch::Receiver<Option<SearchCloseOutcome>>,
) -> SearchCloseOutcome {
    loop {
        if let Some(outcome) = receiver.borrow_and_update().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            return uncertain(SearchError::new(
                SearchErrorKind::TransportLost,
                "file-search cleanup owner disappeared",
            ));
        }
    }
}
