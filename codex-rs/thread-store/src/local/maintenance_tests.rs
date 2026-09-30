use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use pretty_assertions::assert_eq;
use tokio::sync::oneshot;

use super::MaintenanceSupervisor;
use crate::RolloutMaintenance;
use crate::ThreadStore;
use crate::ThreadStoreError;
use crate::local::LocalThreadStore;
use crate::local::test_support::test_config;

#[tokio::test]
async fn maintenance_shutdown_retains_owned_jobs_after_waiter_is_abandoned() {
    let supervisor = Arc::new(MaintenanceSupervisor::default());
    let (entered, started) = oneshot::channel();
    let (release, held) = oneshot::channel();
    supervisor
        .schedule(async move {
            entered.send(()).unwrap();
            held.await.unwrap();
            Ok(())
        })
        .unwrap();
    started.await.unwrap();
    supervisor.begin_shutdown();
    assert!(*supervisor.cancellation.borrow());
    assert!(matches!(
        supervisor.schedule(async { Ok(()) }),
        Err(ThreadStoreError::Conflict { .. })
    ));

    let first = Arc::clone(&supervisor);
    let waiter = tokio::spawn(async move { first.shutdown().await });
    let mut later = Box::pin(supervisor.shutdown());
    assert!(
        tokio::time::timeout(Duration::from_millis(25), &mut later)
            .await
            .is_err()
    );
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    later.await.unwrap();
    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn maintenance_worker_panic_fails_close_only_after_other_jobs_are_joined() {
    let supervisor = MaintenanceSupervisor::default();
    let (entered, started) = oneshot::channel();
    let (release, held) = oneshot::channel();
    supervisor
        .schedule(async move {
            entered.send(()).unwrap();
            panic!("controlled maintenance panic");
        })
        .unwrap();
    supervisor
        .schedule(async move {
            held.await.unwrap();
            Ok(())
        })
        .unwrap();
    started.await.unwrap();
    supervisor.begin_shutdown();
    let mut close = Box::pin(supervisor.shutdown());
    assert!(
        tokio::time::timeout(Duration::from_millis(25), &mut close)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    let error = close.await.unwrap_err().to_string();
    assert!(error.contains("controlled maintenance panic"));
    assert_eq!(supervisor.shutdown().await.unwrap_err().to_string(), error);
}

#[tokio::test]
async fn maintenance_failed_nested_join_is_retained_when_reaping_completed_jobs() {
    let supervisor = MaintenanceSupervisor::default();
    let (finished, observed) = oneshot::channel();
    supervisor
        .schedule(async move {
            finished.send(()).unwrap();
            Err("nested blocking worker join failed".to_owned())
        })
        .unwrap();
    observed.await.unwrap();
    tokio::task::yield_now().await;
    // Admission reaps completed jobs to avoid retaining an unbounded history.
    supervisor.schedule(async { Ok(()) }).unwrap();
    let error = supervisor.shutdown().await.unwrap_err().to_string();
    assert!(error.contains("nested blocking worker join failed"));
    assert_eq!(supervisor.shutdown().await.unwrap_err().to_string(), error);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn maintenance_admission_racing_shutdown_never_loses_an_accepted_job() {
    let supervisor = Arc::new(MaintenanceSupervisor::default());
    let accepted = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let first_completed = Arc::clone(&completed);
    supervisor
        .schedule(async move {
            tokio::task::yield_now().await;
            first_completed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
    accepted.fetch_add(1, Ordering::SeqCst);
    let mut admissions = Vec::new();
    for _ in 0..64 {
        let supervisor = Arc::clone(&supervisor);
        let accepted = Arc::clone(&accepted);
        let completed = Arc::clone(&completed);
        admissions.push(tokio::spawn(async move {
            match supervisor.schedule(async move {
                tokio::task::yield_now().await;
                completed.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }) {
                Ok(()) => {
                    accepted.fetch_add(1, Ordering::SeqCst);
                }
                Err(ThreadStoreError::Conflict { .. }) => {}
                Err(error) => panic!("unexpected admission failure: {error}"),
            }
        }));
    }
    supervisor.begin_shutdown();
    for admission in admissions {
        admission.await.unwrap();
    }
    supervisor.shutdown().await.unwrap();
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        completed.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn local_store_clones_share_maintenance_admission_and_completion() {
    let home = tempfile::TempDir::new().unwrap();
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let clone = store.clone();
    store.begin_shutdown_store();
    assert!(matches!(
        clone
            .run_rollout_maintenance(RolloutMaintenance::Compress)
            .await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    clone.shutdown_store().await.unwrap();
    store.shutdown_store().await.unwrap();
}

#[tokio::test]
async fn local_store_pool_close_survives_an_unpolled_shutdown_waiter() {
    let home = tempfile::TempDir::new().unwrap();
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let pool = codex_state::open_thread_history_db(&store.config.sqlite)
        .await
        .unwrap();
    store.thread_history_db.set(pool.clone()).unwrap();
    let borrowed = pool.acquire().await.unwrap();
    // Constructing the future starts the owned terminal stage immediately.
    drop(store.shutdown_store());
    tokio::time::timeout(Duration::from_secs(1), pool.close_event())
        .await
        .unwrap();
    let mut close = store.shutdown_store();
    assert!(
        tokio::time::timeout(Duration::from_millis(25), &mut close)
            .await
            .is_err()
    );
    drop(borrowed);
    close.await.unwrap();
    assert!(pool.is_closed());
    store.shutdown_store().await.unwrap();
}
