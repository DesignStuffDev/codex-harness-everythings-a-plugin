use super::*;
use codex_rmcp_client::McpShutdownFailure;

#[tokio::test]
async fn shutdown_panic_is_sticky_across_observers() {
    let approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
    let permission_profile = Constrained::allow_any(PermissionProfile::default());
    let mut manager = McpConnectionSet::new_uninitialized(&approval_policy, &permission_profile, /*prefix_mcp_tool_names*/ true);
    let client: ManagedClientFuture = async {
        panic!("controlled connection shutdown panic");
    }.boxed().shared();
    manager.insert_test_client("pending", AsyncManagedClient {
        client,
        is_codex_apps_mcp_server: false,
        server_capabilities: Arc::new(std::sync::Mutex::new(None)),
        cached_server_info: None,
        codex_apps_tools_cache_context: None,
        tool_catalog_cache_context: None,
        startup_complete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        startup_reconnect: None,
        cancel_token: CancellationToken::new(),
    });
    assert_eq!(manager.shutdown().await, Err(McpShutdownFailure::Task));
    assert_eq!(manager.shutdown().await, Err(McpShutdownFailure::Task));
}


#[tokio::test]
async fn uncertain_startup_cleanup_retains_exact_owner_after_observers_leave() -> anyhow::Result<()> {
    use codex_rmcp_client::McpShutdownConfirmation;
    let approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
    let permission_profile = Constrained::allow_any(PermissionProfile::default());
    let mut manager = McpConnectionSet::new_uninitialized(
        &approval_policy, &permission_profile, /*prefix_mcp_tool_names*/ true,
    );
    let client: ManagedClientFuture = async { Err(StartupOutcomeError::Cancelled) }.boxed().shared();
    manager.insert_test_client("cancelled-startup", AsyncManagedClient {
        client,
        is_codex_apps_mcp_server: false,
        server_capabilities: Arc::new(std::sync::Mutex::new(None)),
        cached_server_info: None,
        codex_apps_tools_cache_context: None,
        tool_catalog_cache_context: None,
        startup_complete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        startup_reconnect: None,
        cancel_token: CancellationToken::new(),
    });
    assert_eq!(manager.shutdown_confirmation(), None);
    assert_eq!(manager.shutdown().await, Ok(McpShutdownConfirmation::Unconfirmed));
    assert_eq!(manager.shutdown_confirmation(), Some(Ok(McpShutdownConfirmation::Unconfirmed)));
    let owner = manager.shutdown_task.get().ok_or_else(|| anyhow::anyhow!("missing shutdown owner"))?;
    let first = owner.wait().await?;
    let second = owner.wait().await?;
    assert!(Arc::ptr_eq(&first, &second));
    let weak = Arc::downgrade(owner);
    drop(manager);
    let retained = weak.upgrade().ok_or_else(|| anyhow::anyhow!("unconfirmed owner retired"))?;
    assert_eq!(retained.wait().await?.0, Ok(McpShutdownConfirmation::Unconfirmed));
    Ok(())
}

#[tokio::test]
async fn confirmed_empty_snapshot_retires_after_exact_join() -> anyhow::Result<()> {
    use codex_rmcp_client::McpShutdownConfirmation;
    let approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
    let permission_profile = Constrained::allow_any(PermissionProfile::default());
    let manager = McpConnectionSet::new_uninitialized(
        &approval_policy, &permission_profile, /*prefix_mcp_tool_names*/ true,
    );
    assert_eq!(manager.shutdown_confirmation(), None);
    assert_eq!(manager.shutdown().await, Ok(McpShutdownConfirmation::Confirmed));
    assert_eq!(manager.shutdown_confirmation(), Some(Ok(McpShutdownConfirmation::Confirmed)));
    let owner = manager.shutdown_task.get().ok_or_else(|| anyhow::anyhow!("missing shutdown owner"))?;
    let weak = Arc::downgrade(owner);
    drop(manager);
    assert!(weak.upgrade().is_none());
    Ok(())
}
