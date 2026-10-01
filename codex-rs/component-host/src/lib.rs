//! Discovery and process isolation for explicitly installed trusted engine components.
//! A catalog is an activation snapshot. Changes affect subsequently loaded catalogs;
//! existing bindings retain their installed code and configuration.
//!
//! Installed entrypoints run directly with their declared argv, without shell
//! expansion, and use their immutable package directory as cwd. Windows Python
//! zipapps run through Python. Configuration and the plugin-owned durable state
//! directory are passed through the initialization frame; they are not expanded
//! into argv. Diagnostic stderr is inherited except for auth and attachment
//! components, whose stderr is discarded to protect sensitive material.
//!
//! `call`/`stream` start a fresh child per invocation. `connect` requires an
//! explicit persistent-session handshake and retains one process for concurrent
//! calls. Persistent cancellation abandons only the waiter; accepted operations
//! stay supervised and are never automatically replayed after a disconnect.

mod catalog;
mod management;
mod process;
mod session;
mod session_reply;
mod session_server;
mod session_supervisor;
mod session_wire;

pub use catalog::ComponentCatalog;
pub use codex_component_api::ComponentManifest;
pub use codex_component_api::ComponentSettings;
pub use codex_component_api::ComponentSpec;
pub use codex_component_api::THREAD_STORE_CONTRACT_VERSION;
pub use management::install;
pub use management::remove;
pub use management::select;
pub use process::ComponentShutdown;
pub use process::ComponentStartup;
pub use process::ComponentStream;
pub use process::StreamFrame;
pub use session::ComponentSession;
pub use session::DeferredControl;
pub use session_reply::PendingComponentReply;
pub use session_server::ComponentServer;
pub use session_server::ComponentServerRequest;
pub use session_server::SessionInitialization;
pub use session_wire::SessionComponent;

use std::path::PathBuf;

/// An immutable resolved component; no child starts until a request is made.
#[derive(Clone)]
pub struct ComponentBinding {
    pub plugin_id: String,
    pub spec: ComponentSpec,
    pub config: serde_json::Value,
    pub package_dir: PathBuf,
    pub state_dir: PathBuf,
    pub entrypoint: PathBuf,
    pub args: Vec<String>,
    pub timeout_ms: u64,
}

impl std::fmt::Debug for ComponentBinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComponentBinding")
            .field("plugin_id", &self.plugin_id)
            .field("kind", &self.spec.kind)
            .field("name", &self.spec.name)
            .field("entrypoint", &self.entrypoint)
            .field("state_dir", &self.state_dir)
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}
