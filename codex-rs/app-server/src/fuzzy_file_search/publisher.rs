//! One bounded, ordered publisher; search callbacks never spawn or block on I/O.

use std::num::NonZeroUsize;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use codex_app_server_protocol::FuzzyFileSearchSessionCompletedNotification;
use codex_app_server_protocol::FuzzyFileSearchSessionError;
use codex_app_server_protocol::FuzzyFileSearchSessionErrorKind;
use codex_app_server_protocol::FuzzyFileSearchSessionFailedNotification;
use codex_app_server_protocol::FuzzyFileSearchSessionUpdatedNotification;
use codex_app_server_protocol::ServerNotification;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SessionReporter;
use futures::FutureExt;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::OutgoingMessageSender;

#[derive(Clone, Default)]
pub(crate) struct PublisherFailures(Arc<Mutex<Vec<String>>>);

impl PublisherFailures {
    pub(crate) fn record(&self, error: &str) {
        let mut failures = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failures.len() < 32 {
            failures.push(error.chars().take(1024).collect());
        }
    }

    pub(crate) fn result(&self) -> anyhow::Result<()> {
        let failures = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if failures.is_empty() {
            Ok(())
        } else {
            anyhow::bail!("search publisher cleanup failed: {}", failures.join("; "))
        }
    }
}

#[derive(Default)]
struct State {
    closed: bool,
    query_id: u64,
    query: String,
    revision: u64,
    snapshot: Option<FileSearchSnapshot>,
    complete: bool,
    error: Option<SearchError>,
    start: Option<Arc<dyn codex_file_search_api::SearchStartControl>>,
}

pub(crate) struct SearchObserver {
    state: Mutex<State>,
    changed: watch::Sender<u64>,
    stop: CancellationToken,
    cancellation: Arc<AtomicBool>,
    shutdown_requested: CancellationToken,
}

impl SearchObserver {
    pub(crate) fn new(
        cancellation: Arc<AtomicBool>,
        shutdown_requested: CancellationToken,
    ) -> Self {
        Self {
            state: Mutex::new(State::default()),
            changed: watch::channel(0).0,
            stop: CancellationToken::new(),
            cancellation,
            shutdown_requested,
        }
    }

    pub(crate) fn cancellation_requested(&self) -> bool {
        self.cancellation.load(Ordering::Acquire) || self.shutdown_requested.is_cancelled()
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    pub(crate) fn set_query(
        &self,
        query: String,
        max_query_bytes: NonZeroUsize,
    ) -> anyhow::Result<u64> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(error) = &state.error {
            return Err(error.clone().into());
        }
        anyhow::ensure!(
            !state.closed && !self.cancellation_requested(),
            "file search session is closed"
        );
        if query.len() > max_query_bytes.get() {
            return Err(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "file search query exceeds the negotiated UTF-8 byte limit",
            )
            .into());
        }
        state.query_id = state
            .query_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("file search query identity exhausted"))?;
        state.query = query;
        state.snapshot = None;
        state.complete = false;
        state.revision = state.revision.saturating_add(1);
        self.changed.send_replace(state.revision);
        Ok(state.query_id)
    }

    pub(crate) fn error(&self) -> Option<SearchError> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .error
            .clone()
    }

    pub(crate) fn is_current(&self, query_id: u64) -> bool {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        !state.closed
            && !self.cancellation_requested()
            && state.error.is_none()
            && state.query_id == query_id
    }

    pub(crate) fn request_close(&self) {
        let start = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;
            self.cancellation.store(true, Ordering::Release);
            self.stop.cancel();
            state.start.clone()
        };
        if let Some(start) = start {
            start.request_cancel();
        }
    }

    pub(super) fn bind_start(
        &self,
        control: Arc<dyn codex_file_search_api::SearchStartControl>,
    ) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.start = Some(control);
        state.closed || self.cancellation_requested()
    }

    pub(super) fn unbind_start(
        &self,
        control: &Arc<dyn codex_file_search_api::SearchStartControl>,
    ) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state
            .start
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, control))
        {
            state.start = None;
        }
    }

    pub(super) async fn cancelled(&self) {
        tokio::select! {
            _ = self.stop.cancelled() => {},
            _ = self.shutdown_requested.cancelled() => {},
        }
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .complete
    }

    pub(crate) fn files(&self) -> Vec<codex_app_server_protocol::FuzzyFileSearchResult> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot
            .as_ref()
            .map(super::collect_files)
            .unwrap_or_default()
    }
}

impl SessionReporter for SearchObserver {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed
            || self.cancellation_requested()
            || state.error.is_some()
            || snapshot.query_id != state.query_id
            || snapshot.query != state.query
        {
            return;
        }
        state.snapshot = Some(snapshot.clone());
        state.revision = state.revision.saturating_add(1);
        self.changed.send_replace(state.revision);
    }

    fn on_complete(&self) {
        // Untagged completion cannot be safely associated with an accepted query.
    }

    fn on_complete_tagged(&self, query_id: u64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed
            || self.cancellation_requested()
            || state.error.is_some()
            || state.query_id != query_id
            || state.complete
        {
            return;
        }
        state.complete = true;
        state.revision = state.revision.saturating_add(1);
        self.changed.send_replace(state.revision);
    }

    fn on_error(&self, error: &SearchError) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Cleanup fencing must not erase a failure delivered by accepted work.
        // SearchError already enforces a bounded diagnostic at construction.
        if state.error.is_none() {
            state.error = Some(error.clone());
            state.snapshot = None;
            state.complete = false;
            state.revision = state.revision.saturating_add(1);
            self.changed.send_replace(state.revision);
        }
    }
}

pub(crate) struct SearchPublisher {
    observer: Arc<SearchObserver>,
    completion: watch::Receiver<Option<Result<(), String>>>,
    joined: TaskTracker,
}

impl SearchPublisher {
    pub(crate) fn start(
        connection_id: ConnectionId,
        session_id: String,
        outgoing: Arc<OutgoingMessageSender>,
        tasks: &TaskTracker,
        failures: PublisherFailures,
        shutdown_requested: CancellationToken,
    ) -> Self {
        let observer = Arc::new(SearchObserver::new(
            Arc::new(AtomicBool::new(false)),
            shutdown_requested,
        ));
        let (done, completion) = watch::channel(None);
        let owned = Arc::clone(&observer);
        let joined = TaskTracker::new();
        tasks.spawn(joined.track_future(async move {
            let result = AssertUnwindSafe(publish(&owned, connection_id, session_id, &outgoing))
                .catch_unwind()
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("file search publisher panicked")));
            if let Err(error) = &result {
                failures.record(&format!("{error:#}"));
                owned.request_close();
            }
            done.send_replace(Some(result.map_err(|error| format!("{error:#}"))));
        }));
        joined.close();
        Self {
            observer,
            completion,
            joined,
        }
    }

    pub(crate) fn observer(&self) -> &Arc<SearchObserver> {
        &self.observer
    }

    pub(crate) fn request_close(&self) {
        self.observer.request_close();
    }

    pub(crate) async fn close(self) -> anyhow::Result<()> {
        self.request_close();
        self.joined.wait().await;
        if let Some(error) = self.observer.error() {
            return Err(error.into());
        }
        let result = self.completion.borrow().clone();
        result
            .ok_or_else(|| anyhow::anyhow!("file search publisher completion lost"))?
            .map_err(anyhow::Error::msg)
    }
}

impl Drop for SearchPublisher {
    fn drop(&mut self) {
        self.request_close();
    }
}

enum PendingNotification {
    Updated,
    Completed,
    Failed,
}

async fn publish(
    observer: &SearchObserver,
    connection_id: ConnectionId,
    session_id: String,
    outgoing: &OutgoingMessageSender,
) -> anyhow::Result<()> {
    let mut changes = observer.subscribe();
    let mut sent_snapshot = None;
    let mut completed_query = None;
    loop {
        let pending = {
            let state = observer
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed || observer.shutdown_requested.is_cancelled() {
                return state
                    .error
                    .clone()
                    .map_or(Ok(()), |error| Err(error.into()));
            }
            let snapshot_revision = state
                .snapshot
                .as_ref()
                .map(|_| (state.query_id, state.revision));
            if state.error.is_some() {
                Some((state.query_id, state.revision, PendingNotification::Failed))
            } else if snapshot_revision.is_some()
                && sent_snapshot != snapshot_revision
                && completed_query != Some(state.query_id)
            {
                Some((state.query_id, state.revision, PendingNotification::Updated))
            } else if state.complete && completed_query != Some(state.query_id) {
                Some((
                    state.query_id,
                    state.revision,
                    PendingNotification::Completed,
                ))
            } else {
                None
            }
        };
        let Some((query_id, revision, notification)) = pending else {
            tokio::select! {
                biased;
                _ = observer.stop.cancelled() => continue,
                _ = observer.shutdown_requested.cancelled() => continue,
                changed = changes.changed() => { changed?; }
            }
            continue;
        };
        let permit = tokio::select! {
            biased;
            _ = observer.stop.cancelled() => continue,
                _ = observer.shutdown_requested.cancelled() => continue,
            changed = changes.changed() => { changed?; continue; }
            permit = outgoing.reserve_server_notification_to_connection(connection_id) => permit?,
        };
        // Admission commits under the same short lock as query/close fencing.
        // Search callbacks never hold this lock across an await or client I/O.
        let state = observer
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed
            || observer.shutdown_requested.is_cancelled()
            || state.query_id != query_id
            || state.revision != revision
        {
            continue;
        }
        match notification {
            PendingNotification::Failed => {
                let Some(error) = &state.error else {
                    continue;
                };
                // Failure is a terminal operation event, not a cleanup receipt.
                // Returning immediately after admission makes its delivery once-only.
                let kind = match error.kind() {
                    SearchErrorKind::InvalidInput => FuzzyFileSearchSessionErrorKind::InvalidInput,
                    SearchErrorKind::UnsupportedVersion => {
                        FuzzyFileSearchSessionErrorKind::UnsupportedVersion
                    }
                    SearchErrorKind::UnsupportedOption => {
                        FuzzyFileSearchSessionErrorKind::UnsupportedOption
                    }
                    SearchErrorKind::UnknownLease => FuzzyFileSearchSessionErrorKind::UnknownLease,
                    SearchErrorKind::ClosedLease => FuzzyFileSearchSessionErrorKind::ClosedLease,
                    SearchErrorKind::StaleEpoch => FuzzyFileSearchSessionErrorKind::StaleEpoch,
                    SearchErrorKind::ResourceExhausted => {
                        FuzzyFileSearchSessionErrorKind::ResourceExhausted
                    }
                    SearchErrorKind::SearchFailed => FuzzyFileSearchSessionErrorKind::SearchFailed,
                    SearchErrorKind::TransportLost => {
                        FuzzyFileSearchSessionErrorKind::TransportLost
                    }
                    SearchErrorKind::ForcedShutdown => {
                        FuzzyFileSearchSessionErrorKind::ForcedShutdown
                    }
                };
                permit.send(ServerNotification::FuzzyFileSearchSessionFailed(
                    FuzzyFileSearchSessionFailedNotification {
                        session_id: session_id.clone(),
                        query: state.query.clone(),
                        error: FuzzyFileSearchSessionError {
                            kind,
                            message: error.message().to_owned(),
                        },
                    },
                ));
                return Err(error.clone().into());
            }
            PendingNotification::Completed => {
                permit.send(ServerNotification::FuzzyFileSearchSessionCompleted(
                    FuzzyFileSearchSessionCompletedNotification {
                        session_id: session_id.clone(),
                    },
                ));
                completed_query = Some(query_id);
            }
            PendingNotification::Updated => {
                if let Some(snapshot) = &state.snapshot {
                    permit.send(ServerNotification::FuzzyFileSearchSessionUpdated(
                        FuzzyFileSearchSessionUpdatedNotification {
                            session_id: session_id.clone(),
                            query: state.query.clone(),
                            files: super::collect_files(snapshot),
                        },
                    ));
                    sent_snapshot = Some((query_id, revision));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "publisher_tests.rs"]
mod tests;
