use super::*;
use crate::agent::AgentStatus;
use crate::session::SessionIo;
use crate::session::handlers;
use crate::session::tests::make_session_and_context;
use anyhow::Context;
use codex_protocol::error::CodexErrorDetails;
use pretty_assertions::assert_eq;
use std::sync::Arc;

fn closed_io(termination: SessionLoopTermination) -> SessionIo {
    let (tx_sub, rx_sub) = async_channel::unbounded();
    drop(rx_sub);
    let (_tx_event, rx_event) = async_channel::unbounded();
    SessionIo {
        tx_sub,
        rx_event,
        agent_status: tokio::sync::watch::channel(AgentStatus::PendingInit).1,
        session_loop_termination: termination,
    }
}

#[tokio::test]
async fn cancelled_observer_does_not_erase_terminal_cleanup_failure() -> anyhow::Result<()> {
    let (release, held) = tokio::sync::oneshot::channel();
    let termination = spawn_session_loop(async move {
        let _ = held.await;
        Err(SessionShutdownFailure::McpRuntime)
    });
    let io = Arc::new(closed_io(termination));
    let observer = {
        let io = Arc::clone(&io);
        tokio::spawn(async move { io.shutdown_and_wait().await })
    };
    tokio::task::yield_now().await;
    observer.abort();
    let error = observer.await.err().context("observer should be cancelled")?;
    assert!(error.is_cancelled());
    release.send(()).map_err(|_| anyhow::anyhow!("retained loop disappeared"))?;
    let (first, second) = tokio::join!(io.shutdown_and_wait(), io.shutdown_and_wait());
    for result in [first, second, io.shutdown_and_wait().await] {
        let error = result.err().context("cleanup failure should remain sticky")?;
        assert!(matches!(error.details(), CodexErrorDetails::ShutdownFailed));
    }
    Ok(())
}

#[tokio::test]
async fn joined_loop_panic_is_not_clean_termination() {
    let termination = spawn_session_loop(async { panic!("controlled loop panic") });
    assert_eq!(termination.clone().await, Err(SessionShutdownFailure::LoopPanicked));
    assert_eq!(termination.await, Err(SessionShutdownFailure::LoopPanicked));
}

#[tokio::test]
async fn runtime_cancellation_is_retained_after_runtime_drop() -> anyhow::Result<()> {
    let fixture = std::thread::spawn(|| -> anyhow::Result<SessionLoopTermination> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let termination = runtime.block_on(async {
            spawn_session_loop(std::future::pending())
        });
        drop(runtime);
        Ok(termination)
    });
    let termination = fixture.join().map_err(|_| anyhow::anyhow!("runtime fixture panicked"))??;
    assert_eq!(termination.clone().await, Err(SessionShutdownFailure::LoopCancelled));
    assert_eq!(termination.await, Err(SessionShutdownFailure::LoopCancelled));
    Ok(())
}

#[tokio::test]
async fn prewarm_failure_survives_full_session_cleanup_and_repeated_shutdown() {
    let (session, _context) = make_session_and_context().await;
    let session = Arc::new(session);
    let owner = RetainedTask::spawn(&tokio::runtime::Handle::current(), async {
        panic!("controlled prewarm panic");
    });
    let previous = session.mcp_prewarm_task.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner).replace(owner);
    assert!(previous.is_none());
    assert_eq!(handlers::shutdown(&session, "first-shutdown".to_owned()).await,
        Err(SessionShutdownFailure::McpPrewarm));
    assert!(session.async_hook_results.is_closed());
    assert_eq!(handlers::shutdown(&session, "second-shutdown".to_owned()).await,
        Err(SessionShutdownFailure::McpPrewarm));
}
