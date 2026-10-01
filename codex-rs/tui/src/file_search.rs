//! Selected-provider searches for `@` tokens, with bounded intent/result slots.
//! A retained coordinator serializes opens, acknowledgements and cleanup; the
//! visible manager never owns an async waiter or keeps a retired callback alive.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use codex_file_search_api::FileMatch;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;

use crate::app_event_sender::AppEventSender;

mod coordinator;
mod reporter;
mod runtime;
mod state;
pub(crate) use runtime::FileSearchRuntime;
pub(crate) use runtime::FileSearchSource;
use state::SearchState;
use state::lock;

/// Opaque identities stay valid only for the owning manager/session incarnation.
#[derive(Clone, Debug)]
pub(crate) struct FileSearchRequest {
    manager: Arc<()>,
    session: Arc<()>,
    query_id: u64,
}

/// A wake contains no result payload and does not own manager/provider state.
#[derive(Clone, Debug)]
pub(crate) struct FileSearchWake {
    channel: Arc<()>,
    credit: Arc<runtime::WakeCredit>,
}

#[derive(Debug)]
pub(crate) enum FileSearchDelivery {
    Matches {
        request: FileSearchRequest,
        query: String,
        matches: Vec<FileMatch>,
    },
    Failed {
        request: FileSearchRequest,
        query: String,
        error: SearchError,
    },
}
impl FileSearchDelivery {
    pub(crate) fn error_notice(error: &SearchError) -> String {
        format!("File search failed: {error}")
    }
    fn identity(&self) -> (&FileSearchRequest, &str) {
        match self {
            Self::Matches { request, query, .. } | Self::Failed { request, query, .. } => {
                (request, query)
            }
        }
    }
}

pub(crate) struct FileSearchManager {
    state: Arc<Mutex<SearchState>>,
    runtime: FileSearchRuntime,
}
impl FileSearchManager {
    pub(crate) fn new(root: PathBuf, tx: AppEventSender, runtime: FileSearchRuntime) -> Self {
        let state = Arc::new(Mutex::new(SearchState::new(root, tx)));
        runtime.register(&state);
        Self { state, runtime }
    }
    pub(crate) fn restart(&mut self, root: PathBuf, tx: AppEventSender) {
        *self = Self::new(root, tx, self.runtime.clone());
    }
    pub(crate) fn update_search_dir(&mut self, root: PathBuf) {
        let (intent, session) = {
            let mut state = lock(&self.state);
            state.root = root;
            let session = state.invalidate();
            (state.intent(&self.state), session)
        };
        if let Some(session) = session {
            session.request_close();
        }
        self.runtime.enqueue(intent);
    }
    #[cfg(test)]
    pub(crate) fn accepts(&self, request: &FileSearchRequest, query: &str) -> bool {
        lock(&self.state).accepts(request, query)
    }
    /// Final UI fence for the current bounded result or failure payload.
    pub(crate) fn accepts_delivery(&self, delivery: &FileSearchDelivery) -> bool {
        let state = lock(&self.state);
        let (request, query) = delivery.identity();
        state.same_session(request)
            && state.query_id == request.query_id
            && state.latest_query == query
            && (!query.is_empty() || matches!(delivery, FileSearchDelivery::Failed { .. }))
    }
    pub(crate) fn take_delivery(&self, wake: &FileSearchWake) -> Option<FileSearchDelivery> {
        let mut state = lock(&self.state);
        if !Arc::ptr_eq(&state.channel, &wake.channel) {
            return None;
        }
        let inner = self.runtime.inner();
        let mut queued = lock(&inner.wakes);
        if queued
            .queued
            .upgrade()
            .is_some_and(|credit| Arc::ptr_eq(&credit, &wake.credit))
        {
            queued.queued = std::sync::Weak::new();
        }
        state.wake_queued = false;
        state.delivery.take()
    }
    pub(crate) fn on_user_query(&self, query: &str) -> Result<(), SearchError> {
        let mut failure = None;
        let (intent, session) = {
            let mut state = lock(&self.state);
            if state.exhausted {
                return Err(SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "file search query identity exhausted",
                ));
            }
            if query == state.latest_query
                && (query.is_empty() || state.session.is_some() || state.preparing)
            {
                return Ok(());
            }
            if query.len() > self.runtime.max_query_bytes() {
                let session = state.invalidate();
                failure = Some(SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "file search query exceeds the selected provider UTF-8 byte limit",
                ));
                (state.intent(&self.state), session)
            } else if !state.advance() {
                let session = state.invalidate();
                failure = Some(SearchError::new(
                    SearchErrorKind::ResourceExhausted,
                    "file search query identity exhausted",
                ));
                (state.intent(&self.state), session)
            } else if query.is_empty() {
                let session = state.invalidate();
                (state.intent(&self.state), session)
            } else {
                if state.session.is_none() && !state.preparing {
                    state.session_generation = Arc::new(());
                    state.preparing = true;
                }
                state.latest_query = query.to_owned();
                state.delivery = None;
                (state.intent(&self.state), None)
            }
        };
        if let Some(session) = session {
            session.request_close();
        }
        self.runtime.enqueue(intent);
        if let Some(error) = failure {
            self.runtime.inner().failed(error.clone());
            return Err(error);
        }
        Ok(())
    }
}
impl Drop for FileSearchManager {
    fn drop(&mut self) {
        let session = lock(&self.state).invalidate();
        if let Some(session) = session {
            session.request_close();
        }
        self.runtime.wake();
    }
}

#[cfg(test)]
#[path = "file_search_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "file_search/coordinator_tests.rs"]
mod coordinator_tests;
#[cfg(test)]
#[path = "file_search/test_support.rs"]
mod test_support;
