//! Retained ownership is independent of public backend/session handles.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::Weak;

use codex_component_host::ComponentSession;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use tokio::sync::Notify;
use tokio::sync::watch;

use crate::LeaseIdentity;
use crate::LeaseLimits;
use crate::ProviderIdentity;
use crate::ServiceLimits;

pub(super) struct Provider {
    pub transport: ComponentSession,
    pub identity: ProviderIdentity,
    pub limits: ServiceLimits,
    pub admission: tokio::sync::Mutex<()>,
    pub state: Mutex<ProviderState>,
    pub stopping: watch::Sender<bool>,
    pub completed: watch::Sender<Option<SearchCloseOutcome>>,
}

#[derive(Default)]
pub(super) struct ProviderState {
    pub epoch: u64,
    pub closing: bool,
    pub leases: Vec<Arc<Lease>>,
    pub first_error: Option<SearchError>,
}

pub(super) struct Lease {
    pub provider: Weak<Provider>,
    pub identity: OnceLock<LeaseIdentity>,
    pub request: SearchOpen,
    pub limits: OnceLock<LeaseLimits>,
    pub state: Mutex<LeaseState>,
    pub stopping: watch::Sender<bool>,
    pub completed: watch::Sender<Option<SearchCloseOutcome>>,
    pub changed: Notify,
}

#[derive(Default)]
pub(super) struct LeaseState {
    pub closing: bool,
    pub query_id: u64,
    pub query: String,
    pub revision: u64,
    pub update_active: bool,
    pub poll_active: bool,
    pub first_error: Option<SearchError>,
}

pub(super) fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn closed() -> SearchError {
    SearchError::new(SearchErrorKind::ClosedLease, "file-search owner is closing")
}

pub(super) fn lost() -> SearchError {
    SearchError::new(
        SearchErrorKind::TransportLost,
        "file-search process cleanup is unconfirmed",
    )
}

pub(super) fn uncertain(error: SearchError) -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Err(error.clone()),
        cleanup: CloseCleanup::Unconfirmed(error),
    }
}

pub(super) async fn observe(
    mut receiver: watch::Receiver<Option<SearchCloseOutcome>>,
) -> SearchCloseOutcome {
    loop {
        if let Some(outcome) = receiver.borrow().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            return uncertain(lost());
        }
    }
}

impl Provider {
    pub fn request_shutdown(&self) {
        let mut state = lock(&self.state);
        state.closing = true;
        for lease in &state.leases {
            lease.request_close();
        }
        self.stopping.send_replace(true);
    }

    pub fn fail(&self, error: SearchError) {
        lock(&self.state).first_error.get_or_insert(error);
        self.request_shutdown();
    }

    pub fn reserve(self: &Arc<Self>, request: SearchOpen) -> Result<Arc<Lease>, SearchError> {
        let mut state = lock(&self.state);
        if state.closing {
            return Err(closed());
        }
        let ceiling = self.limits.resources.into_native()?;
        let mut sums = [0_usize; 3];
        for budget in state
            .leases
            .iter()
            .map(|lease| lease.request.budget)
            .chain([request.budget])
        {
            for (sum, value) in sums.iter_mut().zip(budget_values(budget)) {
                *sum = sum.checked_add(value).ok_or_else(exhausted)?;
            }
        }
        if state.leases.len() >= self.limits.max_leases as usize
            || sums
                .into_iter()
                .zip(budget_values(ceiling))
                .any(|(sum, cap)| sum > cap)
        {
            return Err(exhausted());
        }
        let (stopping, _) = watch::channel(false);
        let (completed, _) = watch::channel(None);
        let lease = Arc::new(Lease {
            provider: Arc::downgrade(self),
            identity: OnceLock::new(),
            request,
            limits: OnceLock::new(),
            state: Mutex::new(LeaseState::default()),
            stopping,
            completed,
            changed: Notify::new(),
        });
        state.leases.push(Arc::clone(&lease));
        Ok(lease)
    }
}

fn budget_values(budget: SearchBudget) -> [usize; 3] {
    [
        budget.max_index_entries.get(),
        budget.max_index_bytes.get(),
        budget.max_worker_threads.get(),
    ]
}

fn exhausted() -> SearchError {
    SearchError::new(
        SearchErrorKind::ResourceExhausted,
        "file-search provider allocation is exhausted",
    )
}

impl Lease {
    pub fn request_close(&self) {
        lock(&self.state).closing = true;
        self.stopping.send_replace(true);
        self.changed.notify_waiters();
    }

    pub fn fail(&self, error: SearchError) {
        lock(&self.state).first_error.get_or_insert(error);
        self.request_close();
    }

    pub async fn drain_operations(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let state = lock(&self.state);
                if !state.poll_active && !state.update_active {
                    return;
                }
            }
            changed.await;
        }
    }

    pub fn finish(self: &Arc<Self>, mut outcome: SearchCloseOutcome) {
        let first = lock(&self.state).first_error.clone();
        if let Some(error) = first {
            outcome.operation = Err(error);
        }
        if let Some(provider) = self.provider.upgrade() {
            let mut state = lock(&provider.state);
            if let Err(error) = &outcome.operation {
                state.first_error.get_or_insert_with(|| error.clone());
            }
            if outcome.cleanup == CloseCleanup::Joined {
                state.leases.retain(|lease| !Arc::ptr_eq(lease, self));
            }
        }
        self.completed.send_replace(Some(outcome));
    }
}

/// A public observer disappearing requests cleanup; it does not abort its owner.
pub(super) struct CancelOnDrop(pub Option<watch::Sender<bool>>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(cancel) = self.0.take() {
            cancel.send_replace(true);
        }
    }
}

/// Panic or runtime task cancellation cannot accidentally refund a live lease.
pub(super) struct LeaseOwnerGuard(pub Option<Arc<Lease>>);

impl Drop for LeaseOwnerGuard {
    fn drop(&mut self) {
        if let Some(lease) = self.0.take() {
            let error = lost();
            lease.fail(error.clone());
            lease.finish(uncertain(error.clone()));
            if let Some(provider) = lease.provider.upgrade() {
                provider.fail(error);
            }
        }
    }
}

pub(super) enum OperationKind {
    Update,
    Poll,
}

pub(super) struct OperationGuard {
    pub lease: Arc<Lease>,
    pub kind: OperationKind,
    pub finished: bool,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        {
            let mut state = lock(&self.lease.state);
            match self.kind {
                OperationKind::Update => state.update_active = false,
                OperationKind::Poll => state.poll_active = false,
            }
        }
        if !self.finished {
            self.lease.fail(lost());
        }
        self.lease.changed.notify_waiters();
    }
}
