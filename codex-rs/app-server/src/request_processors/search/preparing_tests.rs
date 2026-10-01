use super::preparing_fixture::*;
use super::*;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use pretty_assertions::assert_eq;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::timeout;

fn one_shot(query: &str, token: &str) -> FuzzyFileSearchParams {
    FuzzyFileSearchParams {
        query: query.into(),
        roots: vec!["/fixture".into()],
        cancellation_token: Some(token.into()),
    }
}
fn session_start() -> FuzzyFileSearchSessionStartParams {
    FuzzyFileSearchSessionStartParams {
        session_id: "held".into(),
        roots: vec!["/fixture".into()],
    }
}

#[tokio::test]
async fn token_retirement_reaches_pending_owner_and_waits_for_cleanup_without_cancelling_sibling() {
    let first = Start::new(Ok(()));
    let sibling = Start::new(Ok(()));
    let (processor, provider, _outgoing) = fixture(vec![first.clone(), sibling.clone()]);
    let connection = Arc::new(SearchConnectionState::default());
    first.check_connection(&connection);
    sibling.check_connection(&connection);
    let opening = processor.fuzzy_file_search(
        ConnectionId(1),
        connection.clone(),
        one_shot("alpha", "first"),
    );
    tokio::pin!(opening);
    assert!(futures::poll!(&mut opening).is_pending());
    entered(&first).await;
    let other = processor.fuzzy_file_search(
        ConnectionId(1),
        connection.clone(),
        one_shot("beta", "sibling"),
    );
    tokio::pin!(other);
    assert!(futures::poll!(&mut other).is_pending());
    entered(&sibling).await;
    processor
        .fuzzy_file_search(ConnectionId(1), connection.clone(), one_shot("", "first"))
        .await
        .expect("token retirement");
    cancelled(&first).await;
    assert!(!sibling.cancelled.is_cancelled());
    assert_eq!(
        connection.state.lock().expect("state").one_shots.len(),
        2,
        "cancel intent cannot release the request slot"
    );
    assert!(
        futures::poll!(&mut opening).is_pending(),
        "cleanup is still gated"
    );
    first.release.add_permits(1);
    assert_eq!(
        timeout(Duration::from_secs(2), opening)
            .await
            .expect("drained first request")
            .expect("clean cancellation"),
        FuzzyFileSearchResponse { files: Vec::new() }
    );
    assert!(!sibling.cancelled.is_cancelled());
    processor
        .fuzzy_file_search(ConnectionId(1), connection.clone(), one_shot("", "sibling"))
        .await
        .expect("sibling retirement");
    cancelled(&sibling).await;
    sibling.release.add_permits(1);
    timeout(Duration::from_secs(2), other)
        .await
        .expect("drained sibling")
        .expect("clean cancellation");
    processor
        .shutdown()
        .await
        .expect("joined consumer shutdown");
    assert!(connection.startups.is_empty());
    assert!(!first.hook_under_fence.load(Ordering::Acquire));
    assert!(!sibling.hook_under_fence.load(Ordering::Acquire));
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[tokio::test]
async fn stop_waits_for_pending_constructor_receipt_before_releasing_starting_entry() {
    let start = Start::new(Ok(()));
    let (processor, provider, _outgoing) = fixture(vec![start.clone()]);
    let connection = Arc::new(SearchConnectionState::default());
    start.check_connection(&connection);
    let opening = processor.fuzzy_file_search_session_start_response(
        ConnectionId(1),
        connection.clone(),
        session_start(),
    );
    tokio::pin!(opening);
    assert!(futures::poll!(&mut opening).is_pending());
    entered(&start).await;
    let stop = processor.fuzzy_file_search_session_stop(
        ConnectionId(1),
        connection.clone(),
        FuzzyFileSearchSessionStopParams {
            session_id: "held".into(),
        },
    );
    tokio::pin!(stop);
    assert!(futures::poll!(&mut stop).is_pending());
    cancelled(&start).await;
    assert!(
        connection
            .state
            .lock()
            .expect("state")
            .pending
            .contains_key("held")
    );
    start.release.add_permits(1);
    timeout(Duration::from_secs(2), stop)
        .await
        .expect("stop receipt")
        .expect("joined stop");
    assert!(
        opening.await.is_err(),
        "retired startup cannot become ready"
    );
    assert!(connection.state.lock().expect("state").pending.is_empty());
    processor.shutdown().await.expect("shutdown");
    assert!(!start.hook_under_fence.load(Ordering::Acquire));
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[tokio::test]
async fn caller_retirement_does_not_hide_a_genuine_closed_lease_constructor_failure() {
    let failure = SearchError::new(
        SearchErrorKind::ClosedLease,
        "backend actually refused this constructor",
    );
    let start = Start::new(Err(failure.clone()));
    let (processor, provider, _outgoing) = fixture(vec![start.clone()]);
    let connection = Arc::new(SearchConnectionState::default());
    let opening = processor.fuzzy_file_search(
        ConnectionId(1),
        connection.clone(),
        one_shot("alpha", "failed"),
    );
    tokio::pin!(opening);
    assert!(futures::poll!(&mut opening).is_pending());
    entered(&start).await;
    processor
        .fuzzy_file_search(ConnectionId(1), connection.clone(), one_shot("", "failed"))
        .await
        .expect("retirement request");
    cancelled(&start).await;
    start.release.add_permits(1);
    let error = timeout(Duration::from_secs(2), opening)
        .await
        .expect("retained failure")
        .expect_err("known ClosedLease is not synthetic cancellation");
    assert!(error.message.contains(failure.message()));
    assert!(
        processor.shutdown().await.is_err(),
        "scope retains the real cause"
    );
    let closed = provider.shutdown().await;
    assert_eq!(closed.operation, Err(failure));
    assert_eq!(closed.cleanup, CloseCleanup::Joined);
}
