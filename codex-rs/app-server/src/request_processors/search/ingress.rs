//! Bounded reader-order cancellation intent; actual RPC execution stays FIFO.
use super::*;
use crate::fuzzy_file_search::SearchStartCause;
use codex_app_server_protocol::ClientRequest;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use std::sync::Weak;
use std::sync::atomic::Ordering;

pub(crate) enum SearchAdmission {
    Other,
    Start(QueuedSearchStart),
    Stop(SearchStopIntent),
}

pub(super) struct SearchStartIntent {
    session_id: String,
    cancelled: AtomicBool,
    observer: Mutex<Option<Arc<PendingSearchObserver>>>,
    finished: AtomicBool,
    cause: Arc<SearchStartCause>,
    cleanup: watch::Sender<Option<StartReceipt>>,
}
#[derive(Clone)]
struct StartReceipt {
    operation: Result<(), String>,
    cleanup: Result<(), String>,
}
impl StartReceipt {
    fn result(self) -> Result<(), String> {
        match (self.operation, self.cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => {
                Err(format!("{error}; search cleanup also failed: {cleanup}"))
            }
        }
    }
}

impl SearchStartIntent {
    fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        let observer = self
            .observer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(observer) = observer {
            observer.request_close();
        }
    }
    fn finish(&self, cleanup: Result<(), String>) {
        let operation = self.cause.operation();
        self.cleanup.send_if_modified(|retained| {
            if retained.is_some() {
                return false;
            }
            *retained = Some(StartReceipt { operation, cleanup });
            true
        });
        self.finished.store(true, Ordering::Release);
        let observer = self
            .observer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        drop(observer);
    }
    async fn wait(&self) -> Result<(), String> {
        let mut receiver = self.cleanup.subscribe();
        loop {
            if let Some(outcome) = receiver.borrow_and_update().clone() {
                return outcome.result();
            }
            if receiver.changed().await.is_err() {
                return Err("search queued-start cleanup receipt lost; cleanup unconfirmed".into());
            }
        }
    }
}

enum Ownership {
    Queued,
    Accepted,
    Complete,
}

/// Constructed before queuing/spawn. Its Drop never invents accepted cleanup.
/// Passive Stop observers own the intent Arc, not this completion authority.
pub(crate) struct QueuedSearchStart {
    intent: Arc<SearchStartIntent>,
    ownership: Ownership,
}
impl QueuedSearchStart {
    pub(super) fn record_failure(&self, error: &anyhow::Error) {
        self.intent.cause.record(error);
    }
    pub(super) fn cause(&self) -> Arc<SearchStartCause> {
        self.intent.cause.clone()
    }
    pub(super) fn accepted(&mut self) {
        self.ownership = Ownership::Accepted;
    }
    pub(super) fn restored_without_work(&mut self) {
        self.ownership = Ownership::Queued;
    }
    pub(super) fn bind(&self, observer: Arc<PendingSearchObserver>) {
        let (cancelled, previous) = {
            let mut current = self
                .intent
                .observer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let previous = current.replace(observer.clone());
            (self.intent.cancelled.load(Ordering::Acquire), previous)
        };
        drop(previous);
        if cancelled {
            observer.request_close();
        }
    }
    pub(super) fn complete(&mut self, cleanup: Result<(), String>) {
        self.intent.finish(cleanup);
        self.ownership = Ownership::Complete;
    }
}
impl Drop for QueuedSearchStart {
    fn drop(&mut self) {
        match self.ownership {
            Ownership::Queued => self.intent.finish(Ok(())),
            Ownership::Accepted => {
                self.intent.request_cancel();
                self.intent.finish(Err(
                    "search admitted startup owner lost; cleanup unconfirmed".into(),
                ));
            }
            Ownership::Complete => {}
        }
    }
}

pub(crate) struct SearchStopIntent(Vec<Arc<SearchStartIntent>>);
impl SearchRequestProcessor {
    /// Called through the authenticated connection gate in reader receive order.
    /// Does not construct/replace a backend session or bypass its exclusive queue.
    pub(crate) fn admit_search_request(
        &self,
        connection_id: ConnectionId,
        connection: &Arc<SearchConnectionState>,
        request: &ClientRequest,
    ) -> Result<SearchAdmission, JSONRPCErrorError> {
        match request {
            ClientRequest::FuzzyFileSearchSessionStart { params, .. } => {
                validate_id(&params.session_id, "sessionId")?;
                if params.session_id.is_empty() {
                    return Err(invalid_request("sessionId must not be empty"));
                }
                if params.roots.is_empty() {
                    return Err(invalid_request("at least one search directory is required"));
                }
                self.register(connection_id, connection)?;
                // Any last external reference from weak pruning drops only
                // after releasing the connection fence, including refusal.
                let mut retained = Vec::new();
                let result = {
                    let mut state = connection
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    state.incoming_starts.retain(|entry| {
                        let Some(intent) = entry.upgrade() else {
                            return false;
                        };
                        let keep = !intent.finished.load(Ordering::Acquire);
                        retained.push(intent);
                        keep
                    });
                    if state.closed {
                        Err(invalid_request("search connection is closed"))
                    } else if state.incoming_starts.len() >= connection::MAX_SEARCHES_PER_CONNECTION
                    {
                        Err(search_error(
                            SearchError::new(
                                SearchErrorKind::ResourceExhausted,
                                "search connection queued-start capacity is exhausted",
                            )
                            .into(),
                        ))
                    } else {
                        let intent = Arc::new(SearchStartIntent {
                            session_id: params.session_id.clone(),
                            cancelled: AtomicBool::new(false),
                            observer: Mutex::new(None),
                            cause: Arc::default(),
                            finished: AtomicBool::new(false),
                            cleanup: watch::channel(None).0,
                        });
                        state.incoming_starts.push(Arc::downgrade(&intent));
                        Ok(SearchAdmission::Start(QueuedSearchStart {
                            intent,
                            ownership: Ownership::Queued,
                        }))
                    }
                };
                drop(retained);
                result
            }
            ClientRequest::FuzzyFileSearchSessionStop { params, .. } => {
                validate_id(&params.session_id, "sessionId")?;
                self.register(connection_id, connection)?;
                let (starts, current) = {
                    let state = connection
                        .state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if state.closed {
                        return Err(invalid_request("search connection is closed"));
                    }
                    (
                        state
                            .incoming_starts
                            .iter()
                            .filter_map(Weak::upgrade)
                            .collect::<Vec<_>>(),
                        state
                            .sessions
                            .get(&params.session_id)
                            .map(SessionEntry::close_control),
                    )
                };
                let starts = starts
                    .into_iter()
                    .filter(|intent| intent.session_id == params.session_id)
                    .collect::<Vec<_>>();
                for intent in &starts {
                    intent.request_cancel();
                }
                if let Some(current) = current {
                    current.request_close();
                }
                Ok(SearchAdmission::Stop(SearchStopIntent(starts)))
            }
            _ => Ok(SearchAdmission::Other),
        }
    }

    pub(crate) async fn start_search_admitted(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionStartParams,
        admission: SearchAdmission,
    ) -> Result<FuzzyFileSearchSessionStartResponse, JSONRPCErrorError> {
        let SearchAdmission::Start(ticket) = admission else {
            return Err(internal_error("file search start admission is missing"));
        };
        self.start_search(connection_id, connection, params, Some(ticket))
            .await
    }

    pub(crate) async fn stop_search_admitted(
        &self,
        connection_id: ConnectionId,
        connection: Arc<SearchConnectionState>,
        params: FuzzyFileSearchSessionStopParams,
        admission: SearchAdmission,
    ) -> Result<FuzzyFileSearchSessionStopResponse, JSONRPCErrorError> {
        let SearchAdmission::Stop(SearchStopIntent(starts)) = admission else {
            return Err(internal_error("file search stop admission is missing"));
        };
        let mut failure = None;
        for start in starts {
            if let Err(error) = start.wait().await {
                failure.get_or_insert_with(|| search_error(anyhow::Error::msg(error)));
            }
        }
        // Entry removal by failed startup cannot erase its receipt. Still run
        // ordinary current-session cleanup even when an earlier cause is known.
        let closed = self
            .fuzzy_file_search_session_stop(connection_id, connection, params)
            .await;
        match failure {
            Some(error) => Err(error),
            None => closed,
        }
    }
}
