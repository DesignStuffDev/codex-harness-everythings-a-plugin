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
        self.runtime.spawn(async move {
            let _token = token;
            // reserve validated the conversion; even an unexpected conversion
            // error still goes through the retained startup/close owner.
            let started = match request.into_native() {
                Ok(request) => match AssertUnwindSafe(async { backend.open(request).await })
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
            };
            let (closing, result) = {
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
                        )
                    }
                    Err(error) => {
                        state.start_failure = Some(error.clone());
                        (true, Err(error))
                    }
                }
            };
            lease.changed.notify_waiters();
            let result = if closing {
                lease.request_close();
                let outcome = lease.outcome().await;
                Err(SearchStartError {
                    operation: result
                        .err()
                        .map(|error| error.operation)
                        .unwrap_or_else(closed_error),
                    cleanup: outcome.cleanup.into(),
                })
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
