use super::*;
use codex_login::CodexAuth;
use crate::config::test_config;
use crate::session::session_shutdown::SessionShutdownFailure;
use crate::thread_manager::StartThreadOptions;
use crate::thread_startup_metadata::ThreadStartupMetadata;
use codex_protocol::error::CodexErrorDetails;
use futures::FutureExt;
use pretty_assertions::assert_eq;

async fn failed_thread() -> anyhow::Result<(tempfile::TempDir, ThreadManager, Arc<CodexThread>)> {
    let home = tempfile::tempdir()?;
    let mut config = test_config().await;
    config.codex_home = codex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(home.path())?;
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"),
        config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let started = manager.start_thread(StartThreadOptions::new(config)).await?;
    started.thread.shutdown_and_wait().await?;
    let mut io = started.thread.io.clone();
    io.session_loop_termination = futures::future::ready(
        Err(SessionShutdownFailure::McpRuntime),
    ).boxed().shared();
    let failed = Arc::new(CodexThread::new(
        Arc::clone(&started.thread.session),
        io,
        ThreadStartupMetadata::from(&started.session_configured),
        started.session_configured.rollout_path,
        started.thread.session_source.clone(),
    ));
    manager.state.threads.write().await.insert(started.thread_id, Arc::clone(&failed));
    Ok((home, manager, failed))
}

#[tokio::test]
async fn shutdown_report_retains_failure_after_live_map_removal() -> anyhow::Result<()> {
    let (_home, manager, failed) = failed_thread().await?;
    let thread_id = failed.session.thread_id();
    let expected = ThreadShutdownReport {
        shutdown_failed: vec![thread_id],
        ..Default::default()
    };
    assert_eq!(manager.shutdown_all_threads_bounded(Duration::from_secs(/*secs*/ 1)).await,
        expected);
    let still_registered = manager.get_thread(thread_id).await?;
    assert!(Arc::ptr_eq(&still_registered, &failed));
    manager.remove_thread(&thread_id).await;
    assert_eq!(manager.shutdown_all_threads_bounded(Duration::from_secs(/*secs*/ 1)).await,
        expected);
    let result = manager.get_thread_for_client_removal(&thread_id).await;
    assert!(matches!(result, Err(error) if matches!(error.details(), CodexErrorDetails::ShutdownFailed)));
    Ok(())
}

#[tokio::test]
async fn partial_startup_failure_is_reported_without_live_registration() -> anyhow::Result<()> {
    let (_home, manager, failed) = failed_thread().await?;
    let thread_id = failed.session.thread_id();
    manager.remove_thread(&thread_id).await;
    // Managed partial startup has a Session before it has a registered CodexThread.
    manager.state.record_failed_session(Arc::clone(&failed.session)).await;
    manager.state.record_failed_session(Arc::clone(&failed.session)).await;
    assert_eq!(manager.state.failed_shutdowns.read().await.len(), 1);
    let weak = Arc::downgrade(&failed.session);
    drop(failed);
    assert!(weak.upgrade().is_some());
    assert_eq!(manager.shutdown_all_threads_bounded(Duration::from_secs(/*secs*/ 1)).await,
        ThreadShutdownReport { shutdown_failed: vec![thread_id], ..Default::default() });
    Ok(())
}

#[tokio::test]
async fn generic_removal_keeps_terminal_failure_for_final_observer() -> anyhow::Result<()> {
    let (_home, manager, failed) = failed_thread().await?;
    let thread_id = failed.session.thread_id();
    manager.remove_thread(&thread_id).await;
    drop(failed);
    assert_eq!(manager.shutdown_all_threads_bounded(Duration::from_secs(/*secs*/ 1)).await,
        ThreadShutdownReport { shutdown_failed: vec![thread_id], ..Default::default() });
    Ok(())
}

#[tokio::test]
async fn repeated_distinct_clean_removals_release_sessions_without_final_drain() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let mut config = test_config().await;
    config.codex_home = codex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(home.path())?;
    let manager = ThreadManager::with_models_provider_and_home_for_tests(
        CodexAuth::from_api_key("dummy"), config.model_provider.clone(),
        config.codex_home.to_path_buf(),
        Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
    );
    let mut ids = std::collections::HashSet::new();
    for index in 0..4 {
        let started = manager.start_thread(StartThreadOptions::new(config.clone())).await?;
        assert!(ids.insert(started.thread_id));
        let weak = Arc::downgrade(&started.thread);
        started.thread.shutdown_and_wait().await?;
        let removed = if index % 2 == 0 {
            manager.remove_thread(&started.thread_id).await
        } else {
            manager.remove_thread_if_matches(&started.thread_id, &started.thread).await
        };
        assert!(removed.as_ref().is_some_and(|thread| Arc::ptr_eq(thread, &started.thread)));
        drop(removed);
        drop(started);
        assert!(weak.upgrade().is_none());
        assert!(manager.state.removed_threads.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner).is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn opportunistic_retirement_preserves_pending_and_failed_exact_owners() -> anyhow::Result<()> {
    let (_home, manager, failed) = failed_thread().await?;
    let mut io = failed.io.clone();
    io.session_loop_termination = std::future::pending().boxed().shared();
    let pending = Arc::new(CodexThread::new(
        Arc::clone(&failed.session), io,
        ThreadStartupMetadata::from(&failed.startup_metadata().to_session_configured_event(/*initial_messages*/ None)),
        failed.rollout_path(), failed.session_source.clone(),
    ));
    manager.state.retain_removed_thread(&failed);
    manager.state.retain_removed_thread(&pending);
    manager.state.retire_completed_removed_threads();
    let removed = manager.state.removed_threads.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(removed.len(), 2);
    assert!(removed.iter().any(|owner| Arc::ptr_eq(owner, &failed)));
    assert!(removed.iter().any(|owner| Arc::ptr_eq(owner, &pending)));
    Ok(())
}
