//! A single retained poll belongs to the lifecycle owner, never its waiter.

use std::sync::Arc;
use std::time::Duration;

use codex_file_search_api::SearchError;
use codex_file_search_api::SearchPoll;
use tokio::sync::oneshot;
use tokio::time::Instant;

use crate::native_backend::lock;
use crate::native_backend_policy::exhausted;
use crate::native_backend_policy::failed;
use crate::native_backend_policy::invalid;
use crate::native_backend_session::NativeSession;
use crate::native_backend_session::SessionState;

pub(super) struct PendingPoll {
    after: u64,
    deadline: Instant,
    sender: oneshot::Sender<Result<SearchPoll, SearchError>>,
}

pub(super) async fn next(
    session: Arc<NativeSession>,
    after: u64,
    wait: Duration,
) -> Result<SearchPoll, SearchError> {
    if wait > session.limits.max_poll_wait {
        return Err(invalid("native file-search poll exceeds negotiated wait"));
    }
    let receiver = {
        let mut state = lock(&session.state);
        if after > state.frame.revision {
            return Err(invalid("native file-search poll cursor is in the future"));
        }
        if state.poll.is_some() {
            return Err(exhausted("native file-search already has an admitted poll"));
        }
        if state.closing || state.frame.revision > after || wait.is_zero() {
            return current(&state, after);
        }
        let (sender, receiver) = oneshot::channel();
        state.poll = Some(PendingPoll {
            after,
            deadline: Instant::now() + wait,
            sender,
        });
        receiver
    };
    // No new job/thread: the already-owned lifecycle actor serves this slot.
    // Dropping receiver does not clear it or permit another accepted poll.
    session.changed.send_replace(());
    receiver
        .await
        .map_err(|_| failed("native file-search retained poll owner disappeared"))?
}

pub(super) fn deadline(state: &SessionState) -> Option<Instant> {
    state.poll.as_ref().map(|poll| poll.deadline)
}

/// Called with the session mutex held. oneshot send is nonblocking and cannot
/// invoke a user callback; the next admission cannot precede receipt completion.
pub(super) fn service(state: &mut SessionState) {
    let Some(poll) = &state.poll else {
        return;
    };
    if !state.closing && state.frame.revision == poll.after && Instant::now() < poll.deadline {
        return;
    }
    if let Some(poll) = state.poll.take() {
        let result = current(state, poll.after);
        let _ = poll.sender.send(result);
    }
}

fn current(state: &SessionState, after: u64) -> Result<SearchPoll, SearchError> {
    if state.frame.revision > after {
        return Ok(SearchPoll::Changed(state.frame.clone()));
    }
    if state.frame.revision == u64::MAX
        && let Some(error) = &state.failure
    {
        return Err(error.clone());
    }
    Ok(SearchPoll::Unchanged { revision: after })
}
