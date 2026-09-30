//! Compose thread persistence separately from host-owned auxiliary SQLite services.
//!
//! The process boundary lives in `codex-thread-store-component`; this module only
//! chooses implementations at engine startup. Selection precedes construction of
//! the native store, so a replacement never starts a second set of native writers
//! or maintenance workers in the host.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use codex_component_host::ComponentCatalog;
use codex_features::Feature;
use codex_rollout::StateDbHandle;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::ThreadStore;
use codex_thread_store_component::LocalStoragePaths;
use codex_thread_store_component::ProcessThreadStore;
use codex_thread_store_component::StorageInitialization;
use codex_thread_store_component::THREAD_STORE_CONTRACT_VERSION;

use crate::config::Config;

/// Persistence dependencies with independent ownership and lifetimes.
///
/// A thread-store implementation owns history and live writers. The host keeps
/// this auxiliary database for memories, goals, queues and agent records. A
/// process implementation must negotiate compatibility with these configured
/// paths; it cannot silently remove the host's other services.
#[derive(Clone)]
pub struct PersistenceServices {
    pub thread_store: Arc<dyn ThreadStore>,
    pub host_state_db: Option<StateDbHandle>,
}

impl PersistenceServices {
    /// Preserve the existing public constructor's native behavior. Concrete
    /// detection is confined to this compatibility composition adapter, never
    /// performed by session runtime code or the selected process implementation.
    pub fn from_legacy_native(thread_store: Arc<dyn ThreadStore>) -> Self {
        let host_state_db = thread_store
            .as_any()
            .downcast_ref::<LocalThreadStore>()
            .and_then(LocalThreadStore::state_db_handle);
        Self {
            thread_store,
            host_state_db,
        }
    }
}

/// Resolve a process-scoped store before accepting sessions. An invalid selected
/// implementation is a startup error, with no fallback or implicit data moves.
pub async fn persistence_from_config(
    config: &Config,
    host_state_db: Option<StateDbHandle>,
) -> Result<PersistenceServices> {
    let catalog = ComponentCatalog::load(&config.codex_home)?;
    let Some(binding) = catalog.selected("thread_store", "default") else {
        return Ok(PersistenceServices::from_legacy_native(
            crate::thread_manager::thread_store_from_config(config, host_state_db),
        ));
    };
    let initialization = StorageInitialization {
        contract_version: THREAD_STORE_CONTRACT_VERSION,
        paths: LocalStoragePaths {
            codex_home: config.codex_home.to_path_buf(),
            sqlite_home: config.sqlite_config().home().to_path_buf(),
        },
        default_model_provider_id: config.model_provider_id.clone(),
        state_db_enabled: host_state_db.is_some(),
        startup_migration: config
            .features
            .enabled(Feature::BackgroundPaginatedRolloutMigration),
        startup_compression: config
            .features
            .enabled(Feature::LocalThreadStoreCompression),
    };
    let thread_store: Arc<dyn ThreadStore> = Arc::new(
        ProcessThreadStore::connect(binding, initialization)
            .await
            .context("initialize selected thread-store component")?,
    );
    Ok(PersistenceServices {
        thread_store,
        host_state_db,
    })
}
