#![allow(clippy::expect_used, clippy::unwrap_used)]

use pretty_assertions::assert_eq;
use serde_json::json;

use crate::BrokerConnectionHandle;
use crate::BrokerNegotiationError;
use crate::GrantedOperationV1;
use crate::ServiceAuthorityHandle;
use crate::ServiceGrantV1;

fn authority() -> ServiceAuthorityHandle {
    ServiceAuthorityHandle::new("private-authority".to_owned()).unwrap()
}

fn operation(name: &str) -> GrantedOperationV1 {
    GrantedOperationV1::new(
        name.to_owned(),
        /*max_request_bytes*/ 1,
        /*max_response_bytes*/ 2,
    )
    .unwrap()
}

#[test]
fn handles_validate_utf8_bytes_and_redact_debug_without_changing_wire_values() {
    let raw = "é".repeat(128);
    let connection = BrokerConnectionHandle::new(raw.clone()).unwrap();
    let grant = ServiceAuthorityHandle::new(raw.clone()).unwrap();
    assert_eq!(serde_json::to_value(&connection).unwrap(), json!(raw));
    assert_eq!(serde_json::to_value(&grant).unwrap(), json!(raw));
    assert_eq!(
        serde_json::from_value::<BrokerConnectionHandle>(json!(raw)).unwrap(),
        connection
    );
    assert_eq!(
        serde_json::from_value::<ServiceAuthorityHandle>(json!(raw)).unwrap(),
        grant
    );
    assert!(!format!("{connection:?} {grant:?}").contains(&raw));
    for raw in [String::new(), "secret-marker".repeat(30)] {
        let connection = BrokerConnectionHandle::new(raw.clone()).unwrap_err();
        let grant = ServiceAuthorityHandle::new(raw.clone()).unwrap_err();
        assert_eq!(connection, BrokerNegotiationError::InvalidConnectionHandle);
        assert_eq!(grant, BrokerNegotiationError::InvalidAuthorityHandle);
        assert!(!format!("{connection} {grant}").contains("secret-marker"));
        assert!(serde_json::from_value::<BrokerConnectionHandle>(json!(raw)).is_err());
        assert!(serde_json::from_value::<ServiceAuthorityHandle>(json!(raw)).is_err());
    }
    for raw in [
        json!("é".repeat(129)),
        json!(null),
        json!(7),
        json!({"value":"token"}),
    ] {
        assert!(serde_json::from_value::<BrokerConnectionHandle>(raw.clone()).is_err());
        assert!(serde_json::from_value::<ServiceAuthorityHandle>(raw).is_err());
    }
}

#[test]
fn operations_validate_names_and_finite_positive_caps_through_serde() {
    let raw = json!({"name":"é".repeat(128),"max_request_bytes":u64::MAX,"max_response_bytes":1});
    let operation: GrantedOperationV1 = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(serde_json::to_value(operation).unwrap(), raw);
    for (field, value, diagnostic) in [
        ("name", json!(""), "invalid broker operation name"),
        (
            "name",
            json!("é".repeat(129)),
            "invalid broker operation name",
        ),
        (
            "max_request_bytes",
            json!(0),
            "byte limits must be positive",
        ),
        (
            "max_response_bytes",
            json!(0),
            "byte limits must be positive",
        ),
    ] {
        let mut invalid = raw.clone();
        invalid[field] = value;
        let error = serde_json::from_value::<GrantedOperationV1>(invalid).unwrap_err();
        assert!(error.to_string().contains(diagnostic));
    }
}

#[test]
fn service_grants_canonicalize_operations_without_collapsing_duplicates_or_identity() {
    let left = ServiceGrantV1::new(
        "host.a".into(),
        /*version*/ 1,
        vec![operation("write"), operation("read")],
        authority(),
    )
    .unwrap();
    let right = ServiceGrantV1::new(
        "host.a".into(),
        /*version*/ 1,
        vec![operation("read"), operation("write")],
        authority(),
    )
    .unwrap();
    assert_eq!(left, right);
    assert_eq!(
        serde_json::to_value(&left).unwrap(),
        json!({"name":"host.a","version":1,
        "operations":[{"name":"read","max_request_bytes":1,"max_response_bytes":2},
        {"name":"write","max_request_bytes":1,"max_response_bytes":2}],"authority":"private-authority"})
    );
    assert_eq!(
        serde_json::from_value::<ServiceGrantV1>(serde_json::to_value(&left).unwrap()).unwrap(),
        left
    );
    assert!(!format!("{left:?}").contains("private-authority"));
    let duplicate = GrantedOperationV1::new(
        "read".into(),
        /*max_request_bytes*/ 9,
        /*max_response_bytes*/ 10,
    )
    .unwrap();
    assert_eq!(
        ServiceGrantV1::new(
            "host.a".into(),
            /*version*/ 1,
            vec![operation("read"), duplicate],
            authority()
        ),
        Err(BrokerNegotiationError::DuplicateOperation)
    );
    let exact_names = ServiceGrantV1::new(
        "host.a".into(),
        /*version*/ 1,
        vec![
            operation("Read"),
            operation("read"),
            operation("é"),
            operation("e\u{301}"),
        ],
        authority(),
    )
    .unwrap();
    assert_eq!(exact_names.operations().len(), 4);
    let other = ServiceGrantV1::new(
        "host.a".into(),
        /*version*/ 1,
        right.operations().to_vec(),
        ServiceAuthorityHandle::new("other".into()).unwrap(),
    )
    .unwrap();
    assert_ne!(left, other);
}

#[test]
fn service_grants_enforce_descriptor_and_operation_count_boundaries() {
    let operations: Vec<_> = (0..32).map(|n| operation(&format!("op{n}"))).collect();
    let grant =
        ServiceGrantV1::new("é".repeat(64), u32::MAX, operations.clone(), authority()).unwrap();
    assert_eq!(grant.name(), "é".repeat(64));
    assert_eq!(grant.version(), u32::MAX);
    for (name, version, operations, expected) in [
        (
            String::new(),
            1,
            operations.clone(),
            BrokerNegotiationError::InvalidServiceName,
        ),
        (
            "é".repeat(65),
            1,
            operations.clone(),
            BrokerNegotiationError::InvalidServiceName,
        ),
        (
            "a".into(),
            0,
            operations.clone(),
            BrokerNegotiationError::InvalidServiceVersion,
        ),
        (
            "a".into(),
            1,
            Vec::new(),
            BrokerNegotiationError::InvalidOperationCount,
        ),
    ] {
        assert_eq!(
            ServiceGrantV1::new(name, version, operations, authority()),
            Err(expected)
        );
    }
    let mut overflow = operations;
    overflow.push(operation("extra"));
    assert_eq!(
        ServiceGrantV1::new("a".into(), /*version*/ 1, overflow, authority()),
        Err(BrokerNegotiationError::InvalidOperationCount)
    );
}

#[test]
fn raw_grants_reject_duplicate_missing_and_unknown_fields_without_value_roundtrip() {
    let operation = r#"{"name":"read","max_request_bytes":1,"max_response_bytes":2}"#;
    for (raw, diagnostic) in [
        (
            operation.replacen('{', "{\"name\":\"other\",", 1),
            "duplicate field",
        ),
        (
            operation.replace(",\"max_request_bytes\":1", ""),
            "missing field",
        ),
        (
            operation.replacen('{', "{\"extra\":true,", 1),
            "unknown field",
        ),
    ] {
        assert!(
            serde_json::from_str::<GrantedOperationV1>(&raw)
                .unwrap_err()
                .to_string()
                .contains(diagnostic)
        );
    }
    let valid = format!(
        r#"{{"name":"host.a","version":1,"operations":[{operation}],"authority":"private-authority"}}"#
    );
    for (raw, diagnostic) in [
        (
            valid.replacen('{', "{\"authority\":\"other\",", 1),
            "duplicate field",
        ),
        (valid.replace(",\"version\":1", ""), "missing field"),
        (valid.replacen('{', "{\"extra\":true,", 1), "unknown field"),
        (
            valid.replace(
                &format!("[{operation}]"),
                &format!("[{operation},{operation}]"),
            ),
            "duplicate broker operation",
        ),
        (
            valid.replace("private-authority", ""),
            "invalid service authority handle",
        ),
    ] {
        assert!(
            serde_json::from_str::<ServiceGrantV1>(&raw)
                .unwrap_err()
                .to_string()
                .contains(diagnostic)
        );
    }
}
