#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;

fn package(root: &Path, id: &str, dependencies: serde_json::Value) -> PathBuf {
    let path = root.join(id);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("entrypoint"), "test fixture").unwrap();
    fs::write(
        path.join(MANIFEST_FILE),
        serde_json::to_vec(&json!({
            "api_version": 1, "id": id, "version": "1.2.0", "entrypoint": "entrypoint",
            "dependencies": dependencies,
            "components": [{"kind":"tool","name":id,"contract_version":1,"metadata":{}}]
        }))
        .unwrap(),
    )
    .unwrap();
    path
}

#[test]
fn absent_settings_preserves_native_runtime() {
    let home = TempDir::new().unwrap();
    let catalog = ComponentCatalog::load(home.path()).unwrap();
    assert_eq!(catalog.settings(), &ComponentSettings::default());
    assert!(catalog.selected("model_transport", "default").is_none());
    assert!(!home.path().join("components").exists());
}

#[test]
fn debug_output_omits_plugin_credentials_and_arguments() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("components");
    package(&root.join("packages"), "example", json!({}));
    let mut catalog = ComponentCatalog::from_settings(
        &root,
        ComponentSettings {
            enabled: vec!["example".to_owned()],
            config: BTreeMap::from([(
                "example".to_owned(),
                json!({"token":"sentinel-config-secret"}),
            )]),
            ..Default::default()
        },
    )
    .unwrap();
    catalog.bindings[0].args = vec!["sentinel-argument-secret".to_owned()];
    catalog.bindings[0].spec.metadata = json!({"token":"sentinel-metadata-secret"});
    for diagnostic in [format!("{:?}", catalog.bindings[0]), format!("{catalog:?}")] {
        assert!(diagnostic.contains("example"));
        for secret in [
            "sentinel-config-secret",
            "sentinel-argument-secret",
            "sentinel-metadata-secret",
        ] {
            assert!(!diagnostic.contains(secret));
        }
    }
}

#[test]
fn installs_resolves_dependencies_and_preserves_state_on_removal() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let dependency = package(sources.path(), "example.base", json!({}));
    let dependent = package(
        sources.path(),
        "example.child",
        json!({"example.base":"^1.0"}),
    );
    crate::install(home.path(), &dependency).unwrap();
    crate::install(home.path(), &dependent).unwrap();
    let bindings = ComponentCatalog::load(home.path())
        .unwrap()
        .components("tool");
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.plugin_id.as_str())
            .collect::<Vec<_>>(),
        vec!["example.base", "example.child"]
    );
    assert_eq!(bindings[0].config, json!({}));
    fs::create_dir_all(&bindings[1].state_dir).unwrap();
    fs::write(bindings[1].state_dir.join("checkpoint"), "durable").unwrap();
    assert!(crate::remove(home.path(), "example.base").is_err());
    crate::remove(home.path(), "example.child").unwrap();
    assert_eq!(
        fs::read_to_string(bindings[1].state_dir.join("checkpoint")).unwrap(),
        "durable"
    );
    crate::remove(home.path(), "example.base").unwrap();
    assert!(
        ComponentCatalog::load(home.path())
            .unwrap()
            .components("tool")
            .is_empty()
    );
}

#[test]
fn invalid_install_rolls_back_without_changing_registry() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let source = package(sources.path(), "dependent", json!({"missing":"^1.0"}));
    assert!(crate::install(home.path(), &source).is_err());
    assert_eq!(
        fs::read_dir(home.path().join("components/objects"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        ComponentCatalog::load(home.path()).unwrap().settings(),
        &ComponentSettings::default()
    );
}

#[test]
fn explicit_selection_can_be_reset_and_does_not_accept_unknown_provider() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let source = package(sources.path(), "example.tool", json!({}));
    crate::install(home.path(), &source).unwrap();
    crate::select(home.path(), "tool", "example.tool", Some("example.tool")).unwrap();
    assert!(
        ComponentCatalog::load(home.path())
            .unwrap()
            .selected("tool", "example.tool")
            .is_some()
    );
    assert!(crate::select(home.path(), "tool", "example.tool", Some("missing")).is_err());
    crate::select(home.path(), "tool", "example.tool", None).unwrap();
    assert!(
        ComponentCatalog::load(home.path())
            .unwrap()
            .selected("tool", "example.tool")
            .is_none()
    );
}

#[test]
fn incompatible_versions_cycles_and_conflicts_fail_before_activation() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("components");
    let packages = root.join("packages");
    package(&packages, "a", json!({"b":"^2.0"}));
    package(&packages, "b", json!({}));
    let settings = ComponentSettings {
        enabled: vec!["a".into(), "b".into()],
        ..Default::default()
    };
    assert!(
        ComponentCatalog::from_settings(&root, settings.clone())
            .unwrap_err()
            .to_string()
            .contains("requires b")
    );
    package(&packages, "a", json!({"b":"^1.0"}));
    package(&packages, "b", json!({"a":"^1.0"}));
    assert!(
        ComponentCatalog::from_settings(&root, settings)
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
}

#[test]
fn manifest_rejects_unsupported_contract_and_entrypoint_escape() {
    let source = TempDir::new().unwrap();
    let path = package(source.path(), "example", json!({}));
    let manifest_path = path.join(MANIFEST_FILE);
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    for modified in [
        {
            let mut value = original.clone();
            value["api_version"] = json!(99);
            value
        },
        {
            let mut value = original.clone();
            value["entrypoint"] = json!("../outside");
            value
        },
        {
            let mut value = original.clone();
            value["components"][0]["kind"] = json!("unknown_contract");
            value
        },
        {
            let mut value = original;
            value["components"][0]["contract_version"] = json!(99);
            value
        },
    ] {
        fs::write(&manifest_path, serde_json::to_vec(&modified).unwrap()).unwrap();
        assert!(read_manifest(&path).is_err());
    }
}

#[cfg(unix)]
#[test]
fn installation_rejects_symlinks_without_copying_outside_content() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let source = package(sources.path(), "example", json!({}));
    std::os::unix::fs::symlink("/etc/passwd", source.join("outside")).unwrap();
    assert!(crate::install(home.path(), &source).is_err());
    assert_eq!(
        fs::read_dir(home.path().join("components/objects"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn storage_installation_rejects_lossy_contract_before_activation() {
    let home = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let package = package(source.path(), "example.storage", json!({}));
    let manifest_path = package.join(MANIFEST_FILE);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["components"][0]["kind"] = json!("thread_store");
    manifest["components"][0]["name"] = json!("default");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(crate::install(home.path(), &package).is_err());
    assert!(
        ComponentCatalog::load(home.path())
            .unwrap()
            .bindings
            .is_empty()
    );

    manifest["components"][0]["contract_version"] = json!(THREAD_STORE_CONTRACT_VERSION);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    crate::install(home.path(), &package).unwrap();
    crate::select(
        home.path(),
        "thread_store",
        "default",
        Some("example.storage"),
    )
    .unwrap();
    let selected = ComponentCatalog::load(home.path())
        .unwrap()
        .selected("thread_store", "default")
        .unwrap();
    assert_eq!(
        selected.spec.contract_version,
        THREAD_STORE_CONTRACT_VERSION
    );
}

#[tokio::test]
async fn removal_and_reinstallation_preserve_existing_bindings() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let source = package(sources.path(), "example", json!({}));
    let old_code = "#!/bin/sh\nread -r init\nprintf '%s\\n' '{\"type\":\"ready\",\"api_version\":1}'\nread -r request\nprintf '%s\\n' '{\"type\":\"result\",\"id\":1,\"result\":\"old\"}'\nread -r shutdown\n";
    let new_code = old_code.replace("\"old\"", "\"new\"");
    fs::write(source.join("entrypoint"), old_code).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(source.join("entrypoint"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    crate::install(home.path(), &source).unwrap();
    let before_removal = ComponentCatalog::load(home.path()).unwrap();
    let old_binding = before_removal.components("tool").remove(0);
    crate::select(home.path(), "tool", "example", Some("example")).unwrap();
    fs::create_dir_all(&old_binding.state_dir).unwrap();
    fs::write(old_binding.state_dir.join("checkpoint"), "keep").unwrap();

    crate::remove(home.path(), "example").unwrap();
    let removed = ComponentCatalog::load(home.path()).unwrap();
    assert_eq!(removed.settings(), &ComponentSettings::default());
    assert!(crate::remove(home.path(), "example").is_err());
    assert_eq!(
        fs::read_to_string(&old_binding.entrypoint).unwrap(),
        old_code
    );
    #[cfg(unix)]
    assert_eq!(
        old_binding.call("execute", json!({})).await.unwrap(),
        json!("old")
    );

    fs::write(source.join("entrypoint"), &new_code).unwrap();
    crate::install(home.path(), &source).unwrap();
    let after_reinstall = ComponentCatalog::load(home.path()).unwrap();
    let new_binding = after_reinstall.components("tool").remove(0);
    assert_ne!(
        before_removal.settings().installed,
        after_reinstall.settings().installed
    );
    assert_ne!(old_binding.package_dir, new_binding.package_dir);
    assert_eq!(old_binding.state_dir, new_binding.state_dir);
    assert_eq!(
        fs::read_to_string(&old_binding.entrypoint).unwrap(),
        old_code
    );
    assert_eq!(
        fs::read_to_string(&new_binding.entrypoint).unwrap(),
        new_code
    );
    assert_eq!(
        fs::read_to_string(new_binding.state_dir.join("checkpoint")).unwrap(),
        "keep"
    );
    #[cfg(unix)]
    {
        assert_eq!(
            old_binding.call("execute", json!({})).await.unwrap(),
            json!("old")
        );
        assert_eq!(
            new_binding.call("execute", json!({})).await.unwrap(),
            json!("new")
        );
    }
    let unchanged = fs::read(home.path().join("components/config.json")).unwrap();
    assert!(crate::install(home.path(), &source).is_err());
    assert_eq!(
        fs::read(home.path().join("components/config.json")).unwrap(),
        unchanged
    );
}

#[test]
fn installed_object_names_cannot_escape_the_store() {
    let home = TempDir::new().unwrap();
    for object in [
        "../escaped",
        "/absolute",
        "folder/object",
        "folder\\object",
        "",
        "trailing.",
    ] {
        let settings = ComponentSettings {
            enabled: vec!["example".to_owned()],
            installed: BTreeMap::from([("example".to_owned(), object.to_owned())]),
            ..Default::default()
        };
        assert!(
            ComponentCatalog::from_settings(&home.path().join("components"), settings)
                .unwrap_err()
                .to_string()
                .contains("invalid installed component object")
        );
    }
}

#[cfg(unix)]
#[test]
fn installed_objects_cannot_be_symlinks_to_external_packages() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let source = package(sources.path(), "example", json!({}));
    let root = home.path().join("components");
    fs::create_dir_all(root.join("objects")).unwrap();
    std::os::unix::fs::symlink(&source, root.join("objects/external")).unwrap();
    let settings = ComponentSettings {
        enabled: vec!["example".to_owned()],
        installed: BTreeMap::from([("example".to_owned(), "external".to_owned())]),
        ..Default::default()
    };
    assert!(
        ComponentCatalog::from_settings(&root, settings)
            .unwrap_err()
            .to_string()
            .contains("without symlinks")
    );
}

#[test]
fn installation_rejects_a_store_nested_inside_its_source() {
    let sources = TempDir::new().unwrap();
    let source = package(sources.path(), "example", json!({}));
    let home = source.join("nested-home");
    let error = crate::install(&home, &source).unwrap_err();
    assert!(error.to_string().contains("inside the source package"));
    assert_eq!(
        ComponentCatalog::load(&home).unwrap().settings(),
        &ComponentSettings::default()
    );
    assert_eq!(
        fs::read_dir(home.join("components/objects"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn competing_install_requires_preserving_the_current_explicit_selection() {
    let home = TempDir::new().unwrap();
    let sources = TempDir::new().unwrap();
    let first = package(sources.path(), "first", json!({}));
    let second = package(sources.path(), "second", json!({}));
    let manifest_path = second.join(MANIFEST_FILE);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["components"][0]["name"] = json!("first");
    fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    crate::install(home.path(), &first).unwrap();
    let settings_before = fs::read(home.path().join("components/config.json")).unwrap();
    let error = crate::install(home.path(), &second).unwrap_err();
    assert!(error.to_string().contains("select the existing provider"));
    assert_eq!(
        fs::read(home.path().join("components/config.json")).unwrap(),
        settings_before
    );
    assert_eq!(
        fs::read_dir(home.path().join("components/objects"))
            .unwrap()
            .count(),
        1
    );

    crate::select(home.path(), "tool", "first", Some("first")).unwrap();
    crate::install(home.path(), &second).unwrap();
    let catalog = ComponentCatalog::load(home.path()).unwrap();
    assert_eq!(catalog.components("tool")[0].plugin_id, "first");
    crate::select(home.path(), "tool", "first", Some("second")).unwrap();
    let catalog = ComponentCatalog::load(home.path()).unwrap();
    assert_eq!(catalog.components("tool")[0].plugin_id, "second");
}
