#![allow(clippy::unwrap_used)]

use std::time::Duration;

use codex_file_search_api::SearchErrorKind;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

use super::native_limits;
use crate::positive;
use crate::service_ceilings;

#[test]
fn negotiated_limits_remain_exact_and_native_output_has_distinct_units() {
    let mut limits = service_ceilings().unwrap();
    limits.max_leases = 2;
    limits.max_query_utf8_bytes = 8192;
    limits.max_roots_options_bytes = 1024;
    limits.max_matches = 80;
    limits.max_frame_bytes = 128 * 1024;
    limits.max_poll_wait_ms = 150;
    limits.resources.max_worker_threads.0 = 26;
    let native = native_limits(&limits, json!({"native_snapshot_bytes": 2 * 1024 * 1024})).unwrap();
    assert_eq!(
        (
            native.max_sessions,
            native.resources,
            native.max_query_bytes,
            native.max_matches,
            native.max_snapshot_bytes,
            native.max_poll_wait
        ),
        (
            positive(2).unwrap(),
            limits.resources.into_native().unwrap(),
            positive(8192).unwrap(),
            positive(80).unwrap(),
            positive(2 * 1024 * 1024).unwrap(),
            Duration::from_millis(150)
        )
    );
    assert!(native.max_roots_options_bytes.get() >= 1024 * std::mem::size_of::<String>());
    assert_ne!(
        native.max_snapshot_bytes.get(),
        limits.max_frame_bytes as usize
    );
}

#[test]
fn empty_or_null_configuration_preserves_the_default_native_output_policy() {
    let limits = service_ceilings().unwrap();
    let empty = native_limits(&limits, json!({})).unwrap();
    let null = native_limits(&limits, Value::Null).unwrap();
    assert_eq!(
        (empty.resources, empty.max_snapshot_bytes),
        (null.resources, null.max_snapshot_bytes)
    );
}

#[test]
fn unknown_fields_and_wrong_types_are_rejected_without_echoing_configuration() {
    let limits = service_ceilings().unwrap();
    for config in [
        json!({"private-value": "do-not-echo-this"}),
        json!([]),
        json!(true),
        json!({"native_snapshot_bytes": "1048576"}),
        json!({"native_snapshot_bytes": -1}),
        json!({"native_snapshot_bytes": 1.5}),
    ] {
        let error = native_limits(&limits, config).unwrap_err();
        assert_eq!(error.kind(), SearchErrorKind::InvalidInput);
        assert!(!error.message().contains("do-not-echo-this"));
        assert!(!error.message().contains("private-value"));
    }
}

#[test]
fn unsupported_native_snapshot_policy_is_not_silently_clamped() {
    let limits = service_ceilings().unwrap();
    for bytes in [0, 65_535, 64 * 1024 * 1024 + 1] {
        assert_eq!(
            native_limits(&limits, json!({"native_snapshot_bytes":bytes}))
                .unwrap_err()
                .kind(),
            SearchErrorKind::UnsupportedOption
        );
    }
    for bytes in [65_536, 64 * 1024 * 1024] {
        assert_eq!(
            native_limits(&limits, json!({"native_snapshot_bytes":bytes}))
                .unwrap()
                .max_snapshot_bytes
                .get(),
            bytes
        );
    }
}

#[test]
fn invalid_negotiated_service_policy_cannot_create_a_native_provider() {
    let mut limits = service_ceilings().unwrap();
    limits.max_leases = 0;
    assert_eq!(
        native_limits(&limits, json!({})).unwrap_err().kind(),
        SearchErrorKind::InvalidInput
    );
}
