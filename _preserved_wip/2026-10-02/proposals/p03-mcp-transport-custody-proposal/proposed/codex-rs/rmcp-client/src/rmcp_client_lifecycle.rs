//! Client construction observers cannot discard a late transport generation.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;

use codex_async_utils::RetainedTask;
use tracing::Instrument;

use crate::client_lifecycle::ClientLifecycle;
use crate::client_lifecycle::McpShutdownConfirmation;
use crate::client_lifecycle::McpShutdownFailure;
use crate::client_lifecycle::PendingShutdown;
use crate::client_lifecycle::OperationCompletion;
use crate::generation_publication::GenerationPublication;
use crate::stdio_server_launcher::StdioServerProcessHandle;
use crate::transport_lifecycle::TransportCloseConfirmation;
use crate::transport_lifecycle::TransportGuard;

use super::AuthStorageOriginator;
use super::ClientState;
use super::PendingTransport;
use super::RmcpClient;
use super::TransportRecipe;

pub(super) fn guard_transport<T: rmcp::transport::Transport<rmcp::RoleClient> + 'static>(
    transport: T,
    lifecycle: &ClientLifecycle,
    confirmation: TransportCloseConfirmation,
    closed_error: fn() -> T::Error,
) -> TransportGuard<T> {
    let (transport, observer) = TransportGuard::new(
        transport, lifecycle.runtime().clone(), confirmation, closed_error,
    );
    lifecycle.attach_transport(observer);
    transport
}

pub(super) struct PendingGeneration {
    transport: Mutex<Option<PendingTransport>>,
}

impl PendingGeneration {
    fn new(transport: PendingTransport, lifecycle: &ClientLifecycle) -> Arc<Self> {
        let generation = Arc::new(Self { transport: Mutex::new(Some(transport)) });
        lifecycle.attach_pending(generation.clone());
        generation
    }

    pub(super) fn take(&self) -> anyhow::Result<PendingTransport> {
        self.transport.lock().unwrap_or_else(PoisonError::into_inner)
            .take().ok_or_else(|| anyhow::anyhow!("MCP transport is closed or already initializing"))
    }

    pub(super) fn process_handle(&self) -> Option<StdioServerProcessHandle> {
        let transport = self.transport.lock().unwrap_or_else(PoisonError::into_inner);
        match transport.as_ref() {
            Some(PendingTransport::Stdio { transport }) => Some(transport.inner().process_handle()),
            _ => None,
        }
    }

    pub(super) fn is_http(&self) -> bool {
        let transport = self.transport.lock().unwrap_or_else(PoisonError::into_inner);
        matches!(transport.as_ref(),
            Some(PendingTransport::StreamableHttp { .. }
                | PendingTransport::StreamableHttpWithOAuth { .. }
                | PendingTransport::StreamableHttpWithAccessTokenOnly { .. }))
    }

    pub(super) fn oauth_runtime(&self) -> Option<crate::oauth::OAuthRuntime> {
        let transport = self.transport.lock().unwrap_or_else(PoisonError::into_inner);
        match transport.as_ref() {
            Some(PendingTransport::StreamableHttpWithOAuth { oauth_runtime, .. }) => {
                Some(oauth_runtime.clone())
            }
            _ => None,
        }
    }
}

impl PendingShutdown for PendingGeneration {
    fn is_empty(&self) -> bool {
        self.transport.lock().unwrap_or_else(PoisonError::into_inner).is_none()
    }

    fn begin_shutdown(&self) {
        let transport = self.transport.lock().unwrap_or_else(PoisonError::into_inner).take();
        // Guard drop transfers the real transport into its retained close owner.
        drop(transport);
    }
}

#[derive(Debug)]
pub(super) struct SharedOperationError(pub(super) Arc<anyhow::Error>);

impl std::fmt::Display for SharedOperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for SharedOperationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref().as_ref())
    }
}

impl RmcpClient {
    pub(super) async fn create_pending_transport_owned(
        recipe: &TransportRecipe,
        lifecycle: &Arc<ClientLifecycle>,
    ) -> anyhow::Result<Arc<PendingGeneration>> {
        ClientLifecycle::observe_completed_clients().await;
        lifecycle.prune_clean().await;
        let lease = lifecycle.reserve()?;
        let recipe = recipe.clone();
        let constructor = Arc::clone(lifecycle);
        let publisher = Arc::clone(lifecycle);
        let originator = AuthStorageOriginator::current();
        let (publication, observer_guard) = GenerationPublication::new();
        let task = RetainedTask::spawn_published(lifecycle.runtime(), originator.scope(async move {
            let result = Self::create_pending_transport(&recipe, &constructor).await
                .map(|transport| {
                    let generation = PendingGeneration::new(transport, &constructor);
                    publication.publish(generation.clone());
                    generation
                })
                .map_err(Arc::new);
            drop(lease);
            OperationCompletion(result)
        }).instrument(tracing::Span::current()), move |task| publisher.track_job(task));
        let result = task.wait().await.map_err(anyhow::Error::new)?;
        match &result.0 {
            Ok(generation) => {
                observer_guard.disarm();
                Ok(Arc::clone(generation))
            },
            Err(error) => Err(anyhow::Error::new(SharedOperationError(Arc::clone(error)))),
        }
    }

    /// Observe exact service and transport closes for every client generation.
    /// Unconfirmed is distinct from a known failure and from complete MCP proof.
    pub async fn shutdown(
        &self,
    ) -> Result<McpShutdownConfirmation, McpShutdownFailure> {
        let previous = {
            let mut state = self.state.lock().await;
            std::mem::replace(&mut *state, ClientState::Closed)
        };
        if let Some(process) = &self.stdio_process {
            process.begin_shutdown();
        }
        self.lifecycle.begin_shutdown();
        drop(previous);
        let lifecycle = self.lifecycle.wait_closed().await;
        let process = match &self.stdio_process {
            Some(process) if process.is_local() => {
                process.wait_closed().await.map_err(|_| McpShutdownFailure::Process)
            }
            Some(process) => process.terminate().await.map_err(|_| McpShutdownFailure::Process),
            None => Ok(()),
        };
        process.and(lifecycle)
    }
}

impl Drop for RmcpClient {
    fn drop(&mut self) {
        if let Some(process) = &self.stdio_process {
            process.begin_shutdown();
        }
        self.lifecycle.begin_shutdown();
    }
}
