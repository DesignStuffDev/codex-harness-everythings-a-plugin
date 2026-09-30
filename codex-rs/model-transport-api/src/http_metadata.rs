use crate::WireError;
#[cfg(feature = "native")]
use base64::Engine;
#[cfg(feature = "native")]
use base64::engine::general_purpose::STANDARD;
#[cfg(feature = "native")]
use http::HeaderMap;
use http::HeaderName;
use http::HeaderValue;
use serde::Deserialize;
use serde::Serialize;
#[cfg(feature = "native")]
use serde_json::Value;
use std::collections::BTreeMap;

const FIXED_HEADERS: &[&str] = &[
    "x-request-id", "x-oai-request-id", "cf-ray", "x-openai-authorization-error",
    "x-error-json", "x-codex-active-limit", "x-codex-promo-message",
    "x-codex-rate-limit-reached-type", "x-codex-credits-has-credits",
    "x-codex-credits-unlimited", "x-codex-credits-balance",
];
const LIMIT_SUFFIXES: &[&str] = &[
    "limit-name", "primary-used-percent", "primary-window-minutes", "primary-reset-at",
    "secondary-used-percent", "secondary-window-minutes", "secondary-reset-at",
];

/// Only HTTP metadata consumed by current native model-error policy crosses here.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HttpErrorHeaders(pub BTreeMap<String, String>);

impl std::fmt::Debug for HttpErrorHeaders {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("HttpErrorHeaders").finish_non_exhaustive()
    }
}

impl HttpErrorHeaders {
    pub fn validate(&self) -> Result<(), WireError> {
        let limit = normalized_limit(self.0.get("x-codex-active-limit").map(String::as_str));
        for (name, value) in &self.0 {
            if !allowed_header(name, &limit) {
                return Err(WireError::Header);
            }
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| WireError::Header)?;
            HeaderValue::from_str(value).map_err(|_| WireError::Header)?
                .to_str().map_err(|_| WireError::Header)?;
        }
        Ok(())
    }

    /// Filter a native map without collapsing present-empty headers into absence.
    #[cfg(feature = "native")]
    pub(crate) fn from_native(headers: HeaderMap) -> Self {
        let limit = normalized_limit(headers.get("x-codex-active-limit").and_then(|v| v.to_str().ok()));
        let values = headers.keys().filter_map(|name| {
            if !allowed_header(name.as_str(), &limit) { return None; }
            let value = headers.get(name)?.to_str().ok()?;
            let value = if name == "x-error-json" {
                project_identity_code(value)?
            } else {
                value.to_owned()
            };
            Some((name.as_str().to_owned(), value))
        }).collect();
        Self(values)
    }

    #[cfg(feature = "native")]
    pub(crate) fn into_native(self) -> Result<HeaderMap, WireError> {
        self.validate()?;
        let mut headers = HeaderMap::new();
        for (name, value) in self.0 {
            let value = if name == "x-error-json" {
                let Some(projected) = project_identity_code(&value) else { continue; };
                projected
            } else { value };
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).map_err(|_| WireError::Header)?,
                HeaderValue::from_str(&value).map_err(|_| WireError::Header)?,
            );
        }
        Ok(headers)
    }
}

fn normalized_limit(active: Option<&str>) -> String {
    active.map(str::trim).filter(|value| !value.is_empty()).unwrap_or("codex")
        .to_ascii_lowercase().replace('_', "-")
}

fn allowed_header(name: &str, limit: &str) -> bool {
    FIXED_HEADERS.contains(&name) || LIMIT_SUFFIXES.iter().any(|suffix| name == format!("x-{limit}-{suffix}"))
}

#[cfg(feature = "native")]
fn project_identity_code(encoded: &str) -> Option<String> {
    let decoded = STANDARD.decode(encoded).ok()?;
    let parsed: Value = serde_json::from_slice(&decoded).ok()?;
    let code = parsed.get("error")?.get("code")?.as_str()?;
    let projected = serde_json::json!({"error": {"code": code}});
    Some(STANDARD.encode(serde_json::to_vec(&projected).ok()?))
}

pub(crate) fn validate_public_url(value: &str) -> Result<(), WireError> {
    let url = url::Url::parse(value).map_err(|_| WireError::Url)?;
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err(WireError::Url);
    }
    Ok(())
}

#[cfg(feature = "native")]
pub(crate) fn redact_url(value: String) -> Result<String, WireError> {
    let mut url = url::Url::parse(&value).map_err(|_| WireError::Url)?;
    if !url.username().is_empty() { url.set_username("").map_err(|_| WireError::Url)?; }
    if url.password().is_some() { url.set_password(None).map_err(|_| WireError::Url)?; }
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.into())
}
