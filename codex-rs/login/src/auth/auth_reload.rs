//! Policy-checked in-memory reload publication. Persistent refresh is a separate boundary.
use super::AuthManager;
use super::AuthPolicyError;
use super::AuthPolicyStamp;
use super::AuthScopedRefreshFailure;
use super::CodexAuth;
use super::RefreshTokenFailedError;
use super::same_owner;

#[derive(Debug, thiserror::Error)]
pub(super) enum AuthLoadError {
    #[error(transparent)]
    Policy(AuthPolicyError),
    #[error(transparent)]
    Source(std::io::Error),
    #[error("cached authentication state is unavailable")]
    CacheUnavailable,
}

#[derive(Debug)]
pub(super) struct LoadedAuth {
    pub(super) policy: AuthPolicyStamp,
    pub(super) update: AuthLoadUpdate,
}

#[derive(Debug)]
pub(super) enum AuthLoadUpdate {
    Replace(Option<CodexAuth>),
    Preserve {
        attempted: Option<CodexAuth>,
        failure: Option<RefreshTokenFailedError>,
    },
}
impl LoadedAuth {
    pub(super) fn captured_auth(&self) -> Option<&CodexAuth> {
        match &self.update {
            AuthLoadUpdate::Replace(auth) => auth.as_ref(),
            AuthLoadUpdate::Preserve { attempted, .. } => attempted.as_ref(),
        }
    }
}
// Retains only the native workspace rejection origin through existing io::Result APIs.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(super) struct NativePolicyRejection(pub(super) String);

pub(super) enum ExternalLoadOrigin {
    Resolver,
    Restriction,
}
pub(super) enum CachePolicy<'a> {
    Unfenced,
    Captured(&'a AuthPolicyStamp),
}
pub(super) struct CacheChange {
    pub(super) changed: bool,
    pub(super) credentials_changed: bool,
}

impl AuthManager {
    pub(super) fn commit_auth_load(
        &self,
        loaded: LoadedAuth,
    ) -> Result<CacheChange, AuthLoadError> {
        self.replace_auth_cache(loaded.update, CachePolicy::Captured(&loaded.policy))
    }

    pub(super) fn replace_auth_cache(
        &self,
        mut update: AuthLoadUpdate,
        policy: CachePolicy<'_>,
    ) -> Result<CacheChange, AuthLoadError> {
        // Declared before locks so even unwinding drops trait-backed auth only after lock release.
        let mut retired_auth = None;
        let mut retired_failure = None;
        let mut cached = self
            .inner
            .write()
            .map_err(|_| AuthLoadError::CacheUnavailable)?;
        let (changed, credentials_changed, owner_changed, failure) = match &update {
            AuthLoadUpdate::Replace(auth) => {
                let changed = !Self::auths_equal(cached.auth.as_ref(), auth.as_ref());
                let credentials_changed =
                    !Self::auths_equal_for_refresh(cached.auth.as_ref(), auth.as_ref());
                (
                    changed,
                    credentials_changed,
                    credentials_changed && !same_owner(cached.auth.as_ref(), auth.as_ref()),
                    None,
                )
            }
            AuthLoadUpdate::Preserve { attempted, failure } => {
                let failure =
                    if Self::auths_equal_for_refresh(cached.auth.as_ref(), attempted.as_ref()) {
                        attempted
                            .as_ref()
                            .zip(failure.as_ref())
                            .map(|(auth, error)| AuthScopedRefreshFailure {
                                auth: auth.clone(),
                                error: error.clone(),
                            })
                    } else {
                        None
                    };
                (false, false, false, failure)
            }
        };
        let policy_guard = match policy {
            CachePolicy::Unfenced => None,
            CachePolicy::Captured(stamp) => match self.workspace_policy.lock_current(stamp) {
                Ok(guard) => Some(guard),
                Err(error) => {
                    drop(cached);
                    return Err(AuthLoadError::Policy(error));
                }
            },
        };
        // Only owned swaps occur under policy lock; this is the policy commit linearization point.
        match &mut update {
            AuthLoadUpdate::Replace(auth) => {
                retired_auth = std::mem::replace(&mut cached.auth, auth.take());
                if credentials_changed {
                    retired_failure = cached.permanent_refresh_failure.take();
                }
            }
            AuthLoadUpdate::Preserve { .. } => {
                if let Some(failure) = failure {
                    retired_failure = cached.permanent_refresh_failure.replace(failure);
                }
            }
        }
        drop(policy_guard);
        // Preserve coherent credentials/revisions; no policy lock is held during wakeups.
        if owner_changed {
            self.auth_route_config
                .application_network_policy()
                .invalidate();
        }
        if credentials_changed {
            self.auth_change_state_tx.send_modify(|state| {
                state.generation += 1;
                if owner_changed {
                    state.owner_generation += 1;
                }
            });
            self.auth_change_tx.send_modify(|revision| *revision += 1);
        }
        drop(cached);
        drop((retired_auth, retired_failure));
        tracing::info!("Reloaded auth, changed: {changed}");
        Ok(CacheChange {
            changed,
            credentials_changed,
        })
    }
}

#[cfg(test)]
#[path = "auth_reload_tests.rs"]
mod tests;
