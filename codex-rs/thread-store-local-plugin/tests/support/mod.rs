// Fixture construction must fail the test immediately if its setup is invalid.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use codex_component_host::ComponentBinding;
use codex_component_host::ComponentCatalog;
use codex_protocol::ThreadId;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadMemoryMode;
use codex_protocol::protocol::TurnCompleteEvent;
use codex_protocol::protocol::TurnStartedEvent;
use codex_protocol::protocol::UserMessageEvent;
use codex_rollout::RolloutConfig;
use codex_rollout::RolloutItem;
use codex_state::SqliteConfig;
use codex_thread_store::CreateThreadParams;
use codex_thread_store::ListThreadsParams;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::SortDirection;
use codex_thread_store::ThreadPersistenceMetadata;
use codex_thread_store::ThreadSortKey;
use codex_thread_store_component::LocalStoragePaths;
use codex_thread_store_component::ProcessThreadStore;
use codex_thread_store_component::StorageInitialization;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde_json::json;
use tempfile::TempDir;

pub struct Fixture {
    pub _root: TempDir,
    pub binding: ComponentBinding,
    pub initialization: StorageInitialization,
}

impl Fixture {
    pub fn install() -> Self {
        let root = tempfile::tempdir().expect("external storage package fixture");
        let package = root.path().join("package");
        fs::create_dir(&package).expect("package directory");
        let executable = format!("storage{}", std::env::consts::EXE_SUFFIX);
        fs::copy(
            codex_utils_cargo_bin::cargo_bin("codex-thread-store-local-plugin").unwrap(),
            package.join(&executable),
        )
        .expect("copy independently built storage subsystem binary");
        fs::write(
            package.join("codex-component.json"),
            serde_json::to_vec(&json!({
                "api_version":1,"id":"test.native-storage","version":"1.0.0",
                "entrypoint":executable,
                "components":[{"kind":"thread_store","name":"default","contract_version":2}]
            }))
            .unwrap(),
        )
        .unwrap();
        let home = root.path().join("home");
        codex_component_host::install(&home, &package)
            .expect("install storage package without rebuilding host");
        codex_component_host::select(
            &home,
            "thread_store",
            "default",
            Some("test.native-storage"),
        )
        .unwrap();
        fs::remove_dir_all(&package).expect("source package removed after installation");
        let binding = ComponentCatalog::load(&home)
            .unwrap()
            .selected("thread_store", "default")
            .unwrap();
        let initialization = StorageInitialization {
            contract_version: 2,
            paths: LocalStoragePaths {
                codex_home: home.clone(),
                sqlite_home: home,
            },
            default_model_provider_id: "test-provider".to_owned(),
            state_db_enabled: true,
            startup_migration: false,
            startup_compression: false,
        };
        Self {
            _root: root,
            binding,
            initialization,
        }
    }

    pub async fn open(&self) -> Arc<ProcessThreadStore> {
        Arc::new(
            ProcessThreadStore::connect(self.binding.clone(), self.initialization.clone())
                .await
                .expect("connect installed native storage process"),
        )
    }

    pub async fn local(&self) -> LocalThreadStore {
        let sqlite = SqliteConfig::from_sqlite_home(
            AbsolutePathBuf::from_absolute_path(&self.initialization.paths.sqlite_home).unwrap(),
        );
        let state = codex_rollout::state_db::try_init(&RolloutConfig {
            codex_home: self.initialization.paths.codex_home.clone(),
            sqlite: sqlite.clone(),
            cwd: self.initialization.paths.codex_home.clone(),
            model_provider_id: "test-provider".to_owned(),
            generate_memories: false,
        })
        .await
        .unwrap();
        LocalThreadStore::new(
            LocalThreadStoreConfig {
                codex_home: self.initialization.paths.codex_home.clone(),
                sqlite,
                default_model_provider_id: "test-provider".to_owned(),
            },
            Some(state),
        )
    }

    pub fn params(&self, thread_id: ThreadId, mode: ThreadHistoryMode) -> CreateThreadParams {
        create_params(thread_id, mode, &self.initialization.paths.codex_home)
    }
}

pub fn create_params(
    thread_id: ThreadId,
    history_mode: ThreadHistoryMode,
    cwd: &Path,
) -> CreateThreadParams {
    CreateThreadParams {
        creator_user_id: None,
        creator_account_id: None,
        session_id: thread_id.into(),
        thread_id,
        extra_config: None,
        forked_from_id: None,
        parent_thread_id: None,
        source: SessionSource::Exec,
        thread_source: None,
        originator: "component-test".to_owned(),
        base_instructions: BaseInstructions {
            text: "Storage component fixture.".to_owned(),
            ..Default::default()
        },
        dynamic_tools: Vec::new(),
        selected_capability_roots: Vec::new(),
        multi_agent_version: None,
        history_mode,
        history_base: None,
        subagent_history_start_ordinal: None,
        initial_window_id: uuid::Uuid::new_v4().to_string(),
        runtime_workspace_roots: None,
        metadata: ThreadPersistenceMetadata {
            cwd: Some(cwd.to_path_buf()),
            model_provider: "test-provider".to_owned(),
            memory_mode: ThreadMemoryMode::Enabled,
        },
    }
}

pub fn message(text: &str) -> RolloutItem {
    RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
        message: text.to_owned(),
        ..Default::default()
    }))
}

pub fn turn(thread_id: ThreadId, turn_id: &str) -> Vec<RolloutItem> {
    vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(TurnStartedEvent {
            turn_id: turn_id.to_owned(),
            root_turn_id: None,
            trace_id: None,
            started_at: Some(10),
            model_context_window: None,
            collaboration_mode_kind: Default::default(),
        })),
        message("searchable storage component message"),
        RolloutItem::ResponseItem(
            codex_protocol::models::ResponseItem::Message {
                id: None,
                role: "user".to_owned(),
                content: vec![codex_protocol::models::ContentItem::InputText {
                    text: "searchable storage component message".to_owned(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            }
            .into(),
        ),
        RolloutItem::EventMsg(EventMsg::ItemCompleted(
            codex_protocol::protocol::ItemCompletedEvent {
                thread_id,
                turn_id: turn_id.to_owned(),
                item: codex_protocol::items::TurnItem::UserMessage(
                    codex_protocol::items::UserMessageItem::new(&[
                        codex_protocol::user_input::UserInput::Text {
                            text: "searchable storage component message".to_owned(),
                            text_elements: Vec::new(),
                        },
                    ]),
                ),
                started_at_ms: Some(10),
                completed_at_ms: 20,
            },
        )),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: turn_id.to_owned(),
            last_agent_message: None,
            error: None,
            started_at: Some(10),
            completed_at: Some(20),
            duration_ms: Some(10_000),
            time_to_first_token_ms: None,
        })),
    ]
}

pub fn list_params() -> ListThreadsParams {
    ListThreadsParams {
        page_size: 100,
        cursor: None,
        sort_key: ThreadSortKey::CreatedAt,
        sort_direction: SortDirection::Desc,
        allowed_sources: Vec::new(),
        model_providers: Some(Vec::new()),
        cwd_filters: None,
        section: None,
        project_id: None,
        archived: false,
        search_term: None,
        relation_filter: None,
        use_state_db_only: true,
    }
}
