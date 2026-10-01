#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use pretty_assertions::assert_eq;
use std::num::NonZeroUsize;
use std::time::Duration;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

fn nz(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

fn budget() -> SearchBudget {
    SearchBudget {
        max_index_entries: nz(64),
        max_index_bytes: nz(8 * 1024 * 1024),
        max_worker_threads: nz(4),
    }
}

fn limits(capacity: usize) -> NativeBackendLimits {
    NativeBackendLimits {
        max_sessions: nz(capacity),
        resources: SearchBudget {
            max_index_entries: nz(64 * capacity),
            max_index_bytes: nz(8 * 1024 * 1024 * capacity),
            max_worker_threads: nz(4 * capacity),
        },
        max_query_bytes: nz(256),
        max_roots_options_bytes: nz(64 * 1024),
        max_matches: nz(10),
        max_snapshot_bytes: nz(64 * 1024),
        max_poll_wait: Duration::from_secs(1),
    }
}

fn backend(capacity: usize) -> NativeSearchBackend {
    NativeSearchBackend::new(std::env::current_dir().unwrap(), limits(capacity)).unwrap()
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("apple.txt"), "a").unwrap();
    root
}

fn request(root: &std::path::Path) -> SearchOpen {
    SearchOpen {
        roots: vec![root.to_path_buf()],
        budget: budget(),
        options: FileSearchOptions {
            limit: nz(10),
            threads: nz(1),
            respect_gitignore: false,
            ..FileSearchOptions::default()
        },
    }
}

async fn query(session: &dyn SearchBackendSession, id: u64) {
    session
        .update_query(SearchQuery {
            id: id.try_into().unwrap(),
            text: "apple".into(),
        })
        .await
        .unwrap();
    let snapshot = timeout(WAIT, async {
        let mut revision = 0;
        loop {
            match session
                .next_snapshot(revision, Duration::from_millis(50))
                .await
                .unwrap()
            {
                SearchPoll::Unchanged { .. } => {}
                SearchPoll::Changed(frame) => {
                    revision = frame.revision;
                    if frame.query_id == id && frame.phase == SearchPhase::Idle {
                        break frame.snapshot.unwrap();
                    }
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(snapshot.query, "apple");
    assert_eq!(
        snapshot
            .matches
            .iter()
            .map(|item| item.path.clone())
            .collect::<Vec<_>>(),
        vec![PathBuf::from("apple.txt")]
    );
}

fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}

struct Release(Option<crossbeam_channel::Sender<()>>);
impl Release {
    fn now(&mut self) {
        drop(self.0.take());
    }
}

fn gate() -> (Arc<dyn Fn() + Send + Sync>, oneshot::Receiver<()>, Release) {
    let (entered, entering) = oneshot::channel();
    let entered = Mutex::new(Some(entered));
    let (send, blocked) = crossbeam_channel::bounded::<()>(1);
    let callback = Arc::new(move || {
        if let Some(entered) = lock(&entered).take() {
            let _ = entered.send(());
        }
        let _ = blocked.recv();
    });
    (callback, entering, Release(Some(send)))
}

enum Abandonment {
    Ticket,
    Finish,
}

async fn abandon_before_start(abandonment: Abandonment) {
    let root = fixture();
    let backend = backend(1);
    let pending = backend.begin_open(request(root.path())).unwrap();
    let control = pending.control();
    match abandonment {
        Abandonment::Ticket => drop(pending),
        Abandonment::Finish => drop(pending.finish()),
    }
    // Current-thread test execution has not yielded: Drop alone must fence the
    // admitted identity even while its cancellation control remains alive.
    assert!(
        lock(&backend.inner.state)
            .sessions
            .values()
            .next()
            .unwrap()
            .is_closing()
    );
    assert!(backend.begin_open(request(root.path())).is_err());
    let expected = SearchStartCancellationOutcome {
        operation: Ok(()),
        cleanup: StartCleanup::NotAdmitted,
    };
    assert_eq!(
        timeout(WAIT, control.cancel_and_wait()).await.unwrap(),
        expected
    );
    assert_eq!(control.cancel_and_wait().await, expected);
    let replacement = backend
        .begin_open(request(root.path()))
        .unwrap()
        .finish()
        .await
        .unwrap();
    query(replacement.as_ref(), 1).await;
    assert_eq!(replacement.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
}

#[tokio::test]
async fn dropped_pending_ticket_cancels_before_native_admission() {
    abandon_before_start(Abandonment::Ticket).await;
}

#[tokio::test]
async fn dropped_unpolled_finish_cancels_before_native_admission() {
    abandon_before_start(Abandonment::Finish).await;
}

#[tokio::test]
async fn unpolled_cancel_observer_latches_intent_and_retains_its_receipt() {
    let root = fixture();
    let backend = backend(1);
    let pending = backend.begin_open(request(root.path())).unwrap();
    let control = pending.control();
    drop(control.cancel_and_wait());
    assert!(
        lock(&backend.inner.state)
            .sessions
            .values()
            .next()
            .unwrap()
            .is_closing()
    );
    let error = pending.finish().await.err().unwrap();
    assert_eq!(
        error,
        SearchStartError {
            operation: closed(),
            cleanup: StartCleanup::NotAdmitted
        }
    );
    assert_eq!(
        control.cancel_and_wait().await,
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::NotAdmitted,
        }
    );
    assert_eq!(backend.shutdown().await, joined());
}

#[test]
fn cancellation_before_pending_control_publication_reaches_queued_native_constructor() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let root = fixture();
        let backend = backend(2);
        let sibling = backend
            .begin_open(request(root.path()))
            .unwrap()
            .finish()
            .await
            .unwrap();
        let (send, blocked) = crossbeam_channel::bounded::<()>(1);
        let mut allow_constructor = Release(Some(send));
        let (entered, entering) = oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            let _ = entered.send(());
            let _ = blocked.recv();
        });
        timeout(WAIT, entering).await.unwrap().unwrap();
        let (hook, entering, mut publish_control) = gate();
        *lock(&backend.inner.before_pending) = Some(hook);
        let pending = backend.begin_open(request(root.path())).unwrap();
        *lock(&backend.inner.before_pending) = None;
        let control = pending.control();
        timeout(WAIT, entering).await.unwrap().unwrap();
        // Owner admission happened, but NativeSession cannot see its control yet.
        control.request_cancel();
        publish_control.now();
        query(sibling.as_ref(), 1).await;
        let early = timeout(Duration::from_millis(30), control.cancel_and_wait()).await;
        let charged = lock(&backend.inner.state).sessions.len();
        allow_constructor.now();
        blocker.await.unwrap();
        assert!(
            early.is_err(),
            "accepted constructor/result ownership must drain"
        );
        assert_eq!(charged, 2);
        let error = timeout(WAIT, pending.finish())
            .await
            .unwrap()
            .err()
            .unwrap();
        assert_eq!(
            (error.operation.kind(), error.cleanup),
            (SearchErrorKind::ClosedLease, StartCleanup::NotAdmitted)
        );
        let expected = SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::NotAdmitted,
        };
        assert_eq!(control.cancel_and_wait().await, expected);
        assert_eq!(control.cancel_and_wait().await, expected);
        // Without forwarding into the pending owner, this would construct native
        // workers then close a ready session and return Confirmed, not NotAdmitted.
        query(sibling.as_ref(), 2).await;
        assert!(!lock(&backend.inner.state).stopping);
        assert_eq!(sibling.close().await, joined());
        assert_eq!(backend.shutdown().await, joined());
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_before_ready_handoff_closes_without_returning_a_public_session() {
    let root = fixture();
    let backend = backend(2);
    let sibling = backend
        .begin_open(request(root.path()))
        .unwrap()
        .finish()
        .await
        .unwrap();
    let (hook, entering, mut handoff) = gate();
    *lock(&backend.inner.before_handoff) = Some(hook);
    let pending = backend.begin_open(request(root.path())).unwrap();
    *lock(&backend.inner.before_handoff) = None;
    let control = pending.control();
    let finish = tokio::spawn(pending.finish());
    timeout(WAIT, entering).await.unwrap().unwrap();
    control.request_cancel();
    query(sibling.as_ref(), 1).await;
    handoff.now();
    let error = timeout(WAIT, finish).await.unwrap().unwrap().err().unwrap();
    assert_eq!(
        error,
        SearchStartError {
            operation: closed(),
            cleanup: StartCleanup::Confirmed
        }
    );
    assert_eq!(
        control.cancel_and_wait().await,
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed
        }
    );
    assert_eq!(sibling.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
}

#[tokio::test]
async fn control_after_handoff_targets_only_original_lease_and_not_its_replacement() {
    let root = fixture();
    let backend = backend(2);
    let pending = backend.begin_open(request(root.path())).unwrap();
    let old_control = pending.control();
    let original = pending.finish().await.unwrap();
    let sibling = backend
        .begin_open(request(root.path()))
        .unwrap()
        .finish()
        .await
        .unwrap();
    let expected = SearchStartCancellationOutcome {
        operation: Ok(()),
        cleanup: StartCleanup::Confirmed,
    };
    assert_eq!(old_control.cancel_and_wait().await, expected);
    assert_eq!(original.close().await, joined());
    let replacement = backend
        .begin_open(request(root.path()))
        .unwrap()
        .finish()
        .await
        .unwrap();
    old_control.request_cancel();
    assert_eq!(old_control.cancel_and_wait().await, expected);
    query(sibling.as_ref(), 1).await;
    query(replacement.as_ref(), 1).await;
    assert_eq!(sibling.close().await, joined());
    assert_eq!(replacement.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
}

#[tokio::test]
async fn real_native_snapshot_failure_survives_cancel_before_public_observer() {
    let root = fixture();
    let mut limits = limits(1);
    limits.max_snapshot_bytes = nz(1);
    let backend = NativeSearchBackend::new(std::env::current_dir().unwrap(), limits).unwrap();
    let pending = backend.begin_open(request(root.path())).unwrap();
    let control = pending.control();
    let native = lock(&backend.inner.state)
        .sessions
        .values()
        .next()
        .unwrap()
        .clone();
    let failure = timeout(WAIT, async {
        loop {
            if let Some(failure) = lock(&native.state).failure.clone() {
                break failure;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    control.request_cancel();
    let error = timeout(WAIT, pending.finish())
        .await
        .unwrap()
        .err()
        .unwrap();
    assert_eq!(failure.kind(), SearchErrorKind::ResourceExhausted);
    assert_eq!(
        error,
        SearchStartError {
            operation: failure.clone(),
            cleanup: StartCleanup::Confirmed
        }
    );
    assert_eq!(
        control.cancel_and_wait().await,
        SearchStartCancellationOutcome {
            operation: Err(failure.clone()),
            cleanup: StartCleanup::Confirmed,
        }
    );
    assert_eq!(
        backend.shutdown().await,
        SearchCloseOutcome {
            operation: Err(failure),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[test]
fn destroyed_runtime_leaves_unpolled_backend_start_receipt_unconfirmed_and_quarantined() {
    let root = fixture();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (backend, pending, control) = {
        let _entered = runtime.enter();
        let backend = backend(1);
        let pending = backend.begin_open(request(root.path())).unwrap();
        let control = pending.control();
        (backend, pending, control)
    };
    runtime.shutdown_background();
    let recovery = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    recovery.block_on(async {
        let outcome = timeout(WAIT, control.cancel_and_wait()).await.unwrap();
        assert!(matches!(outcome.cleanup, StartCleanup::Unconfirmed(_)));
        assert!(outcome.operation.is_err());
        assert_eq!(control.cancel_and_wait().await, outcome);
        let error = timeout(WAIT, pending.finish())
            .await
            .unwrap()
            .err()
            .unwrap();
        assert_eq!(
            error,
            SearchStartError {
                operation: outcome.operation.clone().unwrap_err(),
                cleanup: outcome.cleanup.clone()
            }
        );
        assert_eq!(lock(&backend.inner.state).sessions.len(), 1);
        assert_eq!(lock(&backend.inner.state).used, amounts(budget()));
        let shutdown = timeout(WAIT, backend.shutdown()).await.unwrap();
        assert!(matches!(shutdown.cleanup, CloseCleanup::Unconfirmed(_)));
        assert_eq!(shutdown.operation, outcome.operation);
    });
}
