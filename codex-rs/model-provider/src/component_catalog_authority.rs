use std::future::pending;
use std::time::Duration;

use codex_component_host::DependencyError;
use codex_component_host::DependencyErrorCode;
use codex_component_host::DependencyFuture;
use codex_component_host::DependencyOwner;
use codex_component_host::HostDependencyAuthority;
use codex_component_host::HostOperationScope;
use codex_http_client::HttpClientFactory;
use codex_login::CodexAuth;
use codex_model_catalog_api::CatalogAuthState;
use codex_models_manager::manager::ModelsManagerFuture;
use codex_models_manager::manager::component::CatalogAuthority;
use codex_models_manager::manager::component::CatalogRequestAuthority;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CoreResult;
use tokio::time::Instant;

use super::CatalogEndpointContext;
use super::NativeCatalogAuthority;
use super::NativeLoginPolicy;
use super::Observation;

impl NativeCatalogAuthority {
    fn observe(&self) -> Result<CatalogAuthState, DependencyError> {
        let mut state = self.state.lock()
            .map_err(|_| DependencyError::new(DependencyErrorCode::AuthorityRevoked))?;
        if state.revoked {
            return Err(DependencyError::new(DependencyErrorCode::AuthorityRevoked));
        }
        // A cached credential snapshot and endpoint identity must be from the
        // same native auth generation. Continuous churn fails closed.
        for _ in 0..8 {
            let network_changed = state.network_changes.as_mut().is_some_and(|changes| {
                if changes.has_changed().unwrap_or(true) {
                    changes.borrow_and_update();
                    true
                } else {
                    false
                }
            });
            if network_changed {
                state.generation = state.generation.checked_add(1)
                    .ok_or_else(|| DependencyError::new(DependencyErrorCode::AuthorityRevoked))?;
                self.changes.send_replace(());
            }
            let before = state.auth_changes.as_ref()
                .map(|changes| *changes.borrow()).unwrap_or_default();
            let login_policy = NativeLoginPolicy::capture(self.auth_manager.as_deref());
            let auth = self.auth_manager.as_ref().and_then(|manager| manager.auth_cached());
            let endpoint = self.endpoint.as_ref();
            let snapshot = CatalogAuthState {
                owner_generation: 0,
                identity: endpoint.and_then(|endpoint| endpoint.identity()),
                auth_mode: auth.as_ref().map(CodexAuth::auth_mode),
                uses_codex_backend: auth.as_ref().is_some_and(CodexAuth::uses_codex_backend),
                has_command_auth: endpoint.is_some_and(|endpoint| endpoint.has_command_auth()),
                supports_api_key_models: endpoint.is_some_and(|endpoint| endpoint.supports_api_key_models()),
                has_provider_api_key: endpoint.is_some_and(|endpoint| endpoint.has_provider_api_key()),
                api_key_discovery_enabled: state.api_key_discovery_enabled,
            };
            let after = state.auth_changes.as_ref()
                .map(|changes| *changes.borrow()).unwrap_or_default();
            if before.generation != after.generation
                || login_policy != NativeLoginPolicy::capture(self.auth_manager.as_deref())
                || state.network_changes.as_ref()
                    .is_some_and(|changes| changes.has_changed().unwrap_or(true))
            {
                continue;
            }
            let observation = Observation {
                native_owner_generation: after.owner_generation,
                login_policy,
                auth: snapshot,
            };
            if state.observed.as_ref() != Some(&observation) {
                state.generation = state.generation.checked_add(1)
                    .ok_or_else(|| DependencyError::new(DependencyErrorCode::AuthorityRevoked))?;
                state.observed = Some(observation);
                self.changes.send_replace(());
            }
            let mut snapshot = state.observed.as_ref().expect("observation was initialized").auth.clone();
            snapshot.owner_generation = state.generation;
            return Ok(snapshot);
        }
        Err(DependencyError::new(DependencyErrorCode::OwnerChanged))
    }
}

impl CatalogAuthority for NativeCatalogAuthority {
    fn snapshot(&self, http_client_factory: HttpClientFactory)
        -> ModelsManagerFuture<'_, CoreResult<CatalogRequestAuthority>>
    {
        Box::pin(async move {
            let deadline = Instant::now() + self.request_timeout;
            // Resolve lazy command/external auth before publishing a scope. The
            // native endpoint may refresh again at request time; both response
            // identity and the live owner are checked before returning its data.
            if let Some(manager) = &self.auth_manager {
                tokio::time::timeout_at(deadline, manager.auth()).await
                    .map_err(|_| CodexErr::RequestTimeout)?;
            }
            let auth = self.observe().map_err(|_| CodexErr::InvalidRequest(
                "model catalog owner is no longer current".to_owned(),
            ))?;
            let scope = HostOperationScope::new(
                DependencyOwner { generation: auth.owner_generation, identity: auth.identity.clone() },
                deadline,
            ).with_request_context(CatalogEndpointContext { http_client_factory, auth: auth.clone() });
            Ok(CatalogRequestAuthority { auth, scope })
        })
    }

    fn is_current(&self, auth: &CatalogAuthState) -> bool {
        self.observe().is_ok_and(|current| current == *auth)
    }

    fn set_api_key_model_discovery_enabled(&self, enabled: bool) {
        if let Ok(mut state) = self.state.lock()
            && state.api_key_discovery_enabled != enabled
        {
            state.api_key_discovery_enabled = enabled;
            // Preserve even an unobserved true -> false -> true transition.
            match state.generation.checked_add(1) {
                Some(generation) => state.generation = generation,
                None => state.revoked = true,
            }
            self.changes.send_replace(());
        }
    }

    fn revoke(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.revoked = true;
        }
        self.changes.send_replace(());
    }
}

impl HostDependencyAuthority for NativeCatalogAuthority {
    fn check(&self, owner: &DependencyOwner) -> Result<(), DependencyError> {
        let current = self.observe()?;
        if current.owner_generation != owner.generation || current.identity != owner.identity {
            return Err(DependencyError::new(DependencyErrorCode::OwnerChanged));
        }
        Ok(())
    }

    fn revoked(&self, owner: DependencyOwner) -> DependencyFuture<'_, ()> {
        Box::pin(async move {
            let mut changes = self.changes.subscribe();
            let mut auth_changes = self.auth_manager.as_ref()
                .map(|manager| manager.auth_change_state_receiver());
            let mut network_changes = self.auth_manager.as_ref()
                .and_then(|manager| manager.application_network_policy().changes());
            // Native provider credentials can also come from environment-backed
            // configuration, which has no notification API. Poll only that live
            // identity; auth/policy revocations wake immediately through watches.
            let mut poll = tokio::time::interval(Duration::from_millis(100));
            loop {
                if self.check(&owner).is_err() {
                    return;
                }
                tokio::select! {
                    _ = changes.changed() => {},
                    _ = async {
                        match auth_changes.as_mut() {
                            Some(changes) => { let _ = changes.changed().await; }
                            None => pending::<()>().await,
                        }
                    } => {},
                    _ = async {
                        match network_changes.as_mut() {
                            Some(changes) => { let _ = changes.changed().await; }
                            None => pending::<()>().await,
                        }
                    } => {},
                    _ = poll.tick() => {},
                }
            }
        })
    }
}
