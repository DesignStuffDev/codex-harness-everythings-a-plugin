use super::*;
use super::super::HomeAdmission;
use crate::startup_sync::CuratedCallbackObservation;
use crate::startup_sync::CuratedCallbackScope;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Semaphore;

const BUDGET: Duration = Duration::from_secs(/*secs*/ 5);

// Early returns release bounded fixture work and join the exact owned thread.
// Production observation itself never acquires this handle.
struct FixtureOwner {
    gate: Arc<WorkerGate>,
    release: Option<mpsc::Sender<()>>,
}

impl Drop for FixtureOwner {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        self.gate.request_stop(Instant::now() + BUDGET);
        let handle = {
            let mut state = self.gate.lock();
            state.active.as_mut().and_then(|record| record.handle.take())
        };
        if let Some(handle) = handle {
            let joined = handle.join();
            assert!(joined.is_ok() || std::thread::panicking());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observation_preserves_live_control_handle_and_replacement_admission() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let gate = Arc::new(WorkerGate::default());
    let (entered, ready) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let (control_sent, control_seen) = mpsc::channel();
    let owner = FixtureOwner { gate: Arc::clone(&gate), release: Some(release) };
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), /*callback*/ None, move |control| {
        let _ = control_sent.send(Arc::clone(&control));
        let _ = entered.send(());
        released.recv_timeout(BUDGET).map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
        Ok(())
    })?, HomeAdmission::Started);
    ready.recv_timeout(BUDGET)?;
    let control = control_seen.recv_timeout(BUDGET)?;
    let handle_id = {
        let state = gate.lock();
        state.active.as_ref().and_then(|record| record.handle.as_ref())
            .context("registered native handle")?.thread().id()
    };
    let before = gate.observe();
    assert_eq!(before, CuratedSyncLifecycleObservation {
        generation: Some(1), admission_closed: false,
        native: CuratedSyncNativeCompletion::Running, native_handle_finished: Some(false), operation: None,
        quarantined: false, unexpected_handles: 0,
    });
    assert_eq!(gate.observe(), before);
    assert!(!control.is_cancelled());
    assert_eq!({
        let state = gate.lock();
        state.active.as_ref().and_then(|record| record.handle.as_ref())
            .context("observation retained handle")?.thread().id()
    }, handle_id);
    let scope = CuratedCallbackScope::new(tokio::runtime::Handle::current());
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(scope.callback(|| async {})),
        |_| panic!("observation must not require another worker"))?, HomeAdmission::Subscribed);
    scope.begin_close();
    drop(owner);
    assert!(scope.wait_until(Instant::now() + BUDGET).await.is_complete());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_success_replays_without_stop_and_does_not_claim_callback_or_join_completion() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let gate = Arc::new(WorkerGate::default());
    let _owner = FixtureOwner { gate: Arc::clone(&gate), release: None };
    gate.start_for_home(home.path().to_path_buf(), /*callback*/ None, |_| Ok(()))?;
    let succeeded = tokio::time::timeout(BUDGET, async {
        loop {
            let snapshot = gate.observe();
            if snapshot.operation == Some(CuratedSyncOperationDisposition::Succeeded)
                && snapshot.native_handle_finished == Some(true) {
                break snapshot;
            }
            tokio::task::yield_now().await;
        }
    }).await?;
    assert!(!succeeded.admission_closed);
    assert_eq!(succeeded.native, CuratedSyncNativeCompletion::Running);
    let scope = CuratedCallbackScope::new(tokio::runtime::Handle::current());
    let entered = Arc::new(Semaphore::new(/*permits*/ 0));
    let release = Arc::new(Semaphore::new(/*permits*/ 0));
    let callback = scope.callback({
        let (entered, release) = (Arc::clone(&entered), Arc::clone(&release));
        move || {
            let (entered, release) = (Arc::clone(&entered), Arc::clone(&release));
            async move {
                entered.add_permits(/*n*/ 1);
                if let Ok(Ok(permit)) = tokio::time::timeout(BUDGET, release.acquire()).await {
                    permit.forget();
                }
            }
        }
    });
    assert_eq!(gate.start_for_home(home.path().to_path_buf(), Some(callback),
        |_| panic!("latched success must replay"))?, HomeAdmission::Replayed);
    tokio::time::timeout(BUDGET, entered.acquire()).await??.forget();
    assert_eq!(gate.observe(), succeeded);
    assert_eq!(scope.wait_until(Instant::now()).await, CuratedCallbackObservation {
        pending: 1, completed: 0, suppressed: 0, failed: 0,
    });
    release.add_permits(/*n*/ 1);
    assert_eq!(scope.wait_until(Instant::now() + BUDGET).await, CuratedCallbackObservation {
        pending: 0, completed: 1, suppressed: 0, failed: 0,
    });
    let deadline = Instant::now() + BUDGET;
    let joined = gate.begin_stop(deadline).wait_until(deadline).await;
    assert_eq!(joined.generation, succeeded.generation);
    assert_eq!(joined.native, CuratedSyncNativeCompletion::Joined);
    assert!(joined.sync_succeeded());
    assert_eq!(gate.observe(), CuratedSyncLifecycleObservation {
        admission_closed: true, native: CuratedSyncNativeCompletion::Joined,
        native_handle_finished: None, ..succeeded
    });
    Ok(())
}

#[tokio::test]
async fn observation_preserves_quarantine_and_keeps_error_payload_private() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let _owner = FixtureOwner { gate: Arc::clone(&gate), release: None };
    gate.start(|_| Err(SyncFailure::Publication("fixture-private-path-and-token".to_owned())))?;
    let failed = tokio::time::timeout(BUDGET, async {
        loop {
            let snapshot = gate.observe();
            if snapshot.operation.is_some() && snapshot.native_handle_finished == Some(true) { break snapshot; }
            tokio::task::yield_now().await;
        }
    }).await?;
    assert_eq!(failed.operation, Some(CuratedSyncOperationDisposition::Uncertain));
    assert!(failed.quarantined);
    assert!(!format!("{failed:?}").contains("fixture-private"));
    assert_eq!(gate.observe(), failed);
    assert!(!gate.start(|_| panic!("observation must not clear quarantine"))?);
    let deadline = Instant::now() + BUDGET;
    let joined = gate.begin_stop(deadline).wait_until(deadline).await;
    assert!(joined.quarantined);
    assert!(joined.failure().context("original failure remains")?.to_string().contains("fixture-private"));
    assert_eq!(gate.observe().operation, failed.operation);
    Ok(())
}

#[tokio::test]
async fn explicit_retry_changes_generation_without_rewriting_prior_observation() -> anyhow::Result<()> {
    let gate = Arc::new(WorkerGate::default());
    let _owner = FixtureOwner { gate: Arc::clone(&gate), release: None };
    gate.start(|_| Err(SyncFailure::Ordinary("fixture offline".to_owned())))?;
    let first = tokio::time::timeout(BUDGET, async {
        loop {
            let snapshot = gate.observe();
            if snapshot.operation.is_some() && snapshot.native_handle_finished == Some(true) { break snapshot; }
            tokio::task::yield_now().await;
        }
    }).await?;
    assert_eq!(first.operation, Some(CuratedSyncOperationDisposition::Failed));
    tokio::time::timeout(BUDGET, async {
        loop {
            if gate.start(|_| Ok(()))? { break anyhow::Ok(()); }
            tokio::task::yield_now().await;
        }
    }).await??;
    let second = tokio::time::timeout(BUDGET, async {
        loop {
            let snapshot = gate.observe();
            if snapshot.operation.is_some() && snapshot.native_handle_finished == Some(true) { break snapshot; }
            tokio::task::yield_now().await;
        }
    }).await?;
    assert_eq!((first.generation, second.generation), (Some(1), Some(2)));
    assert_eq!(second.operation, Some(CuratedSyncOperationDisposition::Succeeded));
    assert!(!second.admission_closed);
    assert_eq!(first.operation, Some(CuratedSyncOperationDisposition::Failed));
    let deadline = Instant::now() + BUDGET;
    assert!(gate.begin_stop(deadline).wait_until(deadline).await.sync_succeeded());
    Ok(())
}
