//! Transport custody begins before a handshake and survives cancelled close observers.

use codex_async_utils::RetainedTask;
use codex_async_utils::TaskJoinFailure;
use rmcp::RoleClient;
use rmcp::service::RxJsonRpcMessage;
use rmcp::service::TxJsonRpcMessage;
use rmcp::transport::Transport;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::PoisonError;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::Mutex as AsyncMutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransportCloseConfirmation {
    Confirmed,
    /// The pinned HTTP WorkerTransport discards its private worker's logical
    /// quit result. Observing its close task cannot confirm logical success.
    Unconfirmed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransportCloseFailure {
    Transport,
    Task(TaskJoinFailure),
}

impl std::fmt::Display for TransportCloseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => formatter.write_str("transport close failed"),
            Self::Task(failure) => std::fmt::Display::fmt(failure, formatter),
        }
    }
}

impl std::error::Error for TransportCloseFailure {}

pub(crate) trait TransportCloseObserver: Send + Sync {
    /// Apply any deadline outside this observation. Cancelling a waiter never
    /// cancels the original close task or releases its transport.
    fn wait(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<TransportCloseConfirmation, TransportCloseFailure>>
                + Send
                + '_,
        >,
    >;
}

#[derive(Default)]
struct Registry {
    owners: Mutex<Vec<Arc<dyn Any + Send + Sync>>>,
}

static OWNERS: OnceLock<Arc<Registry>> = OnceLock::new();

type CloseTask<T> = RetainedTask<
    Result<TransportCloseConfirmation, <T as Transport<RoleClient>>::Error>,
>;

struct CloseOwner<T: Transport<RoleClient> + 'static> {
    registry: Arc<Registry>,
    runtime: Handle,
    confirmation: TransportCloseConfirmation,
    // This owner, independently of the task future, retains T after a spawn
    // panic or runtime cancellation destroys the future's captures.
    transport: Mutex<Option<Arc<AsyncMutex<T>>>>,
    task: Mutex<Option<Arc<CloseTask<T>>>>,
}

impl<T: Transport<RoleClient> + 'static> CloseOwner<T> {
    fn start(self: &Arc<Self>, transport: T) {
        self.start_published(transport, || {});
    }

    #[expect(clippy::await_holding_invalid_type, reason = "The registered close owner retains the transport independently; closing it needs exclusive mutable access across await.")]
    fn start_published(self: &Arc<Self>, transport: T, before_spawn: impl FnOnce()) {
        let transport = Arc::new(AsyncMutex::new(transport));
        *self.transport.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(Arc::clone(&transport));
        self.registry
            .owners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Arc::clone(self) as Arc<dyn Any + Send + Sync>);
        let confirmation = self.confirmation;
        // Publication and spawning happen only after the separate owner has
        // custody. No await occurs while moving T or registering either owner.
        RetainedTask::spawn_published(
            &self.runtime,
            async move {
                transport.lock().await.close().await?;
                Ok(confirmation)
            },
            |task| {
                *self.task.lock().unwrap_or_else(PoisonError::into_inner) = Some(task);
                before_spawn();
            },
        );
    }

    async fn observe(&self) -> Result<TransportCloseConfirmation, TransportCloseFailure> {
        let task = loop {
            let task = self.task.lock().unwrap_or_else(PoisonError::into_inner).clone();
            if let Some(task) = task {
                break task;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        let outcome = task.wait().await.map_err(TransportCloseFailure::Task)?;
        match outcome.as_ref() {
            Ok(confirmation) => {
                if *confirmation == TransportCloseConfirmation::Confirmed {
                    self.retire();
                }
                Ok(*confirmation)
            }
            // The concrete, potentially sensitive error stays in the retained
            // task. Repeated callers only receive this bounded classification.
            Err(_) => Err(TransportCloseFailure::Transport),
        }
    }

    fn retire(&self) {
        let retired = {
            let mut owners = self.registry.owners.lock().unwrap_or_else(PoisonError::into_inner);
            owners
                .iter()
                .position(|owner| {
                    owner.downcast_ref::<Self>().is_some_and(|owner| std::ptr::eq(owner, self))
                })
                .map(|index| owners.swap_remove(index))
        };
        // Destruction of a transport must never execute under a registry lock.
        drop(retired);
    }
}

impl<T: Transport<RoleClient> + 'static> TransportCloseObserver for CloseOwner<T> {
    fn wait(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<TransportCloseConfirmation, TransportCloseFailure>>
                + Send
                + '_,
        >,
    > {
        Box::pin(self.observe())
    }
}

/// Directly delegates normal operations; only close transfers the transport
/// into retained ownership. Dropping an unfinished handshake also starts close.
/// The supplied error factory is used only after closure has begun or failed;
/// normal send errors retain the exact T::Error needed by HTTP auth downcasts.
pub(crate) struct TransportGuard<T: Transport<RoleClient> + 'static> {
    transport: Option<T>,
    owner: Arc<CloseOwner<T>>,
    closed_error: fn() -> T::Error,
}

impl<T: Transport<RoleClient> + 'static> TransportGuard<T> {
    pub(crate) fn new(
        transport: T,
        runtime: Handle,
        confirmation: TransportCloseConfirmation,
        closed_error: fn() -> T::Error,
    ) -> (Self, Arc<dyn TransportCloseObserver>) {
        Self::new_in(
            transport,
            runtime,
            confirmation,
            closed_error,
            OWNERS.get_or_init(Default::default),
        )
    }

    fn new_in(
        transport: T,
        runtime: Handle,
        confirmation: TransportCloseConfirmation,
        closed_error: fn() -> T::Error,
        registry: &Arc<Registry>,
    ) -> (Self, Arc<dyn TransportCloseObserver>) {
        let owner = Arc::new(CloseOwner {
            registry: Arc::clone(registry),
            runtime,
            confirmation,
            transport: Mutex::new(None),
            task: Mutex::new(None),
        });
        let observer = Arc::clone(&owner) as Arc<dyn TransportCloseObserver>;
        (
            Self { transport: Some(transport), owner, closed_error },
            observer,
        )
    }

    /// Constructor access before handing the guard to RMCP. Panics if close
    /// has already moved the transport into its retained owner.
    pub(crate) fn inner(&self) -> &T {
        match self.transport.as_ref() {
            Some(transport) => transport,
            None => panic!("transport close has already started"),
        }
    }

    fn start_close(&mut self) {
        if let Some(transport) = self.transport.take() {
            self.owner.start(transport);
        }
    }
}

impl<T: Transport<RoleClient> + 'static> Transport<RoleClient> for TransportGuard<T> {
    type Error = T::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let send = self.transport.as_mut().map(|transport| transport.send(item));
        let closed_error = self.closed_error;
        async move {
            match send {
                Some(send) => send.await,
                None => Err(closed_error()),
            }
        }
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
        match self.transport.as_mut() {
            Some(transport) => transport.receive().await,
            None => None,
        }
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.start_close();
        let owner = Arc::clone(&self.owner);
        let closed_error = self.closed_error;
        async move { owner.observe().await.map(|_| ()).map_err(|_| closed_error()) }
    }
}

impl<T: Transport<RoleClient> + 'static> Drop for TransportGuard<T> {
    fn drop(&mut self) {
        self.start_close();
    }
}

#[cfg(test)]
#[path = "transport_lifecycle_tests.rs"]
mod tests;
