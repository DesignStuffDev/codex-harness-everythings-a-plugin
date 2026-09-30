use super::*;
use crate::local::rollout_migration::MigrationControl;
use crate::local::rollout_migration::startup::run_startup_migration;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn cooperative_startup_wakes_from_maintenance_contention_without_advancing_cursor() {
    let home = TempDir::new().expect("create Codex home");
    let thread_id = ThreadId::new();
    let path = write_rollout(home.path(), thread_id, ThreadHistoryMode::Legacy);
    let store = indexed_store(home.path()).await;
    let original = fs::read(&path).unwrap();
    let _lock = codex_rollout::try_acquire_rollout_maintenance_lock(home.path())
        .unwrap()
        .unwrap();
    let (stop, cancellation) = tokio::sync::watch::channel(false);
    let mut migration = Box::pin(run_startup_migration(
        &store,
        vec![path.clone()],
        MigrationControl::Cooperative(&cancellation),
    ));
    assert!(futures::poll!(&mut migration).is_pending());
    stop.send_replace(true);
    let report = tokio::time::timeout(Duration::from_millis(100), migration)
        .await
        .unwrap()
        .unwrap();
    assert!(report.outcomes.is_empty());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(
        store
            .state_db
            .as_ref()
            .unwrap()
            .get_rollout_migration_state(super::super::LEGACY_TO_PAGINATED_MIGRATION_ID)
            .await
            .unwrap()
            .is_none()
    );
}
