use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_protocol::ThreadId;
use codex_thread_store::*;
use pretty_assertions::assert_eq;
use tokio::sync::Semaphore;

use super::*;

struct Signals {
    starts: AtomicUsize,
    start_entered: Semaphore,
    start_gate: Semaphore,
    close_entered: Semaphore,
    close_gate: Semaphore,
    observation_entered: Semaphore,
    observation_gate: Semaphore,
    cancelled: AtomicUsize,
    closed: AtomicUsize,
    fail_start: AtomicBool,
    fail_close: AtomicBool,
}

impl Default for Signals {
    fn default() -> Self {
        Self {
            starts: AtomicUsize::new(0),
            start_entered: Semaphore::new(0),
            start_gate: Semaphore::new(1024),
            close_entered: Semaphore::new(0),
            close_gate: Semaphore::new(1024),
            observation_entered: Semaphore::new(0),
            observation_gate: Semaphore::new(1024),
            cancelled: AtomicUsize::new(0),
            closed: AtomicUsize::new(0),
            fail_start: AtomicBool::new(false),
            fail_close: AtomicBool::new(false),
        }
    }
}

struct TestRun(Arc<Signals>);

impl RolloutMigrationRun for TestRun {
    fn snapshot(&self) -> ThreadStoreFuture<'_, RolloutMigrationSnapshot> {
        Box::pin(async {
            self.0.observation_entered.add_permits(1);
            self.0.observation_gate.acquire().await.unwrap().forget();
            Ok(RolloutMigrationSnapshot::default())
        })
    }
    fn report(&self) -> ThreadStoreFuture<'_, RolloutMigrationReport> {
        Box::pin(async { Err(conflict("manual rollout migration is still running")) })
    }
    fn cancel(&self) -> ThreadStoreFuture<'_, ()> {
        self.0.cancelled.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }
    fn close(self: Box<Self>) -> ThreadStoreFuture<'static, ()> {
        self.0.cancelled.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            self.0.close_entered.add_permits(1);
            self.0.close_gate.acquire().await.unwrap().forget();
            self.0.closed.fetch_add(1, Ordering::SeqCst);
            if self.0.fail_close.load(Ordering::SeqCst) {
                Err(ThreadStoreError::Internal {
                    message: "fixture close failed".to_owned(),
                })
            } else {
                Ok(())
            }
        })
    }
}

struct TestStore {
    inner: Arc<InMemoryThreadStore>,
    signals: Arc<Signals>,
}

impl TestStore {
    fn new(signals: Arc<Signals>) -> Self {
        Self {
            inner: InMemoryThreadStore::for_id(uuid::Uuid::new_v4().to_string()),
            signals,
        }
    }
}

macro_rules! delegate {
    ($method:ident, $input:ty, $output:ty) => {
        fn $method(&self, params: $input) -> ThreadStoreFuture<'_, $output> {
            self.inner.$method(params)
        }
    };
}

impl ThreadStore for TestStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn supports_manual_rollout_migration(&self) -> bool {
        true
    }
    fn start_rollout_migration(
        &self,
        _options: RolloutMigrationOptions,
    ) -> ThreadStoreFuture<'_, Box<dyn RolloutMigrationRun>> {
        Box::pin(async {
            self.signals.starts.fetch_add(1, Ordering::SeqCst);
            self.signals.start_entered.add_permits(1);
            self.signals.start_gate.acquire().await.unwrap().forget();
            if self.signals.fail_start.load(Ordering::SeqCst) {
                Err(ThreadStoreError::InvalidRequest {
                    message: "fixture start failed".to_owned(),
                })
            } else {
                Ok(Box::new(TestRun(Arc::clone(&self.signals))) as Box<dyn RolloutMigrationRun>)
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
    delegate!(prepare_fork, PrepareForkParams, PreparedFork);
    delegate!(delete_thread, DeleteThreadParams, ());
    fn persist_thread(&self, id: ThreadId, context: PersistContext) -> ThreadStoreFuture<'_, ()> {
        self.inner.persist_thread(id, context)
    }
}

fn request(id: &str) -> StartMigrationRequest {
    StartMigrationRequest {
        contract_version: MANUAL_ROLLOUT_MIGRATION_CONTRACT_VERSION,
        lease_id: id.to_owned(),
        options: RolloutMigrationOptions {
            mode: RolloutMigrationMode::DryRun,
            thread_ids: Vec::new(),
            max_mib_per_second: None,
        },
    }
}

async fn permit(gate: &Semaphore) {
    tokio::time::timeout(Duration::from_secs(2), gate.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
}

#[tokio::test]
async fn released_before_start_is_paired_without_native_admission() {
    let registry = Arc::new(MigrationRegistry::default());
    let signals = Arc::new(Signals::default());
    let release_registry = Arc::clone(&registry);
    let release = tokio::spawn(async move { release_registry.release("early").await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !registry.state.lock().await.entries.contains_key("early") {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        !release.is_finished(),
        "cleanup cannot acknowledge an unpaired start"
    );
    assert!(matches!(
        registry
            .start(&TestStore::new(Arc::clone(&signals)), request("early"))
            .await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    release.await.unwrap().unwrap();
    assert_eq!(signals.starts.load(Ordering::SeqCst), 0);
    assert!(registry.state.lock().await.entries.is_empty());
}

#[tokio::test]
async fn release_during_start_waits_for_late_native_close_without_holding_registry() {
    let registry = Arc::new(MigrationRegistry::default());
    let signals = Arc::new(Signals {
        start_gate: Semaphore::new(0),
        close_gate: Semaphore::new(0),
        ..Default::default()
    });
    let store = TestStore::new(Arc::clone(&signals));
    let starting_registry = Arc::clone(&registry);
    let start = tokio::spawn(async move { starting_registry.start(&store, request("late")).await });
    permit(&signals.start_entered).await;
    let releasing_registry = Arc::clone(&registry);
    let release = tokio::spawn(async move { releasing_registry.release("late").await });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if matches!(
                registry.state.lock().await.entries.get("late"),
                Some(Lease::Preparing { released: true, .. })
            ) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    signals.start_gate.add_permits(1);
    permit(&signals.close_entered).await;
    assert!(!start.is_finished());
    assert!(!release.is_finished());
    // Another operation can access the registry while cleanup awaits its native owner.
    assert!(registry.snapshot("late").await.is_err());
    signals.close_gate.add_permits(1);
    start.await.unwrap().unwrap();
    release.await.unwrap().unwrap();
    assert_eq!(signals.closed.load(Ordering::SeqCst), 1);
    assert!(registry.state.lock().await.entries.is_empty());
}

#[tokio::test]
async fn admitted_observation_finishes_before_consuming_close() {
    let registry = Arc::new(MigrationRegistry::default());
    let signals = Arc::new(Signals {
        observation_gate: Semaphore::new(0),
        ..Default::default()
    });
    registry
        .start(&TestStore::new(Arc::clone(&signals)), request("reader"))
        .await
        .unwrap();
    let reading_registry = Arc::clone(&registry);
    let reader = tokio::spawn(async move { reading_registry.snapshot("reader").await });
    permit(&signals.observation_entered).await;
    let releasing_registry = Arc::clone(&registry);
    let release = tokio::spawn(async move { releasing_registry.release("reader").await });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if matches!(
                registry.state.lock().await.entries.get("reader"),
                Some(Lease::Closing(_))
            ) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!release.is_finished());
    assert_eq!(signals.closed.load(Ordering::SeqCst), 0);
    assert!(registry.snapshot("reader").await.is_err());
    signals.observation_gate.add_permits(1);
    reader.await.unwrap().unwrap();
    release.await.unwrap().unwrap();
    assert_eq!(signals.closed.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn version_failure_and_capacity_rejections_are_paired_and_reclaim_tombstones() {
    let registry = MigrationRegistry::default();
    let signals = Arc::new(Signals::default());
    let store = TestStore::new(Arc::clone(&signals));
    for index in 0..128 {
        let id = format!("version-{index}");
        let mut unsupported = request(&id);
        unsupported.contract_version = 99;
        assert!(matches!(
            registry.start(&store, unsupported).await,
            Err(ThreadStoreError::Unsupported {
                operation: "manual_rollout_migration_contract"
            })
        ));
        registry.release(&id).await.unwrap();
    }
    assert_eq!(signals.starts.load(Ordering::SeqCst), 0);
    for index in 0..MAX_MIGRATION_LEASES {
        registry
            .start(&store, request(&format!("held-{index}")))
            .await
            .unwrap();
    }
    for index in 0..128 {
        let id = format!("overflow-{index}");
        assert!(matches!(
            registry.start(&store, request(&id)).await,
            Err(ThreadStoreError::Conflict { .. })
        ));
        registry.release(&id).await.unwrap();
    }
    assert_eq!(
        registry.state.lock().await.entries.len(),
        MAX_MIGRATION_LEASES
    );
    assert_eq!(signals.starts.load(Ordering::SeqCst), MAX_MIGRATION_LEASES);
    registry.begin_shutdown().await;
    registry.finish_shutdown().await.unwrap();
    assert_eq!(signals.closed.load(Ordering::SeqCst), MAX_MIGRATION_LEASES);
    assert!(registry.state.lock().await.entries.is_empty());
}

#[tokio::test]
async fn shutdown_signals_then_joins_all_runs_and_retains_cleanup_failures() {
    let registry = Arc::new(MigrationRegistry::default());
    let signals = Arc::new(Signals {
        close_gate: Semaphore::new(0),
        fail_close: AtomicBool::new(true),
        ..Default::default()
    });
    let store = TestStore::new(Arc::clone(&signals));
    registry.start(&store, request("first")).await.unwrap();
    registry.start(&store, request("second")).await.unwrap();
    assert!(matches!(
        registry.report("first").await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    registry.cancel("first").await.unwrap();
    assert_eq!(signals.closed.load(Ordering::SeqCst), 0);
    registry.begin_shutdown().await;
    assert!(signals.cancelled.load(Ordering::SeqCst) >= 2);
    assert!(matches!(
        registry.start(&store, request("after-close")).await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    let shutdown_registry = Arc::clone(&registry);
    let shutdown = tokio::spawn(async move { shutdown_registry.finish_shutdown().await });
    permit(&signals.close_entered).await;
    assert!(!shutdown.is_finished());
    signals.close_gate.add_permits(2);
    let error = shutdown.await.unwrap().unwrap_err();
    assert_eq!(error.to_string().matches("fixture close failed").count(), 2);
    assert_eq!(signals.closed.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn sequential_duplicate_cleanup_is_idempotent_with_bounded_retired_history() {
    let registry = MigrationRegistry::default();
    let signals = Arc::new(Signals::default());
    let store = TestStore::new(Arc::clone(&signals));
    for index in 0..MAX_RETIRED_LEASES + 4 {
        let id = format!("released-{index}");
        registry.start(&store, request(&id)).await.unwrap();
        registry.release(&id).await.unwrap();
        registry.release(&id).await.unwrap();
    }
    assert_eq!(
        signals.closed.load(Ordering::SeqCst),
        MAX_RETIRED_LEASES + 4
    );
    let state = registry.state.lock().await;
    assert!(state.entries.is_empty());
    assert_eq!(state.retired.len(), MAX_RETIRED_LEASES);
    assert_eq!(state.retired_order.len(), MAX_RETIRED_LEASES);
}

#[tokio::test(start_paused = true)]
async fn timed_out_unmatched_release_still_fences_a_late_start() {
    let registry = MigrationRegistry::default();
    let signals = Arc::new(Signals::default());
    let error = registry.release("delayed").await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("did not receive its paired start")
    );
    assert!(matches!(
        registry.state.lock().await.entries.get("delayed"),
        Some(Lease::Released(_))
    ));
    let result = registry
        .start(&TestStore::new(Arc::clone(&signals)), request("delayed"))
        .await;
    assert!(matches!(result, Err(ThreadStoreError::Conflict { .. })));
    assert_eq!(signals.starts.load(Ordering::SeqCst), 0);
    assert!(registry.state.lock().await.entries.is_empty());
}

#[tokio::test(start_paused = true)]
async fn unmatched_cleanup_exhaustion_fails_closed_without_unbounded_tombstones() {
    let registry = MigrationRegistry::default();
    for index in 0..MAX_MIGRATION_ENTRIES {
        registry
            .release(&format!("orphan-{index}"))
            .await
            .unwrap_err();
    }
    assert_eq!(
        registry.state.lock().await.entries.len(),
        MAX_MIGRATION_ENTRIES
    );
    registry.release("overflow").await.unwrap_err();
    let signals = Arc::new(Signals::default());
    let store = TestStore::new(Arc::clone(&signals));
    assert!(registry.start(&store, request("overflow")).await.is_err());
    assert!(
        registry
            .start(&store, request("new-after-exhaustion"))
            .await
            .is_err()
    );
    assert_eq!(signals.starts.load(Ordering::SeqCst), 0);
    assert_eq!(
        registry.state.lock().await.entries.len(),
        MAX_MIGRATION_ENTRIES
    );
    registry.begin_shutdown().await;
    registry.finish_shutdown().await.unwrap();
    assert!(registry.state.lock().await.entries.is_empty());
}
