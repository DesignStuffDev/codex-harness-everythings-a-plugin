//! Native replay segment and checkpoint selection, extracted from codex-core.
// Apache-2.0; upstream provenance is recorded at the repository root.

use codex_history::CompactedItem;
use codex_history::PreviousTurnSettings;
use codex_history::RolloutItem;
use codex_protocol::protocol::SessionContextWindow;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::TurnContextItem;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReconstructedWindow {
    pub(crate) number: u64,
    pub(crate) first_id: Option<Uuid>,
    pub(crate) previous_id: Option<Uuid>,
    pub(crate) id: Option<Uuid>,
}

#[derive(Debug, Default)]
pub(crate) enum TurnReferenceContextItem {
    /// No `TurnContextItem` has been seen for this replay span yet.
    ///
    /// This differs from `Cleared`: `NeverSet` means there is no evidence this turn ever
    /// established a baseline, while `Cleared` means a baseline existed and a later compaction
    /// invalidated it. Only the latter must emit an explicit clearing segment for resume/fork
    /// hydration.
    #[default]
    NeverSet,
    /// A previously established baseline was invalidated by later compaction.
    Cleared,
    /// The latest baseline established by this replay span.
    Latest(Box<TurnContextItem>),
}

#[derive(Debug, Clone, Copy)]
// The selected compaction and its replay tail must belong to the same surviving segment.
pub(crate) struct ReplayCheckpoint<'a> {
    pub(crate) compacted: &'a CompactedItem,
    pub(crate) suffix: &'a [RolloutItem],
}

/// Selects the newest compaction that can safely bound replay.
///
/// Returns `None` when reconstruction must replay all supplied items, either because there is no
/// compaction or the newest compaction cannot bound replay.
pub(crate) fn select_input_compaction(
    rollout_items: &[RolloutItem],
    history_mode: ThreadHistoryMode,
) -> Option<ReplayCheckpoint<'_>> {
    // Only the newest compaction can bound replay. If it is incomplete, an older compaction
    // cannot replace the history or window state that the newer one may have changed.
    let (index, compacted) = rollout_items
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, item)| match item {
            RolloutItem::Compacted(compacted) => Some((index, compacted)),
            _ => None,
        })?;
    // Paginated histories always honor this boundary. Other histories only do so when resume
    // metadata identifies a compaction written under the newer resume contract.
    if compacted.replacement_history.is_none()
        || compacted.window_number.is_none()
        || (compacted.resume_metadata.is_none()
            && !matches!(history_mode, ThreadHistoryMode::Paginated))
    {
        return None;
    }
    Some(ReplayCheckpoint {
        compacted,
        suffix: &rollout_items[index + 1..],
    })
}

#[derive(Debug, Default)]
pub(crate) struct ActiveReplaySegment<'a> {
    pub(crate) turn_id: Option<String>,
    pub(crate) turn_completed: bool,
    pub(crate) counts_as_user_turn: bool,
    pub(crate) previous_turn_settings: Option<PreviousTurnSettings>,
    pub(crate) reference_context_item: TurnReferenceContextItem,
    pub(crate) world_state_replay: Vec<&'a RolloutItem>,
    pub(crate) history_checkpoint: Option<ReplayCheckpoint<'a>>,
    pub(crate) window: Option<ReconstructedWindow>,
}

pub(crate) fn turn_ids_are_compatible(active_turn_id: Option<&str>, item_turn_id: Option<&str>) -> bool {
    active_turn_id
        .is_none_or(|turn_id| item_turn_id.is_none_or(|item_turn_id| item_turn_id == turn_id))
}

pub(crate) fn finalize_active_segment<'a>(
    active_segment: ActiveReplaySegment<'a>,
    history_checkpoint: &mut Option<ReplayCheckpoint<'a>>,
    previous_turn_settings: &mut Option<PreviousTurnSettings>,
    reference_context_item: &mut TurnReferenceContextItem,
    world_state_replay: &mut Vec<&'a RolloutItem>,
    window: &mut Option<ReconstructedWindow>,
    pending_rollback_turns: &mut usize,
) {
    // Thread rollback drops the newest surviving real user-message boundaries. In replay, that
    // means skipping the next finalized segments that contain a non-contextual
    // `EventMsg::UserMessage`.
    if *pending_rollback_turns > 0 {
        if active_segment.counts_as_user_turn {
            *pending_rollback_turns -= 1;
        }
        return;
    }

    // Full world-state snapshots are persisted after installing initial context. They still
    // establish a baseline when a child fork removes the parent turn's agent message. Do not
    // count these context-only segments as user turns for rollback, or use a snapshot from
    // before the segment's latest compaction.
    let has_context_baseline = active_segment.counts_as_user_turn
        || active_segment
            .world_state_replay
            .iter()
            .take_while(|item| !matches!(item, RolloutItem::Compacted(_)))
            .any(|item| matches!(item, RolloutItem::WorldState(state) if state.full));
    world_state_replay.extend(active_segment.world_state_replay);

    // A surviving replacement-history compaction is a complete history base. Once we
    // know the newest surviving one, older rollout items do not affect rebuilt history.
    if history_checkpoint.is_none()
        && let Some(segment_history_checkpoint) = active_segment.history_checkpoint
    {
        *history_checkpoint = Some(segment_history_checkpoint);
    }

    if window.is_none() {
        *window = active_segment.window;
    }

    // Restore settings from the newest surviving context baseline.
    if previous_turn_settings.is_none() && has_context_baseline {
        *previous_turn_settings = active_segment.previous_turn_settings;
    }

    // `reference_context_item` comes from the newest surviving context baseline, or
    // from a surviving compaction that explicitly cleared that baseline.
    if matches!(reference_context_item, TurnReferenceContextItem::NeverSet)
        && (has_context_baseline
            || matches!(
                active_segment.reference_context_item,
                TurnReferenceContextItem::Cleared
            ))
    {
        *reference_context_item = active_segment.reference_context_item;
    }
}

pub(crate) fn parse_uuid_v7(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value)
        .ok()
        .filter(|uuid| uuid.get_version_num() == 7)
}

pub(crate) fn reconstructed_window_from_compaction(compacted: &CompactedItem) -> Option<ReconstructedWindow> {
    Some(ReconstructedWindow {
        number: compacted.window_number?,
        first_id: compacted.first_window_id.as_deref().and_then(parse_uuid_v7),
        previous_id: compacted
            .previous_window_id
            .as_deref()
            .and_then(parse_uuid_v7),
        id: compacted.window_id.as_deref().and_then(parse_uuid_v7),
    })
}

pub(crate) fn reconstructed_window_from_session_context_window(
    context_window: &SessionContextWindow,
) -> Option<ReconstructedWindow> {
    let id = parse_uuid_v7(&context_window.window_id)?;
    Some(ReconstructedWindow {
        number: 0,
        first_id: Some(id),
        previous_id: None,
        id: Some(id),
    })
}
