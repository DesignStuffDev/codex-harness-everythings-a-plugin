use super::super::AuthHeaders;
use super::super::AuthManager;
use super::super::AuthRouteConfig;
use super::super::CodexAuth;
use super::super::ExternalAuth;
use super::super::ExternalAuthFuture;
use super::super::ExternalAuthRefreshContext;
use super::super::RefreshTokenError;
use super::*;
use codex_http_client::DestinationPolicy;
use codex_http_client::NetworkPolicyController;
use pretty_assertions::assert_eq;

struct StaticExternal(CodexAuth);
impl ExternalAuth for StaticExternal {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async { Ok(self.0.clone()) })
    }
    fn refresh(&self, _context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        self.resolve()
    }
}

fn poison_policy(manager: &Arc<AuthManager>) {
    let manager = manager.clone();
    assert!(
        std::thread::spawn(move || {
            let _lock = manager.workspace_policy.state.lock().unwrap();
            panic!("intentional private policy poison");
        })
        .join()
        .is_err()
    );
}

#[test]
fn coalesced_workspace_changes_cannot_revive_stamps_or_impersonate_auth_changes() {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let initial = manager.auth_policy_snapshot().unwrap();
    let mut changed = manager.auth_policy_changes();
    let auth_changes = manager.auth_change_state_receiver();
    let legacy_changes = manager.auth_change_receiver();
    let auth_state = *auth_changes.borrow();
    manager
        .set_forced_chatgpt_workspace_id(/*workspace_id*/ None)
        .unwrap();
    assert!(!changed.has_changed().unwrap());
    assert_eq!(
        manager.auth_policy_snapshot().unwrap().stamp(),
        initial.stamp()
    );
    manager
        .set_forced_chatgpt_workspace_id(Some(vec!["private-workspace".into()]))
        .unwrap();
    manager
        .set_forced_chatgpt_workspace_id(/*workspace_id*/ None)
        .unwrap();
    assert!(changed.has_changed().unwrap());
    changed.borrow_and_update();
    assert_eq!(
        manager.validate_policy_stamp(&initial.stamp()),
        Err(AuthPolicyError::Changed)
    );
    assert_eq!(manager.auth_policy_snapshot().unwrap().stamp.epoch, 2);
    assert_eq!(*auth_changes.borrow(), auth_state);
    assert!(!auth_changes.has_changed().unwrap());
    assert!(!legacy_changes.has_changed().unwrap());
    manager
        .set_forced_chatgpt_workspace_id(/*workspace_id*/ None)
        .unwrap();
    assert!(!changed.has_changed().unwrap());
    manager
        .set_forced_chatgpt_workspace_id(Some(vec!["private-workspace".into()]))
        .unwrap();
    assert!(
        !format!("{:?}", manager.auth_policy_snapshot().unwrap()).contains("private-workspace")
    );
}

#[test]
fn managed_intersection_and_empty_policy_preserve_native_validation() {
    let mut manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    Arc::get_mut(&mut manager)
        .unwrap()
        .managed_auth_policy
        .allowed_chatgpt_workspaces = Some(vec!["managed".into()]);
    let api = StaticExternal(CodexAuth::from_api_key("fixture-key"));
    let mut headers = http::HeaderMap::new();
    headers.insert(
        "chatgpt-account-id",
        http::HeaderValue::from_static("managed"),
    );
    let account = StaticExternal(CodexAuth::Headers(AuthHeaders::new(headers)));
    for (forced, effective, allowed) in [
        (None, vec!["managed".to_string()], true),
        (
            Some(vec!["managed".to_string(), "other".to_string()]),
            vec!["managed".to_string()],
            true,
        ),
        (Some(vec!["other".to_string()]), Vec::new(), false),
        (Some(Vec::new()), Vec::new(), false),
    ] {
        manager
            .set_forced_chatgpt_workspace_id(forced.clone())
            .unwrap();
        let snapshot = manager.auth_policy_snapshot().unwrap();
        assert_eq!(snapshot.forced_chatgpt_workspace_id(), forced.as_deref());
        assert_eq!(
            snapshot.effective_chatgpt_workspaces(),
            Some(effective.as_slice())
        );
        let methods = if allowed {
            vec![ForcedLoginMethod::Api, ForcedLoginMethod::Chatgpt]
        } else {
            vec![ForcedLoginMethod::Api]
        };
        assert_eq!(snapshot.allowed_login_methods(), methods.as_slice());
        assert_eq!(manager.allowed_login_methods(), methods);
        assert!(manager.validate_external_auth(&api.0, &api).is_ok());
        assert_eq!(
            manager.validate_external_auth(&account.0, &account).is_ok(),
            allowed
        );
    }
}

#[test]
fn equal_epochs_from_different_managers_do_not_share_authority_or_revoke_network() {
    let controller = NetworkPolicyController::default();
    let policy = controller.policy();
    let mut a = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let mut b = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    for manager in [&mut a, &mut b] {
        let factory = manager
            .http_client_factory()
            .with_network_policy(policy.clone());
        Arc::get_mut(manager).unwrap().auth_route_config =
            AuthRouteConfig::from_http_client_factory(factory);
    }
    assert!(controller.publish(policy.revision(), DestinationPolicy::Unrestricted));
    let before = b.auth_policy_snapshot().unwrap().stamp();
    assert_eq!(
        a.validate_policy_stamp(&before),
        Err(AuthPolicyError::Changed)
    );
    let permit = b
        .http_client_factory()
        .network_policy()
        .acquire(&"https://example.com/".parse().unwrap())
        .unwrap();
    let network_revision = policy.revision();
    a.set_forced_chatgpt_workspace_id(Some(Vec::new())).unwrap();
    assert_eq!(b.validate_policy_stamp(&before), Ok(()));
    assert_eq!(b.auth_policy_snapshot().unwrap().stamp(), before);
    assert_eq!(policy.revision(), network_revision);
    assert!(permit.check().is_ok());
}

#[test]
fn overflow_and_poison_permanently_deny_native_policy_decisions() {
    for poison in [false, true] {
        let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
        let mut changes = manager.auth_policy_changes();
        let expected = if poison {
            AuthPolicyError::Unavailable
        } else {
            AuthPolicyError::RevisionExhausted
        };
        let stamp = if poison {
            let stamp = manager.auth_policy_snapshot().unwrap().stamp();
            poison_policy(&manager);
            stamp
        } else {
            manager.workspace_policy.state.lock().unwrap().epoch = u64::MAX - 1;
            manager
                .set_forced_chatgpt_workspace_id(Some(vec!["last".into()]))
                .unwrap();
            let stamp = manager.auth_policy_snapshot().unwrap().stamp();
            assert_eq!(stamp.epoch, u64::MAX);
            changes.borrow_and_update();
            stamp
        };
        assert_eq!(
            manager.set_forced_chatgpt_workspace_id(Some(vec!["rejected".into()])),
            Err(expected)
        );
        assert!(changes.has_changed().unwrap());
        changes.borrow_and_update();
        assert_eq!(manager.auth_policy_snapshot().unwrap_err(), expected);
        assert_eq!(manager.validate_policy_stamp(&stamp), Err(expected));
        for value in [None, Some(Vec::new()), Some(vec!["last".into()])] {
            assert_eq!(
                manager.set_forced_chatgpt_workspace_id(value),
                Err(expected)
            );
        }
        assert!(!changes.has_changed().unwrap());
        if !poison {
            poison_policy(&manager);
            assert_eq!(manager.auth_policy_snapshot().unwrap_err(), expected);
            assert!(!changes.has_changed().unwrap());
        }
        assert_eq!(manager.forced_chatgpt_workspace_id(), Some(Vec::new()));
        assert_eq!(manager.effective_chatgpt_workspaces(), Some(Vec::new()));
        assert_eq!(
            manager.allowed_login_methods(),
            Vec::<ForcedLoginMethod>::new()
        );
        for method in [ForcedLoginMethod::Api, ForcedLoginMethod::Chatgpt] {
            assert!(!manager.is_login_method_allowed(method));
        }
        let external = StaticExternal(CodexAuth::from_api_key("fixture-key"));
        let Err(RefreshTokenError::Transient(error)) =
            manager.validate_external_auth(&external.0, &external)
        else {
            panic!("unavailable policy must reject even API auth through the provider classifier");
        };
        assert_eq!(
            error.get_ref().unwrap().downcast_ref::<AuthPolicyError>(),
            Some(&expected)
        );
        let state = manager
            .workspace_policy
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_ne!(
            state.forced.as_deref(),
            Some(["rejected".to_string()].as_slice())
        );
    }
}

#[test]
fn concurrent_snapshots_bind_exact_value_to_revision() {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let initial = manager.auth_policy_snapshot().unwrap().stamp();
    std::thread::scope(|scope| {
        let start = Arc::new(std::sync::Barrier::new(2));
        let writer_start = start.clone();
        let writer_manager = manager.clone();
        scope.spawn(move || {
            writer_start.wait();
            for epoch in 1..=1000 {
                writer_manager
                    .set_forced_chatgpt_workspace_id(Some(vec![epoch.to_string()]))
                    .unwrap();
            }
        });
        start.wait();
        for _ in 0..1000 {
            let snapshot = manager.auth_policy_snapshot().unwrap();
            let expected =
                (snapshot.stamp.epoch != 0).then(|| vec![snapshot.stamp.epoch.to_string()]);
            assert_eq!(snapshot.forced_chatgpt_workspace_id(), expected.as_deref());
        }
    });
    assert_eq!(manager.auth_policy_snapshot().unwrap().stamp.epoch, 1000);
    assert_eq!(
        manager.validate_policy_stamp(&initial),
        Err(AuthPolicyError::Changed)
    );
}

#[tokio::test]
async fn unavailable_policy_blocks_native_load_even_with_valid_stored_api_auth() {
    let home = tempfile::tempdir().unwrap();
    super::super::login_with_api_key(
        home.path(),
        "fixture-policy-key",
        crate::AuthCredentialsStoreMode::Ephemeral,
        crate::AuthKeyringBackendKind::default(),
    )
    .unwrap();
    let expected = CodexAuth::from_api_key("fixture-policy-key");
    let manager =
        AuthManager::from_auth_for_testing_with_home(expected.clone(), home.path().to_path_buf());
    assert_eq!(manager.load_auth().await, Some(expected.clone()));
    manager.workspace_policy.state.lock().unwrap().epoch = u64::MAX;
    assert_eq!(
        manager.set_forced_chatgpt_workspace_id(Some(Vec::new())),
        Err(AuthPolicyError::RevisionExhausted)
    );
    assert_eq!(manager.load_auth().await, None);
    // This slice changes policy decisions, not the separate cached-credential ownership API.
    assert_eq!(manager.auth_cached(), Some(expected));
}

#[tokio::test]
async fn unavailable_policy_blocks_external_permanent_failure_cached_return() {
    let expected = CodexAuth::from_api_key("fixture-external-key");
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    manager
        .set_external_auth(Arc::new(StaticExternal(expected.clone())))
        .await
        .unwrap();
    manager.record_permanent_refresh_failure_if_unchanged(
        &expected,
        &codex_protocol::auth::RefreshTokenFailedError::new(
            codex_protocol::auth::RefreshTokenFailedReason::Other,
            "fixture permanent refresh failure".to_string(),
        ),
    );
    assert!(manager.refresh_failure_for_auth(&expected).is_some());
    assert_eq!(manager.load_auth().await, Some(expected.clone()));
    manager.workspace_policy.state.lock().unwrap().epoch = u64::MAX;
    assert_eq!(
        manager.set_forced_chatgpt_workspace_id(Some(Vec::new())),
        Err(AuthPolicyError::RevisionExhausted)
    );
    assert_eq!(manager.load_auth().await, None);
    assert_eq!(manager.auth_cached(), Some(expected));
}
