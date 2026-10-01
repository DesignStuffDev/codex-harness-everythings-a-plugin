//! Exercise the backend contract through real traversal, matching and ownership.
//!
//! Timeouts bound a hung test; Joined receipts and successful quota reuse are the
//! cleanup assertions. These tests never change the process working directory.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use crate::NativeBackendLimits;
use crate::NativeSearchBackend;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use codex_file_search_api::StartCleanup;
use pretty_assertions::assert_eq;
use std::future::poll_fn;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(10);
const POLL_WAIT: Duration = Duration::from_millis(200);
const RETAINED_WAIT: Duration = Duration::from_secs(30);

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

fn budget() -> SearchBudget {
    SearchBudget {
        max_index_entries: nonzero(64),
        max_index_bytes: nonzero(8 * 1024 * 1024),
        max_worker_threads: nonzero(4),
    }
}

fn limits(sessions: usize) -> NativeBackendLimits {
    let allocation = budget();
    NativeBackendLimits {
        max_sessions: nonzero(sessions),
        resources: SearchBudget {
            max_index_entries: nonzero(allocation.max_index_entries.get() * sessions),
            max_index_bytes: nonzero(allocation.max_index_bytes.get() * sessions),
            max_worker_threads: nonzero(allocation.max_worker_threads.get() * sessions),
        },
        max_query_bytes: nonzero(256),
        max_roots_options_bytes: nonzero(64 * 1024),
        max_matches: nonzero(32),
        max_snapshot_bytes: nonzero(64 * 1024),
        max_poll_wait: RETAINED_WAIT,
    }
}

fn options() -> FileSearchOptions {
    FileSearchOptions {
        threads: nonzero(1),
        compute_indices: true,
        respect_gitignore: false,
        ..FileSearchOptions::default()
    }
}

fn fixture() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    write_files(root.path());
    root
}

fn write_files(root: &Path) {
    for name in ["alpha.txt", "beta.txt", "naïve.txt", "中.txt"] {
        std::fs::write(root.join(name), "real native search fixture").unwrap();
    }
}

fn backend(policy: NativeBackendLimits) -> NativeSearchBackend {
    NativeSearchBackend::new(std::env::current_dir().unwrap(), policy).unwrap()
}

fn request(root: &Path) -> SearchOpen {
    SearchOpen {
        roots: vec![root.to_path_buf()],
        options: options(),
        budget: budget(),
    }
}

fn query(id: u64, text: &str) -> SearchQuery {
    SearchQuery {
        id: NonZeroU64::new(id).unwrap(),
        text: text.to_owned(),
    }
}

async fn open(backend: &NativeSearchBackend, request: SearchOpen) -> Arc<dyn SearchBackendSession> {
    timeout(WAIT, backend.open(request))
        .await
        .expect("native admission must finish")
        .unwrap_or_else(|error| panic!("unexpected native startup failure: {error}"))
}

async fn update(session: &dyn SearchBackendSession, id: u64, text: &str) {
    let accepted = timeout(WAIT, session.update_query(query(id, text)))
        .await
        .expect("query admission must finish")
        .expect("valid query must be admitted");
    assert_eq!(accepted.id.get(), id);
}

async fn settled(
    session: &dyn SearchBackendSession,
    id: u64,
    text: &str,
    cursor: &mut u64,
) -> FileSearchSnapshot {
    timeout(WAIT, async {
        loop {
            let poll = session.next_snapshot(*cursor, POLL_WAIT).await.unwrap();
            poll.validate_after(*cursor).unwrap();
            if let SearchPoll::Changed(frame) = poll {
                *cursor = frame.revision;
                assert_eq!(
                    frame.query_id, id,
                    "accepted query identity must not regress"
                );
                assert_eq!(frame.query, text);
                match frame.phase {
                    SearchPhase::Running => {}
                    SearchPhase::Idle => {
                        // The latest-state contract permits coalescing the prior
                        // Running frame; Idle itself must contain its full,
                        // current completed snapshot rather than an old result.
                        let snapshot = frame.snapshot.expect("idle must contain a snapshot");
                        assert_eq!(snapshot.query_id, id);
                        assert_eq!(snapshot.query, text);
                        assert!(snapshot.walk_complete);
                        return snapshot;
                    }
                    phase => panic!("valid query ended before idle: {phase:?}"),
                }
            }
        }
    })
    .await
    .expect("real native query must settle")
}

async fn close(session: &dyn SearchBackendSession) -> SearchCloseOutcome {
    let outcome = timeout(WAIT, session.close())
        .await
        .expect("session close must join");
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    outcome
}

async fn shutdown(backend: &NativeSearchBackend) -> SearchCloseOutcome {
    let outcome = timeout(WAIT, backend.shutdown())
        .await
        .expect("provider shutdown must join");
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    outcome
}

async fn failed(session: &dyn SearchBackendSession) -> SearchError {
    timeout(WAIT, async {
        let mut cursor = 0;
        loop {
            let poll = session.next_snapshot(cursor, POLL_WAIT).await.unwrap();
            poll.validate_after(cursor).unwrap();
            if let SearchPoll::Changed(frame) = poll {
                cursor = frame.revision;
                match frame.phase {
                    SearchPhase::Failed(error) => return error,
                    SearchPhase::Running => {}
                    phase => panic!("exhausted native search reported {phase:?}"),
                }
            }
        }
    })
    .await
    .expect("native exhaustion must become an explicit failure")
}

async fn reopen_after_join(
    backend: &NativeSearchBackend,
    request: SearchOpen,
) -> Arc<dyn SearchBackendSession> {
    timeout(WAIT, async {
        loop {
            match backend.open(request.clone()).await {
                Ok(session) => return session,
                Err(error) => {
                    assert_eq!(error.operation.kind(), SearchErrorKind::ResourceExhausted);
                    assert_eq!(error.cleanup, StartCleanup::NotAdmitted);
                    // Only a real successful re-admission proves that the
                    // provider's prior joined release returned its reservation.
                    tokio::task::yield_now().await;
                }
            }
        }
    })
    .await
    .expect("dropped public ownership must eventually release joined capacity")
}

#[tokio::test]
async fn repeated_normalized_and_a_b_a_queries_keep_exact_ids_and_complete_snapshots() {
    let root = fixture();
    let backend = backend(limits(1));
    let session = open(&backend, request(root.path())).await;
    let mut cursor = 0;
    let queries = [
        ("alpha", Some("alpha.txt")),
        ("alpha", Some("alpha.txt")),
        ("ALPHA", Some("alpha.txt")),
        ("beta", Some("beta.txt")),
        ("alpha", Some("alpha.txt")),
        ("naive", Some("naïve.txt")),
        ("naïve", Some("naïve.txt")),
        ("zzzz_not_present", None),
    ];
    for (index, (text, expected)) in queries.into_iter().enumerate() {
        let id = index as u64 + 1;
        update(session.as_ref(), id, text).await;
        let snapshot = settled(session.as_ref(), id, text, &mut cursor).await;
        assert!(snapshot.scanned_file_count >= 4);
        let paths: Vec<_> = snapshot
            .matches
            .iter()
            .map(|matched| matched.path.clone())
            .collect();
        assert_eq!(
            paths,
            expected.map(PathBuf::from).into_iter().collect::<Vec<_>>()
        );
        assert_eq!(snapshot.total_match_count, usize::from(expected.is_some()));
        for matched in snapshot.matches {
            assert_eq!(matched.root.as_os_str(), root.path().as_os_str());
            assert!(matched.full_path().is_file());
            assert!(matched.indices.is_some_and(|indices| !indices.is_empty()));
        }
    }
    assert_eq!(close(session.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}

#[tokio::test]
async fn abandoned_poll_observer_retains_its_single_admission_until_actual_completion() {
    let root = fixture();
    let backend = backend(limits(1));
    let session = open(&backend, request(root.path())).await;
    let mut cursor = 0;
    update(session.as_ref(), 1, "alpha").await;
    settled(session.as_ref(), 1, "alpha", &mut cursor).await;

    let mut observer = session.next_snapshot(cursor, RETAINED_WAIT);
    poll_fn(|context| {
        assert!(observer.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(observer);
    let rejected = session
        .next_snapshot(cursor, Duration::ZERO)
        .await
        .unwrap_err();
    assert_eq!(rejected.kind(), SearchErrorKind::ResourceExhausted);

    update(session.as_ref(), 2, "beta").await;
    // The abandoned observer is completed by this real revision change. Wait
    // for its owned slot to drain; never cancel it just to make a retry succeed.
    timeout(WAIT, async {
        loop {
            match session.next_snapshot(cursor, Duration::ZERO).await {
                Err(error) => {
                    assert_eq!(error.kind(), SearchErrorKind::ResourceExhausted);
                    tokio::task::yield_now().await;
                }
                Ok(poll) => {
                    poll.validate_after(cursor).unwrap();
                    break;
                }
            }
        }
    })
    .await
    .expect("retained poll must finish after a real update");
    let snapshot = settled(session.as_ref(), 2, "beta", &mut cursor).await;
    assert_eq!(snapshot.matches[0].path, Path::new("beta.txt"));
    assert_eq!(close(session.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}

#[tokio::test]
async fn dropping_last_public_lease_with_an_abandoned_poll_releases_joined_capacity() {
    let root = fixture();
    let backend = backend(limits(1));
    let session = open(&backend, request(root.path())).await;
    let mut cursor = 0;
    update(session.as_ref(), 1, "alpha").await;
    settled(session.as_ref(), 1, "alpha", &mut cursor).await;
    let mut observer = session.next_snapshot(cursor, RETAINED_WAIT);
    poll_fn(|context| {
        assert!(observer.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(observer);
    drop(session);

    let replacement = reopen_after_join(&backend, request(root.path())).await;
    update(replacement.as_ref(), 1, "beta").await;
    let snapshot = settled(replacement.as_ref(), 1, "beta", &mut 0).await;
    assert_eq!(snapshot.matches[0].path, Path::new("beta.txt"));
    assert_eq!(close(replacement.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}

#[tokio::test]
async fn abandoned_open_observer_cannot_orphan_preparing_native_work() {
    let root = fixture();
    let backend = backend(limits(1));
    let mut observer = backend.open(request(root.path()));
    // The current-thread runtime cannot run the owned startup task until this
    // poll yields, so this is a real accepted Preparing session, not a timer.
    poll_fn(|context| {
        assert!(observer.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(observer);

    let replacement = reopen_after_join(&backend, request(root.path())).await;
    update(replacement.as_ref(), 1, "alpha").await;
    let snapshot = settled(replacement.as_ref(), 1, "alpha", &mut 0).await;
    assert_eq!(snapshot.matches[0].path, Path::new("alpha.txt"));
    assert_eq!(close(replacement.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}

#[tokio::test]
async fn closing_one_shared_lease_fences_its_clones_without_harming_siblings() {
    let root = fixture();
    let backend = backend(limits(2));
    let first = open(&backend, request(root.path())).await;
    let sibling = open(&backend, request(root.path())).await;
    let first_clone = first.clone();
    update(first.as_ref(), 1, "alpha").await;
    settled(first.as_ref(), 1, "alpha", &mut 0).await;
    update(sibling.as_ref(), 1, "alpha").await;
    let mut sibling_cursor = 0;
    settled(sibling.as_ref(), 1, "alpha", &mut sibling_cursor).await;

    first_clone.request_close();
    assert_eq!(
        first
            .update_query(query(2, "beta"))
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::ClosedLease
    );
    let outcome = close(first.as_ref()).await;
    assert_eq!(outcome.operation, Ok(()));
    assert_eq!(close(first_clone.as_ref()).await, outcome);
    update(sibling.as_ref(), 2, "beta").await;
    let snapshot = settled(sibling.as_ref(), 2, "beta", &mut sibling_cursor).await;
    assert_eq!(snapshot.matches[0].path, Path::new("beta.txt"));
    assert_eq!(close(sibling.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}

#[tokio::test]
async fn each_aggregate_resource_axis_blocks_second_admission_until_joined_release() {
    let root = fixture();
    for axis in 0..3 {
        let mut policy = limits(2);
        match axis {
            0 => policy.resources.max_index_entries = budget().max_index_entries,
            1 => policy.resources.max_index_bytes = budget().max_index_bytes,
            2 => policy.resources.max_worker_threads = budget().max_worker_threads,
            _ => unreachable!(),
        }
        let backend = backend(policy);
        let first = open(&backend, request(root.path())).await;
        update(first.as_ref(), 1, "alpha").await;
        settled(first.as_ref(), 1, "alpha", &mut 0).await;
        let rejected = backend.open(request(root.path())).await.err().unwrap();
        assert_eq!(
            rejected.operation.kind(),
            SearchErrorKind::ResourceExhausted,
            "axis={axis}"
        );
        assert_eq!(rejected.cleanup, StartCleanup::NotAdmitted);
        assert_eq!(close(first.as_ref()).await.operation, Ok(()));

        let second = open(&backend, request(root.path())).await;
        update(second.as_ref(), 1, "beta").await;
        let snapshot = settled(second.as_ref(), 1, "beta", &mut 0).await;
        assert_eq!(snapshot.matches[0].path, Path::new("beta.txt"));
        assert_eq!(close(second.as_ref()).await.operation, Ok(()));
        assert_eq!(shutdown(&backend).await.operation, Ok(()));
    }
}

#[path = "native_backend_policy_tests.rs"]
mod policy_tests;
