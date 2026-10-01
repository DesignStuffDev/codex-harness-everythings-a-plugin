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
        |_| Err(SyncFailure::Ordinary("first attempt failed".to_string())),
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
                !gate.start(|_| Ok(()))?,
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
    assert!(gate.start(move |_| {
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
    assert!(gate.start(|_| Ok(()))?);
    wait_for_finished(&gate)?;
    let effects = Arc::new(AtomicUsize::new(0));
    for _home in ["first-home", "another-home"] {
        let effects = Arc::clone(&effects);
        assert!(!gate.start(move |_| {
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
    assert!(gate.start(|_| Err(SyncFailure::Quarantined))?);
    wait_for_finished(&gate)?;
    let generation = {
        let state = gate.lock();
        let record = state.active.as_ref().context("quarantined generation")?;
        let observer = record.outcome.clone();
        drop(observer);
        record.generation
    };
    gate.complete(generation, Ok(()));
    assert!(!gate.start(|_| Ok(()))?);
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
    assert!(gate.start(|_| panic!("fixture callback panic"))?);
    wait_for_finished(&gate)?;
    assert!(!gate.start(|_| Ok(()))?);
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
        move |_| {
            drop(probe);
            Ok(())
        },
        |_| Err(io::Error::other("fixture spawn failure")),
    );
    assert!(error.is_err());
    assert!(dropped_rx.recv_timeout(Duration::from_secs(5))?);
    assert!(gate.lock().active.is_none());
    assert!(gate.start(|_| Ok(()))?);
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
    assert!(!gate.start(move |_| {
        drop(probe);
        Ok(())
    })?);
    assert!(dropped_rx.recv_timeout(Duration::from_secs(5))?);
    join_fixture(&gate)
}

#[test]
fn stale_handle_is_retained_without_overwriting_the_current_worker() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    assert!(gate.start(|_| Ok(()))?);
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
    assert!(!gate.start(|_| Ok(()))?);
    join_fixture(&gate)
}

#[test]
fn stop_before_first_start_closes_admission_without_spawning() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    gate.request_stop(Instant::now());
    let started = gate.start_with_spawn(
        |_| panic!("closed gate must not run work"),
        |_| panic!("closed gate must not spawn a worker"),
    )?;
    assert!(!started);
    let state = gate.lock();
    assert!(state.closing);
    assert_eq!(state.next_generation, 0);
    assert!(state.active.is_none());
    Ok(())
}

#[test]
fn stop_before_handle_attachment_keeps_the_exact_worker_owned() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let (release_tx, release_rx) = mpsc::channel();
    let deadline = Instant::now() + Duration::from_secs(2);
    assert!(gate.start_with_spawn(
        |_| panic!("stopped worker must not enter sync"),
        |job| {
            let handle = std::thread::spawn(move || {
                if release_rx.recv_timeout(Duration::from_secs(5)).is_ok() {
                    job();
                }
            });
            gate.request_stop(deadline);
            {
                let state = gate.lock();
                assert!(state.closing);
                let record = state
                    .active
                    .as_ref()
                    .ok_or_else(|| io::Error::other("reserved worker"))?;
                assert!(record.handle.is_none());
                assert!(record.control.is_cancelled());
                assert_eq!(record.control.deadline(), Some(deadline));
            }
            release_tx.send(()).map_err(io::Error::other)?;
            Ok(handle)
        },
    )?);
    wait_for_finished(&gate)?;
    {
        let state = gate.lock();
        let record = state.active.as_ref().context("stopped worker retained")?;
        assert!(state.closing);
        assert!(record.handle.is_some());
        assert!(matches!(
            &record.outcome,
            Some(Err(SyncFailure::Stopped {
                publication_may_have_occurred: false
            }))
        ));
    }
    assert!(!gate.start(|_| Ok(()))?);
    join_fixture(&gate)
}

#[test]
fn worker_receives_registered_control_and_repeated_stop_keeps_earliest_deadline()
-> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let (control_tx, control_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    assert!(gate.start(move |control| {
        control_tx
            .send(Arc::clone(&control))
            .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
        release_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
        assert!(control.is_cancelled());
        Err(SyncFailure::Stopped {
            publication_may_have_occurred: false,
        })
    })?);
    let worker_control = control_rx.recv_timeout(Duration::from_secs(5))?;
    let registered_control = Arc::clone(
        &gate
            .lock()
            .active
            .as_ref()
            .context("registered control")?
            .control,
    );
    assert!(Arc::ptr_eq(&worker_control, &registered_control));
    let first = Instant::now() + Duration::from_secs(5);
    let earlier = first - Duration::from_secs(2);
    gate.request_stop(first);
    gate.request_stop(first + Duration::from_secs(4));
    assert_eq!(worker_control.deadline(), Some(first));
    gate.request_stop(earlier);
    assert!(worker_control.is_cancelled());
    assert_eq!(worker_control.deadline(), Some(earlier));
    release_tx.send(())?;
    wait_for_finished(&gate)?;
    assert!(gate.lock().closing);
    assert!(!gate.start(|_| Ok(()))?);
    join_fixture(&gate)
}

#[test]
fn stop_after_ordinary_failure_prevents_retry_admission() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    assert!(gate.start(|_| Err(SyncFailure::Ordinary("fixture failure".to_string())))?);
    wait_for_finished(&gate)?;
    gate.request_stop(Instant::now());
    assert!(!gate.start(|_| Ok(()))?);
    {
        let state = gate.lock();
        assert!(state.closing);
        let record = state
            .active
            .as_ref()
            .context("failed worker remains owned")?;
        assert_eq!(record.generation, Generation(1));
        assert!(matches!(
            &record.outcome,
            Some(Err(SyncFailure::Ordinary(_)))
        ));
        assert!(record.handle.is_some());
    }
    join_fixture(&gate)
}

#[test]
fn spawn_failure_after_stop_cannot_reopen_admission() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let error = gate.start_with_spawn(
        |_| Ok(()),
        |_| {
            gate.request_stop(Instant::now());
            Err(io::Error::other("fixture spawn failure after stop"))
        },
    );
    assert!(error.is_err());
    assert!(gate.lock().active.is_none());
    assert!(gate.lock().closing);
    assert!(!gate.start(|_| Ok(()))?);
    Ok(())
}
