//! One bounded, ordered publisher; native callbacks never spawn or block on I/O.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use codex_app_server_protocol::FuzzyFileSearchSessionCompletedNotification;
use codex_app_server_protocol::FuzzyFileSearchSessionUpdatedNotification;
use codex_app_server_protocol::ServerNotification;
use codex_file_search::FileSearchSnapshot;
use codex_file_search::SessionReporter;
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
}

pub(crate) struct SearchObserver {
    state: Mutex<State>,
    changed: watch::Sender<u64>,
    stop: CancellationToken,
    cancellation: Arc<AtomicBool>,
}

impl SearchObserver {
    pub(crate) fn new(cancellation: Arc<AtomicBool>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            changed: watch::channel(0).0,
            stop: CancellationToken::new(),
            cancellation,
        }
    }

    pub(crate) fn cancellation(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancellation)
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    pub(crate) fn set_query(&self, query: String) -> anyhow::Result<u64> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        anyhow::ensure!(!state.closed, "file search session is closed");
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

    pub(crate) fn request_close(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.closed = true;
        self.cancellation.store(true, Ordering::Release);
        self.stop.cancel();
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
        if state.closed || snapshot.query_id != state.query_id || snapshot.query != state.query {
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
        if state.closed || state.query_id != query_id || state.complete {
            return;
        }
        state.complete = true;
        state.revision = state.revision.saturating_add(1);
        self.changed.send_replace(state.revision);
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
    ) -> Self {
        let observer = Arc::new(SearchObserver::new(Arc::new(AtomicBool::new(false))));
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
            if state.closed {
                return Ok(());
            }
            let snapshot_revision = state
                .snapshot
                .as_ref()
                .map(|_| (state.query_id, state.revision));
            if snapshot_revision.is_some()
                && sent_snapshot != snapshot_revision
                && completed_query != Some(state.query_id)
            {
                Some((state.query_id, state.revision, false))
            } else if state.complete && completed_query != Some(state.query_id) {
                Some((state.query_id, state.revision, true))
            } else {
                None
            }
        };
        let Some((query_id, revision, completion)) = pending else {
            tokio::select! {
                biased;
                _ = observer.stop.cancelled() => return Ok(()),
                changed = changes.changed() => { changed?; }
            }
            continue;
        };
        let permit = tokio::select! {
            biased;
            _ = observer.stop.cancelled() => return Ok(()),
            changed = changes.changed() => { changed?; continue; }
            permit = outgoing.reserve_server_notification_to_connection(connection_id) => permit?,
        };
        // Admission commits under the same short lock as query/close fencing.
        // Native callbacks never hold this lock across an await or client I/O.
        let state = observer
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed || state.query_id != query_id || state.revision != revision {
            continue;
        }
        if completion {
            permit.send(ServerNotification::FuzzyFileSearchSessionCompleted(
                FuzzyFileSearchSessionCompletedNotification {
                    session_id: session_id.clone(),
                },
            ));
            completed_query = Some(query_id);
        } else if let Some(snapshot) = &state.snapshot {
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

#[cfg(test)]
#[path = "publisher_tests.rs"]
mod tests;
