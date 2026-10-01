//! Retained native implementation of the replaceable search contract.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::PendingSearchStart;
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

#[path = "native_backend_start.rs"]
mod startup;

/// One selected native provider, independent of presentation and wire transport.
/// The embedding process must keep its cwd stable for this provider's lifetime.
pub struct NativeSearchBackend {
    inner: Arc<Provider>,
}

impl NativeSearchBackend {
    /// Reserve one bounded start and expose cancellation before native construction.
    /// The same control targets this lease after a successful ready handoff.
    pub fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        startup::begin_open(self.inner.clone(), request)
    }

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
                #[cfg(test)]
                before_pending: Mutex::new(None),
                #[cfg(test)]
                before_handoff: Mutex::new(None),
            }),
        })
    }
}

impl SearchBackend for NativeSearchBackend {
    fn open(&self, request: SearchOpen) -> SearchStartFuture<'_> {
        match self.begin_open(request) {
            Ok(pending) => pending.finish(),
            Err(error) => Box::pin(async move { Err(error) }),
        }
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
    #[cfg(test)]
    before_pending: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    #[cfg(test)]
    before_handoff: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
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
    if session.is_closing() {
        let outcome = session.finish(SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        });
        provider.finish(&session, &outcome);
        session.publish_close(outcome, StartCleanup::NotAdmitted);
        let _ = ready.send(Err(not_admitted(closed())));
        return;
    }
    let reporter = Arc::new(SessionReporterBridge(session.clone()));
    let pending = provider.owner.begin_create_backend(
        request.roots,
        request.options,
        request.budget,
        provider.limits.output(),
        reporter,
    );
    let result = match pending {
        Ok(pending) => {
            #[cfg(test)]
            {
                let before_pending = lock(&session.before_pending).clone();
                if let Some(before_pending) = before_pending {
                    before_pending();
                }
            }
            session.preparing(pending.control());
            pending.finish().await
        }
        Err(error) => Err(error),
    };
    let native = match result {
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
            session.publish_close(outcome.clone(), error.cleanup.clone());
            let _ = ready.send(Err(SearchStartError {
                operation: outcome.operation.err().unwrap_or(error.operation),
                cleanup: error.cleanup,
            }));
            return;
        }
    };
    session.started(native.clone());
    if session.is_closing() {
        let outcome = native.close_outcome().await;
        let outcome = session.finish(outcome);
        provider.finish(&session, &outcome);
        session.publish_close(outcome.clone(), outcome.cleanup.clone().into());
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
    session.publish_close(outcome.clone(), outcome.cleanup.into());
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
            self.session
                .publish_close(outcome.clone(), outcome.cleanup.into());
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

#[cfg(test)]
#[path = "native_backend_failure_tests.rs"]
mod failure_tests;
