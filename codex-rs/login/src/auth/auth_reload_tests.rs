use super::super::AuthHeaders;
use super::super::AuthRouteConfig;
use super::super::ExternalAuth;
use super::super::ExternalAuthFuture;
use super::super::ExternalAuthRefreshContext;
use super::super::RefreshTokenError;
use super::super::ReloadOutcome;
use super::*;
use base64::Engine;
use codex_http_client::DestinationPolicy;
use codex_http_client::NetworkPolicyController;
use codex_protocol::auth::RefreshTokenFailedReason;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

enum Reply {
    Auth(CodexAuth),
    Permanent,
    Transient,
}
struct HeldExternal {
    initial: CodexAuth,
    reply: Reply,
    calls: AtomicUsize,
    entered: Notify,
    release: Semaphore,
}
impl ExternalAuth for HeldExternal {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async move {
            if self.calls.fetch_add(/*val*/ 1, Ordering::SeqCst) == 0 {
                return Ok(self.initial.clone());
            }
            self.entered.notify_one();
            self.release.acquire().await.unwrap().forget();
            match &self.reply {
                Reply::Auth(auth) => Ok(auth.clone()),
                Reply::Permanent | Reply::Transient => {
                    Err(std::io::Error::other("fixture load failure"))
                }
            }
        })
    }
    fn refresh(&self, _context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        self.resolve()
    }
    fn classify_error(&self, error: std::io::Error) -> RefreshTokenError {
        match &self.reply {
            Reply::Permanent => RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                RefreshTokenFailedReason::Other,
                error.to_string(),
            )),
            Reply::Auth(_) | Reply::Transient => RefreshTokenError::Transient(error),
        }
    }
}
async fn fixture(initial: CodexAuth, reply: Reply) -> (Arc<AuthManager>, Arc<HeldExternal>) {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let external = Arc::new(HeldExternal {
        initial,
        reply,
        calls: AtomicUsize::new(/*v*/ 0),
        entered: Notify::new(),
        release: Semaphore::new(/*permits*/ 0),
    });
    manager.set_external_auth(external.clone()).await.unwrap();
    (manager, external)
}
fn policy_aba(manager: &AuthManager) {
    manager
        .set_forced_chatgpt_workspace_id(Some(Vec::new()))
        .unwrap();
    manager
        .set_forced_chatgpt_workspace_id(/*workspace_id*/ None)
        .unwrap();
}
fn header_auth(token: &str) -> CodexAuth {
    let mut headers = http::HeaderMap::new();
    headers.insert("authorization", http::HeaderValue::from_str(token).unwrap());
    headers.insert(
        "chatgpt-account-id",
        http::HeaderValue::from_static("account-a"),
    );
    CodexAuth::Headers(AuthHeaders::new(headers))
}

#[tokio::test]
async fn both_reload_paths_reject_policy_aba_during_actual_external_resolution() {
    for guarded in [false, true] {
        let initial = header_auth("Bearer initial");
        let (mut manager, external) = fixture(
            initial.clone(),
            Reply::Auth(header_auth("Bearer replacement")),
        )
        .await;
        let controller = NetworkPolicyController::default();
        let policy = controller.policy();
        let mut sibling = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
        for owner in [&mut manager, &mut sibling] {
            let factory = owner
                .http_client_factory()
                .with_network_policy(policy.clone());
            Arc::get_mut(owner).unwrap().auth_route_config =
                AuthRouteConfig::from_http_client_factory(factory);
        }
        assert!(controller.publish(policy.revision(), DestinationPolicy::Unrestricted));
        let permit = sibling
            .http_client_factory()
            .network_policy()
            .acquire(&"https://example.com/".parse().unwrap())
            .unwrap();
        let changes = manager.auth_change_state_receiver();
        let legacy = manager.auth_change_receiver();
        let before = *changes.borrow();
        let task_manager = manager.clone();
        let task = tokio::spawn(async move {
            if guarded {
                assert!(matches!(
                    task_manager
                        .reload_if_account_id_matches(Some("account-a"))
                        .await,
                    ReloadOutcome::Skipped
                ));
            } else {
                assert!(!task_manager.reload().await);
            }
        });
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
            .await
            .unwrap();
        policy_aba(&manager);
        external.release.add_permits(/*n*/ 1);
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(manager.auth_cached(), Some(initial));
        assert_eq!(*changes.borrow(), before);
        assert!(!changes.has_changed().unwrap());
        assert!(!legacy.has_changed().unwrap());
        assert!(permit.check().is_ok());
    }
}

#[tokio::test]
async fn successful_reload_preserves_credential_and_owner_notification_semantics() {
    let (manager, external) = fixture(
        CodexAuth::from_api_key("initial"),
        Reply::Auth(CodexAuth::from_api_key("replacement")),
    )
    .await;
    let changes = manager.auth_change_state_receiver();
    let legacy = manager.auth_change_receiver();
    let before = *changes.borrow();
    let legacy_before = *legacy.borrow();
    let task_manager = manager.clone();
    let task = tokio::spawn(async move { task_manager.reload().await });
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
        .await
        .unwrap();
    external.release.add_permits(/*n*/ 1);
    // Legacy changed compares coarse auth type; credential revisions still detect an API-key replacement.
    assert!(
        !tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        manager.auth_cached().unwrap().get_token().unwrap(),
        "replacement"
    );
    assert_eq!(
        *changes.borrow(),
        super::super::AuthChangeState {
            generation: before.generation + 1,
            owner_generation: before.owner_generation + 1
        }
    );
    assert_eq!(*legacy.borrow(), legacy_before + 1);
}

#[tokio::test]
async fn native_load_error_preserves_cache_but_successful_absence_clears_it() {
    let home = tempfile::tempdir().unwrap();
    let mut manager = AuthManager::from_auth_for_testing_with_home(
        CodexAuth::from_api_key("cached"),
        home.path().to_path_buf(),
    );
    // Excludes access-token env auth without changing the process environment.
    Arc::get_mut(&mut manager).unwrap().forced_login_method =
        Some(codex_protocol::config_types::ForcedLoginMethod::Api);
    let path = home.path().join("auth.json");
    std::fs::write(&path, b"{invalid-json").unwrap();
    assert!(matches!(
        manager.load_auth().await,
        Err(AuthLoadError::Source(_))
    ));
    let changes = manager.auth_change_state_receiver();
    assert!(!manager.reload().await);
    assert_eq!(
        manager.auth_cached().unwrap().get_token().unwrap(),
        "cached"
    );
    assert!(!changes.has_changed().unwrap());
    std::fs::remove_file(path).unwrap();
    assert!(manager.reload().await);
    assert_eq!(manager.auth_cached(), None);
    assert!(changes.has_changed().unwrap());
}

#[tokio::test]
async fn actual_native_loaded_candidate_is_rechecked_at_commit_after_aba() {
    let home = tempfile::tempdir().unwrap();
    super::super::login_with_api_key(
        home.path(),
        "loaded",
        crate::AuthCredentialsStoreMode::Ephemeral,
        crate::AuthKeyringBackendKind::default(),
    )
    .unwrap();
    let manager = AuthManager::from_auth_for_testing_with_home(
        CodexAuth::from_api_key("cached"),
        home.path().to_path_buf(),
    );
    let changes = manager.auth_change_state_receiver();
    let loaded = manager.load_auth().await.unwrap();
    assert_eq!(
        loaded.captured_auth().unwrap().get_token().unwrap(),
        "loaded"
    );
    policy_aba(&manager);
    assert!(matches!(
        manager.commit_auth_load(loaded),
        Err(AuthLoadError::Policy(AuthPolicyError::Changed))
    ));
    assert_eq!(
        manager.auth_cached().unwrap().get_token().unwrap(),
        "cached"
    );
    assert!(!changes.has_changed().unwrap());
    assert!(!manager.reload().await);
    assert_eq!(
        manager.auth_cached().unwrap().get_token().unwrap(),
        "loaded"
    );
    assert!(changes.has_changed().unwrap());
}

#[tokio::test]
async fn permanent_failure_metadata_waits_for_current_policy_and_preserves_fast_path() {
    for revoked in [false, true] {
        let initial = CodexAuth::from_api_key("cached");
        let (manager, external) = fixture(initial.clone(), Reply::Permanent).await;
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { task_manager.reload().await });
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
            .await
            .unwrap();
        if revoked {
            policy_aba(&manager);
        }
        external.release.add_permits(/*n*/ 1);
        assert!(
            !tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
                .await
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            manager.auth_cached().unwrap().get_token().unwrap(),
            "cached"
        );
        assert_eq!(
            manager.refresh_failure_for_auth(&initial).is_some(),
            !revoked
        );
        if !revoked {
            assert!(
                !tokio::time::timeout(Duration::from_secs(/*secs*/ 5), manager.reload())
                    .await
                    .unwrap()
            );
            assert_eq!(external.calls.load(Ordering::SeqCst), 2);
        }
    }
}

#[tokio::test]
async fn preserved_external_failure_does_not_restore_or_poison_a_newer_cache() {
    let (manager, external) = fixture(CodexAuth::from_api_key("initial"), Reply::Permanent).await;
    let task_manager = manager.clone();
    let task = tokio::spawn(async move { task_manager.reload().await });
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
        .await
        .unwrap();
    let newer = CodexAuth::from_api_key("newer");
    manager.set_cached_auth(Some(newer.clone()));
    external.release.add_permits(/*n*/ 1);
    assert!(
        !tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap()
    );
    assert_eq!(manager.auth_cached().unwrap().get_token().unwrap(), "newer");
    assert!(manager.refresh_failure_for_auth(&newer).is_none());
}

#[tokio::test]
async fn transient_external_error_is_not_a_successful_absent_credential() {
    let (manager, external) = fixture(CodexAuth::from_api_key("cached"), Reply::Transient).await;
    let task_manager = manager.clone();
    let task = tokio::spawn(async move { task_manager.reload().await });
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
        .await
        .unwrap();
    external.release.add_permits(/*n*/ 1);
    assert!(
        !tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        manager.auth_cached().unwrap().get_token().unwrap(),
        "cached"
    );
}

#[tokio::test]
async fn valid_native_workspace_filter_still_clears_cached_auth() {
    let home = tempfile::tempdir().unwrap();
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"https://api.openai.com/auth":{"chatgpt_user_id":"user-a"}}"#);
    super::super::login_with_chatgpt_auth_tokens(
        home.path(),
        &format!("header.{payload}.signature"),
        "workspace-a",
        /*chatgpt_plan_type*/ None,
    )
    .unwrap();
    let manager = AuthManager::from_auth_for_testing_with_home(
        header_auth("Bearer cached"),
        home.path().to_path_buf(),
    );
    manager
        .set_forced_chatgpt_workspace_id(Some(vec!["workspace-b".to_string()]))
        .unwrap();
    assert_eq!(manager.load_auth().await.unwrap().captured_auth(), None);
    let changes = manager.auth_change_state_receiver();
    assert!(manager.reload().await);
    assert_eq!(manager.auth_cached(), None);
    assert!(changes.has_changed().unwrap());
}

#[tokio::test]
async fn external_workspace_rejection_still_clears_through_reload_and_auth() {
    for use_auth in [false, true] {
        let initial = header_auth("Bearer cached");
        let (manager, external) = fixture(initial.clone(), Reply::Auth(initial)).await;
        manager
            .set_forced_chatgpt_workspace_id(Some(vec!["other-workspace".to_string()]))
            .unwrap();
        let changes = manager.auth_change_state_receiver();
        let task_manager = manager.clone();
        let task = tokio::spawn(async move {
            if use_auth {
                assert_eq!(task_manager.auth().await, None);
            } else {
                assert!(task_manager.reload().await);
            }
        });
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
            .await
            .unwrap();
        external.release.add_permits(/*n*/ 1);
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(manager.auth_cached(), None);
        assert!(changes.has_changed().unwrap());
        assert_eq!(external.calls.load(Ordering::SeqCst), 2);
    }
}

#[tokio::test]
async fn actual_native_agent_identity_policy_rejection_is_not_a_storage_error() {
    let home = tempfile::tempdir().unwrap();
    let record = super::super::AgentIdentityAuthRecord {
        agent_runtime_id: "fixture-agent".to_string(),
        agent_private_key: codex_agent_identity::generate_agent_key_material()
            .unwrap()
            .private_key_pkcs8_base64,
        account_id: "account-a".to_string(),
        chatgpt_user_id: "user-a".to_string(),
        email: None,
        plan_type: codex_protocol::account::PlanType::Pro,
        chatgpt_account_is_fedramp: false,
        task_id: None,
    };
    let stored = super::super::AuthDotJson {
        auth_mode: Some(codex_protocol::auth::AuthMode::AgentIdentity),
        openai_api_key: None,
        tokens: None,
        last_refresh: None,
        agent_identity: Some(super::super::AgentIdentityStorage::Record(record.clone())),
        personal_access_token: None,
        bedrock_api_key: None,
        bedrock_access_keys: None,
    };
    super::super::save_auth(
        home.path(),
        &stored,
        crate::AuthCredentialsStoreMode::Ephemeral,
        crate::AuthKeyringBackendKind::default(),
    )
    .unwrap();
    let manager = AuthManager::from_auth_for_testing_with_home(
        header_auth("Bearer cached"),
        home.path().to_path_buf(),
    );
    let workspaces = vec!["other-workspace".to_string()];
    let rejection =
        super::super::ensure_auth_workspace_allowed(Some(&workspaces), &record.account_id)
            .unwrap_err();
    assert_eq!(rejection.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(
        rejection.to_string(),
        crate::server::ensure_workspace_account_allowed(Some(&workspaces), &record.account_id)
            .unwrap_err()
    );
    assert!(rejection.get_ref().unwrap().is::<NativePolicyRejection>());
    manager
        .set_forced_chatgpt_workspace_id(Some(workspaces))
        .unwrap();
    assert_eq!(manager.load_auth().await.unwrap().captured_auth(), None);
    let changes = manager.auth_change_state_receiver();
    assert!(manager.reload().await);
    assert_eq!(manager.auth_cached(), None);
    assert!(changes.has_changed().unwrap());
    // The test exercised the real stored loader; source contents remain intact.
    assert_eq!(
        super::super::load_auth_dot_json(
            home.path(),
            crate::AuthCredentialsStoreMode::Ephemeral,
            crate::AuthKeyringBackendKind::default()
        )
        .unwrap(),
        Some(stored)
    );
}

#[path = "auth_acquisition_tests.rs"]
mod acquisition_tests;

#[path = "auth_cache_revision_tests.rs"]
mod cache_revision_tests;

#[path = "auth_reload_source_tests.rs"]
mod source_revision_tests;
