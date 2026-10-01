use std::sync::Mutex;

use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SessionReporter;
use tokio::sync::oneshot;

/// One bounded latest snapshot owned by the CLI consumer, in addition to the
/// facade's frame. No unbounded callback queue and no output before joined
/// cleanup. A transient clone is distinct from native/wire accounting.
pub(super) struct OneShotReporter {
    state: Mutex<State>,
}
struct State {
    snapshot: Option<FileSearchSnapshot>,
    reply: Option<oneshot::Sender<Result<FileSearchSnapshot, SearchError>>>,
}
impl OneShotReporter {
    pub(super) fn new() -> (
        Self,
        oneshot::Receiver<Result<FileSearchSnapshot, SearchError>>,
    ) {
        let (reply, receiver) = oneshot::channel();
        (
            Self {
                state: Mutex::new(State {
                    snapshot: None,
                    reply: Some(reply),
                }),
            },
            receiver,
        )
    }
}
impl SessionReporter for OneShotReporter {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        if snapshot.query_id != 1 {
            return;
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.reply.is_some() {
            state.snapshot = Some(snapshot.clone());
        }
    }
    fn on_complete(&self) {}
    fn on_complete_tagged(&self, query_id: u64) {
        if query_id != 1 {
            return;
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(reply) = state.reply.take() else {
            return;
        };
        let result = state
            .snapshot
            .take()
            .filter(|snapshot| snapshot.walk_complete && snapshot.query_id == query_id)
            .ok_or_else(|| {
                SearchError::new(
                    SearchErrorKind::SearchFailed,
                    "file-search CLI received completion without its current complete snapshot",
                )
            });
        let _ = reply.send(result);
    }
    fn on_error(&self, error: &SearchError) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.snapshot.take();
        if let Some(reply) = state.reply.take() {
            let _ = reply.send(Err(error.clone()));
        }
    }
}
