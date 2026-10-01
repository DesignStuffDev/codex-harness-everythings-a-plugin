use codex_config::ManagedAuthPolicy;
use codex_protocol::config_types::ForcedLoginMethod;
use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use tokio::sync::watch;

/// Failure to observe or validate this manager's local authentication policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AuthPolicyError {
    #[error("authentication policy is unavailable")]
    Unavailable,
    #[error("authentication policy revision is exhausted")]
    RevisionExhausted,
    #[error("authentication policy owner or revision changed")]
    Changed,
}

/// An opaque manager-local observation, not permission to perform later work.
#[derive(Clone)]
pub struct AuthPolicyStamp {
    owner: Arc<()>,
    epoch: u64,
}
impl PartialEq for AuthPolicyStamp {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.epoch == other.epoch
    }
}
impl Eq for AuthPolicyStamp {}
impl fmt::Debug for AuthPolicyStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthPolicyStamp")
            .field("epoch", &self.epoch)
            .finish_non_exhaustive()
    }
}

/// Effective decisions from one policy revision; workspace identifiers are redacted in Debug.
#[derive(Clone)]
pub struct AuthPolicySnapshot {
    stamp: AuthPolicyStamp,
    forced: Option<Vec<String>>,
    effective: Option<Vec<String>>,
    allowed: Vec<ForcedLoginMethod>,
}
impl AuthPolicySnapshot {
    pub fn stamp(&self) -> AuthPolicyStamp {
        self.stamp.clone()
    }
    pub fn forced_chatgpt_workspace_id(&self) -> Option<&[String]> {
        self.forced.as_deref()
    }
    pub fn effective_chatgpt_workspaces(&self) -> Option<&[String]> {
        self.effective.as_deref()
    }
    pub fn allowed_login_methods(&self) -> &[ForcedLoginMethod] {
        &self.allowed
    }
}
impl fmt::Debug for AuthPolicySnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthPolicySnapshot")
            .field("stamp", &self.stamp)
            .finish_non_exhaustive()
    }
}

struct State {
    forced: Option<Vec<String>>,
    epoch: u64,
    failure: Option<AuthPolicyError>,
}
pub(super) struct PolicyCommitGuard<'a> {
    _state: MutexGuard<'a, State>,
}

pub(super) struct WorkspacePolicy {
    state: Mutex<State>,
    owner: Arc<()>,
    changed: watch::Sender<()>,
}
impl fmt::Debug for WorkspacePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkspacePolicy").finish_non_exhaustive()
    }
}
impl WorkspacePolicy {
    pub(super) fn new(forced: Option<Vec<String>>) -> Self {
        Self {
            state: Mutex::new(State {
                forced,
                epoch: 0,
                failure: None,
            }),
            owner: Arc::new(()),
            changed: watch::channel(()).0,
        }
    }

    fn lock_available(&self) -> Result<MutexGuard<'_, State>, AuthPolicyError> {
        let (mut state, poisoned) = match self.state.lock() {
            Ok(state) => (state, false),
            Err(error) => (error.into_inner(), true),
        };
        let newly_unavailable = poisoned && state.failure.is_none();
        if newly_unavailable {
            state.failure = Some(AuthPolicyError::Unavailable);
        }
        if let Some(error) = state.failure {
            drop(state);
            if newly_unavailable {
                self.changed.send_replace(());
            }
            return Err(error);
        }
        Ok(state)
    }

    pub(super) fn lock_current(
        &self,
        stamp: &AuthPolicyStamp,
    ) -> Result<PolicyCommitGuard<'_>, AuthPolicyError> {
        let state = self.lock_available()?;
        if !Arc::ptr_eq(&self.owner, &stamp.owner) || state.epoch != stamp.epoch {
            return Err(AuthPolicyError::Changed);
        }
        Ok(PolicyCommitGuard { _state: state })
    }

    // Only local pure state operations enter here. Wakeups are always outside the state lock.
    fn access<T>(
        &self,
        operation: impl FnOnce(&mut State) -> (Result<T, AuthPolicyError>, bool),
    ) -> Result<T, AuthPolicyError> {
        let mut state = self.lock_available()?;
        let (result, notify) = operation(&mut state);
        drop(state);
        if notify {
            self.changed.send_replace(());
        }
        result
    }

    pub(super) fn set(&self, forced: Option<Vec<String>>) -> Result<(), AuthPolicyError> {
        self.access(|state| {
            if state.forced == forced {
                return (Ok(()), false);
            }
            let Some(epoch) = state.epoch.checked_add(1) else {
                state.failure = Some(AuthPolicyError::RevisionExhausted);
                return (Err(AuthPolicyError::RevisionExhausted), true);
            };
            state.forced = forced;
            state.epoch = epoch;
            (Ok(()), true)
        })
    }

    pub(super) fn snapshot(
        &self,
        managed: &ManagedAuthPolicy,
        forced_login_method: Option<ForcedLoginMethod>,
    ) -> Result<AuthPolicySnapshot, AuthPolicyError> {
        let (forced, stamp) = self.access(|state| {
            (
                Ok((
                    state.forced.clone(),
                    AuthPolicyStamp {
                        owner: self.owner.clone(),
                        epoch: state.epoch,
                    },
                )),
                false,
            )
        })?;
        Ok(AuthPolicySnapshot {
            effective: managed.effective_chatgpt_workspaces(forced.as_deref()),
            allowed: managed.allowed_login_methods(forced_login_method, forced.as_deref()),
            forced,
            stamp,
        })
    }

    pub(super) fn validate(&self, stamp: &AuthPolicyStamp) -> Result<(), AuthPolicyError> {
        self.access(|state| {
            let current = Arc::ptr_eq(&self.owner, &stamp.owner) && state.epoch == stamp.epoch;
            (
                if current {
                    Ok(())
                } else {
                    Err(AuthPolicyError::Changed)
                },
                false,
            )
        })
    }

    pub(super) fn changes(&self) -> watch::Receiver<()> {
        self.changed.subscribe()
    }
}

#[cfg(test)]
#[path = "workspace_policy_tests.rs"]
mod tests;
