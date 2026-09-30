//! Lossless optional JSON and MCP DTOs; captured null remains distinct from absent data.

pub mod optional_value {
    #[derive(serde::Serialize)]
    enum Borrowed<'a> {
        Absent,
        Present(&'a serde_json::Value),
    }
    #[derive(serde::Deserialize)]
    enum Owned {
        Absent,
        Present(serde_json::Value),
    }
    pub fn serialize<S: serde::Serializer>(
        value: &Option<serde_json::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value = match value {
            Some(value) => Borrowed::Present(value),
            None => Borrowed::Absent,
        };
        serde::Serialize::serialize(&value, serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<serde_json::Value>, D::Error> {
        Ok(
            match <Owned as serde::Deserialize>::deserialize(deserializer)? {
                Owned::Present(value) => Some(value),
                Owned::Absent => None,
            },
        )
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::ResponseUsageMetadata")]
pub(crate) struct ResponseUsageMetadataWire {
    amount: Option<String>,
    #[serde(with = "optional_value")]
    metadata: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::mcp::CallToolResult")]
pub(crate) struct CallToolResultWire {
    content: Vec<serde_json::Value>,
    #[serde(with = "optional_value")]
    structured_content: Option<serde_json::Value>,
    is_error: Option<bool>,
    #[serde(with = "optional_value")]
    meta: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::McpInvocation")]
pub(crate) struct McpInvocationWire {
    server: String,
    tool: String,
    #[serde(with = "optional_value")]
    arguments: Option<serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::McpToolCallBeginEvent")]
pub(crate) struct McpToolCallBeginEventWire {
    call_id: String,
    turn_id: String,
    #[serde(with = "McpInvocationWire")]
    invocation: codex_protocol::protocol::McpInvocation,
    connector_id: Option<String>,
    mcp_app_resource_uri: Option<String>,
    mcp_app_ui: Option<codex_protocol::items::McpAppUi>,
    link_id: Option<String>,
    app_name: Option<String>,
    action_name: Option<String>,
    plugin_id: Option<String>,
    read_only_hint: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::McpToolCallEndEvent")]
pub(crate) struct McpToolCallEndEventWire {
    call_id: String,
    turn_id: String,
    #[serde(with = "McpInvocationWire")]
    invocation: codex_protocol::protocol::McpInvocation,
    connector_id: Option<String>,
    mcp_app_resource_uri: Option<String>,
    mcp_app_ui: Option<codex_protocol::items::McpAppUi>,
    link_id: Option<String>,
    app_name: Option<String>,
    action_name: Option<String>,
    plugin_id: Option<String>,
    read_only_hint: Option<bool>,
    duration: std::time::Duration,
    #[serde(with = "tool_result_result")]
    result: Result<codex_protocol::mcp::CallToolResult, String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::McpToolCallItem")]
pub(crate) struct McpToolCallItemWire {
    id: String,
    server: String,
    tool: String,
    arguments: serde_json::Value,
    connector_id: Option<String>,
    mcp_app_resource_uri: Option<String>,
    mcp_app_ui: Option<codex_protocol::items::McpAppUi>,
    link_id: Option<String>,
    app_name: Option<String>,
    action_name: Option<String>,
    plugin_id: Option<String>,
    read_only_hint: Option<bool>,
    status: codex_protocol::items::McpToolCallStatus,
    #[serde(with = "tool_result::option")]
    result: Option<codex_protocol::mcp::CallToolResult>,
    error: Option<codex_protocol::items::McpToolCallError>,
    duration: Option<std::time::Duration>,
}

remote_adapter!(
    response_usage,
    codex_protocol::ResponseUsageMetadata,
    ResponseUsageMetadataWire,
    "ResponseUsageMetadataWire"
);
remote_adapter!(
    tool_result,
    codex_protocol::mcp::CallToolResult,
    CallToolResultWire,
    "CallToolResultWire"
);

pub(crate) mod tool_result_result {
    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Result<codex_protocol::mcp::CallToolResult, String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value = value.as_ref().map(super::tool_result::Borrowed);
        serde::Serialize::serialize(&value, serializer)
    }
    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Result<codex_protocol::mcp::CallToolResult, String>, D::Error> {
        let value = <Result<super::tool_result::Owned, String> as serde::Deserialize>::deserialize(
            deserializer,
        )?;
        Ok(value.map(|value| value.0))
    }
}
