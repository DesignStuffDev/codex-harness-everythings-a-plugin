use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::Weak;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::SessionReporter;
use tokio::sync::Notify;
use tokio::sync::watch;
use tokio_util::task::TaskTracker;

use crate::RuntimePolicy;

pub(crate) struct Provider {
    pub(crate) backend: Arc<dyn SearchBackend>,
    pub(crate) policy: RuntimePolicy,
    pub(crate) runtime: tokio::runtime::Handle,
    pub(crate) state: Mutex<ProviderState>,
    pub(crate) tasks: TaskTracker,
    pub(crate) completed: watch::Sender<Option<SearchCloseOutcome>>,
}
#[derive(Default)]
pub(crate) struct ProviderState {
    pub(crate) closing: bool,
    pub(crate) next_scope: u64,
    pub(crate) next_lease: u64,
    pub(crate) scopes: HashMap<u64, ScopeEntry>,
    pub(crate) used: [usize; 3],
    pub(crate) sessions: usize,
    pub(crate) first_error: Option<SearchError>,
}
pub(crate) struct ScopeEntry {
    pub(crate) owner: Arc<Scope>,
    pub(crate) closing: bool,
    pub(crate) leases: HashMap<u64, Arc<Lease>>,
    pub(crate) first_error: Option<SearchError>,
}
pub(crate) struct Scope {
    pub(crate) id: u64,
    pub(crate) limits: ScopeLimits,
    pub(crate) provider: Weak<Provider>,
    pub(crate) completed: watch::Sender<Option<SearchCloseOutcome>>,
}
pub(crate) struct Lease {
    pub(crate) id: u64,
    pub(crate) scope: u64,
    pub(crate) provider: Weak<Provider>,
    pub(crate) budget: SearchBudget,
    pub(crate) roots: Vec<std::path::PathBuf>,
    pub(crate) match_limit: usize,
    pub(crate) reporter: Arc<dyn SessionReporter>,
    pub(crate) operations: TaskTracker,
    pub(crate) state: Mutex<LeaseState>,
    pub(crate) changed: Notify,
    pub(crate) completed: watch::Sender<Option<SearchCloseOutcome>>,
}
#[derive(Default)]
pub(crate) struct LeaseState {
    pub(crate) closing: bool,
    pub(crate) close_started: bool,
    pub(crate) start_finished: bool,
    pub(crate) start_error: Option<SearchStartError>,
    // Captured before an error itself starts closure. Only a prior explicit
    // fence can classify a confirmed ClosedLease startup as cancellation.
    pub(crate) closing_before_start_result: bool,
    pub(crate) backend: Option<Arc<dyn SearchBackendSession>>,
    pub(crate) update_active: bool,
    pub(crate) query_id: u64,
    pub(crate) query: String,
    pub(crate) first_error: Option<SearchError>,
}

pub(crate) fn allocation(budget: SearchBudget) -> [usize; 3] {
    [
        budget.max_index_entries.get(),
        budget.max_index_bytes.get(),
        budget.max_worker_threads.get(),
    ]
}
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
pub(crate) fn closed() -> SearchError {
    SearchError::new(
        SearchErrorKind::ClosedLease,
        "file-search runtime ownership is closing",
    )
}
pub(crate) fn panicked() -> SearchError {
    SearchError::new(
        SearchErrorKind::SearchFailed,
        "file-search backend or callback panicked",
    )
}
pub(crate) fn lost() -> SearchError {
    SearchError::new(
        SearchErrorKind::TransportLost,
        "file-search retained runtime owner disappeared",
    )
}
pub(crate) fn uncertain(error: SearchError) -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Err(error.clone()),
        cleanup: CloseCleanup::Unconfirmed(error),
    }
}
pub(crate) fn complete(
    sender: &watch::Sender<Option<SearchCloseOutcome>>,
    outcome: SearchCloseOutcome,
) {
    sender.send_if_modified(|slot| {
        if slot.is_none() {
            *slot = Some(outcome);
            true
        } else {
            false
        }
    });
}
pub(crate) async fn observe(
    mut receiver: watch::Receiver<Option<SearchCloseOutcome>>,
) -> SearchCloseOutcome {
    loop {
        if let Some(outcome) = receiver.borrow_and_update().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            return uncertain(lost());
        }
    }
}
