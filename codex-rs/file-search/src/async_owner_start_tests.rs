#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::FileSearchSnapshot;
use crate::native_output::NativeOutputLimits;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

struct Reporter(mpsc::UnboundedSender<FileSearchSnapshot>);
impl SessionReporter for Reporter {
    fn on_update(&self, snapshot: &FileSearchSnapshot) {
        let _ = self.0.send(snapshot.clone());
    }
    fn on_complete(&self) {}
}

fn reporter() -> (Arc<Reporter>, mpsc::UnboundedReceiver<FileSearchSnapshot>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    (Arc::new(Reporter(sender)), receiver)
}

fn tree() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("apple.txt"), "a").unwrap();
    root
}

fn options() -> FileSearchOptions {
    FileSearchOptions {
        threads: NonZero::new(1).unwrap(),
        ..FileSearchOptions::default()
    }
}

fn budget() -> SearchBudget {
    SearchBudget {
        max_index_entries: NonZero::new(32).unwrap(),
        max_index_bytes: NonZero::new(8 * 1024 * 1024).unwrap(),
        max_worker_threads: NonZero::new(4).unwrap(),
    }
}

fn begin(
    owner: &FileSearchOwner,
    root: &std::path::Path,
    reporter: Arc<dyn SessionReporter>,
) -> PendingNativeSearchStart {
    owner
        .begin_create_backend(
            vec![root.to_path_buf()],
            options(),
            budget(),
            NativeOutputLimits {
                max_query_bytes: NonZero::new(1024).unwrap(),
                max_snapshot_bytes: NonZero::new(64 * 1024).unwrap(),
            },
            reporter,
        )
        .unwrap()
}

async fn query(
    session: &ManagedFileSearchSession,
    updates: &mut mpsc::UnboundedReceiver<FileSearchSnapshot>,
    id: u64,
) {
    session.update_query_tagged("apple", id).unwrap();
    let snapshot = timeout(WAIT, async {
        loop {
            let snapshot = updates.recv().await.unwrap();
            if snapshot.query_id == id && snapshot.walk_complete && !snapshot.matches.is_empty() {
                break snapshot;
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

/// Dropping the sender always unblocks native work, including test assertion unwind.
struct Release(Option<crossbeam_channel::Sender<()>>);
impl Release {
    fn now(&mut self) {
        drop(self.0.take());
    }
}

enum Abandonment {
    Ticket,
    FinishObserver,
}

fn queued_start_cancellation(abandonment: Abandonment) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let root = tree();
        let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
        let (send, gate) = crossbeam_channel::bounded::<()>(1);
        let mut release = Release(Some(send));
        let (entered, entering) = tokio::sync::oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            let _ = entered.send(());
            let _ = gate.recv();
        });
        timeout(WAIT, entering).await.unwrap().unwrap();
        let (startup_reporter, _) = reporter();
        let weak = Arc::downgrade(&startup_reporter);
        let pending = begin(&owner, root.path(), startup_reporter);
        let control = pending.control();
        match abandonment {
            Abandonment::Ticket => drop(pending),
            Abandonment::FinishObserver => drop(pending.finish()),
        }
        // No explicit control call precedes this assertion: abandonment alone
        // must latch cancellation even though the control remains alive.
        assert!(
            owner
                .inner
                .state
                .lock()
                .unwrap()
                .entries
                .values()
                .next()
                .unwrap()
                .shutdown
                .load(Ordering::Acquire)
        );
        let observer = control.cancel_and_wait();
        drop(observer);
        let retained = owner.inner.state.lock().unwrap().entries.len();
        let early = timeout(Duration::from_millis(30), control.cancel_and_wait()).await;
        release.now();
        blocker.await.unwrap();
        assert_eq!(retained, 1);
        assert!(early.is_err());
        let expected = SearchStartCancellationOutcome {
            operation: Ok(()),
            cleanup: StartCleanup::NotAdmitted,
        };
        assert_eq!(
            timeout(WAIT, control.cancel_and_wait()).await.unwrap(),
            expected
        );
        assert_eq!(control.cancel_and_wait().await, expected);
        assert!(weak.upgrade().is_none());
        let (reporter, mut updates) = reporter();
        let replacement = begin(&owner, root.path(), reporter).finish().await.unwrap();
        query(&replacement, &mut updates, 1).await;
        assert_eq!(
            replacement.close_outcome().await,
            SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined
            }
        );
        assert_eq!(
            owner.shutdown_outcome().await,
            SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined
            }
        );
    });
}

#[test]
fn dropped_unpolled_ticket_keeps_owner_until_no_admission_is_observed() {
    queued_start_cancellation(Abandonment::Ticket);
}

#[test]
fn dropped_unpolled_finish_observer_keeps_owner_until_no_admission_is_observed() {
    queued_start_cancellation(Abandonment::FinishObserver);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_reaches_real_pool_construction_and_leaves_sibling_queries_usable() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(2).unwrap());
    let (send, gate) = crossbeam_channel::bounded::<()>(1);
    let mut release = Release(Some(send));
    let (entered, entering) = tokio::sync::oneshot::channel();
    let entered = Mutex::new(Some(entered));
    *owner.inner.after_pool.lock().unwrap() = Some(Arc::new(move || {
        if let Some(entered) = entered.lock().unwrap().take() {
            let _ = entered.send(());
        }
        let _ = gate.recv();
    }));
    let (reporter_a, _) = reporter();
    let weak_a = Arc::downgrade(&reporter_a);
    let a = begin(&owner, root.path(), reporter_a);
    let control_a = a.control();
    timeout(WAIT, entering).await.unwrap().unwrap();
    *owner.inner.after_pool.lock().unwrap() = None;
    let (reporter_b, mut updates_b) = reporter();
    let b = begin(&owner, root.path(), reporter_b)
        .finish()
        .await
        .unwrap();
    control_a.request_cancel();
    query(&b, &mut updates_b, 1).await;
    let early = timeout(Duration::from_millis(30), control_a.cancel_and_wait()).await;
    let charged = owner.inner.state.lock().unwrap().entries.len();
    release.now();
    assert!(
        early.is_err(),
        "pool constructor must really return and join"
    );
    assert_eq!(charged, 2);
    let error = timeout(WAIT, a.finish()).await.unwrap().err().unwrap();
    assert_eq!(
        (error.operation.kind(), error.cleanup),
        (SearchErrorKind::ClosedLease, StartCleanup::Confirmed)
    );
    let expected = SearchStartCancellationOutcome {
        operation: Ok(()),
        cleanup: StartCleanup::Confirmed,
    };
    assert_eq!(control_a.cancel_and_wait().await, expected);
    assert_eq!(control_a.cancel_and_wait().await, expected);
    assert!(weak_a.upgrade().is_none());
    query(&b, &mut updates_b, 2).await;
    assert_eq!(
        b.close_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
    assert_eq!(
        owner.shutdown_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_closure_never_sets_a_shared_external_cancel_flag() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(2).unwrap());
    let shared = Arc::new(AtomicBool::new(false));
    let (reporter_a, _) = reporter();
    let a = owner
        .create(
            vec![root.path().into()],
            options(),
            reporter_a,
            Some(shared.clone()),
        )
        .await
        .unwrap();
    let (reporter_b, mut updates_b) = reporter();
    let b = owner
        .create(
            vec![root.path().into()],
            options(),
            reporter_b,
            Some(shared.clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        a.close_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
    assert_eq!(shared.load(Ordering::Acquire), false);
    query(&b, &mut updates_b, 1).await;
    assert_eq!(
        b.close_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
    assert_eq!(shared.load(Ordering::Acquire), false);
    assert_eq!(
        owner.shutdown_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined
        }
    );
}

#[derive(Clone, Copy)]
enum DropOutcome {
    Return,
    Panic,
}

struct GatedDrop {
    outcome: DropOutcome,
    entered: Option<tokio::sync::oneshot::Sender<()>>,
    release: crossbeam_channel::Receiver<()>,
}
impl SessionReporter for GatedDrop {
    fn on_update(&self, _: &FileSearchSnapshot) {}
    fn on_complete(&self) {}
}
impl Drop for GatedDrop {
    fn drop(&mut self) {
        if let Some(entered) = self.entered.take() {
            let _ = entered.send(());
        }
        let _ = self.release.recv();
        if matches!(self.outcome, DropOutcome::Panic) {
            panic!("injected constructor reporter teardown panic");
        }
    }
}

async fn budget_failure_during_teardown(outcome: DropOutcome) {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (send, gate) = crossbeam_channel::bounded::<()>(1);
    let mut release = Release(Some(send));
    let (entered, entering) = tokio::sync::oneshot::channel();
    let reporter = Arc::new(GatedDrop {
        outcome,
        entered: Some(entered),
        release: gate,
    });
    let pending = owner
        .begin_create_inner(
            vec![root.path().into()],
            options(),
            reporter,
            /*cancel_flag*/ None,
            NativePolicy::Bounded(SearchBudget {
                max_index_bytes: NonZero::new(1).unwrap(),
                ..budget()
            }),
        )
        .unwrap_or_else(|_| panic!("owner must admit construction"));
    let control = pending.control.clone();
    timeout(WAIT, entering).await.unwrap().unwrap();
    control.request_cancel();
    let early = timeout(Duration::from_millis(30), control.cancel_and_wait()).await;
    release.now();
    assert!(early.is_err());
    let error = match pending.finish().await {
        Err(CreateFailure::Accepted { error, .. }) => error,
        Err(CreateFailure::Rejected(_)) | Ok(_) => {
            panic!("the admitted constructor must report its real error")
        }
    };
    assert_eq!(error.operation.kind(), SearchErrorKind::ResourceExhausted);
    let close_cleanup = match outcome {
        DropOutcome::Return => {
            assert_eq!(error.cleanup, StartCleanup::NotAdmitted);
            CloseCleanup::Joined
        }
        DropOutcome::Panic => {
            let StartCleanup::Unconfirmed(cleanup) = &error.cleanup else {
                panic!("teardown panic cannot certify constructor cleanup");
            };
            assert_eq!(cleanup.kind(), SearchErrorKind::SearchFailed);
            assert!(cleanup.to_string().contains("startup task failed"));
            assert_eq!(owner.inner.state.lock().unwrap().entries.len(), 1);
            CloseCleanup::Unconfirmed(cleanup.clone())
        }
    };
    let expected = SearchStartCancellationOutcome {
        operation: Err(error.operation.clone()),
        cleanup: error.cleanup.clone(),
    };
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(control.cancel_and_wait().await, expected);
    assert_eq!(
        owner.shutdown_outcome().await,
        SearchCloseOutcome {
            operation: Err(error.operation),
            cleanup: close_cleanup
        }
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_constructor_budget_error_survives_cancellation_during_reporter_teardown() {
    budget_failure_during_teardown(DropOutcome::Return).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_constructor_budget_error_survives_teardown_panic_and_quarantines_capacity() {
    budget_failure_during_teardown(DropOutcome::Panic).await;
}

#[test]
fn released_ready_native_result_retains_real_failure_in_start_error() {
    struct Errors(crossbeam_channel::Sender<SearchError>);
    impl SessionReporter for Errors {
        fn on_update(&self, _: &FileSearchSnapshot) {}
        fn on_complete(&self) {}
        fn on_error(&self, error: &SearchError) {
            let _ = self.0.send(error.clone());
        }
    }
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (failed, errors) = crossbeam_channel::bounded(1);
    let (pending, control, native_failure) = {
        let _entered = runtime.enter();
        let pending = owner
            .begin_create_backend(
                vec![root.path().into()],
                options(),
                budget(),
                NativeOutputLimits {
                    max_query_bytes: NonZero::new(1024).unwrap(),
                    max_snapshot_bytes: NonZero::new(1).unwrap(),
                },
                Arc::new(Errors(failed)),
            )
            .unwrap();
        let control = pending.control();
        // The actual matcher can fail its bounded initial snapshot only after
        // supervisor readiness. The current-thread result observer has never
        // polled, so its owner entry is still Preparing regardless of OS timing.
        let native_failure = errors.recv_timeout(WAIT).unwrap();
        control.request_cancel();
        assert!(matches!(
            owner
                .inner
                .state
                .lock()
                .unwrap()
                .entries
                .values()
                .next()
                .unwrap()
                .phase,
            Phase::Preparing { released: true }
        ));
        (pending, control, native_failure)
    };
    assert_eq!(native_failure.kind(), SearchErrorKind::ResourceExhausted);
    runtime.block_on(async {
        let error = timeout(WAIT, pending.finish())
            .await
            .unwrap()
            .err()
            .unwrap();
        assert_eq!(
            error,
            SearchStartError {
                operation: native_failure.clone(),
                cleanup: StartCleanup::Confirmed
            }
        );
        assert_eq!(
            control.cancel_and_wait().await,
            SearchStartCancellationOutcome {
                operation: Err(native_failure.clone()),
                cleanup: StartCleanup::Confirmed,
            }
        );
        assert_eq!(
            owner.shutdown_outcome().await,
            SearchCloseOutcome {
                operation: Err(native_failure),
                cleanup: CloseCleanup::Joined,
            }
        );
    });
}

#[test]
fn destroyed_runtime_publishes_unconfirmed_for_unpolled_start_owner_without_refunding_capacity() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let (send, gate) = crossbeam_channel::bounded::<()>(1);
    let mut release = Release(Some(send));
    let (entered, entering) = crossbeam_channel::bounded(1);
    let (pending, control, blocker, weak) = {
        let _entered_runtime = runtime.enter();
        let blocker = runtime.spawn_blocking(move || {
            entered.send(()).unwrap();
            let _ = gate.recv();
        });
        entering.recv_timeout(WAIT).unwrap();
        let (reporter, _) = reporter();
        let weak = Arc::downgrade(&reporter);
        let pending = begin(&owner, root.path(), reporter);
        let control = pending.control();
        (pending, control, blocker, weak)
    };
    // No executor turn ever polled the accepted result observer.
    runtime.shutdown_background();
    release.now();
    let recovery = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    recovery.block_on(async {
        timeout(WAIT, blocker).await.unwrap().unwrap();
        let receipt = timeout(WAIT, control.cancel_and_wait()).await.unwrap();
        assert!(matches!(receipt.cleanup, StartCleanup::Unconfirmed(_)));
        assert_eq!(
            receipt.operation.as_ref().unwrap_err().kind(),
            SearchErrorKind::ForcedShutdown
        );
        let error = timeout(WAIT, pending.finish())
            .await
            .unwrap()
            .err()
            .unwrap();
        assert_eq!(
            (error.operation, error.cleanup),
            (
                receipt.operation.clone().unwrap_err(),
                receipt.cleanup.clone()
            )
        );
        assert_eq!(control.cancel_and_wait().await, receipt);
        {
            let state = owner.inner.state.lock().unwrap();
            assert_eq!(state.entries.len(), 1);
            assert!(matches!(
                state.entries.values().next().unwrap().phase,
                Phase::Quarantined
            ));
        }
        assert!(matches!(
            owner.shutdown_outcome().await.cleanup,
            CloseCleanup::Unconfirmed(_)
        ));
    });
    // Shutdown background still owns accepted blocking teardown. Verify its
    // callback storage eventually disappears instead of treating it as joined.
    let until = std::time::Instant::now() + WAIT;
    while weak.upgrade().is_some() && std::time::Instant::now() < until {
        std::thread::yield_now();
    }
    assert!(weak.upgrade().is_none());
}

#[test]
fn destroyed_runtime_publishes_unconfirmed_for_unpolled_close_owner() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (send, gate) = crossbeam_channel::bounded::<()>(1);
    let mut release = Release(Some(send));
    let (entered, entering) = tokio::sync::oneshot::channel();
    let reporter = Arc::new(GatedDrop {
        outcome: DropOutcome::Return,
        entered: Some(entered),
        release: gate,
    });
    let weak = Arc::downgrade(&reporter);
    let (session, control) = runtime.block_on(async {
        let pending = begin(&owner, root.path(), reporter);
        let control = pending.control();
        (pending.finish().await.unwrap(), control)
    });
    {
        let _entered_runtime = runtime.enter();
        session.request_close();
    }
    // No executor turn polls the close result owner scheduled above.
    runtime.shutdown_background();
    release.now();
    let recovery = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    recovery.block_on(async {
        timeout(WAIT, entering).await.unwrap().unwrap();
        let receipt = timeout(WAIT, session.close_outcome()).await.unwrap();
        assert!(matches!(receipt.cleanup, CloseCleanup::Unconfirmed(_)));
        assert_eq!(
            receipt.operation.as_ref().unwrap_err().kind(),
            SearchErrorKind::ForcedShutdown
        );
        assert_eq!(
            control.cancel_and_wait().await,
            SearchStartCancellationOutcome {
                operation: receipt.operation.clone(),
                cleanup: receipt.cleanup.clone().into(),
            }
        );
        assert_eq!(owner.shutdown_outcome().await, receipt);
        assert_eq!(owner.inner.state.lock().unwrap().entries.len(), 1);
    });
    let until = std::time::Instant::now() + WAIT;
    while weak.upgrade().is_some() && std::time::Instant::now() < until {
        std::thread::yield_now();
    }
    assert!(weak.upgrade().is_none());
}
