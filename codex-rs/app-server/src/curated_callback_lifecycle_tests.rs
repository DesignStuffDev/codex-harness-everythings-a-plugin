use super::CuratedCallbackGuard;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Semaphore;

const BUDGET: Duration = Duration::from_secs(/*secs*/ 5);

#[tokio::test]
async fn dropping_one_owner_fences_only_its_embedded_scope() -> anyhow::Result<()> {
    let first = CuratedCallbackGuard::new();
    let second = CuratedCallbackGuard::new();
    let first_scope = first.scope();
    let second_scope = second.scope();
    let first_effects = Arc::new(AtomicUsize::new(/*v*/ 0));
    let first_callback = first_scope.callback({
        let effects = Arc::clone(&first_effects);
        move || {
            effects.fetch_add(/*val*/ 1, Ordering::SeqCst);
            std::future::ready(())
        }
    });
    let done = Arc::new(Semaphore::new(/*permits*/ 0));
    let second_callback = second_scope.callback({
        let done = Arc::clone(&done);
        move || {
            let done = Arc::clone(&done);
            async move {
                done.add_permits(/*n*/ 1);
            }
        }
    });
    drop(first);
    assert!(!first_callback.dispatch());
    assert_eq!(first_effects.load(Ordering::SeqCst), 0);
    assert!(second_callback.dispatch());
    tokio::time::timeout(BUDGET, done.acquire())
        .await??
        .forget();
    tokio::time::timeout(BUDGET, second.finish(Ok(()))).await??;
    assert!(first_scope.wait_until(Instant::now()).await.is_complete());
    assert_eq!(second_scope.wait_until(Instant::now()).await.completed, 1);
    Ok(())
}

#[tokio::test]
async fn aborted_runtime_guard_closes_admission_but_keeps_admitted_body_owned() -> anyhow::Result<()>
{
    let public_guard = CuratedCallbackGuard::new();
    let runtime_guard = public_guard.fork();
    let scope = public_guard.scope();
    let entered = Arc::new(Semaphore::new(/*permits*/ 0));
    let release = Arc::new(Semaphore::new(/*permits*/ 0));
    let steps = Arc::new(AtomicUsize::new(/*v*/ 0));
    let callback = scope.callback({
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        let steps = Arc::clone(&steps);
        move || {
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            let steps = Arc::clone(&steps);
            async move {
                steps.fetch_add(/*val*/ 1, Ordering::SeqCst);
                entered.add_permits(/*n*/ 1);
                if let Ok(permit) = release.acquire().await {
                    permit.forget();
                }
                steps.fetch_add(/*val*/ 1, Ordering::SeqCst);
            }
        }
    });
    assert!(callback.dispatch());
    tokio::time::timeout(BUDGET, entered.acquire())
        .await??
        .forget();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let runtime = tokio::spawn(async move {
        let _guard = runtime_guard;
        let _ = ready_tx.send(());
        std::future::pending::<()>().await;
    });
    tokio::time::timeout(BUDGET, ready_rx).await??;
    runtime.abort();
    assert!(
        runtime
            .await
            .err()
            .context("aborted runtime completed")?
            .is_cancelled()
    );
    assert!(!callback.dispatch());
    assert_eq!(scope.wait_until(Instant::now()).await.pending, 1);
    assert_eq!(steps.load(Ordering::SeqCst), 1);
    release.add_permits(/*n*/ 1);
    tokio::time::timeout(BUDGET, public_guard.finish(Ok(()))).await??;
    assert_eq!(steps.load(Ordering::SeqCst), 2);
    Ok(())
}

#[tokio::test]
async fn primary_failure_and_callback_panic_both_reach_the_caller() -> anyhow::Result<()> {
    let guard = CuratedCallbackGuard::new();
    let scope = guard.scope();
    let entered = Arc::new(Semaphore::new(/*permits*/ 0));
    let callback = scope.callback({
        let entered = Arc::clone(&entered);
        move || {
            let entered = Arc::clone(&entered);
            async move {
                entered.add_permits(/*n*/ 1);
                panic!("fixture callback panic");
            }
        }
    });
    assert!(callback.dispatch());
    tokio::time::timeout(BUDGET, entered.acquire())
        .await??
        .forget();
    let error = tokio::time::timeout(
        BUDGET,
        guard.finish::<()>(Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "fixture startup denied",
        ))),
    )
    .await?
    .err()
    .context("callback failure became success")?;
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(error.to_string().contains("fixture startup denied"));
    assert!(
        error
            .to_string()
            .contains("curated callback cleanup unconfirmed")
    );
    assert_eq!(scope.wait_until(Instant::now()).await.failed, 1);
    Ok(())
}
