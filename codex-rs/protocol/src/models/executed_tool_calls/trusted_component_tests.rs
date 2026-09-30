use super::*;
use pretty_assertions::assert_eq;

#[test]
fn trusted_roundtrip_preserves_private_truncation_and_result_provenance() {
    let call = ExecutedToolCall {
        name: "truncated call".to_owned(),
        arguments: ExecutedToolCallArguments::Truncated {
            truncation: ExecutedToolCallTruncation {
                original_bytes: 20_000,
                max_bytes: 128,
                omitted_calls: Some(3),
                original_name_bytes: Some(400),
            },
        },
        tool_result_sources: Some(vec![ToolResultSource {
            r#type: "resource".to_owned(),
            id: "captured-source".to_owned(),
        }]),
        tool_result_metadata: ToolResultMetadata::new(&serde_json::json!({
            "openai/resource_access": { "resource_id": "captured-source" },
        })),
    };
    let json = serde_json::to_vec(&TrustedComponentExecutedToolCall::from(&call)).unwrap();
    let decoded: TrustedComponentExecutedToolCall = serde_json::from_slice(&json).unwrap();
    assert_eq!(ExecutedToolCall::try_from(decoded).unwrap(), call);

    let provider: ExecutedToolCall =
        serde_json::from_slice(&serde_json::to_vec(&call).unwrap()).unwrap();
    assert!(matches!(
        provider.arguments(),
        ExecutedToolCallArguments::Raw(_)
    ));
    assert_eq!(provider.tool_result_sources, None);
    assert_eq!(provider.tool_result_metadata, ToolResultMetadata::default());
}

#[test]
fn trusted_roundtrip_distinguishes_absent_and_captured_null_metadata() {
    for metadata in [
        ToolResultMetadata::default(),
        ToolResultMetadata::new(&serde_json::Value::Null),
    ] {
        let mut call = ExecutedToolCall::new("tool".to_owned(), serde_json::Value::Null);
        call.set_tool_result_metadata(metadata);
        let bytes = serde_json::to_vec(&TrustedComponentExecutedToolCall::from(&call)).unwrap();
        let snapshot: TrustedComponentExecutedToolCall = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(ExecutedToolCall::try_from(snapshot).unwrap(), call);
    }
}

#[test]
fn trusted_restore_rejects_invalid_source_evidence_without_truncating_it() {
    let mut snapshot = TrustedComponentExecutedToolCall::from(&ExecutedToolCall::new(
        "tool".to_owned(),
        serde_json::Value::Null,
    ));
    snapshot.tool_result_sources = Some(vec![TrustedSource {
        r#type: "resource".to_owned(),
        id: "x".repeat(super::super::MAX_TOOL_RESULT_SOURCE_FIELD_BYTES + 1),
    }]);
    assert_eq!(
        ExecutedToolCall::try_from(snapshot),
        Err(InvalidTrustedComponentExecutedToolCall),
    );
}
