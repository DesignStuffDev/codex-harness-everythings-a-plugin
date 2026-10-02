//! Latest-admitted external installation. Reload/refresh source fencing remains separate.
use super::AuthCredentialsStoreMode;
use super::AuthKeyringBackendKind;
use super::AuthManager;
use super::AuthPolicyStamp;
use super::CachedAuth;
use super::CodexAuth;
use super::ExternalAuth;
use super::RefreshTokenError;
use super::auth_reload::AuthCacheRevision;
use super::auth_reload::AuthLoadError;
use super::auth_reload::AuthLoadUpdate;
use super::auth_reload::CachePolicy;
use super::permanent_external_auth_error;
use super::save_auth;
use super::validate_auth_restrictions;
use std::sync::Arc;

/// Retained identities prevent source/intent ABA without allocating a growing ticket ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AuthSourceRevision(AuthCacheRevision);
impl AuthSourceRevision {
    pub(super) fn new() -> Self {
        Self(AuthCacheRevision::new())
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct InstallIntent(AuthCacheRevision);
impl InstallIntent {
    pub(super) fn new() -> Self {
        Self(AuthCacheRevision::new())
    }
}

pub(super) struct InstallContext {
    pub(super) intent: InstallIntent,
    pub(super) source: AuthSourceRevision,
    pub(super) cache: AuthCacheRevision,
    pub(super) policy: AuthPolicyStamp,
}
impl InstallContext {
    pub(super) fn check_owner(&self, cached: &CachedAuth) -> Result<(), AuthLoadError> {
        if self.intent != cached.install_intent || self.source != cached.source_revision {
            return Err(AuthLoadError::SourceChanged);
        }
        if self.cache != cached.revision {
            return Err(AuthLoadError::CredentialsChanged);
        }
        Ok(())
    }
}
fn install_error(error: AuthLoadError) -> RefreshTokenError {
    RefreshTokenError::Transient(std::io::Error::other(error))
}

/// Owned update forms keep provider installation paired with its prepared credentials.
pub(super) enum AuthOwnerUpdate {
    Cache(AuthLoadUpdate),
    Install {
        provider: Arc<dyn ExternalAuth>,
        auth: CodexAuth,
        context: InstallContext,
    },
    ClearExternal,
}

impl AuthManager {
    pub async fn set_external_auth(
        &self,
        external_auth: Arc<dyn ExternalAuth>,
    ) -> Result<(), RefreshTokenError> {
        if self.workload_identity_selected {
            return Err(permanent_external_auth_error(
                "workload identity auth cannot be replaced at runtime",
            ));
        }
        self.install_external_auth(external_auth).await
    }

    pub(super) async fn install_external_auth(
        &self,
        external_auth: Arc<dyn ExternalAuth>,
    ) -> Result<(), RefreshTokenError> {
        let policy = self
            .auth_policy_snapshot()
            .map_err(AuthLoadError::Policy)
            .map_err(install_error)?;
        let intent = InstallIntent::new();
        let retired_intent;
        let context;
        {
            let mut cached = self
                .inner
                .write()
                .map_err(|_| AuthLoadError::CacheUnavailable)
                .map_err(install_error)?;
            context = InstallContext {
                intent: intent.clone(),
                source: cached.source_revision.clone(),
                cache: cached.revision.clone(),
                policy: policy.stamp(),
            };
            retired_intent = std::mem::replace(&mut cached.install_intent, intent);
        }
        drop(retired_intent);
        // Validating after admission proves the captured policy was current at admission.
        // A failed/cancelled newer attempt never restores the superseded intent.
        self.check_install_context(&context)
            .map_err(install_error)?;
        let auth = external_auth
            .resolve()
            .await
            .map_err(|error| external_auth.classify_error(error))?;
        self.check_install_context(&context)
            .map_err(install_error)?;
        validate_auth_restrictions(
            Some(policy.allowed_login_methods()),
            policy.effective_chatgpt_workspaces(),
            &auth,
        )
        .map_err(|error| external_auth.classify_error(std::io::Error::other(error)))?;
        self.persist_external_auth(&auth)?;
        self.replace_auth_owner(
            AuthOwnerUpdate::Install {
                provider: external_auth,
                auth,
                context,
            },
            CachePolicy::Unfenced,
        )
        .map_err(install_error)?;
        Ok(())
    }

    fn check_install_context(&self, context: &InstallContext) -> Result<(), AuthLoadError> {
        {
            let cached = self
                .inner
                .read()
                .map_err(|_| AuthLoadError::CacheUnavailable)?;
            context.check_owner(&cached)?;
        }
        self.validate_policy_stamp(&context.policy)
            .map_err(AuthLoadError::Policy)
    }

    pub fn clear_external_auth(&self) {
        if self.workload_identity_selected {
            return;
        }
        // Invalidate pending installs even when no external source is active.
        let _ = self.replace_auth_owner(AuthOwnerUpdate::ClearExternal, CachePolicy::Unfenced);
    }

    pub(super) fn external_auth_provider(&self) -> Option<Arc<dyn ExternalAuth>> {
        self.inner
            .read()
            .ok()
            .and_then(|cached| cached.external_auth.clone())
    }

    pub(super) fn commit_external_auth(&self, auth: CodexAuth) -> Result<(), RefreshTokenError> {
        self.persist_external_auth(&auth)?;
        self.set_cached_auth(Some(auth));
        Ok(())
    }

    fn persist_external_auth(&self, auth: &CodexAuth) -> Result<(), RefreshTokenError> {
        if auth.is_external_chatgpt_tokens() {
            let auth_dot_json = auth.get_current_auth_json().ok_or_else(|| {
                RefreshTokenError::Transient(std::io::Error::other(
                    "external ChatGPT auth tokens are missing auth state",
                ))
            })?;
            // This save remains outside the owner lock and is not a conditional persistence fence.
            save_auth(
                &self.codex_home,
                &auth_dot_json,
                AuthCredentialsStoreMode::Ephemeral,
                AuthKeyringBackendKind::default(),
            )
            .map_err(RefreshTokenError::Transient)?;
        }
        Ok(())
    }
}
