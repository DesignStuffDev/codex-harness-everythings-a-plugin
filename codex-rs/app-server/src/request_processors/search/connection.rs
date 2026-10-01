//! Connection-owned search admission, retained startup and joined shutdown.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use codex_file_search_runtime::FileSearchScope;

use crate::fuzzy_file_search::close_result;
use crate::outgoing_message::ConnectionId;
use tokio::sync::watch;
use tokio_util::task::TaskTracker;

use crate::fuzzy_file_search::FuzzyFileSearchSession;
use crate::fuzzy_file_search::PendingSearchObserver;
use crate::fuzzy_file_search::PublisherFailures;
use crate::fuzzy_file_search::SearchCloseControl;

const MAX_SEARCHES_PER_CONNECTION: usize = 16;

pub(crate) struct SearchConnectionState {
    pub(super) scope: OnceLock<FileSearchScope>,
    pub(super) state: Mutex<State>,
    pub(super) startups: TaskTracker,
    pub(super) publishers: TaskTracker,
    pub(super) failures: PublisherFailures,
}

impl std::fmt::Debug for SearchConnectionState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SearchConnectionState")
            .finish_non_exhaustive()
    }
}

impl Default for SearchConnectionState {
    fn default() -> Self {
        Self {
            scope: OnceLock::new(),
            state: Mutex::new(State::default()),
            startups: TaskTracker::new(),
            publishers: TaskTracker::new(),
            failures: PublisherFailures::default(),
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    pub(super) closed: bool,
    pub(super) connection_id: Option<ConnectionId>,
    next_id: u64,
    pub(super) sessions: HashMap<String, SessionEntry>,
    pub(super) pending: HashMap<String, u64>,
    pub(super) one_shots: HashMap<u64, Arc<PendingSearchObserver>>,
    pub(super) tokens: HashMap<String, u64>,
}

impl State {
    pub(super) fn admit(&mut self) -> anyhow::Result<u64> {
        anyhow::ensure!(!self.closed, "search connection is closed");
        anyhow::ensure!(
            self.sessions.len()
                + self.one_shots.len()
                + self
                    .pending
                    .keys()
                    .filter(|id| !self.sessions.contains_key(*id))
                    .count()
                < MAX_SEARCHES_PER_CONNECTION,
            "search connection capacity is exhausted"
        );
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("search identity exhausted"))?;
        Ok(self.next_id)
    }
}

pub(super) enum SessionEntry {
    Starting {
        id: u64,
        observer: Arc<PendingSearchObserver>,
        complete: watch::Receiver<Option<Result<(), String>>>,
    },
    Ready {
        id: u64,
        session: FuzzyFileSearchSession,
    },
}

impl SessionEntry {
    pub(super) fn id(&self) -> u64 {
        match self {
            Self::Starting { id, .. } | Self::Ready { id, .. } => *id,
        }
    }

    pub(super) fn close_control(&self) -> SearchCloseControl {
        match self {
            Self::Starting { observer, .. } => SearchCloseControl::starting(observer.clone()),
            Self::Ready { session, .. } => session.close_control(),
        }
    }

    pub(super) fn request_close(&self) {
        self.close_control().request_close();
    }

    pub(super) async fn close(self) -> anyhow::Result<()> {
        self.request_close();
        match self {
            Self::Ready { session, .. } => session.close().await,
            Self::Starting { mut complete, .. } => loop {
                if let Some(result) = complete.borrow_and_update().clone() {
                    return result.map_err(anyhow::Error::msg);
                }
                complete
                    .changed()
                    .await
                    .map_err(|_| anyhow::anyhow!("search startup cleanup acknowledgement lost"))?;
            },
        }
    }
}

impl SearchConnectionState {
    /// Fence before waiting for connection RPCs; startup and callbacks share it.
    pub(crate) fn request_shutdown(&self) {
        let (one_shots, sessions) = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;
            (
                state.one_shots.values().cloned().collect::<Vec<_>>(),
                state
                    .sessions
                    .values()
                    .map(SessionEntry::close_control)
                    .collect::<Vec<_>>(),
            )
        };
        for cancellation in one_shots {
            cancellation.request_close();
        }
        for session in sessions {
            session.request_close();
        }
        if let Some(scope) = self.scope.get() {
            scope.request_shutdown();
        }
    }

    pub(super) async fn shutdown(&self) -> anyhow::Result<()> {
        self.request_shutdown();
        // Accepted startup tasks cannot register new publishers after this fence.
        self.startups.close();
        self.startups.wait().await;
        let sessions = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut state.sessions)
        };
        for (_, session) in sessions {
            if let Err(error) = session.close().await {
                self.failures.record(&format!("{error:#}"));
            }
        }
        if let Some(scope) = self.scope.get()
            && let Err(error) = close_result(scope.shutdown().await)
        {
            self.failures.record(&format!("{error:#}"));
        }
        self.publishers.close();
        self.publishers.wait().await;
        self.failures.result()
    }
}

impl Drop for SearchConnectionState {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

pub(super) struct OneShotGuard {
    pub(super) connection: Arc<SearchConnectionState>,
    pub(super) id: u64,
    pub(super) token: Option<String>,
    pub(super) cancellation: Arc<PendingSearchObserver>,
}

impl Drop for OneShotGuard {
    fn drop(&mut self) {
        self.cancellation.request_close();
        let mut state = self
            .connection
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.one_shots.remove(&self.id);
        if let Some(token) = &self.token
            && state.tokens.get(token) == Some(&self.id)
        {
            state.tokens.remove(token);
        }
    }
}

/// A missing start response releases its generation; accepted cleanup has owners.
pub(super) struct StartWaiter {
    pub(super) connection: Arc<SearchConnectionState>,
    pub(super) session_id: String,
    pub(super) id: u64,
    pub(super) armed: bool,
}

impl Drop for StartWaiter {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let entry = {
            let mut state = self
                .connection
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state
                .sessions
                .get(&self.session_id)
                .is_some_and(|entry| entry.id() == self.id)
            {
                state.sessions.remove(&self.session_id)
            } else {
                None
            }
        };
        if let Some(entry) = entry {
            entry.request_close();
            // Runtime and publisher owners retain cleanup on drop.
        }
    }
}

/// Dropping an RPC observation requests cancellation without releasing its
/// admitted slot; the retained task owns OneShotGuard until a cleanup receipt.
/// Unconfirmed remains quarantined by the runtime scope after this guard ends.
pub(super) struct OneShotWaiter(pub(super) Arc<PendingSearchObserver>);

impl Drop for OneShotWaiter {
    fn drop(&mut self) {
        self.0.request_close();
    }
}
