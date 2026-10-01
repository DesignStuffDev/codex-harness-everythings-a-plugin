use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
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
    pub(crate) start_finished: bool,
    pub(crate) start_failure: Option<SearchStartError>,
    pub(crate) closing: bool,
    pub(crate) close_started: bool,
    pub(crate) polling: bool,
    pub(crate) updating: bool,
    pub(crate) query_id: u64,
    pub(crate) query: String,
    pub(crate) first_failure: Option<SearchError>,
}

impl Lease {
    pub(crate) fn request_close(self: &Arc<Self>) {
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        let token = owner.tasks.token();
        let (session, start) = {
            let mut state = lock(&self.state);
            state.closing = true;
            let start = !state.close_started;
            state.close_started = true;
            (state.session.clone(), start)
        };
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
        owner.runtime.spawn(async move {
            let _token = token;
            let mut outcome = lease.join().await;
            if let Some(error) = lock(&lease.state).first_failure.clone() {
                outcome.operation = Err(error);
            }
            if let Some(owner) = lease.owner.upgrade() {
                {
                    let mut state = lock(&owner.state);
                    if let Err(error) = &outcome.operation {
                        state.remember_failure(error.clone());
                    }
                    if outcome.cleanup == CloseCleanup::Joined {
                        state.leases.remove(&lease.identity.session_epoch.0);
                        for (used, released) in state.used.iter_mut().zip(lease.allocation()) {
                            *used = used.saturating_sub(released);
                        }
                        state.remember_receipt(lease.identity.clone(), outcome.clone());
                    }
                }
                lease.closed.send_replace(Some(outcome.clone()));
                if matches!(outcome.cleanup, CloseCleanup::Unconfirmed(_)) {
                    owner.request_shutdown();
                }
            } else {
                lease.closed.send_replace(Some(outcome));
            }
        });
    }

    async fn join(&self) -> SearchCloseOutcome {
        // The notification is registered before checking predicates, avoiding
        // the finish-between-check-and-wait race for startup and operations.
        let session = loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let ready = {
                let state = lock(&self.state);
                if state.start_finished && !state.polling && !state.updating {
                    Some((state.session.clone(), state.start_failure.clone()))
                } else {
                    None
                }
            };
            if let Some((session, failure)) = ready {
                if let Some(failure) = failure {
                    return SearchCloseOutcome {
                        operation: Err(failure.operation),
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
        let mut state = lock(&self.state);
        if state.first_failure.is_none() {
            state.first_failure = Some(error);
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
