//! Current-policy credential acquisition; returned values are not future dispatch permits.
use super::AuthManager;
use super::CodexAuth;
use super::validate_auth_restrictions;

impl AuthManager {
    /// The two private callers prepare only a credential clone or its account-bound factory.
    /// Keep preparation outside the policy lock, then linearize the acquisition with both
    /// the credentials read lock and exact current-policy guard held.
    pub(super) fn checked_cached_auth<T>(
        &self,
        prepare: impl FnOnce(&CodexAuth) -> T,
    ) -> Option<T> {
        // Declared before guards: a prepared value is dropped after them even on unwinding.
        let prepared;
        let cached = self.inner.read().ok()?;
        let policy = self.auth_policy_snapshot().ok()?;
        let auth = cached.auth.as_ref()?;
        validate_auth_restrictions(
            Some(policy.allowed_login_methods()),
            policy.effective_chatgpt_workspaces(),
            auth,
        )
        .ok()?;
        prepared = prepare(auth);
        let current = self.workspace_policy.lock_current(&policy.stamp());
        match current {
            Ok(guard) => {
                drop(guard);
                drop(cached);
                Some(prepared)
            }
            Err(_) => {
                drop(cached);
                None
            }
        }
    }
}
