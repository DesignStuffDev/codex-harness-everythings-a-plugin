use super::{Transport, Worker, WorkerContext, WorkerQuitReason, WorkerTransport};
use crate::service::RoleClient;
use std::{io, sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tokio::time::timeout;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
struct OriginalError(Arc<()>);

impl std::fmt::Display for OriginalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("original controlled worker error")
    }
}

impl std::error::Error for OriginalError {}

enum Outcome {
    Success,
    Fatal(Arc<()>),
    Panic,
}

struct HeldWorker {
    stopped: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
    outcome: Outcome,
}

impl Worker for HeldWorker {
    type Error = io::Error;
    type Role = RoleClient;

    fn err_closed() -> Self::Error {
        io::Error::other("controlled worker closed")
    }

    fn err_join(error: tokio::task::JoinError) -> Self::Error {
        io::Error::other(error)
    }

    async fn run(self, context: WorkerContext<Self>) -> Result<(), WorkerQuitReason<Self::Error>> {
        context.cancellation_token.cancelled().await;
        let _ = self.stopped.send(());
        self.release.await.map_err(|_| WorkerQuitReason::HandlerTerminated)?;
        match self.outcome {
            Outcome::Success => Ok(()),
            Outcome::Fatal(marker) => Err(WorkerQuitReason::fatal(
                io::Error::other(OriginalError(marker)), "controlled worker operation",
            )),
            Outcome::Panic => panic!("controlled original worker panic"),
        }
    }
}

fn held(outcome: Outcome) -> (WorkerTransport<HeldWorker>, oneshot::Receiver<()>, oneshot::Sender<()>) {
    let (stopped_tx, stopped) = oneshot::channel();
    let (release, release_rx) = oneshot::channel();
    (WorkerTransport::spawn(HeldWorker {
        stopped: stopped_tx, release: release_rx, outcome,
    }), stopped, release)
}

#[tokio::test]
async fn cancelled_observer_keeps_original_handle_and_repeated_fatal_result() -> TestResult {
    let marker = Arc::new(());
    let (mut transport, stopped, release) = held(Outcome::Fatal(Arc::clone(&marker)));
    // This drops only the borrowed close observer while the actual worker is
    // held after seeing cancellation. It must not consume the original handle.
    assert!(timeout(Duration::from_millis(20), transport.close_observed()).await.is_err());
    timeout(Duration::from_secs(2), stopped).await??;
    assert!(transport.cancel_token().is_cancelled());
    release.send(()).map_err(|_| "worker lost release receiver")?;
    let first = timeout(Duration::from_secs(2), transport.close_observed()).await?
        .ok_or("original worker completion was lost")?;
    match first.result() {
        Ok(Err(WorkerQuitReason::Fatal { error, context })) => {
            assert_eq!(context.as_ref(), "controlled worker operation");
            let original = error.get_ref().and_then(|error| error.downcast_ref::<OriginalError>())
                .ok_or("original concrete operation error was replaced")?;
            assert!(Arc::ptr_eq(&original.0, &marker));
        }
        _ => return Err("expected original inner fatal result after a successful task join".into()),
    }
    let repeated = transport.close_observed().await.ok_or("repeated completion was lost")?;
    assert!(Arc::ptr_eq(&first, &repeated));
    // Legacy close after observation remains a no-op and cannot erase the cache.
    transport.close().await?;
    let after_legacy = transport.close_observed().await.ok_or("legacy close erased observed result")?;
    assert!(Arc::ptr_eq(&first, &after_legacy));
    Ok(())
}

#[tokio::test]
async fn normal_worker_completion_is_a_successful_outer_and_inner_result() -> TestResult {
    let (mut transport, stopped, release) = held(Outcome::Success);
    transport.cancel_token().cancel();
    timeout(Duration::from_secs(2), stopped).await??;
    release.send(()).map_err(|_| "worker lost release receiver")?;
    let completion = timeout(Duration::from_secs(2), transport.close_observed()).await?
        .ok_or("normal completion was lost")?;
    assert!(matches!(completion.result(), Ok(Ok(()))));
    let repeated = transport.close_observed().await.ok_or("normal result was not cached")?;
    assert!(Arc::ptr_eq(&completion, &repeated));
    Ok(())
}

#[tokio::test]
async fn original_worker_panic_stays_an_outer_join_error() -> TestResult {
    let (mut transport, stopped, release) = held(Outcome::Panic);
    transport.cancel_token().cancel();
    timeout(Duration::from_secs(2), stopped).await??;
    release.send(()).map_err(|_| "worker lost release receiver")?;
    let completion = timeout(Duration::from_secs(2), transport.close_observed()).await?
        .ok_or("panic completion was lost")?;
    match completion.result() {
        Err(error) => assert!(error.is_panic()),
        Ok(_) => return Err("worker panic was converted into an inner operation result".into()),
    }
    let repeated = transport.close_observed().await.ok_or("panic result was not cached")?;
    assert!(Arc::ptr_eq(&completion, &repeated));
    Ok(())
}

#[tokio::test]
async fn legacy_close_preserves_its_result_and_leaves_observation_unavailable() -> TestResult {
    let (mut transport, stopped, release) = held(Outcome::Fatal(Arc::new(())));
    transport.cancel_token().cancel();
    timeout(Duration::from_secs(2), stopped).await??;
    release.send(()).map_err(|_| "worker lost release receiver")?;
    // Legacy behavior ignores the inner operation error. The additive API
    // reports None afterwards, never manufacturing a clean completion receipt.
    timeout(Duration::from_secs(2), transport.close()).await??;
    assert!(transport.close_observed().await.is_none());
    assert!(transport.close_observed().await.is_none());
    Ok(())
}
