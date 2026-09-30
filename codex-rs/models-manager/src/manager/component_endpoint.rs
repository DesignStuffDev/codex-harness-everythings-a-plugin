//! Native manager dependency adapter. It never reconstructs HTTP or credentials.

use std::sync::RwLock;
use std::sync::Mutex;

use codex_component_host::DependencyClient;
use codex_component_host::DependencyRequestScope;
use codex_http_client::HttpClientFactory;
use codex_model_catalog_api::CatalogAuthState;
use codex_model_catalog_api::CatalogError;
use codex_model_catalog_api::ENDPOINT_SERVICE;
use codex_model_catalog_api::ENDPOINT_VERSION;
use codex_model_catalog_api::FetchModels;
use codex_model_catalog_api::FetchOutcome;
use codex_model_catalog_api::OwnerValidation;
use codex_model_catalog_api::ValidateOwner;
use codex_protocol::auth::AuthMode;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CoreResult;

use super::ModelsEndpointClient;
use super::ModelsEndpointFuture;
use super::ModelsEndpointResponse;

#[derive(Clone, Debug)]
struct Invocation {
    auth: CatalogAuthState,
    scope: DependencyRequestScope,
}

/// Used only by the serialized native catalog service. Updating the invocation
/// while another operation is running would violate the service contract.
#[derive(Debug)]
pub(super) struct BrokerModelsEndpoint {
    client: DependencyClient,
    invocation: RwLock<Invocation>,
    failure: Mutex<Option<CatalogError>>,
}

impl BrokerModelsEndpoint {
    pub fn new(client: DependencyClient, auth: CatalogAuthState, scope: DependencyRequestScope) -> Self {
        Self { client, invocation: RwLock::new(Invocation { auth, scope }), failure: Mutex::new(None) }
    }

    pub fn set_invocation(&self, auth: CatalogAuthState, scope: DependencyRequestScope) {
        *self.invocation.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Invocation { auth, scope };
        *self.failure.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    fn invocation(&self) -> Invocation {
        self.invocation.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    pub async fn owner_is_current(&self) -> bool {
        let invocation = self.invocation();
        self.validate_identity(invocation.auth.identity.as_deref()).await
    }

    pub fn failure(&self) -> Option<CatalogError> {
        *self.failure.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl ModelsEndpointClient for BrokerModelsEndpoint {
    fn identity(&self) -> Option<String> { self.invocation().auth.identity }
    fn has_command_auth(&self) -> bool { self.invocation().auth.has_command_auth }
    fn uses_codex_backend(&self) -> ModelsEndpointFuture<'_, bool> {
        Box::pin(std::future::ready(self.invocation().auth.uses_codex_backend))
    }
    fn supports_api_key_models(&self) -> bool { self.invocation().auth.supports_api_key_models }
    fn has_provider_api_key(&self) -> bool { self.invocation().auth.has_provider_api_key }

    fn catalog_auth_mode(&self) -> Option<Option<AuthMode>> {
        Some(self.invocation().auth.auth_mode)
    }

    fn catalog_uses_codex_backend(&self) -> Option<bool> {
        Some(self.invocation().auth.uses_codex_backend)
    }

    fn validate_identity<'a>(&'a self, identity: Option<&'a str>) -> ModelsEndpointFuture<'a, bool> {
        Box::pin(async move {
            let invocation = self.invocation();
            if invocation.auth.identity.as_deref() != identity {
                return false;
            }
            let Ok(params) = serde_json::to_value(ValidateOwner { identity: identity.map(str::to_owned) }) else {
                return false;
            };
            let Ok(value) = self.client.call(&invocation.scope, ENDPOINT_SERVICE,
                ENDPOINT_VERSION, "validate_owner", params).await else {
                return false;
            };
            serde_json::from_value::<OwnerValidation>(value).is_ok_and(|validation| validation.matches)
        })
    }

    fn list_models<'a>(&'a self, client_version: &'a str, _http_client_factory: HttpClientFactory)
        -> ModelsEndpointFuture<'a, CoreResult<ModelsEndpointResponse>>
    {
        Box::pin(async move {
            let invocation = self.invocation();
            let params = serde_json::to_value(FetchModels { client_version: client_version.to_owned() })
                .map_err(|_| CodexErr::Fatal("invalid catalog dependency request".to_owned()))?;
            let value = self.client.call(&invocation.scope, ENDPOINT_SERVICE,
                ENDPOINT_VERSION, "fetch", params).await
                .map_err(|_| CodexErr::Fatal("catalog endpoint dependency failed".to_owned()))?;
            let outcome: FetchOutcome = serde_json::from_value(value)
                .map_err(|_| CodexErr::Fatal("invalid catalog endpoint response".to_owned()))?;
            match outcome {
                FetchOutcome::Fetched { models, etag, identity } => Ok(ModelsEndpointResponse { models, etag, identity }),
                FetchOutcome::Failed { error } => {
                    *self.failure.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error);
                    Err(domain_error(error))
                }
            }
        })
    }
}

pub(super) fn domain_error(error: CatalogError) -> CodexErr {
    match error {
        CatalogError::Timeout => CodexErr::RequestTimeout,
        CatalogError::InvalidRequest => CodexErr::InvalidRequest("catalog endpoint rejected request".to_owned()),
        CatalogError::OwnerChanged => CodexErr::Fatal("catalog owner changed".to_owned()),
        CatalogError::Authentication => CodexErr::Fatal("catalog endpoint authentication failed".to_owned()),
        CatalogError::RateLimited => CodexErr::Fatal("catalog endpoint rate limited".to_owned()),
        CatalogError::Unavailable => CodexErr::Fatal("catalog endpoint unavailable".to_owned()),
    }
}
