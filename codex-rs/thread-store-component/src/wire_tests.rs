use std::sync::Arc;

use codex_component_state_codec::rollout_item;
use codex_protocol::ThreadId;
use codex_protocol::models::ExecutedToolCall;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ThreadMemoryMode;
use codex_rollout::RolloutItem;
use codex_thread_store::AppendThreadItemsParams;
use codex_thread_store::ResumeThreadParams;
use codex_thread_store::StoredModelContext;
use codex_thread_store::StoredThreadHistory;
use codex_thread_store::ThreadPersistenceMetadata;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

use super::StorageReply;
use super::StorageRequest;
use super::StorageResponse;
use crate::PreparedForkResponse;

fn evidence() -> RolloutItem {
    let mut output = FunctionCallOutputPayload::from_text("trusted tool result".to_owned());
    output.success = Some(false);
    let arguments: Value = serde_json::from_str(
        r#"{"integer":123456789012345678901234567890,"decimal":0.12345678901234567890123456789}"#,
    )
    .unwrap();
    RolloutItem::ResponseItem(
        ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("call-evidence".to_owned()),
            namespace: None,
            name: Some("exec".to_owned()),
            output,
            internal_chat_message_metadata_passthrough: Some(
                InternalChatMessageMetadataPassthrough {
                    turn_id: Some("turn-evidence".to_owned()),
                    cell_id: Some("cell-evidence".to_owned()),
                    executed_tool_calls: Some(vec![ExecutedToolCall::new(
                        "evaluate".to_owned(),
                        arguments,
                    )]),
                    tool_calls_complete: Some(false),
                    ..Default::default()
                },
            ),
        }
        .into(),
    )
}

fn snapshot(items: &[RolloutItem]) -> Vec<Value> {
    items
        .iter()
        .map(|item| serde_json::to_value(rollout_item::Borrowed(item)).unwrap())
        .collect()
}

#[test]
fn acquisition_and_append_preserve_typed_history_before_backend_policy() {
    let id = ThreadId::new();
    let items = vec![evidence()];
    let expected = snapshot(&items);
    let requests = [
        StorageRequest::AppendItems(AppendThreadItemsParams {
            thread_id: id,
            items: items.clone(),
        }),
        StorageRequest::ResumeThread(ResumeThreadParams {
            thread_id: id,
            rollout_path: None,
            history: Some(Arc::new(items)),
            include_archived: true,
            metadata: ThreadPersistenceMetadata {
                cwd: None,
                model_provider: "provider".to_owned(),
                memory_mode: ThreadMemoryMode::Enabled,
            },
        }),
    ];
    for request in requests {
        let encoded = serde_json::to_value(request).unwrap();
        let decoded: StorageRequest = serde_json::from_value(encoded).unwrap();
        let actual = match decoded {
            StorageRequest::AppendItems(params) => params.items,
            StorageRequest::ResumeThread(params) => params.history.unwrap().as_ref().clone(),
            other => panic!("unexpected request: {other:?}"),
        };
        assert_eq!(snapshot(&actual), expected);
    }
}

#[test]
fn load_and_fork_replies_preserve_backend_owned_context() {
    let id = ThreadId::new();
    let items = vec![evidence()];
    let expected = snapshot(&items);
    let replies = [
        StorageResponse::LoadHistory(StoredThreadHistory {
            thread_id: id,
            items: items.clone(),
        }),
        StorageResponse::LoadLatestModelContext(StoredModelContext {
            thread_id: id,
            items: items.clone(),
        }),
        StorageResponse::PrepareFork(PreparedForkResponse {
            source_thread_id: id,
            history_base: Some(codex_protocol::protocol::HistoryPosition {
                thread_id: id,
                end_ordinal_exclusive: 7,
                end_byte_offset: 700,
            }),
            model_context: StoredModelContext {
                thread_id: id,
                items,
            },
        }),
    ];
    for response in replies {
        let encoded = serde_json::to_value(StorageReply::Ok(Box::new(response))).unwrap();
        let decoded: StorageReply = serde_json::from_value(encoded).unwrap();
        let StorageReply::Ok(response) = decoded else {
            panic!("reply became an error")
        };
        let actual = match *response {
            StorageResponse::LoadHistory(history) => history.items,
            StorageResponse::LoadLatestModelContext(context) => context.items,
            StorageResponse::PrepareFork(prepared) => {
                assert_eq!(prepared.source_thread_id, id);
                assert_eq!(prepared.history_base.unwrap().end_ordinal_exclusive, 7);
                prepared.model_context.items
            }
            other => panic!("unexpected reply: {other:?}"),
        };
        assert_eq!(snapshot(&actual), expected);
    }
}

#[test]
fn old_storage_wire_is_rejected_instead_of_reinterpreted() {
    assert!(
        serde_json::from_value::<StorageRequest>(json!({
            "operation":"flush_thread", "params":ThreadId::new()
        }))
        .is_err()
    );
}
