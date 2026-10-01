use super::AuthManager;
use super::CodexAuth;
use super::ExternalAuth;
use super::ExternalAuthFuture;
use super::ExternalAuthRefreshContext;
use super::RefreshTokenError;
use super::RefreshTokenFailedError;
use super::RefreshTokenFailedReason;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;

#[derive(Debug, PartialEq)]
struct Retirement {
    owner_unlocked: bool,
    // CodexAuth equality compares API-key modes, so retain the exact fixture credential.
    api_key: Option<String>,
    failure: Option<RefreshTokenFailedReason>,
}
type RetirementObserver = (Weak<AuthManager>, Arc<Mutex<Option<Retirement>>>);

struct Provider {
    auth: CodexAuth,
    observer: Option<RetirementObserver>,
}
impl ExternalAuth for Provider {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async { Ok(self.auth.clone()) })
    }
    fn refresh(&self, _context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        self.resolve()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let Some((owner, observed)) = &self.observer else {
            return;
        };
        let Some(manager) = owner.upgrade() else {
            return;
        };
        // Never block or panic in Drop; the test inspects the recorded real retirement state.
        let state = match manager.inner.try_write() {
            Ok(cached) => Retirement {
                owner_unlocked: true,
                api_key: cached
                    .auth
                    .as_ref()
                    .and_then(CodexAuth::api_key)
                    .map(str::to_owned),
                failure: cached
                    .permanent_refresh_failure
                    .as_ref()
                    .map(|failure| failure.error.reason),
            },
            Err(_) => Retirement {
                owner_unlocked: false,
                api_key: None,
                failure: None,
            },
        };
        *observed.lock().unwrap() = Some(state);
    }
}
fn provider(auth: CodexAuth) -> Arc<dyn ExternalAuth> {
    Arc::new(Provider {
        auth,
        observer: None,
    })
}
fn failure() -> RefreshTokenFailedError {
    RefreshTokenFailedError::new(RefreshTokenFailedReason::Other, "previous source failed")
}
async fn observed_provider(
    manager: &Arc<AuthManager>,
    auth: CodexAuth,
) -> Arc<Mutex<Option<Retirement>>> {
    let observed = Arc::new(Mutex::new(None));
    manager
        .set_external_auth(Arc::new(Provider {
            auth,
            observer: Some((Arc::downgrade(manager), observed.clone())),
        }))
        .await
        .unwrap();
    observed
}

#[tokio::test]
async fn replacement_retires_provider_after_cache_and_failure_publication() {
    for same_credentials in [false, true] {
        let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
        let initial = CodexAuth::from_api_key("initial");
        let observed = observed_provider(&manager, initial.clone()).await;
        manager.record_permanent_refresh_failure_if_unchanged(&initial, &failure());
        let changes = manager.auth_change_state_receiver();
        let before = *changes.borrow();
        let replacement = if same_credentials {
            initial
        } else {
            CodexAuth::from_api_key("replacement")
        };
        manager
            .set_external_auth(provider(replacement.clone()))
            .await
            .unwrap();
        assert_eq!(
            *observed.lock().unwrap(),
            Some(Retirement {
                owner_unlocked: true,
                api_key: replacement.api_key().map(str::to_owned),
                failure: None,
            })
        );
        assert_eq!(
            manager.auth_cached().as_ref().and_then(CodexAuth::api_key),
            replacement.api_key(),
        );
        assert_eq!(manager.refresh_failure_for_auth(&replacement), None);
        assert!(manager.has_external_auth());
        if same_credentials {
            assert_eq!(*changes.borrow(), before);
            assert!(!changes.has_changed().unwrap());
        } else {
            assert_eq!(changes.borrow().generation, before.generation + 1);
        }
    }
}

#[tokio::test]
async fn clear_retires_provider_after_clearing_its_cache_and_failure() {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let initial = CodexAuth::from_api_key("initial");
    let observed = observed_provider(&manager, initial.clone()).await;
    manager.record_permanent_refresh_failure_if_unchanged(&initial, &failure());
    manager.clear_external_auth();
    assert_eq!(
        *observed.lock().unwrap(),
        Some(Retirement {
            owner_unlocked: true,
            api_key: None,
            failure: None,
        })
    );
    assert_eq!(manager.auth_cached(), None);
    assert_eq!(manager.refresh_failure_for_auth(&initial), None);
    assert!(!manager.has_external_auth());
}

#[tokio::test]
async fn preparation_failure_preserves_the_previous_source_cache_and_failure() {
    let initial = CodexAuth::from_api_key("initial");
    let manager = AuthManager::from_auth_for_testing(initial.clone());
    let previous = provider(initial.clone());
    manager.set_external_auth(previous.clone()).await.unwrap();
    manager.record_permanent_refresh_failure_if_unchanged(&initial, &failure());
    let changes = manager.auth_change_state_receiver();
    let before = *changes.borrow();
    // Exercise actual missing external-token state preparation, without poisoning shared storage.
    let malformed = CodexAuth::from_external_chatgpt_tokens(
        "header.e30.signature",
        "account-a",
        /*chatgpt_plan_type*/ None,
    )
    .unwrap();
    let CodexAuth::ChatgptAuthTokens(tokens) = &malformed else {
        unreachable!()
    };
    *tokens.state.auth_dot_json.lock().unwrap() = None;
    let error = manager
        .set_external_auth(provider(malformed))
        .await
        .unwrap_err();
    assert!(matches!(&error, RefreshTokenError::Transient(_)));
    assert_eq!(
        error.to_string(),
        "external ChatGPT auth tokens are missing auth state"
    );
    let current = manager.external_auth_provider().unwrap();
    assert!(Arc::ptr_eq(&current, &previous));
    assert_eq!(
        manager.auth_cached().as_ref().and_then(CodexAuth::api_key),
        initial.api_key(),
    );
    assert_eq!(manager.refresh_failure_for_auth(&initial), Some(failure()));
    assert_eq!(*changes.borrow(), before);
    assert!(!changes.has_changed().unwrap());
}

#[test]
fn clear_without_external_source_preserves_native_cache_and_failure() {
    let initial = CodexAuth::from_api_key("native");
    let manager = AuthManager::from_auth_for_testing(initial.clone());
    manager.record_permanent_refresh_failure_if_unchanged(&initial, &failure());
    let changes = manager.auth_change_state_receiver();
    let before = *changes.borrow();
    manager.clear_external_auth();
    assert_eq!(
        manager.auth_cached().as_ref().and_then(CodexAuth::api_key),
        initial.api_key(),
    );
    assert_eq!(manager.refresh_failure_for_auth(&initial), Some(failure()));
    assert_eq!(*changes.borrow(), before);
    assert!(!changes.has_changed().unwrap());
}

#[tokio::test]
async fn workload_owned_source_cannot_be_replaced_or_cleared() {
    let initial = CodexAuth::from_api_key("workload");
    let mut manager = AuthManager::from_auth_for_testing(initial.clone());
    Arc::get_mut(&mut manager)
        .unwrap()
        .workload_identity_selected = true;
    let selected = provider(initial.clone());
    manager
        .install_external_auth(selected.clone())
        .await
        .unwrap();
    manager.clear_external_auth();
    let error = manager
        .set_external_auth(provider(CodexAuth::from_api_key("replacement")))
        .await
        .unwrap_err();
    assert!(matches!(error, RefreshTokenError::Permanent(_)));
    assert!(Arc::ptr_eq(
        &manager.external_auth_provider().unwrap(),
        &selected
    ));
    assert_eq!(
        manager.auth_cached().as_ref().and_then(CodexAuth::api_key),
        initial.api_key(),
    );
}

#[tokio::test]
async fn poisoned_credential_owner_cannot_expose_an_external_source() {
    let initial = CodexAuth::from_api_key("initial");
    let manager = AuthManager::from_auth_for_testing(initial.clone());
    manager.set_external_auth(provider(initial)).await.unwrap();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _owner = manager.inner.write().unwrap();
        panic!("poison only this manager's native owner");
    }));
    assert!(poisoned.is_err());
    assert!(!manager.has_external_auth());
    assert!(manager.external_auth_provider().is_none());
    assert_eq!(manager.auth().await, None);
    let error = manager
        .set_external_auth(provider(CodexAuth::from_api_key("replacement")))
        .await
        .unwrap_err();
    assert!(matches!(error, RefreshTokenError::Transient(_)));
    manager.clear_external_auth();
    assert_eq!(manager.auth_cached(), None);
}
