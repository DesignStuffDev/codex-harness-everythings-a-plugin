#![allow(clippy::expect_used, clippy::unwrap_used)]

use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_json::json;

use crate::ComponentManifest;
use crate::ComponentSpec;
use crate::HostServiceRequirement;
use crate::ServicePresence;
use crate::ServiceRequirementsError;
use crate::ServiceRequirementsV1;

fn service(name: &str) -> HostServiceRequirement {
    HostServiceRequirement::new(
        name.to_owned(),
        /*version*/ 1,
        vec!["fetch".to_owned(), "validate_owner".to_owned()],
        ServicePresence::Required,
    )
    .unwrap()
}

#[test]
fn legacy_manifest_round_trip_omits_service_requirements() {
    let legacy = json!({
        "api_version": 1, "id": "old.package", "version": "1.0.0",
        "entrypoint": "worker", "args": [], "dependencies": {},
        "components": [{"kind": "tool", "name": "old", "contract_version": 1, "metadata": {}}]
    });
    let parsed: ComponentManifest = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(parsed.components[0].service_requirements, None);
    assert_eq!(serde_json::to_value(parsed).unwrap(), legacy);
}

#[test]
fn checked_requirements_preserve_exact_operations_and_presence() {
    let raw = json!({"version": 1, "services": [{
        "name": "host.model_endpoint", "version": 1,
        "operations": ["fetch", "validate_owner"], "presence": "required"
    }]});
    let actual: ServiceRequirementsV1 = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(
        actual,
        ServiceRequirementsV1::new(vec![service("host.model_endpoint")]).unwrap()
    );
    assert_eq!(serde_json::to_value(&actual).unwrap(), raw);
    assert_eq!(actual.services()[0].presence(), ServicePresence::Required);
    let optional = json!({"version": 1, "services": [{
        "name": "host.model_endpoint", "version": 1,
        "operations": ["fetch"], "presence": "optional"
    }]});
    let actual: ServiceRequirementsV1 = serde_json::from_value(optional.clone()).unwrap();
    assert_eq!(serde_json::to_value(actual).unwrap(), optional);
}

#[test]
fn declaration_versions_and_service_count_fail_before_use() {
    for version in [0, 2, u32::MAX] {
        let raw = json!({"version": version, "services": [service("host.a")]});
        assert!(serde_json::from_value::<ServiceRequirementsV1>(raw).is_err());
    }
    assert_eq!(
        ServiceRequirementsV1::new(Vec::new()),
        Err(ServiceRequirementsError::InvalidServiceCount)
    );
    let services: Vec<_> = (0..32).map(|n| service(&format!("host.{n}"))).collect();
    let parsed: ServiceRequirementsV1 =
        serde_json::from_value(json!({"version": 1, "services": services})).unwrap();
    assert_eq!(parsed.services().len(), 32);
    let mut overflow = parsed.services().to_vec();
    overflow.push(service("host.extra"));
    assert_eq!(
        ServiceRequirementsV1::new(overflow),
        Err(ServiceRequirementsError::InvalidServiceCount)
    );
    assert_eq!(
        ServiceRequirementsV1::new(vec![service("host.a"), service("host.a")]),
        Err(ServiceRequirementsError::DuplicateService)
    );
}

#[test]
fn descriptors_enforce_utf8_byte_limits_and_duplicate_operations() {
    let operations: Vec<_> = (0..32).map(|n| format!("op{n}")).collect();
    let valid = HostServiceRequirement::new(
        "é".repeat(64),
        /*version*/ 1,
        operations.clone(),
        ServicePresence::Optional,
    )
    .unwrap();
    assert_eq!(valid.operations(), operations.as_slice());
    assert!(
        HostServiceRequirement::new(
            "host.a".into(),
            /*version*/ 1,
            vec!["é".repeat(128)],
            ServicePresence::Required
        )
        .is_ok()
    );
    assert_eq!(
        HostServiceRequirement::new(
            "é".repeat(65),
            /*version*/ 1,
            vec!["x".into()],
            ServicePresence::Optional
        ),
        Err(ServiceRequirementsError::InvalidServiceName)
    );
    for (name, version, ops, expected) in [
        (
            "",
            1,
            vec!["x".to_owned()],
            ServiceRequirementsError::InvalidServiceName,
        ),
        (
            "host.a",
            0,
            vec!["x".to_owned()],
            ServiceRequirementsError::InvalidServiceVersion,
        ),
        (
            "host.a",
            1,
            vec![],
            ServiceRequirementsError::InvalidOperationCount,
        ),
        (
            "host.a",
            1,
            vec![String::new()],
            ServiceRequirementsError::InvalidOperationName,
        ),
        (
            "host.a",
            1,
            vec!["é".repeat(129)],
            ServiceRequirementsError::InvalidOperationName,
        ),
        (
            "host.a",
            1,
            vec!["x".to_owned(), "x".to_owned()],
            ServiceRequirementsError::DuplicateOperation,
        ),
    ] {
        assert_eq!(
            HostServiceRequirement::new(name.to_owned(), version, ops, ServicePresence::Required),
            Err(expected)
        );
    }
    let mut too_many = operations;
    too_many.push("extra".into());
    assert_eq!(
        HostServiceRequirement::new(
            "host.a".into(),
            /*version*/ 1,
            too_many,
            ServicePresence::Required
        ),
        Err(ServiceRequirementsError::InvalidOperationCount)
    );
}

#[test]
fn raw_duplicates_and_unknown_fields_are_not_erased_by_value_conversion() {
    for (raw, diagnostic) in [
        (
            r#"{"version":1,"version":1,"services":[{"name":"a","version":1,"operations":["x"],"presence":"required"}]}"#,
            "duplicate field",
        ),
        (
            r#"{"version":1,"services":[{"name":"a","version":1,"operations":["x"],"presence":"required"}],"unknown":true}"#,
            "unknown field",
        ),
        (
            r#"{"version":1,"services":[{"name":"a","name":"b","version":1,"operations":["x"],"presence":"required"}]}"#,
            "duplicate field",
        ),
        (
            r#"{"version":1,"services":[{"name":"a","version":1,"operations":["x"],"operations":["y"],"presence":"required"}]}"#,
            "duplicate field",
        ),
        (
            r#"{"version":1,"services":[{"name":"a","version":1,"operations":["x"],"presence":"required","authority":"forged"}]}"#,
            "unknown field",
        ),
        (
            r#"{"version":1,"services":[{"name":"a","version":1,"operations":["x","x"],"presence":"required"}]}"#,
            "duplicate service operation",
        ),
        (
            r#"{"version":1,"services":[{"name":"a","version":1,"operations":["x"],"presence":"maybe"}]}"#,
            "unknown variant",
        ),
    ] {
        let error = serde_json::from_str::<ServiceRequirementsV1>(raw).unwrap_err();
        assert!(
            error.to_string().contains(diagnostic),
            "wrong malformed-declaration diagnostic"
        );
    }
}

#[test]
fn broker_manifest_is_rejected_by_the_legacy_strict_component_schema() {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct OldComponentSpec {
        kind: String,
        name: String,
        contract_version: u32,
        #[serde(default)]
        metadata: serde_json::Value,
    }
    let plain = json!({"kind":"tool","name":"demo","contract_version":1,"metadata":{}});
    let old: OldComponentSpec = serde_json::from_value(plain.clone()).unwrap();
    assert_eq!(
        (old.kind, old.name, old.contract_version, old.metadata),
        ("tool".to_owned(), "demo".to_owned(), 1, json!({}))
    );
    let mut spec: ComponentSpec = serde_json::from_value(plain).unwrap();
    spec.service_requirements = Some(ServiceRequirementsV1::new(vec![service("host.a")]).unwrap());
    let bytes = serde_json::to_vec(&spec).unwrap();
    assert_eq!(
        serde_json::from_slice::<ComponentSpec>(&bytes).unwrap(),
        spec
    );
    let error = serde_json::from_slice::<OldComponentSpec>(&bytes)
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("unknown field `service_requirements`")
    );
}
