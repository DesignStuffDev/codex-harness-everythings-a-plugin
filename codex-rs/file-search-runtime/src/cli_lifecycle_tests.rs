#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Production CLI/facade lifecycle checks at a controllable backend boundary.
//! These do not claim native worker or independently installed process proof.

use std::future::pending;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileMatch;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::MatchType;
use codex_file_search_api::PendingSearchStart;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::QueryAccepted;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchFrame;
use codex_file_search_api::SearchFuture;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use codex_file_search_api::SearchStartError;
use pretty_assertions::assert_eq;
use tokio::sync::Notify;
use tokio::sync::Semaphore;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::FileSearchProvider;
use crate::RuntimePolicy;

use super::run_selected_query;

fn count(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

fn budget() -> SearchBudget {
    SearchBudget {
        max_index_entries: count(16),
        max_index_bytes: count(4096),
        max_worker_threads: count(2),
    }
}

fn request() -> SearchOpen {
    SearchOpen {
        roots: vec![".".into()],
        options: FileSearchOptions {
            limit: count(4),
            threads: count(1),
            ..FileSearchOptions::default()
        },
        budget: budget(),
    }
}

fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}

async fn bounded<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .expect("lifecycle test deadline")
}

struct Fixture {
    provider: FileSearchProvider,
    backend: Arc<Backend>,
    session: Arc<Session>,
}

impl Fixture {
    fn new(close_outcome: SearchCloseOutcome, shutdown_outcome: SearchCloseOutcome) -> Self {
        let session = Arc::new(Session {
            frame: watch::channel(None).0,
            close_returned: watch::channel(false).0,
            closing: AtomicBool::new(false),
            update_active: AtomicBool::new(false),
            poll_active: AtomicBool::new(false),
            changed: Notify::new(),
            drained: Notify::new(),
            update_entered: Semaphore::new(0),
            poll_entered: Semaphore::new(0),
            close_entered: Semaphore::new(0),
            allow_update: Semaphore::new(1),
            allow_close: Semaphore::new(0),
            close_outcome,
        });
        let backend = Arc::new(Backend {
            session: Arc::clone(&session),
            start: crate::pending_fixture::SingleStart::default(),
            shutdown_entered: Semaphore::new(0),
            allow_shutdown: Semaphore::new(0),
            shutdown_outcome,
        });
        let policy = RuntimePolicy {
            provider: ProviderLimits {
                max_scopes: count(1),
                max_sessions: count(1),
                resources: budget(),
            },
            max_query_bytes: count(64),
            max_roots_options_bytes: count(4096),
            max_matches: count(4),
            max_frame_retained_bytes: count(4096),
            poll_wait: Duration::from_millis(20),
        };
        let provider = FileSearchProvider::from_backend(backend.clone(), policy).unwrap();
        Self {
            provider,
            backend,
            session,
        }
    }
}

struct Backend {
    session: Arc<Session>,
    start: crate::pending_fixture::SingleStart,
    shutdown_entered: Semaphore,
    allow_shutdown: Semaphore,
    shutdown_outcome: SearchCloseOutcome,
}

impl SearchBackend for Backend {
    fn begin_open(&self, _: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        let session = Arc::clone(&self.session);
        self.start.begin(
            Box::pin(async move { Ok(session as Arc<dyn SearchBackendSession>) }),
            crate::pending_fixture::FailureOrigin::Genuine,
            /*returned*/ None,
        )
    }

    fn request_shutdown(&self) {
        self.start.request_cancel();
        self.session.request_close();
    }

    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            self.request_shutdown();
            self.shutdown_entered.add_permits(1);
            self.allow_shutdown.acquire().await.unwrap().forget();
            if self.shutdown_outcome.cleanup == CloseCleanup::Joined {
                self.session.wait_inactive().await;
                self.session
                    .close_returned
                    .subscribe()
                    .wait_for(|done| *done)
                    .await
                    .unwrap();
            }
            self.start.completed().await;
            self.shutdown_outcome.clone()
        })
    }
}

struct Session {
    frame: watch::Sender<Option<SearchFrame>>,
    close_returned: watch::Sender<bool>,
    closing: AtomicBool,
    update_active: AtomicBool,
    poll_active: AtomicBool,
    changed: Notify,
    drained: Notify,
    update_entered: Semaphore,
    poll_entered: Semaphore,
    close_entered: Semaphore,
    allow_update: Semaphore,
    allow_close: Semaphore,
    close_outcome: SearchCloseOutcome,
}

impl Session {
    fn publish(&self, phase: SearchPhase) {
        let snapshot = (phase == SearchPhase::Idle).then(|| FileSearchSnapshot {
            query_id: 1,
            query: "needle".into(),
            matches: vec![],
            total_match_count: 0,
            scanned_file_count: 0,
            walk_complete: true,
        });
        self.frame.send_replace(Some(SearchFrame {
            revision: 1,
            query_id: 1,
            query: "needle".into(),
            snapshot,
            phase,
        }));
    }

    async fn wait_inactive(&self) {
        loop {
            let drained = self.drained.notified();
            tokio::pin!(drained);
            drained.as_mut().enable();
            if !self.update_active.load(Ordering::SeqCst)
                && !self.poll_active.load(Ordering::SeqCst)
            {
                return;
            }
            drained.await;
        }
    }
}

struct ActiveOperation<'a> {
    active: &'a AtomicBool,
    drained: &'a Notify,
}

impl Drop for ActiveOperation<'_> {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
        self.drained.notify_waiters();
    }
}

impl SearchBackendSession for Session {
    fn update_query(&self, query: SearchQuery) -> SearchFuture<'_, QueryAccepted> {
        Box::pin(async move {
            if self.closing.load(Ordering::SeqCst) {
                return Err(SearchError::new(
                    SearchErrorKind::ClosedLease,
                    "fixture update closed",
                ));
            }
            self.update_active.store(true, Ordering::SeqCst);
            let _active = ActiveOperation {
                active: &self.update_active,
                drained: &self.drained,
            };
            self.update_entered.add_permits(1);
            self.allow_update.acquire().await.unwrap().forget();
            Ok(QueryAccepted { id: query.id })
        })
    }

    fn next_snapshot(&self, after: u64, wait: Duration) -> SearchFuture<'_, SearchPoll> {
        Box::pin(async move {
            self.poll_active.store(true, Ordering::SeqCst);
            let _active = ActiveOperation {
                active: &self.poll_active,
                drained: &self.drained,
            };
            self.poll_entered.add_permits(1);
            let mut frames = self.frame.subscribe();
            let deadline = tokio::time::Instant::now() + wait;
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.closing.load(Ordering::SeqCst) {
                    return Err(SearchError::new(
                        SearchErrorKind::ClosedLease,
                        "fixture poll closed",
                    ));
                }
                if let Some(frame) = frames.borrow_and_update().clone()
                    && frame.revision > after
                {
                    return Ok(SearchPoll::Changed(frame));
                }
                tokio::select! {
                    _ = &mut changed => {},
                    _ = frames.changed() => {},
                    _ = tokio::time::sleep_until(deadline) => {
                        return Ok(SearchPoll::Unchanged { revision: after });
                    },
                }
            }
        })
    }

    fn request_close(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.changed.notify_waiters();
    }

    fn close(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            self.request_close();
            self.close_entered.add_permits(1);
            self.allow_close.acquire().await.unwrap().forget();
            if self.close_outcome.cleanup == CloseCleanup::Joined {
                self.wait_inactive().await;
            }
            self.close_returned.send_replace(true);
            self.close_outcome.clone()
        })
    }
}

#[tokio::test]
async fn terminal_cancelled_or_closed_without_idle_finishes_with_an_error() {
    for phase in [SearchPhase::Cancelled, SearchPhase::Closed] {
        let Fixture {
            provider,
            backend,
            session,
        } = Fixture::new(joined(), joined());
        let running = tokio::spawn(run_selected_query(
            provider,
            request(),
            "needle".into(),
            pending(),
        ));
        bounded(session.update_entered.acquire())
            .await
            .unwrap()
            .forget();
        session.publish(phase);
        bounded(session.close_entered.acquire())
            .await
            .unwrap()
            .forget();
        assert!(!running.is_finished());
        session.allow_close.add_permits(1);
        bounded(backend.shutdown_entered.acquire())
            .await
            .unwrap()
            .forget();
        assert!(!running.is_finished());
        backend.allow_shutdown.add_permits(1);
        let error = bounded(running).await.unwrap().unwrap_err();
        assert!(error.to_string().contains("closing"));
        assert!(!session.update_active.load(Ordering::SeqCst));
        assert!(!session.poll_active.load(Ordering::SeqCst));
    }
}

#[tokio::test]
async fn terminal_cleanup_preserves_operation_failure_and_uncertainty() {
    let operation = SearchError::new(SearchErrorKind::SearchFailed, "backend completion failed");
    let cleanup = SearchError::new(
        SearchErrorKind::TransportLost,
        "backend cleanup proof missing",
    );
    let outcome = SearchCloseOutcome {
        operation: Err(operation),
        cleanup: CloseCleanup::Unconfirmed(cleanup),
    };
    let Fixture {
        provider,
        backend,
        session,
    } = Fixture::new(outcome.clone(), outcome);
    let running = tokio::spawn(run_selected_query(
        provider,
        request(),
        "needle".into(),
        pending(),
    ));
    bounded(session.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    session.publish(SearchPhase::Cancelled);
    bounded(session.close_entered.acquire())
        .await
        .unwrap()
        .forget();
    session.allow_close.add_permits(1);
    bounded(backend.shutdown_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert!(!running.is_finished());
    backend.allow_shutdown.add_permits(1);
    let error = bounded(running).await.unwrap().unwrap_err().to_string();
    assert!(error.contains("backend completion failed"));
    assert!(error.contains("cleanup unconfirmed"));
    assert!(error.contains("backend cleanup proof missing"));
    bounded(session.wait_inactive()).await;
}

#[tokio::test]
async fn completed_idle_returns_its_snapshot_only_after_joined_cleanup() {
    let Fixture {
        provider,
        backend,
        session,
    } = Fixture::new(joined(), joined());
    let running = tokio::spawn(run_selected_query(
        provider,
        request(),
        "needle".into(),
        pending(),
    ));
    bounded(session.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    session.publish(SearchPhase::Idle);
    bounded(session.close_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert!(!running.is_finished());
    session.allow_close.add_permits(1);
    bounded(backend.shutdown_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert!(!running.is_finished());
    backend.allow_shutdown.add_permits(1);
    assert_eq!(
        bounded(running).await.unwrap().unwrap(),
        FileSearchSnapshot {
            query_id: 1,
            query: "needle".into(),
            matches: vec![],
            total_match_count: 0,
            scanned_file_count: 0,
            walk_complete: true,
        }
    );
}

#[tokio::test]
async fn interruption_after_accepted_update_waits_for_operation_and_cleanup() {
    let Fixture {
        provider,
        backend,
        session,
    } = Fixture::new(joined(), joined());
    session.allow_update.acquire().await.unwrap().forget();
    let (interrupt, interrupted) = oneshot::channel();
    let running = tokio::spawn(run_selected_query(
        provider,
        request(),
        "needle".into(),
        async move {
            interrupted.await.unwrap();
            Ok(())
        },
    ));
    bounded(session.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert!(session.update_active.load(Ordering::SeqCst));
    interrupt.send(()).unwrap();
    bounded(session.close_entered.acquire())
        .await
        .unwrap()
        .forget();
    bounded(backend.shutdown_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert!(!running.is_finished());
    session.allow_close.add_permits(1);
    backend.allow_shutdown.add_permits(1);
    tokio::task::yield_now().await;
    assert!(!running.is_finished());
    assert!(session.update_active.load(Ordering::SeqCst));
    session.allow_update.add_permits(1);
    let error = bounded(running).await.unwrap().unwrap_err();
    assert!(error.to_string().contains("interrupted"));
    assert!(!session.update_active.load(Ordering::SeqCst));
    assert!(!session.poll_active.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn interruption_after_accepted_poll_waits_for_joined_provider_receipt() {
    let Fixture {
        provider,
        backend,
        session,
    } = Fixture::new(joined(), joined());
    let (interrupt, interrupted) = oneshot::channel();
    let running = tokio::spawn(run_selected_query(
        provider,
        request(),
        "needle".into(),
        async move {
            interrupted.await.unwrap();
            Ok(())
        },
    ));
    bounded(session.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    bounded(session.poll_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert!(session.poll_active.load(Ordering::SeqCst));
    interrupt.send(()).unwrap();
    bounded(session.close_entered.acquire())
        .await
        .unwrap()
        .forget();
    bounded(backend.shutdown_entered.acquire())
        .await
        .unwrap()
        .forget();
    session.allow_close.add_permits(1);
    bounded(session.close_returned.subscribe().wait_for(|done| *done))
        .await
        .unwrap();
    assert!(!running.is_finished());
    backend.allow_shutdown.add_permits(1);
    let error = bounded(running).await.unwrap().unwrap_err();
    assert!(error.to_string().contains("interrupted"));
    assert!(!session.poll_active.load(Ordering::SeqCst));
}

#[tokio::test]
async fn requested_indices_missing_from_idle_snapshot_fail_after_cleanup() {
    let Fixture {
        provider,
        backend,
        session,
    } = Fixture::new(joined(), joined());
    let mut input = request();
    input.options.compute_indices = true;
    let running = tokio::spawn(run_selected_query(
        provider,
        input,
        "needle".into(),
        pending(),
    ));
    bounded(session.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    session.frame.send_replace(Some(SearchFrame {
        revision: 1,
        query_id: 1,
        query: "needle".into(),
        snapshot: Some(FileSearchSnapshot {
            query_id: 1,
            query: "needle".into(),
            matches: vec![FileMatch {
                score: 1,
                path: "needle.rs".into(),
                root: ".".into(),
                match_type: MatchType::File,
                indices: None,
            }],
            total_match_count: 1,
            scanned_file_count: 1,
            walk_complete: true,
        }),
        phase: SearchPhase::Idle,
    }));
    bounded(session.close_entered.acquire())
        .await
        .unwrap()
        .forget();
    session.allow_close.add_permits(1);
    bounded(backend.shutdown_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert!(!running.is_finished());
    backend.allow_shutdown.add_permits(1);
    let error = bounded(running).await.unwrap().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("omitted requested highlight indices")
    );
}

#[test]
fn interruption_exit_marker_requires_a_clean_joined_provider() {
    let clean = super::with_cleanup::<()>(Err(super::CliInterrupted.into()), joined()).unwrap_err();
    assert!(clean.downcast_ref::<super::CliInterrupted>().is_some());
    for outcome in [
        SearchCloseOutcome {
            operation: Err(SearchError::new(
                SearchErrorKind::SearchFailed,
                "retained operation failed",
            )),
            cleanup: CloseCleanup::Joined,
        },
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Unconfirmed(SearchError::new(
                SearchErrorKind::TransportLost,
                "cleanup proof lost",
            )),
        },
    ] {
        let failure =
            super::with_cleanup::<()>(Err(super::CliInterrupted.into()), outcome).unwrap_err();
        assert!(failure.downcast_ref::<super::CliInterrupted>().is_none());
        assert!(failure.to_string().contains("interrupted"));
    }
}
