#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Real native matching plus facade startup ownership; no installed worker proof.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_file_search::NativeBackendLimits;
use codex_file_search::NativeSearchBackend;
use codex_file_search_api::*;
use pretty_assertions::assert_eq;
use tokio::sync::Notify;
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::state::lock;
use crate::*;

const WAIT: Duration = Duration::from_secs(5);
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}
fn budget(n: usize) -> SearchBudget {
    SearchBudget {
        max_index_entries: nz(64 * n),
        max_index_bytes: nz(8 * 1024 * 1024 * n),
        max_worker_threads: nz(4 * n),
    }
}
fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("apple.txt"), "a").unwrap();
    root
}
fn request(root: &std::path::Path) -> SearchOpen {
    SearchOpen {
        roots: vec![root.to_path_buf()],
        budget: budget(1),
        options: FileSearchOptions {
            limit: nz(10),
            threads: nz(1),
            respect_gitignore: false,
            ..FileSearchOptions::default()
        },
    }
}
#[derive(Default)]
struct Reporter {
    snapshots: Mutex<Vec<FileSearchSnapshot>>,
    changed: Notify,
}
impl SessionReporter for Reporter {
    fn on_update(&self, value: &FileSearchSnapshot) {
        lock(&self.snapshots).push(value.clone());
        self.changed.notify_waiters();
    }
    fn on_complete(&self) {}
}
async fn query(session: &FileSearchSession, reporter: &Reporter, id: u64) {
    session
        .update_query(SearchQuery {
            id: id.try_into().unwrap(),
            text: "apple".into(),
        })
        .await
        .unwrap();
    let snapshot = timeout(WAIT, async {
        loop {
            let changed = reporter.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let found = lock(&reporter.snapshots)
                .iter()
                .find(|s| s.query_id == id && s.walk_complete && !s.matches.is_empty())
                .cloned();
            if let Some(found) = found {
                break found;
            }
            changed.await;
        }
    })
    .await
    .unwrap();
    assert_eq!(snapshot.query, "apple");
    assert_eq!(
        snapshot
            .matches
            .iter()
            .map(|m| m.path.clone())
            .collect::<Vec<_>>(),
        vec![std::path::PathBuf::from("apple.txt")]
    );
}
struct Gate {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}
impl Gate {
    fn wait(&self) {
        if let Some(entered) = lock(&self.entered).take() {
            let _ = entered.send(());
        }
        let _ = lock(&self.release).recv();
    }
}
struct Release(Option<std::sync::mpsc::Sender<()>>);
impl Release {
    fn now(&mut self) {
        drop(self.0.take());
    }
}
struct CountControl {
    inner: Arc<dyn SearchStartControl>,
    requests: AtomicUsize,
}
impl SearchStartControl for CountControl {
    fn request_cancel(&self) {
        self.requests.fetch_add(1, Ordering::SeqCst);
        self.inner.request_cancel();
    }
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static> {
        self.request_cancel();
        self.inner.cancel_and_wait()
    }
}
struct CountingNative {
    native: NativeSearchBackend,
    starts: AtomicUsize,
    controls: Mutex<Vec<Arc<CountControl>>>,
    gate: Mutex<Option<Arc<Gate>>>,
}
impl SearchBackend for CountingNative {
    fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let pending = self.native.begin_open(request)?;
        let control = Arc::new(CountControl {
            inner: pending.control(),
            requests: AtomicUsize::new(0),
        });
        lock(&self.controls).push(control.clone());
        let gate = lock(&self.gate).clone();
        if let Some(gate) = gate {
            gate.wait();
        }
        Ok(PendingSearchStart::new(control, pending.finish()))
    }
    fn request_shutdown(&self) {
        self.native.request_shutdown();
    }
    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.native.shutdown()
    }
}
fn setup(snapshot_limit: usize) -> (Arc<CountingNative>, FileSearchProvider, FileSearchScope) {
    let native = NativeSearchBackend::new(
        std::env::current_dir().unwrap(),
        NativeBackendLimits {
            max_sessions: nz(2),
            resources: budget(2),
            max_query_bytes: nz(256),
            max_roots_options_bytes: nz(65536),
            max_matches: nz(10),
            max_snapshot_bytes: nz(snapshot_limit),
            max_poll_wait: Duration::from_secs(1),
        },
    )
    .unwrap();
    let backend = Arc::new(CountingNative {
        native,
        starts: AtomicUsize::new(0),
        controls: Mutex::new(vec![]),
        gate: Mutex::new(None),
    });
    let provider = FileSearchProvider::from_backend(
        backend.clone(),
        RuntimePolicy {
            provider: ProviderLimits {
                max_scopes: nz(2),
                max_sessions: nz(2),
                resources: budget(2),
            },
            max_query_bytes: nz(256),
            max_roots_options_bytes: nz(65536),
            max_matches: nz(10),
            max_frame_retained_bytes: nz(128 * 1024),
            poll_wait: Duration::from_millis(20),
        },
    )
    .unwrap();
    let scope = provider
        .scope_factory()
        .new_scope(ScopeLimits {
            max_sessions: nz(2),
        })
        .unwrap();
    (backend, provider, scope)
}
fn first_lease(provider: &FileSearchProvider) -> Arc<crate::state::Lease> {
    lock(&provider.inner.state)
        .scopes
        .values()
        .next()
        .unwrap()
        .leases
        .values()
        .next()
        .unwrap()
        .clone()
}
enum Abandonment {
    Ticket,
    Finish,
}
async fn unpolled(abandonment: Abandonment) {
    let root = fixture();
    let (backend, provider, scope) = setup(65536);
    let pending = scope
        .begin_open(request(root.path()), Arc::new(Reporter::default()))
        .unwrap();
    let control = pending.control();
    match abandonment {
        Abandonment::Ticket => drop(pending),
        Abandonment::Finish => drop(pending.finish()),
    }
    assert!(lock(&first_lease(&provider).state).closing);
    assert_eq!(lock(&provider.inner.state).sessions, 1);
    assert_eq!(backend.starts.load(Ordering::SeqCst), 0);
    let expected = SearchStartCancellationOutcome {
        operation: Ok(()),
        cleanup: StartCleanup::NotAdmitted,
    };
    assert_eq!(
        timeout(WAIT, control.cancel_and_wait()).await.unwrap(),
        expected
    );
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(
        backend.starts.load(Ordering::SeqCst),
        0,
        "cancel before task execution must skip actual backend begin"
    );
    assert_eq!(lock(&provider.inner.state).sessions, 0);
    let reporter = Arc::new(Reporter::default());
    let session = scope
        .begin_open(request(root.path()), reporter.clone())
        .unwrap()
        .finish()
        .await
        .unwrap();
    query(&session, &reporter, 1).await;
    assert_eq!(backend.starts.load(Ordering::SeqCst), 1);
    assert_eq!(session.close().await, joined());
    assert_eq!(provider.shutdown().await, joined());
}
#[tokio::test]
async fn unpolled_runtime_ticket_skips_backend_and_reuses_quota_for_real_matching() {
    unpolled(Abandonment::Ticket).await;
}
#[tokio::test]
async fn unpolled_runtime_finish_skips_backend_and_reuses_quota_for_real_matching() {
    unpolled(Abandonment::Finish).await;
}

#[tokio::test]
async fn ready_before_public_observation_cancellation_preserves_real_sibling_and_replacement() {
    let root = fixture();
    let (backend, provider, scope) = setup(65536);
    let pending = scope
        .begin_open(request(root.path()), Arc::new(Reporter::default()))
        .unwrap();
    let control = pending.control();
    let lease = first_lease(&provider);
    timeout(WAIT, async {
        loop {
            let changed = lease.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if lock(&lease.state).start_finished {
                break;
            }
            changed.await;
        }
    })
    .await
    .unwrap();
    assert!(lock(&lease.state).backend.is_some());
    let sibling_reporter = Arc::new(Reporter::default());
    let sibling = scope
        .open(request(root.path()), sibling_reporter.clone())
        .await
        .unwrap();
    control.request_cancel();
    let error = timeout(WAIT, pending.finish())
        .await
        .unwrap()
        .err()
        .unwrap();
    assert_eq!(error.operation.kind(), SearchErrorKind::ClosedLease);
    assert_eq!(error.cleanup, StartCleanup::Confirmed);
    assert_eq!(
        control.cancel_and_wait().await,
        SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::Confirmed
        }
    );
    query(&sibling, &sibling_reporter, 1).await;
    let replacement_reporter = Arc::new(Reporter::default());
    let replacement = scope
        .open(request(root.path()), replacement_reporter.clone())
        .await
        .unwrap();
    control.request_cancel();
    query(&replacement, &replacement_reporter, 1).await;
    assert_eq!(backend.starts.load(Ordering::SeqCst), 3);
    assert_eq!(replacement.close().await, joined());
    assert_eq!(sibling.close().await, joined());
    assert_eq!(provider.shutdown().await, joined());
}

#[tokio::test]
async fn real_native_snapshot_failure_survives_cancel_and_is_joined_before_runtime_refund() {
    let root = fixture();
    let (_, provider, scope) = setup(1);
    let pending = scope
        .begin_open(request(root.path()), Arc::new(Reporter::default()))
        .unwrap();
    let control = pending.control();
    let lease = first_lease(&provider);
    let failure = timeout(WAIT, async {
        loop {
            let changed = lease.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let failure = lock(&lease.state).first_error.clone();
            if let Some(failure) = failure {
                break failure;
            }
            changed.await;
        }
    })
    .await
    .unwrap();
    assert_eq!(failure.kind(), SearchErrorKind::ResourceExhausted);
    control.request_cancel();
    let error = timeout(WAIT, pending.finish())
        .await
        .unwrap()
        .err()
        .unwrap();
    assert_eq!(
        error,
        SearchStartError {
            operation: failure.clone(),
            cleanup: StartCleanup::Confirmed
        }
    );
    let expected = SearchStartCancellationOutcome {
        operation: Err(failure.clone()),
        cleanup: StartCleanup::Confirmed,
    };
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(lock(&provider.inner.state).sessions, 0);
    assert_eq!(
        provider.shutdown().await,
        SearchCloseOutcome {
            operation: Err(failure),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[test]
fn actual_runtime_loss_before_first_start_poll_retains_uncertainty_and_quota() {
    let root = fixture();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (backend, provider, scope, pending, control) = {
        let _entered = runtime.enter();
        let (backend, provider, scope) = setup(65536);
        let pending = scope
            .begin_open(request(root.path()), Arc::new(Reporter::default()))
            .unwrap();
        let control = pending.control();
        (backend, provider, scope, pending, control)
    };
    runtime.shutdown_background();
    let observer = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    observer.block_on(async {
        let receipt = timeout(WAIT, control.cancel_and_wait()).await.unwrap();
        assert!(matches!(receipt.cleanup, StartCleanup::Unconfirmed(_)));
        assert_eq!(
            receipt.operation.as_ref().unwrap_err().kind(),
            SearchErrorKind::TransportLost
        );
        assert_eq!(control.cancel_and_wait().await, receipt);
        let error = timeout(WAIT, pending.finish())
            .await
            .unwrap()
            .err()
            .unwrap();
        assert_eq!(
            error,
            SearchStartError {
                operation: receipt.operation.clone().unwrap_err(),
                cleanup: receipt.cleanup.clone()
            }
        );
        assert_eq!(backend.starts.load(Ordering::SeqCst), 0);
        assert_eq!(lock(&provider.inner.state).sessions, 1);
        assert_eq!(
            lock(&provider.inner.state).used,
            crate::state::allocation(budget(1))
        );
        assert!(matches!(
            timeout(WAIT, scope.shutdown()).await.unwrap().cleanup,
            CloseCleanup::Unconfirmed(_)
        ));
        assert!(matches!(
            timeout(WAIT, provider.shutdown()).await.unwrap().cleanup,
            CloseCleanup::Unconfirmed(_)
        ));
    });
}

#[path = "pending_publication_tests.rs"]
mod publication;

#[path = "pending_failure_tests.rs"]
mod failures;
