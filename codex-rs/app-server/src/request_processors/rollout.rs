//! Triggers local rollout maintenance without waiting for the background pass.

use super::ThreadRequestProcessor;
use super::thread_processor::unsupported_thread_store_operation;
use crate::error_code::internal_error;
use codex_app_server_protocol::ClientResponsePayload;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RolloutCompressResponse;
use codex_thread_store::RolloutMaintenance;

impl ThreadRequestProcessor {
    pub(crate) async fn rollout_compress(
        &self,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        if !self.thread_store.supports_rollout_maintenance() {
            return Err(unsupported_thread_store_operation("rollout/compress"));
        }

        self.thread_store
            .run_rollout_maintenance(RolloutMaintenance::Compress)
            .await
            .map_err(|error| {
                internal_error(format!("failed to schedule rollout compression: {error}"))
            })?;
        Ok(Some(RolloutCompressResponse {}.into()))
    }
}
