use super::AuthLoadError;
use super::Reply;
use super::header_auth;
use crate::auth::AuthManager;
use crate::auth::CodexAuth;
use crate::auth::ExternalAuth;
use crate::auth::ExternalAuthFuture;
use crate::auth::ExternalAuthRefreshContext;
use crate::auth::RefreshTokenError;
use crate::auth::RefreshTokenFailedError;
use crate::auth::manager::ReloadOutcome;
use codex_protocol::auth::RefreshTokenFailedReason;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

struct ReinstallableSource {
    initial: CodexAuth,
    reply: Reply,
    calls: AtomicUsize,
    entered: Notify,
    release: Semaphore,
}
impl ExternalAuth for ReinstallableSource {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async move {
            // Hold one real reload. Reinstalling the same Arc must not wait on that reload.
            if self.calls.fetch_add(/*val*/ 1, Ordering::SeqCst) != 1 {
                return Ok(self.initial.clone());
            }
            self.entered.notify_one();
            self.release.acquire().await.unwrap().forget();
            match &self.reply {
                Reply::Auth(auth) => Ok(auth.clone()),
                Reply::Permanent | Reply::Transient => {
                    Err(std::io::Error::other("held source failure"))
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
fn source(initial: CodexAuth, reply: Reply) -> Arc<ReinstallableSource> {
    Arc::new(ReinstallableSource {
        initial,
        reply,
        calls: AtomicUsize::new(/*v*/ 0),
        entered: Notify::new(),
        release: Semaphore::new(/*permits*/ 0),
    })
}
enum SourceTransition {
    Replacement,
    SameArcAba,
}

async fn replace_source(
    manager: &AuthManager,
    original: &Arc<ReinstallableSource>,
    transition: SourceTransition,
) -> Arc<dyn ExternalAuth> {
    let replacement = source(
        original.initial.clone(),
        Reply::Auth(original.initial.clone()),
    );
    manager
        .set_external_auth(replacement.clone())
        .await
        .unwrap();
    match transition {
        SourceTransition::Replacement => replacement,
        SourceTransition::SameArcAba => {
            manager.set_external_auth(original.clone()).await.unwrap();
            original.clone()
        }
    }
}

#[tokio::test]
async fn both_reload_paths_reject_equal_credential_source_replacement_and_aba() {
    for guarded in [false, true] {
        for transition in [SourceTransition::Replacement, SourceTransition::SameArcAba] {
            let initial = header_auth("Bearer shared");
            let original = source(initial.clone(), Reply::Auth(header_auth("Bearer stale")));
            let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
            manager.set_external_auth(original.clone()).await.unwrap();
            let revision = manager.inner.read().unwrap().revision.clone();
            let changes = manager.auth_change_state_receiver();
            let legacy = manager.auth_change_receiver();
            let before = *changes.borrow();
            tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
                let reload = async {
                    if guarded {
                        matches!(
                            manager
                                .reload_if_account_id_matches(Some("account-a"))
                                .await,
                            ReloadOutcome::Skipped
                        )
                    } else {
                        !manager.reload().await
                    }
                };
                let replacement = async {
                    original.entered.notified().await;
                    let selected = replace_source(&manager, &original, transition).await;
                    assert_eq!(manager.inner.read().unwrap().revision, revision);
                    original.release.add_permits(/*n*/ 1);
                    selected
                };
                let (rejected, selected) = tokio::join!(reload, replacement);
                assert!(Arc::ptr_eq(
                    &manager.external_auth_provider().unwrap(),
                    &selected
                ));
                assert_eq!(
                    (rejected, manager.auth_cached(), *changes.borrow()),
                    (true, Some(initial.clone()), before)
                );
            })
            .await
            .unwrap();
            assert_eq!(manager.refresh_failure_for_auth(&initial), None);
            assert!(!changes.has_changed().unwrap());
            assert!(!legacy.has_changed().unwrap());
        }
    }
}

#[tokio::test]
async fn old_source_failure_cannot_poison_equal_replacement_or_reinstalled_source() {
    for transition in [SourceTransition::Replacement, SourceTransition::SameArcAba] {
        let initial = header_auth("Bearer shared");
        let original = source(initial.clone(), Reply::Permanent);
        let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
        manager.set_external_auth(original.clone()).await.unwrap();
        let revision = manager.inner.read().unwrap().revision.clone();
        let changes = manager.auth_change_state_receiver();
        let before = *changes.borrow();
        tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
            let replacement = async {
                original.entered.notified().await;
                replace_source(&manager, &original, transition).await;
                assert_eq!(manager.inner.read().unwrap().revision, revision);
                original.release.add_permits(/*n*/ 1);
            };
            let (changed, ()) = tokio::join!(manager.reload(), replacement);
            assert_eq!(
                (
                    changed,
                    manager.auth_cached(),
                    manager.refresh_failure_for_auth(&initial)
                ),
                (false, Some(initial), None),
            );
        })
        .await
        .unwrap();
        assert_eq!(*changes.borrow(), before);
        assert!(!changes.has_changed().unwrap());
    }
}

#[tokio::test]
async fn native_value_and_absence_cannot_overwrite_new_source_with_same_credentials() {
    for present in [false, true] {
        let home = tempfile::tempdir().unwrap();
        if present {
            crate::auth::login_with_api_key(
                home.path(),
                "native-loaded",
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
        Arc::get_mut(&mut manager).unwrap().forced_login_method =
            Some(codex_protocol::config_types::ForcedLoginMethod::Api);
        let loaded = manager.load_auth().await.unwrap();
        assert_eq!(
            loaded.captured_auth().and_then(CodexAuth::api_key),
            present.then_some("native-loaded")
        );
        let revision = manager.inner.read().unwrap().revision.clone();
        let selected: Arc<dyn ExternalAuth> = source(current, Reply::Transient);
        manager.set_external_auth(selected.clone()).await.unwrap();
        assert_eq!(manager.inner.read().unwrap().revision, revision);
        let changes = manager.auth_change_state_receiver();
        assert!(matches!(
            manager.commit_auth_load(loaded),
            Err(AuthLoadError::SourceChanged)
        ));
        assert_eq!(manager.auth_cached().unwrap().api_key(), Some("cached"));
        assert!(Arc::ptr_eq(
            &manager.external_auth_provider().unwrap(),
            &selected
        ));
        assert!(!changes.has_changed().unwrap());
    }
}

#[tokio::test]
async fn pending_install_intent_does_not_revoke_current_source_reload() {
    let initial = header_auth("Bearer current");
    let original = source(initial.clone(), Reply::Auth(initial.clone()));
    let replacement_auth = header_auth("Bearer replacement");
    let pending = source(
        replacement_auth.clone(),
        Reply::Auth(replacement_auth.clone()),
    );
    // Hold its installation resolve rather than its second resolve.
    pending.calls.store(/*val*/ 1, Ordering::SeqCst);
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    manager.set_external_auth(original.clone()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        let current_reload = async {
            pending.entered.notified().await;
            original.release.add_permits(/*n*/ 1);
            assert!(matches!(
                manager
                    .reload_if_account_id_matches(Some("account-a"))
                    .await,
                ReloadOutcome::ReloadedNoChange,
            ));
            assert_eq!(manager.auth_cached(), Some(initial));
            pending.release.add_permits(/*n*/ 1);
        };
        let (installed, ()) =
            tokio::join!(manager.set_external_auth(pending.clone()), current_reload);
        installed.unwrap();
    })
    .await
    .unwrap();
    assert_eq!(manager.auth_cached(), Some(replacement_auth));
}

#[tokio::test]
async fn canceled_reload_does_not_publish_after_source_replacement() {
    let initial = header_auth("Bearer shared");
    let original = source(initial.clone(), Reply::Auth(header_auth("Bearer stale")));
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    manager.set_external_auth(original.clone()).await.unwrap();
    let mut reload = Box::pin(manager.reload());
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        tokio::select! {
            _ = original.entered.notified() => {}
            _ = &mut reload => panic!("reload completed before its held resolution"),
        }
    })
    .await
    .unwrap();
    drop(reload);
    let selected = replace_source(&manager, &original, SourceTransition::Replacement).await;
    original.release.add_permits(/*n*/ 1);
    assert!(Arc::ptr_eq(
        &manager.external_auth_provider().unwrap(),
        &selected
    ));
    assert_eq!(manager.auth_cached(), Some(initial));
    assert_eq!(original.calls.load(Ordering::SeqCst), 2);
    assert_eq!(original.release.available_permits(), 1);
}
