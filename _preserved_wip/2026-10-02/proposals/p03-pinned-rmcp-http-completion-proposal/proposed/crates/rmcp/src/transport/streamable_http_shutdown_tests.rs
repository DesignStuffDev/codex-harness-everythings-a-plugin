use super::*;
use crate::transport::Transport;
use anyhow::Context;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Condvar, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context as TaskContext, Poll};
use tokio::sync::Notify;
#[derive(Clone, Copy)]
enum Scenario { Live, StartupStall, BodyStall, InitializedStall, InitializedFailure, ProtocolPanic, SsePanic }
#[derive(Clone, Copy)]
enum Delete { Deleted, Unsupported, Failed, Stall, Panic }
#[derive(Clone, Copy)]
enum StreamKind { Initial, Common, Response }
#[derive(Default)]
struct Signals {
    startup: Notify,
    initialized: Notify,
    initial_body: Notify,
    common: Notify,
    response: Notify,
    delete_started: Notify,
    dropped_bodies: AtomicUsize,
    dropped_requests: AtomicUsize,
    deleted: Mutex<Vec<String>>,
    drop_gate: Mutex<Option<Arc<DropGate>>>,
}
#[derive(Default)]
struct DropGate { entered: Notify, released: Mutex<bool>, changed: Condvar }
struct ReleaseDrop(Arc<DropGate>);
impl Drop for ReleaseDrop {
    fn drop(&mut self) {
        *self.0.released.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        self.0.changed.notify_all();
    }
}

struct HeldRequest(Arc<Signals>);
impl Drop for HeldRequest {
    fn drop(&mut self) { self.0.dropped_requests.fetch_add(1, Ordering::SeqCst); }
}

struct HeldSse { signals: Arc<Signals>, kind: StreamKind, panic_on_poll: bool }
impl Stream for HeldSse {
    type Item = Result<Sse, SseError>;
    fn poll_next(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        match self.kind {
            StreamKind::Initial => self.signals.initial_body.notify_one(),
            StreamKind::Common => self.signals.common.notify_one(),
            StreamKind::Response => self.signals.response.notify_one(),
        }
        assert!(!self.panic_on_poll, "controlled SSE task panic");
        Poll::Pending
    }
}
impl Drop for HeldSse {
    fn drop(&mut self) {
        let gate = self.signals.drop_gate.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner).clone();
        if let Some(gate) = gate {
            gate.entered.notify_one();
            let mut released = gate.released.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            while !*released {
                released = gate.changed.wait(released).unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        self.signals.dropped_bodies.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Clone)]
// This backend is Send but deliberately !Sync. The worker must not borrow C
// across an await in a way that silently strengthens the public trait contract.
struct Backend {
    signals: Arc<Signals>, scenario: Scenario, delete: Delete,
    _not_sync: std::cell::Cell<()>,
}
impl Backend {
    fn new(scenario: Scenario, delete: Delete) -> Self {
        Self { signals: Arc::new(Signals::default()), scenario, delete, _not_sync: std::cell::Cell::new(()) }
    }
    fn stream(&self, kind: StreamKind) -> BoxedSseStream {
        HeldSse {
            signals: Arc::clone(&self.signals), kind,
            panic_on_poll: matches!(self.scenario, Scenario::SsePanic),
        }.boxed()
    }
}

fn initialize() -> anyhow::Result<ClientJsonRpcMessage> {
    Ok(serde_json::from_value(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "shutdown-test", "version": "1"}}
    }))?)
}
fn initialized() -> anyhow::Result<ClientJsonRpcMessage> {
    Ok(serde_json::from_value(serde_json::json!({
        "jsonrpc": "2.0", "method": "notifications/initialized"
    }))?)
}
fn tools_request() -> anyhow::Result<ClientJsonRpcMessage> {
    Ok(serde_json::from_value(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
    }))?)
}
fn initialize_response() -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": 1,
        "result": {"protocolVersion": "2024-11-05", "capabilities": {},
            "serverInfo": {"name": "shutdown-test", "version": "1"}}})
}

impl StreamableHttpClient for Backend {
    type Error = io::Error;
    fn post_message(
        &self, _uri: Arc<str>, message: ClientJsonRpcMessage, _session_id: Option<Arc<str>>,
        _auth_header: Option<String>, _custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> impl Future<Output = Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>>> + Send + '_ {
        let this = self.clone();
        async move {
        match message {
            ClientJsonRpcMessage::Request(request)
                if matches!(request.request, ClientRequest::InitializeRequest(_)) => {
                this.signals.startup.notify_one();
                if matches!(this.scenario, Scenario::StartupStall) {
                    let _held = HeldRequest(Arc::clone(&this.signals));
                    std::future::pending::<()>().await;
                }
                if matches!(this.scenario, Scenario::BodyStall) {
                    return Ok(StreamableHttpPostResponse::Sse(
                        this.stream(StreamKind::Initial), Some("known-session".to_owned()),
                    ));
                }
                Ok(StreamableHttpPostResponse::Json(
                    serde_json::from_value(initialize_response())?, Some("known-session".to_owned()),
                ))
            }
            ClientJsonRpcMessage::Notification(_) => {
                this.signals.initialized.notify_one();
                if matches!(this.scenario, Scenario::InitializedStall) {
                    let _held = HeldRequest(Arc::clone(&this.signals));
                    std::future::pending::<()>().await;
                }
                if matches!(this.scenario, Scenario::InitializedFailure) {
                    return Err(StreamableHttpError::Client(io::Error::other("initialized rejected")));
                }
                Ok(StreamableHttpPostResponse::Accepted)
            }
            _ => {
                assert!(!matches!(this.scenario, Scenario::ProtocolPanic), "controlled POST panic");
                Ok(StreamableHttpPostResponse::Sse(this.stream(StreamKind::Response), None))
            }
        }
        }
    }
    fn delete_session(
        &self, _uri: Arc<str>, session_id: Arc<str>, _auth_header: Option<String>,
        _custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> impl Future<Output = Result<(), StreamableHttpError<Self::Error>>> + Send + '_ {
        let this = self.clone();
        async move {
        this.signals.deleted.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(session_id.to_string());
        this.signals.delete_started.notify_one();
        match this.delete {
            Delete::Deleted => Ok(()),
            Delete::Unsupported => Err(StreamableHttpError::ServerDoesNotSupportDeleteSession),
            Delete::Failed => Err(StreamableHttpError::Client(io::Error::other("delete rejected"))),
            Delete::Stall => std::future::pending().await,
            Delete::Panic => panic!("controlled cleanup panic"),
        }
        }
    }
    fn get_stream(
        &self, _uri: Arc<str>, _session_id: Option<Arc<str>>, _last_event_id: Option<String>,
        _auth_header: Option<String>, _custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> impl Future<Output = Result<BoxedSseStream, StreamableHttpError<Self::Error>>> + Send + '_ {
        let stream = self.stream(StreamKind::Common);
        async move { Ok(stream) }
    }
}

type TestTransport = WorkerTransport<StreamableHttpClientWorker<Backend>>;
fn worker(backend: Backend) -> (TestTransport, HttpCleanupObserver<io::Error>) {
    let worker = StreamableHttpClientWorker::new(backend,
        StreamableHttpClientTransportConfig::with_uri("http://fixture.invalid/mcp"));
    let observer = worker.cleanup_observer();
    (WorkerTransport::spawn(worker), observer)
}
async fn start(transport: &mut TestTransport) -> anyhow::Result<()> {
    transport.send(initialize()?).await?;
    transport.receive().await.context("missing initialize response")?;
    transport.send(initialized()?).await?;
    Ok(())
}
async fn wait(signal: &Notify) -> anyhow::Result<()> {
    tokio::time::timeout(Duration::from_secs(/*secs*/ 2), signal.notified()).await?;
    Ok(())
}

#[tokio::test]
async fn stalled_startup_request_is_dropped_without_inventing_remote_session() -> anyhow::Result<()> {
    let backend = Backend::new(Scenario::StartupStall, Delete::Deleted);
    let signals = Arc::clone(&backend.signals);
    let (mut transport, observer) = worker(backend);
    let send = tokio::spawn(transport.send(initialize()?));
    wait(&signals.startup).await?;
    let completion = tokio::time::timeout(Duration::from_secs(/*secs*/ 2), transport.close_observed())
        .await?.context("missing worker completion")?;
    let sent = send.await?;
    let receipt = observer.snapshot().context("missing cleanup receipt")?;
    assert!(sent.is_err());
    assert!(matches!(completion.result(), Ok(Err(WorkerQuitReason::Cancelled))));
    assert!(receipt.local_cleanup_complete());
    assert!(matches!(&receipt.remote_delete, HttpRemoteDeleteOutcome::NoKnownSession));
    assert_eq!(signals.dropped_requests.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn stalled_initialize_body_preserves_known_session_for_delete() -> anyhow::Result<()> {
    let backend = Backend::new(Scenario::BodyStall, Delete::Deleted);
    let signals = Arc::clone(&backend.signals);
    let (mut transport, observer) = worker(backend);
    transport.send(initialize()?).await?;
    wait(&signals.initial_body).await?;
    let completion = transport.close_observed().await.context("missing completion")?;
    let receipt = observer.snapshot().context("missing receipt")?;
    assert!(matches!(completion.result(), Ok(Err(WorkerQuitReason::Cancelled))));
    assert!(receipt.local_cleanup_complete());
    assert!(matches!(&receipt.remote_delete, HttpRemoteDeleteOutcome::Deleted));
    assert_eq!(signals.dropped_bodies.load(Ordering::SeqCst), 1);
    assert_eq!(*signals.deleted.lock().unwrap_or_else(std::sync::PoisonError::into_inner),
        vec!["known-session".to_owned()]);
    Ok(())
}

#[tokio::test]
async fn initialized_error_and_stall_both_run_session_cleanup() -> anyhow::Result<()> {
    for scenario in [Scenario::InitializedFailure, Scenario::InitializedStall] {
        let backend = Backend::new(scenario, Delete::Deleted);
        let signals = Arc::clone(&backend.signals);
        let (mut transport, observer) = worker(backend);
        transport.send(initialize()?).await?;
        transport.receive().await.context("missing initialize response")?;
        let send = tokio::spawn(transport.send(initialized()?));
        wait(&signals.initialized).await?;
        let completion = transport.close_observed().await.context("missing completion")?;
        let sent = send.await?;
        let receipt = observer.snapshot().context("early exit bypassed cleanup")?;
        assert!(sent.is_err());
        assert!(matches!(completion.result(), Ok(Err(_))));
        assert!(receipt.local_cleanup_complete());
        assert!(matches!(&receipt.remote_delete, HttpRemoteDeleteOutcome::Deleted));
    }
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn remote_delete_outcomes_do_not_erase_actual_sse_joins() -> anyhow::Result<()> {
    for delete in [Delete::Deleted, Delete::Unsupported, Delete::Failed, Delete::Stall] {
        let backend = Backend::new(Scenario::Live, delete);
        let signals = Arc::clone(&backend.signals);
        let (mut transport, observer) = worker(backend);
        start(&mut transport).await?;
        wait(&signals.common).await?;
        transport.send(tools_request()?).await?;
        wait(&signals.response).await?;
        let completion = transport.close_observed().await.context("missing completion")?;
        let receipt = observer.snapshot().context("missing receipt")?;
        assert!(completion.result().is_ok());
        assert!(receipt.local_cleanup_complete());
        assert_eq!(receipt.joined_sse_tasks, 2);
        assert_eq!(signals.dropped_bodies.load(Ordering::SeqCst), 2);
        assert!(match (delete, &receipt.remote_delete) {
            (Delete::Deleted, HttpRemoteDeleteOutcome::Deleted)
            | (Delete::Unsupported, HttpRemoteDeleteOutcome::Unsupported)
            | (Delete::Stall, HttpRemoteDeleteOutcome::TimedOut) => true,
            (Delete::Failed, HttpRemoteDeleteOutcome::Failed(StreamableHttpError::Client(error))) =>
                error.to_string() == "delete rejected",
            _ => false,
        });
    }
    Ok(())
}

#[tokio::test]
async fn protocol_panic_still_joins_sse_before_preserving_original_panic() -> anyhow::Result<()> {
    let backend = Backend::new(Scenario::ProtocolPanic, Delete::Deleted);
    let signals = Arc::clone(&backend.signals);
    let (mut transport, observer) = worker(backend);
    start(&mut transport).await?;
    wait(&signals.common).await?;
    let sent = transport.send(tools_request()?).await;
    let completion = transport.close_observed().await.context("missing completion")?;
    let receipt = observer.snapshot().context("panic skipped common cleanup")?;
    assert!(sent.is_err());
    assert!(matches!(completion.result(), Err(error) if error.is_panic()));
    assert!(receipt.local_cleanup_complete());
    assert_eq!(receipt.joined_sse_tasks, 1);
    assert_eq!(signals.dropped_bodies.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn actual_sse_task_panic_is_retained_in_cleanup_receipt() -> anyhow::Result<()> {
    let backend = Backend::new(Scenario::SsePanic, Delete::Deleted);
    let signals = Arc::clone(&backend.signals);
    let (mut transport, observer) = worker(backend);
    start(&mut transport).await?;
    wait(&signals.common).await?;
    let _completion = transport.close_observed().await.context("missing completion")?;
    let receipt = observer.snapshot().context("missing receipt")?;
    assert!(!receipt.local_cleanup_complete());
    assert_eq!(receipt.unexpected_sse_task_failures, 1);
    assert!(receipt.first_sse_task_failure.as_ref().is_some_and(tokio::task::JoinError::is_panic));
    assert_eq!(signals.dropped_bodies.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn cloned_worker_observations_cannot_report_another_workers_completion() -> anyhow::Result<()> {
    let first = StreamableHttpClientWorker::new(Backend::new(Scenario::StartupStall, Delete::Deleted),
        StreamableHttpClientTransportConfig::with_uri("http://fixture.invalid/mcp"));
    let second = first.clone();
    let first_observer = first.cleanup_observer();
    let shared_observer = first_observer.clone();
    let second_observer = second.cleanup_observer();
    let mut first = WorkerTransport::spawn(first);
    let mut second = WorkerTransport::spawn(second);
    let _first_completion = first.close_observed().await.context("first completion missing")?;
    let receipt = first_observer.snapshot().context("first receipt missing")?;
    let shared = shared_observer.snapshot().context("cloned observer receipt missing")?;
    assert!(Arc::ptr_eq(&receipt, &shared));
    assert!(second_observer.snapshot().is_none());
    let _second_completion = second.close_observed().await.context("second completion missing")?;
    assert!(second_observer.snapshot().is_some());
    Ok(())
}


#[cfg(feature = "transport-streamable-http-client-reqwest")]
#[tokio::test]
async fn actual_http_sse_task_joins_before_worker_completion() -> anyhow::Result<()> {
    use axum::body::Body;
    use axum::response::Response;

    async fn post(body: String) -> Result<Response, StatusCode> {
        let message: serde_json::Value = serde_json::from_str(&body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let response = if message["method"] == "initialize" {
            Response::builder().status(StatusCode::OK)
                .header("content-type", "application/json")
                .header("mcp-session-id", "real-sse-session")
                .body(Body::from(initialize_response().to_string()))
        } else {
            Response::builder().status(StatusCode::ACCEPTED).body(Body::empty())
        };
        response.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    let get_started = Arc::new(Notify::new());
    let stop = CancellationToken::new();
    let deletes = Arc::new(AtomicUsize::new(0));
    let get_signal = Arc::clone(&get_started);
    let release_body = stop.clone();
    let delete_count = Arc::clone(&deletes);
    let app = axum::Router::new().route("/mcp", axum::routing::post(post)
        .get(move || {
            let started = Arc::clone(&get_signal);
            let release = release_body.clone();
            async move {
                started.notify_one();
                // The real HTTP GET and open response exercise the pinned SSE
                // task. This gate does not claim that a body chunk was parsed.
                let body = futures::stream::once(async { Ok::<_, io::Error>(": held\n\n") })
                    .chain(futures::stream::once(async move {
                        release.cancelled().await;
                        Ok::<_, io::Error>("")
                    }));
                ([(http::header::CONTENT_TYPE, "text/event-stream")], Body::from_stream(body))
            }
        })
        .delete(move || {
            let deletes = Arc::clone(&delete_count);
            async move {
                deletes.fetch_add(1, Ordering::SeqCst);
                StatusCode::NO_CONTENT
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let uri = format!("http://{}/mcp", listener.local_addr()?);
    let shutdown = stop.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).with_graceful_shutdown(shutdown.cancelled_owned()).await
    });
    let worker = StreamableHttpClientWorker::new(reqwest::Client::new(),
        StreamableHttpClientTransportConfig::with_uri(uri));
    let observer = worker.cleanup_observer();
    let mut transport = WorkerTransport::spawn(worker);
    let exchange = async {
        transport.send(initialize()?).await?;
        transport.receive().await.context("missing network initialize response")?;
        transport.send(initialized()?).await?;
        wait(&get_started).await?;
        anyhow::Ok(())
    }.await;
    // Always release and join the owned fixture server before returning an
    // operation error or checking assertions; no server JoinHandle is detached.
    let completion = transport.close_observed().await;
    stop.cancel();
    server.await??;
    exchange?;
    let completion = completion.context("missing network worker completion")?;
    let receipt = observer.snapshot().context("missing network cleanup receipt")?;
    assert!(completion.result().is_ok());
    assert!(receipt.local_cleanup_complete());
    assert_eq!(receipt.joined_sse_tasks, 1);
    assert!(matches!(&receipt.remote_delete, HttpRemoteDeleteOutcome::Deleted));
    assert_eq!(deletes.load(Ordering::SeqCst), 1);
    Ok(())
}


#[tokio::test]
async fn cleanup_panic_leaves_receipt_absent_after_sse_was_joined() -> anyhow::Result<()> {
    let backend = Backend::new(Scenario::Live, Delete::Panic);
    let signals = Arc::clone(&backend.signals);
    let (mut transport, observer) = worker(backend);
    start(&mut transport).await?;
    wait(&signals.common).await?;
    let completion = transport.close_observed().await.context("missing completion")?;
    assert!(matches!(completion.result(), Err(error) if error.is_panic()));
    assert!(observer.snapshot().is_none());
    assert_eq!(signals.dropped_bodies.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn runtime_abort_during_delete_cannot_publish_a_clean_receipt() -> anyhow::Result<()> {
    let fixture = std::thread::spawn(|| -> anyhow::Result<_> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let observed = runtime.block_on(async {
            let backend = Backend::new(Scenario::Live, Delete::Stall);
            let signals = Arc::clone(&backend.signals);
            let (mut transport, observer) = worker(backend);
            start(&mut transport).await?;
            let mut close = Box::pin(transport.close_observed());
            tokio::select! {
                _ = signals.delete_started.notified() => {}
                _ = &mut close => anyhow::bail!("held DELETE completed unexpectedly"),
            }
            drop(close);
            anyhow::Ok((transport, observer))
        });
        drop(runtime);
        observed
    });
    let (mut transport, observer) = fixture.join()
        .map_err(|_| anyhow::anyhow!("runtime fixture panicked"))??;
    let completion = transport.close_observed().await.context("cancelled worker completion missing")?;
    assert!(matches!(completion.result(), Err(error) if error.is_cancelled()));
    assert!(observer.snapshot().is_none());
    Ok(())
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_observation_waits_for_actual_sse_task_destructor_completion() -> anyhow::Result<()> {
    let backend = Backend::new(Scenario::Live, Delete::Deleted);
    let signals = Arc::clone(&backend.signals);
    let gate = Arc::new(DropGate::default());
    let release = ReleaseDrop(Arc::clone(&gate));
    *signals.drop_gate.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&gate));
    let (mut transport, observer) = worker(backend);
    start(&mut transport).await?;
    wait(&signals.common).await?;
    let mut close = Box::pin(transport.close_observed());
    tokio::select! {
        _ = gate.entered.notified() => {}
        _ = &mut close => anyhow::bail!("worker completed before held SSE destructor"),
    }
    drop(close);
    let pending = tokio::time::timeout(Duration::from_millis(/*millis*/ 5), transport.close_observed()).await.is_err();
    let unpublished = observer.snapshot().is_none();
    // This guard also releases the native wait on every error/unwind path.
    drop(release);
    let completion = tokio::time::timeout(Duration::from_secs(/*secs*/ 2), transport.close_observed())
        .await?.context("missing completion after destructor release")?;
    let receipt = observer.snapshot().context("missing joined teardown receipt")?;
    assert!(pending && unpublished);
    assert!(completion.result().is_ok());
    assert!(receipt.local_cleanup_complete());
    assert_eq!(receipt.joined_sse_tasks, 1);
    assert_eq!(signals.dropped_bodies.load(Ordering::SeqCst), 1);
    Ok(())
}
