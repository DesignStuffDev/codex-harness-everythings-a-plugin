//! Versioned, language-neutral manifests for independently installed engine components.
//! The wire protocol uses JSON, never Rust layout or a compiler-specific dynamic ABI.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

mod broker_limits;
mod service_requirements;

pub use broker_limits::BrokerConnectionLimits;
pub use broker_limits::BrokerConnectionLimitsSpec;
pub use broker_limits::BrokerLimitsError;
pub use broker_limits::BrokerProcessLimits;
pub use broker_limits::BrokerProcessLimitsSpec;
pub use broker_limits::MAX_BROKER_BLOCKING_JOBS;
pub use broker_limits::MAX_BROKER_CALLS_PER_CONNECTION;
pub use broker_limits::MAX_BROKER_CONNECTIONS;

pub use service_requirements::HostServiceRequirement;
pub use service_requirements::MAX_HOST_SERVICES;
pub use service_requirements::MAX_SERVICE_OPERATIONS;
pub use service_requirements::SERVICE_REQUIREMENTS_VERSION;
pub use service_requirements::ServicePresence;
pub use service_requirements::ServiceRequirementsError;
pub use service_requirements::ServiceRequirementsV1;

pub const COMPONENT_API_VERSION: u32 = 1;
/// Storage v2 preserves trusted native state before the implementation applies
/// its own persistence encoding. Version 1 used a lossy extra history round trip.
pub const THREAD_STORE_CONTRACT_VERSION: u32 = 2;
/// Persistent file-search providers retain bounded leases and explicit cleanup receipts.
pub const FILE_SEARCH_CONTRACT_VERSION: u32 = 1;
pub const FILE_SEARCH_KIND: &str = "file_search";
pub const MANIFEST_FILE: &str = "codex-component.json";
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComponentSpec {
    pub kind: String,
    pub name: String,
    pub contract_version: u32,
    #[serde(default)]
    pub metadata: Value,
    /// A declaration requests services; only negotiated host grants authorize calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_requirements: Option<ServiceRequirementsV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComponentManifest {
    pub api_version: u32,
    pub id: String,
    pub version: String,
    /// Package-relative executable. No shell command expansion is performed.
    pub entrypoint: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    pub components: Vec<ComponentSpec>,
}

/// User-owned activation/configuration; installation alone never launches code.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComponentSettings {
    /// Plugin identity -> immutable package object basename. Removing this entry
    /// affects future catalogs; existing bindings retain their original object.
    #[serde(default)]
    pub installed: BTreeMap<String, String>,
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
    /// Explicit `kind:name` -> plugin identity selection for replacements.
    #[serde(default)]
    pub selections: BTreeMap<String, String>,
}

#[cfg(test)]
#[path = "service_requirements_tests.rs"]
mod service_requirements_tests;

#[cfg(test)]
#[path = "broker_limits_tests.rs"]
mod broker_limits_tests;
