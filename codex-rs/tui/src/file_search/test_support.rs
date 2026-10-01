//! Controlled backend for adapter lifecycle tests; no installed-plugin claim.
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_file_search_api::*;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::interactive_policy;
use tokio::sync::Semaphore;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tokio_util::task::task_tracker::TaskTrackerToken;

use super::FileSearchRuntime;
use super::FileSearchSource;
use super::state::lock;

#[path = "test_start.rs"]
mod startup;

#[path = "test_start_tests.rs"]
mod startup_tests;

pub(super) struct Backend {
    pub pending: Mutex<VecDeque<Arc<Session>>>,
    pub all: Vec<Arc<Session>>,
    pub opened: Mutex<Vec<SearchOpen>>,
    pub open_entered: Arc<Semaphore>,
    stopping: AtomicBool,
    starts: Mutex<Vec<Arc<dyn SearchStartControl>>>,
    tasks: TaskTracker,
    pub open_ready: watch::Sender<bool>,
}
impl SearchBackend for Backend {
    fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        lock(&self.opened).push(request);
        let mut pending = lock(&self.pending);
        if self.stopping.load(Ordering::Acquire) {
            return Err(SearchStartError {
                operation: SearchError::new(SearchErrorKind::ClosedLease, "fixture backend closed"),
                cleanup: StartCleanup::NotAdmitted,
            });
        }
        let session = pending.pop_front().ok_or_else(|| SearchStartError {
            operation: SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "fixture leases exhausted",
            ),
            cleanup: StartCleanup::NotAdmitted,
        })?;
        let ticket = startup::begin(
            session,
            self.open_ready.subscribe(),
            self.open_entered.clone(),
            self.tasks.token(),
        );
        // The finite queue bounds this ledger. Publication shares the admission
        // fence, so shutdown cannot miss a startup owner it must drain.
        lock(&self.starts).push(ticket.control());
        Ok(ticket)
    }
    fn request_shutdown(&self) {
        {
            let _admission = lock(&self.pending);
            self.stopping.store(true, Ordering::Release);
            self.tasks.close();
        }
        for session in &self.all {
            session.request_close();
        }
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.request_shutdown();
        let starts = lock(&self.starts).clone();
        Box::pin(async move {
            let mut outcome = joined();
            for session in &self.all {
                let closed = session.close().await;
                if outcome.operation.is_ok() {
                    outcome.operation = closed.operation;
                }
                if let CloseCleanup::Unconfirmed(error) = closed.cleanup {
                    outcome.cleanup = CloseCleanup::Unconfirmed(error);
                }
            }
            for start in starts {
                let cancelled = start.cancel_and_wait().await;
                if outcome.operation.is_ok() {
                    outcome.operation = cancelled.operation;
                }
                if let StartCleanup::Unconfirmed(error) = cancelled.cleanup {
                    outcome.cleanup = CloseCleanup::Unconfirmed(error);
                }
            }
            self.tasks.wait().await;
            outcome
        })
    }
}

pub(super) struct Session {
    pub updates: Mutex<Vec<SearchQuery>>,
    pub admitted: Semaphore,
    pub acknowledgements: Semaphore,
    pub update_error: Mutex<Option<SearchError>>,
    pub frames: watch::Sender<Option<SearchFrame>>,
    pub closing: CancellationToken,
    pub close_entered: Semaphore,
    pub close_ready: watch::Sender<bool>,
    pub outcome: SearchCloseOutcome,
    closed: watch::Sender<Option<SearchCloseOutcome>>,
}
impl Session {
    pub fn new(acknowledgements: usize) -> Arc<Self> {
        Self::with_outcome(acknowledgements, joined())
    }
    pub fn with_outcome(acknowledgements: usize, outcome: SearchCloseOutcome) -> Arc<Self> {
        Arc::new(Self {
            updates: Mutex::new(Vec::new()),
            admitted: Semaphore::new(0),
            acknowledgements: Semaphore::new(acknowledgements),
            update_error: Mutex::new(None),
            frames: watch::channel(None).0,
            closing: CancellationToken::new(),
            close_entered: Semaphore::new(0),
            close_ready: watch::channel(true).0,
            outcome,
            closed: watch::channel(None).0,
        })
    }
    pub fn emit(&self, phase: SearchPhase) {
        let query = lock(&self.updates).last().expect("submitted query").clone();
        let snapshot = matches!(phase, SearchPhase::Idle).then(|| FileSearchSnapshot {
            query_id: query.id.get(),
            query: query.text.clone(),
            matches: Vec::new(),
            total_match_count: 0,
            scanned_file_count: 1,
            walk_complete: true,
        });
        self.frames.send_replace(Some(SearchFrame {
            revision: 1,
            query_id: query.id.get(),
            query: query.text,
            snapshot,
            phase,
        }));
    }
}
impl SearchBackendSession for Session {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        Box::pin(async move {
            lock(&self.updates).push(query.clone());
            self.admitted.add_permits(1);
            tokio::select! {
                biased;
                _ = self.closing.cancelled() => Err(SearchError::new(SearchErrorKind::ClosedLease, "fixture closed")),
                permit = self.acknowledgements.acquire() => {
                    permit.expect("ack gate").forget();
                    if let Some(error) = lock(&self.update_error).clone() { return Err(error); }
                    Ok(QueryAccepted { id: query.id })
                }
            }
        })
    }
    fn next_snapshot(&self, after: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(async move {
            let mut frames = self.frames.subscribe();
            let deadline = tokio::time::Instant::now() + wait;
            loop {
                if let Some(frame) = frames.borrow_and_update().clone()
                    && frame.revision > after
                {
                    return Ok(SearchPoll::Changed(frame));
                }
                tokio::select! {
                    _ = self.closing.cancelled() => return Err(SearchError::new(SearchErrorKind::ClosedLease, "fixture closed")),
                    _ = frames.changed() => {},
                    _ = tokio::time::sleep_until(deadline) => return Ok(SearchPoll::Unchanged { revision: after }),
                }
            }
        })
    }
    fn request_close(&self) {
        self.closing.cancel();
    }
    fn close(&self) -> SearchCloseFuture<'_> {
        self.request_close();
        Box::pin(startup::observe_close(self.closed.subscribe()))
    }
}

pub(super) fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}
pub(super) async fn fixture(
    sessions: Vec<Arc<Session>>,
) -> (FileSearchRuntime, FileSearchProvider, Arc<Backend>) {
    let tasks = TaskTracker::new();
    for session in &sessions {
        startup::own_close(session.clone(), tasks.token());
    }
    let backend = Arc::new(Backend {
        pending: Mutex::new(sessions.iter().cloned().collect()),
        all: sessions,
        opened: Mutex::new(Vec::new()),
        open_entered: Arc::new(Semaphore::new(0)),
        stopping: AtomicBool::new(false),
        starts: Mutex::new(Vec::new()),
        tasks,
        open_ready: watch::channel(true).0,
    });
    let policy = interactive_policy().expect("explicit fixture profile");
    let provider =
        FileSearchProvider::from_backend(backend.clone(), policy.runtime).expect("facade");
    let runtime = FileSearchRuntime::start(FileSearchSource::Embedded(provider.scope_factory()))
        .await
        .expect("borrowed TUI scope");
    (runtime, provider, backend)
}
pub(super) async fn permit(gate: &Semaphore) {
    tokio::time::timeout(Duration::from_secs(2), gate.acquire())
        .await
        .expect("gate deadline")
        .expect("gate")
        .forget();
}
