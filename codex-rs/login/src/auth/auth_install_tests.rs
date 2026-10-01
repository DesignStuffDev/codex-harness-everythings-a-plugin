use super::AuthManager;
use super::CodexAuth;
use super::ExternalAuth;
use super::ExternalAuthFuture;
use super::ExternalAuthRefreshContext;
use super::RefreshTokenError;
use super::RefreshTokenFailedError;
use super::RefreshTokenFailedReason;
use pretty_assertions::assert_eq;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::Semaphore;

#[derive(Clone, Copy)]
enum Reply {
    Key(&'static str),
    Failure,
}
enum Timing {
    Ready,
    Held,
}
struct Provider {
    reply: Reply,
    timing: Timing,
    entered: Notify,
    release: Semaphore,
    calls: AtomicUsize,
    classified: AtomicUsize,
}
impl ExternalAuth for Provider {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async move {
            let call = self.calls.fetch_add(/*val*/ 1, Ordering::SeqCst);
            if call == 0 && matches!(self.timing, Timing::Held) {
                self.entered.notify_one();
                self.release.acquire().await.unwrap().forget();
            }
            match self.reply {
                Reply::Key(key) => Ok(CodexAuth::from_api_key(key)),
                Reply::Failure => Err(std::io::Error::other("provider rejected install")),
            }
        })
    }
    fn refresh(&self, _context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async { Ok(CodexAuth::from_api_key("refreshed")) })
    }
    fn classify_error(&self, error: std::io::Error) -> RefreshTokenError {
        self.classified.fetch_add(/*val*/ 1, Ordering::SeqCst);
        RefreshTokenError::Permanent(RefreshTokenFailedError::new(
            RefreshTokenFailedReason::Revoked,
            error.to_string(),
        ))
    }
}
fn provider(reply: Reply, timing: Timing) -> Arc<Provider> {
    Arc::new(Provider {
        reply,
        timing,
        entered: Notify::new(),
        release: Semaphore::new(/*permits*/ 0),
        calls: AtomicUsize::new(/*v*/ 0),
        classified: AtomicUsize::new(/*v*/ 0),
    })
}
async fn entered(
    install: &mut (impl Future<Output = Result<(), RefreshTokenError>> + Unpin),
    source: &Provider,
) {
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        tokio::select! {
            result = install => panic!("installer completed before release: {result:?}"),
            () = source.entered.notified() => {}
        }
    })
    .await
    .unwrap();
}
async fn finish(
    install: impl Future<Output = Result<(), RefreshTokenError>>,
) -> Result<(), RefreshTokenError> {
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), install)
        .await
        .unwrap()
}
fn assert_owner(manager: &AuthManager, source: &Arc<Provider>, key: &str) {
    let expected: Arc<dyn ExternalAuth> = source.clone();
    assert!(Arc::ptr_eq(
        &manager.external_auth_provider().unwrap(),
        &expected
    ));
    assert_eq!(
        manager.auth_cached().as_ref().and_then(CodexAuth::api_key),
        Some(key)
    );
}
fn previous_failure() -> RefreshTokenFailedError {
    RefreshTokenFailedError::new(RefreshTokenFailedReason::Other, "previous source failed")
}

#[tokio::test]
async fn newer_install_wins_over_held_older_install_even_with_equal_credentials() {
    for winner_key in ["initial", "newer"] {
        let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
        let initial = provider(Reply::Key("initial"), Timing::Ready);
        manager.set_external_auth(initial).await.unwrap();
        let changes = manager.auth_change_state_receiver();
        let before = *changes.borrow();
        let older = provider(Reply::Key("stale"), Timing::Held);
        let mut pending = Box::pin(manager.set_external_auth(older.clone()));
        entered(&mut pending, &older).await;
        let winner = provider(Reply::Key(winner_key), Timing::Ready);
        manager.set_external_auth(winner.clone()).await.unwrap();
        let after = *changes.borrow();
        if winner_key == "initial" {
            assert_eq!(after, before);
            assert!(!changes.has_changed().unwrap());
        }
        older.release.add_permits(/*n*/ 1);
        assert!(matches!(
            finish(pending).await,
            Err(RefreshTokenError::Transient(_))
        ));
        assert_owner(&manager, &winner, winner_key);
        assert_eq!(*changes.borrow(), after);
        assert_eq!(manager.auth().await.unwrap().api_key(), Some(winner_key));
        assert_eq!(winner.calls.load(Ordering::SeqCst), 2);
        assert_eq!(older.classified.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn clear_invalidates_initial_install_and_preserves_native_state() {
    for native in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let auth = native.then(|| CodexAuth::from_api_key("native"));
        let mut manager = AuthManager::from_optional_auth_for_testing(auth.clone());
        let unique = Arc::get_mut(&mut manager).unwrap();
        unique.codex_home = home.path().to_path_buf();
        unique.forced_login_method = Some(codex_protocol::config_types::ForcedLoginMethod::Api);
        if let Some(auth) = &auth {
            manager.record_permanent_refresh_failure_if_unchanged(auth, &previous_failure());
        }
        let changes = manager.auth_change_state_receiver();
        let before = *changes.borrow();
        let candidate = provider(Reply::Key("stale"), Timing::Held);
        let mut pending = Box::pin(manager.set_external_auth(candidate.clone()));
        entered(&mut pending, &candidate).await;
        manager.clear_external_auth();
        candidate.release.add_permits(/*n*/ 1);
        assert!(matches!(
            finish(pending).await,
            Err(RefreshTokenError::Transient(_))
        ));
        assert!(!manager.has_external_auth());
        assert_eq!(
            manager.auth_cached().as_ref().and_then(CodexAuth::api_key),
            native.then_some("native")
        );
        assert_eq!(
            manager.refresh_failure_for_auth(&CodexAuth::from_api_key("native")),
            native.then(previous_failure)
        );
        assert_eq!(*changes.borrow(), before);
        assert!(!changes.has_changed().unwrap());
        assert_eq!(candidate.classified.load(Ordering::SeqCst), 0);
        // Ordinary native loading remains usable after clear(None) invalidates installation.
        crate::auth::login_with_api_key(
            home.path(),
            "loaded",
            crate::AuthCredentialsStoreMode::Ephemeral,
            crate::AuthKeyringBackendKind::default(),
        )
        .unwrap();
        manager.reload().await;
        assert_eq!(manager.auth_cached().unwrap().api_key(), Some("loaded"));
    }
}

#[tokio::test]
async fn failed_or_cancelled_newer_install_never_revives_older_install() {
    for cancel in [false, true] {
        let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
        let active = provider(Reply::Key("active"), Timing::Ready);
        manager.set_external_auth(active.clone()).await.unwrap();
        let auth = CodexAuth::from_api_key("active");
        manager.record_permanent_refresh_failure_if_unchanged(&auth, &previous_failure());
        let changes = manager.auth_change_state_receiver();
        let before = *changes.borrow();
        let older = provider(Reply::Key("stale"), Timing::Held);
        let mut old_pending = Box::pin(manager.set_external_auth(older.clone()));
        entered(&mut old_pending, &older).await;
        let newer = provider(Reply::Failure, Timing::Held);
        let mut new_pending = Box::pin(manager.set_external_auth(newer.clone()));
        entered(&mut new_pending, &newer).await;
        if cancel {
            drop(new_pending);
            assert_eq!(newer.classified.load(Ordering::SeqCst), 0);
        } else {
            newer.release.add_permits(/*n*/ 1);
            let error = finish(new_pending).await.unwrap_err();
            assert!(
                matches!(&error, RefreshTokenError::Permanent(failure) if failure.reason == RefreshTokenFailedReason::Revoked)
            );
            assert_eq!(error.to_string(), "provider rejected install");
            assert_eq!(newer.classified.load(Ordering::SeqCst), 1);
        }
        older.release.add_permits(/*n*/ 1);
        assert!(matches!(
            finish(old_pending).await,
            Err(RefreshTokenError::Transient(_))
        ));
        assert_owner(&manager, &active, "active");
        assert_eq!(
            manager.refresh_failure_for_auth(&auth),
            Some(previous_failure())
        );
        assert_eq!(*changes.borrow(), before);
        assert!(!changes.has_changed().unwrap());
        assert_eq!(older.classified.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn policy_aba_rejects_install_without_provider_classification() {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let active = provider(Reply::Key("active"), Timing::Ready);
    manager.set_external_auth(active.clone()).await.unwrap();
    let candidate = provider(Reply::Key("stale"), Timing::Held);
    let mut pending = Box::pin(manager.set_external_auth(candidate.clone()));
    entered(&mut pending, &candidate).await;
    manager
        .set_forced_chatgpt_workspace_id(Some(vec!["other".to_string()]))
        .unwrap();
    manager
        .set_forced_chatgpt_workspace_id(/*workspace_id*/ None)
        .unwrap();
    candidate.release.add_permits(/*n*/ 1);
    assert!(matches!(
        finish(pending).await,
        Err(RefreshTokenError::Transient(_))
    ));
    assert_owner(&manager, &active, "active");
    assert_eq!(candidate.classified.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cache_change_and_aba_reject_install_and_preserve_current_failure() {
    for aba in [false, true] {
        let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
        let active = provider(Reply::Key("active"), Timing::Ready);
        manager.set_external_auth(active.clone()).await.unwrap();
        let candidate = provider(Reply::Key("stale"), Timing::Held);
        let mut pending = Box::pin(manager.set_external_auth(candidate.clone()));
        entered(&mut pending, &candidate).await;
        // The current provider publishes through the actual authority-refresh path.
        manager.refresh_token_from_authority().await.unwrap();
        let key = if aba { "active" } else { "refreshed" };
        if aba {
            manager.reload().await;
        }
        assert_owner(&manager, &active, key);
        let current = CodexAuth::from_api_key(key);
        manager.record_permanent_refresh_failure_if_unchanged(&current, &previous_failure());
        let changes = manager.auth_change_state_receiver();
        let before = *changes.borrow();
        candidate.release.add_permits(/*n*/ 1);
        assert!(matches!(
            finish(pending).await,
            Err(RefreshTokenError::Transient(_))
        ));
        assert_owner(&manager, &active, key);
        assert_eq!(
            manager.refresh_failure_for_auth(&current),
            Some(previous_failure())
        );
        assert_eq!(*changes.borrow(), before);
        assert!(!changes.has_changed().unwrap());
        assert_eq!(candidate.classified.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn unpolled_install_does_not_supersede_legitimate_current_install() {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let current = provider(Reply::Key("current"), Timing::Held);
    let mut pending = Box::pin(manager.set_external_auth(current.clone()));
    entered(&mut pending, &current).await;
    let unused = provider(Reply::Key("unpolled"), Timing::Ready);
    let unpolled = manager.set_external_auth(unused.clone());
    current.release.add_permits(/*n*/ 1);
    finish(pending).await.unwrap();
    assert_owner(&manager, &current, "current");
    assert_eq!(manager.auth().await.unwrap().api_key(), Some("current"));
    assert_eq!(current.calls.load(Ordering::SeqCst), 2);
    assert_eq!(unused.calls.load(Ordering::SeqCst), 0);
    drop(unpolled);
}

#[tokio::test]
async fn stale_provider_error_retains_classification_without_poisoning_winner() {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    let older = provider(Reply::Failure, Timing::Held);
    let mut pending = Box::pin(manager.set_external_auth(older.clone()));
    entered(&mut pending, &older).await;
    let winner = provider(Reply::Key("winner"), Timing::Ready);
    manager.set_external_auth(winner.clone()).await.unwrap();
    older.release.add_permits(/*n*/ 1);
    let error = finish(pending).await.unwrap_err();
    assert!(
        matches!(&error, RefreshTokenError::Permanent(failure) if failure.reason == RefreshTokenFailedReason::Revoked)
    );
    assert_eq!(error.to_string(), "provider rejected install");
    assert_eq!(older.classified.load(Ordering::SeqCst), 1);
    assert_owner(&manager, &winner, "winner");
    assert_eq!(manager.auth().await.unwrap().api_key(), Some("winner"));
    assert_eq!(
        manager.refresh_failure_for_auth(&CodexAuth::from_api_key("winner")),
        None
    );
}

#[tokio::test]
async fn poisoned_owner_rejects_install_before_resolving_provider() {
    let manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _owner = manager.inner.write().unwrap();
            panic!("poison this owner's admission");
        }))
        .is_err()
    );
    let candidate = provider(Reply::Key("unresolved"), Timing::Ready);
    assert!(matches!(
        manager.set_external_auth(candidate.clone()).await,
        Err(RefreshTokenError::Transient(_))
    ));
    assert_eq!(candidate.calls.load(Ordering::SeqCst), 0);
    assert_eq!(candidate.classified.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn workload_public_noops_do_not_supersede_private_initial_install() {
    let mut manager = AuthManager::from_optional_auth_for_testing(/*auth*/ None);
    Arc::get_mut(&mut manager)
        .unwrap()
        .workload_identity_selected = true;
    let selected = provider(Reply::Key("workload"), Timing::Held);
    let mut pending = Box::pin(manager.install_external_auth(selected.clone()));
    entered(&mut pending, &selected).await;
    manager.clear_external_auth();
    let denied = provider(Reply::Key("denied"), Timing::Ready);
    assert!(matches!(
        manager.set_external_auth(denied.clone()).await,
        Err(RefreshTokenError::Permanent(_))
    ));
    selected.release.add_permits(/*n*/ 1);
    finish(pending).await.unwrap();
    assert_owner(&manager, &selected, "workload");
    assert_eq!(denied.calls.load(Ordering::SeqCst), 0);
}
