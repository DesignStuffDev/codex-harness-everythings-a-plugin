use super::FuzzyFileSearchSessionError;
use super::FuzzyFileSearchSessionErrorKind;
use super::FuzzyFileSearchSessionFailedNotification;
use crate::ServerNotification;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn session_failed_notification_round_trips_the_wire_envelope() -> serde_json::Result<()> {
    let payload = FuzzyFileSearchSessionFailedNotification {
        session_id: "search-7".to_string(),
        query: "résumé".to_string(),
        error: FuzzyFileSearchSessionError {
            kind: FuzzyFileSearchSessionErrorKind::TransportLost,
            message: "Search transport closed while searching for résumé".to_string(),
        },
    };
    let wire = json!({
        "method": "fuzzyFileSearch/sessionFailed",
        "params": {
            "sessionId": "search-7",
            "query": "résumé",
            "error": {
                "kind": "transportLost",
                "message": "Search transport closed while searching for résumé"
            }
        }
    });

    assert_eq!(
        serde_json::to_value(ServerNotification::FuzzyFileSearchSessionFailed(
            payload.clone()
        ))?,
        wire,
    );
    let decoded: ServerNotification = serde_json::from_value(wire)?;
    let ServerNotification::FuzzyFileSearchSessionFailed(decoded) = decoded else {
        panic!("sessionFailed must decode to its dedicated notification variant");
    };
    assert_eq!(decoded, payload);
    Ok(())
}

#[test]
fn session_error_categories_use_camel_case_on_the_wire() -> serde_json::Result<()> {
    let categories = [
        (
            FuzzyFileSearchSessionErrorKind::InvalidInput,
            "invalidInput",
        ),
        (
            FuzzyFileSearchSessionErrorKind::UnsupportedVersion,
            "unsupportedVersion",
        ),
        (
            FuzzyFileSearchSessionErrorKind::UnsupportedOption,
            "unsupportedOption",
        ),
        (
            FuzzyFileSearchSessionErrorKind::UnknownLease,
            "unknownLease",
        ),
        (FuzzyFileSearchSessionErrorKind::ClosedLease, "closedLease"),
        (FuzzyFileSearchSessionErrorKind::StaleEpoch, "staleEpoch"),
        (
            FuzzyFileSearchSessionErrorKind::ResourceExhausted,
            "resourceExhausted",
        ),
        (
            FuzzyFileSearchSessionErrorKind::SearchFailed,
            "searchFailed",
        ),
        (
            FuzzyFileSearchSessionErrorKind::TransportLost,
            "transportLost",
        ),
        (
            FuzzyFileSearchSessionErrorKind::ForcedShutdown,
            "forcedShutdown",
        ),
    ];

    for (kind, wire_kind) in categories {
        let error = FuzzyFileSearchSessionError {
            kind,
            message: "search operation failed".to_string(),
        };
        let wire = json!({ "kind": wire_kind, "message": "search operation failed" });
        assert_eq!(serde_json::to_value(&error)?, wire);
        assert_eq!(
            serde_json::from_value::<FuzzyFileSearchSessionError>(wire)?,
            error,
        );
    }
    Ok(())
}
