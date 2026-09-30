//! Exercise the actual native factory's startup ownership with a real SQLite lock.

use std::sync::Arc;
use std::time::Duration;

use codex_features::Feature;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_state::SqliteConfig;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use serde_json::json;

use super::thread_store_from_config;

#[tokio::test]
async fn native_factory_shutdown_joins_startup_migration_current_path() {
    let home = tempfile::TempDir::new().unwrap();
    let mut config = crate::config::test_config().await;
    config.codex_home = AbsolutePathBuf::from_absolute_path(home.path()).unwrap();
    config.cwd = config.codex_home.clone();
    config.sqlite = SqliteConfig::new_for_testing(config.codex_home.clone());
    config.features.enable(Feature::BackgroundPaginatedRolloutMigration);
    config.features.disable(Feature::LocalThreadStoreCompression);
    let id = ThreadId::new();
    let directory = home.path().join("sessions/2025/01/03");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("rollout-2025-01-03T00-00-00-{id}.jsonl"));
    let metadata = json!({
        "timestamp": "2025-01-03T00:00:00Z",
        "type": "session_meta",
        "payload": {
            "session_id": id,
            "id": id,
            "timestamp": "2025-01-03T00:00:00Z",
            "cwd": home.path(),
            "originator": "native_factory_test",
            "cli_version": "test",
            "source": "cli",
            "model_provider": config.model_provider_id,
            "history_mode": "legacy"
        }
    });
    let input = json!({
        "timestamp": "2025-01-03T00:00:01Z",
        "type": "event_msg",
        "payload": { "type": "user_message", "message": "retain this turn", "kind": "plain" }
    });
    std::fs::write(&path, format!("{metadata}\n{input}\n")).unwrap();
    let state = codex_rollout::state_db::try_init(&config).await.unwrap();
    assert!(state.get_thread(id).await.unwrap().is_some());
    let history = codex_state::open_thread_history_db(&config.sqlite).await.unwrap();
    let transaction = history.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let writer_lock = home.path().join("thread-writer-locks").join(format!("{id}.lock"));
    assert!(!writer_lock.exists());

    // Startup is registered by the factory itself. There is no explicit test
    // maintenance request which could conceal a detached-factory regression.
    let store = thread_store_from_config(&config, Some(Arc::clone(&state)));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !writer_lock.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }).await.expect("factory startup migration owns the source path");
    let closing_store = Arc::clone(&store);
    let mut close = tokio::spawn(async move { closing_store.shutdown_store().await });
    let early_close = tokio::time::timeout(Duration::from_millis(150), &mut close).await;
    // Release even if the assertion detects the original detached-worker bug.
    transaction.rollback().await.unwrap();
    assert!(early_close.is_err(), "store close must join factory-started mutation");
    tokio::time::timeout(Duration::from_secs(10), close).await.unwrap().unwrap().unwrap();

    assert!(!writer_lock.exists());
    assert!(!home.path().join("rollout-migrations").join(format!("{id}.pending")).exists());
    assert_eq!(
        codex_rollout::read_session_meta_line(&path).await.unwrap().meta.history_mode,
        ThreadHistoryMode::Paginated
    );
    assert!(state.get_rollout_migration_state("legacy_to_paginated_v1").await.unwrap().is_none());
    history.close().await;
    state.close().await;
}
