use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;

use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::SessionReporter;
use codex_file_search_api::StartCleanup;
use futures::FutureExt;
use tokio::sync::Notify;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::PendingFileSearchStart;
use crate::policy::exhausted;
use crate::state::*;

pub(crate) fn begin_open(
    scope: Arc<Scope>,
    request: SearchOpen,
    reporter: Arc<dyn SessionReporter>,
) -> Result<PendingFileSearchStart, SearchStartError> {
    let provider = scope.provider.upgrade().ok_or_else(|| rejected(closed()))?;
    let request = provider.policy.compact_open(request).map_err(rejected)?;
    let token = provider.tasks.token();
    let (lease, local_token) = {
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
            cancelled: watch::channel(None).0,
        });
        let local_token = lease.operations.token();
        state.next_lease = id;
        state.used = used;
        state.sessions += 1;
        // Same admission mutex protects provider/scope fence and retained task.
        // The token already exists before this Preparing entry becomes visible.
        if let Some(scope) = state.scopes.get_mut(&scope.id) {
            scope.leases.insert(id, Arc::clone(&lease));
        }
        (lease, local_token)
    };
    let (sender, receiver) = oneshot::channel();
    let pending = PendingFileSearchStart::new(Arc::clone(&lease), receiver);
    let runtime = provider.runtime.clone();
    let guard = StartCompletionGuard {
        lease: Some(Arc::clone(&lease)),
        _token: token,
        _local_token: local_token,
    };
    runtime.spawn(async move {
        let mut guard = guard;
        run_start(lease, provider, request, sender).await;
        guard.lease.take();
    });
    Ok(pending)
}

async fn run_start(
    owner: Arc<Lease>,
    provider: Arc<Provider>,
    request: SearchOpen,
    sender: oneshot::Sender<Result<(), SearchStartError>>,
) {
    let skip = {
        let mut state = lock(&owner.state);
        if state.closing {
            state.skipped_before_backend = true;
            true
        } else {
            false
        }
    };
    let started = if skip {
        Err(rejected(closed()))
    } else {
        match std::panic::catch_unwind(AssertUnwindSafe(|| provider.backend.begin_open(request))) {
            Ok(Ok(pending)) => {
                let control = pending.control();
                let closing = {
                    let mut state = lock(&owner.state);
                    state.pending_start = Some(control);
                    state.closing
                };
                owner.changed.notify_waiters();
                if closing {
                    // Forward the latched intent outside the lease mutex.
                    owner.request_close();
                }
                match AssertUnwindSafe(pending.finish()).catch_unwind().await {
                    Ok(result) => result,
                    Err(_) => Err(unconfirmed_start(panicked())),
                }
            }
            Ok(Err(error)) => Err(error),
            Err(_) => Err(unconfirmed_start(panicked())),
        }
    };
    let (backend, failure, closing, pump_local_token) = {
        let mut state = lock(&owner.state);
        state.start_finished = true;
        match started {
            Ok(backend) => {
                state.backend = Some(Arc::clone(&backend));
                let token = (!state.closing).then(|| owner.operations.token());
                (Some(backend), None, state.closing, token)
            }
            Err(error) => {
                // Capture a known spontaneous failure before automatic close
                // invokes any backend hook. A later closing flag cannot erase it.
                if !state.skipped_before_backend
                    && (state.pending_start.is_none()
                        || error.operation.kind() != SearchErrorKind::ClosedLease
                        || !state.closing)
                {
                    state
                        .first_error
                        .get_or_insert_with(|| error.operation.clone());
                }
                if let StartCleanup::Unconfirmed(cleanup) = &error.cleanup {
                    state
                        .cleanup_uncertain
                        .get_or_insert_with(|| cleanup.clone());
                }
                state.start_error = Some(error.clone());
                (None, Some(error), true, None)
            }
        }
    };
    // Propagate the already retained winner, including a spontaneous ClosedLease,
    // before automatic cleanup can invoke a failing external hook.
    if failure.is_some() {
        let known = lock(&owner.state).first_error.clone();
        if let Some(error) = known {
            owner.record_failure(error);
        }
    }
    if let (Some(backend), Some(local_token)) = (backend, pump_local_token) {
        let pump = Arc::clone(&owner);
        let pump_guard = crate::session::OperationGuard {
            lease: Arc::clone(&pump),
            kind: crate::session::Operation::Pump,
            finished: false,
            admission_released: false,
            _token: provider.tasks.token(),
            _local_token: local_token,
        };
        let policy = provider.policy;
        provider.runtime.spawn(async move {
            crate::observation::run(pump, backend, policy, pump_guard).await;
        });
    }
    owner.changed.notify_waiters();
    if closing {
        owner.request_close();
    }
    // The retained startup actor never awaits facade close: close drains its
    // local startup token before confirming cleanup. Only the passive public
    // observer waits for the final cancellation/close receipt.
    if sender.send(failure.map_or(Ok(()), Err)).is_err() {
        owner.request_close();
    }
}
fn rejected(operation: codex_file_search_api::SearchError) -> SearchStartError {
    SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    }
}
fn unconfirmed_start(operation: codex_file_search_api::SearchError) -> SearchStartError {
    SearchStartError {
        cleanup: StartCleanup::Unconfirmed(operation.clone()),
        operation,
    }
}
struct StartCompletionGuard {
    lease: Option<Arc<Lease>>,
    _token: tokio_util::task::task_tracker::TaskTrackerToken,
    _local_token: tokio_util::task::task_tracker::TaskTrackerToken,
}
impl Drop for StartCompletionGuard {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let operation = lease.retained_failure().unwrap_or_else(lost);
            {
                let mut state = lock(&lease.state);
                state.start_finished = true;
                state.cleanup_uncertain.get_or_insert_with(lost);
                state.start_error.get_or_insert_with(|| SearchStartError {
                    operation: operation.clone(),
                    cleanup: StartCleanup::Unconfirmed(lost()),
                });
            }
            lease.record_failure(operation);
            lease.changed.notify_waiters();
            lease.request_close();
        }
    }
}
