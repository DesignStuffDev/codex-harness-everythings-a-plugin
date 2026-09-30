use codex_protocol::models::ContentItemKind;
use codex_protocol::models::ExecutedToolCall;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::TrustedComponentExecutedToolCall;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
#[serde(remote = "InternalChatMessageMetadataPassthrough", deny_unknown_fields)]
pub(crate) struct MetadataWire {
    turn_id: Option<String>,
    create_time: Option<serde_json::Number>,
    content_item_kinds: Option<Vec<ContentItemKind>>,
    cell_id: Option<String>,
    #[serde(with = "calls")]
    executed_tool_calls: Option<Vec<ExecutedToolCall>>,
    tool_calls_complete: Option<bool>,
}

remote_adapter!(
    metadata,
    InternalChatMessageMetadataPassthrough,
    MetadataWire,
    "MetadataWire"
);

mod calls {
    use super::*;

    pub(super) fn serialize<S: serde::Serializer>(
        calls: &Option<Vec<ExecutedToolCall>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .map(TrustedComponentExecutedToolCall::from)
                    .collect::<Vec<_>>()
            })
            .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<ExecutedToolCall>>, D::Error> {
        Option::<Vec<TrustedComponentExecutedToolCall>>::deserialize(deserializer)?
            .map(|calls| {
                calls
                    .into_iter()
                    .map(|call| ExecutedToolCall::try_from(call).map_err(serde::de::Error::custom))
                    .collect()
            })
            .transpose()
    }
}
