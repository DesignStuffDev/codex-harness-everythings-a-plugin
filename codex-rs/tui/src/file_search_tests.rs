//! Real native searches verify TUI result identity and outer-owner cleanup.

use super::*;
use anyhow::Context;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;

async fn matching_result(
    rx: &mut UnboundedReceiver<AppEvent>,
    manager: &FileSearchManager,
    root: &std::path::Path,
) -> anyhow::Result<(FileSearchRequest, String)> {
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(event) = rx.recv().await {
            if let AppEvent::FileSearchResult { request, query, matches } = event
                && manager.accepts(&request, &query)
                && matches.iter().any(|item| item.path == PathBuf::from("needle.rs"))
            {
                assert!(matches.iter().all(|item| item.root == root));
                return Ok((request, query));
            }
        }
        anyhow::bail!("search event channel closed before the expected result")
    }).await.context("native search did not produce the expected current-root result")?
}

#[tokio::test]
async fn repeated_query_text_and_empty_query_fence_already_queued_results() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("needle.rs"), "fixture")?;
    let (tx, mut rx) = unbounded_channel();
    let runtime = FileSearchRuntime::new();
    let manager = FileSearchManager::new(root.path().to_path_buf(), AppEventSender::new(tx), runtime.clone());
    manager.on_user_query("needle".to_owned());
    let (old_request, old_query) = matching_result(&mut rx, &manager, root.path()).await?;
    manager.on_user_query("different".to_owned());
    manager.on_user_query("needle".to_owned());
    assert!(!manager.accepts(&old_request, &old_query));
    let (current_request, current_query) = matching_result(&mut rx, &manager, root.path()).await?;
    assert_eq!(current_query, old_query);
    assert!(current_request.query_id > old_request.query_id);
    assert!(manager.accepts(&current_request, &current_query));
    let retained_session = manager.state.lock().unwrap().session.clone().context("live native session")?;
    manager.on_user_query(String::new());
    assert!(!manager.accepts(&current_request, &current_query));
    drop(manager);
    runtime.shutdown().await?;
    assert!(retained_session.update_query_tagged("needle", current_request.query_id + 1).is_err());
    assert!(runtime.0.tasks.is_empty());
    Ok(())
}

#[tokio::test]
async fn cwd_and_reconnect_fence_same_query_results_and_search_the_new_root() -> anyhow::Result<()> {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    std::fs::write(first.path().join("needle.rs"), "first")?;
    std::fs::write(second.path().join("needle.rs"), "second")?;
    let (tx, mut rx) = unbounded_channel();
    let runtime = FileSearchRuntime::new();
    let mut manager = FileSearchManager::new(first.path().to_path_buf(), AppEventSender::new(tx), runtime.clone());
    manager.on_user_query("needle".to_owned());
    let (first_request, query) = matching_result(&mut rx, &manager, first.path()).await?;
    manager.update_search_dir(second.path().to_path_buf());
    manager.on_user_query(query.clone());
    assert!(!manager.accepts(&first_request, &query));
    let (second_request, query) = matching_result(&mut rx, &manager, second.path()).await?;
    let old_state = Arc::downgrade(&manager.state);
    let (new_tx, mut new_rx) = unbounded_channel();
    manager.restart(second.path().to_path_buf(), AppEventSender::new(new_tx));
    assert!(Arc::ptr_eq(&manager.runtime.0, &runtime.0));
    manager.on_user_query(query.clone());
    assert!(!manager.accepts(&first_request, &query));
    assert!(!manager.accepts(&second_request, &query));
    let (reconnected_request, query) = matching_result(&mut new_rx, &manager, second.path()).await?;
    assert!(manager.accepts(&reconnected_request, &query));
    runtime.shutdown().await?;
    assert!(old_state.upgrade().is_none());
    // Queued old-channel results may remain, but new-generation callbacks must
    // use the replacement channel. Cleanup must also release every old sender.
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(event) = rx.recv().await {
            if let AppEvent::FileSearchResult { request, query, .. } = event {
                assert!(!manager.accepts(&request, &query));
                assert!(!Arc::ptr_eq(&request.manager, &reconnected_request.manager));
            }
        }
    }).await.context("obsolete event senders survived native cleanup")?;
    assert!(runtime.0.tasks.is_empty());
    drop(manager);
    Ok(())
}

#[tokio::test]
async fn manager_drop_during_startup_releases_weak_state_and_outer_owner_drains() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("needle.rs"), "fixture")?;
    let (tx, mut rx) = unbounded_channel();
    let runtime = FileSearchRuntime::new();
    let manager = FileSearchManager::new(root.path().to_path_buf(), AppEventSender::new(tx), runtime.clone());
    let state = Arc::downgrade(&manager.state);
    manager.on_user_query("needle".to_owned());
    // Current-thread scheduling keeps creation pending until this task yields.
    drop(manager);
    assert!(state.upgrade().is_none());
    runtime.shutdown().await?;
    assert!(runtime.0.tasks.is_empty());
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn closed_runtime_failure_clears_current_results_without_accepting_old_generation() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("needle.rs"), "fixture")?;
    let (tx, mut rx) = unbounded_channel();
    let runtime = FileSearchRuntime::new();
    let manager = FileSearchManager::new(root.path().to_path_buf(), AppEventSender::new(tx), runtime.clone());
    manager.on_user_query("needle".to_owned());
    let (old_request, query) = matching_result(&mut rx, &manager, root.path()).await?;
    runtime.shutdown().await?;
    manager.on_user_query("different".to_owned());
    assert!(!manager.accepts(&old_request, &query));
    let cleared = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(event) = rx.recv().await {
            if let AppEvent::FileSearchResult { request, query, matches } = event
                && manager.accepts(&request, &query)
            {
                return Some((query, matches));
            }
        }
        None
    }).await?.context("current failure result")?;
    assert_eq!(cleared, ("different".to_owned(), Vec::new()));
    drop(manager);
    runtime.shutdown().await?;
    Ok(())
}
