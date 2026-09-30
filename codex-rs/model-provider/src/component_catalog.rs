//! Native, credential-free authority for the separately selected catalog manager.
//!
//! The broker retains the real endpoint and the HTTP factory accepted with each
//! operation. Credentials remain owned by `AuthManager` and the native endpoint.

use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use codex_component_host::DependencyError;
use codex_component_host::DependencyErrorCode;
use codex_component_host::HostDependencyDescriptor;
use codex_component_host::HostDependencyRegistry;
use codex_http_client::HttpClientFactory;
use codex_login::AuthChangeState;
use codex_login::AuthManager;
use codex_model_catalog_api::CatalogAuthState;
use codex_model_catalog_api::ENDPOINT_SERVICE;
use codex_model_catalog_api::ENDPOINT_VERSION;
use codex_models_manager::manager::ModelsEndpointClient;
use codex_models_manager::manager::component::CatalogAuthority;
use codex_protocol::config_types::ForcedLoginMethod;
use tokio::sync::watch;

#[path = "component_catalog_authority.rs"]
mod authority;
#[path = "component_catalog_service.rs"]
mod service;

/// Creates provider-owned dependencies for one selected catalog manager.
///
/// Pass the provider's actual endpoint, or `None` for an authoritative static
/// catalog. `None` never manufactures an OpenAI endpoint for another provider.
/// The optional grant lets custom catalog plugins supply their own discovery.
pub fn create_catalog_dependencies(
    endpoint: Option<Arc<dyn ModelsEndpointClient>>,
    auth_manager: Option<Arc<AuthManager>>,
    request_timeout: Duration,
) -> Result<(Arc<dyn CatalogAuthority>, HostDependencyRegistry), DependencyError> {
    let authority = Arc::new(NativeCatalogAuthority::new(endpoint, auth_manager, request_timeout));
    let mut registry = HostDependencyRegistry::new();
    registry.register_optional(
        HostDependencyDescriptor {
            name: ENDPOINT_SERVICE.to_owned(),
            version: ENDPOINT_VERSION,
            operations: vec!["fetch".to_owned(), "validate_owner".to_owned()],
        },
        authority.clone(),
        authority.clone(),
    ).map_err(|_| DependencyError::new(DependencyErrorCode::ServiceFailure))?;
    Ok((authority, registry))
}

struct NativeCatalogAuthority {
    endpoint: Option<Arc<dyn ModelsEndpointClient>>,
    auth_manager: Option<Arc<AuthManager>>,
    request_timeout: Duration,
    state: Mutex<AuthorityState>,
    changes: watch::Sender<()>,
}

impl fmt::Debug for NativeCatalogAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("NativeCatalogAuthority").finish_non_exhaustive()
    }
}

struct AuthorityState {
    generation: u64,
    revoked: bool,
    api_key_discovery_enabled: bool,
    observed: Option<Observation>,
    auth_changes: Option<watch::Receiver<AuthChangeState>>,
    network_changes: Option<watch::Receiver<()>>,
}

#[derive(PartialEq, Eq)]
struct Observation {
    // Native revisions survive coalesced A -> B -> A ownership notifications.
    native_owner_generation: u64,
    login_policy: NativeLoginPolicy,
    // This comparison always uses owner_generation zero; our epoch is assigned afterward.
    auth: CatalogAuthState,
}

// Workspace restrictions stay host-only. No account IDs enter CatalogAuthState.
#[derive(PartialEq, Eq)]
struct NativeLoginPolicy {
    allowed_methods: Vec<ForcedLoginMethod>,
    allowed_workspaces: Option<Vec<String>>,
}

impl NativeLoginPolicy {
    fn capture(manager: Option<&AuthManager>) -> Self {
        Self {
            allowed_methods: manager.map(AuthManager::allowed_login_methods).unwrap_or_default(),
            allowed_workspaces: manager.and_then(AuthManager::effective_chatgpt_workspaces),
        }
    }
}

/// This type is retained only in `HostOperationScope::request_context`.
/// In particular, there is no shared mutable "current HTTP factory".
#[derive(Clone)]
struct CatalogEndpointContext {
    http_client_factory: HttpClientFactory,
    auth: CatalogAuthState,
}

impl NativeCatalogAuthority {
    fn new(
        endpoint: Option<Arc<dyn ModelsEndpointClient>>,
        auth_manager: Option<Arc<AuthManager>>,
        request_timeout: Duration,
    ) -> Self {
        let auth_changes = auth_manager.as_ref().map(|manager| manager.auth_change_state_receiver());
        let network_changes = auth_manager.as_ref()
            .and_then(|manager| manager.application_network_policy().changes());
        Self {
            endpoint,
            auth_manager,
            request_timeout,
            state: Mutex::new(AuthorityState {
                generation: 0,
                revoked: false,
                api_key_discovery_enabled: false,
                observed: None,
                auth_changes,
                network_changes,
            }),
            changes: watch::channel(()).0,
        }
    }
}

#[cfg(test)]
#[path = "component_catalog_tests.rs"]
mod tests;
