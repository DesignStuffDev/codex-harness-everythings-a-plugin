//! Retained native implementation of the replaceable search contract.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::SearchStartFuture;
use codex_file_search_api::StartCleanup;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio_util::task::TaskTracker;
use tokio_util::task::task_tracker::TaskTrackerToken;

use crate::FileSearchOwner;
use crate::native_backend_policy::NativeBackendLimits;
use crate::native_backend_policy::closed;
use crate::native_backend_policy::exhausted;
use crate::native_backend_policy::failed;
use crate::native_backend_policy::invalid;
use crate::native_backend_session::NativeLease;
use crate::native_backend_session::NativeSession;
use crate::native_backend_session::SessionReporterBridge;

/// One selected native provider, independent of presentation and wire transport.
/// The embedding process must keep its cwd stable for this provider's lifetime.
pub struct NativeSearchBackend {
    inner: Arc<Provider>,
}

impl NativeSearchBackend {
    pub fn new(base_dir: PathBuf, limits: NativeBackendLimits) -> Result<Self, SearchError> {
        limits.validate()?;
        if !base_dir.is_absolute()
            || std::env::current_dir()
                .map_err(|_| invalid("native file-search working directory is unavailable"))?
                .as_os_str()
                != base_dir.as_os_str()
        {
            return Err(invalid(
                "native file-search base must equal the immutable process working directory",
            ));
        }
        let runtime = Handle::try_current()
            .map_err(|_| invalid("native file-search backend requires an active runtime"))?;
        let (closed, _) = watch::channel(None);
        Ok(Self {
            inner: Arc::new(Provider {
                owner: FileSearchOwner::new(limits.max_sessions),
                base_dir,
                limits,
                runtime,
                state: Mutex::new(Registry::default()),
                tasks: TaskTracker::new(),
                closed,
            }),
        })
    }
}

impl SearchBackend for NativeSearchBackend {
    fn open(&self, request: SearchOpen) -> SearchStartFuture<'_> {
        Box::pin(async move {
            let request = self
                .inner
                .limits
                .prepare_open(request, &self.inner.base_dir)
                .map_err(not_admitted)?;
            let (ready_tx, ready_rx) = oneshot::channel();
            let (session, token) = {
                let mut state = lock(&self.inner.state);
                if state.stopping {
                    return Err(not_admitted(closed()));
                }
                if state.sessions.len() >= self.inner.limits.max_sessions.get() {
                    return Err(not_admitted(exhausted(
                        "native file-search provider session capacity exhausted",
                    )));
                }
                let allocation = amounts(request.budget);
                let ceiling = amounts(self.inner.limits.resources);
                let mut next = [0; 3];
                for index in 0..3 {
                    next[index] = state.used[index]
                        .checked_add(allocation[index])
                        .filter(|value| *value <= ceiling[index])
                        .ok_or_else(|| {
                            not_admitted(exhausted(
                                "native file-search aggregate resource capacity exhausted",
                            ))
                        })?;
                }
                let id = state.next_id.checked_add(1).ok_or_else(|| {
                    not_admitted(exhausted(
                        "native file-search provider identities exhausted",
                    ))
                })?;
                let session = Arc::new(NativeSession::new(id, allocation, self.inner.limits));
                let token = self.inner.tasks.token();
                state.next_id = id;
                state.used = next;
                state.sessions.insert(id, session.clone());
                (session, token)
            };
            // Only the observing open future holds this public lease. Its drop
            // fences the session even while startup/lifecycle tasks retain state.
            let public: Arc<dyn SearchBackendSession> = Arc::new(NativeLease {
                inner: session.clone(),
            });
            let provider = self.inner.clone();
            let guard = SessionTaskGuard {
                provider: provider.clone(),
                session: session.clone(),
                armed: true,
                _token: token,
            };
            self.inner.runtime.spawn(async move {
                let mut guard = guard;
                run_session(provider, session, request, ready_tx).await;
                guard.armed = false;
            });
            match ready_rx.await {
                Ok(Ok(())) => Ok(public),
                Ok(Err(error)) => Err(error),
                Err(_) => Err(SearchStartError {
                    operation: failed("native file-search startup owner disappeared"),
                    cleanup: StartCleanup::Unconfirmed(failed(
                        "native file-search startup receipt was lost",
                    )),
                }),
            }
        })
    }

    fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.request_shutdown();
        Box::pin(observe_close(self.inner.closed.subscribe()))
    }
}

impl Drop for NativeSearchBackend {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

pub(super) struct Provider {
    owner: FileSearchOwner,
    base_dir: PathBuf,
    limits: NativeBackendLimits,
    runtime: Handle,
    state: Mutex<Registry>,
    tasks: TaskTracker,
    closed: watch::Sender<Option<SearchCloseOutcome>>,
}

#[derive(Default)]
struct Registry {
    sessions: HashMap<u64, Arc<NativeSession>>,
    next_id: u64,
    used: [usize; 3],
    stopping: bool,
    operation: Option<SearchError>,
    uncertainty: Option<SearchError>,
}

impl Provider {
    fn finish(&self, session: &NativeSession, outcome: &SearchCloseOutcome) {
        let mut state = lock(&self.state);
        if let Err(error) = &outcome.operation {
            state.operation.get_or_insert_with(|| error.clone());
        }
        if let CloseCleanup::Unconfirmed(error) = &outcome.cleanup {
            state.uncertainty.get_or_insert_with(|| error.clone());
        } else if state.sessions.remove(&session.id).is_some() {
            for index in 0..3 {
                state.used[index] -= session.allocation[index];
            }
        }
    }

    fn request_shutdown(self: &Arc<Self>) {
        let sessions = {
            let mut state = lock(&self.state);
            if state.stopping {
                return;
            }
            state.stopping = true;
            state.sessions.values().cloned().collect::<Vec<_>>()
        };
        for session in sessions {
            session.request_close();
        }
        self.owner.request_shutdown();
        self.tasks.close();
        let provider = self.clone();
        let guard = ShutdownGuard {
            provider: provider.clone(),
            armed: true,
        };
        self.runtime.spawn(async move {
            let mut guard = guard;
            provider.tasks.wait().await;
            let mut outcome = provider.owner.shutdown_outcome().await;
            {
                let state = lock(&provider.state);
                if let Some(error) = &state.operation {
                    outcome.operation = Err(error.clone());
                }
                if let Some(error) = &state.uncertainty {
                    outcome.cleanup = CloseCleanup::Unconfirmed(error.clone());
                } else if !state.sessions.is_empty() {
                    outcome.cleanup = CloseCleanup::Unconfirmed(failed(
                        "native file-search provider retains unjoined sessions",
                    ));
                }
            }
            provider.closed.send_replace(Some(outcome));
            guard.armed = false;
        });
    }
}

async fn run_session(
    provider: Arc<Provider>,
    session: Arc<NativeSession>,
    request: SearchOpen,
    ready: oneshot::Sender<Result<(), SearchStartError>>,
) {
    let reporter = Arc::new(SessionReporterBridge(session.clone()));
    let native = match provider
        .owner
        .create_backend(
            request.roots,
            request.options,
            request.budget,
            provider.limits.output(),
            reporter,
        )
        .await
    {
        Ok(native) => native,
        Err(error) => {
            let cleanup = match &error.cleanup {
                StartCleanup::NotAdmitted | StartCleanup::Confirmed => CloseCleanup::Joined,
                StartCleanup::Unconfirmed(error) => CloseCleanup::Unconfirmed(error.clone()),
            };
            let operation = if session.is_closing()
                && error.operation.kind() == codex_file_search_api::SearchErrorKind::ClosedLease
                && cleanup == CloseCleanup::Joined
            {
                Ok(())
            } else {
                Err(error.operation.clone())
            };
            let outcome = session.finish(SearchCloseOutcome { operation, cleanup });
            provider.finish(&session, &outcome);
            session.publish_close(outcome);
            let _ = ready.send(Err(error));
            return;
        }
    };
    session.started(native.clone());
    if session.is_closing() {
        let outcome = native.close_outcome().await;
        let outcome = session.finish(outcome);
        provider.finish(&session, &outcome);
        session.publish_close(outcome.clone());
        let _ = ready.send(Err(SearchStartError {
            operation: outcome.operation.err().unwrap_or_else(closed),
            cleanup: outcome.cleanup.into(),
        }));
        return;
    }
    if ready.send(Ok(())).is_err() {
        session.request_close();
    }
    let mut changes = session.changed.subscribe();
    let mut unexpected_stop = false;
    loop {
        session.service_poll();
        if session.is_closing() {
            break;
        }
        if native.is_finished() {
            unexpected_stop = true;
            break;
        }
        let check = tokio::time::Instant::now() + std::time::Duration::from_millis(20);
        let deadline = session
            .poll_deadline()
            .map_or(check, |deadline| deadline.min(check));
        tokio::select! {
            _ = changes.changed() => {},
            _ = tokio::time::sleep_until(deadline) => {},
        }
    }
    session.request_close();
    let outcome = native.close_outcome().await;
    if unexpected_stop && outcome.operation.is_ok() {
        session.fail(failed(
            "native file-search worker terminated without closure",
        ));
    }
    let outcome = session.finish(outcome);
    provider.finish(&session, &outcome);
    session.publish_close(outcome);
}

struct SessionTaskGuard {
    provider: Arc<Provider>,
    session: Arc<NativeSession>,
    armed: bool,
    _token: TaskTrackerToken,
}
impl Drop for SessionTaskGuard {
    fn drop(&mut self) {
        if self.armed {
            self.session.request_close();
            let error =
                failed("native file-search lifecycle owner ended without a cleanup receipt");
            let outcome = self.session.finish(SearchCloseOutcome {
                operation: Err(error.clone()),
                cleanup: CloseCleanup::Unconfirmed(error),
            });
            self.provider.finish(&self.session, &outcome);
            self.session.publish_close(outcome);
        }
    }
}

fn amounts(budget: SearchBudget) -> [usize; 3] {
    [
        budget.max_index_entries.get(),
        budget.max_index_bytes.get(),
        budget.max_worker_threads.get(),
    ]
}
fn not_admitted(operation: SearchError) -> SearchStartError {
    SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    }
}
pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
pub(super) async fn observe_close(
    mut receiver: watch::Receiver<Option<SearchCloseOutcome>>,
) -> SearchCloseOutcome {
    loop {
        if let Some(outcome) = receiver.borrow_and_update().clone() {
            return outcome;
        }
        if receiver.changed().await.is_err() {
            let error = failed("native file-search cleanup receipt owner disappeared");
            return SearchCloseOutcome {
                operation: Err(error.clone()),
                cleanup: CloseCleanup::Unconfirmed(error),
            };
        }
    }
}

struct ShutdownGuard {
    provider: Arc<Provider>,
    armed: bool,
}
impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        if self.armed {
            let error = failed("native file-search shutdown owner ended without a cleanup receipt");
            let operation = lock(&self.provider.state)
                .operation
                .clone()
                .unwrap_or_else(|| error.clone());
            self.provider.closed.send_replace(Some(SearchCloseOutcome {
                operation: Err(operation),
                cleanup: CloseCleanup::Unconfirmed(error),
            }));
        }
    }
}
