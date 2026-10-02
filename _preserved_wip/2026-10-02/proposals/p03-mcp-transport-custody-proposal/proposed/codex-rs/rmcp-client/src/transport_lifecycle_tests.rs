use super::*;
use pretty_assertions::assert_eq;
use std::error::Error;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tokio::sync::oneshot;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error("original send failure")]
    Send(Arc<()>),
    #[error("original close failure")]
    Close(Arc<()>),
    #[error("transport closure unavailable")]
    Closed,
}

#[derive(Clone, Copy)]
enum Behavior {
    Complete,
    Fail,
    Panic,
}

struct TestTransport {
    calls: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    marker: Arc<()>,
    started: Option<oneshot::Sender<()>>,
    release: oneshot::Receiver<()>,
    behavior: Behavior,
    incoming: Option<RxJsonRpcMessage<RoleClient>>,
}

impl Transport<RoleClient> for TestTransport {
    type Error = TestError;

    fn send(
        &mut self,
        _item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        std::future::ready(Err(TestError::Send(Arc::clone(&self.marker))))
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
        self.incoming.take()
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        (&mut self.release).await.map_err(|_| TestError::Closed)?;
        match self.behavior {
            Behavior::Complete => Ok(()),
            Behavior::Fail => Err(TestError::Close(Arc::clone(&self.marker))),
            Behavior::Panic => panic!("controlled transport close panic"),
        }
    }
}

impl Drop for TestTransport {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::AcqRel);
    }
}

struct Fixture {
    guard: TransportGuard<TestTransport>,
    observer: Arc<dyn TransportCloseObserver>,
    owner: Arc<CloseOwner<TestTransport>>,
    registry: Arc<Registry>,
    started: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    calls: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    marker: Arc<()>,
}

fn fixture(
    runtime: Handle,
    confirmation: TransportCloseConfirmation,
    behavior: Behavior,
) -> Fixture {
    let registry = Arc::new(Registry::default());
    let calls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    let marker = Arc::new(());
    let (started_tx, started) = oneshot::channel();
    let (release, release_rx) = oneshot::channel();
    let (guard, observer) = TransportGuard::new_in(
        TestTransport {
            calls: Arc::clone(&calls),
            drops: Arc::clone(&drops),
            marker: Arc::clone(&marker),
            started: Some(started_tx),
            release: release_rx,
            behavior,
            incoming: None,
        },
        runtime,
        confirmation,
        || TestError::Closed,
        &registry,
    );
    let owner = Arc::clone(&guard.owner);
    Fixture { guard, observer, owner, registry, started, release, calls, drops, marker }
}

fn count(registry: &Registry) -> usize {
    registry.owners.lock().unwrap_or_else(PoisonError::into_inner).len()
}

async fn observe(observer: &dyn TransportCloseObserver) -> Result<TransportCloseConfirmation, Box<dyn Error + Send + Sync>> {
    Ok(tokio::time::timeout(Duration::from_secs(2), observer.wait()).await??)
}

#[tokio::test]
async fn cancelled_and_expired_close_observers_share_one_retained_transport() -> TestResult {
    let mut fixture = fixture(Handle::current(), TransportCloseConfirmation::Confirmed, Behavior::Complete);
    // Calling close must transfer custody even if this future is never polled.
    drop(fixture.guard.close());
    tokio::time::timeout(Duration::from_secs(2), fixture.started).await??;
    assert_eq!(count(&fixture.registry), 1);
    assert!(tokio::time::timeout(Duration::from_millis(2), fixture.observer.wait()).await.is_err());
    let waiter = tokio::spawn({
        let observer = Arc::clone(&fixture.observer);
        async move { observer.wait().await }
    });
    tokio::task::yield_now().await;
    waiter.abort();
    assert!(waiter.await.err().ok_or("waiter unexpectedly completed")?.is_cancelled());
    drop(fixture.guard);
    assert_eq!(fixture.drops.load(Ordering::Acquire), 0);
    fixture.release.send(()).map_err(|_| "close task lost release receiver")?;
    let (first, second) = tokio::join!(observe(fixture.observer.as_ref()), observe(fixture.observer.as_ref()));
    assert_eq!(first?, TransportCloseConfirmation::Confirmed);
    assert_eq!(second?, TransportCloseConfirmation::Confirmed);
    assert_eq!(observe(fixture.observer.as_ref()).await?, TransportCloseConfirmation::Confirmed);
    assert_eq!(fixture.calls.load(Ordering::Acquire), 1);
    assert_eq!(count(&fixture.registry), 0);
    drop(fixture.observer);
    drop(fixture.owner);
    assert_eq!(fixture.drops.load(Ordering::Acquire), 1);
    Ok(())
}

#[tokio::test]
async fn handshake_drop_starts_close_and_unconfirmed_observation_keeps_custody() -> TestResult {
    let fixture = fixture(Handle::current(), TransportCloseConfirmation::Unconfirmed, Behavior::Complete);
    let weak = Arc::downgrade(&fixture.owner);
    drop(fixture.guard);
    tokio::time::timeout(Duration::from_secs(2), fixture.started).await??;
    fixture.release.send(()).map_err(|_| "close task lost release receiver")?;
    for _ in 0..2 {
        assert_eq!(observe(fixture.observer.as_ref()).await?, TransportCloseConfirmation::Unconfirmed);
    }
    assert_eq!(count(&fixture.registry), 1);
    drop(fixture.observer);
    drop(fixture.owner);
    assert!(weak.upgrade().is_some());
    assert_eq!(fixture.drops.load(Ordering::Acquire), 0);
    assert_eq!(fixture.calls.load(Ordering::Acquire), 1);
    Ok(())
}

#[tokio::test]
async fn close_error_is_sticky_and_original_error_remains_in_primary_custody() -> TestResult {
    let mut fixture = fixture(Handle::current(), TransportCloseConfirmation::Confirmed, Behavior::Fail);
    drop(fixture.guard.close());
    tokio::time::timeout(Duration::from_secs(2), fixture.started).await??;
    fixture.release.send(()).map_err(|_| "close task lost release receiver")?;
    for _ in 0..2 {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), fixture.observer.wait()).await?,
            Err(TransportCloseFailure::Transport),
        );
        assert!(matches!(fixture.guard.close().await, Err(TestError::Closed)));
    }
    let task = fixture.owner.task.lock().unwrap_or_else(PoisonError::into_inner).clone()
        .ok_or("close task was not published")?;
    let original = task.wait().await?;
    match original.as_ref() {
        Err(TestError::Close(marker)) => assert!(Arc::ptr_eq(marker, &fixture.marker)),
        _ => return Err("original concrete close failure was not retained".into()),
    }
    assert_eq!(count(&fixture.registry), 1);
    assert_eq!(fixture.calls.load(Ordering::Acquire), 1);
    let weak = Arc::downgrade(&fixture.owner);
    drop(fixture.guard);
    drop(fixture.observer);
    drop(fixture.owner);
    assert!(weak.upgrade().is_some());
    assert_eq!(fixture.drops.load(Ordering::Acquire), 0);
    Ok(())
}

#[tokio::test]
async fn panic_retains_transport_and_reports_the_same_join_failure() -> TestResult {
    let mut fixture = fixture(Handle::current(), TransportCloseConfirmation::Confirmed, Behavior::Panic);
    drop(fixture.guard.close());
    tokio::time::timeout(Duration::from_secs(2), fixture.started).await??;
    fixture.release.send(()).map_err(|_| "close task lost release receiver")?;
    for _ in 0..2 {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), fixture.observer.wait()).await?,
            Err(TransportCloseFailure::Task(TaskJoinFailure::Panicked)),
        );
        assert!(matches!(fixture.guard.close().await, Err(TestError::Closed)));
    }
    drop(fixture.guard);
    drop(fixture.observer);
    drop(fixture.owner);
    assert_eq!(count(&fixture.registry), 1);
    assert_eq!(fixture.drops.load(Ordering::Acquire), 0);
    Ok(())
}

#[tokio::test]
async fn runtime_cancellation_retains_transport_after_close_future_is_destroyed() -> TestResult {
    let (owner, observer, registry, drops, calls) = std::thread::spawn(|| -> Result<_, Box<dyn Error + Send + Sync>> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let fixture = fixture(runtime.handle().clone(), TransportCloseConfirmation::Confirmed, Behavior::Complete);
        drop(fixture.guard);
        runtime.block_on(async { tokio::time::timeout(Duration::from_secs(2), fixture.started).await })??;
        // Runtime destruction drops the actual close future while the release
        // channel remains alive. The separately registered owner must keep T.
        drop(runtime);
        assert_eq!(fixture.drops.load(Ordering::Acquire), 0);
        drop(fixture.release);
        Ok((fixture.owner, fixture.observer, fixture.registry, fixture.drops, fixture.calls))
    }).join().map_err(|_| "controlled runtime thread panicked")??;
    for _ in 0..2 {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), observer.wait()).await?,
            Err(TransportCloseFailure::Task(TaskJoinFailure::Cancelled)),
        );
    }
    let weak = Arc::downgrade(&owner);
    drop(observer);
    drop(owner);
    assert!(weak.upgrade().is_some());
    assert_eq!(count(&registry), 1);
    assert_eq!(drops.load(Ordering::Acquire), 0);
    assert_eq!(calls.load(Ordering::Acquire), 1);
    Ok(())
}

#[tokio::test]
async fn ordinary_send_and_receive_preserve_concrete_transport_contract() -> TestResult {
    let mut fixture = fixture(Handle::current(), TransportCloseConfirmation::Confirmed, Behavior::Complete);
    assert!(Arc::ptr_eq(&fixture.guard.inner().marker, &fixture.marker));
    let message = serde_json::from_value(serde_json::json!({"jsonrpc":"2.0", "id":1, "method":"ping"}))?;
    let result: Result<(), TestError> = fixture.guard.send(message).await;
    match result {
        Err(TestError::Send(marker)) => assert!(Arc::ptr_eq(&marker, &fixture.marker)),
        _ => return Err("ordinary send error changed concrete type or identity".into()),
    }
    let incoming = serde_json::json!({"jsonrpc":"2.0", "id":1, "result":{}});
    let transport = fixture.guard.transport.as_mut().ok_or("transport unexpectedly closed")?;
    transport.incoming = Some(serde_json::from_value(incoming.clone())?);
    assert_eq!(serde_json::to_value(fixture.guard.receive().await)?, incoming);
    assert_eq!(count(&fixture.registry), 0);
    drop(fixture.guard.close());
    tokio::time::timeout(Duration::from_secs(2), fixture.started).await??;
    fixture.release.send(()).map_err(|_| "close task lost release receiver")?;
    assert_eq!(observe(fixture.observer.as_ref()).await?, TransportCloseConfirmation::Confirmed);
    assert!(fixture.guard.receive().await.is_none());
    Ok(())
}

#[tokio::test]
async fn spawn_panic_preserves_registered_transport_before_future_ever_runs() -> TestResult {
    let mut fixture = fixture(Handle::current(), TransportCloseConfirmation::Confirmed, Behavior::Complete);
    let transport = fixture.guard.transport.take().ok_or("transport unexpectedly closed")?;
    fixture.owner.start_published(transport, || {
        panic!("controlled publisher panic before spawn");
    });
    for _ in 0..2 {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), fixture.observer.wait()).await?,
            Err(TransportCloseFailure::Task(TaskJoinFailure::SpawnPanicked)),
        );
        assert!(matches!(fixture.guard.close().await, Err(TestError::Closed)));
    }
    let weak = Arc::downgrade(&fixture.owner);
    drop(fixture.guard);
    drop(fixture.observer);
    drop(fixture.owner);
    assert!(weak.upgrade().is_some());
    assert_eq!(count(&fixture.registry), 1);
    assert_eq!(fixture.drops.load(Ordering::Acquire), 0);
    assert_eq!(fixture.calls.load(Ordering::Acquire), 0);
    Ok(())
}
