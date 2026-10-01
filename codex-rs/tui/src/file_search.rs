//! Owned native searches for `@` tokens, with identities checked at UI delivery.

use codex_file_search as file_search;
use std::future::Future;
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use tokio_util::task::TaskTracker;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

/// Lives outside the complete App::run future, including its startup error paths.
/// Native ownership remains with FileSearchOwner; this tracker joins the TUI's
/// asynchronous admission and result-delivery callbacks around that owner.
#[derive(Clone)]
pub(crate) struct FileSearchRuntime(Arc<RuntimeInner>);

struct RuntimeInner {
    native: file_search::FileSearchOwner,
    tasks: TaskTracker,
    closing: Mutex<bool>,
}

impl FileSearchRuntime {
    pub(crate) fn new() -> Self {
        // Allow bounded overlap while obsolete sessions finish closing.
        #[expect(clippy::expect_used, reason = "the fixed session limit is nonzero")]
        const MAX_SESSIONS: NonZero<usize> = NonZero::new(8).expect("eight is nonzero");
        Self(Arc::new(RuntimeInner {
            native: file_search::FileSearchOwner::new(MAX_SESSIONS),
            tasks: TaskTracker::new(),
            closing: Mutex::new(false),
        }))
    }

    fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) -> bool {
        let closing = self.0.closing.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if *closing {
            return false;
        }
        self.0.tasks.spawn(task);
        true
    }

    fn close(&self, session: file_search::ManagedFileSearchSession) {
        session.request_close();
        self.spawn(async move {
            if let Err(error) = session.close().await {
                tracing::warn!("file search session cleanup failed: {error}");
            }
        });
    }

    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        {
            let mut closing = self.0.closing.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            *closing = true;
            self.0.tasks.close();
        }
        self.0.native.request_shutdown();
        self.0.tasks.wait().await;
        // A successful return acknowledges the actual native joins, not merely
        // completion of the TUI callbacks. Native cleanup errors are retained.
        self.0.native.shutdown().await
    }
}

/// Arc identity prevents generation reuse, including across reconnect managers.
/// Query IDs are monotonically increasing within each manager and are recorded
/// before submission, so A -> B -> A cannot accept the first A's queued result.
#[derive(Clone, Debug)]
pub(crate) struct FileSearchRequest {
    manager: Arc<()>,
    session: Arc<()>,
    query_id: u64,
}

pub(crate) struct FileSearchManager {
    state: Arc<Mutex<SearchState>>,
    search_dir: PathBuf,
    app_tx: AppEventSender,
    runtime: FileSearchRuntime,
}

struct SearchState {
    latest_query: String,
    query_id: u64,
    manager_generation: Arc<()>,
    session_generation: Arc<()>,
    session: Option<file_search::ManagedFileSearchSession>,
    starting: bool,
}

impl SearchState {
    fn request(&self) -> FileSearchRequest {
        FileSearchRequest {
            manager: Arc::clone(&self.manager_generation),
            session: Arc::clone(&self.session_generation),
            query_id: self.query_id,
        }
    }

    fn same_session(&self, request: &FileSearchRequest) -> bool {
        Arc::ptr_eq(&self.manager_generation, &request.manager)
            && Arc::ptr_eq(&self.session_generation, &request.session)
    }

    fn accepts(&self, request: &FileSearchRequest, query: &str) -> bool {
        self.same_session(request)
            && self.query_id == request.query_id
            && !query.is_empty()
            && self.latest_query == query
    }

    fn invalidate(&mut self) -> Option<file_search::ManagedFileSearchSession> {
        self.latest_query.clear();
        self.session_generation = Arc::new(());
        self.starting = false;
        self.session.take()
    }
}

impl FileSearchManager {
    pub(crate) fn new(search_dir: PathBuf, tx: AppEventSender, runtime: FileSearchRuntime) -> Self {
        Self {
            state: Arc::new(Mutex::new(SearchState {
                latest_query: String::new(),
                query_id: 0,
                manager_generation: Arc::new(()),
                session_generation: Arc::new(()),
                session: None,
                starting: false,
            })),
            search_dir,
            app_tx: tx,
            runtime,
        }
    }

    pub(crate) fn restart(&mut self, search_dir: PathBuf, app_tx: AppEventSender) {
        // Reconnection rotates the delivery channel and manager generation,
        // while the outer owner still retains obsolete native cleanup.
        *self = Self::new(search_dir, app_tx, self.runtime.clone());
    }

    pub(crate) fn update_search_dir(&mut self, new_dir: PathBuf) {
        self.search_dir = new_dir;
        let session = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).invalidate();
        if let Some(session) = session {
            self.runtime.close(session);
        }
    }

    /// Recheck queued results at delivery, after any query/CWD/reconnect events.
    pub(crate) fn accepts(&self, request: &FileSearchRequest, query: &str) -> bool {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).accepts(request, query)
    }

    pub(crate) fn on_user_query(&self, query: String) {
        let (request, session) = {
            let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if query == state.latest_query
                && (query.is_empty() || state.session.is_some() || state.starting)
            {
                return;
            }
            let next_id = state.query_id.checked_add(1);
            if query.is_empty() || next_id.is_none() {
                let session = state.invalidate();
                drop(state);
                if let Some(session) = session {
                    self.runtime.close(session);
                }
                if next_id.is_none() {
                    tracing::warn!("file search query identities exhausted");
                }
                return;
            }
            if let Some(next_id) = next_id {
                state.query_id = next_id;
            }
            state.latest_query = query.clone();
            if state.starting {
                return;
            }
            if state.session.is_none() {
                state.starting = true;
                state.session_generation = Arc::new(());
            }
            (state.request(), state.session.clone())
        };
        let reporter = Arc::new(TuiSessionReporter {
            state: Arc::downgrade(&self.state),
            app_tx: self.app_tx.clone(),
            generation: request.clone(),
        });
        if let Some(session) = session {
            reporter.submit(&self.runtime, &session, &query, request.query_id);
            return;
        }
        let runtime = self.runtime.clone();
        let search_dir = self.search_dir.clone();
        let admission_reporter = Arc::clone(&reporter);
        let admitted = self.runtime.spawn(async move {
            let still_current = admission_reporter.state.upgrade().is_some_and(|state| {
                let state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.same_session(&admission_reporter.generation) && state.starting
            });
            if !still_current {
                return;
            }
            let result = runtime.0.native.create(
                vec![search_dir],
                file_search::FileSearchOptions {
                    compute_indices: true,
                    ..Default::default()
                },
                admission_reporter.clone(),
                /*cancel_flag*/ None,
            ).await;
            match result {
                Ok(session) => {
                    let query = admission_reporter.state.upgrade().and_then(|state| {
                        let mut state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                        if !state.same_session(&admission_reporter.generation) || !state.starting {
                            return None;
                        }
                        state.starting = false;
                        state.session = Some(session.clone());
                        Some((state.latest_query.clone(), state.query_id))
                    });
                    if let Some((query, query_id)) = query {
                        admission_reporter.submit(&runtime, &session, &query, query_id);
                    } else if let Err(error) = session.close().await {
                        tracing::warn!("obsolete file search cleanup failed: {error}");
                    }
                }
                Err(error) => admission_reporter.failed(&runtime, /*query_id*/ None, error),
            }
        });
        if !admitted {
            reporter.failed(&self.runtime, /*query_id*/ None, anyhow::anyhow!("file search is shutting down"));
        }
    }
}

impl Drop for FileSearchManager {
    fn drop(&mut self) {
        let session = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).invalidate();
        if let Some(session) = session {
            self.runtime.close(session);
        }
    }
}

struct TuiSessionReporter {
    state: Weak<Mutex<SearchState>>,
    app_tx: AppEventSender,
    generation: FileSearchRequest,
}

impl TuiSessionReporter {
    fn submit(&self, runtime: &FileSearchRuntime, session: &file_search::ManagedFileSearchSession, query: &str, query_id: u64) {
        if let Err(error) = session.update_query_tagged(query, query_id) {
            self.failed(runtime, Some(query_id), error);
        }
    }

    fn failed(&self, runtime: &FileSearchRuntime, query_id: Option<u64>, error: anyhow::Error) {
        let Some(state) = self.state.upgrade() else { return; };
        let (request, query, session) = {
            let mut state = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if !state.same_session(&self.generation)
                || query_id.is_some_and(|id| state.query_id != id)
                || state.latest_query.is_empty()
            {
                return;
            }
            state.starting = false;
            // Fence already queued native snapshots even when failure preserves
            // the query text for a later retry.
            state.session_generation = Arc::new(());
            (state.request(), state.latest_query.clone(), state.session.take())
        };
        tracing::warn!("file search request failed: {error}");
        if let Some(session) = session {
            runtime.close(session);
        }
        self.app_tx.send(AppEvent::FileSearchResult { request, query, matches: Vec::new() });
    }
}

impl file_search::SessionReporter for TuiSessionReporter {
    fn on_update(&self, snapshot: &file_search::FileSearchSnapshot) {
        let Some(state) = self.state.upgrade() else { return; };
        let request = FileSearchRequest { query_id: snapshot.query_id, ..self.generation.clone() };
        if !state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).accepts(&request, &snapshot.query) {
            return;
        }
        self.app_tx.send(AppEvent::FileSearchResult {
            request,
            query: snapshot.query.clone(),
            matches: snapshot.matches.clone(),
        });
    }

    fn on_complete(&self) {}
}

#[cfg(test)]
#[path = "file_search_tests.rs"]
mod tests;
