#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use pretty_assertions::assert_eq;
use tokio::task::AbortHandle;
use tokio::time::sleep;
use tokio::time::timeout;

use super::*;

fn client(
    command_tx: mpsc::Sender<ClientCommand>,
    worker_handle: JoinHandle<()>,
) -> InProcessAppServerClient {
    let (_event_tx, event_rx) = mpsc::unbounded_channel();
    InProcessAppServerClient {
        command_tx,
        event_rx,
        worker_handle,
        shutdown_runtime: Handle::current(),
    }
}

async fn acknowledgement(
    commands: &mut mpsc::Receiver<ClientCommand>,
) -> oneshot::Sender<IoResult<()>> {
    match commands.recv().await {
        Some(ClientCommand::Shutdown { response_tx }) => response_tx,
        _ => panic!("expected owned shutdown command"),
    }
}

async fn finished(worker: &AbortHandle) {
    timeout(Duration::from_secs(1), async {
        while !worker.is_finished() {
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("facade worker must terminate");
}

#[tokio::test(start_paused = true)]
async fn shutdown_allows_the_shared_embedded_budget_before_joining() {
    let (commands, mut receiver) = mpsc::channel(1);
    let worker = tokio::spawn(async move {
        let ack = acknowledgement(&mut receiver).await;
        sleep(IN_PROCESS_SHUTDOWN_BUDGET).await;
        ack.send(Ok(())).unwrap();
        sleep(Duration::from_secs(2)).await;
    });
    let started = Instant::now();
    client(commands, worker).shutdown().await.unwrap();
    assert_eq!(
        Instant::now() - started,
        IN_PROCESS_SHUTDOWN_BUDGET + Duration::from_secs(2)
    );
}

#[tokio::test(start_paused = true)]
async fn full_command_queue_is_inside_the_single_deadline() {
    let (commands, receiver) = mpsc::channel(1);
    let (occupied, _response) = oneshot::channel();
    assert!(
        commands
            .try_send(ClientCommand::Shutdown {
                response_tx: occupied
            })
            .is_ok()
    );
    let worker = tokio::spawn(async move {
        let _keep_channel_open = receiver;
        std::future::pending::<()>().await;
    });
    let task = worker.abort_handle();
    let started = Instant::now();
    let error = client(commands, worker).shutdown().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert_eq!(Instant::now() - started, FACADE_SHUTDOWN_BUDGET);
    assert!(error.to_string().contains("admission or acknowledgement"));
    assert!(error.to_string().contains("unconfirmed"));
    finished(&task).await;
}

#[tokio::test(start_paused = true)]
async fn acknowledgement_error_still_waits_for_worker_and_keeps_first_failure() {
    let (commands, mut receiver) = mpsc::channel(1);
    let (acknowledged, saw_ack) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let worker = tokio::spawn(async move {
        acknowledgement(&mut receiver)
            .await
            .send(Err(IoError::new(
                ErrorKind::PermissionDenied,
                "storage drain denied",
            )))
            .unwrap();
        acknowledged.send(()).unwrap();
        released.await.unwrap();
    });
    let task = worker.abort_handle();
    let receipt = tokio::spawn(client(commands, worker).shutdown());
    saw_ack.await.unwrap();
    assert!(!receipt.is_finished());
    release.send(()).unwrap();
    let error = receipt.await.unwrap().unwrap_err();
    assert_eq!(
        (error.kind(), error.to_string()),
        (
            ErrorKind::PermissionDenied,
            "storage drain denied".to_owned()
        )
    );
    assert!(task.is_finished());
}

#[tokio::test(start_paused = true)]
async fn acknowledgement_loss_cannot_be_reported_as_success() {
    let (commands, mut receiver) = mpsc::channel(1);
    let worker = tokio::spawn(async move {
        drop(acknowledgement(&mut receiver).await);
    });
    let error = client(commands, worker).shutdown().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::BrokenPipe);
    assert!(error.to_string().contains("acknowledgement lost"));
}

#[tokio::test(start_paused = true)]
async fn worker_panic_is_reported_with_or_without_successful_acknowledgement() {
    for mode in ["before acknowledgement", "after acknowledgement"] {
        let (commands, mut receiver) = mpsc::channel(1);
        let worker = tokio::spawn(async move {
            let ack = acknowledgement(&mut receiver).await;
            if mode == "after acknowledgement" {
                ack.send(Ok(())).unwrap();
            }
            panic!("fixture worker panicked {mode}");
        });
        let error = client(commands, worker).shutdown().await.unwrap_err();
        assert!(error.to_string().contains("worker failed"), "{error}");
        assert!(error.to_string().contains("panicked"), "{error}");
        assert!(error.to_string().contains("unconfirmed"));
    }
}

#[tokio::test(start_paused = true)]
async fn first_operation_error_survives_later_worker_panic() {
    let (commands, mut receiver) = mpsc::channel(1);
    let worker = tokio::spawn(async move {
        acknowledgement(&mut receiver)
            .await
            .send(Err(IoError::new(
                ErrorKind::PermissionDenied,
                "first storage failure",
            )))
            .unwrap();
        panic!("later worker panic");
    });
    let error = client(commands, worker).shutdown().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
    assert!(error.to_string().starts_with("first storage failure;"));
    assert!(error.to_string().contains("later worker panic"));
}

#[tokio::test(start_paused = true)]
async fn acknowledgement_does_not_reset_the_deadline_for_a_stalled_worker() {
    let (commands, mut receiver) = mpsc::channel(1);
    let worker = tokio::spawn(async move {
        let ack = acknowledgement(&mut receiver).await;
        sleep(Duration::from_secs(100)).await;
        ack.send(Ok(())).unwrap();
        std::future::pending::<()>().await;
    });
    let task = worker.abort_handle();
    let started = Instant::now();
    let error = client(commands, worker).shutdown().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert_eq!(Instant::now() - started, FACADE_SHUTDOWN_BUDGET);
    assert!(error.to_string().contains("abort requested"));
    finished(&task).await;
}

#[tokio::test(start_paused = true)]
async fn dropping_unpolled_shutdown_receipt_preserves_owned_graceful_shutdown() {
    let (commands, mut receiver) = mpsc::channel(1);
    let _retained_request_sender = commands.clone();
    let (done, finished_shutdown) = oneshot::channel();
    let worker = tokio::spawn(async move {
        let ack = acknowledgement(&mut receiver).await;
        sleep(Duration::from_secs(100)).await;
        ack.send(Ok(())).unwrap();
        done.send(()).unwrap();
    });
    let task = worker.abort_handle();
    drop(client(commands, worker).shutdown());
    finished_shutdown.await.unwrap();
    finished(&task).await;
}

#[tokio::test(start_paused = true)]
async fn enum_dispatcher_preserves_shutdown_ownership_before_its_receipt_is_polled() {
    let (commands, mut receiver) = mpsc::channel(1);
    let _retained_request_sender = commands.clone();
    let (done, finished_shutdown) = oneshot::channel();
    let worker = tokio::spawn(async move {
        let ack = acknowledgement(&mut receiver).await;
        sleep(Duration::from_secs(100)).await;
        ack.send(Ok(())).unwrap();
        done.send(()).unwrap();
    });
    let task = worker.abort_handle();
    let facade = crate::AppServerClient::InProcess(client(commands, worker));
    drop(facade.shutdown());
    finished_shutdown.await.unwrap();
    finished(&task).await;
}

#[tokio::test(start_paused = true)]
async fn cancelling_polled_shutdown_receipt_does_not_abort_the_drain() {
    let (commands, mut receiver) = mpsc::channel(1);
    let (started, saw_start) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let completed = Arc::new(AtomicBool::new(false));
    let worker_completed = Arc::clone(&completed);
    let worker = tokio::spawn(async move {
        let ack = acknowledgement(&mut receiver).await;
        started.send(()).unwrap();
        released.await.unwrap();
        worker_completed.store(true, Ordering::Release);
        ack.send(Ok(())).unwrap();
    });
    let task = worker.abort_handle();
    let receipt = tokio::spawn(client(commands, worker).shutdown());
    saw_start.await.unwrap();
    receipt.abort();
    assert!(receipt.await.unwrap_err().is_cancelled());
    assert!(!completed.load(Ordering::Acquire));
    release.send(()).unwrap();
    finished(&task).await;
    assert!(completed.load(Ordering::Acquire));
}

#[test]
fn shutdown_future_can_be_constructed_outside_its_runtime_context() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let client = runtime.block_on(async {
        let (commands, mut receiver) = mpsc::channel(1);
        let worker = tokio::spawn(async move {
            acknowledgement(&mut receiver).await.send(Ok(())).unwrap();
        });
        client(commands, worker)
    });
    let receipt = client.shutdown();
    runtime.block_on(receipt).unwrap();
}

#[test]
fn destroyed_runtime_cannot_produce_a_successful_shutdown_receipt() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let client = runtime.block_on(async {
        let (commands, receiver) = mpsc::channel(1);
        let worker = tokio::spawn(async move {
            let _keep_channel_open = receiver;
            std::future::pending::<()>().await;
        });
        client(commands, worker)
    });
    drop(runtime);
    let receipt = client.shutdown();
    let observing_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let error = observing_runtime.block_on(receipt).unwrap_err();
    assert!(error.to_string().contains("cleanup are unconfirmed"));
}
