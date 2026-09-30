//! Staged differential coverage; link after the new crate and native bridge are integrated.
//! Uses the actual extracted native history engine without a core ReplayHistory adapter.

use super::tests::make_session_and_context;
use codex_context_engine::ContextManager;
use codex_context_replay::ReplayInput;
use codex_context_replay::ReviewPolicy;
use codex_context_replay::RolloutReconstruction;
use codex_features::Feature;
use codex_history::CompactedItem;
use codex_history::ResponseItemEnvelope;
use codex_history::RolloutItem;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::ThreadRolledBackEvent;
use codex_protocol::protocol::WorldStateItem;
use pretty_assertions::assert_eq;
use serde_json::json;
use uuid::Uuid;

fn message(id: &str, text: &str) -> ResponseItemEnvelope {
    ResponseItemEnvelope::new(ResponseItem::Message {
        id: Some(codex_protocol::ResponseItemId::with_suffix("msg", id)),
        role: "user".to_owned(),
        content: vec![ContentItem::InputText { text: text.to_owned() }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    })
}

fn checkpoint(items: Option<Vec<ResponseItemEnvelope>>) -> RolloutItem {
    RolloutItem::Compacted(CompactedItem {
        message: "legacy summary".to_owned(),
        replacement_history: items,
        guardian_history: None,
        retained_context: None,
        mcp_resource_origins: None,
        window_number: Some(2),
        first_window_id: Some(Uuid::now_v7().to_string()),
        previous_window_id: None,
        window_id: Some(Uuid::now_v7().to_string()),
        compaction_response_id: None,
        latest_token_usage_record: None,
        resume_metadata: None,
    })
}

#[tokio::test]
async fn standalone_native_engine_replay_matches_core_and_all_companion_metadata() {
    let (session, mut turn_context) = make_session_and_context().await;
    let context = turn_context.to_turn_context_item();
    let old = message("old", "first accepted input");
    let recent = message("recent", "second accepted input");
    let cases = [
        vec![
            RolloutItem::ResponseItem(old.clone()),
            RolloutItem::TurnContext(context.clone()),
            RolloutItem::WorldState(WorldStateItem::full(json!({"tools":{"old":1}}).as_object().expect("object").clone())),
            RolloutItem::WorldState(WorldStateItem::patch(json!({"tools":{"old":null,"new":[null,2]}}).as_object().expect("object").clone())),
        ],
        vec![
            checkpoint(Some(vec![old.clone()])),
            RolloutItem::TurnContext(context.clone()),
            RolloutItem::ResponseItem(recent.clone()),
            RolloutItem::WorldState(WorldStateItem::full(json!({"after":"checkpoint"}).as_object().expect("object").clone())),
        ],
        vec![
            RolloutItem::ResponseItem(old.clone()),
            checkpoint(None),
            RolloutItem::TurnContext(context),
            RolloutItem::ResponseItem(recent.clone()),
        ],
        vec![
            RolloutItem::ResponseItem(old.clone()),
            RolloutItem::ResponseItem(recent),
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(ThreadRolledBackEvent { num_turns: 1 })),
        ],
        vec![checkpoint(Some(vec![old])), checkpoint(None)],
    ];
    for history_mode in [ThreadHistoryMode::Legacy, ThreadHistoryMode::Paginated] {
        turn_context.history_mode = history_mode;
        for items in &cases {
            let legacy = session.reconstruct_history_from_rollout(&turn_context, items).await;
            let expected = RolloutReconstruction {
                history: legacy.history,
                retained_context: legacy.retained_context,
                guardian_history: legacy.guardian_history,
                last_started_turn_id: legacy.last_started_turn_id,
                previous_turn_settings: legacy.previous_turn_settings,
                reference_context_item: legacy.reference_context_item,
                world_state_baseline: legacy.world_state_baseline.map(|snapshot|
                    codex_context_replay::WorldStateSnapshot::from(&snapshot.into_object())),
                window_number: legacy.window_number,
                first_window_id: legacy.first_window_id,
                previous_window_id: legacy.previous_window_id,
                window_id: legacy.window_id,
            };
            let input = ReplayInput {
                items: items.clone(),
                history_mode,
                truncation_policy: turn_context.model_info().truncation_policy.into(),
                review_policy: ReviewPolicy {
                    independent_review: !turn_context.config.features.enabled(Feature::GuardianReuseParentCompaction),
                    retain_inherited_user_messages: !turn_context.session_source.is_non_root_agent(),
                },
            };
            let input = serde_json::from_slice(&serde_json::to_vec(&input).expect("owned replay request")).expect("wire replay request");
            let actual = codex_context_replay::reconstruct::<ContextManager>(input);
            let actual: RolloutReconstruction = serde_json::from_slice(&serde_json::to_vec(&actual).expect("owned replay response")).expect("wire replay response");
            assert_eq!(actual, expected);
        }
    }
}
