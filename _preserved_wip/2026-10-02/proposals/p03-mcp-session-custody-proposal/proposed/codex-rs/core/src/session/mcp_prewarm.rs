//! Best-effort MCP prewarming.
//!
//! A bounded channel coalesces refresh requests. The worker only prepares the
//! newest thread state; exact model steps remain the correctness path.

use super::*;

impl Session {
    pub(crate) fn request_mcp_runtime_refresh(&self) {
        if self.mcp_prewarm_shutdown.is_cancelled() { return; }
        // Plugin changes can reuse connections but still change their skill resources.
        self.services.mcp_runtime.invalidate_resource_caches();
        self.request_mcp_runtime_reprojection();
    }

    /// Reproject contributor state without invalidating cached MCP resources.
    pub(crate) fn request_mcp_runtime_reprojection(&self) {
        if self.mcp_prewarm_shutdown.is_cancelled() { return; }
        self.mark_mcp_runtime_dirty();
        self.schedule_mcp_prewarm();
    }

    pub(super) fn start_mcp_prewarm_worker(
        self: &Arc<Self>,
        requests: async_channel::Receiver<()>,
        mut auth_changes: tokio::sync::watch::Receiver<u64>,
    ) {
        let session = Arc::downgrade(self);
        let shutdown = self.mcp_prewarm_shutdown.clone();
        let _worker = codex_async_utils::RetainedTask::spawn_published(&self.services.runtime_handle, async move {
            loop {
                let auth_changed = tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    request = requests.recv() => {
                        if request.is_err() {
                            break;
                        }
                        false
                    },
                    auth_change = auth_changes.changed() => {
                        if auth_change.is_err() {
                            break;
                        }
                        true
                    },
                };
                let Some(session) = session.upgrade() else {
                    break;
                };
                if auth_changed {
                    session.mark_mcp_runtime_dirty();
                }
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    _ = session.refresh_mcp_if_dirty() => {},
                }
            }
        }, |worker| {
            // Stop observers see Reserved custody before the task can execute.
            let mut slot = self.mcp_prewarm_task.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            assert!(slot.is_none(), "MCP prewarm worker may only start once");
            *slot = Some(worker);
        });
    }

    pub(super) fn schedule_mcp_prewarm(&self) {
        let _ = self.mcp_prewarm_tx.try_send(());
    }

    pub(super) async fn stop_mcp_prewarm_worker(&self) -> Result<(), codex_async_utils::TaskJoinFailure> {
        self.mcp_prewarm_shutdown.cancel();
        self.mcp_prewarm_tx.close();
        let worker = self
            .mcp_prewarm_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(worker) = worker {
            worker.wait().await?;
        }
        Ok(())
    }
}
