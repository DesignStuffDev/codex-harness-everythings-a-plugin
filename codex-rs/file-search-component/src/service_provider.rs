use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::StartCleanup;
use futures::FutureExt;
use tokio::sync::oneshot;

use crate::BackendContext;
use crate::INITIALIZE_METHOD;
use crate::InitializeRequest;
use crate::InitializeResponse;
use crate::PendingServiceReply;
use crate::contract::invalid;
use crate::contract::reply;
use crate::service::Inner;
use crate::service::closed_error;
use crate::service::lock;
use crate::service::panic_error;
use crate::service::uncertain;

impl Inner {
    pub(crate) fn initialize(self: &Arc<Self>, request: InitializeRequest) -> PendingServiceReply {
        let identity = request.identity.clone();
        let result = (|| {
            request.validate()?;
            if request.base_dir.as_os_str() != self.environment.startup_dir.as_os_str() {
                return Err(invalid(
                    "file-search initialization differs from immutable startup directory",
                ));
            }
            self.ceilings.negotiate(&request.requested_limits)
        })();
        let limits = match result {
            Ok(limits) => limits,
            Err(error) => {
                return PendingServiceReply::ready(reply(
                    INITIALIZE_METHOD,
                    identity,
                    Err::<InitializeResponse, _>(error),
                ));
            }
        };
        // Own a task token before publishing accepted state. Shutdown cannot
        // observe the state and finish between admission and spawning the owner.
        let token = self.tasks.token();
        {
            let mut state = lock(&self.state);
            let error = if state.closing {
                Some(closed_error())
            } else if state.identity.is_some() {
                Some(invalid("file-search provider is already initialized"))
            } else {
                None
            };
            if let Some(error) = error {
                return PendingServiceReply::ready(reply(
                    INITIALIZE_METHOD,
                    identity,
                    Err::<InitializeResponse, _>(error),
                ));
            }
            state.identity = Some(identity.clone());
            state.limits = Some(limits.clone());
        }
        let context = BackendContext {
            base_dir: request.base_dir,
            plugin_config: self.environment.plugin_config.clone(),
            state_dir: self.environment.state_dir.clone(),
            limits: limits.clone(),
        };
        let (sender, receiver) = oneshot::channel();
        let owner = Arc::clone(self);
        self.runtime.spawn(async move {
            let _token = token;
            let created = AssertUnwindSafe(async { owner.factory.create(context).await })
                .catch_unwind()
                .await;
            let mut fence = false;
            let result = {
                let mut state = lock(&owner.state);
                state.initialized = true;
                match created {
                    Ok(Ok(backend)) => {
                        state.backend = Some(backend);
                        if state.closing {
                            fence = true;
                            Err(closed_error())
                        } else {
                            Ok(InitializeResponse {
                                limits,
                                native_path_platform: platform().to_owned(),
                                encoding: "file_search1".to_owned(),
                            })
                        }
                    }
                    failure => {
                        let (error, cleanup) = match failure {
                            Ok(Err(error)) => (
                                error.operation,
                                match error.cleanup {
                                    StartCleanup::NotAdmitted | StartCleanup::Confirmed => {
                                        CloseCleanup::Joined
                                    }
                                    StartCleanup::Unconfirmed(error) => {
                                        CloseCleanup::Unconfirmed(error)
                                    }
                                },
                            ),
                            Err(_) => {
                                let error = panic_error();
                                (error.clone(), CloseCleanup::Unconfirmed(error))
                            }
                            Ok(Ok(_)) => unreachable!(),
                        };
                        state.remember_failure(error.clone());
                        state.initialization_cleanup = Some(cleanup);
                        fence = true;
                        Err(error)
                    }
                }
            };
            if fence {
                owner.request_shutdown();
            }
            let _ = sender.send(reply(INITIALIZE_METHOD, identity, result));
        });
        PendingServiceReply {
            future: Box::pin(async move {
                receiver.await.unwrap_or_else(|_| {
                    Err("file-search initialization owner disappeared".to_owned())
                })
            }),
        }
    }

    pub(crate) fn request_shutdown(self: &Arc<Self>) {
        let (leases, backend, start) = {
            let mut state = lock(&self.state);
            state.closing = true;
            let start = !state.shutdown_started;
            state.shutdown_started = true;
            (
                state.leases.values().cloned().collect::<Vec<_>>(),
                state.backend.clone(),
                start,
            )
        };
        // Fence all leases before the tracker is closed/waited. request_close
        // registers its retained cleanup owner synchronously.
        for lease in leases {
            lease.request_close();
        }
        if let Some(backend) = backend
            && std::panic::catch_unwind(AssertUnwindSafe(|| backend.request_shutdown())).is_err()
        {
            lock(&self.state).remember_failure(panic_error());
        }
        if !start {
            return;
        }
        let owner = Arc::clone(self);
        self.runtime.spawn(async move {
            owner.tasks.close();
            owner.tasks.wait().await;
            let backend = lock(&owner.state).backend.clone();
            let mut outcome = if let Some(backend) = backend {
                match AssertUnwindSafe(async { backend.shutdown().await })
                    .catch_unwind()
                    .await
                {
                    Ok(outcome) => outcome,
                    Err(_) => uncertain(panic_error()),
                }
            } else {
                SearchCloseOutcome {
                    operation: Ok(()),
                    cleanup: lock(&owner.state)
                        .initialization_cleanup
                        .clone()
                        .unwrap_or(CloseCleanup::Joined),
                }
            };
            {
                let mut state = lock(&owner.state);
                if let Some(error) = state.first_failure.clone() {
                    outcome.operation = Err(error);
                }
                if let CloseCleanup::Joined = outcome.cleanup {
                    // Whole-provider recovery can release quarantined capacity;
                    // it never rewrites an earlier immutable lease receipt.
                    let recovered = state
                        .leases
                        .values()
                        .filter_map(|lease| {
                            lease
                                .closed
                                .borrow()
                                .clone()
                                .map(|outcome| (lease.identity.clone(), outcome))
                        })
                        .collect::<Vec<_>>();
                    for (identity, receipt) in recovered {
                        state.remember_receipt(identity, receipt);
                    }
                    state.leases.clear();
                    state.used = [0; 3];
                }
            }
            owner.closed.send_replace(Some(outcome));
        });
    }
}

pub(crate) const fn platform() -> &'static str {
    if cfg!(windows) {
        "windows_wide"
    } else if cfg!(unix) {
        "unix_bytes"
    } else {
        "unsupported"
    }
}
