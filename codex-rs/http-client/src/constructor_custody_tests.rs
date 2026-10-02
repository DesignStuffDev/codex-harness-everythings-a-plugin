use super::*;
use crate::BuildRouteAwareHttpClientError;
use crate::ClientRouteClass;
use pretty_assertions::assert_eq;
use std::future::poll_fn;
use std::time::Duration;
use tokio::sync::oneshot;

fn build_error() -> BuildOutput {
    Err(BuildRouteAwareHttpClientError::InvalidProxyConfig {
        route_class: ClientRouteClass::Other,
    }
    .into())
}

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[tokio::test]
async fn original_build_error_is_transferred_and_capacity_is_reusable() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    for _ in 0..3 {
        let result = lease
            .start(build_error)
            .unwrap()
            .wait()
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            result,
            Err(RouteAwareClientPoolError::Build(
                BuildRouteAwareHttpClientError::InvalidProxyConfig {
                    route_class: ClientRouteClass::Other
                }
            ))
        ));
    }
    registry.begin_close(deadline());
    assert_eq!(
        registry.wait_until(deadline()).await,
        HttpConstructionObservation {
            retained: 0,
            pending: 0,
            failed: 0,
            joined: 3,
            cancelled: 0,
            recovery_attempts: 0,
        }
    );
}

#[tokio::test]
async fn native_panic_retains_original_join_error_and_unclean_receipt() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let result = lease
        .start(|| panic!("constructor test panic"))
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert!(result.unwrap_err().is_panic());
    registry.begin_close(deadline());
    assert_eq!(
        registry.wait_until(deadline()).await,
        HttpConstructionObservation {
            retained: 1,
            pending: 0,
            failed: 1,
            joined: 1,
            cancelled: 0,
            recovery_attempts: 0,
        }
    );
}

#[tokio::test]
async fn canceled_observer_keeps_native_handle_and_abandoned_disposal_owned() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let ticket = lease
        .start(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            build_error()
        })
        .unwrap();
    tokio::time::timeout_at(deadline().into(), entered_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        lease.start(build_error),
        Err(HttpConstructionError::CapacityExceeded)
    ));
    assert!(
        tokio::time::timeout_at(tokio::time::Instant::now(), ticket.wait())
            .await
            .is_err()
    );
    let pending = registry.wait_until(Instant::now()).await;
    assert_eq!(
        (pending.retained, pending.pending, pending.joined),
        (1, 1, 0)
    );
    release_tx.send(()).unwrap();
    assert_eq!(
        registry.wait_until(deadline()).await,
        HttpConstructionObservation {
            retained: 0,
            pending: 0,
            failed: 0,
            joined: 1,
            cancelled: 0,
            recovery_attempts: 0,
        }
    );
    // Cancellation does not globally close another accepted/public lease.
    let _result = lease
        .start(build_error)
        .unwrap()
        .wait()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn canceled_queued_poller_does_not_take_the_primary_handle() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let ticket = lease.start(build_error).unwrap();
    let permit = ticket.0.native.poller.acquire().await.unwrap();
    assert!(
        tokio::time::timeout_at(tokio::time::Instant::now(), ticket.0.native.join())
            .await
            .is_err()
    );
    drop(permit);
    let _result = ticket.wait().await.unwrap().unwrap();
    assert!(registry.wait_until(deadline()).await.is_complete());
}

#[tokio::test]
async fn observation_before_handle_attachment_is_woken() {
    let task = Arc::new(Task::new());
    let waiter_task = Arc::clone(&task);
    let (polling_tx, polling_rx) = oneshot::channel();
    let waiter = tokio::spawn(async move {
        let _permit = waiter_task.poller.acquire().await.unwrap();
        let mut polling_tx = Some(polling_tx);
        poll_fn(|cx| {
            let result = waiter_task.poll_join(cx);
            if let Some(sender) = polling_tx.take() {
                sender.send(()).unwrap();
            }
            result
        })
        .await;
    });
    tokio::time::timeout_at(deadline().into(), polling_rx)
        .await
        .unwrap()
        .unwrap();
    task.attach(Ok(tokio::task::spawn_blocking(|| 7)));
    tokio::time::timeout_at(deadline().into(), waiter)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(task.take().unwrap().unwrap(), 7);
}

struct HeldPanicPayload {
    entered: Option<oneshot::Sender<()>>,
    release: std::sync::mpsc::Receiver<()>,
}
impl Drop for HeldPanicPayload {
    fn drop(&mut self) {
        self.entered.take().unwrap().send(()).unwrap();
        self.release.recv().unwrap();
    }
}

#[tokio::test]
async fn abandoned_panic_payload_disposal_respects_shared_observation_deadline() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let ticket = lease
        .start(move || {
            std::panic::panic_any(HeldPanicPayload {
                entered: Some(entered_tx),
                release: release_rx,
            })
        })
        .unwrap();
    drop(ticket);
    tokio::time::timeout_at(deadline().into(), entered_rx)
        .await
        .unwrap()
        .unwrap();
    registry.begin_close(deadline());
    assert_eq!(
        registry.wait_until(Instant::now()).await,
        HttpConstructionObservation {
            retained: 1,
            pending: 1,
            failed: 1,
            joined: 1,
            cancelled: 0,
            recovery_attempts: 0,
        }
    );
    release_tx.send(()).unwrap();
    assert_eq!(
        registry.wait_until(deadline()).await,
        HttpConstructionObservation {
            retained: 1,
            pending: 0,
            failed: 1,
            joined: 1,
            cancelled: 0,
            recovery_attempts: 0,
        }
    );
}

#[tokio::test]
async fn process_observer_never_renews_its_original_deadline() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let ticket = lease
        .start(move || {
            release_rx.recv().unwrap();
            build_error()
        })
        .unwrap();
    drop(ticket);
    registry.begin_close(deadline());
    let shutdown = HttpConstructionProcessShutdown {
        registry: Arc::clone(&registry),
        deadline: Instant::now(),
    };
    let observed = shutdown.wait_until(deadline()).await;
    assert_eq!((observed.pending, observed.joined), (1, 0));
    release_tx.send(()).unwrap();
    assert!(registry.wait_until(deadline()).await.is_complete());
}

#[tokio::test]
async fn repeated_process_close_cannot_renew_the_registry_deadline() {
    let registry = Registry::new(/*capacity*/ 1);
    let lease = PoolLease::with_registry(Arc::clone(&registry));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let ticket = lease
        .start(move || {
            release_rx.recv().unwrap();
            build_error()
        })
        .unwrap();
    drop(ticket);
    let first = registry.begin_close(Instant::now());
    let later = registry.begin_close(deadline());
    let observed = registry.wait_until(deadline()).await;
    // Release native work before checking the close/observation results.
    release_tx.send(()).unwrap();
    assert_eq!(later, first);
    assert_eq!((observed.pending, observed.joined), (1, 0));
    let records = lock(&registry.state).records.clone();
    for record in records {
        tokio::time::timeout_at(deadline().into(), record.settle())
            .await
            .unwrap();
    }
}
