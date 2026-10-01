use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::SearchStartControl;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use futures::FutureExt;
use tokio::sync::Notify;
use tokio::sync::watch;

use crate::LeaseIdentity;
use crate::LeaseLimits;
use crate::WireBudget;
use crate::service::Inner;
use crate::service::lock;
use crate::service::observe_close;
use crate::service::panic_error;
use crate::service::uncertain;

pub(crate) struct Lease {
    pub(crate) identity: LeaseIdentity,
    pub(crate) roots: Vec<std::path::PathBuf>,
    pub(crate) options: FileSearchOptions,
    pub(crate) limits: LeaseLimits,
    pub(crate) budget: WireBudget,
    pub(crate) owner: Weak<Inner>,
    pub(crate) state: Mutex<LeaseState>,
    pub(crate) changed: Notify,
    pub(crate) closed: watch::Sender<Option<SearchCloseOutcome>>,
}

#[derive(Default)]
pub(crate) struct LeaseState {
    pub(crate) session: Option<Arc<dyn SearchBackendSession>>,
    pub(crate) pending_start: Option<Arc<dyn SearchStartControl>>,
    pub(crate) start_finished: bool,
    pub(crate) start_failure: Option<SearchStartError>,
    pub(crate) start_origin: StartOrigin,
    pub(crate) startup_closed_before_result: bool,
    pub(crate) expected_start_cancellation: bool,
    pub(crate) closing: bool,
    pub(crate) close_started: bool,
    pub(crate) polling: bool,
    pub(crate) updating: bool,
    pub(crate) query_id: u64,
    pub(crate) query: String,
    pub(crate) first_failure: Option<SearchError>,
    pub(crate) completed: bool,
}

#[derive(Clone, Copy, Default)]
pub(crate) enum StartOrigin {
    #[default]
    Backend,
    CancelledBeforeAdmission,
}

impl Lease {
    pub(crate) fn request_close(self: &Arc<Self>) {
        let Some(owner) = self.owner.upgrade() else {
            self.finish(uncertain(owner_lost()));
            return;
        };
        let token = owner.tasks.token();
        let (session, pending, start) = {
            let mut state = lock(&self.state);
            state.closing = true;
            let start = !state.close_started;
            state.close_started = true;
            (state.session.clone(), state.pending_start.clone(), start)
        };
        self.retain_known_start_failure();
        if let Some(pending) = pending {
            self.request_start_cancel(&pending);
        }
        if let Some(session) = session
            && std::panic::catch_unwind(AssertUnwindSafe(|| session.request_close())).is_err()
        {
            self.retain_failure(panic_error());
        }
        self.changed.notify_waiters();
        if !start {
            return;
        }
        let lease = Arc::clone(self);
        let guard = CloseCompletionGuard {
            lease: Some(Arc::clone(&lease)),
            _token: token,
        };
        owner.runtime.spawn(async move {
            let mut guard = guard;
            let outcome = lease.join().await;
            let unconfirmed = matches!(outcome.cleanup, CloseCleanup::Unconfirmed(_));
            lease.finish(outcome);
            guard.lease.take();
            if unconfirmed && let Some(owner) = lease.owner.upgrade() {
                owner.request_shutdown();
            }
        });
    }

    pub(crate) fn request_start_cancel(&self, control: &Arc<dyn SearchStartControl>) {
        if std::panic::catch_unwind(AssertUnwindSafe(|| control.request_cancel())).is_err() {
            self.retain_failure(panic_error());
        }
    }

    async fn join(&self) -> SearchCloseOutcome {
        let mut cancellation: Option<SearchStartCancellationOutcome> = None;
        // The notification is registered before checking predicates, avoiding
        // the finish-between-check-and-wait race for startup and operations.
        let session = loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let (pending, ready) = {
                let state = lock(&self.state);
                let ready = if state.start_finished && !state.polling && !state.updating {
                    Some((
                        state.session.clone(),
                        state.start_failure.clone(),
                        state.start_origin,
                    ))
                } else {
                    None
                };
                (state.pending_start.clone(), ready)
            };
            if cancellation.is_none()
                && let Some(pending) = pending
            {
                self.retain_known_start_failure();
                // This independent retained drain route must run while startup
                // is still pending, including when request_cancel panicked.
                let outcome = match AssertUnwindSafe(async { pending.cancel_and_wait().await })
                    .catch_unwind()
                    .await
                {
                    Ok(outcome) => outcome,
                    Err(_) => SearchStartCancellationOutcome {
                        operation: Err(panic_error()),
                        cleanup: StartCleanup::Unconfirmed(panic_error()),
                    },
                };
                let known_failure = {
                    let state = lock(&self.state);
                    state.start_failure.as_ref().and_then(|failure| {
                        let expected = failure.operation.kind() == SearchErrorKind::ClosedLease
                            && matches!(
                                &failure.cleanup,
                                StartCleanup::NotAdmitted | StartCleanup::Confirmed
                            )
                            && outcome.operation.is_ok()
                            && matches!(
                                &outcome.cleanup,
                                StartCleanup::NotAdmitted | StartCleanup::Confirmed
                            );
                        (!expected).then(|| failure.operation.clone())
                    })
                };
                if let Some(error) = known_failure {
                    self.retain_failure(error);
                }
                if let Err(error) = &outcome.operation {
                    self.retain_failure(error.clone());
                }
                if let StartCleanup::Unconfirmed(error) = outcome.cleanup {
                    return SearchCloseOutcome {
                        operation: outcome.operation,
                        cleanup: CloseCleanup::Unconfirmed(error),
                    };
                }
                cancellation = Some(outcome);
                continue;
            }
            if let Some((session, failure, origin)) = ready {
                if let Some(failure) = failure {
                    // A closing flag alone cannot erase a genuine backend
                    // ClosedLease. Only local non-admission or the backend's
                    // retained successful cancellation receipt establishes it.
                    let expected_close = (matches!(origin, StartOrigin::CancelledBeforeAdmission)
                        || cancellation
                            .as_ref()
                            .is_some_and(|outcome| outcome.operation.is_ok()))
                        && failure.operation.kind() == SearchErrorKind::ClosedLease
                        && matches!(
                            &failure.cleanup,
                            StartCleanup::NotAdmitted | StartCleanup::Confirmed
                        );
                    lock(&self.state).expected_start_cancellation = expected_close;
                    return SearchCloseOutcome {
                        operation: if expected_close {
                            Ok(())
                        } else {
                            Err(failure.operation)
                        },
                        cleanup: match failure.cleanup {
                            StartCleanup::NotAdmitted | StartCleanup::Confirmed => {
                                CloseCleanup::Joined
                            }
                            StartCleanup::Unconfirmed(error) => CloseCleanup::Unconfirmed(error),
                        },
                    };
                }
                break session;
            }
            notified.await;
        };
        let Some(session) = session else {
            return uncertain(panic_error());
        };
        match AssertUnwindSafe(async { session.close().await })
            .catch_unwind()
            .await
        {
            Ok(outcome) => outcome,
            Err(_) => uncertain(panic_error()),
        }
    }

    pub(crate) async fn outcome(&self) -> SearchCloseOutcome {
        observe_close(self.closed.subscribe()).await
    }

    pub(crate) fn allocation(&self) -> [u64; 3] {
        [
            self.budget.max_index_entries.0,
            self.budget.max_index_bytes.0,
            self.budget.max_worker_threads.0,
        ]
    }

    pub(crate) fn retain_failure(&self, error: SearchError) {
        let completed = {
            let mut state = lock(&self.state);
            state.first_failure.get_or_insert_with(|| error.clone());
            state.completed
        };
        if completed && let Some(owner) = self.owner.upgrade() {
            // A late genuine failure remains in the owner ledger without
            // rewriting the immutable uncertainty already returned to clients.
            lock(&owner.state).remember_failure(error);
        }
    }

    fn retain_known_start_failure(&self) {
        let failure = {
            let state = lock(&self.state);
            state.start_failure.as_ref().and_then(|failure| {
                let known_real = failure.operation.kind() != SearchErrorKind::ClosedLease
                    || (!state.startup_closed_before_result
                        && matches!(state.start_origin, StartOrigin::Backend));
                known_real.then(|| failure.operation.clone())
            })
        };
        if let Some(error) = failure {
            self.retain_failure(error);
        }
    }

    pub(crate) fn finish(&self, mut outcome: SearchCloseOutcome) {
        self.retain_known_start_failure();
        {
            let mut state = lock(&self.state);
            if state.completed {
                return;
            }
            state.completed = true;
            state.closing = true;
            if let Some(error) = &state.first_failure {
                outcome.operation = Err(error.clone());
            }
        }
        if let Some(owner) = self.owner.upgrade() {
            let mut state = lock(&owner.state);
            if let Err(error) = &outcome.operation {
                state.remember_failure(error.clone());
            }
            if outcome.cleanup == CloseCleanup::Joined {
                if state
                    .leases
                    .remove(&self.identity.session_epoch.0)
                    .is_some()
                {
                    for (used, released) in state.used.iter_mut().zip(self.allocation()) {
                        *used = used.saturating_sub(released);
                    }
                }
                state.remember_receipt(self.identity.clone(), outcome.clone());
            } else {
                // A guard can execute during runtime teardown: fence admission
                // synchronously, without depending on another spawned task.
                state.closing = true;
            }
        }
        self.closed.send_replace(Some(outcome));
    }
}

pub(crate) fn owner_lost() -> SearchError {
    SearchError::new(
        SearchErrorKind::TransportLost,
        "file-search service owner disappeared",
    )
}

struct CloseCompletionGuard {
    lease: Option<Arc<Lease>>,
    _token: tokio_util::task::task_tracker::TaskTrackerToken,
}

impl Drop for CloseCompletionGuard {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            lease.finish(uncertain(owner_lost()));
        }
    }
}

pub(crate) enum Operation {
    Poll,
    Update,
}

/// Accepted slot lifetime belongs to the operation owner, not its observer.
/// Drop also clears flags during panic unwinding before close joins the owner.
pub(crate) struct OperationGuard {
    pub(crate) lease: Arc<Lease>,
    pub(crate) operation: Operation,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        let mut state = lock(&self.lease.state);
        match self.operation {
            Operation::Poll => state.polling = false,
            Operation::Update => state.updating = false,
        }
        drop(state);
        self.lease.changed.notify_waiters();
    }
}
