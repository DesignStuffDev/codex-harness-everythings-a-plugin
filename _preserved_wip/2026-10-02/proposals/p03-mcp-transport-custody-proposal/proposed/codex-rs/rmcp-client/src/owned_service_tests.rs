//! Exercise the concrete RMCP service join without depending on worker-abort safety.

use super::OwnedService;
use crate::elicitation_client_service::ElicitationClientService;
use crate::rmcp_client::ElicitationPauseState;
use pretty_assertions::assert_eq;
use rmcp::RoleServer;
use rmcp::model::ClientInfo;
use rmcp::model::ClientJsonRpcMessage;
use rmcp::model::ClientRequest;
use rmcp::model::PingRequest;
use rmcp::model::ServerJsonRpcMessage;
use rmcp::model::ServerResult;
use rmcp::service::RoleClient;
use rmcp::service::RxJsonRpcMessage;
use rmcp::service::ServiceError;
use rmcp::service::TxJsonRpcMessage;
use rmcp::service::serve_directly;
use rmcp::transport::Transport;
use rmcp::transport::async_rw::AsyncRwTransport;
use std::future::Future;
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::io::DuplexStream;
use tokio::io::ReadHalf;
use tokio::io::WriteHalf;
use tokio::sync::Semaphore;
use tokio::sync::oneshot;
use tokio::time::timeout;

type ClientTransport = AsyncRwTransport<RoleClient, ReadHalf<DuplexStream>, WriteHalf<DuplexStream>>;
type ServerTransport = AsyncRwTransport<RoleServer, ReadHalf<DuplexStream>, WriteHalf<DuplexStream>>;

struct GatedClientTransport {
    inner: ClientTransport,
    close_started: Option<oneshot::Sender<()>>,
    close_gate: Arc<Semaphore>,
    close_calls: Arc<AtomicUsize>,
    close_finished: Arc<AtomicBool>,
}

impl Transport<RoleClient> for GatedClientTransport {
    type Error = io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        self.inner.send(item)
    }

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<RoleClient>>> + Send {
        self.inner.receive()
    }

    async fn close(&mut self) -> io::Result<()> {
        self.close_calls.fetch_add(/*val*/ 1, Ordering::SeqCst);
        if let Some(started) = self.close_started.take() {
            let _ = started.send(());
        }
        self.close_gate.acquire().await.map_err(io::Error::other)?.forget();
        self.inner.close().await?;
        self.close_finished.store(/*val*/ true, Ordering::SeqCst);
        Ok(())
    }
}

struct Fixture {
    owner: Arc<OwnedService>,
    server: ServerTransport,
    close_started: oneshot::Receiver<()>,
    close_gate: Arc<Semaphore>,
    close_calls: Arc<AtomicUsize>,
    close_finished: Arc<AtomicBool>,
}

fn fixture() -> Fixture {
    let service = ElicitationClientService::new(
        ClientInfo::default(),
        Box::new(|_, _| Box::pin(async { Err(anyhow::anyhow!("unexpected elicitation")) })),
        ElicitationPauseState::new(),
    );
    let (client, server) = tokio::io::duplex(/*max_buf_size*/ 4096);
    let (client_read, client_write) = tokio::io::split(client);
    let (server_read, server_write) = tokio::io::split(server);
    let (started_tx, close_started) = oneshot::channel();
    let close_gate = Arc::new(Semaphore::new(/*permits*/ 0));
    let close_calls = Arc::new(AtomicUsize::new(/*v*/ 0));
    let close_finished = Arc::new(AtomicBool::new(/*v*/ false));
    let transport = GatedClientTransport {
        inner: AsyncRwTransport::new(client_read, client_write),
        close_started: Some(started_tx),
        close_gate: Arc::clone(&close_gate),
        close_calls: Arc::clone(&close_calls),
        close_finished: Arc::clone(&close_finished),
    };
    let running = serve_directly(service, transport, /*peer_info*/ None);
    Fixture {
        owner: OwnedService::new(running, &tokio::runtime::Handle::current()),
        server: AsyncRwTransport::new(server_read, server_write),
        close_started,
        close_gate,
        close_calls,
        close_finished,
    }
}

#[tokio::test]
async fn peer_operations_survive_owner_adoption_and_cancelled_close_observers() -> anyhow::Result<()> {
    let Fixture {
        owner, mut server, close_started, close_gate, close_calls, close_finished,
    } = fixture();
    let request = owner.send_request(ClientRequest::PingRequest(PingRequest::default()));
    let response = async {
        let Some(ClientJsonRpcMessage::Request(request)) = server.receive().await else {
            anyhow::bail!("expected a client request");
        };
        assert!(matches!(request.request, ClientRequest::PingRequest(_)));
        server.send(ServerJsonRpcMessage::response(ServerResult::empty(()), request.id)).await?;
        Ok::<_, anyhow::Error>(())
    };
    let (reply, response) = timeout(Duration::from_secs(/*secs*/ 5), async {
        tokio::join!(request, response)
    }).await?;
    response?;
    assert_eq!(reply?, ServerResult::empty(()));

    owner.begin_shutdown();
    owner.begin_shutdown();
    timeout(Duration::from_secs(/*secs*/ 5), close_started).await??;
    // The actual RMCP service task is blocked inside transport.close. Expiry
    // drops only this observation of the retained completion task.
    assert!(timeout(Duration::from_millis(/*millis*/ 20), owner.wait_closed()).await.is_err());
    assert_eq!(close_calls.load(Ordering::SeqCst), 1);
    assert!(!close_finished.load(Ordering::SeqCst));
    close_gate.add_permits(/*n*/ 1);
    timeout(Duration::from_secs(/*secs*/ 5), owner.wait_closed()).await??;
    owner.begin_shutdown();
    owner.wait_closed().await?;
    assert!(owner.is_closed());
    assert!(close_finished.load(Ordering::SeqCst));
    assert_eq!(close_calls.load(Ordering::SeqCst), 1);
    let after_close = owner.send_request(ClientRequest::PingRequest(PingRequest::default())).await;
    assert!(matches!(after_close, Err(ServiceError::TransportClosed)));
    Ok(())
}

#[tokio::test]
async fn dropping_last_service_owner_signals_but_retains_the_real_join() -> anyhow::Result<()> {
    let Fixture {
        owner, server: _server, close_started, close_gate, close_calls, close_finished,
    } = fixture();
    let completion = Arc::clone(&owner.completion);
    let weak = Arc::downgrade(&owner);
    drop(owner);
    timeout(Duration::from_secs(/*secs*/ 5), close_started).await??;
    assert!(weak.upgrade().is_none());
    assert!(timeout(Duration::from_millis(/*millis*/ 20), completion.wait()).await.is_err());
    assert!(!close_finished.load(Ordering::SeqCst));
    close_gate.add_permits(/*n*/ 1);
    let observed = timeout(Duration::from_secs(/*secs*/ 5), completion.wait()).await??;
    assert!(observed.is_ok());
    let repeated = completion.wait().await?;
    assert!(Arc::ptr_eq(&observed, &repeated));
    assert!(close_finished.load(Ordering::SeqCst));
    assert_eq!(close_calls.load(Ordering::SeqCst), 1);
    Ok(())
}
