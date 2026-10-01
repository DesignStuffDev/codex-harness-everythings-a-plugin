//! Weak presentation callbacks and one bounded result/wake slot.
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SessionReporter;

use super::FileSearchDelivery;
use super::FileSearchRequest;
use super::FileSearchWake;
use super::runtime::RuntimeInner;
use super::runtime::WakeCredit;
use super::state::SearchState;
use super::state::lock;

pub(super) struct TuiSessionReporter {
    pub state: Weak<Mutex<SearchState>>,
    pub runtime: Weak<RuntimeInner>,
    pub generation: FileSearchRequest,
}
impl TuiSessionReporter {
    pub fn fail(&self, error: SearchError) {
        if let (Some(state), Some(runtime)) = (self.state.upgrade(), self.runtime.upgrade()) {
            fail(&state, &self.generation, &runtime, error);
        }
    }
}
impl SessionReporter for TuiSessionReporter {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let (Some(state), Some(runtime)) = (self.state.upgrade(), self.runtime.upgrade()) else {
            return;
        };
        let request = FileSearchRequest {
            query_id: snapshot.query_id,
            ..self.generation.clone()
        };
        let mut state = lock(&state);
        if !state.accepts(&request, &snapshot.query) {
            return;
        }
        state.delivery = Some(FileSearchDelivery::Matches {
            request,
            query: snapshot.query.clone(),
            matches: snapshot.matches.clone(),
        });
        wake(&mut state, &runtime);
        let retry = !state.wake_queued;
        drop(state);
        if retry {
            runtime.signal.send_replace(());
        }
    }
    fn on_complete(&self) {}
    fn on_error(&self, error: &SearchError) {
        self.fail(error.clone());
    }
}

pub(super) fn fail(
    state: &Arc<Mutex<SearchState>>,
    generation: &FileSearchRequest,
    runtime: &RuntimeInner,
    error: SearchError,
) {
    let session = {
        let mut state = lock(state);
        if !state.same_session(generation) {
            return;
        }
        // Actual stale operation failures remain in runtime close receipts.
        // Do not turn deliberate retired-session closure into a new failure.
        runtime.failed(error.clone());
        // A's failure retires the whole incarnation, including pending B. A
        // query-id equality check here would leave B stranded on a dead lease.
        state.session_generation = Arc::new(());
        state.preparing = false;
        let session = state.session.take();
        state.delivery = Some(FileSearchDelivery::Failed {
            request: state.request(),
            query: state.latest_query.clone(),
            error,
        });
        wake(&mut state, runtime);
        session
    };
    if let Some(session) = session {
        session.request_close();
    }
    runtime.signal.send_replace(());
}

fn wake(state: &mut SearchState, runtime: &RuntimeInner) {
    if state.wake_queued {
        return;
    }
    let mut gate = lock(&runtime.wakes);
    if !Arc::ptr_eq(&state.channel, &gate.channel) {
        return;
    }
    if gate.queued.upgrade().is_some() {
        // Same-channel restarts reuse the old wake to drain the new current
        // mailbox. The payload still passes its own manager/session/query fence.
        state.wake_queued = true;
        return;
    }
    let Ok(permit) = Arc::clone(&runtime.wake_capacity).try_acquire_owned() else {
        return;
    };
    let credit = Arc::new(WakeCredit { _permit: permit });
    state.wake_queued = state.tx.send_file_search_wake(FileSearchWake {
        channel: Arc::clone(&state.channel),
        credit: Arc::clone(&credit),
    });
    if state.wake_queued {
        gate.queued = Arc::downgrade(&credit);
    }
}

/// Voice filtering can end without another backend frame. One coordinator timer
/// retries an unqueued slot; a closed channel retires the lease, never claims UI
/// delivery, and does not spin/log repeatedly against a dead queue.
pub(super) fn retry_pending(runtime: &RuntimeInner) -> bool {
    let Some(manager) = lock(&runtime.state).active.upgrade() else {
        return false;
    };
    let mut state = lock(&manager);
    if state.delivery.is_none() || state.wake_queued {
        return false;
    }
    if state.tx.app_event_tx.is_closed() {
        let session = state.session.take();
        state.preparing = false;
        state.session_generation = Arc::new(());
        runtime.failed(SearchError::new(
            SearchErrorKind::TransportLost,
            "TUI file-search delivery channel is closed",
        ));
        drop(state);
        if let Some(session) = session {
            session.request_close();
        }
        return false;
    }
    wake(&mut state, runtime);
    !state.wake_queued
}
