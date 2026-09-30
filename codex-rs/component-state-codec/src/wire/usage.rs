use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::protocol::TokenUsage;
use codex_protocol::protocol::TokenUsageInfo;
use codex_protocol::protocol::TokenUsageRecord;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
#[serde(remote = "TokenUsage", deny_unknown_fields)]
pub(crate) struct TokenUsageWire {
    input_tokens: i64,
    cached_input_tokens: i64,
    cache_write_input_tokens: i64,
    output_tokens: i64,
    reasoning_output_tokens: i64,
    total_tokens: i64,
    codex_rollout_budget_units: Option<serde_json::Number>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "TokenUsageInfo", deny_unknown_fields)]
pub(crate) struct TokenUsageInfoWire {
    #[serde(with = "TokenUsageWire")]
    total_token_usage: TokenUsage,
    #[serde(with = "TokenUsageWire")]
    last_token_usage: TokenUsage,
    model_context_window: Option<i64>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "TokenUsageRecord", deny_unknown_fields)]
pub(crate) struct TokenUsageRecordWire {
    thread_id: ThreadId,
    turn_id: String,
    session_id: SessionId,
    root_turn_id: String,
    response_id: String,
    #[serde(with = "TokenUsageWire")]
    usage: TokenUsage,
    #[serde(with = "TokenUsageWire")]
    turn_token_usage: TokenUsage,
    #[serde(with = "TokenUsageWire")]
    thread_token_usage: TokenUsage,
}

remote_adapter!(token_usage, TokenUsage, TokenUsageWire, "TokenUsageWire");
remote_adapter!(
    token_usage_info,
    TokenUsageInfo,
    TokenUsageInfoWire,
    "TokenUsageInfoWire"
);
remote_adapter!(
    token_usage_record,
    TokenUsageRecord,
    TokenUsageRecordWire,
    "TokenUsageRecordWire"
);
