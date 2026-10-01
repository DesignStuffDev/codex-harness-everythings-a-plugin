//! Manual migration through an installed native worker, including owned cleanup.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_state::SqliteConfig;
use codex_thread_store::AppendThreadItemsParams;
use codex_thread_store::ListTurnsParams;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::PersistContext;
use codex_thread_store::RolloutMigrationCounts;
use codex_thread_store::RolloutMigrationMode;
use codex_thread_store::RolloutMigrationOptions;
use codex_thread_store::RolloutMigrationPhase;
use codex_thread_store::RolloutMigrationReport;
use codex_thread_store::RolloutMigrationRun;
use codex_thread_store::RolloutMigrationSnapshot;
use codex_thread_store::RolloutMigrationStatus;
use codex_thread_store::SortDirection;
use codex_thread_store::StoredTurnItemsView;
use codex_thread_store::ThreadMetadataPatch;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use codex_thread_store::UpdateThreadMetadataParams;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;

use crate::support::Fixture;
use crate::support::turn;

async fn legacy_rollout(
    fixture: &Fixture,
    native: &LocalThreadStore,
    turn_id: &str,
) -> (ThreadId, PathBuf) {
    let id = ThreadId::new();
    native
        .create_thread(fixture.params(id, ThreadHistoryMode::Legacy))
        .await
        .unwrap();
    native
        .append_items(AppendThreadItemsParams {
            thread_id: id,
            items: turn(id, turn_id),
        })
        .await
        .unwrap();
    native
        .persist_thread(id, PersistContext::Standard)
        .await
        .unwrap();
    native.flush_thread(id).await.unwrap();
    let path = native.local_rollout_path(id).await.unwrap().unwrap();
    native.shutdown_thread(id).await.unwrap();
    // Match the engine's indexing of a durable legacy rollout before migrating.
    native
        .update_thread_metadata(UpdateThreadMetadataParams {
            thread_id: id,
            patch: ThreadMetadataPatch {
                rollout_path: Some(path.clone()),
                ..Default::default()
            },
            include_archived: false,
        })
        .await
        .unwrap();
    (id, path)
}

async fn finished(run: &dyn RolloutMigrationRun) -> RolloutMigrationSnapshot {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = run.snapshot().await.unwrap();
            if matches!(
                snapshot.phase,
                RolloutMigrationPhase::Completed | RolloutMigrationPhase::Cancelled
            ) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("migration must reach an explicit terminal phase")
}

async fn assert_paginated_turn(store: &dyn ThreadStore, id: ThreadId, turn_id: &str) {
    let page = store
        .list_turns(ListTurnsParams {
            thread_id: id,
            include_archived: false,
            cursor: None,
            page_size: 10,
            sort_direction: SortDirection::Asc,
            items_view: StoredTurnItemsView::Summary,
        })
        .await
        .unwrap();
    assert_eq!(
        page.turns
            .iter()
            .map(|turn| turn.turn_id.as_str())
            .collect::<Vec<_>>(),
        vec![turn_id]
    );
}

fn assert_cleanup(home: &Path, id: ThreadId, path: &Path) {
    assert!(
        !home
            .join("thread-writer-locks")
            .join(format!("{id}.lock"))
            .exists()
    );
    assert!(
        !home
            .join("rollout-migrations")
            .join(format!("{id}.pending"))
            .exists()
    );
    let filename = path.file_name().unwrap().to_str().unwrap();
    assert!(
        !path
            .with_file_name(format!(".{filename}.paginated.tmp"))
            .exists()
    );
}

#[tokio::test]
async fn installed_manual_migration_matches_native_dry_run_and_survives_restart() {
    let fixture = Fixture::install();
    let native = fixture.local().await;
    let (selected, selected_path) = legacy_rollout(&fixture, &native, "selected-turn").await;
    let (_, untouched_path) = legacy_rollout(&fixture, &native, "untouched-turn").await;
    let originals = [&selected_path, &untouched_path].map(|path| std::fs::read(path).unwrap());
    let options = RolloutMigrationOptions {
        thread_ids: vec![selected],
        ..Default::default()
    };
    let expected = native.migrate_rollouts(options.clone()).await.unwrap();
    let state = native.state_db_handle().unwrap();
    native.shutdown_store().await.unwrap();
    state.close().await;

    let process = fixture.open().await;
    assert!(process.supports_manual_rollout_migration());
    assert_eq!(process.capabilities().manual_rollout_migration, Some(1));
    let dry_run = process
        .start_rollout_migration(options.clone())
        .await
        .unwrap();
    let snapshot = finished(dry_run.as_ref()).await;
    assert_eq!(
        (
            snapshot.phase,
            snapshot.processed_paths,
            snapshot.total_paths,
            snapshot.counts
        ),
        (
            RolloutMigrationPhase::Completed,
            2,
            Some(2),
            RolloutMigrationCounts {
                eligible: 1,
                ..Default::default()
            },
        )
    );
    assert_eq!(dry_run.report().await.unwrap(), expected);
    assert_eq!(
        dry_run.report().await.unwrap(),
        expected,
        "report remains available until close"
    );
    dry_run.close().await.unwrap();
    assert_eq!(
        [&selected_path, &untouched_path].map(|path| std::fs::read(path).unwrap()),
        originals
    );

    let applying = process
        .start_rollout_migration(RolloutMigrationOptions {
            mode: RolloutMigrationMode::Apply,
            ..options.clone()
        })
        .await
        .unwrap();
    let snapshot = finished(applying.as_ref()).await;
    assert_eq!(
        (snapshot.phase, snapshot.counts),
        (
            RolloutMigrationPhase::Completed,
            RolloutMigrationCounts {
                migrated: 1,
                ..Default::default()
            }
        )
    );
    let report = applying.report().await.unwrap();
    assert_eq!(report.outcomes.len(), 1);
    let outcome = &report.outcomes[0];
    assert_eq!(
        (
            outcome.thread_id,
            &outcome.rollout_path,
            outcome.status,
            outcome.failure_reason,
            &outcome.message
        ),
        (
            Some(selected),
            &selected_path,
            RolloutMigrationStatus::Migrated,
            None,
            &None
        )
    );
    assert!(outcome.bytes_processed > 0);
    applying.close().await.unwrap();
    assert_eq!(std::fs::read(&untouched_path).unwrap(), originals[1]);
    process.shutdown_process().await.unwrap();

    let restarted = fixture.open().await;
    assert_paginated_turn(restarted.as_ref(), selected, "selected-turn").await;
    assert_cleanup(
        &fixture.initialization.paths.codex_home,
        selected,
        &selected_path,
    );
    let repeated = restarted.start_rollout_migration(options).await.unwrap();
    assert_eq!(
        finished(repeated.as_ref()).await.counts,
        RolloutMigrationCounts {
            already_paginated: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        repeated.report().await.unwrap().outcomes[0].status,
        RolloutMigrationStatus::AlreadyPaginated
    );
    repeated.close().await.unwrap();
    restarted.shutdown_process().await.unwrap();
}

#[tokio::test]
async fn rejected_manual_runs_release_cleanup_capacity_and_empty_run_finishes() {
    let fixture = Fixture::install();
    let process = fixture.open().await;
    tokio::time::timeout(Duration::from_secs(20), async {
        // Exceed the cleanup lane capacity: rejected starts must release their
        // reservation even though no native migration handle was returned.
        for rate in [0, u64::MAX].into_iter().cycle().take(64) {
            let error = process
                .start_rollout_migration(RolloutMigrationOptions {
                    max_mib_per_second: Some(rate),
                    ..Default::default()
                })
                .await
                .err()
                .expect("invalid migration rate must fail admission");
            assert!(matches!(error, ThreadStoreError::InvalidRequest { .. }));
        }
    })
    .await
    .expect("rejected starts must not exhaust reserved cleanup slots");
    let run = process
        .start_rollout_migration(RolloutMigrationOptions::default())
        .await
        .unwrap();
    let snapshot = finished(run.as_ref()).await;
    assert_eq!(
        (
            snapshot.phase,
            snapshot.processed_paths,
            snapshot.total_paths,
            snapshot.counts
        ),
        (
            RolloutMigrationPhase::Completed,
            0,
            Some(0),
            RolloutMigrationCounts::default()
        )
    );
    assert_eq!(
        run.report().await.unwrap(),
        RolloutMigrationReport::default()
    );
    run.close().await.unwrap();
    process.shutdown_process().await.unwrap();
}

enum Drain {
    CancelAndCloseRun,
    ShutdownStoreWithHeldRun,
}

async fn blocked_native_path_is_drained(action: Drain) {
    let fixture = Fixture::install();
    let native = fixture.local().await;
    let (id, path) = legacy_rollout(&fixture, &native, "manual-drain").await;
    let state = native.state_db_handle().unwrap();
    native.shutdown_store().await.unwrap();
    let process = fixture.open().await;
    let home = &fixture.initialization.paths.codex_home;
    let writer_lock = home.join("thread-writer-locks").join(format!("{id}.lock"));
    let sqlite = SqliteConfig::from_sqlite_home(
        AbsolutePathBuf::from_absolute_path(&fixture.initialization.paths.sqlite_home).unwrap(),
    );
    let history = codex_state::open_thread_history_db(&sqlite).await.unwrap();
    let transaction = history.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let run = process
        .start_rollout_migration(RolloutMigrationOptions {
            mode: RolloutMigrationMode::Apply,
            ..Default::default()
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !writer_lock.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("migration must own the source before testing cleanup");
    assert_eq!(
        run.snapshot().await.unwrap().phase,
        RolloutMigrationPhase::Running
    );
    assert!(matches!(
        run.report().await,
        Err(ThreadStoreError::Conflict { .. })
    ));
    if matches!(&action, Drain::CancelAndCloseRun) {
        run.cancel().await.unwrap();
        assert_eq!(
            run.snapshot().await.unwrap().phase,
            RolloutMigrationPhase::Running,
            "cancel acknowledges a stop request, not completed cleanup"
        );
    }

    let closing_process = Arc::clone(&process);
    let mut closing = tokio::spawn(async move {
        match action {
            Drain::CancelAndCloseRun => run.close().await.map_err(anyhow::Error::from),
            Drain::ShutdownStoreWithHeldRun => {
                let result = closing_process.shutdown_process().await;
                // A still-held observation lease must not keep storage alive.
                assert!(run.snapshot().await.is_err());
                drop(run);
                result
            }
        }
    });
    let early_close = tokio::time::timeout(Duration::from_millis(150), &mut closing).await;
    // Always release the real DB lock before asserting, including failure paths.
    transaction.rollback().await.unwrap();
    assert!(
        early_close.is_err(),
        "cleanup must join the accepted native mutation"
    );
    tokio::time::timeout(Duration::from_secs(10), closing)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_cleanup(home, id, &path);
    assert_eq!(
        codex_rollout::read_session_meta_line(&path)
            .await
            .unwrap()
            .meta
            .history_mode,
        ThreadHistoryMode::Paginated
    );
    assert!(
        state
            .get_rollout_migration_state("legacy_to_paginated_v1")
            .await
            .unwrap()
            .is_none()
    );
    process.shutdown_process().await.unwrap();
    history.close().await;
    state.close().await;
    let restarted = fixture.open().await;
    assert_paginated_turn(restarted.as_ref(), id, "manual-drain").await;
    restarted.shutdown_process().await.unwrap();
}

#[tokio::test]
async fn cancelled_manual_run_close_joins_current_native_path() {
    blocked_native_path_is_drained(Drain::CancelAndCloseRun).await;
}

#[tokio::test]
async fn installed_process_shutdown_joins_manual_migration_with_unreleased_run() {
    blocked_native_path_is_drained(Drain::ShutdownStoreWithHeldRun).await;
}

#[cfg(unix)]
#[tokio::test]
async fn migration_report_preserves_non_utf8_native_paths_across_process_boundary() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::install();
    let native = fixture.local().await;
    let (_, original) = legacy_rollout(&fixture, &native, "raw-path-turn").await;
    let directory = fixture
        .initialization
        .paths
        .codex_home
        .join("sessions")
        .join(OsString::from_vec(b"native-\xff".to_vec()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join(original.file_name().unwrap());
    std::fs::rename(original, &path).unwrap();
    let expected = native
        .migrate_rollouts(RolloutMigrationOptions::default())
        .await
        .unwrap();
    assert_eq!(expected.outcomes.len(), 1);
    assert_eq!(expected.outcomes[0].rollout_path, path);
    let process = fixture.open().await;
    let run = process
        .start_rollout_migration(RolloutMigrationOptions::default())
        .await
        .unwrap();
    assert_eq!(
        finished(run.as_ref()).await.phase,
        RolloutMigrationPhase::Completed
    );
    assert_eq!(run.report().await.unwrap(), expected);
    run.close().await.unwrap();
    process.shutdown_process().await.unwrap();
    native.shutdown_store().await.unwrap();
    native.state_db_handle().unwrap().close().await;
}
