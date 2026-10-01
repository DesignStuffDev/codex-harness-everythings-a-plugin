use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use futures::FutureExt;
use tokio::sync::Notify;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::LeaseLimits;
use crate::OPEN_METHOD;
use crate::OpenRequest;
use crate::OpenResponse;
use crate::PendingServiceReply;
use crate::WireU64;
use crate::contract::exhausted;
use crate::contract::invalid;
use crate::contract::reply;
use crate::contract::version;
use crate::service::Inner;
use crate::service::closed_error;
use crate::service::lock;
use crate::service::panic_error;
use crate::service_lease::Lease;
use crate::service_lease::LeaseState;
use crate::service_lease::StartOrigin;
use crate::service_lease::owner_lost;

impl Inner {
    pub(crate) fn open(self: &Arc<Self>, request: OpenRequest) -> PendingServiceReply {
        let identity = request.identity.clone();
        let token = self.tasks.token();
        let admitted = self.reserve(&request);
        let (lease, backend) = match admitted {
            Ok(value) => value,
            Err(error) => {
                return PendingServiceReply::ready(reply(
                    OPEN_METHOD,
                    identity,
                    Err::<OpenResponse, _>(SearchStartError {
                        operation: error,
                        cleanup: StartCleanup::NotAdmitted,
                    }),
                ));
            }
        };
        let (sender, receiver) = oneshot::channel();
        let guard = StartCompletionGuard {
            lease: Some(Arc::clone(&lease)),
            _token: token,
        };
        self.runtime.spawn(async move {
            let mut guard = guard;
            // reserve validated the conversion; even an unexpected conversion
            // error still goes through the retained startup/close owner.
            let cancelled_before_admission = {
                let mut state = lock(&lease.state);
                if state.closing {
                    state.start_origin = StartOrigin::CancelledBeforeAdmission;
                    true
                } else {
                    false
                }
            };
            let started = if cancelled_before_admission {
                Err(SearchStartError {
                    operation: closed_error(),
                    cleanup: StartCleanup::NotAdmitted,
                })
            } else {
                match request.into_native() {
                    Ok(request) => match AssertUnwindSafe(async {
                        // begin_open is synchronous and external; no state lock is
                        // held through admission or any cancellation hook.
                        let pending = backend.begin_open(request)?;
                        let control = pending.control();
                        let closing = {
                            let mut state = lock(&lease.state);
                            state.pending_start = Some(Arc::clone(&control));
                            state.closing
                        };
                        lease.changed.notify_waiters();
                        if closing {
                            lease.request_start_cancel(&control);
                        }
                        pending.finish().await
                    })
                    .catch_unwind()
                    .await
                    {
                        Ok(result) => result,
                        Err(_) => {
                            let error = panic_error();
                            Err(SearchStartError {
                                operation: error.clone(),
                                cleanup: StartCleanup::Unconfirmed(error),
                            })
                        }
                    },
                    Err(error) => Err(SearchStartError {
                        operation: error,
                        cleanup: StartCleanup::NotAdmitted,
                    }),
                }
            };
            let (closing, result, late_error) = {
                let mut state = lock(&lease.state);
                state.start_finished = true;
                match started {
                    Ok(session) => {
                        state.session = Some(session);
                        (
                            state.closing,
                            Ok(OpenResponse {
                                initial_cursor: WireU64(0),
                                limits: lease.limits.clone(),
                                budget: lease.budget,
                            }),
                            None,
                        )
                    }
                    Err(error) => {
                        state.startup_closed_before_result = state.closing;
                        state.start_failure = Some(error.clone());
                        let late_error = state.completed.then(|| error.operation.clone());
                        (true, Err(error), late_error)
                    }
                }
            };
            lease.changed.notify_waiters();
            guard.lease.take();
            if let Some(error) = late_error {
                lease.retain_failure(error);
            }
            let result = if closing {
                lease.request_close();
                let outcome = lease.outcome().await;
                let failure = result.err();
                let cleanup = if failure
                    .as_ref()
                    .is_some_and(|error| error.cleanup == StartCleanup::NotAdmitted)
                    && outcome.cleanup == CloseCleanup::Joined
                {
                    StartCleanup::NotAdmitted
                } else {
                    outcome.cleanup.clone().into()
                };
                let operation = match failure {
                    Some(error) if !lock(&lease.state).expected_start_cancellation => {
                        error.operation
                    }
                    Some(error) => outcome.operation.err().unwrap_or(error.operation),
                    None => outcome.operation.err().unwrap_or_else(closed_error),
                };
                Err(SearchStartError { operation, cleanup })
            } else {
                result
            };
            let _ = sender.send(reply(OPEN_METHOD, identity, result));
        });
        PendingServiceReply {
            future: Box::pin(async move {
                receiver
                    .await
                    .unwrap_or_else(|_| Err("file-search startup owner disappeared".to_owned()))
            }),
        }
    }

    fn reserve(
        self: &Arc<Self>,
        request: &OpenRequest,
    ) -> Result<(Arc<Lease>, Arc<dyn SearchBackend>), SearchError> {
        version(request.contract_version)?;
        request.identity.validate()?;
        let mut state = lock(&self.state);
        if state.identity.as_ref().map(|value| &value.provider_id)
            != Some(&request.identity.provider_id)
        {
            return Err(invalid("file-search provider identity does not match"));
        }
        // Only NEW opens advance this watermark. Existing older leases remain
        // valid; client admission is serialized through host start_with_cleanup.
        if request.identity.session_epoch.0 <= state.last_epoch {
            return Err(SearchError::new(
                SearchErrorKind::StaleEpoch,
                "file-search new-session epoch must strictly increase",
            ));
        }
        state.last_epoch = request.identity.session_epoch.0;
        let result = (|| {
            if state.closing {
                return Err(closed_error());
            }
            let limits = state
                .limits
                .as_ref()
                .ok_or_else(|| invalid("file-search provider has not initialized"))?;
            let backend = state
                .backend
                .clone()
                .ok_or_else(|| invalid("file-search provider is still preparing"))?;
            request.validate(limits)?;
            if state.leases.len() >= limits.max_leases as usize {
                return Err(exhausted("file-search retained lease ceiling reached"));
            }
            let allocation = [
                request.budget.max_index_entries.0,
                request.budget.max_index_bytes.0,
                request.budget.max_worker_threads.0,
            ];
            let ceilings = [
                limits.resources.max_index_entries.0,
                limits.resources.max_index_bytes.0,
                limits.resources.max_worker_threads.0,
            ];
            let mut used = [0; 3];
            for index in 0..3 {
                used[index] = state.used[index]
                    .checked_add(allocation[index])
                    .filter(|value| *value <= ceilings[index])
                    .ok_or_else(|| exhausted("file-search aggregate resource ceiling reached"))?;
            }
            let (closed, _) = watch::channel(None);
            let lease = Arc::new(Lease {
                identity: request.identity.clone(),
                roots: request.roots.clone(),
                options: request.options.clone().into_native()?,
                limits: LeaseLimits::from(limits),
                budget: request.budget,
                owner: Arc::downgrade(self),
                state: Mutex::new(LeaseState::default()),
                changed: Notify::new(),
                closed,
            });
            state.used = used;
            state
                .leases
                .insert(request.identity.session_epoch.0, Arc::clone(&lease));
            Ok((lease, backend))
        })();
        if let Err(error) = &result {
            state.remember_receipt(
                request.identity.clone(),
                SearchCloseOutcome {
                    operation: Err(error.clone()),
                    cleanup: CloseCleanup::Joined,
                },
            );
        }
        result
    }
}

struct StartCompletionGuard {
    lease: Option<Arc<Lease>>,
    _token: tokio_util::task::task_tracker::TaskTrackerToken,
}

impl Drop for StartCompletionGuard {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let error = owner_lost();
            {
                let mut state = lock(&lease.state);
                state.start_finished = true;
                state.start_failure.get_or_insert(SearchStartError {
                    operation: error.clone(),
                    cleanup: StartCleanup::Unconfirmed(error.clone()),
                });
                state.closing = true;
            }
            lease.changed.notify_waiters();
            // This guard may run without an executor. Publish before its task
            // token drops; do not call external hooks or spawn a cleanup task.
            lease.finish(crate::service::uncertain(error));
        }
    }
}
