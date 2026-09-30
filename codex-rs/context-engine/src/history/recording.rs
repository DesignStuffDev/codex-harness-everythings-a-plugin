//! Native ContextManager methods, lifted without changing algorithm bodies.

use super::*;

impl ContextManager {

    /// `items` is ordered from oldest to newest.
    pub fn record_items<I>(&mut self, items: I, policy: TruncationPolicy)
    where
        I: IntoIterator,
        I::Item: Deref<Target = ResponseItem>,
    {
        for item in items {
            self.record_item_with_metadata(&item, /*metadata*/ None, policy);
        }
    }

    /// Records output and annotates the original envelopes with captured provenance.
    /// Tool output truncation applies only to live history, preserving full rollout payloads.
    pub fn record_annotated_items(
        &mut self,
        items: &mut [ResponseItemEnvelope],
        policy: TruncationPolicy,
    ) {
        for envelope in items {
            if let Some(source) =
                self.record_item_with_metadata(&envelope.item, envelope.metadata.as_ref(), policy)
            {
                envelope.metadata.get_or_insert_default().retained_source = Some(source);
            }
        }
    }

    /// Replays persisted originals without assigning new identities to known versions.
    pub fn replay_annotated_item(
        &mut self,
        envelope: &ResponseItemEnvelope,
        policy: TruncationPolicy,
    ) {
        let captured =
            self.record_item_with_metadata(&envelope.item, envelope.metadata.as_ref(), policy);
        if let Some(source) = envelope
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.retained_source.as_ref())
            && captured.as_ref().is_some_and(|captured| {
                captured.id == source.id && captured.complete == source.complete
            })
            && Arc::make_mut(&mut self.retained_context).restore_source_revision(source)
            && let Some(recorded) = Arc::make_mut(&mut self.items).last_mut()
        {
            recorded.metadata.get_or_insert_default().retained_source = Some(source.clone());
        }
    }

    pub(super) fn record_item_with_metadata(
        &mut self,
        item: &ResponseItem,
        metadata: Option<&CodexHarnessMetadata>,
        policy: TruncationPolicy,
    ) -> Option<codex_history::RetainedSource> {
        if !is_api_message(item, metadata) {
            return None;
        }
        let mut processed = ResponseItemEnvelope {
            item: item.clone(),
            metadata: metadata.cloned(),
        };
        if let ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } = &mut processed.item
        {
            // The override already includes the tool's serialization allowance.
            let policy = metadata
                .and_then(|metadata| metadata.history_truncation_token_limit)
                .map(TruncationPolicy::Tokens)
                .unwrap_or_else(|| with_serialization_allowance(policy));
            truncate_function_output_payload(output, policy, estimate_audio_token_count);
        }
        if let Some(review_history) = &mut self.review_history
            && !is_guardian_context_message(item)
        {
            review_history.record(&processed);
        }
        if let Some(metadata) = metadata
            && Arc::make_mut(&mut self.retained_context).record_sender_user_messages(metadata)
        {
            self.user_message_revision = self.user_message_revision.saturating_add(1);
        }
        let source = self.record_retained_message(
            item,
            metadata,
            user_authorization::RetainedMessageSource::Original,
        );
        if let Some(source) = &source {
            processed.metadata.get_or_insert_default().retained_source = Some(source.clone());
        }
        Arc::make_mut(&mut self.items).push(processed);
        source
    }

    /// Returns the history prepared for sending to the model. This applies a proper
    /// normalization and drops un-suited items. Unsupported image and audio content
    /// is stripped from messages and tool outputs according to `input_modalities`.
    pub fn for_prompt(self, input_modalities: &[InputModality]) -> Vec<ResponseItem> {
        self.for_prompt_annotated(input_modalities)
            .into_iter()
            .map(ResponseItemEnvelope::into_item)
            .collect()
    }

    /// Returns normalized history envelopes for internal consumers that must retain metadata.
    pub fn for_prompt_annotated(
        mut self,
        input_modalities: &[InputModality],
    ) -> Vec<ResponseItemEnvelope> {
        self.normalize_history(input_modalities);
        Arc::unwrap_or_clone(self.items)
    }

    /// Iterates over raw response items without exposing their history envelopes.
    pub fn raw_items(
        &self,
    ) -> impl Clone + ExactSizeIterator<Item = &ResponseItem> + DoubleEndedIterator {
        self.items.iter().map(|envelope| &envelope.item)
    }

    /// Returns annotated history items without cloning their response payloads.
    pub fn annotated_items(&self) -> &[ResponseItemEnvelope] {
        &self.items
    }

    /// Returns annotated history items and consumes the snapshot.
    pub fn into_annotated_items(self) -> Vec<ResponseItemEnvelope> {
        Arc::unwrap_or_clone(self.into_shared_annotated_items())
    }

    /// Keeps shared response items while releasing the snapshot's unrelated metadata.
    pub fn into_shared_annotated_items(self) -> Arc<Vec<ResponseItemEnvelope>> {
        self.items
    }

    pub fn remove_first_item(&mut self) {
        if !self.items.is_empty() {
            // Remove the oldest item (front of the list). Items are ordered from
            // oldest → newest, so index 0 is the first entry recorded.
            let items = Arc::make_mut(&mut self.items);
            let removed = items.remove(0);
            // If the removed item participates in a call/output pair, also remove
            // its corresponding counterpart to keep the invariants intact without
            // running a full normalization pass.
            normalize::remove_corresponding_for(items, &removed.item);
            self.world_state_baseline = None;
        }
    }

    /// This function enforces a couple of invariants on the in-memory history:
    /// 1. every call (function/custom) has a corresponding output entry
    /// 2. every output has a corresponding call entry or names an external tool event
    /// 3. unsupported image and audio content is stripped from messages and tool outputs
    pub(super) fn normalize_history(&mut self, input_modalities: &[InputModality]) {
        let items = Arc::make_mut(&mut self.items);

        // all function/tool calls must have a corresponding output
        normalize::ensure_call_outputs_present(items);

        // Paired outputs must have a corresponding call; named external outputs stand alone.
        normalize::remove_orphan_outputs(items);

        // strip images when model does not support them
        normalize::strip_images_when_unsupported(input_modalities, items);

        // strip audio when model does not support it
        normalize::strip_audio_when_unsupported(input_modalities, items);
    }
}
