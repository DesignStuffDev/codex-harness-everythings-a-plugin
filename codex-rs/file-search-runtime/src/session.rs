use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::QueryAccepted;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchQuery;
use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::StartCleanup;
use futures::FutureExt;
use tokio::sync::oneshot;

use crate::policy::copy_query;
use crate::policy::exhausted;
use crate::policy::invalid;
use crate::state::*;

/// Public cloneable lease; background owners retain only the internal Lease.
#[derive(Clone)]
pub struct FileSearchSession {
    handle: Arc<SessionHandle>,
}
struct SessionHandle(Arc<Lease>);
impl Drop for SessionHandle {
    fn drop(&mut self) {
        self.0.request_close();
    }
}
impl FileSearchSession {
    pub(crate) fn new(inner: Arc<Lease>) -> Self {
        Self {
            handle: Arc::new(SessionHandle(inner)),
        }
    }
    pub fn request_close(&self) {
        self.handle.0.request_close();
    }
    /// Requests closure immediately. Dropping this observer does not stop close.
    pub fn close(&self) -> SearchCloseFuture<'static> {
        self.request_close();
        Box::pin(observe(self.handle.0.completed.subscribe()))
    }
    /// Observes the retained terminal receipt without requesting closure.
    /// Useful for one-shot consumers when a backend cancels or closes before a
    /// current-query Idle callback. Dropping this observer changes no ownership;
    /// only the receipt, including any explicit uncertainty, describes cleanup.
    pub fn wait_closed(&self) -> SearchCloseFuture<'static> {
        Box::pin(observe(self.handle.0.completed.subscribe()))
    }
    /// Acknowledges selected-backend admission, not a queued transport write.
    /// Only one update is admitted at a time; callers coalesce unsent UI changes.
    pub async fn update_query(&self, query: SearchQuery) -> Result<QueryAccepted, SearchError> {
        let lease = Arc::clone(&self.handle.0);
        let provider = lease.provider.upgrade().ok_or_else(closed)?;
        let token = provider.tasks.token();
        let local_token = lease.operations.token();
        let (backend, query) = {
            let provider_state = lock(&provider.state);
            if provider_state.closing
                || provider_state
                    .scopes
                    .get(&lease.scope)
                    .is_none_or(|scope| scope.closing)
            {
                return Err(closed());
            }
            let mut state = lock(&lease.state);
            if state.closing {
                return Err(closed());
            }
            if state.update_active {
                return Err(exhausted("file-search update already retained"));
            }
            query.validate_after(state.query_id, provider.policy.max_query_bytes)?;
            let compact = copy_query(&query.text)?;
            let expected = copy_query(&query.text)?;
            let backend = state.backend.clone().ok_or_else(closed)?;
            state.query_id = query.id.get();
            state.query = expected;
            state.update_active = true;
            (
                backend,
                SearchQuery {
                    id: query.id,
                    text: compact,
                },
            )
        };
        let (sender, receiver) = oneshot::channel();
        let closing_receipt = lease.completed.subscribe();
        let guard = OperationGuard {
            lease: Arc::clone(&lease),
            kind: Operation::Update,
            finished: false,
            admission_released: false,
            _token: token,
            _local_token: local_token,
        };
        provider.runtime.spawn(async move {
            let mut guard = guard;
            let id = query.id;
            let result = match AssertUnwindSafe(async { backend.update_query(query).await })
                .catch_unwind()
                .await
            {
                Ok(Ok(receipt)) if receipt.id == id => Ok(receipt),
                Ok(Ok(_)) => Err(invalid(
                    "file-search backend acknowledged a different query",
                )),
                Ok(Err(error)) => Err(error),
                Err(_) => Err(panicked()),
            };
            if let Err(error) = &result
                && !(error.kind() == SearchErrorKind::ClosedLease && lock(&lease.state).closing)
            {
                lease.fail(error.clone());
            }
            guard.finished = true;
            {
                let mut state = lock(&lease.state);
                state.update_active = false;
                guard.admission_released = true;
                let _ = sender.send(result);
            }
            drop(guard);
        });
        tokio::select! {
            result = receiver => result.unwrap_or_else(|_| Err(lost())),
            outcome = observe(closing_receipt) => Err(close_cause(&outcome)),
        }
    }
}

/// Preserve a retained failure when closure wins an operation's observer race.
pub(crate) fn close_cause(outcome: &SearchCloseOutcome) -> SearchError {
    match &outcome.operation {
        Err(error) => error.clone(),
        Ok(()) => match &outcome.cleanup {
            CloseCleanup::Unconfirmed(error) => error.clone(),
            CloseCleanup::Joined => closed(),
        },
    }
}

impl Lease {
    pub(crate) fn record_failure(&self, error: SearchError) {
        let error = lock(&self.state).first_error.get_or_insert(error).clone();
        if let Some(provider) = self.provider.upgrade() {
            let mut state = lock(&provider.state);
            state.first_error.get_or_insert_with(|| error.clone());
            if let Some(scope) = state.scopes.get_mut(&self.scope) {
                scope.first_error.get_or_insert(error);
            }
        }
    }
    pub(crate) fn fail(self: &Arc<Self>, error: SearchError) {
        self.record_failure(error);
        self.request_close();
    }
    pub(crate) fn request_close(self: &Arc<Self>) {
        let Some(provider) = self.provider.upgrade() else {
            let operation = self.retained_failure().unwrap_or_else(lost);
            self.publish_completion(SearchStartCancellationOutcome {
                operation: Err(operation),
                cleanup: StartCleanup::Unconfirmed(lost()),
            });
            return;
        };
        let token = provider.tasks.token();
        let (backend, pending, start) = {
            let mut state = lock(&self.state);
            state.closing = true;
            let start = !state.close_started;
            state.close_started = true;
            (state.backend.clone(), state.pending_start.clone(), start)
        };
        if let Some(pending) = pending
            && std::panic::catch_unwind(AssertUnwindSafe(|| pending.request_cancel())).is_err()
        {
            self.record_failure(panicked());
        }
        if let Some(backend) = backend
            && std::panic::catch_unwind(AssertUnwindSafe(|| backend.request_close())).is_err()
        {
            self.record_failure(panicked());
        }
        self.operations.close();
        self.changed.notify_waiters();
        if !start {
            return;
        }
        let lease = Arc::clone(self);
        let runtime = provider.runtime.clone();
        let guard = CloseCompletionGuard {
            lease: Some(Arc::clone(&lease)),
            _token: token,
        };
        runtime.spawn(async move {
            let mut guard = guard;
            let mut receipt = lease.join_backend().await;
            {
                let mut state = lock(&lease.state);
                state.backend_cancellation = Some(receipt.clone());
            }
            let initial_uncertainty = matches!(receipt.cleanup, StartCleanup::Unconfirmed(_));
            if !initial_uncertainty {
                // Includes the accepted startup result owner, update and pump.
                // Startup itself never waits for this close actor.
                lease.drain_local().await;
                receipt = lease.reconcile_start(receipt).await;
            }
            if let Some(error) = lease.retained_operation(&receipt) {
                receipt.operation = Err(error);
            }
            if let Some(error) = &lock(&lease.state).cleanup_uncertain {
                receipt.cleanup = StartCleanup::Unconfirmed(error.clone());
            }
            let unconfirmed = matches!(receipt.cleanup, StartCleanup::Unconfirmed(_));
            {
                let mut state = lock(&provider.state);
                if let Err(error) = &receipt.operation {
                    state.first_error.get_or_insert_with(|| error.clone());
                }
                let mut refund = false;
                if let Some(scope) = state.scopes.get_mut(&lease.scope) {
                    if let Err(error) = &receipt.operation {
                        scope.first_error.get_or_insert_with(|| error.clone());
                    }
                    if !unconfirmed {
                        refund = scope.leases.remove(&lease.id).is_some();
                    }
                }
                if refund {
                    state.sessions -= 1;
                    for (used, released) in state.used.iter_mut().zip(allocation(lease.budget)) {
                        *used -= released;
                    }
                }
            }
            lease.publish_completion(receipt);
            if unconfirmed {
                // Quota and accepted local ownership survive early uncertainty.
                provider.request_shutdown();
                lease.drain_local().await;
                if let Some(error) = lease.retained_failure() {
                    // Late real failures still enter ledgers, without rewriting
                    // the immutable uncertain receipt or refunding this lease.
                    lease.record_failure(error);
                }
            } else {
                let (pending, backend) = {
                    let mut state = lock(&lease.state);
                    (state.pending_start.take(), state.backend.take())
                };
                drop(pending);
                drop(backend);
            }
            guard.lease.take();
        });
    }
    async fn drain_local(&self) {
        self.operations.wait().await;
    }
}

pub(crate) enum Operation {
    Update,
    Pump,
}
pub(crate) struct OperationGuard {
    pub(crate) lease: Arc<Lease>,
    pub(crate) kind: Operation,
    pub(crate) finished: bool,
    pub(crate) admission_released: bool,
    pub(crate) _token: tokio_util::task::task_tracker::TaskTrackerToken,
    pub(crate) _local_token: tokio_util::task::task_tracker::TaskTrackerToken,
}
impl Drop for OperationGuard {
    fn drop(&mut self) {
        {
            let mut state = lock(&self.lease.state);
            match self.kind {
                Operation::Update if !self.admission_released => state.update_active = false,
                Operation::Update => {}
                Operation::Pump => {}
            }
            if !self.finished {
                state.first_error.get_or_insert_with(lost);
            }
        }
        self.lease.changed.notify_waiters();
        if !self.finished {
            self.lease.fail(lost());
        }
    }
}
struct CloseCompletionGuard {
    lease: Option<Arc<Lease>>,
    _token: tokio_util::task::task_tracker::TaskTrackerToken,
}
impl Drop for CloseCompletionGuard {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let operation = lease.retained_failure().unwrap_or_else(lost);
            lock(&lease.state)
                .cleanup_uncertain
                .get_or_insert_with(lost);
            lease.record_failure(operation.clone());
            lease.publish_completion(SearchStartCancellationOutcome {
                operation: Err(operation),
                cleanup: StartCleanup::Unconfirmed(lost()),
            });
        }
    }
}
