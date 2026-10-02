use super::CuratedSyncNativeCompletion;
use super::CuratedSyncWorkerObservation;
use super::SyncFailure;
use super::WorkerGate;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::io;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

const FIXTURE_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 5);

fn assert_joined(observation: &CuratedSyncWorkerObservation) {
    assert_eq!(observation.native, CuratedSyncNativeCompletion::Joined);
    assert!(!observation.is_pending());
    assert_eq!(observation.unexpected_handles, 0);
}

#[tokio::test]
async fn stop_before_start_returns_idle_without_admitting_work() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let stop = gate.begin_stop(Instant::now());
    let observation = stop.wait_until(Instant::now()).await;
    assert_eq!(observation.native, CuratedSyncNativeCompletion::Idle);
    assert_eq!(observation.generation, None);
    assert!(!observation.is_pending());
    assert!(!gate.start(|_| panic!("closed gate admitted work"))?);
    Ok(())
}

#[tokio::test]
async fn completed_job_is_pending_until_its_exact_handle_is_attached() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let (completed_tx, completed_rx) = mpsc::channel();
    assert!(gate.start_with_spawn(
        |_| Ok(()),
        |job| {
            let handle = std::thread::spawn(move || {
                job();
                let _ = completed_tx.send(());
            });
            completed_rx
                .recv_timeout(FIXTURE_TIMEOUT)
                .map_err(io::Error::other)?;
            let _stop = gate.begin_stop(Instant::now() + FIXTURE_TIMEOUT);
            let observation = gate.poll_stop();
            assert_eq!(
                observation.native,
                CuratedSyncNativeCompletion::AwaitingHandle
            );
            assert!(observation.sync_succeeded());
            assert!(observation.is_pending());
            Ok(handle)
        }
    )?);
    let stop = gate.begin_stop(Instant::now() + FIXTURE_TIMEOUT);
    let observation = stop.wait_until(Instant::now() + FIXTURE_TIMEOUT).await;
    assert_joined(&observation);
    assert_eq!(observation.generation, Some(1));
    assert!(observation.sync_succeeded());
    assert!(!gate.start(|_| Ok(()))?);
    Ok(())
}

#[tokio::test]
async fn timeout_and_cancelled_observer_preserve_worker_during_native_teardown()
-> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let (completed_tx, completed_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    assert!(gate.start_with_spawn(
        |_| Ok(()),
        |job| {
            Ok(std::thread::spawn(move || {
                job();
                let _ = completed_tx.send(());
                // Models teardown after the job's outcome, before native exit.
                let _ = release_rx.recv_timeout(FIXTURE_TIMEOUT);
            }))
        }
    )?);
    completed_rx.recv_timeout(FIXTURE_TIMEOUT)?;
    let stop = gate.begin_stop(Instant::now() + FIXTURE_TIMEOUT);
    let pending = stop.wait_until(Instant::now()).await;
    assert!(pending.is_pending());
    assert!(pending.sync_succeeded());
    let observer = tokio::spawn({
        let stop = stop.clone();
        async move { stop.wait_until(Instant::now() + FIXTURE_TIMEOUT).await }
    });
    tokio::task::yield_now().await;
    observer.abort();
    let error = observer
        .await
        .err()
        .context("cancelled observer unexpectedly completed")?;
    assert!(error.is_cancelled());
    assert!(
        gate.lock()
            .active
            .as_ref()
            .context("retained worker")?
            .handle
            .is_some()
    );
    release_tx.send(())?;
    let joined = stop.wait_until(Instant::now() + FIXTURE_TIMEOUT).await;
    assert_joined(&joined);
    assert!(joined.sync_succeeded());
    Ok(())
}

#[test]
fn concurrent_observers_share_one_join_and_preserve_original_uncertainty() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    assert!(gate.start(|_| Err(SyncFailure::Quarantined))?);
    let deadline = Instant::now() + FIXTURE_TIMEOUT;
    while !gate
        .lock()
        .active
        .as_ref()
        .and_then(|record| record.handle.as_ref())
        .is_some_and(std::thread::JoinHandle::is_finished)
    {
        anyhow::ensure!(Instant::now() < deadline, "fixture worker did not finish");
        std::thread::yield_now();
    }
    let _stop = gate.begin_stop(deadline);
    let barrier = Arc::new(Barrier::new(/*n*/ 3));
    let observers: Vec<_> = (0..2)
        .map(|_| {
            let gate = Arc::clone(&gate);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                loop {
                    let observation = gate.poll_stop();
                    if !observation.is_pending() || Instant::now() >= deadline {
                        return observation;
                    }
                    std::thread::yield_now();
                }
            })
        })
        .collect();
    barrier.wait();
    for observer in observers {
        let observation = observer
            .join()
            .map_err(|_| anyhow::anyhow!("observer panicked"))?;
        assert_joined(&observation);
        assert!(observation.quarantined);
        assert!(matches!(
            observation.outcome,
            Some(Err(SyncFailure::Quarantined))
        ));
    }
    assert!(
        gate.lock()
            .active
            .as_ref()
            .context("joined record")?
            .handle
            .is_none()
    );
    assert!(!gate.start(|_| Ok(()))?);
    Ok(())
}

#[tokio::test]
async fn spawn_failure_after_stop_is_explicit_and_never_looks_like_a_join() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let result = gate.start_with_spawn(
        |_| Ok(()),
        |_| {
            let _stop = gate.begin_stop(Instant::now());
            Err(io::Error::other("fixture spawn failure"))
        },
    );
    assert!(result.is_err());
    let stop = gate.begin_stop(Instant::now());
    let observation = stop.wait_until(Instant::now()).await;
    assert_eq!(
        observation.native,
        CuratedSyncNativeCompletion::SpawnFailed {
            kind: io::ErrorKind::Other,
        }
    );
    assert_eq!(observation.generation, Some(1));
    assert!(!observation.is_pending());
    assert!(observation.outcome.is_none());
    assert!(!gate.start(|_| Ok(()))?);
    Ok(())
}

struct PanicPayload {
    gate: Arc<WorkerGate>,
    dropped: mpsc::Sender<bool>,
}

impl Drop for PanicPayload {
    fn drop(&mut self) {
        let _ = self.dropped.send(self.gate.state.try_lock().is_ok());
    }
}

#[tokio::test]
async fn native_panic_after_success_is_separate_and_payload_drops_outside_mutex()
-> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let (completed_tx, completed_rx) = mpsc::channel();
    let (dropped_tx, dropped_rx) = mpsc::channel();
    let payload = PanicPayload {
        gate: Arc::clone(&gate),
        dropped: dropped_tx,
    };
    assert!(gate.start_with_spawn(
        |_| Ok(()),
        |job| {
            Ok(std::thread::spawn(move || {
                job();
                let _ = completed_tx.send(());
                std::panic::panic_any(payload);
            }))
        }
    )?);
    completed_rx.recv_timeout(FIXTURE_TIMEOUT)?;
    let stop = gate.begin_stop(Instant::now() + FIXTURE_TIMEOUT);
    let observation = stop.wait_until(Instant::now() + FIXTURE_TIMEOUT).await;
    assert_eq!(observation.native, CuratedSyncNativeCompletion::Panicked);
    assert!(observation.sync_succeeded());
    assert!(observation.quarantined);
    assert!(!observation.is_pending());
    assert!(dropped_rx.recv_timeout(FIXTURE_TIMEOUT)?);
    Ok(())
}

#[cfg(target_os = "linux")]
struct NativeTeardown {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

#[cfg(target_os = "linux")]
impl Drop for NativeTeardown {
    fn drop(&mut self) {
        let _ = self.entered.send(());
        let _ = self.release.recv_timeout(FIXTURE_TIMEOUT);
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn stop_during_retry_join_keeps_the_exact_joined_record() -> anyhow::Result<()> {
    thread_local! {
        static TEARDOWN: std::cell::RefCell<Option<NativeTeardown>> = const {
            std::cell::RefCell::new(/*value*/ None)
        };
    }
    let gate = Arc::new(WorkerGate::default());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    assert!(gate.start_with_spawn(
        |_| Err(SyncFailure::Ordinary("retryable fixture".to_string())),
        |job| Ok(std::thread::spawn(move || {
            TEARDOWN.with(|slot| {
                *slot.borrow_mut() = Some(NativeTeardown {
                    entered: entered_tx,
                    release: release_rx,
                });
            });
            job();
        })),
    )?);
    entered_rx.recv_timeout(FIXTURE_TIMEOUT)?;
    let deadline = Instant::now() + FIXTURE_TIMEOUT;
    while !gate
        .lock()
        .active
        .as_ref()
        .and_then(|record| record.handle.as_ref())
        .is_some_and(std::thread::JoinHandle::is_finished)
    {
        anyhow::ensure!(
            Instant::now() < deadline,
            "native handle did not become joinable"
        );
        std::thread::yield_now();
    }
    let retry = std::thread::spawn({
        let gate = Arc::clone(&gate);
        move || gate.start(|_| panic!("stop must prevent retry"))
    });
    while gate
        .lock()
        .active
        .as_ref()
        .context("joining record")?
        .native_completion
        != CuratedSyncNativeCompletion::Joining
    {
        anyhow::ensure!(Instant::now() < deadline, "retry did not enter exact join");
        std::thread::yield_now();
    }
    let stop = gate.begin_stop(deadline);
    let pending = stop.wait_until(Instant::now()).await;
    assert_eq!(pending.native, CuratedSyncNativeCompletion::Joining);
    assert!(pending.is_pending());
    release_tx.send(())?;
    assert!(
        !retry
            .join()
            .map_err(|_| anyhow::anyhow!("retry fixture panicked"))??
    );
    let observation = stop.wait_until(deadline).await;
    assert_joined(&observation);
    assert_eq!(observation.generation, Some(1));
    assert!(matches!(
        observation.outcome,
        Some(Err(SyncFailure::Ordinary(_)))
    ));
    Ok(())
}
