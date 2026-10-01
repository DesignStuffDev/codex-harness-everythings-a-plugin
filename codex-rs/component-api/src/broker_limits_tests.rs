#![allow(clippy::expect_used, clippy::unwrap_used)]

use pretty_assertions::assert_eq;
use serde_json::json;

use crate::BrokerConnectionLimits;
use crate::BrokerConnectionLimitsSpec;
use crate::BrokerLimitsError;
use crate::BrokerProcessLimits;
use crate::BrokerProcessLimitsSpec;

fn connection() -> BrokerConnectionLimitsSpec {
    BrokerConnectionLimitsSpec {
        max_in_flight: 4,
        max_request_bytes: 64 * 1024,
        max_response_bytes: 1024 * 1024,
        max_spool_bytes: 8 * 1024 * 1024,
        max_decoded_nodes: 4096,
        max_decoded_string_bytes: 2 * 1024 * 1024,
        max_decode_jobs: 1,
        max_encode_jobs: 1,
        max_ordinary_codec_jobs: 1,
    }
}

fn process() -> BrokerProcessLimitsSpec {
    BrokerProcessLimitsSpec {
        max_broker_connections: 2,
        max_in_flight: 8,
        max_spool_bytes: 16 * 1024 * 1024,
        max_decoded_nodes: 8192,
        max_decoded_string_bytes: 4 * 1024 * 1024,
        max_blocking_jobs: 3,
    }
}

#[test]
fn limits_round_trip_through_the_checked_wire_shape() {
    let raw = connection();
    let checked = BrokerConnectionLimits::new(raw.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&checked).unwrap(),
        serde_json::to_value(raw).unwrap()
    );
    assert_eq!(
        serde_json::from_value::<BrokerConnectionLimits>(serde_json::to_value(&checked).unwrap())
            .unwrap(),
        checked
    );
    let shared = BrokerProcessLimits::new(process()).unwrap();
    assert_eq!(checked.validate_for(&shared), Ok(()));
    assert_eq!(
        serde_json::from_value::<BrokerProcessLimits>(serde_json::to_value(&shared).unwrap())
            .unwrap(),
        shared
    );
}

#[test]
fn maximum_call_budget_fits_both_request_and_response_without_overflow() {
    let mut spec = connection();
    spec.max_spool_bytes = spec.max_request_bytes + spec.max_response_bytes;
    assert!(BrokerConnectionLimits::new(spec.clone()).is_ok());
    spec.max_spool_bytes -= 1;
    assert_eq!(
        BrokerConnectionLimits::new(spec),
        Err(BrokerLimitsError::SpoolCannotFitCall)
    );
    let mut spec = connection();
    spec.max_request_bytes = u64::MAX;
    assert_eq!(
        BrokerConnectionLimits::new(spec),
        Err(BrokerLimitsError::SizeOverflow)
    );
    let checked = BrokerConnectionLimits::new(connection()).unwrap();
    let mut shared = process();
    shared.max_spool_bytes = 1024 * 1024;
    assert_eq!(
        checked.validate_for(&BrokerProcessLimits::new(shared).unwrap()),
        Err(BrokerLimitsError::ProcessCannotFitCall)
    );
}

#[test]
fn invalid_concurrency_and_zero_budgets_are_rejected_during_deserialization() {
    // Explicit per-connection ceilings may exceed the recommended composition
    // profile; shared running capacity is independently bounded by the process.
    let mut maximum = connection();
    maximum.max_in_flight = 16;
    maximum.max_decode_jobs = 16;
    maximum.max_encode_jobs = 16;
    maximum.max_ordinary_codec_jobs = 32;
    let checked = BrokerConnectionLimits::new(maximum).unwrap();
    let mut shared = process();
    shared.max_broker_connections = 16;
    shared.max_in_flight = 256;
    shared.max_blocking_jobs = 8;
    let shared = BrokerProcessLimits::new(shared).unwrap();
    assert_eq!(checked.validate_for(&shared), Ok(()));
    let raw = serde_json::to_value(&shared).unwrap();
    assert_eq!(
        serde_json::from_value::<BrokerProcessLimits>(raw).unwrap(),
        shared
    );
    for (field, value) in [
        ("max_in_flight", 0),
        ("max_in_flight", 17),
        ("max_request_bytes", 0),
        ("max_response_bytes", 0),
        ("max_spool_bytes", 0),
        ("max_decoded_nodes", 0),
        ("max_decoded_string_bytes", 0),
        ("max_decode_jobs", 0),
        ("max_decode_jobs", 5),
        ("max_encode_jobs", 0),
        ("max_encode_jobs", 5),
        ("max_ordinary_codec_jobs", 0),
        ("max_ordinary_codec_jobs", 33),
    ] {
        let mut raw = serde_json::to_value(connection()).unwrap();
        raw[field] = json!(value);
        assert!(
            serde_json::from_value::<BrokerConnectionLimits>(raw).is_err(),
            "accepted invalid connection limit"
        );
    }
    for (field, value) in [
        ("max_broker_connections", 0),
        ("max_broker_connections", 17),
        ("max_in_flight", 0),
        ("max_in_flight", 33),
        ("max_spool_bytes", 0),
        ("max_decoded_nodes", 0),
        ("max_decoded_string_bytes", 0),
        ("max_blocking_jobs", 0),
        ("max_blocking_jobs", 2),
        ("max_blocking_jobs", 9),
    ] {
        let mut raw = serde_json::to_value(process()).unwrap();
        raw[field] = json!(value);
        assert!(
            serde_json::from_value::<BrokerProcessLimits>(raw).is_err(),
            "accepted invalid process limit"
        );
    }
}

#[test]
fn limit_schema_rejects_raw_duplicates_missing_and_unknown_fields() {
    let valid = serde_json::to_string(&connection()).unwrap();
    let duplicate = valid.replacen("{", "{\"max_request_bytes\":1,", 1);
    assert!(
        serde_json::from_str::<BrokerConnectionLimits>(&duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicate field")
    );
    let mut missing = serde_json::to_value(connection()).unwrap();
    missing.as_object_mut().unwrap().remove("max_decode_jobs");
    assert!(
        serde_json::from_value::<BrokerConnectionLimits>(missing)
            .unwrap_err()
            .to_string()
            .contains("missing field")
    );
    let mut unknown = serde_json::to_value(connection()).unwrap();
    unknown["ordinary_history_bytes"] = json!(1024);
    assert!(
        serde_json::from_value::<BrokerConnectionLimits>(unknown)
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );

    let valid = serde_json::to_string(&process()).unwrap();
    let duplicate = valid.replacen("{", "{\"max_blocking_jobs\":3,", 1);
    assert!(
        serde_json::from_str::<BrokerProcessLimits>(&duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicate field")
    );
    let mut missing = serde_json::to_value(process()).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("max_broker_connections");
    assert!(
        serde_json::from_value::<BrokerProcessLimits>(missing)
            .unwrap_err()
            .to_string()
            .contains("missing field")
    );
    let mut unknown = serde_json::to_value(process()).unwrap();
    unknown["ordinary_history_bytes"] = json!(1024);
    assert!(
        serde_json::from_value::<BrokerProcessLimits>(unknown)
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
}
