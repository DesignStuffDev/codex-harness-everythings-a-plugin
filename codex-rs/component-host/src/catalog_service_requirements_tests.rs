#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;

use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;

use super::read_manifest;
use codex_component_api::ComponentManifest;
use codex_component_api::MANIFEST_FILE;

#[test]
fn broker_declarations_fail_closed_until_runtime_activation() {
    let package = TempDir::new().unwrap();
    let mut manifest = json!({
        "api_version": 1, "id": "broker.example", "version": "1.0.0",
        "entrypoint": "worker", "args": [], "dependencies": {},
        "components": [{"kind": "tool", "name": "example", "contract_version": 1, "metadata": {}}]
    });
    let path = package.path().join(MANIFEST_FILE);
    fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let original: ComponentManifest = serde_json::from_value(manifest.clone()).unwrap();
    assert_eq!(read_manifest(package.path()).unwrap(), original);
    for presence in ["required", "optional"] {
        manifest["components"][0]["service_requirements"] = json!({
            "version": 1, "services": [{"name": "host.echo", "version": 1,
            "operations": ["echo"], "presence": presence}]
        });
        // Valid DTOs must not imply this runtime has activated a broker.
        assert!(serde_json::from_value::<ComponentManifest>(manifest.clone()).is_ok());
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(
            read_manifest(package.path()).unwrap_err().to_string(),
            "host service requirements are not supported by this host"
        );
    }
    manifest["components"][0]
        .as_object_mut()
        .unwrap()
        .remove("service_requirements");
    fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert_eq!(read_manifest(package.path()).unwrap(), original);
}
