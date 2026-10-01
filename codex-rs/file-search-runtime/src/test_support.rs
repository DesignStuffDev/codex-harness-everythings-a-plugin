#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::collections::VecDeque;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_file_search_api::*;
use tokio::sync::Notify;
use tokio::sync::Semaphore;
use tokio::sync::watch;

use crate::state::lock;
use crate::*;

pub(super) struct Backend {
    pub sessions: Mutex<VecDeque<Arc<Session>>>,
    pub all: Vec<Arc<Session>>,
    pub starts: Semaphore,
    pub opened: Semaphore,
    pub stopped: AtomicBool,
    pub panic_shutdown: AtomicBool,
    pub shutdowns: Semaphore,
    pub shutdown_entered: Semaphore,
    pub outcome: SearchCloseOutcome,
}
impl Backend {
    pub fn new(sessions: Vec<Arc<Session>>, starts: usize) -> Arc<Self> {
        Arc::new(Self {
            sessions: Mutex::new(sessions.iter().cloned().collect()),
            all: sessions,
            starts: Semaphore::new(starts),
            opened: Semaphore::new(0),
            shutdowns: Semaphore::new(1),
            shutdown_entered: Semaphore::new(0),
            stopped: AtomicBool::new(false),
            panic_shutdown: AtomicBool::new(false),
            outcome: joined(),
        })
    }
}
impl SearchBackend for Backend {
    fn open(&self, _: SearchOpen) -> SearchStartFuture<'_> {
        Box::pin(async move {
            self.opened.add_permits(1);
            self.starts.acquire().await.expect("start gate").forget();
            lock(&self.sessions)
                .pop_front()
                .map(|value| value as Arc<dyn SearchBackendSession>)
                .ok_or_else(|| SearchStartError {
                    operation: failure(),
                    cleanup: StartCleanup::NotAdmitted,
                })
        })
    }
    fn request_shutdown(&self) {
        assert!(
            !self.panic_shutdown.load(Ordering::SeqCst),
            "controlled shutdown hook panic"
        );
        self.stopped.store(true, Ordering::SeqCst);
        for session in &self.all {
            session.request_close();
        }
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            for session in &self.all {
                session.closing.store(true, Ordering::SeqCst);
                session.changed.notify_waiters();
            }
            self.shutdown_entered.add_permits(1);
            self.shutdowns
                .acquire()
                .await
                .expect("shutdown gate")
                .forget();
            self.outcome.clone()
        })
    }
}

pub(super) struct Session {
    pub frame: watch::Sender<Option<SearchFrame>>,
    pub closing: AtomicBool,
    pub panic_close: AtomicBool,
    pub close_releases_update: AtomicBool,
    pub update_error: Mutex<Option<SearchError>>,
    pub changed: Notify,
    pub updates: Semaphore,
    pub update_entered: Semaphore,
    pub closes: Semaphore,
    pub close_entered: Semaphore,
    pub close_count: AtomicUsize,
    pub outcome: SearchCloseOutcome,
}
impl Session {
    pub fn new(closes: usize, outcome: SearchCloseOutcome) -> Arc<Self> {
        Arc::new(Self {
            frame: watch::channel(None).0,
            closing: AtomicBool::new(false),
            panic_close: AtomicBool::new(false),
            close_releases_update: AtomicBool::new(false),
            update_error: Mutex::new(None),
            changed: Notify::new(),
            updates: Semaphore::new(100),
            update_entered: Semaphore::new(0),
            closes: Semaphore::new(closes),
            close_entered: Semaphore::new(0),
            close_count: AtomicUsize::new(0),
            outcome,
        })
    }
    pub fn emit(&self, revision: u64, id: u64, query: &str, idle: bool) {
        self.frame.send_replace(Some(SearchFrame {
            revision,
            query_id: id,
            query: query.to_owned(),
            snapshot: Some(FileSearchSnapshot {
                query_id: id,
                query: query.to_owned(),
                matches: vec![],
                total_match_count: 0,
                scanned_file_count: 0,
                walk_complete: idle,
            }),
            phase: if idle {
                SearchPhase::Idle
            } else {
                SearchPhase::Running
            },
        }));
    }
}
impl SearchBackendSession for Session {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        Box::pin(async move {
            self.update_entered.add_permits(1);
            self.updates.acquire().await.expect("update gate").forget();
            if let Some(error) = lock(&self.update_error).clone() {
                return Err(error);
            }
            Ok(QueryAccepted { id: query.id })
        })
    }
    fn next_snapshot(&self, after: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(async move {
            let mut frames = self.frame.subscribe();
            let deadline = tokio::time::Instant::now() + wait;
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.closing.load(Ordering::SeqCst) {
                    return Err(SearchError::new(
                        SearchErrorKind::ClosedLease,
                        "closed fixture",
                    ));
                }
                if let Some(frame) = frames.borrow_and_update().clone()
                    && frame.revision > after
                {
                    return Ok(SearchPoll::Changed(frame));
                }
                tokio::select! {
                    _ = &mut changed => {},
                    _ = frames.changed() => {},
                    _ = tokio::time::sleep_until(deadline) => return Ok(SearchPoll::Unchanged { revision: after }),
                }
            }
        })
    }
    fn request_close(&self) {
        assert!(
            !self.panic_close.load(Ordering::SeqCst),
            "controlled close hook panic"
        );
        self.closing.store(true, Ordering::SeqCst);
        self.changed.notify_waiters();
    }
    fn close(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            self.closing.store(true, Ordering::SeqCst);
            self.changed.notify_waiters();
            if self.close_releases_update.load(Ordering::SeqCst) {
                self.updates.add_permits(1);
            }
            self.close_count.fetch_add(1, Ordering::SeqCst);
            self.close_entered.add_permits(1);
            self.closes.acquire().await.expect("close gate").forget();
            self.outcome.clone()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Event {
    Update(u64, String),
    Complete(u64),
    Error(SearchErrorKind),
}
#[derive(Default)]
pub(super) struct Reporter {
    pub events: Mutex<Vec<Event>>,
    pub changed: Notify,
}
impl SessionReporter for Reporter {
    fn on_update(&self, value: &FileSearchSnapshot) {
        lock(&self.events).push(Event::Update(value.query_id, value.query.clone()));
        self.changed.notify_waiters();
    }
    fn on_complete(&self) {
        panic!("untagged completion must not be used");
    }
    fn on_complete_tagged(&self, id: u64) {
        lock(&self.events).push(Event::Complete(id));
        self.changed.notify_waiters();
    }
    fn on_error(&self, error: &SearchError) {
        lock(&self.events).push(Event::Error(error.kind()));
        self.changed.notify_waiters();
    }
}
impl Reporter {
    pub async fn wait_len(&self, minimum: usize) {
        bounded(async {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if lock(&self.events).len() >= minimum {
                    return;
                }
                changed.await;
            }
        })
        .await
    }
}
pub(super) fn positive(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
pub(super) fn budget() -> SearchBudget {
    SearchBudget {
        max_index_entries: positive(10),
        max_index_bytes: positive(4096),
        max_worker_threads: positive(6),
    }
}
pub(super) fn policy() -> RuntimePolicy {
    RuntimePolicy {
        provider: ProviderLimits {
            max_scopes: positive(2),
            max_sessions: positive(2),
            resources: SearchBudget {
                max_index_entries: positive(20),
                max_index_bytes: positive(8192),
                max_worker_threads: positive(12),
            },
        },
        max_query_bytes: positive(64),
        max_roots_options_bytes: positive(4096),
        max_matches: positive(20),
        max_frame_retained_bytes: positive(4096),
        poll_wait: Duration::from_millis(20),
    }
}
pub(super) fn request() -> SearchOpen {
    SearchOpen {
        roots: vec![".".into()],
        options: FileSearchOptions::default(),
        budget: budget(),
    }
}
pub(super) fn query(id: u64, text: &str) -> SearchQuery {
    SearchQuery {
        id: NonZeroU64::new(id).unwrap(),
        text: text.to_owned(),
    }
}
pub(super) fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}
pub(super) fn failure() -> SearchError {
    SearchError::new(SearchErrorKind::SearchFailed, "controlled fixture failure")
}
pub(super) fn provider(backend: Arc<Backend>) -> FileSearchProvider {
    FileSearchProvider::from_backend(backend, policy()).unwrap()
}
pub(super) fn scope(provider: &FileSearchProvider) -> FileSearchScope {
    provider
        .scope_factory()
        .new_scope(ScopeLimits {
            max_sessions: positive(1),
        })
        .unwrap()
}
pub(super) async fn bounded<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .expect("test deadline")
}
