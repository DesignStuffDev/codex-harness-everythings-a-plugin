//! Explicitly selected credential acquisition through the native external-auth contract.
//! OAuth, keyring persistence, login restrictions, and account ownership remain with login.

use std::path::Path;

use codex_component_host::ComponentBinding;
use codex_component_host::ComponentCatalog;
use codex_protocol::auth::RefreshTokenFailedError;
use codex_protocol::auth::RefreshTokenFailedReason;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use thiserror::Error;

use crate::CodexAuth;
use crate::ExternalAuth;
use crate::ExternalAuthFuture;
use crate::ExternalAuthRefreshContext;
use crate::ExternalAuthRefreshReason;
use crate::RefreshTokenError;

const MAX_CREDENTIAL_BYTES: usize = 64 * 1024;

/// An installed provider owns credential acquisition and refresh across process invocations.
/// Only the native AuthManager validates and commits the returned credentials.
pub(crate) struct ComponentAuth(ComponentBinding);

impl std::fmt::Debug for ComponentAuth {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComponentAuth")
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Error)]
enum ComponentAuthError {
    #[error("component authentication configuration is invalid")]
    Configuration,
    #[error("component credential provider is temporarily unavailable")]
    Unavailable,
    #[error("component credential provider returned invalid credentials")]
    InvalidResponse,
    #[error("component credential provider rejected authentication")]
    Rejected,
    #[error("component credential refresh cannot change the current account")]
    AccountChanged,
    #[error("component credentials were rejected by native authentication policy")]
    PolicyRejected,
}

impl From<ComponentAuthError> for RefreshTokenError {
    fn from(error: ComponentAuthError) -> Self {
        match error {
            ComponentAuthError::Unavailable => Self::Transient(std::io::Error::other(error)),
            ComponentAuthError::Configuration
            | ComponentAuthError::InvalidResponse
            | ComponentAuthError::Rejected
            | ComponentAuthError::AccountChanged
            | ComponentAuthError::PolicyRejected => Self::Permanent(RefreshTokenFailedError::new(
                RefreshTokenFailedReason::Other,
                error.to_string(),
            )),
        }
    }
}

// Do not derive Debug: these fields contain credentials. Parsing failures are also redacted.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum CredentialResponse {
    ApiKey {
        api_key: String,
    },
    Chatgpt {
        access_token: String,
        chatgpt_account_id: String,
        chatgpt_plan_type: Option<String>,
    },
    Error {
        kind: FailureKind,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum FailureKind {
    Transient,
    Permanent,
}

impl ComponentAuth {
    pub(crate) fn selected(codex_home: &Path) -> Result<Option<Self>, RefreshTokenError> {
        let catalog =
            ComponentCatalog::load(codex_home).map_err(|_| ComponentAuthError::Configuration)?;
        Ok(catalog.selected("auth", "default").map(Self))
    }

    async fn request(&self, method: &str, params: Value) -> std::io::Result<CodexAuth> {
        // Neither transport errors nor deserialization errors are safe diagnostics: a provider
        // can include a token in an error message, unknown field, or unexpected wire value.
        let value = self
            .0
            .call(method, params)
            .await
            .map_err(|_| std::io::Error::other(ComponentAuthError::Unavailable))?;
        let response: CredentialResponse = serde_json::from_value(value)
            .map_err(|_| std::io::Error::other(ComponentAuthError::InvalidResponse))?;
        let auth = match response {
            CredentialResponse::ApiKey { api_key }
                if !api_key.trim().is_empty() && api_key.len() <= MAX_CREDENTIAL_BYTES =>
            {
                Ok(CodexAuth::from_api_key(&api_key))
            }
            CredentialResponse::Chatgpt {
                access_token,
                chatgpt_account_id,
                chatgpt_plan_type,
            } if !access_token.trim().is_empty()
                && access_token.len() <= MAX_CREDENTIAL_BYTES
                && !chatgpt_account_id.trim().is_empty()
                && chatgpt_account_id.len() <= 1024
                && chatgpt_plan_type
                    .as_ref()
                    .is_none_or(|plan| plan.len() <= 128) =>
            {
                CodexAuth::from_external_chatgpt_tokens(
                    &access_token,
                    &chatgpt_account_id,
                    chatgpt_plan_type.as_deref(),
                )
                .map_err(|_| ComponentAuthError::InvalidResponse)
            }
            CredentialResponse::Error {
                kind: FailureKind::Transient,
            } => Err(ComponentAuthError::Unavailable),
            CredentialResponse::Error {
                kind: FailureKind::Permanent,
            } => Err(ComponentAuthError::Rejected),
            CredentialResponse::ApiKey { .. } | CredentialResponse::Chatgpt { .. } => {
                Err(ComponentAuthError::InvalidResponse)
            }
        };
        auth.map_err(std::io::Error::other)
    }
}

impl ExternalAuth for ComponentAuth {
    fn resolve(&self) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(self.request("auth.resolve", json!({})))
    }

    fn refresh(&self, context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, CodexAuth> {
        Box::pin(async move {
            let reason = match context.reason {
                ExternalAuthRefreshReason::Unauthorized => "unauthorized",
            };
            let auth = self
                .request(
                    "auth.refresh",
                    json!({
                        "reason": reason,
                        "previous_account_id": context.previous_account_id,
                    }),
                )
                .await?;
            if let Some(previous) = context.previous_account_id
                && auth.get_account_id().as_deref() != Some(previous.as_str())
            {
                return Err(std::io::Error::other(ComponentAuthError::AccountChanged));
            }
            Ok(auth)
        })
    }

    fn classify_error(&self, error: std::io::Error) -> RefreshTokenError {
        error
            .get_ref()
            .and_then(|source| source.downcast_ref::<ComponentAuthError>())
            .copied()
            .unwrap_or(ComponentAuthError::PolicyRejected)
            .into()
    }
}

#[cfg(test)]
#[path = "component_auth_tests.rs"]
mod tests;
