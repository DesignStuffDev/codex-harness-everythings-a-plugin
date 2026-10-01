//! One owned observation/callback pump per admitted lease. No callback holds a
//! provider/scope/session state mutex, and joined close waits for the pump.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchPoll;
use futures::FutureExt;

use crate::RuntimePolicy;
use crate::policy::invalid;
use crate::session::OperationGuard;
use crate::state::*;

pub(crate) async fn run(
    lease: Arc<Lease>,
    backend: Arc<dyn SearchBackendSession>,
    policy: RuntimePolicy,
    mut guard: OperationGuard,
) {
    let result = AssertUnwindSafe(pump(&lease, &backend, policy))
        .catch_unwind()
        .await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => lease.fail(error),
        Err(_) => lease.fail(panicked()),
    }
    let failure = lock(&lease.state).first_error.clone();
    if let Some(error) = failure
        && std::panic::catch_unwind(AssertUnwindSafe(|| lease.reporter.on_error(&error))).is_err()
    {
        lease.fail(panicked());
    }
    guard.finished = true;
}

async fn pump(
    lease: &Arc<Lease>,
    backend: &Arc<dyn SearchBackendSession>,
    policy: RuntimePolicy,
) -> Result<(), SearchError> {
    let mut cursor = 0;
    let mut completed_query = None;
    loop {
        if lock(&lease.state).closing {
            return Ok(());
        }
        // Do not abandon this accepted poll on close; request_close wakes it,
        // and the retained pump waits its actual completion before joined close.
        let poll_started = tokio::time::Instant::now();
        let poll = match backend.next_snapshot(cursor, policy.poll_wait).await {
            Ok(poll) => poll,
            Err(error) if error.kind() == SearchErrorKind::StaleEpoch => {
                // No revision accompanies an error. Yield a bounded delay and
                // retry the same cursor; never replay a query or relabel a frame.
                tokio::time::sleep(policy.poll_wait.min(Duration::from_millis(5))).await;
                continue;
            }
            Err(error)
                if error.kind() == SearchErrorKind::ClosedLease && lock(&lease.state).closing =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        poll.validate_after(cursor)?;
        let SearchPoll::Changed(frame) = poll else {
            // A backend may return no progress without consuming its wait. Pace
            // those replies so this pump cannot monopolize a single-threaded
            // executor and prevent updates or cancellation from being admitted.
            let remaining = policy
                .poll_wait
                .min(Duration::from_millis(5))
                .saturating_sub(poll_started.elapsed());
            if remaining.is_zero() {
                tokio::task::yield_now().await;
            } else {
                tokio::time::sleep(remaining).await;
            }
            continue;
        };
        policy.validate_frame(&frame, &lease.roots, lease.match_limit)?;
        cursor = frame.revision;
        if let SearchPhase::Failed(error) = &frame.phase {
            return Err(error.clone());
        }
        if matches!(frame.phase, SearchPhase::Cancelled | SearchPhase::Closed) {
            lease.request_close();
            return Ok(());
        }
        let current = {
            let state = lock(&lease.state);
            if frame.query_id > state.query_id
                || (frame.query_id == state.query_id && frame.query != state.query)
            {
                return Err(invalid("file-search backend delivered an unadmitted query"));
            }
            !state.closing && frame.query_id != 0 && frame.query_id == state.query_id
        };
        if !current {
            continue;
        }
        if let Some(snapshot) = &frame.snapshot {
            // This check is the callback admission point. Already admitted
            // callbacks may overlap a later update/close; their owner is joined.
            // UI/outgoing queues must retain their own identity fence at delivery.
            if matches_current(lease, frame.query_id, &frame.query) {
                lease.reporter.on_update(snapshot);
            }
        }
        if frame.phase == SearchPhase::Idle
            && completed_query != Some(frame.query_id)
            && matches_current(lease, frame.query_id, &frame.query)
        {
            lease.reporter.on_complete_tagged(frame.query_id);
            completed_query = Some(frame.query_id);
        }
    }
}
fn matches_current(lease: &Lease, query_id: u64, query: &str) -> bool {
    let state = lock(&lease.state);
    !state.closing && state.query_id == query_id && state.query == query
}
