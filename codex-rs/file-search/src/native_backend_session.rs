//! Public lease ownership is separate from retained native tasks and callbacks.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::QueryAccepted;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchFrame;
use codex_file_search_api::SearchFuture;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use tokio::sync::watch;

use crate::ManagedFileSearchSession;
use crate::SessionReporter;
use crate::native_backend::lock;
use crate::native_backend::observe_close;
use crate::native_backend_policy::NativeBackendLimits;
use crate::native_backend_policy::closed;
use crate::native_backend_policy::exhausted;
use crate::native_backend_policy::failed;

pub(super) struct NativeLease {
    pub(super) inner: Arc<NativeSession>,
}
impl Drop for NativeLease {
    fn drop(&mut self) {
        self.inner.request_close();
    }
}

impl SearchBackendSession for NativeLease {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        Box::pin(async move {
            {
                let mut state = lock(&self.inner.state);
                if let Some(error) = &state.failure {
                    return Err(error.clone());
                }
                if !state.closing {
                    query
                        .validate_after(state.frame.query_id, self.inner.limits.max_query_bytes)?;
                    if state.frame.revision == u64::MAX {
                        state.fail(exhausted("native file-search frame revisions exhausted"));
                        drop(state);
                        self.inner.changed.send_replace(());
                        return Err(exhausted("native file-search frame revisions exhausted"));
                    }
                    let text = crate::native_output::exact_string(&query.text)?;
                    let id = query.id;
                    drop(query);
                    // Enqueue and identity transition share this fence. Native
                    // callbacks cannot publish an obsolete identity between them.
                    if let Some(native) = &state.native
                        && native.update_query_tagged(&text, id.get()).is_ok()
                    {
                        state.frame = SearchFrame {
                            revision: state.frame.revision + 1,
                            query_id: id.get(),
                            query: text,
                            snapshot: None,
                            phase: SearchPhase::Running,
                        };
                        drop(state);
                        self.inner.changed.send_replace(());
                        return Ok(QueryAccepted { id });
                    }
                } else {
                    drop(query);
                }
            }
            // Native failure closes admission before its terminal callback can
            // acquire the bridge lock. Join outside that lock before inventing a
            // ClosedLease error, or it can hide the actual resource/worker error.
            // The existing lifecycle actor retains cleanup if this waiter drops.
            self.inner.request_close();
            let outcome = observe_close(self.inner.closed.subscribe()).await;
            Err(match outcome.operation {
                Err(error) => error,
                Ok(()) => match outcome.cleanup {
                    CloseCleanup::Joined => closed(),
                    CloseCleanup::Unconfirmed(error) => error,
                },
            })
        })
    }

    fn next_snapshot(&self, after_revision: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(crate::native_backend_poll::next(
            self.inner.clone(),
            after_revision,
            wait,
        ))
    }

    fn request_close(&self) {
        self.inner.request_close();
    }
    fn close(&self) -> SearchCloseFuture<'_> {
        self.request_close();
        Box::pin(observe_close(self.inner.closed.subscribe()))
    }
}

#[cfg(test)]
type BeforeError = Arc<dyn Fn(&SearchError) + Send + Sync>;

pub(super) struct NativeSession {
    pub(super) id: u64,
    pub(super) allocation: [usize; 3],
    pub(super) limits: NativeBackendLimits,
    pub(super) state: Mutex<SessionState>,
    pub(super) changed: watch::Sender<()>,
    pub(super) closed: watch::Sender<Option<SearchCloseOutcome>>,
    #[cfg(test)]
    pub(super) before_error: Mutex<Option<BeforeError>>,
}

pub(super) struct SessionState {
    pub(super) frame: SearchFrame,
    pub(super) failure: Option<SearchError>,
    pub(super) closing: bool,
    pub(super) poll: Option<crate::native_backend_poll::PendingPoll>,
    native: Option<ManagedFileSearchSession>,
    outcome: Option<SearchCloseOutcome>,
}

impl NativeSession {
    pub(super) fn new(id: u64, allocation: [usize; 3], limits: NativeBackendLimits) -> Self {
        let (changed, _) = watch::channel(());
        let (closed, _) = watch::channel(None);
        Self {
            id,
            allocation,
            limits,
            changed,
            closed,
            #[cfg(test)]
            before_error: Mutex::new(None),
            state: Mutex::new(SessionState {
                frame: SearchFrame {
                    revision: 0,
                    query_id: 0,
                    query: String::new(),
                    snapshot: None,
                    phase: SearchPhase::Running,
                },
                failure: None,
                closing: false,
                poll: None,
                native: None,
                outcome: None,
            }),
        }
    }

    pub(super) fn started(&self, native: ManagedFileSearchSession) {
        let closing = {
            let mut state = lock(&self.state);
            state.native = Some(native.clone());
            state.closing
        };
        if closing {
            native.request_close();
        }
    }

    pub(super) fn is_closing(&self) -> bool {
        lock(&self.state).closing
    }

    pub(super) fn service_poll(&self) {
        crate::native_backend_poll::service(&mut lock(&self.state));
    }
    pub(super) fn poll_deadline(&self) -> Option<tokio::time::Instant> {
        crate::native_backend_poll::deadline(&lock(&self.state))
    }

    pub(super) fn request_close(&self) {
        let native = {
            let mut state = lock(&self.state);
            if !state.closing {
                state.closing = true;
                if state.failure.is_none() && state.advance().is_ok() {
                    state.frame.phase = SearchPhase::Cancelled;
                }
            }
            crate::native_backend_poll::service(&mut state);
            state.native.clone()
        };
        if let Some(native) = native {
            native.request_close();
        }
        self.changed.send_replace(());
    }

    pub(super) fn fail(&self, error: SearchError) {
        lock(&self.state).fail(error);
        self.changed.send_replace(());
    }

    /// Fixes the receipt before provider bookkeeping. Publication occurs only
    /// after aggregate reservations have been released/quarantined by Provider.
    pub(super) fn finish(&self, mut outcome: SearchCloseOutcome) -> SearchCloseOutcome {
        let mut state = lock(&self.state);
        if let Some(prior) = &state.outcome {
            return prior.clone();
        }
        if let Some(error) = &state.failure {
            outcome.operation = Err(error.clone());
        }
        if let Err(error) = &outcome.operation {
            state.fail(error.clone());
        } else if let CloseCleanup::Unconfirmed(error) = &outcome.cleanup {
            if state.advance().is_ok() {
                state.frame.phase = SearchPhase::Failed(error.clone());
            }
        } else if state.advance().is_ok() {
            state.frame.phase = SearchPhase::Closed;
        }
        if let Some(error) = &state.failure {
            outcome.operation = Err(error.clone());
        }
        state.closing = true;
        state.native = None;
        state.outcome = Some(outcome.clone());
        crate::native_backend_poll::service(&mut state);
        drop(state);
        self.changed.send_replace(());
        outcome
    }

    pub(super) fn publish_close(&self, outcome: SearchCloseOutcome) {
        self.closed.send_replace(Some(outcome));
    }
}

impl SessionState {
    fn advance(&mut self) -> Result<(), SearchError> {
        let Some(revision) = self.frame.revision.checked_add(1) else {
            let error = exhausted("native file-search frame revisions exhausted");
            self.failure.get_or_insert_with(|| error.clone());
            self.closing = true;
            self.frame.phase = SearchPhase::Failed(self.failure.clone().unwrap_or(error.clone()));
            return Err(error);
        };
        self.frame.revision = revision;
        Ok(())
    }

    pub(super) fn fail(&mut self, error: SearchError) {
        if self.failure.is_none() {
            self.failure = Some(error.clone());
            let _ = self.advance();
            self.frame.phase = SearchPhase::Failed(error);
        }
        self.closing = true;
    }
}

pub(super) struct SessionReporterBridge(pub(super) Arc<NativeSession>);
impl SessionReporter for SessionReporterBridge {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let mut state = lock(&self.0.state);
        if state.closing
            || snapshot.query_id != state.frame.query_id
            || snapshot.query != state.frame.query
        {
            return;
        }
        let mut frame = SearchFrame {
            revision: state.frame.revision,
            query_id: snapshot.query_id,
            query: snapshot.query.clone(),
            snapshot: Some(snapshot.clone()),
            phase: SearchPhase::Running,
        };
        if let Err(error) = frame.validate() {
            state.fail(error);
        } else if state.advance().is_ok() {
            frame.revision = state.frame.revision;
            state.frame = frame;
        }
        drop(state);
        self.0.changed.send_replace(());
    }

    fn on_complete(&self) {
        self.0.fail(failed(
            "native file-search completion omitted query identity",
        ));
    }

    fn on_complete_tagged(&self, query_id: u64) {
        let mut state = lock(&self.0.state);
        if state.closing || query_id != state.frame.query_id {
            return;
        }
        if state.frame.snapshot.as_ref().is_none_or(|snapshot| {
            !snapshot.walk_complete
                || snapshot.query_id != query_id
                || snapshot.query != state.frame.query
        }) {
            state.fail(failed(
                "native file-search idle lacked a current complete snapshot",
            ));
        } else if state.advance().is_ok() {
            state.frame.phase = SearchPhase::Idle;
        }
        drop(state);
        self.0.changed.send_replace(());
    }

    fn on_error(&self, error: &SearchError) {
        #[cfg(test)]
        {
            let before_error = lock(&self.0.before_error).clone();
            if let Some(before_error) = before_error {
                before_error(error);
            }
        }
        self.0.fail(error.clone());
    }
}
