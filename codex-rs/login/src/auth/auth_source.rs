//! Paired native source/cache publication. Admission ordering and source fencing are separate.
use super::AuthCredentialsStoreMode;
use super::AuthKeyringBackendKind;
use super::AuthManager;
use super::CodexAuth;
use super::ExternalAuth;
use super::RefreshTokenError;
use super::auth_reload::AuthLoadUpdate;
use super::auth_reload::CachePolicy;
use super::permanent_external_auth_error;
use super::save_auth;
use std::sync::Arc;

/// Owned update forms keep provider installation paired with its prepared credentials.
pub(super) enum AuthOwnerUpdate {
    Cache(AuthLoadUpdate),
    Install {
        provider: Arc<dyn ExternalAuth>,
        auth: CodexAuth,
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
        let auth = self.resolve_external_auth(external_auth.as_ref()).await?;
        self.persist_external_auth(&auth)?;
        self.replace_auth_owner(
            AuthOwnerUpdate::Install {
                provider: external_auth,
                auth,
            },
            CachePolicy::Unfenced,
        )
        .map_err(|error| RefreshTokenError::Transient(std::io::Error::other(error)))?;
        Ok(())
    }

    pub fn clear_external_auth(&self) {
        if self.workload_identity_selected {
            return;
        }
        // The owner tests presence and clears the matching cache within one write guard.
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
