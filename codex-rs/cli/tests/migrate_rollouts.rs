use std::fs;
use std::sync::Arc;

use anyhow::Result;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_rollout::RolloutConfig;
use codex_thread_store::ItemSortKey;
use codex_thread_store::ListItemsParams;
use codex_thread_store::LoadThreadHistoryParams;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::SortDirection;
use codex_thread_store::ThreadStore;
use pretty_assertions::assert_eq;
use serde_json::json;

#[path = "migrate_rollouts/support.rs"]
mod support;

#[cfg(unix)]
#[path = "migrate_rollouts/lifecycle_tests.rs"]
mod lifecycle_tests;

use support::*;

#[tokio::test]
async fn selected_native_dry_run_matches_native_failures_without_storage_mutation() -> Result<()> {
    let root = tempfile::tempdir()?;
    let native = root.path().join("native");
    let selected = root.path().join("selected");
    let mut originals = Vec::new();
    for home in [&native, &selected] {
        for id in [FIRST_ID, SECOND_ID, BROKEN_ID] {
            let path = seed(home, root.path(), id)?;
            if id == BROKEN_ID {
                fs::write(&path, "invalid rollout metadata\n")?;
            }
            originals.push((path.clone(), fs::read(path)?));
        }
    }
    install_native(&selected)?;
    let native_output = run(&native, &[]).await?;
    let selected_output = run(&selected, &[]).await?;
    assert!(!native_output.status.success());
    assert!(!selected_output.status.success());
    let expected = report(&native_output, &native)?;
    assert_eq!(report(&selected_output, &selected)?, expected);
    let outcomes = expected["outcomes"].as_array().expect("report outcomes");
    assert_eq!(outcomes.len(), 3);
    assert_eq!(
        outcomes
            .iter()
            .filter(|item| item["status"] == "eligible")
            .count(),
        2
    );
    assert!(
        outcomes.iter().any(|item| item["status"] == "failed"
            && item["failure_reason"] == "invalid_session_metadata")
    );
    for (path, original) in originals {
        assert_eq!(fs::read(path)?, original);
    }
    assert_no_metadata(&native)?;
    assert_no_metadata(&selected)?;
    #[cfg(unix)]
    assert_reaped(&selected).await?;
    Ok(())
}

#[tokio::test]
async fn selected_native_apply_matches_native_filter_and_cold_readback() -> Result<()> {
    let root = tempfile::tempdir()?;
    let native = root.path().join("native");
    let selected = root.path().join("selected");
    let mut unchanged = Vec::new();
    for home in [&native, &selected] {
        seed(home, root.path(), FIRST_ID)?;
        let path = seed(home, root.path(), SECOND_ID)?;
        unchanged.push((path.clone(), fs::read(path)?));
    }
    install_native(&selected)?;
    let mut reports = Vec::new();
    let mut histories = Vec::new();
    for home in [&native, &selected] {
        let output = run(
            home,
            &["--apply", "--thread", FIRST_ID, "--max-mib-per-second", "8"],
        )
        .await?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let migrated = report(&output, home)?;
        assert_eq!(migrated["outcomes"].as_array().expect("outcomes").len(), 1);
        assert_eq!(migrated["outcomes"][0]["status"], "migrated");
        reports.push(migrated);
        let metadata = codex_rollout::read_session_meta_line(&rollout(home, FIRST_ID)).await?;
        assert_eq!(metadata.meta.history_mode, ThreadHistoryMode::Paginated);
        // Reopen after CLI/worker exit: accepted writes must already be durable.
        let readback = RolloutConfig {
            codex_home: home.to_path_buf(),
            sqlite: sqlite(home)?,
            cwd: home.to_path_buf(),
            model_provider_id: "openai".to_owned(),
            generate_memories: false,
        };
        let state = codex_rollout::state_db::try_init(&readback).await?;
        let store = LocalThreadStore::new(
            LocalThreadStoreConfig::from_config(&readback),
            Some(Arc::clone(&state)),
        );
        // Paginated threads deliberately reject the legacy full-history API.
        // Verify both actual SQLite projection and the supported resume context.
        let model_context = store
            .load_latest_model_context(LoadThreadHistoryParams {
                thread_id: ThreadId::from_string(FIRST_ID)?,
                include_archived: false,
            })
            .await?;
        let items = store
            .list_items(ListItemsParams {
                thread_id: ThreadId::from_string(FIRST_ID)?,
                turn_id: None,
                include_archived: false,
                position: None,
                page_size: 10,
                sort_direction: SortDirection::Asc,
                sort_key: ItemSortKey::CreatedAtOrdinal,
                after_updated_at_ordinal: None,
            })
            .await?;
        store.shutdown_store().await?;
        state.close().await;
        let projected_items = items
            .items
            .iter()
            .map(|item| serde_json::from_slice::<serde_json::Value>(&item.item_json))
            .collect::<Result<Vec<_>, _>>()?;
        for (view, representation) in [
            ("model_context", serde_json::to_value(&model_context)?),
            ("projected_items", serde_json::to_value(&projected_items)?),
        ] {
            let serialized = representation.to_string();
            assert!(
                serialized.contains("Preserve this migration question"),
                "{view} omitted the question after cold readback: {serialized}"
            );
            assert!(
                serialized.contains("Preserve this migration answer"),
                "{view} omitted the answer after cold readback: {serialized}"
            );
        }
        let mut history = json!({"model_context":model_context,"items":items});
        normalize(&mut history, home);
        histories.push(history);
        #[cfg(unix)]
        if home == &selected {
            assert_reaped(home).await?;
        }
        let repeated = run(home, &["--apply", "--thread", FIRST_ID]).await?;
        assert!(
            repeated.status.success(),
            "{}",
            String::from_utf8_lossy(&repeated.stderr)
        );
        assert_eq!(
            report(&repeated, home)?["outcomes"][0]["status"],
            "already_paginated"
        );
    }
    assert_eq!(reports[0], reports[1]);
    assert_eq!(histories[0], histories[1]);
    for (path, original) in unchanged {
        assert_eq!(fs::read(path)?, original);
    }
    #[cfg(unix)]
    assert_reaped(&selected).await?;
    Ok(())
}
