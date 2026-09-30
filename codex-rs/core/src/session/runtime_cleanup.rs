//! Native runtime teardown, invoked once by the session lifetime supervisor.

use super::session::Session;
use codex_protocol::protocol::TurnAbortReason;
use std::sync::Arc;
use tracing::warn;

pub(super) async fn run(sess: &Arc<Session>) -> super::runtime_lifecycle::ShutdownOutcome {
    use super::runtime_lifecycle::cleanup_stage;

    let mut failures = Vec::new();
    let startup_prewarm = {
        let mut state = sess.state.lock().await;
        // Stop admission and take the current warmup together so resume cannot replace it.
        state.shutting_down = true;
        state.take_session_startup_prewarm()
    };
    cleanup_stage(
        "startup prewarm",
        async {
            if let Some(startup_prewarm) = startup_prewarm {
                startup_prewarm.abort().await;
            }
        },
        &mut failures,
    )
    .await;
    cleanup_stage(
        "realtime",
        async {
            let _ = sess.conversation.shutdown().await;
        },
        &mut failures,
    )
    .await;
    cleanup_stage(
        "active turns",
        sess.abort_all_tasks(TurnAbortReason::Interrupted),
        &mut failures,
    )
    .await;
    let shell_snapshot_prewarm = sess.state.lock().await.shell_snapshot_prewarm.take();
    cleanup_stage(
        "shell snapshot prewarm",
        async {
            if let Some(shell_snapshot_prewarm) = shell_snapshot_prewarm {
                shell_snapshot_prewarm.abort();
                let _ = shell_snapshot_prewarm.await;
            }
        },
        &mut failures,
    )
    .await;
    cleanup_stage("hooks", sess.hooks().shutdown(), &mut failures).await;
    sess.async_hook_results.close();
    while sess.async_hook_results.try_recv().is_ok() {}
    cleanup_stage(
        "unified exec",
        sess.services.unified_exec_manager.terminate_all_processes(),
        &mut failures,
    )
    .await;
    cleanup_stage(
        "code mode",
        async {
            if let Err(err) = sess.services.code_mode_service.shutdown().await {
                warn!("failed to shutdown code mode session: {err}");
            }
        },
        &mut failures,
    )
    .await;
    cleanup_stage("MCP prewarm", sess.stop_mcp_prewarm_worker(), &mut failures).await;
    cleanup_stage(
        "MCP runtime",
        async {
            let _refresh = sess.mcp_refresh.acquire().await;
            sess.mcp_refresh.close();
            sess.services.mcp_runtime.shutdown().await;
        },
        &mut failures,
    )
    .await;

    cleanup_stage(
        "code mode messages",
        sess.drain_code_mode_messages(),
        &mut failures,
    )
    .await;

    cleanup_stage(
        "session end hooks",
        crate::hook_runtime::run_session_end_hooks(sess),
        &mut failures,
    )
    .await;
    emit_thread_stop_lifecycle(sess, &mut failures).await;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Arc::from(format!(
            "session cleanup stages failed: {}",
            failures.join(", ")
        )))
    }
}

async fn emit_thread_stop_lifecycle(sess: &Session, failures: &mut Vec<&'static str>) {
    for contributor in sess.services.extensions.thread_lifecycle_contributors() {
        super::runtime_lifecycle::cleanup_stage(
            "thread stop contributor",
            async {
                contributor
                    .on_thread_stop(codex_extension_api::ThreadStopInput {
                        session_store: &sess.services.session_extension_data,
                        thread_store: &sess.services.thread_extension_data,
                    })
                    .await;
            },
            failures,
        )
        .await;
    }
}
