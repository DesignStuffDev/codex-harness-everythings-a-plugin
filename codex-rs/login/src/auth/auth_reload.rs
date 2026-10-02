//! Source/cache/policy-checked reload publication. Persistent refresh is a separate boundary.
use super::AuthManager;
use super::AuthPolicyError;
use super::AuthPolicyStamp;
use super::AuthScopedRefreshFailure;
use super::CodexAuth;
use super::ExternalAuth;
use super::RefreshTokenFailedError;
use super::auth_source::AuthOwnerUpdate;
use super::auth_source::AuthSourceRevision;
use super::auth_source::InstallIntent;
use super::same_owner;
use std::fmt;
use std::sync::Arc;

/// Exact credential-cache identity, retained by loads so transitions cannot alias after ABA.
/// This does not identify external sources or mutations outside cache replacement.
#[derive(Clone)]
pub(super) struct AuthCacheRevision(Arc<()>);
impl AuthCacheRevision {
    pub(super) fn new() -> Self {
        Self(Arc::new(()))
    }
}
impl PartialEq for AuthCacheRevision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for AuthCacheRevision {}
impl fmt::Debug for AuthCacheRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthCacheRevision(..)")
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AuthLoadError {
    #[error(transparent)]
    Policy(AuthPolicyError),
    #[error(transparent)]
    Source(std::io::Error),
    #[error("cached authentication state is unavailable")]
    CacheUnavailable,
    #[error("cached authentication credentials changed")]
    CredentialsChanged,
    #[error("external authentication source or install intent changed")]
    SourceChanged,
}

#[derive(Debug)]
pub(super) struct LoadedAuth {
    pub(super) policy: AuthPolicyStamp,
    pub(super) revision: AuthCacheRevision,
    pub(super) source: AuthSourceRevision,
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
    Captured {
        policy: &'a AuthPolicyStamp,
        revision: &'a AuthCacheRevision,
        source: &'a AuthSourceRevision,
    },
}
pub(super) struct CacheChange {
    pub(super) changed: bool,
    pub(super) credentials_changed: bool,
}

// Kept internal: callers cannot combine a source install with a Preserve update.
enum SourceChange {
    Keep,
    Install(Arc<dyn ExternalAuth>),
    Clear,
}

impl AuthManager {
    pub(super) fn commit_auth_load(
        &self,
        loaded: LoadedAuth,
    ) -> Result<CacheChange, AuthLoadError> {
        self.replace_auth_cache(
            loaded.update,
            CachePolicy::Captured {
                policy: &loaded.policy,
                revision: &loaded.revision,
                source: &loaded.source,
            },
        )
    }

    pub(super) fn replace_auth_cache(
        &self,
        update: AuthLoadUpdate,
        policy: CachePolicy<'_>,
    ) -> Result<CacheChange, AuthLoadError> {
        self.replace_auth_owner(AuthOwnerUpdate::Cache(update), policy)
    }

    pub(super) fn replace_auth_owner(
        &self,
        update: AuthOwnerUpdate,
        policy: CachePolicy<'_>,
    ) -> Result<CacheChange, AuthLoadError> {
        let (mut update, source, install) = match update {
            AuthOwnerUpdate::Cache(update) => (update, SourceChange::Keep, None),
            AuthOwnerUpdate::Install {
                provider,
                auth,
                context,
            } => (
                AuthLoadUpdate::Replace(Some(auth)),
                SourceChange::Install(provider),
                Some(context),
            ),
            AuthOwnerUpdate::ClearExternal => {
                (AuthLoadUpdate::Replace(None), SourceChange::Clear, None)
            }
        };
        // An Install always uses its admitted policy/cache, regardless of the caller's cache mode.
        let policy = match &install {
            Some(context) => CachePolicy::Captured {
                policy: &context.policy,
                revision: &context.cache,
                source: &context.source,
            },
            None => policy,
        };
        let mut next_source = (!matches!(source, SourceChange::Keep)).then(AuthSourceRevision::new);
        let mut next_intent = matches!(source, SourceChange::Clear).then(InstallIntent::new);
        let mut retired_source = None;
        let mut retired_intent = None;
        // Declared before locks so even unwinding drops trait-backed auth only after lock release.
        let retired_provider;
        let mut retired_auth = None;
        let mut retired_failure = None;
        let mut retired_revision = None;
        let mut next_revision = None;
        let mut cached = self
            .inner
            .write()
            .map_err(|_| AuthLoadError::CacheUnavailable)?;
        if let Some(context) = &install
            && let Err(error) = context.check_owner(&cached)
        {
            drop(cached);
            return Err(error);
        }
        if let Some(intent) = next_intent.take() {
            retired_intent = Some(std::mem::replace(&mut cached.install_intent, intent));
        }
        if matches!(source, SourceChange::Clear) && cached.external_auth.is_none() {
            return Ok(CacheChange {
                changed: false,
                credentials_changed: false,
            });
        }
        if let CachePolicy::Captured { source, .. } = &policy
            && &cached.source_revision != *source
        {
            drop(cached);
            return Err(AuthLoadError::SourceChanged);
        }
        if let CachePolicy::Captured { revision, .. } = &policy
            && &cached.revision != *revision
        {
            drop(cached);
            return Err(AuthLoadError::CredentialsChanged);
        }
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
        // Allocate outside the policy guard and retain old identities until after all locks.
        if credentials_changed {
            next_revision = Some(AuthCacheRevision::new());
        }
        let policy_guard = match policy {
            CachePolicy::Unfenced => None,
            CachePolicy::Captured { policy, .. } => {
                match self.workspace_policy.lock_current(policy) {
                    Ok(guard) => Some(guard),
                    Err(error) => {
                        drop(cached);
                        return Err(AuthLoadError::Policy(error));
                    }
                }
            }
        };
        // Only owned swaps occur under policy lock; this is the policy commit linearization point.
        match &mut update {
            AuthLoadUpdate::Replace(auth) => {
                retired_auth = std::mem::replace(&mut cached.auth, auth.take());
                if credentials_changed || !matches!(source, SourceChange::Keep) {
                    retired_failure = cached.permanent_refresh_failure.take();
                }
            }
            AuthLoadUpdate::Preserve { .. } => {
                if let Some(failure) = failure {
                    retired_failure = cached.permanent_refresh_failure.replace(failure);
                }
            }
        }
        retired_provider = match source {
            SourceChange::Keep => None,
            SourceChange::Install(provider) => cached.external_auth.replace(provider),
            SourceChange::Clear => cached.external_auth.take(),
        };
        if let Some(revision) = next_source.take() {
            retired_source = Some(std::mem::replace(&mut cached.source_revision, revision));
        }
        if let Some(revision) = next_revision.take() {
            retired_revision = Some(std::mem::replace(&mut cached.revision, revision));
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
        drop((
            retired_provider,
            retired_source,
            retired_intent,
            retired_auth,
            retired_failure,
            retired_revision,
        ));
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
