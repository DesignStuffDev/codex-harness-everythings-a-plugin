//! Real installed-worker shutdown while native maintenance owns durable work.

use std::sync::Arc;
use std::time::Duration;

use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_state::SqliteConfig;
use codex_thread_store::AppendThreadItemsParams;
use codex_thread_store::ListTurnsParams;
use codex_thread_store::PersistContext;
use codex_thread_store::RolloutMaintenance;
use codex_thread_store::SortDirection;
use codex_thread_store::StoredTurnItemsView;
use codex_thread_store::ThreadStore;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;

use crate::support::Fixture;
use crate::support::turn;

#[tokio::test]
async fn installed_store_close_waits_for_current_migration_before_acknowledging_shutdown() {
    let fixture = Fixture::install();
    let native = fixture.local().await;
    let id = ThreadId::new();
    native
        .create_thread(fixture.params(id, ThreadHistoryMode::Legacy))
        .await
        .unwrap();
    native
        .append_items(AppendThreadItemsParams {
            thread_id: id,
            items: turn(id, "migration-drain"),
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
    // Engine metadata observation normally indexes a newly written legacy
    // rollout. Reproduce that native step so migration has its required row.
    native
        .update_thread_metadata(codex_thread_store::UpdateThreadMetadataParams {
            thread_id: id,
            patch: codex_thread_store::ThreadMetadataPatch {
                rollout_path: Some(path.clone()),
                ..Default::default()
            },
            include_archived: false,
        })
        .await
        .unwrap();
    let state = native.state_db_handle().unwrap();
    assert!(state.get_thread(id).await.unwrap().is_some());
    let home = &fixture.initialization.paths.codex_home;
    let writer_lock = home.join("thread-writer-locks").join(format!("{id}.lock"));
    assert!(
        !writer_lock.exists(),
        "source writer must be closed before migration"
    );

    // Hold the real history DB's write lock. Once migration takes the source's
    // writer lock, it is inside a single path and blocked at history deletion.
    // This establishes an owned mutation without relying on a large-file race
    // or a production-only test hook.
    let sqlite = SqliteConfig::from_sqlite_home(
        AbsolutePathBuf::from_absolute_path(&fixture.initialization.paths.sqlite_home).unwrap(),
    );
    let history = codex_state::open_thread_history_db(&sqlite).await.unwrap();
    let transaction = history.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let process = fixture.open().await;
    process
        .run_rollout_maintenance(RolloutMaintenance::MigrateOnStartup)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !writer_lock.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("native migration must own the source writer before shutdown");

    let closing_store = Arc::clone(&process);
    let mut closing = tokio::spawn(async move { closing_store.shutdown_process().await });
    let early_close = tokio::time::timeout(Duration::from_millis(150), &mut closing).await;
    // Release the real DB lock even when the assertion exposes a lifecycle bug.
    transaction.rollback().await.unwrap();
    assert!(
        early_close.is_err(),
        "successful close must wait for the current migration path"
    );
    tokio::time::timeout(Duration::from_secs(10), closing)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert!(
        !writer_lock.exists(),
        "successful close releases mutation ownership"
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
    assert_eq!(
        codex_rollout::read_session_meta_line(&path)
            .await
            .unwrap()
            .meta
            .history_mode,
        ThreadHistoryMode::Paginated
    );
    let turns = native
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
    assert_eq!(turns.turns.len(), 1);
    assert_eq!(turns.turns[0].turn_id, "migration-drain");
    assert!(
        state
            .get_rollout_migration_state("legacy_to_paginated_v1")
            .await
            .unwrap()
            .is_none(),
        "a stopped startup pass must not advance its discovery cursor"
    );
    history.close().await;
}
