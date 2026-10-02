use super::*;
use super::super::WorkerGate;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Semaphore;

const BUDGET: Duration = Duration::from_secs(/*secs*/ 5);

// Release and join the exact native fixture handle on early assertion/? paths.
struct HeldWorker {
    gate: Arc<WorkerGate>,
    release: Option<mpsc::Sender<()>>,
}
impl HeldWorker {
    fn release(&mut self) -> anyhow::Result<()> {
        self.release.take().context("fixture already released")?.send(())?;
        Ok(())
    }
}
impl Drop for HeldWorker {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() { let _ = release.send(()); }
        self.gate.request_stop(Instant::now() + BUDGET);
        let handle = self.gate.lock().active.as_mut().and_then(|record| record.handle.take());
        if let Some(handle) = handle {
            let result = handle.join();
            assert!(result.is_ok() || std::thread::panicking(), "fixture worker panicked");
        }
    }
}

fn scope() -> Arc<CuratedCallbackScope> {
    CuratedCallbackScope::new(tokio::runtime::Handle::current())
}

fn counter_callback(
    scope: &Arc<CuratedCallbackScope>,
    calls: &Arc<AtomicUsize>,
    done: &Arc<Semaphore>,
) -> CuratedSyncCallback {
    scope.callback({
        let (calls, done) = (Arc::clone(calls), Arc::clone(done));
        move || {
            let (calls, done) = (Arc::clone(&calls), Arc::clone(&done));
            async move {
                calls.fetch_add(/*val*/ 1, Ordering::SeqCst);
                done.add_permits(/*n*/ 1);
            }
        }
    })
}

#[tokio::test]
async fn held_same_home_worker_delivers_once_to_replacement_without_old_scope_effects() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let snapshot = home.path().join("snapshot");
    std::fs::write(&snapshot, "old")?;
    let (first, replacement) = (scope(), scope());
    let (first_calls, second_calls) = (Arc::new(AtomicUsize::new(/*v*/ 0)), Arc::new(AtomicUsize::new(/*v*/ 0)));
    let done = Arc::new(Semaphore::new(/*permits*/ 0));
    let gate = Arc::new(WorkerGate::default());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    assert_eq!(gate.start_for_home(
        home.path().to_path_buf(), Some(counter_callback(&first, &first_calls, &done)),
        { let snapshot = snapshot.clone(); move |_| {
            let _ = entered_tx.send(());
            release_rx.recv_timeout(BUDGET).map_err(|err| super::super::SyncFailure::Ordinary(err.to_string()))?;
            std::fs::write(snapshot, "new").map_err(|err| super::super::SyncFailure::Ordinary(err.to_string()))?;
            Ok(())
        } },
    )?, HomeAdmission::Started);
    let mut held = HeldWorker { gate: Arc::clone(&gate), release: Some(release_tx) };
    entered_rx.recv_timeout(BUDGET)?;
    let generation = gate.lock().active.as_ref().context("active generation")?.generation;
    first.begin_close();
    assert_eq!(std::fs::read_to_string(&snapshot)?, "old");
    let seen_snapshot = Arc::new(std::sync::Mutex::new(String::new()));
    let callback = replacement.callback({
        let (snapshot, seen, calls, done) = (snapshot.clone(), Arc::clone(&seen_snapshot), Arc::clone(&second_calls), Arc::clone(&done));
        move || {
            let (snapshot, seen, calls, done) = (snapshot.clone(), Arc::clone(&seen), Arc::clone(&calls), Arc::clone(&done));
            async move {
                let text = std::fs::read_to_string(snapshot).unwrap_or_default();
                *seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = text;
                calls.fetch_add(/*val*/ 1, Ordering::SeqCst);
                done.add_permits(/*n*/ 1);
            }
        }
    });
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(callback.clone()), |_| panic!("replacement started duplicate worker"))?, HomeAdmission::Subscribed);
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(callback.clone()), |_| panic!("duplicate started worker"))?, HomeAdmission::AlreadyRegistered);
    held.release()?;
    tokio::time::timeout(BUDGET, done.acquire()).await??.forget();
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(callback), |_| panic!("success latch reopened"))?, HomeAdmission::AlreadyRegistered);
    let result = gate.begin_stop(Instant::now() + BUDGET).wait_until(Instant::now() + BUDGET).await;
    assert_eq!(result.generation, Some(generation.0));
    assert!(result.sync_succeeded());
    assert!(!result.is_pending());
    assert_eq!((first_calls.load(Ordering::SeqCst), second_calls.load(Ordering::SeqCst)), (0, 1));
    assert_eq!(*seen_snapshot.lock().unwrap_or_else(std::sync::PoisonError::into_inner), "new");
    assert!(first.wait_until(Instant::now() + BUDGET).await.is_complete());
    assert!(replacement.wait_until(Instant::now() + BUDGET).await.is_complete());
    Ok(())
}

#[tokio::test]
async fn completion_before_registration_replays_only_to_same_home_once() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let other_home = tempfile::tempdir()?;
    let gate = Arc::new(WorkerGate::default());
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), /*callback*/ None, |_| Ok(()))?, HomeAdmission::Started);
    let _owner = HeldWorker { gate: Arc::clone(&gate), release: None };
    tokio::time::timeout(BUDGET, async {
        while gate.lock().active.as_ref().is_some_and(|record| record.outcome.is_none()) { tokio::task::yield_now().await; }
    }).await?;
    let (same, other) = (scope(), scope());
    let calls = Arc::new(AtomicUsize::new(/*v*/ 0));
    let done = Arc::new(Semaphore::new(/*permits*/ 0));
    assert_eq!(gate.start_for_home(other_home.path().to_path_buf(), Some(counter_callback(&other, &calls, &done)), |_| panic!("different home started"))?, HomeAdmission::UnsupportedHome);
    let callback = counter_callback(&same, &calls, &done);
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(callback.clone()), |_| panic!("replay started worker"))?, HomeAdmission::Replayed);
    tokio::time::timeout(BUDGET, done.acquire()).await??.forget();
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(callback), |_| panic!("duplicate replay"))?, HomeAdmission::AlreadyRegistered);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(same.wait_until(Instant::now() + BUDGET).await.is_complete());
    assert!(other.wait_until(Instant::now() + BUDGET).await.is_complete());
    Ok(())
}

#[tokio::test]
async fn native_stop_before_completion_rejects_callback_without_erasing_outcome() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let scope = scope();
    let calls = Arc::new(AtomicUsize::new(/*v*/ 0));
    let done = Arc::new(Semaphore::new(/*permits*/ 0));
    let gate = Arc::new(WorkerGate::default());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    gate.start_for_home(home.path().to_path_buf(), Some(counter_callback(&scope, &calls, &done)), move |_| {
        let _ = entered_tx.send(());
        release_rx.recv_timeout(BUDGET).map_err(|err| super::super::SyncFailure::Ordinary(err.to_string()))?;
        Ok(())
    })?;
    let mut held = HeldWorker { gate: Arc::clone(&gate), release: Some(release_tx) };
    entered_rx.recv_timeout(BUDGET)?;
    let deadline = Instant::now() + BUDGET;
    let stopped = gate.begin_stop(deadline);
    held.release()?;
    let outcome = stopped.wait_until(deadline).await;
    assert!(outcome.sync_succeeded(), "original work result is immutable despite later stop");
    assert!(!outcome.is_pending());
    assert!(scope.wait_until(deadline).await.is_complete());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    Ok(())
}

struct DropProbe {
    gate: std::sync::Weak<WorkerGate>,
    observed: mpsc::Sender<bool>,
}
impl Drop for DropProbe {
    fn drop(&mut self) {
        let unlocked = self.gate.upgrade().is_some_and(|gate| gate.state.try_lock().is_ok());
        let _ = self.observed.send(unlocked);
    }
}

#[tokio::test]
async fn rejected_and_duplicate_callback_captures_drop_outside_worker_mutex() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    let scope = scope();
    let gate = Arc::new(WorkerGate::default());
    gate.start_for_home(home.path().to_path_buf(), Some(scope.callback(|| async {})), |_| Ok(()))?;
    let _owner = HeldWorker { gate: Arc::clone(&gate), release: None };
    for (path, expected) in [(home.path(), HomeAdmission::AlreadyRegistered), (other.path(), HomeAdmission::UnsupportedHome)] {
        let (observed, receive) = mpsc::channel();
        let probe = DropProbe { gate: Arc::downgrade(&gate), observed };
        let callback = scope.callback(move || {
            let _borrow_probe = &probe;
            std::future::ready(())
        });
        assert_eq!(gate.start_for_home(path.to_path_buf(), Some(callback), |_| panic!("rejected work started"))?, expected);
        assert!(receive.recv_timeout(BUDGET)?, "capture dropped while registry locked");
    }
    assert!(scope.wait_until(Instant::now() + BUDGET).await.is_complete());
    Ok(())
}

#[tokio::test]
async fn retired_scope_identity_remains_weakly_owned_until_new_scope_replay() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let gate = Arc::new(WorkerGate::default());
    let first = scope();
    let calls = Arc::new(AtomicUsize::new(/*v*/ 0));
    let done = Arc::new(Semaphore::new(/*permits*/ 0));
    gate.start_for_home(home.path().to_path_buf(), Some(counter_callback(&first, &calls, &done)), |_| Ok(()))?;
    let _owner = HeldWorker { gate: Arc::clone(&gate), release: None };
    tokio::time::timeout(BUDGET, done.acquire()).await??.forget();
    assert!(first.wait_until(Instant::now() + BUDGET).await.is_complete());
    drop(first);
    {
        let state = gate.lock();
        let delivery = state.active.as_ref().context("generation")?.delivery.as_ref().context("delivery")?;
        assert_eq!(delivery.seen.len(), 1);
        // A Weak retains the allocation identity without retaining its callback
        // body/manager. A later B allocation cannot reuse this address.
        assert_eq!(delivery.seen[0].strong_count(), 0);
    }
    let replacement = scope();
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(counter_callback(&replacement, &calls, &done)), |_| panic!("replacement restarted worker"))?, HomeAdmission::Replayed);
    tokio::time::timeout(BUDGET, done.acquire()).await??.forget();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(replacement.wait_until(Instant::now() + BUDGET).await.is_complete());
    Ok(())
}

#[tokio::test]
async fn ordinary_failure_ends_generation_subscription_and_retry_accepts_explicit_reregistration() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let gate = Arc::new(WorkerGate::default());
    let (first, second) = (scope(), scope());
    let (first_calls, second_calls) = (Arc::new(AtomicUsize::new(/*v*/ 0)), Arc::new(AtomicUsize::new(/*v*/ 0)));
    let done = Arc::new(Semaphore::new(/*permits*/ 0));
    let first_callback = counter_callback(&first, &first_calls, &done);
    let second_callback = counter_callback(&second, &second_calls, &done);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    gate.start_for_home(home.path().to_path_buf(), Some(first_callback.clone()), move |_| {
        let _ = entered_tx.send(());
        let _ = release_rx.recv_timeout(BUDGET);
        Err(super::super::SyncFailure::Ordinary("fixture sync unavailable".to_string()))
    })?;
    let mut held = HeldWorker { gate: Arc::clone(&gate), release: Some(release_tx) };
    entered_rx.recv_timeout(BUDGET)?;
    let first_generation = gate.lock().active.as_ref().context("first generation")?.generation;
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(second_callback.clone()), |_| panic!("parallel worker"))?, HomeAdmission::Subscribed);
    held.release()?;
    tokio::time::timeout(BUDGET, async {
        while !gate.lock().active.as_ref().is_some_and(|record| record.handle.as_ref().is_some_and(std::thread::JoinHandle::is_finished)) {
            tokio::task::yield_now().await;
        }
    }).await?;
    assert_eq!((first_calls.load(Ordering::SeqCst), second_calls.load(Ordering::SeqCst)), (0, 0));
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    held.release = Some(release_tx);
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(first_callback), move |_| {
        let _ = entered_tx.send(());
        release_rx.recv_timeout(BUDGET).map_err(|err| super::super::SyncFailure::Ordinary(err.to_string()))?;
        Ok(())
    })?, HomeAdmission::Started);
    entered_rx.recv_timeout(BUDGET)?;
    let second_generation = gate.lock().active.as_ref().context("retry generation")?.generation;
    assert_ne!(first_generation, second_generation);
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(second_callback), |_| panic!("resubscription restarted worker"))?, HomeAdmission::Subscribed);
    held.release()?;
    for _ in 0..2 { tokio::time::timeout(BUDGET, done.acquire()).await??.forget(); }
    let deadline = Instant::now() + BUDGET;
    let observed = gate.begin_stop(deadline).wait_until(deadline).await;
    assert_eq!(observed.generation, Some(second_generation.0));
    assert!(observed.sync_succeeded());
    assert_eq!((first_calls.load(Ordering::SeqCst), second_calls.load(Ordering::SeqCst)), (1, 1));
    assert!(first.wait_until(deadline).await.is_complete());
    assert!(second.wait_until(deadline).await.is_complete());
    Ok(())
}
