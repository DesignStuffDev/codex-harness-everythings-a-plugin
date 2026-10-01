#![allow(clippy::expect_used, clippy::unwrap_used)]

use pretty_assertions::assert_eq;
use serde_json::json;

use crate::BrokerAcknowledgementV1;
use crate::BrokerConnectionHandle;
use crate::BrokerConnectionLimits;
use crate::BrokerConnectionLimitsSpec;
use crate::BrokerNegotiationError;
use crate::BrokerOfferV1;
use crate::GrantedOperationV1;
use crate::HostServiceRequirement;
use crate::ServiceAuthorityHandle;
use crate::ServiceGrantV1;
use crate::ServicePresence;
use crate::ServiceRequirementsV1;

fn limits() -> BrokerConnectionLimits {
    BrokerConnectionLimits::new(BrokerConnectionLimitsSpec {
        max_in_flight: 4,
        max_request_bytes: 1024,
        max_response_bytes: 2048,
        max_spool_bytes: 8192,
        max_decoded_nodes: 4096,
        max_decoded_string_bytes: 4096,
        max_decode_jobs: 1,
        max_encode_jobs: 1,
        max_ordinary_codec_jobs: 1,
    })
    .unwrap()
}

fn grant(name: &str, version: u32, operations: &[&str]) -> ServiceGrantV1 {
    ServiceGrantV1::new(
        name.to_owned(),
        version,
        operations
            .iter()
            .map(|name| GrantedOperationV1::new((*name).to_owned(), 32, 64).unwrap())
            .collect(),
        ServiceAuthorityHandle::new(format!("authority-{name}-{version}")).unwrap(),
    )
    .unwrap()
}

fn offer(services: Vec<ServiceGrantV1>) -> BrokerOfferV1 {
    BrokerOfferV1::new(
        BrokerConnectionHandle::new("connection-secret".into()).unwrap(),
        limits(),
        services,
    )
    .unwrap()
}

fn declaration(
    name: &str,
    version: u32,
    operations: &[&str],
    presence: ServicePresence,
) -> HostServiceRequirement {
    HostServiceRequirement::new(
        name.to_owned(),
        version,
        operations.iter().map(|name| (*name).to_owned()).collect(),
        presence,
    )
    .unwrap()
}

#[test]
fn reordered_service_and_operation_sets_have_one_offer_and_acknowledgement() {
    let canonical = offer(vec![
        grant("a", 1, &["read", "write"]),
        grant("a", 2, &["read"]),
        grant("z", 1, &["read"]),
    ]);
    let reordered = offer(vec![
        grant("z", 1, &["read"]),
        grant("a", 2, &["read"]),
        grant("a", 1, &["write", "read"]),
    ]);
    assert_eq!(canonical, reordered);
    let bytes = serde_json::to_vec(&canonical).unwrap();
    assert_eq!(
        serde_json::from_slice::<BrokerOfferV1>(&bytes).unwrap(),
        canonical
    );
    let mut raw = serde_json::to_value(&canonical).unwrap();
    raw["services"].as_array_mut().unwrap().reverse();
    for service in raw["services"].as_array_mut().unwrap() {
        service["operations"].as_array_mut().unwrap().reverse();
    }
    let acknowledgement: BrokerAcknowledgementV1 =
        serde_json::from_slice(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(
        acknowledgement,
        BrokerAcknowledgementV1::from_offer(&canonical)
    );
    assert_eq!(
        serde_json::to_value(&acknowledgement).unwrap(),
        serde_json::to_value(&canonical).unwrap()
    );
    assert_eq!(canonical.validate_acknowledgement(&acknowledgement), Ok(()));
    let requirements = ServiceRequirementsV1::new(vec![
        declaration("z", 1, &["read"], ServicePresence::Required),
        declaration("a", 2, &["read"], ServicePresence::Required),
        declaration("a", 1, &["write", "read"], ServicePresence::Required),
    ])
    .unwrap();
    assert_eq!(canonical.validate_requirements(&requirements), Ok(()));
}

#[test]
fn offer_rejects_duplicate_services_and_directional_caps_at_connection_boundaries() {
    let services: Vec<_> = (0..32)
        .map(|n| grant(&format!("service-{n}"), 1, &["read"]))
        .collect();
    let maximum = offer(services.clone());
    assert_eq!(
        serde_json::from_slice::<BrokerOfferV1>(&serde_json::to_vec(&maximum).unwrap()).unwrap(),
        maximum
    );
    let mut too_many = services;
    too_many.push(grant("extra", 1, &["read"]));
    let connection = BrokerConnectionHandle::new("connection".into()).unwrap();
    assert_eq!(
        BrokerOfferV1::new(connection.clone(), limits(), too_many),
        Err(BrokerNegotiationError::TooManyServices)
    );
    assert_eq!(
        BrokerOfferV1::new(
            connection.clone(),
            limits(),
            vec![grant("a", 1, &["read"]), grant("a", 1, &["write"])]
        ),
        Err(BrokerNegotiationError::DuplicateService)
    );
    for (request, response, valid) in [(1024, 2048, true), (1025, 2048, false), (1024, 2049, false)]
    {
        let service = ServiceGrantV1::new(
            "a".into(),
            1,
            vec![GrantedOperationV1::new("read".into(), request, response).unwrap()],
            ServiceAuthorityHandle::new("authority".into()).unwrap(),
        )
        .unwrap();
        let actual = BrokerOfferV1::new(connection.clone(), limits(), vec![service]);
        if valid {
            assert!(actual.is_ok());
        } else {
            assert_eq!(
                actual,
                Err(BrokerNegotiationError::OperationLimitExceedsConnection)
            );
        }
    }
}

#[test]
fn required_and_optional_descriptors_match_exact_versions_and_complete_operations() {
    let optional = ServiceRequirementsV1::new(vec![declaration(
        "a",
        1,
        &["write", "read"],
        ServicePresence::Optional,
    )])
    .unwrap();
    assert_eq!(offer(vec![]).validate_requirements(&optional), Ok(()));
    assert_eq!(
        offer(vec![grant("a", 1, &["read", "write"])]).validate_requirements(&optional),
        Ok(())
    );
    assert_eq!(
        offer(vec![grant("a", 1, &["read"])]).validate_requirements(&optional),
        Err(BrokerNegotiationError::OperationSetMismatch)
    );
    assert_eq!(
        offer(vec![grant("a", 1, &["read", "write", "extra"])]).validate_requirements(&optional),
        Err(BrokerNegotiationError::OperationSetMismatch)
    );
    for service in [
        grant("a", 2, &["read", "write"]),
        grant("A", 1, &["read", "write"]),
    ] {
        assert_eq!(
            offer(vec![service]).validate_requirements(&optional),
            Err(BrokerNegotiationError::UndeclaredService)
        );
    }
    let mixed = ServiceRequirementsV1::new(vec![
        declaration("a", 1, &["read"], ServicePresence::Required),
        declaration("b", 1, &["read"], ServicePresence::Optional),
    ])
    .unwrap();
    assert_eq!(
        offer(vec![grant("a", 1, &["read"])]).validate_requirements(&mixed),
        Ok(())
    );
    assert_eq!(
        offer(vec![]).validate_requirements(&mixed),
        Err(BrokerNegotiationError::MissingRequiredService)
    );
    assert_eq!(
        offer(vec![grant("b", 1, &["read"])]).validate_requirements(&mixed),
        Err(BrokerNegotiationError::MissingRequiredService)
    );
    let exact_unicode = ServiceRequirementsV1::new(vec![declaration(
        "é",
        1,
        &["read"],
        ServicePresence::Required,
    )])
    .unwrap();
    assert_eq!(
        offer(vec![grant("e\u{301}", 1, &["read"])]).validate_requirements(&exact_unicode),
        Err(BrokerNegotiationError::UndeclaredService)
    );
}

#[test]
fn acknowledgement_must_echo_every_limit_handle_descriptor_operation_and_cap() {
    let offered = offer(vec![grant("a", 1, &["read", "write"])]);
    let original = serde_json::to_value(&offered).unwrap();
    let mut alternatives = Vec::new();
    for (field, value) in [
        ("max_in_flight", 5),
        ("max_request_bytes", 1025),
        ("max_response_bytes", 2049),
        ("max_spool_bytes", 8193),
        ("max_decoded_nodes", 4097),
        ("max_decoded_string_bytes", 4097),
        ("max_decode_jobs", 2),
        ("max_encode_jobs", 2),
        ("max_ordinary_codec_jobs", 2),
    ] {
        let mut changed = original.clone();
        changed["limits"][field] = json!(value);
        alternatives.push(changed);
    }
    for pointer in [
        "/connection",
        "/services/0/authority",
        "/services/0/name",
        "/services/0/operations/0/name",
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("different-secret");
        alternatives.push(changed);
    }
    for (pointer, value) in [
        ("/services/0/version", 2),
        ("/services/0/operations/0/max_request_bytes", 33),
        ("/services/0/operations/0/max_request_bytes", 31),
        ("/services/0/operations/0/max_response_bytes", 65),
        ("/services/0/operations/0/max_response_bytes", 63),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = json!(value);
        alternatives.push(changed);
    }
    let mut missing_service = original.clone();
    missing_service["services"] = json!([]);
    alternatives.push(missing_service);
    let mut extra_service = original.clone();
    extra_service["services"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(grant("b", 1, &["read"])).unwrap());
    alternatives.push(extra_service);
    let mut missing_operation = original.clone();
    missing_operation["services"][0]["operations"]
        .as_array_mut()
        .unwrap()
        .pop();
    alternatives.push(missing_operation);
    let mut extra_operation = original;
    extra_operation["services"][0]["operations"]
        .as_array_mut()
        .unwrap()
        .push(
            serde_json::to_value(GrantedOperationV1::new("extra".into(), 32, 64).unwrap()).unwrap(),
        );
    alternatives.push(extra_operation);
    for changed in alternatives {
        let acknowledgement: BrokerAcknowledgementV1 =
            serde_json::from_slice(&serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(
            offered.validate_acknowledgement(&acknowledgement),
            Err(BrokerNegotiationError::AcknowledgementMismatch)
        );
    }
    let acknowledgement = BrokerAcknowledgementV1::from_offer(&offer(vec![]));
    assert!(
        !offered
            .validate_acknowledgement(&acknowledgement)
            .unwrap_err()
            .to_string()
            .contains("secret")
    );
}

#[test]
fn offer_and_acknowledgement_parse_original_strict_bytes_through_nested_checks() {
    let valid = serde_json::to_string(&offer(vec![grant("a", 1, &["read"])])).unwrap();
    let mut malformed = Vec::new();
    for (needle, replacement) in [
        ("{\"version\":", "{\"version\":1,\"version\":"),
        ("{\"version\":", "{\"unknown\":true,\"version\":"),
        ("\"limits\":{", "\"limits\":{\"max_in_flight\":4,"),
        ("\"services\":[{", "\"services\":[{\"name\":\"a\","),
        ("\"operations\":[{", "\"operations\":[{\"name\":\"read\","),
    ] {
        assert!(
            valid.contains(needle),
            "raw fixture substitution must apply"
        );
        malformed.push(valid.replacen(needle, replacement, 1));
    }
    for version in [0, 2, u32::MAX] {
        let mut raw = serde_json::from_str::<serde_json::Value>(&valid).unwrap();
        raw["version"] = json!(version);
        malformed.push(serde_json::to_string(&raw).unwrap());
    }
    for field in ["version", "connection", "limits", "services"] {
        let mut raw = serde_json::from_str::<serde_json::Value>(&valid).unwrap();
        raw.as_object_mut().unwrap().remove(field);
        malformed.push(serde_json::to_string(&raw).unwrap());
    }
    let mut over_cap = serde_json::from_str::<serde_json::Value>(&valid).unwrap();
    over_cap["services"][0]["operations"][0]["max_response_bytes"] = json!(2049);
    malformed.push(serde_json::to_string(&over_cap).unwrap());
    for raw in malformed {
        assert!(
            serde_json::from_str::<BrokerOfferV1>(&raw).is_err(),
            "invalid offer accepted"
        );
        assert!(
            serde_json::from_str::<BrokerAcknowledgementV1>(&raw).is_err(),
            "invalid acknowledgement accepted"
        );
    }
}
