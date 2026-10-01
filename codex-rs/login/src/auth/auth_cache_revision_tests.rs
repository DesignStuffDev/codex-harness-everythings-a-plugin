use super::HeldExternal;
use super::Reply;
use super::fixture;
use super::header_auth;
use crate::auth::AuthManager;
use crate::auth::CodexAuth;
use crate::auth::manager::ReloadOutcome;
use base64::Engine;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

#[tokio::test]
async fn both_reload_paths_reject_replacement_and_aba_without_policy_change() {
    for guarded in [false, true] {
        for aba in [false, true] {
            let initial = header_auth("Bearer initial");
            let (manager, external) =
                fixture(initial.clone(), Reply::Auth(header_auth("Bearer stale"))).await;
            let policy = manager.auth_policy_snapshot().unwrap().stamp();
            let task_manager = manager.clone();
            let task = tokio::spawn(async move {
                if guarded {
                    matches!(
                        task_manager
                            .reload_if_account_id_matches(Some("account-a"))
                            .await,
                        ReloadOutcome::Skipped
                    )
                } else {
                    !task_manager.reload().await
                }
            });
            tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
                .await
                .unwrap();
            let newer = header_auth("Bearer newer");
            manager.set_cached_auth(Some(newer.clone()));
            let expected = if aba {
                manager.set_cached_auth(Some(initial.clone()));
                initial
            } else {
                newer
            };
            let changes = manager.auth_change_state_receiver();
            let before = *changes.borrow();
            external.release.add_permits(/*n*/ 1);
            let rejected = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(manager.validate_policy_stamp(&policy), Ok(()));
            assert_eq!(
                (rejected, manager.auth_cached(), *changes.borrow()),
                (true, Some(expected), before)
            );
            assert!(!changes.has_changed().unwrap());
            assert_eq!(external.calls.load(Ordering::SeqCst), 2);
        }
    }
}

#[tokio::test]
async fn cleared_external_credentials_are_not_restored_by_held_load() {
    let (manager, external) = fixture(
        header_auth("Bearer initial"),
        Reply::Auth(header_auth("Bearer stale")),
    )
    .await;
    let task_manager = manager.clone();
    let task = tokio::spawn(async move { task_manager.reload().await });
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
        .await
        .unwrap();
    manager.clear_external_auth();
    assert_eq!(manager.auth_cached(), None);
    external.release.add_permits(/*n*/ 1);
    assert!(
        !tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap()
    );
    assert_eq!(manager.auth_cached(), None);
    assert!(!manager.has_external_auth());
}

fn same_owner_token(token: &str) -> CodexAuth {
    let claims = serde_json::json!({
        "jti": token,
        "https://api.openai.com/auth": {"chatgpt_user_id": "user-a"},
    });
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
    CodexAuth::from_external_chatgpt_tokens(
        &format!("header.{payload}.signature"),
        "account-a",
        /*chatgpt_plan_type*/ None,
    )
    .unwrap()
}

#[tokio::test]
async fn same_owner_token_change_rejects_an_older_reload_candidate() {
    let home = tempfile::tempdir().unwrap();
    let initial = same_owner_token("initial");
    let manager =
        AuthManager::from_auth_for_testing_with_home(initial.clone(), home.path().to_path_buf());
    let external = Arc::new(HeldExternal {
        initial,
        reply: Reply::Auth(same_owner_token("stale")),
        calls: AtomicUsize::new(/*v*/ 0),
        entered: Notify::new(),
        release: Semaphore::new(/*permits*/ 0),
    });
    manager.set_external_auth(external.clone()).await.unwrap();
    let before = *manager.auth_change_state_receiver().borrow();
    let task_manager = manager.clone();
    let task = tokio::spawn(async move {
        task_manager
            .reload_if_account_id_matches(Some("account-a"))
            .await
    });
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
        .await
        .unwrap();
    let expected = same_owner_token("newer");
    manager.set_cached_auth(Some(expected.clone()));
    let after = *manager.auth_change_state_receiver().borrow();
    assert_eq!(after.generation, before.generation + 1);
    assert_eq!(after.owner_generation, before.owner_generation);
    external.release.add_permits(/*n*/ 1);
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap(),
        ReloadOutcome::Skipped
    ));
    assert_eq!(
        manager.auth_cached().unwrap().get_token().unwrap(),
        expected.get_token().unwrap()
    );
    assert_eq!(*manager.auth_change_state_receiver().borrow(), after);
}

#[tokio::test]
async fn actual_native_loaded_value_or_absence_requires_unchanged_cache() {
    for has_loaded_auth in [false, true] {
        for changed in [false, true] {
            let home = tempfile::tempdir().unwrap();
            if has_loaded_auth {
                crate::auth::login_with_api_key(
                    home.path(),
                    "loaded",
                    crate::AuthCredentialsStoreMode::Ephemeral,
                    crate::AuthKeyringBackendKind::default(),
                )
                .unwrap();
            }
            let current = CodexAuth::from_api_key("cached");
            let mut manager = AuthManager::from_auth_for_testing_with_home(
                current.clone(),
                home.path().to_path_buf(),
            );
            // Disable unrelated access-token environment loading without mutating process state.
            Arc::get_mut(&mut manager).unwrap().forced_login_method =
                Some(codex_protocol::config_types::ForcedLoginMethod::Api);
            let policy = manager.auth_policy_snapshot().unwrap().stamp();
            let loaded = manager.load_auth().await.unwrap();
            assert_eq!(
                loaded.captured_auth().map(|auth| auth.get_token().unwrap()),
                has_loaded_auth.then(|| "loaded".to_string())
            );
            manager.set_cached_auth(Some(if changed {
                CodexAuth::from_api_key("newer")
            } else {
                current
            }));
            assert_eq!(manager.validate_policy_stamp(&policy), Ok(()));
            let result = manager.commit_auth_load(loaded);
            assert_eq!(result.is_ok(), !changed);
            let expected = if changed {
                Some("newer")
            } else if has_loaded_auth {
                Some("loaded")
            } else {
                None
            };
            assert_eq!(
                manager.auth_cached().map(|auth| auth.get_token().unwrap()),
                expected.map(str::to_string)
            );
        }
    }
}

#[tokio::test]
async fn failure_from_before_aba_does_not_poison_restored_credentials() {
    let initial = header_auth("Bearer initial");
    let (manager, external) = fixture(initial.clone(), Reply::Permanent).await;
    let task_manager = manager.clone();
    let task = tokio::spawn(async move { task_manager.reload().await });
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), external.entered.notified())
        .await
        .unwrap();
    manager.set_cached_auth(Some(header_auth("Bearer other")));
    manager.set_cached_auth(Some(initial.clone()));
    external.release.add_permits(/*n*/ 1);
    assert!(
        !tokio::time::timeout(Duration::from_secs(/*secs*/ 5), task)
            .await
            .unwrap()
            .unwrap()
    );
    assert_eq!(manager.auth_cached(), Some(initial.clone()));
    assert!(manager.refresh_failure_for_auth(&initial).is_none());
}

#[tokio::test]
async fn loaded_candidate_cannot_publish_into_another_manager() {
    let home = tempfile::tempdir().unwrap();
    crate::auth::login_with_api_key(
        home.path(),
        "loaded",
        crate::AuthCredentialsStoreMode::Ephemeral,
        crate::AuthKeyringBackendKind::default(),
    )
    .unwrap();
    let first = AuthManager::from_auth_for_testing_with_home(
        CodexAuth::from_api_key("same"),
        home.path().to_path_buf(),
    );
    let other = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("same"));
    let loaded = first.load_auth().await.unwrap();
    assert!(other.commit_auth_load(loaded).is_err());
    assert_eq!(other.auth_cached().unwrap().get_token().unwrap(), "same");
}
