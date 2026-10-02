use super::CloseFailure;
use super::LocalProcessOwner;
use super::Registry;
use codex_utils_pty::Command;
use codex_utils_pty::ProcessMode;
use pretty_assertions::assert_eq;
use std::io;
use std::sync::Arc;
use std::sync::PoisonError;
use std::sync::mpsc;
use std::time::Duration;

fn child_command() -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "read line"]);
    command.process_mode(ProcessMode::NewGroup);
    command
}

#[tokio::test]
async fn cancelled_observer_preserves_cleanup_and_the_first_deadline() -> io::Result<()> {
    let registry = Arc::new(Registry::default());
    let (owner, stdin, stdout) = LocalProcessOwner::new_in(
        child_command().spawn()?,
        "test child".to_owned(),
        &registry,
        /*observer*/ None,
    )?;
    drop(stdin);
    drop(stdout);
    owner.record_framing(Ok(()));
    owner.begin_shutdown();
    let first_deadline = owner.state.shutdown_at.get().copied();
    let observer_owner = owner.clone();
    let observer = tokio::spawn(async move { observer_owner.wait_closed().await });
    tokio::task::yield_now().await;
    observer.abort();
    let _ = observer.await;
    owner.begin_shutdown();
    assert_eq!(owner.state.shutdown_at.get().copied(), first_deadline);
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), owner.wait_closed())
        .await.map_err(io::Error::other)??;
    assert!(owner.state.child.lock().await.is_none());
    assert!(owner.state.stderr.lock().await.is_none());
    assert!(registry.processes.lock().unwrap_or_else(PoisonError::into_inner).is_empty());
    Ok(())
}

#[tokio::test]
async fn framing_failure_does_not_skip_reaping_or_retire_failure_custody() -> io::Result<()> {
    let registry = Arc::new(Registry::default());
    let (owner, stdin, stdout) = LocalProcessOwner::new_in(
        child_command().spawn()?,
        "test child".to_owned(),
        &registry,
        /*observer*/ None,
    )?;
    drop(stdin);
    drop(stdout);
    owner.record_framing(Err(io::Error::other("original framing failure")));
    owner.record_framing(Ok(()));
    let error = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), owner.wait_closed())
        .await.map_err(io::Error::other)?
        .err().ok_or_else(|| io::Error::other("framing failure was lost"))?;
    assert_eq!(error.to_string(), CloseFailure::Framing.to_string());
    assert!(owner.state.child.lock().await.is_none());
    assert!(owner.state.stderr.lock().await.is_none());
    assert_eq!(registry.processes.lock().unwrap_or_else(PoisonError::into_inner).len(), 1);
    let errors = owner.state.errors.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(errors.iter().map(ToString::to_string).collect::<Vec<_>>(), vec!["original framing failure"]);
    Ok(())
}

#[tokio::test]
async fn missing_stdio_publishes_the_registered_owner_before_worker_creation() -> io::Result<()> {
    let registry = Arc::new(Registry::default());
    let observer_registry = Arc::clone(&registry);
    let (published_tx, published_rx) = mpsc::channel();
    let observer = Arc::new(move |owner: LocalProcessOwner| {
        let registered_outside_lock = observer_registry.processes.try_lock().is_ok_and(|processes| {
            processes.iter().any(|state| Arc::ptr_eq(state, &owner.state))
        });
        let before_worker_creation = owner.state.worker.get().is_none();
        // A roster already closing must be able to signal this generation
        // synchronously, even though its launcher will return an error.
        owner.begin_shutdown();
        let _ = published_tx.send((owner, registered_outside_lock, before_worker_creation));
    });
    let mut child = child_command().spawn()?;
    drop(child.stdout.take());
    let result = LocalProcessOwner::new_in(
        child,
        "test child".to_owned(),
        &registry,
        Some(observer),
    );
    assert!(result.is_err());
    let (owner, registered_outside_lock, before_worker_creation) =
        published_rx.try_recv().map_err(io::Error::other)?;
    assert!(registered_outside_lock);
    assert!(before_worker_creation);
    assert!(owner.is_shutdown_started());
    let error = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), owner.wait_closed())
        .await.map_err(io::Error::other)?
        .err().ok_or_else(|| io::Error::other("failed launch reported confirmed closure"))?;
    assert_eq!(error.to_string(), CloseFailure::Framing.to_string());
    assert!(owner.state.child.lock().await.is_none());
    assert!(owner.state.stderr.lock().await.is_none());
    assert_eq!(registry.processes.lock().unwrap_or_else(PoisonError::into_inner).len(), 1);
    Ok(())
}

#[tokio::test]
async fn stderr_diagnostic_does_not_block_confirmed_ownership_retirement() -> io::Result<()> {
    let registry = Arc::new(Registry::default());
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf '\\377' >&2; read line"]);
    command.process_mode(ProcessMode::NewGroup);
    let (owner, stdin, stdout) = LocalProcessOwner::new_in(
        command.spawn()?,
        "test child".to_owned(),
        &registry,
        /*observer*/ None,
    )?;
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        loop {
            if !owner.state.diagnostic_errors.lock().unwrap_or_else(PoisonError::into_inner).is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    }).await.map_err(io::Error::other)?;
    drop(stdin);
    drop(stdout);
    owner.record_framing(Ok(()));
    owner.wait_closed().await?;
    assert!(owner.state.child.lock().await.is_none());
    assert!(owner.state.stderr.lock().await.is_none());
    assert!(registry.processes.lock().unwrap_or_else(PoisonError::into_inner).is_empty());
    assert!(owner.state.errors.lock().unwrap_or_else(PoisonError::into_inner).is_empty());
    let diagnostics = owner.state.diagnostic_errors.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(diagnostics.iter().map(io::Error::kind).collect::<Vec<_>>(), vec![io::ErrorKind::InvalidData]);
    Ok(())
}

#[test]
#[expect(
    clippy::await_holding_invalid_type,
    reason = "the test reaps a child retained after its original runtime cancelled the worker"
)]
fn cancelled_worker_does_not_drop_its_child_or_stderr_reader() -> io::Result<()> {
    let registry = Arc::new(Registry::default());
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let (owner, stdin, stdout) = runtime.block_on(async {
        LocalProcessOwner::new_in(
            child_command().spawn()?,
            "test child".to_owned(),
            &registry,
            /*observer*/ None,
        )
    })?;
    drop(runtime);
    // Inspect ownership after the runtime has cancelled the actual worker,
    // rather than only cancelling a waiter of its result.
    let child_retained = owner.state.child.try_lock().is_ok_and(|slot| slot.is_some());
    let stderr_retained = owner.state.stderr.try_lock().is_ok_and(|slot| slot.is_some());
    drop(stdin);
    drop(stdout);
    let cleanup = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    cleanup.block_on(async {
        let mut child = owner.state.child.lock().await;
        if let Some(child) = child.as_mut() {
            // The original SIGCHLD stream belonged to the stopped runtime.
            // Repeated kill/wait polls still collect its cached or OS exit status.
            tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
                loop {
                    if child.kill().await.is_ok() || child.id().is_none() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(/*millis*/ 10)).await;
                }
            }).await.map_err(io::Error::other)?;
        }
        child.take();
        owner.state.stderr.lock().await.take();
        let error = owner.terminate().await.err()
            .ok_or_else(|| io::Error::other("cancelled worker reported success"))?;
        assert_eq!(error.to_string(), CloseFailure::TaskCancelled.to_string());
        Ok::<_, io::Error>(())
    })?;
    assert_eq!((child_retained, stderr_retained), (true, true));
    assert_eq!(registry.processes.lock().unwrap_or_else(PoisonError::into_inner).len(), 1);
    Ok(())
}
