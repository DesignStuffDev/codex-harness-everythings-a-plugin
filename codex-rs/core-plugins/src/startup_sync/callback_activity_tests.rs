use super::super::CallbackRegistry;
use super::*;
use crate::startup_sync::CuratedCallbackObservation;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Semaphore;

const BUDGET: Duration = Duration::from_secs(/*secs*/ 5);

struct ReleaseOnDrop(Arc<Semaphore>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.add_permits(/*n*/ 1);
    }
}

#[tokio::test]
async fn activity_preserves_admission_and_exact_tasks_until_explicit_final_wait()
-> anyhow::Result<()> {
    let registry = Arc::new(CallbackRegistry::default());
    let scope = CuratedCallbackScope::with_registry(tokio::runtime::Handle::current(), &registry);
    let entered = Arc::new(Semaphore::new(/*permits*/ 0));
    let release = Arc::new(Semaphore::new(/*permits*/ 0));
    let release_on_drop = ReleaseOnDrop(Arc::clone(&release));
    let callback = scope.callback({
        let (entered, release) = (Arc::clone(&entered), Arc::clone(&release));
        move || {
            let (entered, release) = (Arc::clone(&entered), Arc::clone(&release));
            async move {
                entered.add_permits(/*n*/ 1);
                match tokio::time::timeout(BUDGET, release.acquire()).await {
                    Ok(Ok(permit)) => permit.forget(),
                    result => panic!("fixture release failed: {result:?}"),
                }
            }
        }
    });
    assert!(callback.dispatch());
    tokio::time::timeout(BUDGET, entered.acquire())
        .await??
        .forget();
    let handle_id = {
        let state = lock(&scope.state);
        match state.records.first().context("registered action")? {
            Record::Running(handle) => handle.id(),
            _ => anyhow::bail!("fixture should own a running handle"),
        }
    };
    let running = CuratedCallbackActivity {
        running: 1,
        ..Default::default()
    };
    assert_eq!(scope.activity(), running);
    assert_eq!(scope.activity(), running);
    assert_eq!(
        {
            let state = lock(&scope.state);
            match state.records.first().context("retained action")? {
                Record::Running(handle) => handle.id(),
                _ => anyhow::bail!("activity changed task custody"),
            }
        },
        handle_id
    );
    assert!(
        scope.callback(|| async {}).dispatch(),
        "read must leave admission open"
    );
    drop(release_on_drop);
    tokio::time::timeout(BUDGET, async {
        while scope.activity().finished_unjoined != 2 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert_eq!(
        scope.activity(),
        CuratedCallbackActivity {
            finished_unjoined: 2,
            ..Default::default()
        }
    );
    assert_eq!(
        scope.wait_until(Instant::now() + BUDGET).await,
        CuratedCallbackObservation {
            pending: 0,
            completed: 2,
            suppressed: 0,
            failed: 0,
        }
    );
    assert_eq!(
        scope.activity(),
        CuratedCallbackActivity {
            admission_closed: true,
            completed: 2,
            ..Default::default()
        }
    );
    Ok(())
}

#[tokio::test]
async fn finished_activity_does_not_turn_task_panic_into_success() -> anyhow::Result<()> {
    let registry = Arc::new(CallbackRegistry::default());
    let scope = CuratedCallbackScope::with_registry(tokio::runtime::Handle::current(), &registry);
    assert!(
        scope
            .callback(|| async { panic!("fixture callback panic") })
            .dispatch()
    );
    tokio::time::timeout(BUDGET, async {
        while scope.activity().finished_unjoined != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert_eq!(
        scope.activity(),
        CuratedCallbackActivity {
            finished_unjoined: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        scope.wait_until(Instant::now() + BUDGET).await,
        CuratedCallbackObservation {
            pending: 0,
            completed: 0,
            suppressed: 0,
            failed: 1,
        }
    );
    assert_eq!(
        scope.activity(),
        CuratedCallbackActivity {
            admission_closed: true,
            failed: 1,
            ..Default::default()
        }
    );
    assert!(!scope.callback(|| async {}).dispatch());
    Ok(())
}
