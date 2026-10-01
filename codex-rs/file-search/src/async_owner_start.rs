//! Synchronously admitted native starts and result-owner loss receipts.

use super::*;
use codex_file_search_api::SearchStartCancellationFuture;
use codex_file_search_api::SearchStartCancellationOutcome;
use codex_file_search_api::SearchStartControl;
use std::future::Future;
use std::pin::Pin;

type OwnerStartFuture =
    Pin<Box<dyn Future<Output = Result<ManagedFileSearchSession, CreateFailure>> + Send>>;

/// Internal public-owner separation: controls hold receipt state, not a lease.
pub(crate) struct PendingNativeSearchStart {
    inner: OwnerStart,
}

impl PendingNativeSearchStart {
    pub(crate) fn control(&self) -> Arc<dyn SearchStartControl> {
        self.inner.control.clone()
    }

    pub(crate) async fn finish(self) -> Result<ManagedFileSearchSession, SearchStartError> {
        self.inner.completion.await.map_err(|error| match error {
            CreateFailure::Rejected(error) | CreateFailure::Accepted { error, .. } => error,
        })
    }
}

pub(super) struct OwnerStart {
    control: Arc<OwnerStartControl>,
    completion: OwnerStartFuture,
}

impl OwnerStart {
    pub(super) fn finish(self) -> OwnerStartFuture {
        self.completion
    }
}

struct OwnerStartControl {
    owner: Arc<OwnerInner>,
    id: u64,
    cancelled: Arc<Completion<SearchStartCancellationOutcome>>,
}

impl SearchStartControl for OwnerStartControl {
    fn request_cancel(&self) {
        self.owner.release(self.id);
    }

    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        let cancelled = self.cancelled.clone();
        Box::pin(async move { cancelled.wait().await })
    }
}

impl FileSearchOwner {
    /// Admit one backend constructor and expose its cancellation before waiting.
    pub(crate) fn begin_create_backend(
        &self,
        roots: Vec<PathBuf>,
        options: FileSearchOptions,
        budget: SearchBudget,
        output: crate::native_output::NativeOutputLimits,
        reporter: Arc<dyn SessionReporter>,
    ) -> Result<PendingNativeSearchStart, SearchStartError> {
        self.begin_create_inner(
            roots,
            options,
            reporter,
            /*cancel_flag*/ None,
            NativePolicy::Backend { budget, output },
        )
        .map(|inner| PendingNativeSearchStart { inner })
        .map_err(|error| match error {
            CreateFailure::Rejected(error) | CreateFailure::Accepted { error, .. } => error,
        })
    }

    pub(super) fn begin_create_inner(
        &self,
        roots: Vec<PathBuf>,
        options: FileSearchOptions,
        reporter: Arc<dyn SessionReporter>,
        cancel_flag: Option<Arc<AtomicBool>>,
        policy: NativePolicy,
    ) -> Result<OwnerStart, CreateFailure> {
        let legacy = matches!(policy, NativePolicy::Legacy);
        let runtime = Handle::try_current().map_err(|error| {
            CreateFailure::rejected(SearchErrorKind::SearchFailed, error.to_string())
        })?;
        let (id, ready, closed, cancelled, shutdown, token) = {
            let mut state = self
                .inner
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.stopping {
                return Err(CreateFailure::rejected(
                    SearchErrorKind::ClosedLease,
                    "file-search owner is shutting down",
                ));
            }
            if state.entries.len() >= self.inner.capacity {
                return Err(CreateFailure::rejected(
                    SearchErrorKind::ResourceExhausted,
                    "file-search owner session capacity reached",
                ));
            }
            let id = state.next_id.checked_add(1).ok_or_else(|| {
                CreateFailure::rejected(
                    SearchErrorKind::ResourceExhausted,
                    "file-search session identities exhausted",
                )
            })?;
            let ready = Arc::new(Completion::default());
            let closed = Arc::new(Completion::default());
            let cancelled = Arc::new(Completion::default());
            let shutdown = Arc::new(AtomicBool::new(false));
            let token = self.inner.tasks.token();
            state.next_id = id;
            state.entries.insert(
                id,
                Entry {
                    phase: Phase::Preparing { released: false },
                    runtime: runtime.clone(),
                    ready: ready.clone(),
                    closed: closed.clone(),
                    cancelled: cancelled.clone(),
                    shutdown: shutdown.clone(),
                },
            );
            (id, ready, closed, cancelled, shutdown, token)
        };
        let session = ManagedFileSearchSession {
            lease: Arc::new(Lease {
                owner: self.inner.clone(),
                id,
                closed,
            }),
        };
        let control = Arc::new(OwnerStartControl {
            owner: self.inner.clone(),
            id,
            cancelled,
        });
        // The guard precedes both spawns, including destruction before first poll.
        let native_failure = Arc::new(Mutex::new(None));
        let guard = ResultOwnerGuard::new(
            self.inner.clone(),
            id,
            ResultStage::Startup(native_failure.clone()),
            token,
        );
        let startup_control = crate::native_session::NativeStartup {
            shutdown,
            failure: native_failure.clone(),
            #[cfg(test)]
            after_pool: self
                .inner
                .after_pool
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        };
        let startup = runtime.spawn_blocking(move || {
            match policy {
                // Keep the legacy API's deferred-construction/drop-error behavior.
                // Only bounded/backend starts expose the new Preparing guarantee.
                NativePolicy::Legacy => create_session_with_receipt(
                    roots,
                    options,
                    reporter,
                    cancel_flag,
                    NativePolicy::Legacy,
                ),
                policy => crate::native_session::create_session_with_private_shutdown(
                    roots,
                    options,
                    reporter,
                    cancel_flag,
                    policy,
                    startup_control,
                ),
            }
        });
        let owner = self.inner.clone();
        runtime.spawn(async move {
            let mut guard = guard;
            let result = startup.await.unwrap_or_else(|error| {
                let cleanup = task_failure("startup", error);
                let operation = native_failure
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone()
                    .unwrap_or_else(|| cleanup.clone());
                Err(SearchStartError {
                    cleanup: StartCleanup::Unconfirmed(cleanup),
                    operation,
                })
            });
            owner.started(id, result);
            guard.disarm();
        });
        let completion = Box::pin(async move {
            if let Err(failure) = ready.wait().await {
                let outcome = session.close_outcome().await;
                let error = match failure {
                    ReadyFailure::Construction(error) => error,
                    ReadyFailure::Released(operation) => SearchStartError {
                        // Keep the legacy cancelled-start wrapper's original
                        // error split; typed starts retain the actual native cause.
                        operation: if legacy {
                            operation
                        } else {
                            outcome.operation.clone().err().unwrap_or(operation)
                        },
                        cleanup: outcome.cleanup.clone().into(),
                    },
                };
                return Err(CreateFailure::Accepted {
                    error,
                    legacy_cleanup: legacy_close_error(&outcome),
                });
            }
            // A ready notification is not the handoff. Serialize the final
            // public transfer against release while this exact identity exists.
            let failure = {
                let state = session
                    .lease
                    .owner
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match state.entries.get(&id).map(|entry| &entry.phase) {
                    Some(Phase::Ready(_)) => None,
                    Some(Phase::Preparing { .. } | Phase::Closing | Phase::Quarantined) | None => {
                        Some(SearchError::new(
                            SearchErrorKind::ClosedLease,
                            "file-search session closed during startup",
                        ))
                    }
                }
            };
            if let Some(operation) = failure {
                let outcome = session.close_outcome().await;
                return Err(CreateFailure::Accepted {
                    error: SearchStartError {
                        operation: outcome.operation.clone().err().unwrap_or(operation),
                        cleanup: outcome.cleanup.clone().into(),
                    },
                    legacy_cleanup: legacy_close_error(&outcome),
                });
            }
            Ok(session)
        });
        Ok(OwnerStart {
            control,
            completion,
        })
    }
}

pub(super) enum ResultStage {
    Startup(Arc<Mutex<Option<SearchError>>>),
    Close(Arc<Mutex<Option<SearchError>>>),
}

pub(super) struct ResultOwnerGuard {
    owner: Arc<OwnerInner>,
    id: u64,
    stage: ResultStage,
    armed: bool,
    _token: TaskTrackerToken,
}

impl ResultOwnerGuard {
    pub(super) fn new(
        owner: Arc<OwnerInner>,
        id: u64,
        stage: ResultStage,
        token: TaskTrackerToken,
    ) -> Self {
        Self {
            owner,
            id,
            stage,
            armed: true,
            _token: token,
        }
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ResultOwnerGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let cleanup = SearchError::new(
            SearchErrorKind::ForcedShutdown,
            "file-search result owner disappeared before cleanup was observed",
        );
        let operation = match &self.stage {
            ResultStage::Startup(failure) | ResultStage::Close(failure) => failure
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
                .unwrap_or_else(|| cleanup.clone()),
        };
        let mut state = self
            .owner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(entry) = state.entries.get_mut(&self.id) else {
            return;
        };
        // These owners only hold Preparing/Closing. Do not drop a live native
        // session, callback owner, or user object while unwinding under this lock.
        match &entry.phase {
            Phase::Ready(_) => return,
            Phase::Preparing { .. } | Phase::Closing | Phase::Quarantined => {}
        }
        entry.shutdown.store(true, Ordering::Release);
        entry.phase = Phase::Quarantined;
        let ready = entry.ready.clone();
        let closed = entry.closed.clone();
        let cancelled = entry.cancelled.clone();
        let outcome = SearchCloseOutcome {
            operation: Err(operation.clone()),
            cleanup: CloseCleanup::Unconfirmed(cleanup.clone()),
        };
        record_outcome(&mut state, &outcome);
        drop(state);
        ready.finish(Err(ReadyFailure::Construction(SearchStartError {
            operation: operation.clone(),
            cleanup: StartCleanup::Unconfirmed(cleanup.clone()),
        })));
        cancelled.finish(SearchStartCancellationOutcome {
            operation: Err(operation),
            cleanup: StartCleanup::Unconfirmed(cleanup),
        });
        closed.finish(outcome);
    }
}

#[cfg(test)]
#[path = "async_owner_start_tests.rs"]
mod tests;
