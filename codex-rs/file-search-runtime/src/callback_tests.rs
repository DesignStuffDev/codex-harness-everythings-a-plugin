use super::*;
use pretty_assertions::assert_eq;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use tokio::sync::Semaphore;

struct CallbackGate {
    entered: Semaphore,
    released: Mutex<bool>,
    condition: Condvar,
    completions: AtomicUsize,
}
impl CallbackGate {
    fn release(&self) {
        *lock(&self.released) = true;
        self.condition.notify_all();
    }
}
impl SessionReporter for CallbackGate {
    fn on_update(&self, _: &FileSearchSnapshot) {
        self.entered.add_permits(1);
        let mut released = lock(&self.released);
        while !*released {
            released = self.condition.wait(released).unwrap();
        }
    }
    fn on_complete(&self) {
        self.completions.fetch_add(1, Ordering::SeqCst);
    }
}
struct ReleaseCallback(Arc<CallbackGate>);
impl Drop for ReleaseCallback {
    fn drop(&mut self) {
        self.0.release();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn joined_close_waits_for_actual_running_callback_and_fences_completion() {
    let native = Session::new(1, joined());
    let provider = provider(Backend::new(vec![Arc::clone(&native)], 1));
    let scope = scope(&provider);
    let reporter = Arc::new(CallbackGate {
        entered: Semaphore::new(0),
        released: Mutex::new(false),
        condition: Condvar::new(),
        completions: AtomicUsize::new(0),
    });
    let release = ReleaseCallback(Arc::clone(&reporter));
    let session = scope.open(request(), reporter.clone()).await.unwrap();
    session.update_query(query(1, "blocked")).await.unwrap();
    native.emit(1, 1, "blocked", true);
    bounded(reporter.entered.acquire()).await.unwrap().forget();
    assert!(
        tokio::time::timeout(Duration::from_millis(30), scope.shutdown())
            .await
            .is_err()
    );
    assert_eq!(lock(&provider.inner.state).sessions, 1);
    drop(release);
    assert_eq!(bounded(scope.shutdown()).await, joined());
    assert_eq!(reporter.completions.load(Ordering::SeqCst), 0);
    assert_eq!(native.close_count.load(Ordering::SeqCst), 1);
    assert_eq!(bounded(provider.shutdown()).await, joined());
}

struct PanicReporter(Arc<Reporter>);
impl SessionReporter for PanicReporter {
    fn on_update(&self, _: &FileSearchSnapshot) {
        panic!("controlled callback panic");
    }
    fn on_complete(&self) {
        panic!("completion after callback failure");
    }
    fn on_error(&self, error: &SearchError) {
        self.0.on_error(error);
    }
}
#[tokio::test]
async fn callback_panic_becomes_retained_failure_with_joined_backend_cleanup() {
    let native = Session::new(1, joined());
    let provider = provider(Backend::new(vec![Arc::clone(&native)], 1));
    let scope = scope(&provider);
    let observed = Arc::new(Reporter::default());
    let session = scope
        .open(request(), Arc::new(PanicReporter(Arc::clone(&observed))))
        .await
        .unwrap();
    session.update_query(query(1, "panic")).await.unwrap();
    native.emit(1, 1, "panic", true);
    observed.wait_len(1).await;
    let outcome = bounded(session.close()).await;
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert!(
        matches!(outcome.operation, Err(error) if error.kind() == SearchErrorKind::SearchFailed)
    );
    assert_eq!(
        *lock(&observed.events),
        vec![Event::Error(SearchErrorKind::SearchFailed)]
    );
    assert_eq!(lock(&provider.inner.state).sessions, 0);
    let outcome = bounded(provider.shutdown()).await;
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert!(outcome.operation.is_err());
}

#[tokio::test]
async fn unconfirmed_reservation_waits_for_separate_provider_recovery_receipt() {
    let unconfirmed = SearchCloseOutcome {
        operation: Err(failure()),
        cleanup: CloseCleanup::Unconfirmed(failure()),
    };
    let native = Session::new(1, unconfirmed.clone());
    let backend = Backend::new(vec![native], 1);
    backend.shutdowns.acquire().await.unwrap().forget();
    let provider = provider(Arc::clone(&backend));
    let scope = scope(&provider);
    let session = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    assert_eq!(bounded(session.close()).await, unconfirmed);
    bounded(backend.shutdown_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert_eq!(lock(&provider.inner.state).sessions, 1);
    assert_eq!(lock(&provider.inner.state).used, [10, 4096, 6]);
    assert_eq!(bounded(scope.shutdown()).await, unconfirmed);
    backend.shutdowns.add_permits(1);
    assert_eq!(
        bounded(provider.shutdown()).await,
        SearchCloseOutcome {
            operation: Err(failure()),
            cleanup: CloseCleanup::Joined
        }
    );
    assert_eq!(lock(&provider.inner.state).sessions, 0);
    assert_eq!(bounded(session.close()).await, unconfirmed); // old receipt never rewritten
    assert_eq!(bounded(scope.shutdown()).await, unconfirmed);
}

#[tokio::test]
async fn receiver_backing_capacity_overflow_reports_error_without_success_callback() {
    let native = Session::new(1, joined());
    let provider = provider(Backend::new(vec![Arc::clone(&native)], 1));
    let scope = scope(&provider);
    let reporter = Arc::new(Reporter::default());
    let session = scope.open(request(), reporter.clone()).await.unwrap();
    session.update_query(query(1, "tiny")).await.unwrap();
    let mut huge_capacity = String::with_capacity(8192);
    huge_capacity.push_str("tiny");
    native.frame.send_replace(Some(SearchFrame {
        revision: 1,
        query_id: 1,
        query: huge_capacity,
        snapshot: Some(FileSearchSnapshot {
            query_id: 1,
            query: "tiny".to_owned(),
            matches: vec![],
            total_match_count: 0,
            scanned_file_count: 0,
            walk_complete: true,
        }),
        phase: SearchPhase::Idle,
    }));
    // The fixture clones snapshots when polled, which compacts String capacity.
    // Exercise the production policy directly on the held frame too; the owned
    // runtime path below uses a genuinely large matched PathBuf instead.
    assert_eq!(
        provider
            .inner
            .policy
            .validate_frame(native.frame.borrow().as_ref().unwrap(), &[".".into()], 20)
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    let path = std::path::PathBuf::from("x".repeat(5000));
    native.frame.send_replace(Some(SearchFrame {
        revision: 2,
        query_id: 1,
        query: "tiny".to_owned(),
        phase: SearchPhase::Idle,
        snapshot: Some(FileSearchSnapshot {
            query_id: 1,
            query: "tiny".to_owned(),
            matches: vec![FileMatch {
                score: 1,
                path,
                root: ".".into(),
                match_type: MatchType::File,
                indices: None,
            }],
            total_match_count: 1,
            scanned_file_count: 1,
            walk_complete: true,
        }),
    }));
    reporter.wait_len(1).await;
    let outcome = bounded(session.close()).await;
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert!(
        matches!(outcome.operation, Err(error) if error.kind() == SearchErrorKind::ResourceExhausted)
    );
    assert_eq!(
        *lock(&reporter.events),
        vec![Event::Error(SearchErrorKind::ResourceExhausted)]
    );
    let outcome = bounded(provider.shutdown()).await;
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert!(outcome.operation.is_err());
}

#[tokio::test]
async fn dropping_public_provider_fences_surviving_scope_and_session_handles() {
    let native = Session::new(1, joined());
    let backend = Backend::new(vec![Arc::clone(&native)], 1);
    let provider = provider(Arc::clone(&backend));
    let factory = provider.scope_factory();
    let scope = scope(&provider);
    let session = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    drop(provider);
    assert_eq!(
        factory
            .new_scope(ScopeLimits {
                max_sessions: positive(1)
            })
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ClosedLease
    );
    assert_eq!(
        session
            .update_query(query(1, "closed"))
            .await
            .err()
            .unwrap()
            .kind(),
        SearchErrorKind::ClosedLease
    );
    assert_eq!(bounded(session.close()).await, joined());
    assert_eq!(bounded(scope.shutdown()).await, joined());
    assert!(backend.stopped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn owned_close_and_shutdown_recover_panicking_nonblocking_control_hooks() {
    let native = Session::new(1, joined());
    native.panic_close.store(true, Ordering::SeqCst);
    native.close_releases_update.store(true, Ordering::SeqCst);
    native.updates.acquire_many(100).await.unwrap().forget();
    let backend = Backend::new(vec![Arc::clone(&native)], 1);
    backend.panic_shutdown.store(true, Ordering::SeqCst);
    let provider = provider(backend);
    let scope = scope(&provider);
    let session = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    let updating = session.clone();
    let update = tokio::spawn(async move { updating.update_query(query(1, "gated")).await });
    bounded(native.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    let outcome = bounded(session.close()).await;
    let _ = bounded(update).await.unwrap();
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert!(
        matches!(outcome.operation, Err(error) if error.kind() == SearchErrorKind::SearchFailed)
    );
    assert_eq!(native.close_count.load(Ordering::SeqCst), 1);
    let outcome = bounded(provider.shutdown()).await;
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert!(
        matches!(outcome.operation, Err(error) if error.kind() == SearchErrorKind::SearchFailed)
    );
}

#[test]
fn rejected_unpolled_shutdown_tasks_report_uncertainty_after_runtime_destruction() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider = {
        let _entered = runtime.enter();
        provider(Backend::new(vec![], 0))
    };
    let scope = scope(&provider);
    drop(runtime);
    let observing = provider.shutdown();
    let observer_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let outcome = observer_runtime.block_on(bounded(observing));
    assert!(matches!(outcome.cleanup, CloseCleanup::Unconfirmed(_)));
    let scope_outcome = observer_runtime.block_on(bounded(scope.shutdown()));
    assert!(matches!(
        scope_outcome.cleanup,
        CloseCleanup::Unconfirmed(_)
    ));
}

#[tokio::test]
async fn accepted_update_failure_after_uncertain_receipt_survives_provider_recovery() {
    let uncertain = SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Unconfirmed(failure()),
    };
    let native = Session::new(1, uncertain.clone());
    native.updates.acquire_many(100).await.unwrap().forget();
    let provider = provider(Backend::new(vec![Arc::clone(&native)], 1));
    let scope = scope(&provider);
    let session = scope
        .open(request(), Arc::new(Reporter::default()))
        .await
        .unwrap();
    let updating = session.clone();
    let update = tokio::spawn(async move { updating.update_query(query(1, "late")).await });
    bounded(native.update_entered.acquire())
        .await
        .unwrap()
        .forget();
    assert_eq!(bounded(session.close()).await, uncertain);
    assert_eq!(bounded(scope.shutdown()).await, uncertain);
    *lock(&native.update_error) = Some(failure());
    native.updates.add_permits(1);
    let _ = bounded(update).await.unwrap();
    let outcome = bounded(provider.shutdown()).await;
    assert_eq!(
        outcome,
        SearchCloseOutcome {
            operation: Err(failure()),
            cleanup: CloseCleanup::Joined
        }
    );
    assert_eq!(bounded(session.close()).await, uncertain); // immutable earlier receipt
}
