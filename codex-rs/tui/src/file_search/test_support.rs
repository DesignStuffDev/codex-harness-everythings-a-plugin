//! Controlled backend for adapter lifecycle tests; no installed-plugin claim.
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use codex_file_search_api::*;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::interactive_policy;
use tokio::sync::Semaphore;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::FileSearchRuntime;
use super::FileSearchSource;
use super::state::lock;

pub(super) struct Backend {
    pub pending: Mutex<VecDeque<Arc<Session>>>,
    pub all: Vec<Arc<Session>>,
    pub opened: Mutex<Vec<SearchOpen>>,
    pub open_entered: Semaphore,
    pub open_ready: watch::Sender<bool>,
}
impl SearchBackend for Backend {
    fn open(&self, request: SearchOpen) -> SearchStartFuture<'_> {
        lock(&self.opened).push(request);
        Box::pin(async move {
            self.open_entered.add_permits(1);
            let mut ready = self.open_ready.subscribe();
            while !*ready.borrow_and_update() {
                ready.changed().await.expect("open gate");
            }
            lock(&self.pending)
                .pop_front()
                .map(|session| session as Arc<dyn SearchBackendSession>)
                .ok_or_else(|| SearchStartError {
                    operation: SearchError::new(
                        SearchErrorKind::ResourceExhausted,
                        "fixture leases exhausted",
                    ),
                    cleanup: StartCleanup::NotAdmitted,
                })
        })
    }
    fn request_shutdown(&self) {
        for session in &self.all {
            session.request_close();
        }
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.request_shutdown();
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
        Box::pin(async move {
            self.close_entered.add_permits(1);
            let mut ready = self.close_ready.subscribe();
            while !*ready.borrow_and_update() {
                ready.changed().await.expect("close gate");
            }
            self.outcome.clone()
        })
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
    let backend = Arc::new(Backend {
        pending: Mutex::new(sessions.iter().cloned().collect()),
        all: sessions,
        opened: Mutex::new(Vec::new()),
        open_entered: Semaphore::new(0),
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
