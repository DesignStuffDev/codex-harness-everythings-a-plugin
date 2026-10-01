#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileMatch;
use codex_file_search_api::MatchType;
use codex_file_search_api::QueryAccepted;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use pretty_assertions::assert_eq;
use serde_json::json;

use super::state::lock;
use super::state::observe;

#[path = "process_test_fixture.rs"]
mod fixture;
use fixture::Fixture;

fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}

fn query(id: u64, text: &str) -> SearchQuery {
    SearchQuery {
        id: NonZeroU64::new(id).unwrap(),
        text: text.to_owned(),
    }
}

#[path = "process_startup_tests.rs"]
mod startup;

#[tokio::test]
async fn process_round_trip_preserves_original_roots_query_and_explicit_cwd() {
    let before = std::env::current_dir().unwrap();
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    assert_eq!(
        fixture.records("cwd"),
        fixture.initialization.base_dir.to_str().unwrap()
    );
    assert_eq!(std::env::current_dir().unwrap(), before);
    let session = backend.open(fixture.request()).await.unwrap();
    let submitted = query(1, "quote\" \\ λ");
    assert_eq!(
        session.update_query(submitted.clone()).await.unwrap(),
        QueryAccepted { id: submitted.id }
    );
    let SearchPoll::Changed(frame) = session.next_snapshot(0, Duration::ZERO).await.unwrap() else {
        panic!("expected snapshot");
    };
    assert_eq!((frame.query_id, frame.query), (1, submitted.text));
    assert_eq!(
        frame.snapshot.unwrap().matches,
        vec![FileMatch {
            score: 7,
            path: "file.txt".into(),
            match_type: MatchType::File,
            root: ".".into(),
            indices: None,
        }]
    );
    assert_eq!(session.close().await, joined());
    assert_eq!(session.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
async fn abandoned_open_keeps_allocation_until_reserved_release_joins() {
    let mut fixture = Fixture::new(json!({"hold_open":true,"hold_release":true}));
    fixture.initialization.requested_limits.max_leases = 1;
    let backend = fixture.connect().await;
    let opener = Arc::clone(&backend);
    let request = fixture.request();
    let opening = tokio::spawn(async move { opener.open(request).await });
    fixture.wait("open", 1).await;
    opening.abort();
    assert!(matches!(opening.await, Err(error) if error.is_cancelled()));
    fixture.wait("release_started", 1).await;
    assert_eq!(lock(&backend.inner.state).leases.len(), 1);
    assert_eq!(
        backend
            .open(fixture.request())
            .await
            .err()
            .unwrap()
            .operation
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    fixture.gate("release_gate");
    tokio::time::timeout(Duration::from_secs(5), async {
        while !lock(&backend.inner.state).leases.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    fixture.gate("open_gate");
    let replacement = backend.open(fixture.request()).await.unwrap();
    assert_eq!(replacement.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    assert_eq!(fixture.records("epochs"), "1\n2\n");
    fixture.assert_reaped();
}

#[tokio::test]
async fn abandoned_poll_and_update_keep_their_lanes_until_actual_replies() {
    let fixture = Fixture::new(json!({"hold_poll":true,"hold_update":true}));
    let backend = fixture.connect().await;
    let session = backend.open(fixture.request()).await.unwrap();
    let poller = Arc::clone(&session);
    let polling = tokio::spawn(async move { poller.next_snapshot(0, Duration::ZERO).await });
    fixture.wait("next_snapshot", 1).await;
    polling.abort();
    assert!(polling.await.unwrap_err().is_cancelled());
    assert_eq!(
        session
            .next_snapshot(0, Duration::ZERO)
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    let updater = Arc::clone(&session);
    let updating = tokio::spawn(async move { updater.update_query(query(1, "first")).await });
    fixture.wait("update_query", 1).await;
    updating.abort();
    assert!(updating.await.unwrap_err().is_cancelled());
    assert_eq!(
        session
            .update_query(query(2, "second"))
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    fixture.gate("update_gate");
    fixture.wait("update_done", 1).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match session.update_query(query(2, "second")).await {
                Err(error) if error.kind() == SearchErrorKind::ResourceExhausted => {
                    tokio::time::sleep(Duration::from_millis(5)).await
                }
                result => {
                    result.unwrap();
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    fixture.gate("poll_gate");
    fixture.wait("poll_done", 1).await;
    assert_eq!(session.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    assert_eq!(fixture.records("next_snapshot").lines().count(), 1);
    assert_eq!(fixture.records("update_query").lines().count(), 2);
    fixture.assert_reaped();
}

#[tokio::test]
async fn last_public_session_and_backend_drops_request_owned_cleanup() {
    let fixture = Fixture::new(json!({"hold_release":true}));
    let backend = fixture.connect().await;
    let session = backend.open(fixture.request()).await.unwrap();
    let second = Arc::clone(&session);
    drop(session);
    assert_eq!(
        second.update_query(query(1, "alive")).await.unwrap(),
        QueryAccepted {
            id: NonZeroU64::new(1).unwrap()
        }
    );
    drop(second);
    fixture.wait("release_started", 1).await;
    let completion = backend.inner.completed.subscribe();
    drop(backend);
    fixture.gate("release_gate");
    assert_eq!(observe(completion).await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
async fn unconfirmed_release_retains_allocation_and_original_operation_failure() {
    let fixture = Fixture::new(json!({"unconfirmed":true,"operation_error":true}));
    let backend = fixture.connect().await;
    let session = backend.open(fixture.request()).await.unwrap();
    let expected = SearchCloseOutcome {
        operation: Err(SearchError::new(
            SearchErrorKind::SearchFailed,
            "original search failure",
        )),
        cleanup: CloseCleanup::Unconfirmed(SearchError::new(
            SearchErrorKind::TransportLost,
            "cleanup proof unavailable",
        )),
    };
    assert_eq!(session.close().await, expected);
    assert_eq!(session.close().await, expected);
    assert_eq!(backend.shutdown().await, expected);
    assert_eq!(lock(&backend.inner.state).leases.len(), 1);
    assert_eq!(
        backend
            .open(fixture.request())
            .await
            .err()
            .unwrap()
            .operation
            .kind(),
        SearchErrorKind::ClosedLease
    );
    fixture.assert_reaped();
}

#[tokio::test]
async fn mismatched_lease_reply_fences_the_provider_without_replay() {
    let fixture = Fixture::new(json!({"bad_identity":true}));
    let backend = fixture.connect().await;
    let session = backend.open(fixture.request()).await.unwrap();
    assert_eq!(
        session
            .next_snapshot(0, Duration::ZERO)
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::TransportLost
    );
    assert_eq!(
        backend.shutdown().await.operation.unwrap_err().kind(),
        SearchErrorKind::TransportLost
    );
    assert_eq!(fixture.records("next_snapshot").lines().count(), 1);
    fixture.assert_reaped();
}

#[tokio::test]
async fn obsolete_query_frames_are_not_presented_as_the_new_query() {
    let fixture = Fixture::new(json!({"old_query":true}));
    let backend = fixture.connect().await;
    let session = backend.open(fixture.request()).await.unwrap();
    session.update_query(query(1, "current")).await.unwrap();
    assert_eq!(
        session
            .next_snapshot(0, Duration::ZERO)
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::StaleEpoch
    );
    assert_eq!(session.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
async fn ordered_admission_does_not_wait_for_the_previous_open_response() {
    let fixture = Fixture::new(json!({"hold_open":true}));
    let backend = fixture.connect().await;
    let first_backend = Arc::clone(&backend);
    let first_request = fixture.request();
    let first = tokio::spawn(async move { first_backend.open(first_request).await });
    fixture.wait("open", 1).await;
    let second_backend = Arc::clone(&backend);
    let second_request = fixture.request();
    let second = tokio::spawn(async move { second_backend.open(second_request).await });
    fixture.wait("open", 2).await;
    assert_eq!(fixture.records("epochs"), "1\n2\n");
    first.abort();
    second.abort();
    assert!(matches!(first.await, Err(error) if error.is_cancelled()));
    assert!(matches!(second.await, Err(error) if error.is_cancelled()));
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test]
async fn oversize_query_is_rejected_before_encoding_or_identity_admission() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let session = backend.open(fixture.request()).await.unwrap();
    let oversized = "λ".repeat(32_769);
    assert_eq!(
        session
            .update_query(query(1, &oversized))
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    assert_eq!(
        session
            .update_query(query(1, "admitted once"))
            .await
            .unwrap(),
        QueryAccepted {
            id: NonZeroU64::new(1).unwrap()
        }
    );
    assert_eq!(session.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    assert_eq!(fixture.records("update_query").lines().count(), 1);
    fixture.assert_reaped();
}

#[tokio::test]
async fn oversized_roots_are_rejected_before_retained_startup_or_wire_admission() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let mut request = fixture.request();
    request.roots = vec!["x".repeat(262_145).into()];
    let error = backend.open(request).await.err().unwrap();
    assert_eq!(
        (error.operation.kind(), error.cleanup),
        (
            SearchErrorKind::ResourceExhausted,
            codex_file_search_api::StartCleanup::NotAdmitted
        )
    );
    assert_eq!(lock(&backend.inner.state).leases.len(), 0);
    let session = backend.open(fixture.request()).await.unwrap();
    assert_eq!(session.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    assert_eq!(fixture.records("epochs"), "1\n");
    fixture.assert_reaped();
}
