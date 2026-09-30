use serde::Deserialize;
use serde::Serialize;

/// Native usage, including budget units omitted by ordinary provider serialization.
/// Signed counters and every valid JSON number retain their native policy owner.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    #[serde(default)]
    pub cache_write_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
    pub codex_rollout_budget_units: Option<serde_json::Number>,
}

#[cfg(feature = "native")]
impl From<codex_protocol::protocol::TokenUsage> for Usage {
    fn from(usage: codex_protocol::protocol::TokenUsage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            cache_write_input_tokens: usage.cache_write_input_tokens,
            output_tokens: usage.output_tokens,
            reasoning_output_tokens: usage.reasoning_output_tokens,
            total_tokens: usage.total_tokens,
            codex_rollout_budget_units: usage.codex_rollout_budget_units,
        }
    }
}

#[cfg(feature = "native")]
impl From<Usage> for codex_protocol::protocol::TokenUsage {
    fn from(usage: Usage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            cache_write_input_tokens: usage.cache_write_input_tokens,
            output_tokens: usage.output_tokens,
            reasoning_output_tokens: usage.reasoning_output_tokens,
            total_tokens: usage.total_tokens,
            codex_rollout_budget_units: usage.codex_rollout_budget_units,
        }
    }
}
