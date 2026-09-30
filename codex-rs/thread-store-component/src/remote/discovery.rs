//! Storage-specific lossless mirrors; native serde stays unchanged.

use chrono::DateTime;
use chrono::Utc;
use codex_protocol::ThreadId;
use codex_protocol::models::PermissionProfile;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::GitInfo;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TokenUsage;
use codex_thread_store::ClearableField;
use codex_thread_store::ExtraConfig;
use codex_thread_store::ListThreadsParams;
use codex_thread_store::ReadThreadByRolloutPathParams;
use codex_thread_store::SortDirection;
use codex_thread_store::StoredThread;
use codex_thread_store::StoredThreadHistory;
use codex_thread_store::StoredThreadSearchResult;
use codex_thread_store::ThreadPage;
use codex_thread_store::ThreadRelationFilter;
use codex_thread_store::ThreadSearchPage;
use codex_thread_store::ThreadSortKey;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(remote = "ReadThreadByRolloutPathParams", deny_unknown_fields)]
struct ReadThreadByRolloutPathParamsWire {
    #[serde(with = "codex_component_state_codec::native_path")]
    rollout_path: PathBuf,
    include_archived: bool,
    include_history: bool,
}
remote_adapter!(
    read_thread_by_rollout_path_params,
    ReadThreadByRolloutPathParams,
    ReadThreadByRolloutPathParamsWire,
    "ReadThreadByRolloutPathParamsWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ListThreadsParams", deny_unknown_fields)]
struct ListThreadsParamsWire {
    page_size: usize,
    cursor: Option<String>,
    sort_key: ThreadSortKey,
    sort_direction: SortDirection,
    allowed_sources: Vec<SessionSource>,
    model_providers: Option<Vec<String>>,
    #[serde(with = "codex_component_state_codec::native_path::option_vec")]
    cwd_filters: Option<Vec<PathBuf>>,
    #[serde(with = "super::clearable")]
    section: Option<Option<String>>,
    #[serde(with = "super::clearable")]
    project_id: ClearableField<String>,
    archived: bool,
    search_term: Option<String>,
    relation_filter: Option<ThreadRelationFilter>,
    use_state_db_only: bool,
}
remote_adapter!(
    list_threads_params,
    ListThreadsParams,
    ListThreadsParamsWire,
    "ListThreadsParamsWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "StoredThread", deny_unknown_fields)]
struct StoredThreadWire {
    originator: Option<String>,
    thread_id: ThreadId,
    extra_config: Option<ExtraConfig>,
    #[serde(with = "codex_component_state_codec::native_path::option")]
    rollout_path: Option<PathBuf>,
    forked_from_id: Option<ThreadId>,
    parent_thread_id: Option<ThreadId>,
    preview: String,
    name: Option<String>,
    model_provider: String,
    model: Option<String>,
    #[serde(with = "codex_component_state_codec::reasoning_effort::option")]
    reasoning_effort: Option<ReasoningEffort>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    recency_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    section: Option<codex_state::ThreadSection>,
    section_position: Option<i64>,
    section_entered_at: Option<DateTime<Utc>>,
    project_id: Option<String>,
    daybreak_enabled: Option<bool>,
    #[serde(with = "codex_component_state_codec::native_path")]
    cwd: PathBuf,
    cli_version: String,
    source: SessionSource,
    history_mode: ThreadHistoryMode,
    #[serde(with = "codex_component_state_codec::thread_source::option")]
    thread_source: Option<ThreadSource>,
    agent_nickname: Option<String>,
    agent_role: Option<String>,
    agent_path: Option<String>,
    git_info: Option<GitInfo>,
    approval_mode: AskForApproval,
    #[serde(with = "codex_component_state_codec::permission_profile")]
    permission_profile: PermissionProfile,
    #[serde(with = "codex_component_state_codec::token_usage::option")]
    token_usage: Option<TokenUsage>,
    first_user_message: Option<String>,
    #[serde(with = "super::stored_thread_history::option")]
    history: Option<StoredThreadHistory>,
}
remote_adapter!(
    stored_thread,
    StoredThread,
    StoredThreadWire,
    "StoredThreadWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ThreadPage", deny_unknown_fields)]
struct ThreadPageWire {
    #[serde(with = "super::stored_thread::vec")]
    items: Vec<StoredThread>,
    next_cursor: Option<String>,
}
remote_adapter!(thread_page, ThreadPage, ThreadPageWire, "ThreadPageWire");

#[derive(Serialize, Deserialize)]
#[serde(remote = "StoredThreadSearchResult", deny_unknown_fields)]
struct StoredThreadSearchResultWire {
    #[serde(with = "super::stored_thread")]
    thread: StoredThread,
    snippet: String,
}
remote_adapter!(
    stored_thread_search_result,
    StoredThreadSearchResult,
    StoredThreadSearchResultWire,
    "StoredThreadSearchResultWire"
);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ThreadSearchPage", deny_unknown_fields)]
struct ThreadSearchPageWire {
    #[serde(with = "super::stored_thread_search_result::vec")]
    items: Vec<StoredThreadSearchResult>,
    next_cursor: Option<String>,
}
remote_adapter!(
    thread_search_page,
    ThreadSearchPage,
    ThreadSearchPageWire,
    "ThreadSearchPageWire"
);
