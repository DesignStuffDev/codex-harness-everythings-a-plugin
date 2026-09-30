//! Native model-visible token/media estimates; transport metadata is excluded.
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use codex_protocol::DEFAULT_FUNCTION_NAMESPACE;
use codex_protocol::models::{AgentMessageInputContent, ContentItem, FunctionCallOutputBody, FunctionCallOutputContentItem, ImageDetail, ImageReference, ResponseItem};
use codex_utils_audio::estimate_audio_token_count;
use codex_utils_cache::{BlockingLruCache, sha1_digest};
use codex_utils_output_truncation::{approx_bytes_for_tokens, approx_tokens_from_byte_count_i64};
use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock, OnceLock};
use crate::util::serialized_json_bytes;

fn estimate_reasoning_length(encoded_len: usize) -> usize {
    encoded_len
        .saturating_mul(3)
        .checked_div(4)
        .unwrap_or(0)
        .saturating_sub(650)
}

fn estimate_encrypted_function_output_length(encoded_len: usize) -> usize {
    encoded_len.saturating_mul(9).div_ceil(16)
}

/// Returns the same coarse, model-visible token estimate used for full history estimates.
///
/// Counts content directly, excluding transport IDs, metadata, and outer JSON escaping.
/// Original-detail file images use the maximum patch count.
pub fn estimate_item_token_count(item: &ResponseItem) -> i64 {
    let model_visible_bytes = estimate_response_item_model_visible_bytes(item);
    approx_tokens_from_byte_count_i64(model_visible_bytes)
}

/// Approximate model-visible byte cost for one image input.
///
/// The estimator later converts bytes to tokens using a 4-bytes/token heuristic
/// with ceiling division, so 7,373 bytes maps to approximately 1,844 tokens.
const RESIZED_IMAGE_BYTES_ESTIMATE: i64 = 7373;
// See https://platform.openai.com/docs/guides/images-vision#calculating-costs.
// Use a direct 32px patch count only for `detail: "original"`;
// all other image inputs continue to use `RESIZED_IMAGE_BYTES_ESTIMATE`.
const ORIGINAL_IMAGE_PATCH_SIZE: u32 = 32;
// See https://platform.openai.com/docs/guides/images-vision#model-sizing-behavior.
// Keep this hard-coded for now; move it into model capabilities if the patch
// budget starts changing often across model releases.
const ORIGINAL_IMAGE_MAX_PATCHES: usize = 10_000;
const ORIGINAL_IMAGE_ESTIMATE_CACHE_SIZE: usize = 32;

type OriginalImageEstimateCache = BlockingLruCache<[u8; 20], Arc<OnceLock<Option<i64>>>>;

static ORIGINAL_IMAGE_ESTIMATE_CACHE: LazyLock<OriginalImageEstimateCache> = LazyLock::new(|| {
    BlockingLruCache::new(
        NonZeroUsize::new(ORIGINAL_IMAGE_ESTIMATE_CACHE_SIZE).unwrap_or(NonZeroUsize::MIN),
    )
});

fn estimate_response_item_model_visible_bytes(item: &ResponseItem) -> i64 {
    match item {
        ResponseItem::Message { content, .. } => content
            .iter()
            .map(|part| match part {
                ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                    text_bytes(text)
                }
                ContentItem::InputImage { image, detail } => {
                    estimate_image_reference_bytes(image, *detail)
                }
                ContentItem::InputAudio { audio_url } => estimate_audio_bytes(audio_url),
            })
            .fold(0i64, i64::saturating_add),
        ResponseItem::AgentMessage {
            author,
            recipient,
            content,
            ..
        } => content
            .iter()
            .map(|part| match part {
                AgentMessageInputContent::InputText { text } => text_bytes(text),
                AgentMessageInputContent::EncryptedContent { encrypted_content } => i64::try_from(
                    estimate_encrypted_function_output_length(encrypted_content.len()),
                )
                .unwrap_or(i64::MAX),
            })
            .fold(
                text_bytes(author).saturating_add(text_bytes(recipient)),
                i64::saturating_add,
            ),
        ResponseItem::Reasoning {
            encrypted_content: Some(content),
            ..
        }
        | ResponseItem::Compaction {
            encrypted_content: content,
            ..
        }
        | ResponseItem::ContextCompaction {
            encrypted_content: Some(content),
            ..
        } => i64::try_from(estimate_reasoning_length(content.len())).unwrap_or(i64::MAX),
        ResponseItem::FunctionCall {
            name,
            namespace,
            arguments: input,
            ..
        }
        | ResponseItem::CustomToolCall {
            name,
            namespace,
            input,
            ..
        } => text_bytes(name)
            .saturating_add(text_bytes(
                namespace.as_deref().unwrap_or(DEFAULT_FUNCTION_NAMESPACE),
            ))
            .saturating_add(text_bytes(input)),
        ResponseItem::FunctionCallOutput {
            call_id,
            name,
            namespace,
            output,
            ..
        } => estimate_function_output_bytes(&output.body)
            .saturating_add(text_bytes(call_id.as_deref().unwrap_or_default()))
            .saturating_add(text_bytes(name.as_deref().unwrap_or_default()))
            .saturating_add(text_bytes(namespace.as_deref().unwrap_or_default())),
        ResponseItem::CustomToolCallOutput {
            call_id,
            name,
            output,
            ..
        } => estimate_function_output_bytes(&output.body)
            .saturating_add(text_bytes(call_id))
            .saturating_add(text_bytes(name.as_deref().unwrap_or_default())),
        // These payloads are themselves JSON arguments, rather than transport envelopes
        // around text. Keep their JSON syntax in the estimate.
        ResponseItem::AdditionalTools { tools, .. } => json_content_bytes(tools),
        ResponseItem::ToolSearchCall { arguments, .. } => json_content_bytes(arguments),
        ResponseItem::ToolSearchOutput { tools, .. } => json_content_bytes(tools),
        ResponseItem::LocalShellCall { action, .. } => json_content_bytes(action),
        ResponseItem::WebSearchCall { action, .. } => {
            action.as_ref().map(json_content_bytes).unwrap_or_default()
        }
        ResponseItem::ImageGenerationCall {
            revised_prompt,
            result,
            ..
        } => text_bytes(revised_prompt.as_deref().unwrap_or_default()).saturating_add(
            if result.is_empty() {
                0
            } else {
                RESIZED_IMAGE_BYTES_ESTIMATE
            },
        ),
        ResponseItem::ContextCompaction {
            encrypted_content: None,
            ..
        } => 0,
        // Plaintext reasoning is excluded from replay accounting.
        ResponseItem::Reasoning {
            encrypted_content: None,
            ..
        } => 0,
        ResponseItem::ConfigurationUpdate { .. }
        | ResponseItem::CompactionTrigger { .. }
        | ResponseItem::Other => 0,
    }
}

fn text_bytes(text: &str) -> i64 {
    i64::try_from(text.len()).unwrap_or(i64::MAX)
}

fn json_content_bytes(value: &(impl serde::Serialize + ?Sized)) -> i64 {
    serialized_json_bytes(value)
        .map(|len| i64::try_from(len).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

/// Extracts inline image bytes for the original-detail dimension estimate.
fn parse_base64_image_data_url(url: &str) -> Option<&str> {
    if !url
        .get(.."data:".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
    {
        return None;
    }
    let comma_index = url.find(',')?;
    let metadata = &url[..comma_index];
    let payload = &url[comma_index + 1..];
    // Parse the media type and parameters without decoding. This keeps the
    // estimator cheap while ensuring we only apply modality heuristics to
    // appropriately typed base64 data URLs.
    let metadata_without_scheme = &metadata["data:".len()..];
    let mut metadata_parts = metadata_without_scheme.split(';');
    let mime_type = metadata_parts.next().unwrap_or_default();
    let has_base64_marker = metadata_parts.any(|part| part.eq_ignore_ascii_case("base64"));
    if !mime_type
        .get(.."image/".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("image/"))
    {
        return None;
    }
    if !has_base64_marker {
        return None;
    }
    Some(payload)
}

fn estimate_original_image_bytes(image_url: &str) -> Option<i64> {
    let key = sha1_digest(image_url.as_bytes());
    ORIGINAL_IMAGE_ESTIMATE_CACHE.get_or_init(key, || {
        let payload = match parse_base64_image_data_url(image_url) {
            Some(payload) => payload,
            None => {
                tracing::trace!("skipping original-detail estimate for non-base64 image data URL");
                return None;
            }
        };
        let bytes = match BASE64_STANDARD.decode(payload) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::trace!("failed to decode original-detail image payload: {error}");
                return None;
            }
        };
        let dynamic = match image::load_from_memory(&bytes) {
            Ok(dynamic) => dynamic,
            Err(error) => {
                tracing::trace!("failed to decode original-detail image bytes: {error}");
                return None;
            }
        };
        let width = i64::from(dynamic.width());
        let height = i64::from(dynamic.height());
        let patch_size = i64::from(ORIGINAL_IMAGE_PATCH_SIZE);
        let patches_wide = width.saturating_add(patch_size.saturating_sub(1)) / patch_size;
        let patches_high = height.saturating_add(patch_size.saturating_sub(1)) / patch_size;
        let patch_count = patches_wide.saturating_mul(patches_high);
        let patch_count = usize::try_from(patch_count).unwrap_or(usize::MAX);
        let patch_count = patch_count.min(ORIGINAL_IMAGE_MAX_PATCHES);
        Some(i64::try_from(approx_bytes_for_tokens(patch_count)).unwrap_or(i64::MAX))
    })
}

/// Inline image estimate, excluding the data URL prefix and message framing.
fn estimate_image_bytes(image_url: &str, detail: Option<ImageDetail>) -> i64 {
    match detail {
        Some(ImageDetail::Original) => {
            estimate_original_image_bytes(image_url).unwrap_or(RESIZED_IMAGE_BYTES_ESTIMATE)
        }
        _ => RESIZED_IMAGE_BYTES_ESTIMATE,
    }
}

/// Image estimate for callers that only have the reference. Original-detail file images use the
/// maximum patch count because their dimensions are not available from the reference.
pub fn estimate_image_reference_bytes(
    image: &ImageReference,
    detail: Option<ImageDetail>,
) -> i64 {
    match image {
        ImageReference::Inline { image_url } => estimate_image_bytes(image_url, detail),
        ImageReference::File { .. } if detail == Some(ImageDetail::Original) => {
            i64::try_from(approx_bytes_for_tokens(ORIGINAL_IMAGE_MAX_PATCHES)).unwrap_or(i64::MAX)
        }
        ImageReference::File { .. } => RESIZED_IMAGE_BYTES_ESTIMATE,
    }
}

fn estimate_audio_bytes(audio_url: &str) -> i64 {
    i64::try_from(approx_bytes_for_tokens(estimate_audio_token_count(
        audio_url,
    )))
    .unwrap_or(i64::MAX)
}

fn estimate_function_output_bytes(output: &FunctionCallOutputBody) -> i64 {
    match output {
        FunctionCallOutputBody::Text(text) => text_bytes(text),
        FunctionCallOutputBody::ContentItems(items) => items
            .iter()
            .map(|part| match part {
                FunctionCallOutputContentItem::InputText { text } => text_bytes(text),
                FunctionCallOutputContentItem::InputImage { image, detail } => {
                    estimate_image_reference_bytes(image, *detail)
                }
                FunctionCallOutputContentItem::InputAudio { audio_url } => {
                    estimate_audio_bytes(audio_url)
                }
                FunctionCallOutputContentItem::EncryptedContent { encrypted_content } => {
                    i64::try_from(estimate_encrypted_function_output_length(
                        encrypted_content.len(),
                    ))
                    .unwrap_or(i64::MAX)
                }
            })
            .fold(0i64, i64::saturating_add),
    }
}


#[cfg(test)]
#[path = "token_estimation_tests.rs"]
mod tests;
