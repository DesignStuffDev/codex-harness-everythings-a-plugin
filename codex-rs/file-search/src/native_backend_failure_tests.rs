//! Hold the real terminal callback behind query admission to exercise the
//! native-shutdown/bridge-publication race without sleeps or fabricated errors.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::NativeSearchBackend;
use super::lock;
use crate::NativeBackendLimits;
use crate::native_backend_session::NativeSession;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileMatch;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::MatchType;
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
use pretty_assertions::assert_eq;
use std::future::poll_fn;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Poll;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::oneshot;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(10);

fn nz(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

fn query(id: u64, text: &str) -> SearchQuery {
    SearchQuery {
        id: NonZeroU64::new(id).unwrap(),
        text: text.to_owned(),
    }
}

struct FailureWindow {
    root: TempDir,
    backend: NativeSearchBackend,
    session: Arc<dyn SearchBackendSession>,
    native: Arc<NativeSession>,
    error: SearchError,
    // Dropping the sole sender always unblocks the callback, including panic.
    release: Option<crossbeam_channel::Sender<()>>,
}

impl FailureWindow {
    async fn open() -> Self {
        let root = tempfile::tempdir().unwrap();
        // Traversal includes the root directory itself as relative path "".
        // Its initial empty-query snapshot has one FileMatch and root payload.
        let snapshot_bytes =
            std::mem::size_of::<FileMatch>() + root.path().as_os_str().as_encoded_bytes().len();
        let exhausting_query = "x".repeat(snapshot_bytes + 1);
        let budget = SearchBudget {
            max_index_entries: nz(1),
            max_index_bytes: nz(8 * 1024 * 1024),
            max_worker_threads: nz(4),
        };
        let backend = NativeSearchBackend::new(
            std::env::current_dir().unwrap(),
            NativeBackendLimits {
                max_sessions: nz(1),
                resources: budget,
                max_query_bytes: nz(exhausting_query.len()),
                max_roots_options_bytes: nz(64 * 1024),
                max_matches: nz(1),
                max_snapshot_bytes: nz(snapshot_bytes),
                max_poll_wait: WAIT,
            },
        )
        .unwrap();
        let session = timeout(
            WAIT,
            backend.open(SearchOpen {
                roots: vec![root.path().to_path_buf()],
                options: FileSearchOptions {
                    limit: nz(1),
                    threads: nz(1),
                    respect_gitignore: false,
                    ..FileSearchOptions::default()
                },
                budget,
            }),
        )
        .await
        .unwrap()
        .unwrap();
        timeout(WAIT, async {
            let mut cursor = 0;
            loop {
                if let SearchPoll::Changed(frame) = session
                    .next_snapshot(cursor, Duration::from_millis(100))
                    .await
                    .unwrap()
                {
                    cursor = frame.revision;
                    if frame.phase == SearchPhase::Idle {
                        let snapshot = frame.snapshot.unwrap();
                        assert_eq!(snapshot.query_id, 0);
                        assert!(snapshot.walk_complete);
                        assert_eq!(snapshot.total_match_count, 1);
                        assert_eq!(snapshot.matches.len(), 1);
                        assert_eq!(snapshot.matches[0].path, std::path::Path::new(""));
                        assert_eq!(snapshot.matches[0].root, root.path());
                        assert_eq!(snapshot.matches[0].match_type, MatchType::Directory);
                        break;
                    }
                    assert_eq!(frame.phase, SearchPhase::Running);
                }
            }
        })
        .await
        .unwrap();
        let native = lock(&backend.inner.state)
            .sessions
            .values()
            .next()
            .unwrap()
            .clone();
        let (entered_tx, entered_rx) = oneshot::channel();
        let entered_tx = Mutex::new(Some(entered_tx));
        let (release_tx, release_rx) = crossbeam_channel::bounded::<()>(1);
        *lock(&native.before_error) = Some(Arc::new(move |error| {
            if let Some(sender) = lock(&entered_tx).take() {
                let _ = sender.send(error.clone());
            }
            let _ = release_rx.recv();
        }));
        // The initial root result fits. This admitted query alone exceeds the
        // native snapshot cap; the real matcher records ResourceExhausted.
        let accepted = timeout(WAIT, session.update_query(query(1, &exhausting_query)))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(accepted.id.get(), 1);
        let error = timeout(WAIT, entered_rx).await.unwrap().unwrap();
        assert_eq!(error.kind(), SearchErrorKind::ResourceExhausted);
        {
            let state = lock(&native.state);
            assert_eq!(state.failure, None);
            assert!(!state.closing);
        }
        Self {
            root,
            backend,
            session,
            native,
            error,
            release: Some(release_tx),
        }
    }

    async fn assert_joined_and_reusable(&self) {
        let expected = SearchCloseOutcome {
            operation: Err(self.error.clone()),
            cleanup: CloseCleanup::Joined,
        };
        assert_eq!(timeout(WAIT, self.session.close()).await.unwrap(), expected);
        assert_eq!(timeout(WAIT, self.session.close()).await.unwrap(), expected);
        assert_eq!(lock(&self.native.state).frame.query_id, 1);
        // The failed session's joined receipt must release the sole aggregate
        // reservation. A new real traversal can start and close successfully.
        let replacement = timeout(
            WAIT,
            self.backend.open(SearchOpen {
                roots: vec![self.root.path().to_path_buf()],
                options: FileSearchOptions {
                    limit: nz(1),
                    threads: nz(1),
                    respect_gitignore: false,
                    ..FileSearchOptions::default()
                },
                budget: self.backend.inner.limits.resources,
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            timeout(WAIT, replacement.close()).await.unwrap(),
            SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined,
            }
        );
        assert_eq!(
            timeout(WAIT, self.backend.shutdown()).await.unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn rejected_query_waits_for_native_failure_before_error_callback_publication() {
    let mut window = FailureWindow::open().await;
    let mut update = window.session.update_query(query(2, "later"));
    poll_fn(|cx| {
        assert!(update.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(lock(&window.native.state).closing);
    drop(window.release.take());
    assert_eq!(
        timeout(WAIT, update).await.unwrap(),
        Err(window.error.clone())
    );
    window.assert_joined_and_reusable().await;
}

#[tokio::test]
async fn closing_query_keeps_native_failure_not_yet_published_by_callback() {
    let mut window = FailureWindow::open().await;
    window.session.request_close();
    let mut update = window.session.update_query(query(2, "later"));
    poll_fn(|cx| {
        assert!(update.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(window.release.take());
    assert_eq!(
        timeout(WAIT, update).await.unwrap(),
        Err(window.error.clone())
    );
    window.assert_joined_and_reusable().await;
}

#[tokio::test]
async fn abandoned_rejected_query_retains_native_failure_and_joined_cleanup() {
    let mut window = FailureWindow::open().await;
    let mut update = window.session.update_query(query(2, "later"));
    poll_fn(|cx| {
        assert!(update.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(update);
    drop(window.release.take());
    window.assert_joined_and_reusable().await;
}
