//! Every client generation remains attached through construction and final close.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::PoisonError;
use std::time::Duration;

use codex_async_utils::RetainedTask;
use codex_async_utils::RetainedTaskOutcome;
use tokio_util::sync::CancellationToken;

use crate::owned_service::OwnedService;
use crate::local_process_owner::LocalProcessOwner;
use crate::transport_lifecycle::TransportCloseConfirmation;
use crate::transport_lifecycle::TransportCloseObserver;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpShutdownConfirmation {
    /// Instrumented service/transport handles joined successfully. For local
    /// stdio this includes the direct child and stderr, not descendant emptiness.
    Confirmed,
    /// An exact outer join completed, but an upstream private result is unavailable.
    Unconfirmed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum McpShutdownFailure {
    #[error("MCP service shutdown failed")]
    Service,
    #[error("MCP transport shutdown failed")]
    Transport,
    #[error("MCP process shutdown failed")]
    Process,
    #[error("MCP lifecycle task did not complete normally")]
    Task,
    #[error("MCP lifecycle admission is closed")]
    Closed,
}

type ShutdownResult = Result<McpShutdownConfirmation, McpShutdownFailure>;

pub(crate) trait PendingShutdown: Send + Sync {
    fn begin_shutdown(&self);

    /// Empty admission shells hold no transport after ownership transfers.
    fn is_empty(&self) -> bool {
        false
    }
}

/// The construction attempt returned normally. Any resources it created have
/// independent lifecycle owners; its operation error is not a cleanup failure.
/// Callers and the client roster retain the original operation result until
/// observation, while only panic/cancellation requires task-registry custody.
pub(crate) struct OperationCompletion<T>(pub(crate) Result<T, Arc<anyhow::Error>>);

impl<T: Send + Sync + 'static> RetainedTaskOutcome for OperationCompletion<T> {
    fn succeeded(&self) -> bool {
        true
    }
}

trait LifecycleJob: Send + Sync {
    fn wait(&self) -> Pin<Box<dyn Future<Output = Result<(), McpShutdownFailure>> + Send + '_>>;
}

struct Job<T: RetainedTaskOutcome>(Arc<RetainedTask<T>>);

impl<T: RetainedTaskOutcome> LifecycleJob for Job<T> {
    fn wait(&self) -> Pin<Box<dyn Future<Output = Result<(), McpShutdownFailure>> + Send + '_>> {
        Box::pin(async { self.0.wait().await.map(|_| ()).map_err(|_| McpShutdownFailure::Task) })
    }
}

#[derive(Default)]
struct State {
    closing: bool,
    constructions: usize,
    prune_cursor: usize,
    services: Vec<Arc<OwnedService>>,
    processes: Vec<Arc<LocalProcessOwner>>,
    transports: Vec<Arc<dyn TransportCloseObserver>>,
    pending: Vec<Arc<dyn PendingShutdown>>,
    jobs: Vec<Arc<dyn LifecycleJob>>,
    completion: Option<Arc<RetainedTask<ShutdownResult>>>,
}

static CLIENTS: OnceLock<Mutex<Vec<Arc<ClientLifecycle>>>> = OnceLock::new();

pub(crate) struct ClientLifecycle {
    state: Mutex<State>,
    runtime: tokio::runtime::Handle,
    stop: CancellationToken,
}

impl ClientLifecycle {
    pub(crate) fn new(runtime: tokio::runtime::Handle) -> Arc<Self> {
        let owner = Arc::new(Self {
            state: Mutex::new(State::default()),
            runtime,
            stop: CancellationToken::new(),
        });
        CLIENTS.get_or_init(Default::default).lock()
            .unwrap_or_else(PoisonError::into_inner).push(Arc::clone(&owner));
        owner
    }

    pub(crate) fn runtime(&self) -> &tokio::runtime::Handle {
        &self.runtime
    }

    pub(crate) fn stop_token(&self) -> CancellationToken {
        self.stop.clone()
    }

    pub(crate) fn reserve(self: &Arc<Self>) -> Result<ConstructionLease, McpShutdownFailure> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.closing {
            return Err(McpShutdownFailure::Closed);
        }
        state.constructions += 1;
        Ok(ConstructionLease { owner: Arc::clone(self) })
    }

    pub(crate) fn track_job<T: RetainedTaskOutcome>(&self, job: Arc<RetainedTask<T>>) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
            .jobs.push(Arc::new(Job(job)));
    }

    pub(crate) fn attach_transport(&self, transport: Arc<dyn TransportCloseObserver>) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).transports.push(transport);
    }

    pub(crate) fn attach_pending(&self, pending: Arc<dyn PendingShutdown>) {
        let closing = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.pending.push(Arc::clone(&pending));
            state.closing
        };
        if closing {
            pending.begin_shutdown();
        }
    }

    pub(crate) fn attach_process(&self, process: LocalProcessOwner) {
        let process = Arc::new(process);
        let closing = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.processes.push(Arc::clone(&process));
            state.closing
        };
        if closing {
            process.begin_shutdown();
        }
    }

    pub(crate) fn attach_service(&self, service: Arc<OwnedService>) {
        let closing = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.services.push(Arc::clone(&service));
            state.closing
        };
        if closing {
            service.begin_shutdown();
        }
    }

    pub(crate) fn begin_shutdown(self: &Arc<Self>) {
        let (services, processes, pending) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if state.closing {
                return;
            }
            state.closing = true;
            (state.services.clone(), state.processes.clone(), state.pending.clone())
        };
        self.stop.cancel();
        for process in processes {
            process.begin_shutdown();
        }
        for service in services {
            service.begin_shutdown();
        }
        for pending in pending {
            pending.begin_shutdown();
        }
        let owner = Arc::clone(self);
        let publisher = Arc::clone(self);
        let _completion = RetainedTask::spawn_published(&self.runtime, async move {
            owner.finish().await
        }, move |task| {
            publisher.state.lock().unwrap_or_else(PoisonError::into_inner).completion = Some(task);
        });
    }

    async fn finish(&self) -> ShutdownResult {
        // A reserved construction may publish after the fence. Its publication
        // attaches to this same roster and immediately starts shutdown.
        let (services, processes, transports, jobs) = loop {
            let snapshot = {
                let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
                (state.constructions == 0).then(|| {
                    (state.services.clone(), state.processes.clone(), state.transports.clone(), state.jobs.clone())
                })
            };
            if let Some(snapshot) = snapshot {
                break snapshot;
            }
            tokio::time::sleep(Duration::from_millis(/*millis*/ 10)).await;
        };
        let mut failure = None;
        let mut confirmation = McpShutdownConfirmation::Confirmed;
        for job in jobs {
            if let Err(error) = job.wait().await {
                failure.get_or_insert(error);
            }
        }
        for service in services {
            if let Err(error) = service.wait_closed().await {
                failure.get_or_insert(error);
            }
        }
        for transport in transports {
            match transport.wait().await {
                Ok(TransportCloseConfirmation::Confirmed) => {}
                Ok(TransportCloseConfirmation::Unconfirmed) => {
                    confirmation = McpShutdownConfirmation::Unconfirmed;
                }
                Err(_) => { failure.get_or_insert(McpShutdownFailure::Transport); }
            }
        }
        for process in processes {
            if process.wait_closed().await.is_err() {
                failure.get_or_insert(McpShutdownFailure::Process);
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(confirmation),
        }
    }

    pub(crate) async fn wait_closed(self: &Arc<Self>) -> ShutdownResult {
        self.begin_shutdown();
        let completion = loop {
            let completion = self.state.lock().unwrap_or_else(PoisonError::into_inner)
                .completion.clone();
            if let Some(completion) = completion {
                break completion;
            }
            tokio::time::sleep(Duration::from_millis(/*millis*/ 10)).await;
        };
        let result = completion.wait().await.map_err(|_| McpShutdownFailure::Task)?;
        let result = *result;
        if result == Ok(McpShutdownConfirmation::Confirmed) {
            self.retire();
        }
        result
    }
}

pub(crate) struct ConstructionLease {
    owner: Arc<ClientLifecycle>,
}

impl Drop for ConstructionLease {
    fn drop(&mut self) {
        self.owner.state.lock().unwrap_or_else(PoisonError::into_inner).constructions -= 1;
    }
}

pub(crate) struct ClientConstructionGuard(Option<Arc<ClientLifecycle>>);

impl ClientConstructionGuard {
    pub(crate) fn new(owner: Arc<ClientLifecycle>) -> Self {
        Self(Some(owner))
    }

    pub(crate) fn disarm(mut self) {
        let _ = self.0.take();
    }
}

impl Drop for ClientConstructionGuard {
    fn drop(&mut self) {
        if let Some(owner) = self.0.take() {
            owner.begin_shutdown();
        }
    }
}

#[path = "client_lifecycle_pruning.rs"]
mod pruning;

#[cfg(test)]
#[path = "client_lifecycle_tests.rs"]
mod tests;
