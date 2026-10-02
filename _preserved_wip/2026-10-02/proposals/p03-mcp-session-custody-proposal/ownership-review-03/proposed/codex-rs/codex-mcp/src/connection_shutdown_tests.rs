use super::*;
use codex_async_utils::TaskJoinFailure;

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
    assert_eq!(manager.shutdown().await, Err(TaskJoinFailure::Panicked));
    assert_eq!(manager.shutdown().await, Err(TaskJoinFailure::Panicked));
}
