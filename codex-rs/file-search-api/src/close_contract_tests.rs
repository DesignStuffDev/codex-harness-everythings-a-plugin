//! Wire validation only; actual owner/quota/worker tests belong to implementations.

use crate::CloseCleanup;
use crate::SearchCloseOutcome;
use crate::SearchError;
use crate::SearchErrorKind;
use pretty_assertions::assert_eq;

#[test]
fn close_receipt_preserves_operation_failure_independently_of_cleanup() {
    let failure = SearchError::new(SearchErrorKind::SearchFailed, "index failed");
    let cleanup_failure = SearchError::new(SearchErrorKind::TransportLost, "receipt lost");
    for (wire, expected) in [
        (
            serde_json::json!({
                "operation": {"Err": {"kind": "search_failed", "message": "index failed"}},
                "cleanup": {"status": "joined"}
            }),
            SearchCloseOutcome {
                operation: Err(failure.clone()),
                cleanup: CloseCleanup::Joined,
            },
        ),
        (
            serde_json::json!({
                "operation": {"Err": {"kind": "search_failed", "message": "index failed"}},
                "cleanup": {"status": "unconfirmed", "error": {
                    "kind": "transport_lost", "message": "receipt lost"
                }}
            }),
            SearchCloseOutcome {
                operation: Err(failure),
                cleanup: CloseCleanup::Unconfirmed(cleanup_failure),
            },
        ),
    ] {
        let decoded: SearchCloseOutcome = serde_json::from_value(wire.clone()).expect("receipt");
        assert_eq!(decoded, expected);
        assert_eq!(serde_json::to_value(decoded).expect("encode receipt"), wire);
    }
}

#[test]
fn close_receipt_rejects_missing_unknown_or_oversized_cleanup_evidence() {
    let oversized = "x".repeat(SearchError::MAX_MESSAGE_BYTES + 1);
    for wire in [
        serde_json::json!({"operation": {"Ok": null}}),
        serde_json::json!({"operation": {"Ok": null}, "cleanup": {"status": "reaped"}}),
        serde_json::json!({"operation": {"Ok": null}, "cleanup": {"status": "unconfirmed"}}),
        serde_json::json!({
            "operation": {"Ok": null},
            "cleanup": {"status": "unconfirmed", "error": {
                "kind": "transport_lost", "message": oversized
            }}
        }),
    ] {
        assert!(serde_json::from_value::<SearchCloseOutcome>(wire).is_err());
    }
}
