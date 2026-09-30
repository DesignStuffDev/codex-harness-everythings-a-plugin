use codex_component_host::DependencyContext;
use codex_component_host::DependencyError;
use codex_component_host::DependencyErrorCode;
use codex_component_host::DependencyFuture;
use codex_component_host::HostDependencyService;
use codex_model_catalog_api::CatalogError;
use codex_model_catalog_api::FetchModels;
use codex_model_catalog_api::FetchOutcome;
use codex_model_catalog_api::OwnerValidation;
use codex_model_catalog_api::ValidateOwner;
use codex_models_manager::manager::component::CatalogAuthority;
use codex_protocol::auth::AuthMode;
use codex_protocol::error::CodexErr;
use codex_protocol::error::CodexErrorDetails;
use serde_json::Value;

use super::CatalogEndpointContext;
use super::NativeCatalogAuthority;

impl HostDependencyService for NativeCatalogAuthority {
    fn call(&self, context: DependencyContext, method: String, params: Value)
        -> DependencyFuture<'_, Result<Value, DependencyError>>
    {
        Box::pin(async move {
            context.check_authority()?;
            let request = context.request_context::<CatalogEndpointContext>()
                .ok_or_else(|| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
            if context.owner_generation() != request.auth.owner_generation
                || context.owner_identity() != request.auth.identity.as_deref()
                || !self.is_current(&request.auth)
            {
                return Err(DependencyError::new(DependencyErrorCode::OwnerChanged));
            }
            let response = match method.as_str() {
                "fetch" => {
                    let params: FetchModels = serde_json::from_value(params)
                        .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
                    let outcome = tokio::select! {
                        biased;
                        _ = context.cancelled() => {
                            return Err(context.check_authority().err().unwrap_or_else(||
                                DependencyError::new(DependencyErrorCode::Cancelled)));
                        }
                        result = self.fetch(request, params) => result?,
                    };
                    serde_json::to_value(outcome)
                }
                "validate_owner" => {
                    let params: ValidateOwner = serde_json::from_value(params)
                        .map_err(|_| DependencyError::new(DependencyErrorCode::InvalidRequest))?;
                    serde_json::to_value(OwnerValidation {
                        matches: params.identity == request.auth.identity && self.is_current(&request.auth),
                    })
                }
                _ => return Err(DependencyError::new(DependencyErrorCode::InvalidRequest)),
            }.map_err(|_| DependencyError::new(DependencyErrorCode::ServiceFailure))?;
            context.check_authority()?;
            Ok(response)
        })
    }
}

impl NativeCatalogAuthority {
    pub(super) async fn fetch(&self, request: &CatalogEndpointContext, params: FetchModels)
        -> Result<FetchOutcome, DependencyError>
    {
        if !self.is_current(&request.auth) {
            return Err(DependencyError::new(DependencyErrorCode::OwnerChanged));
        }
        if params.client_version.is_empty() || params.client_version.len() > 128
            || !params.client_version.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(DependencyError::new(DependencyErrorCode::InvalidRequest));
        }
        let endpoint = self.endpoint.as_ref()
            .ok_or_else(|| DependencyError::new(DependencyErrorCode::ServiceUnavailable))?;
        let auth = &request.auth;
        let uses_api_key = auth.has_provider_api_key || auth.auth_mode == Some(AuthMode::ApiKey);
        // These are the native manager's discovery gates. A selected component
        // cannot use the host grant to bypass a disabled discovery capability.
        if uses_api_key && !auth.has_command_auth
            && (!auth.supports_api_key_models || !auth.api_key_discovery_enabled)
        {
            return Ok(FetchOutcome::Failed { error: CatalogError::InvalidRequest });
        }
        if !(auth.uses_codex_backend || auth.has_command_auth
            || (auth.supports_api_key_models && uses_api_key))
        {
            return Ok(FetchOutcome::Failed { error: CatalogError::Unavailable });
        }
        let result = endpoint.list_models(&params.client_version, request.http_client_factory.clone()).await;
        if !self.is_current(auth) {
            return Err(DependencyError::new(DependencyErrorCode::OwnerChanged));
        }
        Ok(match result {
            Ok(response) if Some(&response.identity) == auth.identity.as_ref() => FetchOutcome::Fetched {
                models: response.models,
                etag: response.etag,
                identity: response.identity,
            },
            Ok(_) => return Err(DependencyError::new(DependencyErrorCode::OwnerChanged)),
            Err(error) => FetchOutcome::Failed { error: classify_error(&error) },
        })
    }
}

fn classify_error(error: &CodexErr) -> CatalogError {
    match error.details() {
        CodexErrorDetails::Timeout | CodexErrorDetails::RequestTimeout => CatalogError::Timeout,
        CodexErrorDetails::UnexpectedStatus(error) => match error.status.as_u16() {
            401 | 403 => CatalogError::Authentication,
            429 => CatalogError::RateLimited,
            400 | 404 | 422 => CatalogError::InvalidRequest,
            _ => CatalogError::Unavailable,
        },
        CodexErrorDetails::RateLimitExceeded(_) | CodexErrorDetails::UsageLimitReached(_)
            | CodexErrorDetails::QuotaExceeded => CatalogError::RateLimited,
        CodexErrorDetails::RefreshTokenFailed(_) => CatalogError::Authentication,
        CodexErrorDetails::InvalidRequest(_) => CatalogError::InvalidRequest,
        // Discard all native diagnostics: provider URLs, response bodies and
        // transport/auth errors can contain credential material.
        _ => CatalogError::Unavailable,
    }
}
