//! Operations share a peer; one retained owner joins the concrete RMCP service.

use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use codex_async_utils::RetainedTask;
use rmcp::service::Peer;
use rmcp::service::QuitReason;
use rmcp::service::RoleClient;
use rmcp::service::RunningService;
use rmcp::service::RunningServiceCancellationToken;

use crate::client_lifecycle::McpShutdownFailure;
use crate::elicitation_client_service::ElicitationClientService;

#[derive(Debug)]
enum ServiceFailure {
    Join(tokio::task::JoinError),
    Reason(QuitReason),
}

pub(crate) struct OwnedService {
    peer: Peer<RoleClient>,
    service: ElicitationClientService,
    cancellation: Mutex<Option<RunningServiceCancellationToken>>,
    closing: AtomicBool,
    completion: Arc<RetainedTask<Result<(), ServiceFailure>>>,
}

impl OwnedService {
    pub(crate) fn new(
        service: RunningService<RoleClient, ElicitationClientService>,
        runtime: &tokio::runtime::Handle,
    ) -> Arc<Self> {
        let peer = service.peer().clone();
        let handler = service.service().clone();
        let cancellation = service.cancellation_token();
        // The primary task owns waiting(), including RMCP's private JoinHandle.
        // Cancelling an observer never cancels this task or drops that handle.
        let completion = RetainedTask::spawn(runtime, async move {
            match service.waiting().await {
                Ok(QuitReason::Closed | QuitReason::Cancelled) => Ok(()),
                Ok(QuitReason::JoinError(error)) => Err(ServiceFailure::Join(error)),
                Ok(reason) => Err(ServiceFailure::Reason(reason)),
                Err(error) => Err(ServiceFailure::Join(error)),
            }
        });
        Arc::new(Self {
            peer,
            service: handler,
            cancellation: Mutex::new(Some(cancellation)),
            closing: AtomicBool::new(false),
            completion,
        })
    }

    pub(crate) fn peer(&self) -> &Peer<RoleClient> {
        &self.peer
    }

    pub(crate) fn service(&self) -> &ElicitationClientService {
        &self.service
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closing.load(Ordering::Acquire) || self.peer.is_transport_closed()
    }

    pub(crate) fn begin_shutdown(&self) {
        self.closing.store(true, Ordering::Release);
        let cancellation = self
            .cancellation
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
    }

    pub(crate) async fn wait_closed(&self) -> Result<(), McpShutdownFailure> {
        match self.completion.wait().await {
            Ok(result) => match result.as_ref() {
                Ok(()) => Ok(()),
                Err(ServiceFailure::Join(error)) => {
                    // Inspect only the bounded classification; retain the original.
                    let _ = error.is_cancelled();
                    Err(McpShutdownFailure::Service)
                }
                Err(ServiceFailure::Reason(reason)) => {
                    let _ = std::mem::discriminant(reason);
                    Err(McpShutdownFailure::Service)
                }
            },
            Err(_) => Err(McpShutdownFailure::Task),
        }
    }
}

impl Deref for OwnedService {
    type Target = Peer<RoleClient>;

    fn deref(&self) -> &Self::Target {
        &self.peer
    }
}

impl Drop for OwnedService {
    fn drop(&mut self) {
        self.begin_shutdown();
    }
}

impl crate::client_lifecycle::PendingShutdown for OwnedService {
    fn begin_shutdown(&self) {
        OwnedService::begin_shutdown(self);
    }
}

#[cfg(test)]
#[path = "owned_service_tests.rs"]
mod tests;
