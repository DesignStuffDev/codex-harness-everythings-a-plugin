use std::sync::Arc;

use anyhow::Result;
use anyhow::ensure;
use codex_agent_message_board_extension::LocalAgentMessageBoard;
use codex_rollout::RolloutConfig;
use codex_rollout::StateDbHandle;
use codex_state::SqliteConfig;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::RolloutMaintenance;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use codex_thread_store::ThreadStoreShutdownGuard;
use codex_thread_store_component::StorageCapabilities;
use codex_thread_store_component::StorageInitialization;
use codex_thread_store_component::THREAD_STORE_CONTRACT_VERSION;
use codex_utils_absolute_path::AbsolutePathBuf;

pub(super) struct OpenedStore {
    pub(super) store: Arc<dyn ThreadStore>,
    pub(super) capabilities: StorageCapabilities,
    pub(super) owner: ThreadStoreShutdownGuard,
    // This runtime was constructed by the plugin, so the plugin owns its
    // reclamation worker and pools independently of the host's state handle.
    pub(super) state_db: Option<StateDbHandle>,
}

pub(super) async fn open(initialization: StorageInitialization) -> Result<OpenedStore> {
    ensure!(
        initialization.contract_version == THREAD_STORE_CONTRACT_VERSION,
        "unsupported thread-store contract"
    );
    let paths = initialization.paths;
    ensure!(
        paths.codex_home.is_absolute(),
        "codex_home must be absolute"
    );
    let sqlite =
        SqliteConfig::from_sqlite_home(AbsolutePathBuf::from_absolute_path(&paths.sqlite_home)?);
    let state_db = if initialization.state_db_enabled {
        Some(
            codex_rollout::state_db::try_init(&RolloutConfig {
                codex_home: paths.codex_home.clone(),
                sqlite: sqlite.clone(),
                cwd: paths.codex_home.clone(),
                model_provider_id: initialization.default_model_provider_id.clone(),
                // Initialization only uses roots and provider; individual writes
                // carry their original ThreadPersistenceMetadata memory mode.
                generate_memories: false,
            })
            .await?,
        )
    } else {
        None
    };
    let cleanup_sqlite = sqlite.clone();
    let store = Arc::new(
        LocalThreadStore::new(
            LocalThreadStoreConfig {
                codex_home: paths.codex_home.clone(),
                sqlite,
                default_model_provider_id: initialization.default_model_provider_id,
            },
            state_db.clone(),
        )
        .with_thread_data_cleanup(move |thread_ids| {
            let sqlite = cleanup_sqlite.clone();
            Box::pin(async move {
                let boards = thread_ids.into_iter().map(Into::into).collect::<Vec<_>>();
                LocalAgentMessageBoard::delete_boards(&sqlite, &boards)
                    .await
                    .map_err(|error| ThreadStoreError::Internal {
                        message: format!("failed to delete agent message boards: {error}"),
                    })
            })
        }),
    );
    let owner = ThreadStoreShutdownGuard::new(store.clone());
    let scheduling = store
        .run_rollout_maintenance(RolloutMaintenance::Startup {
            migrate_rollouts: initialization.startup_migration,
            compress_rollouts: initialization.startup_compression,
        })
        .await;
    if let Err(error) = scheduling {
        let cleanup = owner.shutdown().await;
        if let Some(state_db) = &state_db {
            state_db.close().await;
        }
        return match cleanup {
            Ok(()) => Err(error.into()),
            Err(cleanup) => Err(anyhow::anyhow!(
                "native storage initialization failed: {error}; maintenance cleanup failed: {cleanup}"
            )),
        };
    }
    let capabilities = StorageCapabilities::for_store(store.as_ref(), paths);
    Ok(OpenedStore {
        store,
        capabilities,
        owner,
        state_db,
    })
}
