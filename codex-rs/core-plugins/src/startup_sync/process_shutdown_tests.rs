use super::*;
use crate::startup_sync::SyncFailure;
use crate::startup_sync::callback_scope::CallbackRegistry;
use crate::startup_sync::callback_scope::CuratedCallbackScope;
use crate::startup_sync::callback_scope::CuratedSyncCallback;
use crate::startup_sync::worker::WorkerGate;
use anyhow::Context;
use futures::FutureExt;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::Semaphore;
use tokio::sync::oneshot;

const BUDGET: Duration = Duration::from_secs(/*secs*/ 5);

struct OwnedThread(Option<std::thread::JoinHandle<()>>);

impl OwnedThread {
    fn spawn(work: impl FnOnce() + Send + 'static) -> Self {
        Self(Some(std::thread::spawn(work)))
    }
}

impl Drop for OwnedThread {
    fn drop(&mut self) {
        if let Some(thread) = self.0.take() {
            let result = thread.join();
            assert!(
                result.is_ok() || std::thread::panicking(),
                "fixture thread panicked"
            );
        }
    }
}

struct RuntimeThread {
    stop: Option<oneshot::Sender<()>>,
    _thread: OwnedThread,
}

impl Drop for RuntimeThread {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        // The owned thread joins after the signal, including error/unwind paths.
    }
}

struct Fixture {
    // Drop the runtime and join its native owner before dropping callback custody.
    _runtime: RuntimeThread,
    handle: Handle,
    registry: Arc<CallbackRegistry>,
    scope: Arc<CuratedCallbackScope>,
    gate: Arc<WorkerGate>,
    worker_release: Option<mpsc::Sender<()>>,
}

impl Fixture {
    fn new() -> anyhow::Result<Self> {
        let gate = Arc::new(WorkerGate::default());
        let cleanup_gate = Arc::clone(&gate);
        let (ready_tx, ready_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = oneshot::channel();
        let runtime = RuntimeThread {
            stop: Some(stop_tx),
            _thread: OwnedThread::spawn(
                move || match tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                {
                    Ok(runtime) => {
                        let _ = ready_tx.send(Ok(runtime.handle().clone()));
                        runtime.block_on(async {
                            let _ = stop_rx.await;
                            let deadline = Instant::now() + BUDGET + BUDGET;
                            let observed =
                                cleanup_gate.begin_stop(deadline).wait_until(deadline).await;
                            assert!(!observed.is_pending(), "fixture worker remained pending");
                        });
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                },
            ),
        };
        let handle = ready_rx.recv_timeout(BUDGET)??;
        let registry = Arc::new(CallbackRegistry::default());
        let scope = CuratedCallbackScope::with_registry(handle.clone(), &registry);
        Ok(Self {
            _runtime: runtime,
            handle,
            registry,
            scope,
            gate,
            worker_release: None,
        })
    }

    fn begin(&self, deadline: Instant) -> CuratedProcessShutdown {
        let callbacks = self.registry.begin_close();
        let native = self.gate.begin_stop(deadline);
        CuratedProcessShutdown {
            native,
            callbacks,
            deadline,
        }
    }

    fn held_callback(&self) -> (CuratedSyncCallback, Arc<Semaphore>, mpsc::Receiver<()>) {
        let release = Arc::new(Semaphore::new(/*permits*/ 0));
        let (entered_tx, entered_rx) = mpsc::channel();
        let callback = self.scope.callback({
            let release = Arc::clone(&release);
            move || {
                let (release, entered_tx) = (Arc::clone(&release), entered_tx.clone());
                async move {
                    let _ = entered_tx.send(());
                    if let Ok(permit) = release.acquire().await {
                        permit.forget();
                    }
                }
            }
        });
        (callback, release, entered_rx)
    }

    fn hold_worker(
        &mut self,
        outcome: Result<(), SyncFailure>,
    ) -> anyhow::Result<mpsc::Receiver<()>> {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        self.worker_release = Some(release_tx);
        assert!(self.gate.start(move |_| {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(BUDGET).map_err(|error| {
                SyncFailure::Ordinary(format!("fixture worker release: {error}"))
            })?;
            outcome
        })?);
        Ok(entered_rx)
    }

    fn release_worker(&mut self) -> anyhow::Result<()> {
        self.worker_release
            .take()
            .context("held fixture worker")?
            .send(())?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(release) = self.worker_release.take() {
            let _ = release.send(());
        }
        // RuntimeThread then observes the exact gate handle before joining its
        // own native owner. A failing test never drops a live native handle.
    }
}

#[tokio::test]
async fn process_fence_rejects_existing_and_later_scope_dispatch() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let callback = fixture.scope.callback(|| async {});
    assert!(
        callback
            .scope_identity()
            .ptr_eq(&Arc::downgrade(&fixture.scope))
    );
    let deadline = Instant::now() + BUDGET;
    let stop = fixture.begin(deadline);
    assert!(!callback.dispatch());
    let late = CuratedCallbackScope::with_registry(fixture.handle.clone(), &fixture.registry);
    assert!(!late.callback(|| async {}).dispatch());
    let observed = stop.wait_until(deadline).await;
    assert!(observed.is_complete());
    assert_eq!(observed.callbacks, CuratedCallbackObservation::default());
    Ok(())
}

#[tokio::test]
async fn concurrent_registration_cannot_escape_process_snapshot() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let release = Arc::new(Semaphore::new(/*permits*/ 0));
    let (accepted_tx, accepted_rx) = mpsc::channel();
    let mut starts = Vec::new();
    let mut threads = Vec::new();
    for _ in 0..8 {
        let (start_tx, start_rx) = mpsc::channel();
        starts.push(start_tx);
        let registry = Arc::clone(&fixture.registry);
        let runtime = fixture.handle.clone();
        let release = Arc::clone(&release);
        let accepted_tx = accepted_tx.clone();
        threads.push(OwnedThread::spawn(move || {
            assert!(
                start_rx.recv_timeout(BUDGET).is_ok(),
                "registration barrier timed out"
            );
            let scope = CuratedCallbackScope::with_registry(runtime, &registry);
            let callback = scope.callback(move || {
                let release = Arc::clone(&release);
                async move {
                    if let Ok(permit) = release.acquire().await {
                        permit.forget();
                    }
                }
            });
            let _ = accepted_tx.send(callback.dispatch());
        }));
    }
    for start in starts {
        start.send(())?;
    }
    let deadline = Instant::now() + BUDGET;
    let stop = fixture.begin(deadline);
    let mut accepted = 0;
    for _ in 0..8 {
        accepted += usize::from(accepted_rx.recv_timeout(BUDGET)?);
    }
    drop(threads);
    release.add_permits(/*n*/ 8);
    let observed = stop.wait_until(deadline).await;
    assert!(observed.is_complete());
    assert_eq!(
        observed.callbacks.completed + observed.callbacks.suppressed,
        accepted
    );
    Ok(())
}

#[tokio::test]
async fn joined_without_a_published_outcome_is_not_clean() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let deadline = Instant::now() + BUDGET;
    let mut observed = fixture.begin(deadline).wait_until(deadline).await;
    assert_eq!(observed.native.native, CuratedSyncNativeCompletion::Idle);
    assert!(observed.is_complete());
    // Public observations must fail closed on this inconsistent combination.
    observed.native.native = CuratedSyncNativeCompletion::Joined;
    assert!(!observed.clean_native_ownership());
    assert!(!observed.is_complete());
    Ok(())
}

#[tokio::test]
async fn expired_begin_deadline_cannot_be_extended_by_observer() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let (callback, release, entered) = fixture.held_callback();
    assert!(callback.dispatch());
    entered.recv_timeout(BUDGET)?;
    let stop = fixture.begin(Instant::now());
    let observed = tokio::time::timeout(
        Duration::from_secs(/*secs*/ 1),
        stop.wait_until(Instant::now() + BUDGET),
    )
    .await?;
    assert!(observed.clean_native_ownership());
    assert_eq!(observed.callbacks.pending, 1);
    assert!(!observed.is_complete());
    release.add_permits(/*n*/ 1);
    // A separate observer can still harvest after the first observer expires.
    let observed = fixture
        .registry
        .begin_close()
        .wait_until(Instant::now() + BUDGET)
        .await;
    assert_eq!(observed.completed, 1);
    assert!(observed.is_complete());
    Ok(())
}

#[tokio::test]
async fn cancelled_aggregate_observer_keeps_pending_scope_reachable() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let (callback, release, entered) = fixture.held_callback();
    assert!(callback.dispatch());
    entered.recv_timeout(BUDGET)?;
    let deadline = Instant::now() + BUDGET;
    let stop = fixture.begin(deadline);
    let mut observer = Box::pin(stop.wait_until(deadline));
    assert!(observer.as_mut().now_or_never().is_none());
    drop(observer);
    drop(stop);
    drop(callback);
    let recovered = fixture.registry.begin_close();
    assert_eq!(recovered.wait_until(Instant::now()).await.pending, 1);
    release.add_permits(/*n*/ 1);
    let observed = recovered.wait_until(deadline).await;
    assert!(observed.is_complete());
    assert_eq!(observed.completed, 1);
    Ok(())
}

#[tokio::test]
async fn optional_sync_failure_is_nonfatal_after_exact_join_and_callback_completion()
-> anyhow::Result<()> {
    let mut fixture = Fixture::new()?;
    fixture
        .hold_worker(Err(SyncFailure::Ordinary(
            "optional sync unavailable".to_string(),
        )))?
        .recv_timeout(BUDGET)?;
    let (callback, release, entered) = fixture.held_callback();
    assert!(callback.dispatch());
    entered.recv_timeout(BUDGET)?;
    let deadline = Instant::now() + BUDGET;
    let stop = fixture.begin(deadline);
    let pending = stop.wait_until(Instant::now()).await;
    assert!(!pending.clean_native_ownership());
    assert_eq!(pending.callbacks.pending, 1);
    fixture.release_worker()?;
    release.add_permits(/*n*/ 1);
    let observed = stop.wait_until(deadline).await;
    assert_eq!(observed.native.native, CuratedSyncNativeCompletion::Joined);
    assert_eq!(
        observed
            .native
            .failure()
            .context("original optional sync failure")?
            .to_string(),
        "optional sync unavailable"
    );
    assert_eq!(observed.callbacks.completed, 1);
    assert!(observed.is_complete());
    Ok(())
}

#[tokio::test]
async fn joined_quarantine_remains_unclean() -> anyhow::Result<()> {
    let mut fixture = Fixture::new()?;
    fixture
        .hold_worker(Err(SyncFailure::Quarantined))?
        .recv_timeout(BUDGET)?;
    let deadline = Instant::now() + BUDGET;
    let stop = fixture.begin(deadline);
    fixture.release_worker()?;
    let observed = stop.wait_until(deadline).await;
    assert_eq!(observed.native.native, CuratedSyncNativeCompletion::Joined);
    assert!(observed.native.quarantined);
    assert!(observed.native.failure().is_some());
    assert!(!observed.clean_native_ownership());
    assert!(!observed.is_complete());
    Ok(())
}

#[tokio::test]
async fn callback_panic_remains_failed_across_process_observers() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let (entered_tx, entered_rx) = mpsc::channel();
    let callback = fixture.scope.callback(move || -> std::future::Ready<()> {
        let _ = entered_tx.send(());
        panic!("fixture callback failure")
    });
    assert!(callback.dispatch());
    entered_rx.recv_timeout(BUDGET)?;
    let deadline = Instant::now() + BUDGET;
    let observed = fixture.begin(deadline).wait_until(deadline).await;
    assert!(observed.clean_native_ownership());
    assert_eq!(
        (observed.callbacks.pending, observed.callbacks.failed),
        (0, 1)
    );
    assert!(!observed.is_complete());
    let repeated = fixture.begin(deadline).wait_until(deadline).await;
    assert_eq!(repeated.callbacks.failed, 1);
    assert!(!callback.dispatch());
    Ok(())
}
