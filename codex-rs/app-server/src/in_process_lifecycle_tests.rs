use crate::curated_callback_lifecycle::CuratedCallbackGuard;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use codex_protocol::ThreadId;
use codex_thread_store::*;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use super::ErrorKind;
use super::InProcessClientHandle;
use super::InProcessClientMessage;
use super::InProcessClientSender;
use super::IoError;
use super::IoResult;
use super::StoreShutdownGuard;
use super::finish_processor_and_services;

enum CloseBehavior {
    Complete,
    Fail,
    Stall,
    Delayed(std::time::Duration),
}

struct ShutdownProbe {
    inner: Arc<InMemoryThreadStore>,
    id: String,
    begun: AtomicBool,
    completed: AtomicBool,
    behavior: CloseBehavior,
}

impl ShutdownProbe {
    fn new(behavior: CloseBehavior) -> Arc<Self> {
        let id = uuid::Uuid::new_v4().to_string();
        Arc::new(Self {
            inner: InMemoryThreadStore::for_id(id.clone()),
            id,
            begun: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            behavior,
        })
    }
}

impl Drop for ShutdownProbe {
    fn drop(&mut self) {
        InMemoryThreadStore::remove_id(&self.id);
    }
}

macro_rules! delegate {
    ($method:ident, $input:ty, $output:ty) => {
        fn $method(&self, params: $input) -> ThreadStoreFuture<'_, $output> {
            self.inner.$method(params)
        }
    };
}

impl ThreadStore for ShutdownProbe {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn begin_shutdown_store(&self) {
        self.begun.store(true, Ordering::Release);
    }

    fn shutdown_store(&self) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async {
            match self.behavior {
                CloseBehavior::Complete => {
                    self.completed.store(true, Ordering::Release);
                    Ok(())
                }
                CloseBehavior::Fail => Err(ThreadStoreError::Internal {
                    message: "backend diagnostic with private-token".to_owned(),
                }),
                CloseBehavior::Stall => std::future::pending().await,
                CloseBehavior::Delayed(delay) => {
                    tokio::time::sleep(delay).await;
                    self.completed.store(true, Ordering::Release);
                    Ok(())
                }
            }
        })
    }

    delegate!(create_thread, CreateThreadParams, ());
    delegate!(resume_thread, ResumeThreadParams, ());
    delegate!(append_items, AppendThreadItemsParams, ());
    delegate!(flush_thread, ThreadId, ());
    delegate!(shutdown_thread, ThreadId, ());
    delegate!(discard_thread, ThreadId, ());
    delegate!(load_history, LoadThreadHistoryParams, StoredThreadHistory);
    delegate!(read_thread, ReadThreadParams, StoredThread);
    delegate!(
        read_thread_by_rollout_path,
        ReadThreadByRolloutPathParams,
        StoredThread
    );
    delegate!(list_threads, ListThreadsParams, ThreadPage);
    delegate!(
        update_thread_metadata,
        UpdateThreadMetadataParams,
        Option<StoredThread>
    );
    delegate!(archive_thread, ArchiveThreadParams, ());
    delegate!(unarchive_thread, ArchiveThreadParams, StoredThread);
    delegate!(delete_thread, DeleteThreadParams, ());

    fn persist_thread(&self, id: ThreadId, context: PersistContext) -> ThreadStoreFuture<'_, ()> {
        self.inner.persist_thread(id, context)
    }
}

#[tokio::test(start_paused = true)]
async fn stalled_processor_is_aborted_then_storage_closes_with_other_holders_alive() {
    let store = ShutdownProbe::new(CloseBehavior::Complete);
    let guard = StoreShutdownGuard::new(store.clone());
    let detached_store_reference = store.clone();
    let mut processor = tokio::spawn(async move {
        let _retained = detached_store_reference;
        std::future::pending::<std::io::Result<()>>().await
    });
    let search_home = tempfile::tempdir().expect("search home");
    let (_, search_guard) = crate::file_search_services::start(search_home.path())
        .await
        .expect("search provider");
    let result = finish_processor_and_services(
        &mut processor,
        &guard,
        &search_guard,
        &CuratedCallbackGuard::new(),
        crate::process_final::drain_deadline(super::PROCESSOR_SHUTDOWN_TIMEOUT),
    )
    .await;
    assert_eq!(
        result.expect_err("processor timeout is reported").kind(),
        ErrorKind::TimedOut
    );
    assert_eq!(
        (
            store.begun.load(Ordering::Acquire),
            store.completed.load(Ordering::Acquire)
        ),
        (true, true)
    );
}

#[tokio::test]
async fn aborting_runtime_fences_storage_without_waiting_for_other_references_to_drop() {
    let store = ShutdownProbe::new(CloseBehavior::Complete);
    let guard = StoreShutdownGuard::new(store.clone());
    let (started, ready) = oneshot::channel();
    let runtime = tokio::spawn(async move {
        let _guard = guard;
        started.send(()).expect("runtime started");
        std::future::pending::<()>().await;
    });
    ready.await.expect("runtime owns guard");
    runtime.abort();
    assert!(runtime.await.expect_err("aborted runtime").is_cancelled());
    assert!(store.begun.load(Ordering::Acquire));
}

#[tokio::test]
async fn processor_cleanup_failure_reaches_client_after_storage_is_joined() {
    let store = ShutdownProbe::new(CloseBehavior::Complete);
    let guard = StoreShutdownGuard::new(store.clone());
    let mut processor =
        tokio::spawn(async { Err(IoError::other("search publisher cleanup failed")) });
    let search_home = tempfile::tempdir().expect("search home");
    let (_, search_guard) = crate::file_search_services::start(search_home.path())
        .await
        .expect("search provider");
    let error = finish_processor_and_services(
        &mut processor,
        &guard,
        &search_guard,
        &CuratedCallbackGuard::new(),
        crate::process_final::drain_deadline(super::PROCESSOR_SHUTDOWN_TIMEOUT),
    )
    .await
    .expect_err("search cleanup failure must not become successful shutdown");
    assert_eq!(error.kind(), ErrorKind::Other);
    assert_eq!(error.to_string(), "search publisher cleanup failed");
    assert_eq!(
        (
            store.begun.load(Ordering::Acquire),
            store.completed.load(Ordering::Acquire)
        ),
        (true, true)
    );
}

#[tokio::test]
async fn cancelling_client_shutdown_fences_storage_while_runtime_is_detached() {
    let store = ShutdownProbe::new(CloseBehavior::Complete);
    let callback_guard = CuratedCallbackGuard::new();
    let callback_scope = callback_guard.scope();
    let callback = callback_scope.callback(|| async {});
    let (client_tx, mut client_rx) = mpsc::channel(1);
    let (_event_tx, event_rx) = mpsc::channel(1);
    let runtime_handle = tokio::spawn(std::future::pending::<IoResult<()>>());
    let abort_runtime = runtime_handle.abort_handle();
    let client = InProcessClientHandle {
        client: InProcessClientSender { client_tx },
        event_rx,
        runtime_handle,
        _store_lifecycle: Some(StoreShutdownGuard::new(store.clone())),
        _search_lifecycle: None,
        _curated_callback_lifecycle: Some(callback_guard),
        _featured_warmup_lifecycle: None,
        _test_codex_home: None,
    };
    let shutting_down = tokio::spawn(client.shutdown());
    let request = client_rx.recv().await.expect("shutdown request");
    assert!(matches!(request, InProcessClientMessage::Shutdown { .. }));
    shutting_down.abort();
    assert!(
        shutting_down
            .await
            .expect_err("caller cancelled")
            .is_cancelled()
    );
    assert!(store.begun.load(Ordering::Acquire));
    assert!(!callback.dispatch());
    assert!(
        callback_scope
            .wait_until(std::time::Instant::now())
            .await
            .is_complete()
    );
    abort_runtime.abort();
}

#[tokio::test(start_paused = true)]
async fn stalled_store_shutdown_reports_unknown_write_outcomes() {
    let store = ShutdownProbe::new(CloseBehavior::Stall);
    let guard = StoreShutdownGuard::new(store.clone());
    let error = guard.finish(Ok(())).await.expect_err("store cannot finish");
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert!(
        error
            .to_string()
            .contains("accepted write outcomes may be unknown")
    );
    assert!(store.begun.load(Ordering::Acquire));
}

#[tokio::test]
async fn client_observes_primary_failure_and_sanitized_storage_cleanup_failure() {
    let store = ShutdownProbe::new(CloseBehavior::Fail);
    let (client_tx, mut client_rx) = mpsc::channel(1);
    let (_event_tx, event_rx) = mpsc::channel(1);
    let guard = StoreShutdownGuard::new(store.clone());
    let runtime_handle = tokio::spawn(async move {
        let Some(InProcessClientMessage::Shutdown { done_tx }) = client_rx.recv().await else {
            panic!("expected shutdown request");
        };
        let outcome = guard
            .finish(Err(IoError::new(
                ErrorKind::PermissionDenied,
                "startup denied",
            )))
            .await;
        let _ = done_tx.send(());
        outcome
    });
    let client = InProcessClientHandle {
        client: InProcessClientSender { client_tx },
        event_rx,
        runtime_handle,
        _store_lifecycle: Some(StoreShutdownGuard::new(store)),
        _search_lifecycle: None,
        _curated_callback_lifecycle: None,
        _featured_warmup_lifecycle: None,
        _test_codex_home: None,
    };
    let error = client
        .shutdown()
        .await
        .expect_err("shutdown failure reaches the caller");
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
    assert!(error.to_string().contains("startup denied"));
    assert!(
        error
            .to_string()
            .contains("accepted write outcomes may be unknown")
    );
    assert!(!error.to_string().contains("private-token"));
}

#[path = "in_process_search_lifecycle_tests.rs"]
mod search_services;

// Exercise the actual outer store owner independently of the legacy processor
// budget. Paused time verifies budget composition; installed-host acceptance
// remains a separate required runtime gate.
async fn finish_with_slow_store(primary: IoResult<()>) -> IoResult<()> {
    let store = ShutdownProbe::new(CloseBehavior::Delayed(std::time::Duration::from_secs(
        /*secs*/ 46,
    )));
    let guard = StoreShutdownGuard::new(store.clone());
    let mut processor = tokio::spawn(async move { primary });
    let search_home = tempfile::tempdir()?;
    let (_, search_guard) = crate::file_search_services::start(search_home.path()).await?;
    let started = tokio::time::Instant::now();
    let result = finish_processor_and_services(
        &mut processor,
        &guard,
        &search_guard,
        &CuratedCallbackGuard::new(),
        crate::process_final::drain_deadline(super::PROCESSOR_SHUTDOWN_TIMEOUT),
    )
    .await;
    assert!(store.begun.load(Ordering::Acquire));
    assert!(store.completed.load(Ordering::Acquire));
    assert!(started.elapsed() >= std::time::Duration::from_secs(/*secs*/ 46));
    result
}

#[tokio::test(start_paused = true)]
async fn selected_store_may_drain_beyond_45_seconds_within_its_owner_budget() -> IoResult<()> {
    finish_with_slow_store(Ok(())).await
}

#[tokio::test(start_paused = true)]
async fn slow_store_success_preserves_an_independent_session_failure() {
    let result = finish_with_slow_store(Err(IoError::other("session drain failed"))).await;
    match result {
        Err(error) => assert_eq!(error.to_string(), "session drain failed"),
        Ok(()) => panic!("successful store close must not erase the session failure"),
    }
}

#[tokio::test(start_paused = true)]
async fn processor_observation_uses_callers_remaining_deadline_then_closes_storage() {
    struct RecordAbort(Arc<std::sync::Mutex<Option<tokio::time::Instant>>>);
    impl Drop for RecordAbort {
        fn drop(&mut self) {
            *self.0.lock().unwrap() = Some(tokio::time::Instant::now());
        }
    }
    let store = ShutdownProbe::new(CloseBehavior::Complete);
    let guard = StoreShutdownGuard::new(store.clone());
    let aborted_at = Arc::new(std::sync::Mutex::new(None));
    let observed_abort = Arc::clone(&aborted_at);
    let (started, ready) = oneshot::channel();
    let mut processor = tokio::spawn(async move {
        let _record = RecordAbort(observed_abort);
        started.send(()).expect("processor entered");
        std::future::pending::<IoResult<()>>().await
    });
    ready.await.expect("processor owns abort observation");
    let search_home = tempfile::tempdir().expect("search home");
    let (_, search_guard) = crate::file_search_services::start(search_home.path())
        .await
        .expect("search provider");
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    tokio::time::advance(std::time::Duration::from_secs(4)).await;
    let result = finish_processor_and_services(
        &mut processor,
        &guard,
        &search_guard,
        &CuratedCallbackGuard::new(),
        deadline,
    )
    .await;
    assert_eq!(
        result.expect_err("supplied deadline expires").kind(),
        ErrorKind::TimedOut
    );
    let observed_abort = aborted_at
        .lock()
        .unwrap()
        .expect("processor abort observed");
    assert!(observed_abort >= deadline);
    // Permit timer granularity, but not a renewed 45-second processor budget.
    // Observe the processor itself, excluding later provider cleanup latency.
    assert!(observed_abort <= deadline + std::time::Duration::from_millis(100));
    assert_eq!(
        (
            store.begun.load(Ordering::Acquire),
            store.completed.load(Ordering::Acquire)
        ),
        (true, true),
    );
}
