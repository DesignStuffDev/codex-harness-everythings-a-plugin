use super::*;
use pretty_assertions::assert_eq;

fn options() -> FileSearchOptions {
    FileSearchOptions {
        threads: NonZero::new(1).unwrap(),
        ..FileSearchOptions::default()
    }
}

fn budget(entries: usize) -> SearchBudget {
    SearchBudget {
        max_index_entries: NonZero::new(entries).unwrap(),
        max_index_bytes: NonZero::new(8 * 1024 * 1024).unwrap(),
        max_worker_threads: NonZero::new(4).unwrap(),
    }
}

async fn wait_finished(session: &ManagedFileSearchSession) {
    timeout(WAIT, async {
        while !session.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abandoned_typed_close_retains_capacity_until_blocked_callback_joins_then_reuses_it() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (release, gate) = crossbeam_channel::bounded(1);
    let (reporter, mut updates) = new_reporter(Callback::Block(gate));
    let session = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .unwrap();
    let receipt_observer = session.clone();
    session.update_query_tagged("apple", 1).unwrap();
    snapshot(&mut updates, 1).await;
    session.request_close();
    let waiter = tokio::spawn(session.close_outcome());
    waiter.abort();
    let _ = waiter.await;
    let (other, _) = new_reporter(Callback::Observe);
    let denied = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), other)
        .await;
    let mut close = tokio::spawn(receipt_observer.close_outcome());
    let early = timeout(Duration::from_millis(50), &mut close).await;
    release.send(()).unwrap();
    let error = denied
        .err()
        .expect("a blocked close still owns its capacity");
    assert_eq!(
        (error.operation.kind(), error.cleanup),
        (
            SearchErrorKind::ResourceExhausted,
            StartCleanup::NotAdmitted
        )
    );
    assert!(
        early.is_err(),
        "no joined receipt may precede the callback's return"
    );
    assert_eq!(
        timeout(WAIT, close).await.unwrap().unwrap(),
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        }
    );
    let (reporter, mut updates) = new_reporter(Callback::Observe);
    let reused = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .unwrap();
    reused.update_query_tagged("banana", 1).unwrap();
    snapshot(&mut updates, 1).await;
    assert_eq!(
        reused.close_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        }
    );
    assert_eq!(
        owner.shutdown_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        }
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn joined_reporter_failure_keeps_its_typed_receipt_and_releases_capacity() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, mut updates) = new_reporter(Callback::Panic);
    let weak = Arc::downgrade(&reporter);
    let session = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .unwrap();
    let again = session.clone();
    session.update_query_tagged("apple", 1).unwrap();
    snapshot(&mut updates, 1).await;
    wait_finished(&session).await;
    let receipt = session.close_outcome().await;
    assert_eq!(
        receipt.operation.as_ref().unwrap_err().kind(),
        SearchErrorKind::SearchFailed
    );
    assert_eq!(receipt.cleanup, CloseCleanup::Joined);
    assert!(weak.upgrade().is_none());
    let (reporter, _) = new_reporter(Callback::Observe);
    let reused = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .unwrap();
    assert_eq!(again.close_outcome().await, receipt);
    assert_eq!(
        reused.close_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        }
    );
    assert_eq!(owner.shutdown_outcome().await, receipt);
    assert_eq!(owner.shutdown_outcome().await, receipt);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bounded_entry_exhaustion_retains_resource_kind_after_join_and_permits_reuse() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, _) = new_reporter(Callback::Observe);
    let operation = match owner
        .create_bounded(vec![root.path().into()], options(), budget(1), reporter)
        .await
    {
        Ok(session) => {
            wait_finished(&session).await;
            let receipt = session.close_outcome().await;
            assert_eq!(receipt.cleanup, CloseCleanup::Joined);
            receipt.operation.unwrap_err()
        }
        Err(error) => {
            // Exhaustion can race the startup acknowledgement. Either receipt
            // must certify actual joined work, not pretend it was never admitted.
            assert_eq!(error.cleanup, StartCleanup::Confirmed);
            error.operation
        }
    };
    assert_eq!(operation.kind(), SearchErrorKind::ResourceExhausted);
    let (reporter, mut updates) = new_reporter(Callback::Observe);
    let reused = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .unwrap();
    reused.update_query_tagged("apple", 1).unwrap();
    snapshot(&mut updates, 1).await;
    assert_eq!(
        reused.close_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        }
    );
    assert_eq!(
        owner.shutdown_outcome().await,
        SearchCloseOutcome {
            operation: Err(operation),
            cleanup: CloseCleanup::Joined,
        }
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_native_budget_has_not_admitted_receipt_and_does_not_strand_owner_slot() {
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, _) = new_reporter(Callback::Observe);
    let rejected = SearchBudget {
        max_index_bytes: NonZero::new(1).unwrap(),
        ..budget(32)
    };
    let error = owner
        .create_bounded(vec![root.path().into()], options(), rejected, reporter)
        .await
        .err()
        .expect("fixed native allocations cannot fit one charged byte");
    assert_eq!(
        (error.operation.kind(), error.cleanup),
        (
            SearchErrorKind::ResourceExhausted,
            StartCleanup::NotAdmitted
        )
    );
    let (reporter, _) = new_reporter(Callback::Observe);
    let session = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .unwrap();
    assert_eq!(
        session.close_outcome().await,
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        }
    );
    assert_eq!(owner.shutdown_outcome().await.cleanup, CloseCleanup::Joined);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn teardown_panic_preserves_first_native_failure_and_quarantines_capacity() {
    struct PanickingDropReporter {
        failures: mpsc::UnboundedSender<SearchError>,
        entered: Option<tokio::sync::oneshot::Sender<()>>,
        release: crossbeam_channel::Receiver<()>,
    }
    impl SessionReporter for PanickingDropReporter {
        fn on_update(&self, snapshot: &FileSearchSnapshot) {
            if snapshot.query_id != 0 {
                panic!("injected first native reporter failure");
            }
        }
        fn on_complete(&self) {}
        fn on_error(&self, error: &SearchError) {
            let _ = self.failures.send(error.clone());
        }
    }
    impl Drop for PanickingDropReporter {
        fn drop(&mut self) {
            let _ = self.entered.take().unwrap().send(());
            self.release.recv().unwrap();
            panic!("injected owner close task teardown failure");
        }
    }
    let root = tree();
    let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (entered, dropping) = tokio::sync::oneshot::channel();
    let (release, gate) = crossbeam_channel::bounded(1);
    let (failures, mut failed) = mpsc::unbounded_channel();
    let reporter = Arc::new(PanickingDropReporter {
        failures,
        entered: Some(entered),
        release: gate,
    });
    let weak = Arc::downgrade(&reporter);
    let session = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .unwrap();
    let repeated = session.clone();
    session.update_query_tagged("apple", 1).unwrap();
    let first_failure = timeout(WAIT, failed.recv()).await.unwrap().unwrap();
    wait_finished(&session).await;
    let mut close = tokio::spawn(session.close_outcome());
    timeout(WAIT, dropping).await.unwrap().unwrap();
    let early = timeout(Duration::from_millis(50), &mut close).await;
    release.send(()).unwrap();
    assert!(
        early.is_err(),
        "teardown must finish before its receipt is observed"
    );
    let receipt = timeout(WAIT, close).await.unwrap().unwrap();
    assert_eq!(receipt.operation, Err(first_failure));
    let CloseCleanup::Unconfirmed(cleanup) = &receipt.cleanup else {
        panic!("a lost close task receipt cannot certify joined native ownership");
    };
    assert_eq!(cleanup.kind(), SearchErrorKind::SearchFailed);
    assert!(weak.upgrade().is_none());
    assert_eq!(repeated.close_outcome().await, receipt);
    let (reporter, _) = new_reporter(Callback::Observe);
    let denied = owner
        .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
        .await
        .err()
        .expect("uncertain cleanup must quarantine capacity");
    assert_eq!(
        (denied.operation.kind(), denied.cleanup),
        (
            SearchErrorKind::ResourceExhausted,
            StartCleanup::NotAdmitted
        )
    );
    assert_eq!(owner.shutdown_outcome().await, receipt);
    assert_eq!(owner.shutdown_outcome().await, receipt);
    assert_eq!(owner.inner.state.lock().unwrap().entries.len(), 1);
}

#[test]
fn abandoned_bounded_start_waiter_keeps_ownership_until_actual_startup_and_cleanup() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let root = tree();
        let owner = FileSearchOwner::new(NonZero::new(1).unwrap());
        let (release, gate) = crossbeam_channel::bounded(1);
        let (entered, started) = crossbeam_channel::bounded(1);
        let blocker = tokio::task::spawn_blocking(move || {
            entered.send(()).unwrap();
            gate.recv().unwrap();
        });
        started.recv_timeout(WAIT).unwrap();
        let (reporter, _) = new_reporter(Callback::Observe);
        let weak = Arc::downgrade(&reporter);
        let mut pending = Box::pin(owner.create_bounded(
            vec![root.path().into()],
            options(),
            budget(32),
            reporter,
        ));
        std::future::poll_fn(|context| {
            assert!(std::future::Future::poll(pending.as_mut(), context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(pending);
        let (reporter, _) = new_reporter(Callback::Observe);
        let denied = owner
            .create_bounded(vec![root.path().into()], options(), budget(32), reporter)
            .await;
        owner.request_shutdown();
        release.send(()).unwrap();
        blocker.await.unwrap();
        let error = denied
            .err()
            .expect("unstarted admitted work retains capacity");
        assert_eq!(
            (error.operation.kind(), error.cleanup),
            (
                SearchErrorKind::ResourceExhausted,
                StartCleanup::NotAdmitted
            )
        );
        assert_eq!(
            timeout(WAIT, owner.shutdown_outcome()).await.unwrap(),
            SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined,
            }
        );
        assert!(weak.upgrade().is_none());
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legacy_constructor_cleanup_error_stays_conservative_while_typed_receipt_is_not_admitted() {
    let legacy = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, _) = new_reporter(Callback::Observe);
    let error = legacy
        .create(Vec::new(), options(), reporter, /*cancel_flag*/ None)
        .await
        .err()
        .expect("native construction rejects missing roots");
    let compatibility = error.downcast_ref::<FileSearchStartError>().unwrap();
    assert_eq!(
        compatibility
            .operation_error()
            .downcast_ref::<SearchError>()
            .unwrap()
            .kind(),
        SearchErrorKind::InvalidInput
    );
    assert_eq!(
        compatibility
            .cleanup_error()
            .unwrap()
            .downcast_ref::<SearchError>()
            .unwrap()
            .kind(),
        SearchErrorKind::InvalidInput
    );
    let typed = FileSearchOwner::new(NonZero::new(1).unwrap());
    let (reporter, _) = new_reporter(Callback::Observe);
    let error = typed
        .create_bounded(Vec::new(), options(), budget(32), reporter)
        .await
        .err()
        .expect("the typed constructor also rejects missing roots");
    assert_eq!(
        (error.operation.kind(), error.cleanup),
        (SearchErrorKind::InvalidInput, StartCleanup::NotAdmitted)
    );
    assert_eq!(
        legacy.shutdown_outcome().await.cleanup,
        CloseCleanup::Joined
    );
    assert_eq!(typed.shutdown_outcome().await.cleanup, CloseCleanup::Joined);
}
