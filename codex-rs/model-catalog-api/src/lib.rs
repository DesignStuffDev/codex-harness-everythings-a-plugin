//! Owned, credential-free model catalog and endpoint dependency contracts.

use std::path::PathBuf;

use codex_protocol::auth::AuthMode;
use codex_protocol::openai_models::ModelInfo;
use serde::Deserialize;
use serde::Serialize;

pub const CONTRACT_VERSION: u32 = 1;
pub const ENDPOINT_SERVICE: &str = "host.model_endpoint";
pub const ENDPOINT_VERSION: u32 = 1;
pub const OPEN: &str = "model_catalog.open";
pub const REFRESH: &str = "model_catalog.refresh";
pub const REFRESH_ETAG: &str = "model_catalog.refresh_etag";
pub const SNAPSHOT: &str = "model_catalog.snapshot";

/// Configuration selects the native merge policy, never an implicit alternate backend.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CatalogSource {
    BundledMerge,
    Authoritative,
    Static { models: Vec<ModelInfo> },
    /// Native provider seeds (for example Bedrock), replaceable by a custom catalog.
    ProviderStatic { models: Vec<ModelInfo> },
}

/// Native-file interoperability is explicit. Custom components default to their own state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CatalogCache {
    Disabled,
    ComponentState,
    NativeDirectory { directory: PathBuf },
}

/// A request-time policy snapshot. No credential, token, account ID or URL is present.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogAuthState {
    pub owner_generation: u64,
    pub identity: Option<String>,
    pub auth_mode: Option<AuthMode>,
    pub uses_codex_backend: bool,
    pub has_command_auth: bool,
    pub supports_api_key_models: bool,
    pub has_provider_api_key: bool,
    pub api_key_discovery_enabled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshStrategy {
    Online,
    Offline,
    OnlineIfUncached,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpenCatalog {
    pub contract_version: u32,
    pub client_version: String,
    pub source: CatalogSource,
    pub cache: CatalogCache,
    pub auth: CatalogAuthState,
    pub refresh: RefreshStrategy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshCatalog {
    pub auth: CatalogAuthState,
    pub strategy: RefreshStrategy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshEtag {
    pub auth: CatalogAuthState,
    pub etag: String,
}

/// Host publication must additionally validate the live owner after receiving this value.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSnapshot {
    pub contract_version: u32,
    pub owner_generation: u64,
    pub identity: Option<String>,
    pub revision: u64,
    pub etag: Option<String>,
    pub models: Vec<ModelInfo>,
}

/// Domain failures are distinct from broker/IPC failures and never carry provider diagnostics.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogError {
    OwnerChanged,
    Authentication,
    RateLimited,
    Timeout,
    Unavailable,
    InvalidRequest,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CatalogOutcome {
    Ready { snapshot: CatalogSnapshot },
    Failed { error: CatalogError, snapshot: CatalogSnapshot },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FetchModels {
    pub client_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum FetchOutcome {
    Fetched { models: Vec<ModelInfo>, etag: Option<String>, identity: String },
    Failed { error: CatalogError },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateOwner {
    pub identity: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerValidation {
    pub matches: bool,
}
