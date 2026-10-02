use super::*;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use tokio::sync::Semaphore;
use tokio::sync::oneshot;

const BUDGET: Duration = Duration::from_secs(/*secs*/ 5);

struct OwnedThread<T>(Option<std::thread::JoinHandle<T>>);
impl<T: Send + 'static> OwnedThread<T> {
    fn spawn(work: impl FnOnce() -> T + Send + 'static) -> Self {
        Self(Some(std::thread::spawn(work)))
    }
    fn join(mut self) -> anyhow::Result<T> {
        self.0
            .take()
            .context("owned fixture handle")?
            .join()
            .map_err(|_| anyhow::anyhow!("fixture thread panicked"))
    }
}
impl<T> Drop for OwnedThread<T> {
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
    _worker: OwnedThread<()>,
}
impl Drop for RuntimeThread {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        // The owned worker field joins after this signal, including early ? paths.
    }
}

fn fixture() -> anyhow::Result<(
    RuntimeThread,
    Arc<CallbackRegistry>,
    Arc<CuratedCallbackScope>,
)> {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (stop_tx, stop_rx) = oneshot::channel();
    let runtime = RuntimeThread {
        stop: Some(stop_tx),
        _worker: OwnedThread::spawn(move || {
            match tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                Ok(runtime) => {
                    let _ = ready_tx.send(Ok(runtime.handle().clone()));
                    runtime.block_on(async {
                        let _ = stop_rx.await;
                    });
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            }
        }),
    };
    let handle = ready_rx.recv_timeout(BUDGET)??;
    let registry = Arc::new(CallbackRegistry::default());
    let scope = CuratedCallbackScope::with_registry(handle, &registry);
    Ok((runtime, registry, scope))
}

#[tokio::test]
async fn close_wins_reserved_dispatch_before_factory_or_handle_attachment() -> anyhow::Result<()> {
    let (_runtime, registry, scope) = fixture()?;
    let effects = Arc::new(AtomicUsize::new(/*v*/ 0));
    let callback = scope.callback({
        let effects = Arc::clone(&effects);
        move || {
            effects.fetch_add(/*val*/ 1, Ordering::SeqCst);
            std::future::ready(())
        }
    });
    let (reserved_tx, reserved_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let dispatcher = OwnedThread::spawn({
        let (callback, runtime) = (callback.clone(), scope.runtime.clone());
        move || {
            callback.dispatch_with_spawn(|job| {
                let _ = reserved_tx.send(());
                assert!(
                    release_rx.recv_timeout(BUDGET).is_ok(),
                    "dispatch barrier timed out"
                );
                runtime.spawn(job)
            })
        }
    });
    reserved_rx.recv_timeout(BUDGET)?;
    scope.begin_close();
    assert_eq!(scope.poll().pending, 1);
    assert_eq!(lock(&registry.scopes).len(), 1);
    assert!(!callback.dispatch());
    release_tx.send(())?;
    assert!(dispatcher.join()?);
    let observed = scope.wait_until(Instant::now() + BUDGET).await;
    assert!(observed.is_complete());
    assert_eq!(
        (
            observed.completed,
            observed.suppressed,
            effects.load(Ordering::SeqCst)
        ),
        (0, 1, 0)
    );
    assert!(lock(&registry.scopes).is_empty());
    Ok(())
}

#[tokio::test]
async fn finished_action_stays_pending_until_exact_handle_attachment() -> anyhow::Result<()> {
    let (_runtime, registry, scope) = fixture()?;
    let callback = scope.callback(|| async {});
    let (finished_tx, finished_rx) = mpsc::channel();
    let (attach_tx, attach_rx) = mpsc::channel();
    let dispatcher = OwnedThread::spawn({
        let runtime = scope.runtime.clone();
        move || {
            callback.dispatch_with_spawn(|job| {
                let handle = runtime.spawn(job);
                let deadline = Instant::now() + BUDGET;
                while !handle.is_finished() {
                    assert!(Instant::now() < deadline, "action did not finish");
                    std::thread::yield_now();
                }
                let _ = finished_tx.send(());
                assert!(
                    attach_rx.recv_timeout(BUDGET).is_ok(),
                    "attachment barrier timed out"
                );
                handle
            })
        }
    });
    finished_rx.recv_timeout(BUDGET)?;
    scope.begin_close();
    assert_eq!(scope.poll().pending, 1);
    assert_eq!(lock(&registry.scopes).len(), 1);
    attach_tx.send(())?;
    assert!(dispatcher.join()?);
    let observed = scope.wait_until(Instant::now() + BUDGET).await;
    assert!(observed.is_complete());
    assert_eq!(observed.completed, 1);
    assert!(lock(&registry.scopes).is_empty());
    Ok(())
}

#[tokio::test]
async fn cancelled_observer_and_abandoned_producer_keep_held_action_owned() -> anyhow::Result<()> {
    let (_runtime, registry, scope) = fixture()?;
    let (entered, release) = (
        Arc::new(Semaphore::new(/*permits*/ 0)),
        Arc::new(Semaphore::new(/*permits*/ 0)),
    );
    let steps = Arc::new(AtomicUsize::new(/*v*/ 0));
    let callback = scope.callback({
        let (entered, release, steps) = (
            Arc::clone(&entered),
            Arc::clone(&release),
            Arc::clone(&steps),
        );
        move || {
            let (entered, release, steps) = (
                Arc::clone(&entered),
                Arc::clone(&release),
                Arc::clone(&steps),
            );
            async move {
                steps.fetch_add(/*val*/ 1, Ordering::SeqCst);
                entered.add_permits(/*n*/ 1);
                if let Ok(permit) = release.acquire().await {
                    permit.forget();
                }
                steps.fetch_add(/*val*/ 1, Ordering::SeqCst);
            }
        }
    });
    assert!(OwnedThread::spawn(move || callback.dispatch()).join()?);
    tokio::time::timeout(BUDGET, entered.acquire())
        .await??
        .forget();
    assert_eq!(scope.wait_until(Instant::now()).await.pending, 1);
    let observer = tokio::spawn({
        let scope = Arc::clone(&scope);
        async move { scope.wait().await }
    });
    tokio::task::yield_now().await;
    observer.abort();
    assert!(
        observer
            .await
            .err()
            .context("cancelled observer")?
            .is_cancelled()
    );
    let weak = Arc::downgrade(&scope);
    drop(scope);
    assert!(weak.upgrade().is_some());
    assert_eq!(steps.load(Ordering::SeqCst), 1);
    let recovered = lock(&registry.scopes)
        .first()
        .cloned()
        .context("primary scope custody")?;
    assert_eq!(recovered.poll().pending, 1);
    release.add_permits(/*n*/ 1);
    assert!(
        recovered
            .wait_until(Instant::now() + BUDGET)
            .await
            .is_complete()
    );
    assert_eq!(steps.load(Ordering::SeqCst), 2);
    assert!(lock(&registry.scopes).is_empty());
    drop(recovered);
    assert!(weak.upgrade().is_none());
    Ok(())
}

struct PanicPayload(Arc<AtomicUsize>);
impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.fetch_add(/*val*/ 1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn factory_and_spawn_panics_stay_failed_with_original_payloads_retained() -> anyhow::Result<()>
{
    for spawn_panics in [false, true] {
        let (_runtime, registry, scope) = fixture()?;
        let dropped = Arc::new(AtomicUsize::new(/*v*/ 0));
        let (entered_tx, entered_rx) = mpsc::channel();
        let callback = scope.callback({
            let dropped = Arc::clone(&dropped);
            move || -> std::future::Ready<()> {
                let _ = entered_tx.send(());
                std::panic::panic_any(PanicPayload(Arc::clone(&dropped)))
            }
        });
        if spawn_panics {
            assert!(
                callback.dispatch_with_spawn(|_| std::panic::panic_any(PanicPayload(Arc::clone(
                    &dropped
                ))))
            );
        } else {
            assert!(callback.dispatch());
            entered_rx.recv_timeout(BUDGET)?;
        }
        let observed = tokio::time::timeout(BUDGET, async {
            loop {
                let observed = scope.poll();
                if observed.pending == 0 {
                    break observed;
                }
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert_eq!((observed.pending, observed.failed), (0, 1));
        assert!(!observed.is_complete());
        assert!(!lock(&scope.state).closed);
        assert!(
            !callback.dispatch(),
            "an observed failure fences new reservations"
        );
        assert_eq!(scope.wait_until(Instant::now()).await.failed, 1);
        assert_eq!(lock(&registry.scopes).len(), 1);
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        drop(callback);
        drop(scope);
        drop(registry);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[test]
fn runtime_shutdown_is_a_retained_join_failure_not_suppression() -> anyhow::Result<()> {
    let (runtime, registry, scope) = fixture()?;
    let (entered_tx, entered_rx) = mpsc::channel();
    let callback = scope.callback(move || {
        let entered = entered_tx.clone();
        async move {
            let _ = entered.send(());
            std::future::pending::<()>().await;
        }
    });
    assert!(callback.dispatch());
    entered_rx.recv_timeout(BUDGET)?;
    drop(runtime);
    let observer_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let observed = observer_runtime.block_on(scope.wait_until(Instant::now() + BUDGET));
    assert_eq!(
        (observed.pending, observed.suppressed, observed.failed),
        (0, 0, 1)
    );
    assert!(!observed.is_complete());
    assert_eq!(lock(&registry.scopes).len(), 1);
    Ok(())
}

#[tokio::test]
async fn closing_one_scope_preserves_other_and_open_idle_scopes_remain_registered()
-> anyhow::Result<()> {
    let (_runtime, registry, first) = fixture()?;
    let second = CuratedCallbackScope::with_registry(first.runtime.clone(), &registry);
    let first_callback = first.callback(|| async {});
    let second_callback = second.callback(|| async {});
    assert!(first.poll().is_complete());
    assert_eq!(lock(&registry.scopes).len(), 2);
    first.begin_close();
    assert!(!first_callback.dispatch());
    assert!(second_callback.dispatch());
    tokio::time::timeout(BUDGET, async {
        while second.poll().completed == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert_eq!(lock(&registry.scopes).len(), 1);
    assert!(second.wait_until(Instant::now()).await.is_complete());
    assert!(lock(&registry.scopes).is_empty());
    Ok(())
}

#[tokio::test]
async fn concurrent_native_observers_share_the_exact_join() -> anyhow::Result<()> {
    let (_runtime, _registry, scope) = fixture()?;
    assert!(scope.callback(|| async {}).dispatch());
    tokio::time::timeout(BUDGET, async {
        loop {
            let finished = matches!(lock(&scope.state).records.first(), Some(Record::Running(handle)) if handle.is_finished());
            if finished { break; }
            tokio::task::yield_now().await;
        }
    }).await?;
    scope.begin_close();
    let (ready_tx, ready_rx) = mpsc::channel();
    let mut releases = Vec::new();
    let observers: Vec<_> = (0..2)
        .map(|_| {
            let (scope, ready_tx) = (Arc::clone(&scope), ready_tx.clone());
            let (release_tx, release_rx) = mpsc::channel();
            releases.push(release_tx);
            OwnedThread::spawn(move || -> anyhow::Result<CuratedCallbackObservation> {
                ready_tx.send(())?;
                release_rx.recv_timeout(BUDGET)?;
                let deadline = Instant::now() + BUDGET;
                loop {
                    let observed = scope.poll();
                    if observed.pending == 0 || Instant::now() >= deadline {
                        return Ok(observed);
                    }
                    std::thread::yield_now();
                }
            })
        })
        .collect();
    for _ in 0..2 {
        ready_rx.recv_timeout(BUDGET)?;
    }
    for release in releases {
        release.send(())?;
    }
    for observer in observers {
        let observed = observer.join()??;
        assert!(observed.is_complete());
        assert_eq!(observed.completed, 1);
    }
    Ok(())
}
