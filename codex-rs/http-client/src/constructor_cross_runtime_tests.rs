use super::task::Record;
use super::*;
use crate::BuildCustomCaTransportError;
use crate::BuildRouteAwareHttpClientError;
use crate::ClientRouteClass;
use pretty_assertions::assert_eq;
use std::future::poll_fn;
use std::task::Poll;
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug)]
struct DisposalProbe(Option<oneshot::Sender<()>>);

impl std::fmt::Display for DisposalProbe {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("owned constructor test error")
    }
}

impl std::error::Error for DisposalProbe {}

impl Drop for DisposalProbe {
    fn drop(&mut self) {
        if let Some(disposed) = self.0.take() {
            let _ = disposed.send(());
        }
    }
}

fn error_with_probe(disposed: oneshot::Sender<()>) -> BuildOutput {
    Err(
        BuildRouteAwareHttpClientError::CustomCa(BuildCustomCaTransportError::ReadCaFile {
            source_env: "CONSTRUCTOR_TEST_FIXTURE",
            path: "not-read-fixture.pem".into(),
            source: std::io::Error::other(DisposalProbe(Some(disposed))),
        })
        .into(),
    )
}

fn ordinary_error() -> BuildOutput {
    Err(BuildRouteAwareHttpClientError::InvalidProxyConfig {
        route_class: ClientRouteClass::Other,
    }
    .into())
}

async fn admission_recovers_capacity(
    registry: &Arc<Registry>,
    lease: &PoolLease,
    original: &Arc<Job>,
    disposed: oneshot::Receiver<()>,
    expected_cancellations: usize,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(/*secs*/ 5);
    // Ordinary admission is the recovery trigger; no process-final close or
    // registry observer has run on B before the original output is disposed.
    match lease.start(ordinary_error) {
        Ok(ticket) => {
            let result = tokio::time::timeout_at(deadline, ticket.wait())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(result.is_err());
        }
        Err(error) => assert_eq!(error, HttpConstructionError::CapacityExceeded),
    }
    tokio::time::timeout_at(deadline, disposed)
        .await
        .unwrap()
        .unwrap();
    // The destructor signal precedes task completion. Join those exact tasks
    // before requiring capacity reuse; neither passive join starts recovery.
    tokio::time::timeout_at(deadline, async {
        original.disposal.join().await;
        original.cleanup.join().await;
    })
    .await
    .unwrap();
    let ticket = lease.start(ordinary_error).unwrap();
    let result = tokio::time::timeout_at(deadline, ticket.wait())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    let observed = registry.wait_until(deadline.into_std()).await;
    assert!(observed.is_complete());
    assert_eq!(
        (observed.cancelled, observed.recovery_attempts),
        (expected_cancellations, 1)
    );
}

#[test]
fn ordinary_admission_recovers_cleanup_canceled_with_previous_runtime() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let (disposed_tx, mut disposed_rx) = oneshot::channel();
    let runtime_a = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let original = runtime_a.block_on(async {
        let ticket = lease.start(move || error_with_probe(disposed_tx)).unwrap();
        let original = Arc::clone(&ticket.0);
        original.native.join().await;
        // The native result is positively joined. Current-thread A cannot poll
        // this newly spawned cleanup before this root future returns.
        drop(ticket);
        original
    });
    drop(runtime_a);
    assert_eq!(
        disposed_rx.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    );
    let runtime_b = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime_b.block_on(admission_recovers_capacity(
        &registry,
        &lease,
        &original,
        disposed_rx,
        /*expected_cancellations*/ 1,
    ));
}

#[test]
fn ordinary_admission_recovers_canceled_disposal_queued_on_previous_runtime() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let (disposed_tx, mut disposed_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let runtime_a = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(/*val*/ 1)
        .build()
        .unwrap();
    let (original, blocker) = runtime_a.block_on(async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(/*secs*/ 5);
        let ticket = lease.start(move || error_with_probe(disposed_tx)).unwrap();
        let original = Arc::clone(&ticket.0);
        original.native.join().await;
        let (entered_tx, entered_rx) = oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            entered_tx.send(()).unwrap();
            release_rx
                .recv_timeout(Duration::from_secs(/*secs*/ 5))
                .unwrap();
        });
        tokio::time::timeout_at(deadline, entered_rx)
            .await
            .unwrap()
            .unwrap();
        drop(ticket);
        // The sole blocking worker remains held: attachment proves disposal is
        // queued, while the payload stays in the native result record. Abort
        // that exact queued task before release; runtime shutdown alone can
        // allow a busy worker to execute queued blocking work before exiting.
        tokio::time::timeout_at(
            deadline,
            poll_fn(|cx| {
                let record = lock(&original.disposal.record);
                match &*record {
                    Record::AwaitingHandle => {
                        original.disposal.attachment_waker.register(cx.waker());
                        Poll::Pending
                    }
                    Record::Running(handle) => {
                        assert!(!handle.is_finished());
                        handle.abort();
                        Poll::Ready(())
                    }
                    Record::Joined(_) | Record::SpawnFailed { .. } => {
                        panic!("disposal did not remain queued behind the held worker")
                    }
                }
            }),
        )
        .await
        .unwrap();
        (original, blocker)
    });
    runtime_a.shutdown_background();
    release_tx.send(()).unwrap();
    let runtime_b = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime_b.block_on(async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(/*secs*/ 5);
        tokio::time::timeout_at(deadline, blocker)
            .await
            .unwrap()
            .unwrap();
        // Abort marks the queued task; the released worker must consume it
        // before its canceled result can be joined. These passive joins and
        // cancellation checks run before B's admission can start recovery.
        tokio::time::timeout_at(deadline, async {
            original.disposal.join().await;
            original.cleanup.join().await;
        })
        .await
        .unwrap();
        assert!(original.disposal.cancelled.load(Ordering::Acquire));
        assert!(original.cleanup.cancelled.load(Ordering::Acquire));
        assert_eq!(
            disposed_rx.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        );
        admission_recovers_capacity(
            &registry,
            &lease,
            &original,
            disposed_rx,
            /*expected_cancellations*/ 2,
        )
        .await;
    });
}

#[tokio::test(flavor = "current_thread")]
async fn canceled_cleanup_after_successful_disposal_is_settled() {
    #[derive(Debug)]
    struct CountedProbe(DisposalProbe, Arc<AtomicUsize>);
    impl std::fmt::Display for CountedProbe {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.0, formatter)
        }
    }
    impl std::error::Error for CountedProbe {}
    impl Drop for CountedProbe {
        fn drop(&mut self) {
            self.1.fetch_add(1, Ordering::AcqRel);
        }
    }

    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let drops = Arc::new(AtomicUsize::new(/*v*/ 0));
    let (disposed_tx, disposed_rx) = oneshot::channel();
    let probe = CountedProbe(DisposalProbe(Some(disposed_tx)), Arc::clone(&drops));
    let ticket = lease
        .start(move || {
            Err(
                BuildRouteAwareHttpClientError::CustomCa(BuildCustomCaTransportError::ReadCaFile {
                    source_env: "CONSTRUCTOR_TEST_FIXTURE",
                    path: "not-read-fixture.pem".into(),
                    source: std::io::Error::other(probe),
                })
                .into(),
            )
        })
        .unwrap();
    let original = Arc::clone(&ticket.0);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(/*secs*/ 5);
    tokio::time::timeout_at(deadline, original.native.join())
        .await
        .unwrap();
    let disposal_permit = original.disposal.poller.try_acquire().unwrap();
    drop(ticket);

    // Cleanup can attach disposal, but cannot pass its disposal.join() while
    // this test retains the sole poller permit.
    tokio::time::timeout_at(
        deadline,
        poll_fn(|cx| {
            let record = lock(&original.disposal.record);
            match &*record {
                Record::AwaitingHandle => {
                    original.disposal.attachment_waker.register(cx.waker());
                    Poll::Pending
                }
                Record::Running(_) => Poll::Ready(()),
                Record::Joined(_) | Record::SpawnFailed { .. } => {
                    panic!("disposal must be attached and remain unobserved")
                }
            }
        }),
    )
    .await
    .unwrap();
    tokio::time::timeout_at(deadline, disposed_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout_at(deadline, poll_fn(|cx| original.disposal.poll_join(cx)))
        .await
        .unwrap();
    assert!(original.disposal.succeeded());
    assert_eq!(drops.load(Ordering::Acquire), 1);

    {
        let record = lock(&original.cleanup.record);
        let Record::Running(handle) = &*record else {
            panic!("cleanup must still be awaiting the disposal poller permit");
        };
        handle.abort();
    }
    drop(disposal_permit);
    tokio::time::timeout_at(deadline, original.cleanup.join())
        .await
        .unwrap();
    assert!(original.cleanup.cancelled.load(Ordering::Acquire));

    let expected = HttpConstructionObservation {
        retained: 0,
        pending: 0,
        failed: 0,
        joined: 1,
        cancelled: 1,
        recovery_attempts: 0,
    };
    assert_eq!(registry.wait_until(deadline.into_std()).await, expected);
    // Re-observation neither retries disposal nor counts its cancellation twice.
    assert_eq!(registry.wait_until(deadline.into_std()).await, expected);
    assert_eq!(drops.load(Ordering::Acquire), 1);
}
