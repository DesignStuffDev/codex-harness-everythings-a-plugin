use super::AuthManager;
use super::CodexAuth;
use super::ExternalAuth;
use super::ExternalAuthFuture;
use super::ExternalAuthRefreshContext;
use super::RefreshTokenError;
use super::RefreshTokenFailedError;
use super::RefreshTokenFailedReason;
use crate::auth::AuthHeaders;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

struct HeldRefresh {
    auth: CodexAuth,
    calls: AtomicUsize,
    entered: Notify,
    release: Semaphore,
}
impl ExternalAuth for HeldRefresh {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async { Ok(self.auth.clone()) })
    }
    fn refresh(&self, context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async move {
            assert_eq!(context.previous_account_id.as_deref(), Some("account-a"));
            self.calls.fetch_add(/*val*/ 1, Ordering::SeqCst);
            self.entered.notify_one();
            self.release.acquire().await.unwrap().forget();
            Err(std::io::Error::other("held permanent refresh failure"))
        })
    }
    fn classify_error(&self, error: std::io::Error) -> RefreshTokenError {
        RefreshTokenError::Permanent(RefreshTokenFailedError::new(
            RefreshTokenFailedReason::Other,
            error.to_string(),
        ))
    }
}
fn auth(token: &str) -> CodexAuth {
    let mut headers = http::HeaderMap::new();
    headers.insert("authorization", http::HeaderValue::from_str(token).unwrap());
    headers.insert(
        "chatgpt-account-id",
        http::HeaderValue::from_static("account-a"),
    );
    CodexAuth::Headers(AuthHeaders::new(headers))
}
fn provider(auth: CodexAuth) -> Arc<HeldRefresh> {
    Arc::new(HeldRefresh {
        auth,
        calls: AtomicUsize::new(/*v*/ 0),
        entered: Notify::new(),
        release: Semaphore::new(/*permits*/ 0),
    })
}
fn expected_failure() -> RefreshTokenFailedError {
    RefreshTokenFailedError::new(
        RefreshTokenFailedReason::Other,
        "held permanent refresh failure",
    )
}
#[derive(Clone, Copy)]
enum RefreshPath {
    Guarded,
    Direct,
}
async fn refresh(manager: &AuthManager, path: RefreshPath) -> Result<(), RefreshTokenError> {
    match path {
        RefreshPath::Guarded => manager.refresh_token().await,
        RefreshPath::Direct => manager.refresh_token_from_authority().await,
    }
}
fn assert_original_error(result: Result<(), RefreshTokenError>) {
    let Err(RefreshTokenError::Permanent(error)) = result else {
        panic!("unexpected refresh result: {result:?}");
    };
    assert_eq!(error, expected_failure());
}
#[derive(Clone, Copy)]
enum SourceChange {
    Unchanged,
    Replace,
    SameArcAba,
}

#[tokio::test]
async fn both_public_refresh_paths_fence_old_source_failure_and_preserve_current_fast_path() {
    for path in [RefreshPath::Guarded, RefreshPath::Direct] {
        for change in [
            SourceChange::Unchanged,
            SourceChange::Replace,
            SourceChange::SameArcAba,
        ] {
            let initial = auth("Bearer initial");
            let original = provider(initial.clone());
            let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
            manager.set_external_auth(original.clone()).await.unwrap();
            let revision = manager.inner.read().unwrap().revision.clone();
            let changes = manager.auth_change_state_receiver();
            let before = *changes.borrow();
            tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
                let alter = async {
                    original.entered.notified().await;
                    let selected = match change {
                        SourceChange::Unchanged => original.clone(),
                        SourceChange::Replace | SourceChange::SameArcAba => {
                            let replacement = provider(initial.clone());
                            manager
                                .set_external_auth(replacement.clone())
                                .await
                                .unwrap();
                            match change {
                                SourceChange::Replace => replacement,
                                SourceChange::SameArcAba => {
                                    manager.set_external_auth(original.clone()).await.unwrap();
                                    original.clone()
                                }
                                SourceChange::Unchanged => unreachable!(),
                            }
                        }
                    };
                    assert_eq!(manager.inner.read().unwrap().revision, revision);
                    original.release.add_permits(/*n*/ 1);
                    selected
                };
                let (result, selected) = tokio::join!(refresh(&manager, path), alter);
                assert_original_error(result);
                assert_eq!(manager.auth_cached(), Some(initial.clone()));
                assert_eq!(
                    manager.refresh_failure_for_auth(&initial),
                    matches!(change, SourceChange::Unchanged).then(expected_failure)
                );
                let calls = selected.calls.load(Ordering::SeqCst);
                if !matches!(change, SourceChange::Unchanged) {
                    selected.release.add_permits(/*n*/ 1);
                }
                assert_original_error(refresh(&manager, path).await);
                assert_eq!(
                    selected.calls.load(Ordering::SeqCst),
                    calls + usize::from(!matches!(change, SourceChange::Unchanged))
                );
                assert_eq!(
                    manager.refresh_failure_for_auth(&initial),
                    Some(expected_failure())
                );
            })
            .await
            .unwrap();
            assert_eq!(*changes.borrow(), before);
            assert!(!changes.has_changed().unwrap());
        }
    }
}

enum OwnerChange {
    CredentialAba,
    PolicyAba,
}
#[tokio::test]
async fn refresh_failure_requires_original_cache_and_policy_even_with_same_source() {
    for path in [RefreshPath::Guarded, RefreshPath::Direct] {
        for change in [OwnerChange::CredentialAba, OwnerChange::PolicyAba] {
            let initial = auth("Bearer initial");
            let original = provider(initial.clone());
            let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
            manager.set_external_auth(original.clone()).await.unwrap();
            tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
                let alter = async {
                    original.entered.notified().await;
                    match change {
                        OwnerChange::CredentialAba => {
                            manager.set_cached_auth(Some(auth("Bearer intermediate")));
                            manager.set_cached_auth(Some(initial.clone()));
                        }
                        OwnerChange::PolicyAba => {
                            manager
                                .set_forced_chatgpt_workspace_id(Some(Vec::new()))
                                .unwrap();
                            manager
                                .set_forced_chatgpt_workspace_id(/*workspace_id*/ None)
                                .unwrap();
                        }
                    }
                    original.release.add_permits(/*n*/ 1);
                };
                let (result, ()) = tokio::join!(refresh(&manager, path), alter);
                assert_original_error(result);
            })
            .await
            .unwrap();
            assert_eq!(manager.auth_cached(), Some(initial.clone()));
            assert_eq!(manager.refresh_failure_for_auth(&initial), None);
        }
    }
}

#[tokio::test]
async fn canceled_public_refresh_releases_admission_without_caching_a_failure() {
    let initial = auth("Bearer initial");
    let original = provider(initial.clone());
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    manager.set_external_auth(original.clone()).await.unwrap();
    let mut operation = Box::pin(manager.refresh_token_from_authority());
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        tokio::select! {
            _ = original.entered.notified() => {}
            _ = &mut operation => panic!("refresh completed before held callback release"),
        }
    })
    .await
    .unwrap();
    drop(operation);
    assert_eq!(manager.refresh_failure_for_auth(&initial), None);
    original.release.add_permits(/*n*/ 1);
    assert_original_error(
        tokio::time::timeout(
            Duration::from_secs(/*secs*/ 5),
            manager.refresh_token_from_authority(),
        )
        .await
        .unwrap(),
    );
    assert_eq!(original.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        manager.refresh_failure_for_auth(&initial),
        Some(expected_failure())
    );
}

#[tokio::test]
async fn same_home_managers_do_not_share_refresh_failure_authority() {
    let home = tempfile::tempdir().unwrap();
    let initial = auth("Bearer shared");
    let first =
        AuthManager::from_auth_for_testing_with_home(initial.clone(), home.path().to_path_buf());
    let second =
        AuthManager::from_auth_for_testing_with_home(initial.clone(), home.path().to_path_buf());
    let first_source = provider(initial.clone());
    first.set_external_auth(first_source.clone()).await.unwrap();
    second
        .set_external_auth(provider(initial.clone()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        let alter_sibling = async {
            first_source.entered.notified().await;
            second
                .set_external_auth(provider(initial.clone()))
                .await
                .unwrap();
            first_source.release.add_permits(/*n*/ 1);
        };
        let (result, ()) = tokio::join!(first.refresh_token_from_authority(), alter_sibling);
        assert_original_error(result);
    })
    .await
    .unwrap();
    assert_eq!(
        first.refresh_failure_for_auth(&initial),
        Some(expected_failure())
    );
    assert_eq!(second.refresh_failure_for_auth(&initial), None);
    assert_eq!(
        (first.auth_cached(), second.auth_cached()),
        (Some(initial.clone()), Some(initial))
    );
    // Header credentials do not persist here: this proves per-manager failure isolation only.
}
