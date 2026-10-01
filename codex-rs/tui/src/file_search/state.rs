use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::Weak;

use super::FileSearchDelivery;
use super::FileSearchRequest;
use crate::app_event_sender::AppEventSender;
use codex_file_search_runtime::FileSearchSession;

pub(super) fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) struct SearchState {
    pub root: PathBuf,
    pub latest_query: String,
    pub query_id: u64,
    pub manager_generation: Arc<()>,
    pub session_generation: Arc<()>,
    pub session: Option<FileSearchSession>,
    pub preparing: bool,
    pub exhausted: bool,
    pub tx: AppEventSender,
    pub channel: Arc<()>,
    pub delivery: Option<FileSearchDelivery>,
    pub wake_queued: bool,
}
impl SearchState {
    pub fn new(root: PathBuf, tx: AppEventSender) -> Self {
        Self {
            root,
            tx,
            channel: Arc::new(()),
            latest_query: String::new(),
            query_id: 0,
            manager_generation: Arc::new(()),
            session_generation: Arc::new(()),
            session: None,
            preparing: false,
            exhausted: false,
            delivery: None,
            wake_queued: false,
        }
    }
    pub fn request(&self) -> FileSearchRequest {
        FileSearchRequest {
            manager: Arc::clone(&self.manager_generation),
            session: Arc::clone(&self.session_generation),
            query_id: self.query_id,
        }
    }
    pub fn same_session(&self, request: &FileSearchRequest) -> bool {
        Arc::ptr_eq(&self.manager_generation, &request.manager)
            && Arc::ptr_eq(&self.session_generation, &request.session)
    }
    pub fn accepts(&self, request: &FileSearchRequest, query: &str) -> bool {
        self.same_session(request)
            && self.query_id == request.query_id
            && !query.is_empty()
            && self.latest_query == query
    }
    pub fn advance(&mut self) -> bool {
        if let Some(id) = self.query_id.checked_add(1) {
            self.query_id = id;
            true
        } else {
            self.exhausted = true;
            false
        }
    }
    pub fn invalidate(&mut self) -> Option<FileSearchSession> {
        self.advance();
        self.latest_query.clear();
        self.session_generation = Arc::new(());
        self.preparing = false;
        self.delivery = None;
        self.session.take()
    }
    pub fn intent(&self, state: &Arc<Mutex<Self>>) -> Intent {
        Intent {
            state: Arc::downgrade(state),
            request: self.request(),
            root: self.root.clone(),
            query: self.latest_query.clone(),
        }
    }
}

#[derive(Clone)]
pub(super) struct Intent {
    pub state: Weak<Mutex<SearchState>>,
    pub request: FileSearchRequest,
    pub root: PathBuf,
    pub query: String,
}
impl Intent {
    pub fn current(&self) -> bool {
        self.state
            .upgrade()
            .is_some_and(|state| lock(&state).accepts(&self.request, &self.query))
    }
}
