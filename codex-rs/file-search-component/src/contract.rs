use std::path::PathBuf;

use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use serde::Deserialize;
use serde::Serialize;

use crate::LeaseLimits;
use crate::ServiceLimits;
use crate::WireBudget;
use crate::WireOptions;
use crate::WireU64;

pub use codex_component_api::FILE_SEARCH_CONTRACT_VERSION;
pub const INITIALIZE_METHOD: &str = "file_search/initialize";
pub const OPEN_METHOD: &str = "file_search/open";
pub const UPDATE_METHOD: &str = "file_search/update_query";
pub const POLL_METHOD: &str = "file_search/next_snapshot";
pub const RELEASE_METHOD: &str = "file_search/release";
pub const SHUTDOWN_METHOD: &str = "file_search/shutdown";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderIdentity {
    pub provider_id: String,
}

impl ProviderIdentity {
    pub fn validate(&self) -> Result<(), SearchError> {
        validate_uuid(&self.provider_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseIdentity {
    pub provider_id: String,
    pub lease_id: String,
    pub session_epoch: WireU64,
}

impl LeaseIdentity {
    pub fn validate(&self) -> Result<(), SearchError> {
        validate_uuid(&self.provider_id)?;
        validate_uuid(&self.lease_id)?;
        if self.session_epoch.0 == 0 {
            return Err(invalid("file-search session epoch must be positive"));
        }
        Ok(())
    }
}

pub(crate) fn validate_uuid(value: &str) -> Result<(), SearchError> {
    let parsed = uuid::Uuid::parse_str(value)
        .map_err(|_| invalid("file-search identity must be a canonical UUID-v4"))?;
    if value.len() != 36
        || parsed.get_version() != Some(uuid::Version::Random)
        || parsed.hyphenated().to_string() != value
    {
        return Err(invalid("file-search identity must be a canonical UUID-v4"));
    }
    Ok(())
}

pub(crate) fn invalid(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::InvalidInput, message)
}

pub(crate) fn exhausted(message: &'static str) -> SearchError {
    SearchError::new(SearchErrorKind::ResourceExhausted, message)
}

pub(crate) fn version(version: u32) -> Result<(), SearchError> {
    if version != FILE_SEARCH_CONTRACT_VERSION {
        return Err(SearchError::new(
            SearchErrorKind::UnsupportedVersion,
            "unsupported file-search contract version",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitializeRequest {
    pub contract_version: u32,
    pub identity: ProviderIdentity,
    #[serde(with = "codex_component_path_codec::native_path")]
    pub base_dir: PathBuf,
    pub requested_limits: ServiceLimits,
}

impl InitializeRequest {
    pub fn validate(&self) -> Result<(), SearchError> {
        version(self.contract_version)?;
        self.identity.validate()?;
        if !self.base_dir.is_absolute() {
            return Err(invalid("file-search base directory must be absolute"));
        }
        self.requested_limits.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitializeResponse {
    pub limits: ServiceLimits,
    pub native_path_platform: String,
    pub encoding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRequest {
    pub contract_version: u32,
    pub identity: ProviderIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRequest {
    pub contract_version: u32,
    pub identity: LeaseIdentity,
    #[serde(with = "codex_component_path_codec::native_path::vec")]
    pub roots: Vec<PathBuf>,
    pub options: WireOptions,
    pub budget: WireBudget,
}

impl OpenRequest {
    pub fn from_native(identity: LeaseIdentity, request: SearchOpen) -> Result<Self, SearchError> {
        Ok(Self {
            contract_version: FILE_SEARCH_CONTRACT_VERSION,
            identity,
            roots: request.roots,
            options: WireOptions::from_native(&request.options)?,
            budget: WireBudget::from_native(request.budget),
        })
    }

    pub fn into_native(self) -> Result<SearchOpen, SearchError> {
        Ok(SearchOpen {
            roots: self.roots,
            options: self.options.into_native()?,
            budget: self.budget.into_native()?,
        })
    }

    pub fn validate(&self, limits: &ServiceLimits) -> Result<(), SearchError> {
        version(self.contract_version)?;
        self.identity.validate()?;
        let options = self.options.clone().into_native()?;
        if options.limit.get() > limits.max_matches as usize {
            return Err(exhausted(
                "file-search match limit exceeds negotiated ceiling",
            ));
        }
        self.budget
            .into_native()?
            .validate_within(&limits.resources.into_native()?)?;
        #[derive(Serialize)]
        struct RootsOptions<'a> {
            #[serde(with = "codex_component_path_codec::native_path::vec")]
            roots: &'a [PathBuf],
            options: &'a WireOptions,
        }
        crate::limits::check_size(
            &RootsOptions {
                roots: &self.roots,
                options: &self.options,
            },
            u64::from(limits.max_roots_options_bytes),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenResponse {
    pub initial_cursor: WireU64,
    pub limits: LeaseLimits,
    pub budget: WireBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseRequest {
    pub contract_version: u32,
    pub identity: LeaseIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest {
    pub contract_version: u32,
    pub identity: LeaseIdentity,
    pub query_epoch: WireU64,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateResponse {
    pub accepted_query_epoch: WireU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PollRequest {
    pub contract_version: u32,
    pub identity: LeaseIdentity,
    pub after_revision: WireU64,
    pub wait_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply<T, E = SearchError> {
    Ok { result: T },
    Error { error: E },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireReply<I, T, E = SearchError> {
    pub contract_version: u32,
    pub method: String,
    pub identity: I,
    pub reply: Reply<T, E>,
}

pub(crate) fn reply<I: Serialize, T: Serialize, E: Serialize>(
    method: &str,
    identity: I,
    result: Result<T, E>,
) -> Result<serde_json::Value, String> {
    let reply = match result {
        Ok(result) => Reply::Ok { result },
        Err(error) => Reply::Error { error },
    };
    serde_json::to_value(WireReply {
        contract_version: FILE_SEARCH_CONTRACT_VERSION,
        method: method.to_owned(),
        identity,
        reply,
    })
    .map_err(|_| "file-search response encoding failed".to_owned())
}
