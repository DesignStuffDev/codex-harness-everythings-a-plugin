use super::*;
use crate::local::rollout_migration::MigrationControl;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn cooperative_migration_stops_between_paths_after_publishing_current_path() {
    let home = TempDir::new().expect("create Codex home");
    let first_id = ThreadId::new();
    let second_id = ThreadId::new();
    let first = write_rollout(
        home.path(),
        first_id,
        SessionSource::Cli,
        vec![user_message("first")],
    );
    let second = write_rollout(
        home.path(),
        second_id,
        SessionSource::Cli,
        vec![user_message("second")],
    );
    let original_second = fs::read(&second).unwrap();
    let store = indexed_store(home.path()).await;
    let (stop, cancellation) = tokio::sync::watch::channel(false);
    let report = store
        .migrate_rollouts_with_progress_for_trigger(
            apply_options(),
            |progress| {
                assert_eq!(progress.processed_paths, 1);
                assert_eq!(
                    progress.outcome_status,
                    Some(RolloutMigrationStatus::Migrated)
                );
                stop.send_replace(true);
            },
            RolloutMigrationTrigger::Startup,
            RolloutMigrationPaths::Known(vec![first.clone(), second.clone()]),
            MigrationControl::Cooperative(&cancellation),
        )
        .await
        .unwrap();
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].thread_id, Some(first_id));
    assert!(matches!(&read_rollout(&first)[0].item,
        RolloutItem::SessionMeta(metadata) if metadata.meta.history_mode == ThreadHistoryMode::Paginated));
    assert!(!migration_journal_path(home.path(), first_id).exists());
    assert_eq!(fs::read(&second).unwrap(), original_second);

    // The remaining legacy path is resumable on a later native startup.
    store.migrate_rollouts_on_startup().await.unwrap();
    assert!(matches!(&read_rollout(&second)[0].item,
        RolloutItem::SessionMeta(metadata) if metadata.meta.history_mode == ThreadHistoryMode::Paginated));
}
