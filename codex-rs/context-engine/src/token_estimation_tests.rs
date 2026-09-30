//! Native media/token/truncation regression tests copied with import-only adaptation.
use super::*;
use crate::ContextManager;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use codex_protocol::models::{ContentItemKind, DEFAULT_IMAGE_DETAIL, FunctionCallOutputPayload, InternalChatMessageMetadataPassthrough};
use codex_utils_output_truncation::{TruncationPolicy, approx_token_count};
use image::{ImageBuffer, ImageFormat, Luma, Rgba};
use pretty_assertions::assert_eq;
const TEST_WAV_SAMPLE_RATE: u32 = 8_000;

fn raw_items(history: &ContextManager) -> Vec<ResponseItem> {
    history.raw_items().cloned().collect()
}

fn unknown_content_metadata() -> InternalChatMessageMetadataPassthrough {
    InternalChatMessageMetadataPassthrough {
        content_item_kinds: Some(vec![ContentItemKind("unknown".to_string())]),
        ..Default::default()
    }
}

fn pcm_wav_data_url(sample_count: u32) -> (String, usize) {
    let padding = sample_count % 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + sample_count + padding).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&TEST_WAV_SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&TEST_WAV_SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&sample_count.to_le_bytes());
    bytes.resize(
        bytes.len() + sample_count as usize + padding as usize,
        /*value*/ 0,
    );
    let payload = BASE64_STANDARD.encode(bytes);
    let payload_len = payload.len();
    (format!("data:audio/wav;base64,{payload}"), payload_len)
}

#[test]
fn image_data_url_payload_does_not_dominate_message_estimate() {
    let payload = "A".repeat(100_000);
    let image_url = format!("data:image/png;base64,{payload}");
    let image_item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![
            ContentItem::InputText {
                text: "Here is the screenshot".to_string(),
            },
            ContentItem::InputImage {
                image: ImageReference::Inline { image_url },
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };
    let text_only_item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "Here is the screenshot".to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    let raw_len = serde_json::to_string(&image_item).unwrap().len() as i64;
    let estimated = estimate_response_item_model_visible_bytes(&image_item);
    let expected = "Here is the screenshot".len() as i64 + RESIZED_IMAGE_BYTES_ESTIMATE;
    let text_only_estimated = estimate_response_item_model_visible_bytes(&text_only_item);

    assert_eq!(estimated, expected);
    assert!(estimated < raw_len);
    assert!(estimated > text_only_estimated);
}

/// File images use the fixed estimate for normal detail and the maximum patch count for original.
#[test]
fn file_images_use_detail_appropriate_estimates() {
    let item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![
            ContentItem::InputImage {
                image: ImageReference::File {
                    file_id: "file_high".to_string(),
                },
                detail: Some(ImageDetail::High),
            },
            ContentItem::InputImage {
                image: ImageReference::File {
                    file_id: "file_original".to_string(),
                },
                detail: Some(ImageDetail::Original),
            },
        ],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = RESIZED_IMAGE_BYTES_ESTIMATE
        .saturating_add(approx_bytes_for_tokens(/*tokens*/ 10_000) as i64);

    assert_eq!(estimated, expected);
}

#[test]
fn function_call_output_estimates_inline_and_file_images() {
    let payload = "B".repeat(50_000);
    let image_url = format!("data:image/png;base64,{payload}");
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-abc".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputText {
                text: "Screenshot captured".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::Inline { image_url },
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::File {
                    file_id: "file_original".to_string(),
                },
                detail: Some(ImageDetail::Original),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let raw_len = serde_json::to_string(&item).unwrap().len() as i64;
    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = "call-abc".len() as i64
        + "Screenshot captured".len() as i64
        + RESIZED_IMAGE_BYTES_ESTIMATE
        + approx_bytes_for_tokens(/*tokens*/ 10_000) as i64;

    assert_eq!(estimated, expected);
    assert!(estimated < raw_len);
}

#[test]
fn image_data_url_payload_does_not_dominate_custom_tool_call_output_estimate() {
    let payload = "C".repeat(50_000);
    let image_url = format!("data:image/png;base64,{payload}");
    let item = ResponseItem::CustomToolCallOutput {
        id: None,
        call_id: "call-js-repl".to_string(),
        name: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputText {
                text: "Screenshot captured".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::Inline { image_url },
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let raw_len = serde_json::to_string(&item).unwrap().len() as i64;
    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = "call-js-repl".len() as i64
        + "Screenshot captured".len() as i64
        + RESIZED_IMAGE_BYTES_ESTIMATE;

    assert_eq!(estimated, expected);
    assert!(estimated < raw_len);
}

#[test]
fn audio_data_url_payload_does_not_dominate_message_estimate() {
    let (audio_url, _) = pcm_wav_data_url(/*sample_count*/ 801);
    let item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputAudio { audio_url }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    let raw_len = serde_json::to_string(&item).unwrap().len() as i64;
    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = approx_bytes_for_tokens(/*tokens*/ 2) as i64;

    assert_eq!(estimated, expected);
    assert!(estimated < raw_len);
}

#[test]
fn audio_data_url_payload_does_not_dominate_function_call_output_estimate() {
    let (audio_url, _) = pcm_wav_data_url(/*sample_count*/ 800);
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-audio".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputAudio { audio_url },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let raw_len = serde_json::to_string(&item).unwrap().len() as i64;
    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = "call-audio".len() as i64 + approx_bytes_for_tokens(/*tokens*/ 1) as i64;

    assert_eq!(estimated, expected);
    assert!(estimated < raw_len);
}

#[test]
fn audio_data_url_payload_does_not_dominate_custom_tool_call_output_estimate() {
    let (audio_url, _) = pcm_wav_data_url(/*sample_count*/ 80_000);
    let item = ResponseItem::CustomToolCallOutput {
        id: None,
        call_id: "call-custom-audio".to_string(),
        name: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputAudio { audio_url },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let raw_len = serde_json::to_string(&item).unwrap().len() as i64;
    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected =
        "call-custom-audio".len() as i64 + approx_bytes_for_tokens(/*tokens*/ 100) as i64;

    assert_eq!(estimated, expected);
    assert!(estimated < raw_len);
}

#[test]
fn malformed_audio_data_url_falls_back_to_whole_url_size_cost() {
    let payload = "A".repeat(/*n*/ 100_000);
    let audio_url = format!("data:audio/wav;base64,{payload}");
    let fallback_bytes = approx_bytes_for_tokens(approx_token_count(&audio_url)) as i64;
    let item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputAudio { audio_url }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);

    assert_eq!(estimated, fallback_bytes);
}

#[test]
fn record_items_omits_audio_that_exceeds_the_output_budget() {
    let (audio_url, _) = pcm_wav_data_url(/*sample_count*/ 80_000);
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-audio".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::ContentItems(vec![
                FunctionCallOutputContentItem::InputAudio { audio_url },
            ]),
            success: Some(true),
        },
        internal_chat_message_metadata_passthrough: None,
    };
    let mut history = ContextManager::new();

    history.record_items([&item], TruncationPolicy::Tokens(50));

    assert_eq!(
        raw_items(&history),
        &[ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("call-audio".to_string()),
            name: None,
            namespace: None,
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::ContentItems(vec![
                    FunctionCallOutputContentItem::InputText {
                        text: "[omitted 1 audio items ...]".to_string(),
                    },
                ]),
                success: Some(true),
            },
            internal_chat_message_metadata_passthrough: None,
        }]
    );
}

#[test]
fn non_base64_image_urls_use_image_estimates() {
    let message_item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputImage {
            image: ImageReference::Inline {
                image_url: "https://example.com/foo.png".to_string(),
            },
            detail: Some(DEFAULT_IMAGE_DETAIL),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };
    let function_output_item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-1".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::Inline {
                    image_url: "file:///tmp/foo.png".to_string(),
                },
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    assert_eq!(
        estimate_response_item_model_visible_bytes(&message_item),
        RESIZED_IMAGE_BYTES_ESTIMATE
    );
    assert_eq!(
        estimate_response_item_model_visible_bytes(&function_output_item),
        "call-1".len() as i64 + RESIZED_IMAGE_BYTES_ESTIMATE
    );
}

#[test]
fn passthrough_metadata_does_not_change_context_estimates() {
    for output in [
        serde_json::json!({"type": "message", "role": "user", "content": [{"type": "input_text", "text": "hello"}]}),
        serde_json::json!({"type": "function_call_output", "call_id": "call", "output": "result"}),
        serde_json::json!({"type": "custom_tool_call_output", "call_id": "call", "output": "result"}),
        serde_json::json!({"type": "tool_search_output", "call_id": "call", "execution": "client", "status": "completed", "tools": []}),
    ] {
        let original: ResponseItem = serde_json::from_value(output.clone()).expect("response item");
        let before = estimate_response_item_model_visible_bytes(&original);
        for turn_metadata in [
            None,
            Some(InternalChatMessageMetadataPassthrough::default()),
            Some(InternalChatMessageMetadataPassthrough {
                turn_id: Some("turn-\"你好\"".to_string()),
                create_time: Some(123.into()),
                ..unknown_content_metadata()
            }),
        ] {
            let mut output = output.clone();
            output["internal_chat_message_metadata_passthrough"] =
                serde_json::to_value(turn_metadata).expect("turn metadata");
            let mut item: ResponseItem = serde_json::from_value(output).expect("response item");
            assert_eq!(estimate_response_item_model_visible_bytes(&item), before);
            let mut call = codex_protocol::models::ExecutedToolCall::new(
                "test_tool".to_string(),
                serde_json::json!({"input": "x".repeat(8 * 1024)}),
            );
            call.set_tool_result_metadata(codex_protocol::models::ToolResultMetadata::new(
                &serde_json::json!({"provider": "x".repeat(16 * 1024)}),
            ));
            item.append_executed_tool_calls(vec![call]);
            item.set_tool_call_cell_id("cell");
            item.mark_tool_calls_complete();
            assert_eq!(estimate_response_item_model_visible_bytes(&item), before);
            item.clear_internal_chat_message_metadata_passthrough();
            assert_eq!(item, original);
        }
    }
}

#[test]
fn encrypted_function_output_uses_plaintext_byte_estimate() {
    let encrypted_content = "A".repeat(1_868);
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-encrypted".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::EncryptedContent {
                encrypted_content: encrypted_content.clone(),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = "call-encrypted".len() as i64
        + estimate_encrypted_function_output_length(encrypted_content.len()) as i64;

    assert_eq!(estimated, expected);

    let agent_message = InterAgentCommunication::new_encrypted(
        AgentPath::root(),
        AgentPath::root().join("worker").expect("valid worker path"),
        Vec::new(),
        encrypted_content.clone(),
        /*trigger_turn*/ true,
    )
    .to_model_input_item();
    let expected_agent = "/root".len() as i64
        + "/root/worker".len() as i64
        + "Message Type: NEW_TASK\nTask name: /root/worker\nSender: /root\nPayload:\n".len() as i64
        + estimate_encrypted_function_output_length(encrypted_content.len()) as i64;

    assert_eq!(
        estimate_response_item_model_visible_bytes(&agent_message),
        expected_agent
    );
}

#[test]
fn data_url_without_base64_marker_uses_image_estimate() {
    let item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputImage {
            image: ImageReference::Inline {
                image_url: "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>"
                    .to_string(),
            },
            detail: Some(DEFAULT_IMAGE_DETAIL),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    assert_eq!(
        estimate_response_item_model_visible_bytes(&item),
        RESIZED_IMAGE_BYTES_ESTIMATE
    );
}

#[test]
fn non_image_base64_data_url_uses_image_estimate() {
    let payload = "C".repeat(4_096);
    let image_url = format!("data:application/octet-stream;base64,{payload}");
    let expected = "call-octet".len() as i64 + RESIZED_IMAGE_BYTES_ESTIMATE;
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-octet".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::Inline { image_url },
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);

    assert_eq!(estimated, expected);
}

#[test]
fn mixed_case_data_url_markers_are_adjusted() {
    let payload = "F".repeat(1_024);
    let image_url = format!("DATA:image/png;BASE64,{payload}");
    let item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputImage {
            image: ImageReference::Inline { image_url },
            detail: Some(DEFAULT_IMAGE_DETAIL),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = RESIZED_IMAGE_BYTES_ESTIMATE;

    assert_eq!(estimated, expected);
}

#[test]
fn multiple_inline_images_apply_multiple_fixed_costs() {
    let payload_one = "D".repeat(100);
    let payload_two = "E".repeat(200);
    let image_url_one = format!("data:image/png;base64,{payload_one}");
    let image_url_two = format!("data:image/jpeg;base64,{payload_two}");
    let item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![
            ContentItem::InputText {
                text: "images".to_string(),
            },
            ContentItem::InputImage {
                image: ImageReference::Inline {
                    image_url: image_url_one,
                },
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
            ContentItem::InputImage {
                image: ImageReference::Inline {
                    image_url: image_url_two,
                },
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = "images".len() as i64 + (2 * RESIZED_IMAGE_BYTES_ESTIMATE);

    assert_eq!(estimated, expected);
}

#[test]
fn original_detail_images_scale_with_dimensions() {
    // 2304x864 at 32px patches yields 72 * 27 = 1,944 patches.
    // The byte heuristic uses 4 bytes per token, so the replacement cost is 7,776 bytes.
    const EXPECTED_ORIGINAL_DETAIL_IMAGE_BYTES: i64 = 7_776;

    let width = 2304;
    let height = 864;
    let image = ImageBuffer::from_pixel(width, height, Rgba([12u8, 34, 56, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("encode png");
    let payload = BASE64_STANDARD.encode(bytes.get_ref());
    let image_url = format!("data:image/png;base64,{payload}");
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-original".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::Inline { image_url },
                detail: Some(ImageDetail::Original),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = "call-original".len() as i64 + EXPECTED_ORIGINAL_DETAIL_IMAGE_BYTES;

    assert_eq!(estimated, expected);
}

#[test]
fn original_detail_images_are_capped_at_max_patch_count() {
    // 3201x3201 at 32px patches yields 101 * 101 = 10,201 patches,
    // which exceeds the original-detail patch budget.
    let width = 3201;
    let height = 3201;
    let image = ImageBuffer::from_pixel(width, height, Luma([12u8]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("encode png");
    let payload = BASE64_STANDARD.encode(bytes.get_ref());
    let image_url = format!("data:image/png;base64,{payload}");
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-original-capped".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::Inline { image_url },
                detail: Some(ImageDetail::Original),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);
    let capped_original_detail_image_bytes =
        i64::try_from(approx_bytes_for_tokens(ORIGINAL_IMAGE_MAX_PATCHES)).unwrap();
    let expected = "call-original-capped".len() as i64 + capped_original_detail_image_bytes;

    assert_eq!(estimated, expected);
}

#[test]
fn original_detail_webp_images_scale_with_dimensions() {
    // Same dimensions as the PNG case above, so the patch-based replacement cost is the same.
    const EXPECTED_ORIGINAL_DETAIL_IMAGE_BYTES: i64 = 7_776;

    let width = 2304;
    let height = 864;
    let image = ImageBuffer::from_pixel(width, height, Rgba([12u8, 34, 56, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::WebP)
        .expect("encode webp");
    let payload = BASE64_STANDARD.encode(bytes.get_ref());
    let image_url = format!("data:image/webp;base64,{payload}");
    let item = ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("call-original-webp".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload::from_content_items(vec![
            FunctionCallOutputContentItem::InputImage {
                image: ImageReference::Inline { image_url },
                detail: Some(ImageDetail::Original),
            },
        ]),
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);
    let expected = "call-original-webp".len() as i64 + EXPECTED_ORIGINAL_DETAIL_IMAGE_BYTES;

    assert_eq!(estimated, expected);
}

#[test]
fn text_only_items_count_decoded_content() {
    let item = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: "Hello, \"world\"!\nこんにちは".to_string(),
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };

    let estimated = estimate_response_item_model_visible_bytes(&item);

    assert_eq!(estimated, "Hello, \"world\"!\nこんにちは".len() as i64);
}
