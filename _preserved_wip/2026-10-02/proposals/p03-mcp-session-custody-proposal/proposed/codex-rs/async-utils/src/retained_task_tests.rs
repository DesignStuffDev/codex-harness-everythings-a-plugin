use super::*;
use pretty_assertions::assert_eq;
use std::error::Error;
use std::sync::Barrier;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn count(registry: &Registry) -> usize {
    registry.tasks.lock().unwrap_or_else(PoisonError::into_inner).len()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reserves_and_publishes_before_execution() -> TestResult {
    let registry = Arc::new(Registry::default());
    let published = Arc::new(AtomicBool::new(false));
    let worker_published = Arc::clone(&published);
    let owner = RetainedTask::spawn_in(&Handle::current(), async move {
        assert!(worker_published.load(Ordering::Acquire));
    }, &registry, |owner| {
        assert_eq!(count(&registry), 1);
        assert!(matches!(*owner.state.lock().unwrap_or_else(PoisonError::into_inner), State::Reserved));
        published.store(true, Ordering::Release);
    });
    tokio::time::timeout(Duration::from_secs(2), owner.wait()).await??;
    assert_eq!(count(&registry), 0);
    Ok(())
}

#[tokio::test]
async fn cancelled_and_expired_observers_preserve_exact_task() -> TestResult {
    let registry = Arc::new(Registry::default());
    let (release, held) = oneshot::channel();
    let owner = RetainedTask::spawn_in(&Handle::current(), async move {
        let _ = held.await;
        Ok::<_, ()>(37)
    }, &registry, |_| {});
    assert!(tokio::time::timeout(Duration::from_millis(2), owner.wait()).await.is_err());
    let observer = tokio::spawn({ let owner = Arc::clone(&owner); async move { owner.wait().await } });
    tokio::task::yield_now().await;
    observer.abort();
    let error = observer.await.err().ok_or("observer unexpectedly finished")?;
    assert!(error.is_cancelled());
    assert_eq!(count(&registry), 1);
    release.send(()).map_err(|_| "worker lost its release receiver")?;
    let first = tokio::time::timeout(Duration::from_secs(2), owner.wait()).await??;
    let second = owner.wait().await?;
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(*first, Ok(37));
    assert_eq!(count(&registry), 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_native_observers_share_one_completion() -> TestResult {
    let registry = Arc::new(Registry::default());
    let owner = RetainedTask::spawn_in(&Handle::current(), async { Ok::<_, ()>(91) }, &registry, |_| {});
    let barrier = Arc::new(Barrier::new(3));
    let mut observers = Vec::new();
    for _ in 0..2 {
        let owner = Arc::clone(&owner);
        let barrier = Arc::clone(&barrier);
        let runtime = Handle::current();
        observers.push(std::thread::spawn(move || {
            barrier.wait();
            runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(2), owner.wait()).await
            })
        }));
    }
    barrier.wait();
    let first = observers.remove(0).join().map_err(|_| "observer panicked")???;
    let second = observers.remove(0).join().map_err(|_| "observer panicked")???;
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(*first, Ok(91));
    assert_eq!(count(&registry), 0);
    Ok(())
}

#[tokio::test]
async fn panic_cancellation_and_logical_failure_remain_reachable() -> TestResult {
    let registry = Arc::new(Registry::default());
    let panic_owner = RetainedTask::<()>::spawn_in(&Handle::current(), async {
        panic!("controlled retained task panic");
    }, &registry, |_| {});
    assert_eq!(panic_owner.wait().await.err(), Some(TaskJoinFailure::Panicked));
    let cancelled = RetainedTask::spawn_in(&Handle::current(), std::future::pending::<()>(), &registry, |_| {});
    {
        let state = cancelled.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let State::Running(handle) = &*state { handle.abort(); }
    }
    assert_eq!(cancelled.wait().await.err(), Some(TaskJoinFailure::Cancelled));
    let failed = RetainedTask::spawn_in(&Handle::current(), async { Err::<(), _>("original failure") }, &registry, |_| {});
    assert_eq!(*failed.wait().await?, Err("original failure"));
    let weak = Arc::downgrade(&failed);
    drop(failed);
    assert!(weak.upgrade().is_some());
    assert_eq!(count(&registry), 3);
    let state = panic_owner.state.lock().unwrap_or_else(PoisonError::into_inner);
    assert!(matches!(&*state, State::Complete(completion) if completion._join_error.as_ref().is_some_and(JoinError::is_panic)));
    Ok(())
}

struct DropProbe {
    registry: Arc<Registry>,
    outside_lock: Arc<AtomicBool>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.outside_lock.store(self.registry.tasks.try_lock().is_ok(), Ordering::Release);
    }
}

#[tokio::test]
async fn publication_unwind_keeps_failure_and_drops_captures_outside_registry_lock() -> TestResult {
    let registry = Arc::new(Registry::default());
    let outside_lock = Arc::new(AtomicBool::new(false));
    let probe = DropProbe { registry: Arc::clone(&registry), outside_lock: Arc::clone(&outside_lock) };
    let owner = RetainedTask::spawn_in(&Handle::current(), async move { drop(probe); }, &registry, |_| {
        panic!("controlled publisher panic");
    });
    assert_eq!(owner.wait().await.err(), Some(TaskJoinFailure::SpawnPanicked));
    assert!(outside_lock.load(Ordering::Acquire));
    assert_eq!(count(&registry), 1);
    Ok(())
}
