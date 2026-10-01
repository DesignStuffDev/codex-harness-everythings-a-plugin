use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;

use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::SessionReporter;
use codex_file_search_api::StartCleanup;
use futures::FutureExt;
use tokio::sync::Notify;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::FileSearchSession;
use crate::policy::exhausted;
use crate::session::close_cause;
use crate::state::*;

pub(crate) async fn open(
    scope: Arc<Scope>,
    request: SearchOpen,
    reporter: Arc<dyn SessionReporter>,
) -> Result<FileSearchSession, SearchStartError> {
    let provider = scope.provider.upgrade().ok_or_else(|| rejected(closed()))?;
    let request = provider.policy.compact_open(request).map_err(rejected)?;
    let token = provider.tasks.token();
    let lease = {
        let mut state = lock(&provider.state);
        if state.closing {
            return Err(rejected(closed()));
        }
        let scope_state = state
            .scopes
            .get(&scope.id)
            .ok_or_else(|| rejected(closed()))?;
        if scope_state.closing {
            return Err(rejected(closed()));
        }
        if state.sessions >= provider.policy.provider.max_sessions.get()
            || scope_state.leases.len() >= scope.limits.max_sessions.get()
        {
            return Err(rejected(exhausted(
                "file-search provider or scope session capacity reached",
            )));
        }
        let mut used = [0; 3];
        for (index, (amount, cap)) in allocation(request.budget)
            .into_iter()
            .zip(allocation(provider.policy.provider.resources))
            .enumerate()
        {
            used[index] = state.used[index]
                .checked_add(amount)
                .filter(|sum| *sum <= cap)
                .ok_or_else(|| {
                    rejected(exhausted("file-search aggregate resource capacity reached"))
                })?;
        }
        let id = state
            .next_lease
            .checked_add(1)
            .ok_or_else(|| rejected(exhausted("file-search session identities exhausted")))?;
        let lease = Arc::new(Lease {
            id,
            scope: scope.id,
            provider: Arc::downgrade(&provider),
            budget: request.budget,
            roots: request.roots.clone(),
            match_limit: request.options.limit.get(),
            reporter,
            operations: tokio_util::task::TaskTracker::new(),
            state: Mutex::new(LeaseState::default()),
            changed: Notify::new(),
            completed: watch::channel(None).0,
        });
        state.next_lease = id;
        state.used = used;
        state.sessions += 1;
        // Same admission mutex protects provider/scope fence and retained task.
        // The token already exists before this Preparing entry becomes visible.
        if let Some(scope) = state.scopes.get_mut(&scope.id) {
            scope.leases.insert(id, Arc::clone(&lease));
        }
        lease
    };
    let mut abandon = AbandonedStart(Some(Arc::clone(&lease)));
    let (sender, receiver) = oneshot::channel();
    let owner = Arc::clone(&lease);
    let runtime = provider.runtime.clone();
    let guard = StartCompletionGuard {
        lease: Some(Arc::clone(&owner)),
        _token: token,
    };
    runtime.spawn(async move {
        let mut guard = guard;
        let started = match AssertUnwindSafe(async { provider.backend.open(request).await })
            .catch_unwind()
            .await
        {
            Ok(result) => result,
            Err(_) => {
                let error = panicked();
                Err(SearchStartError {
                    operation: error.clone(),
                    cleanup: StartCleanup::Unconfirmed(error),
                })
            }
        };
        let (backend, failure, closing, pump_local_token) = {
            let mut state = lock(&owner.state);
            state.start_finished = true;
            match started {
                Ok(backend) => {
                    state.backend = Some(Arc::clone(&backend));
                    (
                        Some(backend),
                        None,
                        state.closing,
                        Some(owner.operations.token()),
                    )
                }
                Err(error) => {
                    state.closing_before_start_result = state.closing;
                    state.start_error = Some(error.clone());
                    (None, Some(error), true, None)
                }
            }
        };
        if let (Some(backend), Some(local_token)) = (backend, pump_local_token) {
            let pump_token = provider.tasks.token();
            let pump = Arc::clone(&owner);
            let provider = Arc::clone(&provider);
            let runtime = provider.runtime.clone();
            let pump_guard = crate::session::OperationGuard {
                lease: Arc::clone(&pump),
                kind: crate::session::Operation::Pump,
                finished: false,
                admission_released: false,
                _token: pump_token,
                _local_token: local_token,
            };
            runtime.spawn(async move {
                crate::observation::run(pump, backend, provider.policy, pump_guard).await;
            });
        }
        owner.changed.notify_waiters();
        guard.lease.take();
        let result = if closing {
            owner.request_close();
            let outcome = observe(owner.completed.subscribe()).await;
            let cleanup = if failure
                .as_ref()
                .is_some_and(|error| error.cleanup == StartCleanup::NotAdmitted)
                && outcome.cleanup == codex_file_search_api::CloseCleanup::Joined
            {
                StartCleanup::NotAdmitted
            } else {
                outcome.cleanup.clone().into()
            };
            let operation = match failure {
                Some(error)
                    if error.operation.kind()
                        != codex_file_search_api::SearchErrorKind::ClosedLease =>
                {
                    error.operation
                }
                Some(error) if outcome.operation.is_ok() => error.operation,
                _ => close_cause(&outcome),
            };
            Err(SearchStartError { operation, cleanup })
        } else {
            Ok(())
        };
        let _ = sender.send(result);
    });
    let result = receiver.await.unwrap_or_else(|_| {
        Err(SearchStartError {
            operation: lost(),
            cleanup: StartCleanup::Unconfirmed(lost()),
        })
    });
    result?;
    let closing = lock(&lease.state).closing;
    if closing {
        let outcome = observe(lease.completed.subscribe()).await;
        return Err(SearchStartError {
            operation: close_cause(&outcome),
            cleanup: outcome.cleanup.into(),
        });
    }
    // Ownership transfers without an await; a dropped receiver leaves the
    // AbandonedStart guard to request close of the same retained lease.
    abandon.0.take();
    Ok(FileSearchSession::new(lease))
}
fn rejected(operation: codex_file_search_api::SearchError) -> SearchStartError {
    SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    }
}
struct AbandonedStart(Option<Arc<Lease>>);
impl Drop for AbandonedStart {
    fn drop(&mut self) {
        if let Some(lease) = self.0.take() {
            lease.request_close();
        }
    }
}
struct StartCompletionGuard {
    lease: Option<Arc<Lease>>,
    _token: tokio_util::task::task_tracker::TaskTrackerToken,
}
impl Drop for StartCompletionGuard {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            {
                let mut state = lock(&lease.state);
                state.start_finished = true;
                state.start_error = Some(SearchStartError {
                    operation: lost(),
                    cleanup: StartCleanup::Unconfirmed(lost()),
                });
            }
            lease.changed.notify_waiters();
            lease.request_close();
        }
    }
}
