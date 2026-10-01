use super::Generation;
use super::Phase;
use super::SyncFailure;
use super::WorkerGate;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

fn wait_for_finished(gate: &WorkerGate) -> anyhow::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let finished = gate
            .lock()
            .active
            .as_ref()
            .and_then(|record| record.handle.as_ref())
            .is_some_and(std::thread::JoinHandle::is_finished);
        if finished {
            return Ok(());
        }
        anyhow::ensure!(Instant::now() < deadline, "worker did not finish");
        std::thread::yield_now();
    }
}

// Test fixtures have no child processes. Joining their exact native workers is
// fixture cleanup, not a production quarantine recovery operation.
fn join_fixture(gate: &WorkerGate) -> anyhow::Result<()> {
    let handles = {
        let mut state = gate.lock();
        let mut handles = std::mem::take(&mut state.unexpected_handles);
        if let Some(handle) = state
            .active
            .as_mut()
            .and_then(|record| record.handle.take())
        {
            handles.push(handle);
        }
        handles
    };
    for handle in handles {
        anyhow::ensure!(handle.join().is_ok(), "fixture worker panicked");
    }
    Ok(())
}

#[test]
fn completed_worker_waits_for_handle_and_stale_outcome_cannot_change_retry() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let (completed_tx, completed_rx) = mpsc::channel();
    let started = gate.start_with_spawn(
        || Err(SyncFailure::Ordinary("first attempt failed".to_string())),
        |job| {
            let handle = std::thread::spawn(move || {
                job();
                let _ = completed_tx.send(());
            });
            completed_rx
                .recv_timeout(Duration::from_secs(5))
                .map_err(io::Error::other)?;
            {
                let state = gate.lock();
                let record = state
                    .active
                    .as_ref()
                    .ok_or_else(|| io::Error::other("missing generation"))?;
                assert_eq!(record.phase, Phase::Running);
                assert!(matches!(
                    &record.outcome,
                    Some(Err(SyncFailure::Ordinary(_)))
                ));
                assert!(record.handle.is_none());
            }
            assert!(
                !gate.start(|| Ok(()))?,
                "completion without a handle must not admit a retry"
            );
            Ok(handle)
        },
    )?;
    assert!(started);
    wait_for_finished(&gate)?;
    let first = gate
        .lock()
        .active
        .as_ref()
        .context("first generation")?
        .generation;
    let (release_tx, release_rx) = mpsc::channel();
    assert!(gate.start(move || {
        release_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
        Ok(())
    })?);
    let second = gate
        .lock()
        .active
        .as_ref()
        .context("second generation")?
        .generation;
    assert_ne!(first, second);
    gate.complete(first, Err(SyncFailure::Unwind));
    {
        let state = gate.lock();
        let record = state.active.as_ref().context("second generation remains")?;
        assert_eq!(record.generation, second);
        assert_eq!(record.phase, Phase::Running);
        assert!(record.outcome.is_none());
        assert!(record.handle.is_some());
    }
    release_tx.send(())?;
    wait_for_finished(&gate)?;
    gate.complete(
        first,
        Err(SyncFailure::Ordinary("late failure".to_string())),
    );
    assert_eq!(
        gate.lock().active.as_ref().context("success latch")?.phase,
        Phase::SuccessLatched
    );
    join_fixture(&gate)
}

#[test]
fn success_latches_across_later_callers_and_retains_native_handle() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    assert!(gate.start(|| Ok(()))?);
    wait_for_finished(&gate)?;
    let effects = Arc::new(AtomicUsize::new(0));
    for _home in ["first-home", "another-home"] {
        let effects = Arc::clone(&effects);
        assert!(!gate.start(move || {
            effects.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })?);
    }
    assert_eq!(effects.load(Ordering::SeqCst), 0);
    {
        let state = gate.lock();
        let record = state.active.as_ref().context("latched generation")?;
        assert_eq!(record.phase, Phase::SuccessLatched);
        assert!(record.handle.is_some());
        assert!(matches!(&record.outcome, Some(Ok(()))));
    }
    join_fixture(&gate)
}

#[test]
fn unknown_observation_survives_late_success_and_dropped_observer() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    assert!(gate.start(|| Err(SyncFailure::Quarantined))?);
    wait_for_finished(&gate)?;
    let generation = {
        let state = gate.lock();
        let record = state.active.as_ref().context("quarantined generation")?;
        let observer = record.outcome.clone();
        drop(observer);
        record.generation
    };
    gate.complete(generation, Ok(()));
    assert!(!gate.start(|| Ok(()))?);
    {
        let state = gate.lock();
        let record = state.active.as_ref().context("quarantine remains")?;
        assert_eq!(record.phase, Phase::Quarantined);
        assert!(matches!(
            &record.outcome,
            Some(Err(SyncFailure::Quarantined))
        ));
        assert!(
            record
                .handle
                .as_ref()
                .is_some_and(std::thread::JoinHandle::is_finished)
        );
    }
    join_fixture(&gate)
}

#[test]
fn unwind_anywhere_in_worker_quarantines_the_generation() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    assert!(gate.start(|| panic!("fixture callback panic"))?);
    wait_for_finished(&gate)?;
    assert!(!gate.start(|| Ok(()))?);
    {
        let state = gate.lock();
        let record = state.active.as_ref().context("unwind generation")?;
        assert_eq!(record.phase, Phase::Quarantined);
        assert!(matches!(&record.outcome, Some(Err(SyncFailure::Unwind))));
        assert!(record.handle.is_some());
    }
    join_fixture(&gate)
}

struct DropProbe {
    gate: Arc<WorkerGate>,
    dropped: mpsc::Sender<bool>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        let unlocked = self.gate.state.try_lock().is_ok();
        let _ = self.dropped.send(unlocked);
    }
}

#[test]
fn spawn_failure_and_rejected_admission_drop_captures_outside_gate_lock() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let (dropped_tx, dropped_rx) = mpsc::channel();
    let probe = DropProbe {
        gate: Arc::clone(&gate),
        dropped: dropped_tx.clone(),
    };
    let error = gate.start_with_spawn(
        move || {
            drop(probe);
            Ok(())
        },
        |_| Err(io::Error::other("fixture spawn failure")),
    );
    assert!(error.is_err());
    assert!(dropped_rx.recv_timeout(Duration::from_secs(5))?);
    assert!(gate.lock().active.is_none());
    assert!(gate.start(|| Ok(()))?);
    wait_for_finished(&gate)?;
    assert_eq!(
        gate.lock()
            .active
            .as_ref()
            .context("replacement generation")?
            .generation,
        Generation(2)
    );
    let probe = DropProbe {
        gate: Arc::clone(&gate),
        dropped: dropped_tx,
    };
    assert!(!gate.start(move || {
        drop(probe);
        Ok(())
    })?);
    assert!(dropped_rx.recv_timeout(Duration::from_secs(5))?);
    join_fixture(&gate)
}

#[test]
fn stale_handle_is_retained_without_overwriting_the_current_worker() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    assert!(gate.start(|| Ok(()))?);
    wait_for_finished(&gate)?;
    let actual_worker = gate
        .lock()
        .active
        .as_ref()
        .and_then(|record| record.handle.as_ref())
        .context("registered native handle")?
        .thread()
        .id();
    let stale = std::thread::spawn(|| {});
    let stale_worker = stale.thread().id();
    gate.attach(Generation(0), stale);
    {
        let state = gate.lock();
        assert_eq!(state.unexpected_handles.len(), 1);
        assert_eq!(state.unexpected_handles[0].thread().id(), stale_worker);
        assert_eq!(
            state
                .active
                .as_ref()
                .and_then(|record| record.handle.as_ref())
                .context("original handle remains")?
                .thread()
                .id(),
            actual_worker,
        );
    }
    assert!(!gate.start(|| Ok(()))?);
    join_fixture(&gate)
}
