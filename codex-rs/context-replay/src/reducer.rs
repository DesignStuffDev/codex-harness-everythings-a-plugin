//! Native rollout reconstruction with explicit inputs and a history semantic dependency.
// Apache-2.0; derived from core/src/session/rollout_reconstruction.rs without changing selection order.

use codex_history::RolloutItem;
use codex_history::PreviousTurnSettings;
use codex_protocol::protocol::EventMsg;
use crate::ReplayHistory;
use crate::ReplayInput;
use crate::RolloutReconstruction;
use crate::WorldStateSnapshot;
use crate::selection::*;

/// Reconstruct one owned rollout without session access, I/O, or asynchronous callbacks.
///
/// The history implementation owns item replay/retention semantics; it must be initialized
/// exclusively from the supplied policy and may not reuse state from another reconstruction.
pub fn reconstruct<H: ReplayHistory>(input: ReplayInput) -> RolloutReconstruction {
    let rollout_items = input.items.as_slice();
    // Select the compaction and suffix that can affect reconstruction.
    let has_legacy_compaction_without_window_number =
        rollout_items.iter().any(|item| {
            matches!(item, RolloutItem::Compacted(compacted) if compacted.window_number.is_none())
        });
    let initial_window = if has_legacy_compaction_without_window_number {
        None
    } else {
        rollout_items.iter().find_map(|item| match item {
            RolloutItem::SessionMeta(session_meta) => session_meta
                .meta
                .context_window
                .as_ref()
                .and_then(reconstructed_window_from_session_context_window),
            _ => None,
        })
    };
    let input_checkpoint = select_input_compaction(rollout_items, input.history_mode);
    let replay_items = input_checkpoint.map_or(rollout_items, |checkpoint| checkpoint.suffix);
    let resume_metadata =
        input_checkpoint.and_then(|checkpoint| checkpoint.compacted.resume_metadata.as_ref());
    let mut history_checkpoint = input_checkpoint;
    let mut window = input_checkpoint
        .and_then(|checkpoint| reconstructed_window_from_compaction(checkpoint.compacted));

    // Scan the selected items backward to find the newest surviving turn state.
    let last_started_turn_id = replay_items
        .iter()
        .rev()
        .find_map(|item| match item {
            RolloutItem::EventMsg(EventMsg::TurnStarted(event)) => Some(event.turn_id.clone()),
            RolloutItem::SessionMeta(_)
            | RolloutItem::ResponseItem(_)
            | RolloutItem::InterAgentCommunication(_)
            | RolloutItem::InterAgentCommunicationMetadata { .. }
            | RolloutItem::TurnContext(_)
            | RolloutItem::WorldState(_)
            | RolloutItem::RetainedContext(_)
            | RolloutItem::SecurityRiskScore(_)
            | RolloutItem::TokenUsageRecord(_)
            | RolloutItem::RealtimeItem(_)
            | RolloutItem::Compacted(_)
            | RolloutItem::EventMsg(_) => None,
        })
        .or_else(|| resume_metadata.and_then(|metadata| metadata.last_started_turn_id.clone()));

    let mut previous_turn_settings = None;
    let mut reference_context_item = TurnReferenceContextItem::NeverSet;
    let mut world_state_replay = Vec::new();
    // Rollback is "drop the newest N user turns". While scanning in reverse, that becomes
    // "skip the next N user-turn segments we finalize".
    let mut pending_rollback_turns = 0usize;
    // Reverse replay accumulates rollout items into the newest in-progress turn segment until
    // we hit its matching `TurnStarted`, at which point the segment can be finalized.
    let mut active_segment: Option<ActiveReplaySegment<'_>> = None;

    for (index, item) in replay_items.iter().enumerate().rev() {
        match item {
            RolloutItem::Compacted(compacted) => {
                let active_segment =
                    active_segment.get_or_insert_with(ActiveReplaySegment::default);
                active_segment.world_state_replay.push(item);
                if active_segment.window.is_none()
                    && let Some(compaction_window) =
                        reconstructed_window_from_compaction(compacted)
                {
                    active_segment.window = Some(compaction_window);
                }
                // Looking backward, compaction clears any older baseline unless a newer
                // `TurnContextItem` in this same segment has already re-established it.
                if matches!(
                    active_segment.reference_context_item,
                    TurnReferenceContextItem::NeverSet
                ) {
                    active_segment.reference_context_item = TurnReferenceContextItem::Cleared;
                }
                if active_segment.history_checkpoint.is_none()
                    && compacted.replacement_history.is_some()
                {
                    active_segment.history_checkpoint = Some(ReplayCheckpoint {
                        compacted,
                        suffix: &replay_items[index + 1..],
                    });
                }
            }
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(rollback)) => {
                pending_rollback_turns = pending_rollback_turns
                    .saturating_add(usize::try_from(rollback.num_turns).unwrap_or(usize::MAX));
            }
            RolloutItem::EventMsg(EventMsg::TurnComplete(event)) => {
                let active_segment =
                    active_segment.get_or_insert_with(ActiveReplaySegment::default);
                active_segment.turn_completed = true;
                // Reverse replay often sees `TurnComplete` before any turn-scoped metadata.
                // Capture the turn id early so later `TurnContext` / abort items can match it.
                if active_segment.turn_id.is_none() {
                    active_segment.turn_id = Some(event.turn_id.clone());
                }
            }
            RolloutItem::EventMsg(EventMsg::TurnAborted(event)) => {
                if let Some(active_segment) = active_segment.as_mut() {
                    if active_segment.turn_id.is_none()
                        && let Some(turn_id) = &event.turn_id
                    {
                        active_segment.turn_id = Some(turn_id.clone());
                    }
                } else if let Some(turn_id) = &event.turn_id {
                    active_segment = Some(ActiveReplaySegment {
                        turn_id: Some(turn_id.clone()),
                        ..Default::default()
                    });
                }
            }
            RolloutItem::EventMsg(EventMsg::UserMessage(_)) => {
                let active_segment =
                    active_segment.get_or_insert_with(ActiveReplaySegment::default);
                active_segment.counts_as_user_turn = true;
            }
            RolloutItem::TurnContext(ctx) => {
                let active_segment =
                    active_segment.get_or_insert_with(ActiveReplaySegment::default);
                // `TurnContextItem` can attach metadata to an existing segment, but only a
                // real `UserMessage` event should make the segment count as a user turn.
                if active_segment.turn_id.is_none() {
                    active_segment.turn_id = ctx.turn_id.clone();
                }
                if turn_ids_are_compatible(
                    active_segment.turn_id.as_deref(),
                    ctx.turn_id.as_deref(),
                ) {
                    active_segment.previous_turn_settings = Some(PreviousTurnSettings {
                        model: ctx.model.clone(),
                        cyber_access_program: ctx.cyber_access_program,
                        comp_hash: ctx.comp_hash.clone(),
                        realtime_active: ctx.realtime_active,
                    });
                    if matches!(
                        active_segment.reference_context_item,
                        TurnReferenceContextItem::NeverSet
                    ) {
                        active_segment.reference_context_item =
                            TurnReferenceContextItem::Latest(Box::new(ctx.clone()));
                    }
                }
            }
            RolloutItem::WorldState(_) => {
                let active_segment =
                    active_segment.get_or_insert_with(ActiveReplaySegment::default);
                active_segment.world_state_replay.push(item);
            }
            RolloutItem::EventMsg(EventMsg::TurnStarted(event)) => {
                // `TurnStarted` is the oldest boundary of the active reverse segment.
                if active_segment.as_ref().is_some_and(|active_segment| {
                    turn_ids_are_compatible(
                        active_segment.turn_id.as_deref(),
                        Some(event.turn_id.as_str()),
                    )
                }) && let Some(active_segment) = active_segment.take()
                {
                    finalize_active_segment(
                        active_segment,
                        &mut history_checkpoint,
                        &mut previous_turn_settings,
                        &mut reference_context_item,
                        &mut world_state_replay,
                        &mut window,
                        &mut pending_rollback_turns,
                    );
                }
            }
            RolloutItem::ResponseItem(response_item) => {
                let active_segment =
                    active_segment.get_or_insert_with(ActiveReplaySegment::default);
                active_segment.counts_as_user_turn |=
                    H::is_user_turn_boundary(&response_item.item);
            }
            RolloutItem::InterAgentCommunication(_) => {
                let active_segment =
                    active_segment.get_or_insert_with(ActiveReplaySegment::default);
                active_segment.counts_as_user_turn = true;
            }
            RolloutItem::EventMsg(_)
            | RolloutItem::SessionMeta(_)
            | RolloutItem::RealtimeItem(_)
            | RolloutItem::RetainedContext(_)
            | RolloutItem::SecurityRiskScore(_)
            | RolloutItem::TokenUsageRecord(_)
            | RolloutItem::InterAgentCommunicationMetadata { .. } => {}
        }
    }

    if let Some(mut active_segment) = active_segment.take() {
        // A companion turn context only restores the context baseline. Once that turn
        // completes, its settings are newer than the compaction metadata.
        if resume_metadata.is_some() && !active_segment.turn_completed {
            active_segment.previous_turn_settings = None;
        }
        finalize_active_segment(
            active_segment,
            &mut history_checkpoint,
            &mut previous_turn_settings,
            &mut reference_context_item,
            &mut world_state_replay,
            &mut window,
            &mut pending_rollback_turns,
        );
    }

    if previous_turn_settings.is_none() {
        previous_turn_settings =
            resume_metadata.and_then(|metadata| metadata.previous_turn_settings.clone());
    }

    let fallback_window_number = u64::try_from(
        rollout_items
            .iter()
            .filter(|item| matches!(item, RolloutItem::Compacted(_)))
            .count(),
    )
    .unwrap_or(u64::MAX);

    // Build model-visible history from the selected compaction and its newer suffix.
    let mut history = H::new(input.review_policy);
    let mut saw_legacy_compaction_without_replacement_history = false;
    if let Some(checkpoint) = history_checkpoint
        && let Some(items) = &checkpoint.compacted.replacement_history
    {
        history.replace_annotated(items.clone());
        history.restore_review_context(
            checkpoint.compacted.retained_context.as_ref(),
            checkpoint.compacted.guardian_history.as_ref(),
            // Keep the backup during replay; the installing session resolves its reviewer.
            /*reviewer_compaction_hash*/
            None,
        );
    }
    let rollout_suffix =
        history_checkpoint.map_or(rollout_items, |checkpoint| checkpoint.suffix);
    for item in rollout_suffix {
        match item {
            RolloutItem::RetainedContext(event) => {
                history.record_retained_context(event);
            }
            RolloutItem::ResponseItem(response_item) => {
                history.replay_annotated_item(
                    response_item,
                    input.truncation_policy,
                );
            }
            RolloutItem::InterAgentCommunication(communication) => {
                let response_item = communication.to_model_input_item();
                history.record_legacy_communication(
                    &response_item,
                    input.truncation_policy,
                );
            }
            RolloutItem::InterAgentCommunicationMetadata { .. } => {}
            RolloutItem::Compacted(compacted) => {
                // Reverse replay already chose the newest surviving compaction. Any newer
                // replacement compaction belongs to a rolled-back turn; replay its original
                // items so the rollback can still find the removed user boundary.
                if compacted.replacement_history.is_none() {
                    saw_legacy_compaction_without_replacement_history = true;
                    // Legacy rollouts without `replacement_history` should rebuild the
                    // historical TurnContext at the correct insertion point from persisted
                    // `TurnContextItem`s. These are rare enough that we currently just clear
                    // `reference_context_item`, reinject canonical context at the end of the
                    // resumed conversation, and accept the temporary out-of-distribution
                    // prompt shape.
                    // TODO(ccunningham): if we drop support for None replacement_history compaction items,
                    // we can get rid of this second loop entirely and just build `history` directly in the first loop.
                    history.apply_legacy_compaction(&compacted.message);
                }
            }
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(rollback)) => {
                history.drop_last_n_user_turns(rollback.num_turns);
            }
            RolloutItem::EventMsg(_)
            | RolloutItem::TurnContext(_)
            | RolloutItem::RealtimeItem(_)
            | RolloutItem::WorldState(_)
            | RolloutItem::SecurityRiskScore(_)
            | RolloutItem::TokenUsageRecord(_)
            | RolloutItem::SessionMeta(_) => {}
        }
    }

    let reference_context_item = match reference_context_item {
        TurnReferenceContextItem::NeverSet | TurnReferenceContextItem::Cleared => None,
        TurnReferenceContextItem::Latest(turn_reference_context_item) => {
            Some(*turn_reference_context_item)
        }
    };
    let reference_context_item = if saw_legacy_compaction_without_replacement_history {
        None
    } else {
        reference_context_item
    };

    // Replay the collected world-state records chronologically so compaction resets and merge
    // patches keep their original meaning.
    world_state_replay.reverse();
    let mut world_state_baseline: Option<WorldStateSnapshot> = None;
    for item in world_state_replay {
        match item {
            RolloutItem::Compacted(_) => world_state_baseline = None,
            RolloutItem::WorldState(world_state) if world_state.full => {
                world_state_baseline = Some(WorldStateSnapshot::from(&world_state.state));
            }
            RolloutItem::WorldState(world_state) => {
                let Some(baseline) = world_state_baseline.as_mut() else {
                    tracing::warn!("ignored world-state patch without a full snapshot");
                    continue;
                };
                baseline.apply_merge_patch(&world_state.state);
            }
            RolloutItem::SessionMeta(_)
            | RolloutItem::ResponseItem(_)
            | RolloutItem::InterAgentCommunication(_)
            | RolloutItem::InterAgentCommunicationMetadata { .. }
            | RolloutItem::TurnContext(_)
            | RolloutItem::RealtimeItem(_)
            | RolloutItem::TokenUsageRecord(_)
            | RolloutItem::RetainedContext(_)
            | RolloutItem::SecurityRiskScore(_)
            | RolloutItem::EventMsg(_) => {
                unreachable!("only world-state replay items are collected")
            }
        }
    }

    let window = window.or(initial_window).unwrap_or(ReconstructedWindow {
        number: fallback_window_number,
        first_id: None,
        previous_id: None,
        id: None,
    });
    let projection = history.finish();
    RolloutReconstruction {
        retained_context: projection.retained_context,
        guardian_history: projection.guardian_history,
        last_started_turn_id,
        history: projection.history,
        previous_turn_settings,
        reference_context_item,
        world_state_baseline,
        window_number: window.number,
        first_window_id: window.first_id,
        previous_window_id: window.previous_id,
        window_id: window.id,
    }
}
