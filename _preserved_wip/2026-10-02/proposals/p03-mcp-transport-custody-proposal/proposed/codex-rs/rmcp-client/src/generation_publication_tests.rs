use super::*;
use pretty_assertions::assert_eq;
use std::error::Error;
use std::sync::Barrier;
use std::sync::Weak;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

struct Generation {
    calls: AtomicUsize,
    callback_outside_lock: AtomicBool,
    publication: Weak<GenerationPublication>,
}

impl Generation {
    fn new(publication: &Arc<GenerationPublication>) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            callback_outside_lock: AtomicBool::new(false),
            publication: Arc::downgrade(publication),
        })
    }
}

impl PendingShutdown for Generation {
    fn begin_shutdown(&self) {
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(publication) = self.publication.upgrade() {
            self.callback_outside_lock.store(publication.state.try_lock().is_ok(), Ordering::Release);
        }
    }
}

#[test]
fn abandonment_after_publication_signals_once_outside_the_lock() {
    let (publication, guard) = GenerationPublication::new();
    let generation = Generation::new(&publication);
    let cancellation = publication.cancellation_token();
    publication.publish(generation.clone());
    assert!(!cancellation.is_cancelled());
    assert_eq!(generation.calls.load(Ordering::Acquire), 0);
    drop(guard);
    assert!(cancellation.is_cancelled());
    assert_eq!(generation.calls.load(Ordering::Acquire), 1);
    assert!(generation.callback_outside_lock.load(Ordering::Acquire));
    drop(publication);
    assert_eq!(generation.calls.load(Ordering::Acquire), 1);
}

#[test]
fn abandonment_before_publication_closes_the_late_generation() {
    let (publication, guard) = GenerationPublication::new();
    let generation = Generation::new(&publication);
    let cancellation = publication.cancellation_token();
    drop(guard);
    assert!(cancellation.is_cancelled());
    assert_eq!(generation.calls.load(Ordering::Acquire), 0);
    publication.publish(generation.clone());
    assert_eq!(generation.calls.load(Ordering::Acquire), 1);
    assert!(generation.callback_outside_lock.load(Ordering::Acquire));
    drop(publication);
    assert_eq!(generation.calls.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn cancelled_observer_closes_late_publication_exactly_once() -> TestResult {
    let (publication, guard) = GenerationPublication::new();
    let generation = Generation::new(&publication);
    let cancellation = publication.cancellation_token();
    let observer = tokio::spawn(async move {
        let _guard = guard;
        std::future::pending::<()>().await;
    });
    tokio::task::yield_now().await;
    observer.abort();
    let error = observer.await.err().ok_or("observer unexpectedly completed")?;
    assert!(error.is_cancelled());
    tokio::time::timeout(Duration::from_secs(2), cancellation.cancelled()).await?;
    publication.publish(generation.clone());
    for _ in 0..2 {
        tokio::time::timeout(Duration::from_secs(2), cancellation.cancelled()).await?;
        assert_eq!(generation.calls.load(Ordering::Acquire), 1);
    }
    assert!(generation.callback_outside_lock.load(Ordering::Acquire));
    Ok(())
}

#[test]
fn disarming_before_or_after_publication_preserves_the_live_generation() {
    for publish_first in [false, true] {
        let (publication, guard) = GenerationPublication::new();
        let generation = Generation::new(&publication);
        let cancellation = publication.cancellation_token();
        if publish_first {
            publication.publish(generation.clone());
        }
        guard.disarm();
        if !publish_first {
            publication.publish(generation.clone());
        }
        drop(publication);
        assert!(!cancellation.is_cancelled());
        assert_eq!(generation.calls.load(Ordering::Acquire), 0);
    }
}

#[test]
fn concurrent_abandonment_and_publication_have_one_shutdown_signal() -> TestResult {
    for _ in 0..32 {
        let (publication, guard) = GenerationPublication::new();
        let generation = Generation::new(&publication);
        let cancellation = publication.cancellation_token();
        let barrier = Arc::new(Barrier::new(2));
        let abandon_barrier = Arc::clone(&barrier);
        let observer = std::thread::spawn(move || {
            abandon_barrier.wait();
            drop(guard);
        });
        barrier.wait();
        publication.publish(generation.clone());
        observer.join().map_err(|_| "admission observer panicked")?;
        assert!(cancellation.is_cancelled());
        assert_eq!(generation.calls.load(Ordering::Acquire), 1);
    }
    Ok(())
}

#[test]
fn admitted_generation_guard_signals_shutdown_on_drop() {
    let (publication, unused_guard) = GenerationPublication::new();
    unused_guard.disarm();
    let generation = Generation::new(&publication);
    let guard = GenerationAdmissionGuard::for_generation(generation.clone());
    assert_eq!(generation.calls.load(Ordering::Acquire), 0);
    drop(guard);
    assert_eq!(generation.calls.load(Ordering::Acquire), 1);
}
