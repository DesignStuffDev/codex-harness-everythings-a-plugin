//! Versioned, language-neutral manifests for independently installed engine components.
//! The wire protocol uses JSON, never Rust layout or a compiler-specific dynamic ABI.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

pub const COMPONENT_API_VERSION: u32 = 1;
/// Ordered large one-invocation messages, explicitly negotiated by model contract 2.
pub const STREAMING_SESSION_VERSION: u32 = 1;
/// Storage v2 preserves trusted native state before the implementation applies
/// its own persistence encoding. Version 1 used a lossy extra history round trip.
pub const THREAD_STORE_CONTRACT_VERSION: u32 = 2;
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
