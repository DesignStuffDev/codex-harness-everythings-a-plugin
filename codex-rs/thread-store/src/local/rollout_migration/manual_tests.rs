use std::time::Duration;

use pretty_assertions::assert_eq;

use super::*;
use crate::RolloutMigrationCounts;
use crate::RolloutMigrationMode;
use crate::ThreadStore;
use crate::local::test_support::test_config;
use crate::local::test_support::write_session_file;

async fn finished(run: &dyn RolloutMigrationRun) -> RolloutMigrationSnapshot {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = run.snapshot().await.unwrap();
            if matches!(snapshot.phase, RolloutMigrationPhase::Completed | RolloutMigrationPhase::Cancelled) {
                return snapshot;
            }
            tokio::task::yield_now().await;
        }
    }).await.unwrap()
}

#[tokio::test]
async fn dry_run_preserves_storage_and_coalesced_progress_matches_native_report() {
    let home = tempfile::TempDir::new().unwrap();
    let mut originals = Vec::new();
    for _ in 0..4 {
        let path = write_session_file(home.path(), "2025-01-03T00-00-00", uuid::Uuid::new_v4()).unwrap();
        originals.push((path.clone(), std::fs::read(path).unwrap()));
    }
    let config = test_config(home.path());
    let store = LocalThreadStore::new(config.clone(), /*state_db*/ None);
    let expected = store.migrate_rollouts(RolloutMigrationOptions::default()).await.unwrap();
    let run = store.start_rollout_migration(RolloutMigrationOptions::default()).await.unwrap();
    // No yield occurred since admission; 0/unknown is a live scanning phase.
    assert_eq!(run.snapshot().await.unwrap(), RolloutMigrationSnapshot::default());
    assert!(matches!(run.report().await, Err(ThreadStoreError::Conflict { .. })));
    let snapshot = finished(run.as_ref()).await;
    assert_eq!(snapshot.phase, RolloutMigrationPhase::Completed);
    assert_eq!((snapshot.processed_paths, snapshot.total_paths), (4, Some(4)));
    assert_eq!(snapshot.counts, RolloutMigrationCounts { eligible: 4, ..Default::default() });
    assert_eq!(run.report().await.unwrap(), expected);
    assert_eq!(run.report().await.unwrap(), expected, "completed reports remain observable until release");
    run.close().await.unwrap();
    store.shutdown_store().await.unwrap();
    for (path, bytes) in originals {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    assert!(!config.sqlite.state_db_path().exists());
    assert!(!config.sqlite.thread_history_db_path().exists());
}

#[tokio::test]
async fn zero_path_run_finishes_explicitly_and_invalid_limits_do_not_poison_admission() {
    let home = tempfile::TempDir::new().unwrap();
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    for rate in [0, u64::MAX] {
        let result = store.start_rollout_migration(RolloutMigrationOptions {
            max_mib_per_second: Some(rate), ..Default::default()
        }).await;
        assert!(matches!(result, Err(ThreadStoreError::InvalidRequest { .. })));
    }
    let run = store.start_rollout_migration(RolloutMigrationOptions::default()).await.unwrap();
    let snapshot = finished(run.as_ref()).await;
    assert_eq!(snapshot.phase, RolloutMigrationPhase::Completed);
    assert_eq!((snapshot.processed_paths, snapshot.total_paths), (0, Some(0)));
    assert!(snapshot.revision >= 2, "discovery and completion are separate transitions");
    assert_eq!(run.report().await.unwrap(), RolloutMigrationReport::default());
    run.close().await.unwrap();
    store.shutdown_store().await.unwrap();
}

#[tokio::test]
async fn busy_operation_reports_error_but_release_joins_and_preserves_healthy_store() {
    let home = tempfile::TempDir::new().unwrap();
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let lock = codex_rollout::try_acquire_rollout_maintenance_lock(home.path()).unwrap().unwrap();
    let run = store.start_rollout_migration(RolloutMigrationOptions {
        mode: RolloutMigrationMode::Apply, ..Default::default()
    }).await.unwrap();
    assert_eq!(finished(run.as_ref()).await.phase, RolloutMigrationPhase::Completed);
    assert!(matches!(run.report().await, Err(ThreadStoreError::Conflict { .. })));
    run.close().await.unwrap();
    drop(lock);
    let run = store.start_rollout_migration(RolloutMigrationOptions::default()).await.unwrap();
    finished(run.as_ref()).await;
    assert_eq!(run.report().await.unwrap(), RolloutMigrationReport::default());
    run.close().await.unwrap();
    store.shutdown_store().await.unwrap();
}

#[tokio::test]
async fn cancelling_manual_run_joins_current_path_without_setting_startup_cursor() {
    let home = tempfile::TempDir::new().unwrap();
    let id = uuid::Uuid::new_v4();
    let path = write_session_file(home.path(), "2025-01-03T00-00-00", id).unwrap();
    let config = test_config(home.path());
    let state = codex_rollout::state_db::try_init(&codex_rollout::RolloutConfig {
        codex_home: config.codex_home.clone(), sqlite: config.sqlite.clone(),
        cwd: home.path().to_path_buf(), model_provider_id: config.default_model_provider_id.clone(),
        generate_memories: false,
    }).await.unwrap();
    let history = codex_state::open_thread_history_db(&config.sqlite).await.unwrap();
    let transaction = history.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let store = LocalThreadStore::new(config, Some(Arc::clone(&state)));
    let run = store.start_rollout_migration(RolloutMigrationOptions {
        mode: RolloutMigrationMode::Apply, ..Default::default()
    }).await.unwrap();
    let writer = home.path().join("thread-writer-locks").join(format!("{id}.lock"));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !writer.exists() { tokio::time::sleep(Duration::from_millis(5)).await; }
    }).await.unwrap();
    run.cancel().await.unwrap();
    let mut close = tokio::spawn(async move { run.close().await });
    let early_close = tokio::time::timeout(Duration::from_millis(150), &mut close).await;
    transaction.rollback().await.unwrap();
    assert!(early_close.is_err(), "cancel acknowledges stop, not cleanup completion");
    tokio::time::timeout(Duration::from_secs(10), close).await.unwrap().unwrap().unwrap();
    assert!(!writer.exists());
    assert_eq!(codex_rollout::read_session_meta_line(&path).await.unwrap().meta.history_mode,
        codex_protocol::protocol::ThreadHistoryMode::Paginated);
    assert!(state.get_rollout_migration_state("legacy_to_paginated_v1").await.unwrap().is_none());
    store.shutdown_store().await.unwrap();
    history.close().await;
    state.close().await;
}
