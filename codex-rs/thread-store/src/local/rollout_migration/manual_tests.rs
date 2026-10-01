use std::time::Duration;

use pretty_assertions::assert_eq;

use super::*;
use crate::RolloutMigrationCounts;
use crate::RolloutMigrationMode;
use crate::RolloutMigrationStatus;
use crate::ThreadStore;
use crate::local::test_support::test_config;
use crate::local::test_support::write_session_file;

async fn finished(run: &dyn RolloutMigrationRun) -> RolloutMigrationSnapshot {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = run.snapshot().await.unwrap();
            if matches!(
                snapshot.phase,
                RolloutMigrationPhase::Completed | RolloutMigrationPhase::Cancelled
            ) {
                return snapshot;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn dry_run_preserves_storage_and_coalesced_progress_matches_native_report() {
    let home = tempfile::TempDir::new().unwrap();
    let mut originals = Vec::new();
    for _ in 0..4 {
        let path =
            write_session_file(home.path(), "2025-01-03T00-00-00", uuid::Uuid::new_v4()).unwrap();
        originals.push((path.clone(), std::fs::read(path).unwrap()));
    }
    let config = test_config(home.path());
    let store = LocalThreadStore::new(config.clone(), /*state_db*/ None);
    let expected = store
        .migrate_rollouts(RolloutMigrationOptions::default())
        .await
        .unwrap();
    let run = store
        .start_rollout_migration(RolloutMigrationOptions::default())
        .await
        .unwrap();
    // No yield occurred since admission; 0/unknown is a live scanning phase.
    assert_eq!(
        run.snapshot().await.unwrap(),
        RolloutMigrationSnapshot::default()
    );
    assert!(matches!(
        run.report().await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    let snapshot = finished(run.as_ref()).await;
    assert_eq!(snapshot.phase, RolloutMigrationPhase::Completed);
    assert_eq!(
        (snapshot.processed_paths, snapshot.total_paths),
        (4, Some(4))
    );
    assert_eq!(
        snapshot.counts,
        RolloutMigrationCounts {
            eligible: 4,
            ..Default::default()
        }
    );
    assert_eq!(run.report().await.unwrap(), expected);
    assert_eq!(
        run.report().await.unwrap(),
        expected,
        "completed reports remain observable until release"
    );
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
        let result = store
            .start_rollout_migration(RolloutMigrationOptions {
                max_mib_per_second: Some(rate),
                ..Default::default()
            })
            .await;
        assert!(matches!(
            result,
            Err(ThreadStoreError::InvalidRequest { .. })
        ));
    }
    let run = store
        .start_rollout_migration(RolloutMigrationOptions::default())
        .await
        .unwrap();
    let snapshot = finished(run.as_ref()).await;
    assert_eq!(snapshot.phase, RolloutMigrationPhase::Completed);
    assert_eq!(
        (snapshot.processed_paths, snapshot.total_paths),
        (0, Some(0))
    );
    assert!(
        snapshot.revision >= 2,
        "discovery and completion are separate transitions"
    );
    assert_eq!(
        run.report().await.unwrap(),
        RolloutMigrationReport::default()
    );
    run.close().await.unwrap();
    store.shutdown_store().await.unwrap();
}

#[tokio::test]
async fn busy_operation_reports_error_but_release_joins_and_preserves_healthy_store() {
    let home = tempfile::TempDir::new().unwrap();
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let lock = codex_rollout::try_acquire_rollout_maintenance_lock(home.path())
        .unwrap()
        .unwrap();
    let run = store
        .start_rollout_migration(RolloutMigrationOptions {
            mode: RolloutMigrationMode::Apply,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        finished(run.as_ref()).await.phase,
        RolloutMigrationPhase::Completed
    );
    assert!(matches!(
        run.report().await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    run.close().await.unwrap();
    drop(lock);
    let run = store
        .start_rollout_migration(RolloutMigrationOptions::default())
        .await
        .unwrap();
    finished(run.as_ref()).await;
    assert_eq!(
        run.report().await.unwrap(),
        RolloutMigrationReport::default()
    );
    run.close().await.unwrap();
    store.shutdown_store().await.unwrap();
}

struct BlockedMigration {
    _home: tempfile::TempDir,
    thread_id: codex_protocol::ThreadId,
    path: std::path::PathBuf,
    writer: std::path::PathBuf,
    journal: std::path::PathBuf,
    state: codex_rollout::StateDbHandle,
    state_pool: sqlx::SqlitePool,
    transaction: sqlx::Transaction<'static, sqlx::Sqlite>,
    store: LocalThreadStore,
    run: Box<dyn RolloutMigrationRun>,
}

async fn blocked_migration() -> BlockedMigration {
    let home = tempfile::TempDir::new().unwrap();
    let id = uuid::Uuid::new_v4();
    let thread_id = codex_protocol::ThreadId::from_string(&id.to_string()).unwrap();
    let path = write_session_file(home.path(), "2025-01-03T00-00-00", id).unwrap();
    // The discovery helper uses a filename-style timestamp in its JSON too.
    // Full conversion needs a valid native rollout, not merely a readable header.
    let timestamp = "2025-01-03T00:00:00Z";
    let line = codex_rollout::RolloutLine {
        timestamp: timestamp.to_owned(),
        ordinal: None,
        item: codex_rollout::RolloutItem::SessionMeta(codex_protocol::protocol::SessionMetaLine {
            meta: codex_protocol::protocol::SessionMeta {
                session_id: thread_id.into(),
                id: thread_id,
                timestamp: timestamp.to_owned(),
                cwd: home.path().to_path_buf(),
                originator: "test-originator".to_owned(),
                cli_version: "0.0.0".to_owned(),
                source: codex_protocol::protocol::SessionSource::Cli,
                model_provider: Some("test-provider".to_owned()),
                ..Default::default()
            },
            git: None,
        }),
    };
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string(&line).unwrap()),
    )
    .unwrap();
    let config = test_config(home.path());
    let state = codex_rollout::state_db::try_init(&codex_rollout::RolloutConfig {
        codex_home: config.codex_home.clone(),
        sqlite: config.sqlite.clone(),
        cwd: home.path().to_path_buf(),
        model_provider_id: config.default_model_provider_id.clone(),
        generate_memories: false,
    })
    .await
    .unwrap();
    assert!(
        state.get_thread(thread_id).await.unwrap().is_some(),
        "valid rollout metadata was backfilled"
    );
    // Block the final metadata publication, allowing projection and rollout rename
    // to finish first. A writer lock alone could also denote failed-path cleanup.
    let state_pool = config
        .sqlite
        .open_read_write_pool(&config.sqlite.state_db_path())
        .await
        .unwrap();
    let transaction = state_pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let store = LocalThreadStore::new(config, Some(Arc::clone(&state)));
    let run = store
        .start_rollout_migration(RolloutMigrationOptions {
            mode: RolloutMigrationMode::Apply,
            ..Default::default()
        })
        .await
        .unwrap();
    let writer = home
        .path()
        .join("thread-writer-locks")
        .join(format!("{id}.lock"));
    let journal = home
        .path()
        .join("rollout-migrations")
        .join(format!("{id}.pending"));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let metadata = codex_rollout::read_session_meta_line(&path).await.unwrap();
            if metadata.meta.history_mode == codex_protocol::protocol::ThreadHistoryMode::Paginated
                && journal.exists()
            {
                break;
            }
            let snapshot = run.snapshot().await.unwrap();
            assert!(
                !matches!(
                    snapshot.phase,
                    RolloutMigrationPhase::Completed | RolloutMigrationPhase::Cancelled
                ),
                "migration settled before publication was blocked: {:?}",
                run.report().await
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(writer.exists());
    BlockedMigration {
        _home: home,
        thread_id,
        path,
        writer,
        journal,
        state,
        state_pool,
        transaction,
        store,
        run,
    }
}

#[tokio::test]
async fn cancelling_manual_run_joins_current_path_without_setting_startup_cursor() {
    let blocked = blocked_migration().await;
    blocked.run.cancel().await.unwrap();
    let mut close = tokio::spawn(async move { blocked.run.close().await });
    let early_close = tokio::time::timeout(Duration::from_millis(150), &mut close).await;
    blocked.transaction.rollback().await.unwrap();
    assert!(
        early_close.is_err(),
        "cancel acknowledges stop, not cleanup completion"
    );
    tokio::time::timeout(Duration::from_secs(10), close)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!blocked.writer.exists());
    assert_eq!(
        codex_rollout::read_session_meta_line(&blocked.path)
            .await
            .unwrap()
            .meta
            .history_mode,
        codex_protocol::protocol::ThreadHistoryMode::Paginated
    );
    assert_eq!(
        blocked
            .state
            .get_thread(blocked.thread_id)
            .await
            .unwrap()
            .unwrap()
            .history_mode,
        codex_protocol::protocol::ThreadHistoryMode::Paginated
    );
    assert!(!blocked.journal.exists());
    assert!(
        blocked
            .state
            .get_rollout_migration_state("legacy_to_paginated_v1")
            .await
            .unwrap()
            .is_none()
    );
    blocked.store.shutdown_store().await.unwrap();
    blocked.state_pool.close().await;
    blocked.state.close().await;
}

#[tokio::test]
async fn abandoned_close_keeps_publication_owned_until_store_shutdown_joins() {
    let blocked = blocked_migration().await;
    // Dropping an unpolled close future releases the observer, not the backend job.
    drop(blocked.run.close());
    let store = blocked.store.clone();
    let mut shutdown = tokio::spawn(async move { store.shutdown_store().await });
    let early_shutdown = tokio::time::timeout(Duration::from_millis(150), &mut shutdown).await;
    blocked.transaction.rollback().await.unwrap();
    assert!(
        early_shutdown.is_err(),
        "store close must join the still-blocked publication"
    );
    tokio::time::timeout(Duration::from_secs(10), shutdown)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!blocked.writer.exists());
    assert_eq!(
        codex_rollout::read_session_meta_line(&blocked.path)
            .await
            .unwrap()
            .meta
            .history_mode,
        codex_protocol::protocol::ThreadHistoryMode::Paginated
    );
    assert_eq!(
        blocked
            .state
            .get_thread(blocked.thread_id)
            .await
            .unwrap()
            .unwrap()
            .history_mode,
        codex_protocol::protocol::ThreadHistoryMode::Paginated
    );
    assert!(!blocked.journal.exists());
    assert!(matches!(
        blocked
            .store
            .start_rollout_migration(RolloutMigrationOptions::default())
            .await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    blocked.state_pool.close().await;
    blocked.state.close().await;
}

#[tokio::test]
async fn cancelled_before_worker_admission_retains_empty_prefix_without_mutation() {
    let home = tempfile::TempDir::new().unwrap();
    let path =
        write_session_file(home.path(), "2025-01-03T00-00-00", uuid::Uuid::new_v4()).unwrap();
    let original = std::fs::read(&path).unwrap();
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let run = store
        .start_rollout_migration(RolloutMigrationOptions {
            mode: RolloutMigrationMode::Apply,
            ..Default::default()
        })
        .await
        .unwrap();
    run.cancel().await.unwrap();
    run.cancel().await.unwrap();
    let snapshot = finished(run.as_ref()).await;
    assert_eq!(snapshot.phase, RolloutMigrationPhase::Cancelled);
    assert_eq!(snapshot.processed_paths, 0);
    assert_eq!(snapshot.counts, RolloutMigrationCounts::default());
    assert_eq!(
        run.report().await.unwrap(),
        RolloutMigrationReport::default()
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
    run.close().await.unwrap();
    store.shutdown_store().await.unwrap();
}

#[tokio::test]
async fn per_path_failure_and_filter_preserve_native_report_and_counts() {
    let home = tempfile::TempDir::new().unwrap();
    let good_id = uuid::Uuid::new_v4();
    let bad_id = uuid::Uuid::new_v4();
    write_session_file(home.path(), "2025-01-03T00-00-00", good_id).unwrap();
    let bad_path = write_session_file(home.path(), "2025-01-03T00-00-00", bad_id).unwrap();
    std::fs::write(&bad_path, b"invalid rollout metadata\n").unwrap();
    write_session_file(home.path(), "2025-01-03T00-00-00", uuid::Uuid::new_v4()).unwrap();
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let options = RolloutMigrationOptions {
        thread_ids: vec![
            codex_protocol::ThreadId::from_string(&good_id.to_string()).unwrap(),
            codex_protocol::ThreadId::from_string(&bad_id.to_string()).unwrap(),
        ],
        ..Default::default()
    };
    let expected = store.migrate_rollouts(options.clone()).await.unwrap();
    let run = store.start_rollout_migration(options).await.unwrap();
    let snapshot = finished(run.as_ref()).await;
    assert_eq!(snapshot.phase, RolloutMigrationPhase::Completed);
    assert_eq!(snapshot.processed_paths, 3);
    assert_eq!(
        snapshot.counts,
        RolloutMigrationCounts {
            eligible: 1,
            failed: 1,
            ..Default::default()
        }
    );
    assert_eq!(run.report().await.unwrap(), expected);
    assert_eq!(expected.outcomes.len(), 2);
    assert!(
        expected
            .outcomes
            .iter()
            .any(|outcome| outcome.rollout_path == bad_path
                && outcome.status == RolloutMigrationStatus::Failed
                && outcome.failure_reason.is_some())
    );
    run.close().await.unwrap();
    store.shutdown_store().await.unwrap();
}
