use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use codex_component_api::COMPONENT_API_VERSION;
use codex_component_api::ComponentManifest;
use codex_component_api::ComponentSettings;
use codex_component_api::FILE_SEARCH_CONTRACT_VERSION;
use codex_component_api::FILE_SEARCH_KIND;
use codex_component_api::MANIFEST_FILE;
use codex_component_api::THREAD_STORE_CONTRACT_VERSION;
use semver::Version;
use semver::VersionReq;

use crate::ComponentBinding;

/// Validated activation snapshot. Dependency order is deterministic and no code runs on load.
#[derive(Clone, Default)]
pub struct ComponentCatalog {
    bindings: Vec<ComponentBinding>,
    settings: ComponentSettings,
}

impl std::fmt::Debug for ComponentCatalog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComponentCatalog")
            .field("bindings", &self.bindings)
            .field("enabled", &self.settings.enabled)
            .field("selections", &self.settings.selections)
            .finish_non_exhaustive()
    }
}

impl ComponentCatalog {
    pub fn load(codex_home: &Path) -> Result<Self> {
        let root = codex_home.join("components");
        let settings = read_settings(&root)?;
        Self::from_settings(&root, settings)
    }

    pub(crate) fn from_settings(root: &Path, settings: ComponentSettings) -> Result<Self> {
        for (id, object) in &settings.installed {
            validate_id(id)?;
            ensure!(
                object
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                    && object.len() <= 240
                    && !object.ends_with('.')
                    && object.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                    }),
                "invalid installed component object: {object}"
            );
        }
        for config in settings.config.values() {
            ensure!(
                config.is_object(),
                "plugin configuration must be a JSON object"
            );
        }
        let mut manifests = BTreeMap::new();
        for id in &settings.enabled {
            validate_id(id)?;
            ensure!(!manifests.contains_key(id), "component enabled twice: {id}");
            let package = if let Some(object) = settings.installed.get(id) {
                let objects = root.join("objects").canonicalize()?;
                let package = objects.join(object);
                let metadata = fs::symlink_metadata(&package)?;
                ensure!(
                    metadata.is_dir() && !metadata.file_type().is_symlink(),
                    "installed component object must be a directory without symlinks"
                );
                let canonical = package.canonicalize()?;
                ensure!(
                    canonical.starts_with(&objects),
                    "installed component object escapes its store"
                );
                canonical
            } else {
                // Compatibility for explicitly authored startup configurations.
                root.join("packages").join(id)
            };
            let manifest = read_manifest(&package)?;
            ensure!(
                manifest.id == *id,
                "package identity differs from directory: {id}"
            );
            manifests.insert(id.clone(), (manifest, package));
        }
        let mut ordered = Vec::new();
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for id in manifests.keys() {
            visit(id, &manifests, &mut visiting, &mut visited, &mut ordered)?;
        }
        let mut bindings = Vec::new();
        let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for id in ordered {
            let (manifest, package) = &manifests[&id];
            let package_dir = package.canonicalize()?;
            let entrypoint = package_dir.join(&manifest.entrypoint).canonicalize()?;
            ensure!(
                entrypoint.starts_with(&package_dir),
                "entrypoint escapes package {id}"
            );
            ensure!(entrypoint.is_file(), "entrypoint is not a file: {id}");
            for spec in &manifest.components {
                let key = format!("{}:{}", spec.kind, spec.name);
                owners.entry(key).or_default().push(id.clone());
                bindings.push(ComponentBinding {
                    plugin_id: id.clone(),
                    spec: spec.clone(),
                    config: settings
                        .config
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({})),
                    package_dir: package_dir.clone(),
                    state_dir: absolute(root)?.join("state").join(&id),
                    entrypoint: entrypoint.clone(),
                    args: manifest.args.clone(),
                    timeout_ms: 120_000,
                });
            }
        }
        for (key, id) in &settings.selections {
            ensure!(
                owners.get(key).is_some_and(|ids| ids.contains(id)),
                "selected component {key} is not provided by enabled plugin {id}"
            );
        }
        for (key, ids) in owners {
            ensure!(
                ids.len() == 1 || settings.selections.contains_key(&key),
                "component conflict for {key}: explicitly select the existing provider before installing another, then select the desired provider"
            );
        }
        Ok(Self { bindings, settings })
    }

    /// Additive registrations, filtered by any explicit implementation selections.
    pub fn components(&self, kind: &str) -> Vec<ComponentBinding> {
        self.bindings
            .iter()
            .filter(|binding| {
                binding.spec.kind == kind
                    && self
                        .settings
                        .selections
                        .get(&format!("{}:{}", kind, binding.spec.name))
                        .is_none_or(|id| id == &binding.plugin_id)
            })
            .cloned()
            .collect()
    }

    /// Replacement contracts require an explicit user-owned selection.
    pub fn selected(&self, kind: &str, name: &str) -> Option<ComponentBinding> {
        let id = self.settings.selections.get(&format!("{kind}:{name}"))?;
        self.bindings
            .iter()
            .find(|binding| {
                binding.plugin_id == *id && binding.spec.kind == kind && binding.spec.name == name
            })
            .cloned()
    }

    pub fn settings(&self) -> &ComponentSettings {
        &self.settings
    }
}

fn visit(
    id: &str,
    manifests: &BTreeMap<String, (ComponentManifest, PathBuf)>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(id) {
        return Ok(());
    }
    ensure!(
        visiting.insert(id.to_owned()),
        "component dependency cycle at {id}"
    );
    for (dependency, requirement) in &manifests[id].0.dependencies {
        let Some((manifest, _)) = manifests.get(dependency) else {
            bail!("{id} requires enabled component plugin {dependency}");
        };
        ensure!(
            VersionReq::parse(requirement)?.matches(&Version::parse(&manifest.version)?),
            "{id} requires {dependency} {requirement}, found {}",
            manifest.version
        );
        visit(dependency, manifests, visiting, visited, ordered)?;
    }
    visiting.remove(id);
    visited.insert(id.to_owned());
    ordered.push(id.to_owned());
    Ok(())
}

pub(crate) fn validate_id(id: &str) -> Result<()> {
    ensure!(
        id.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
            && id.len() <= 128
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.'),
        "invalid plugin identity: {id}"
    );
    Ok(())
}

pub(crate) fn read_manifest(package: &Path) -> Result<ComponentManifest> {
    let manifest: ComponentManifest = read_json(&package.join(MANIFEST_FILE))?;
    validate_id(&manifest.id)?;
    ensure!(
        manifest.api_version == COMPONENT_API_VERSION,
        "incompatible component API version"
    );
    Version::parse(&manifest.version).context("invalid plugin version")?;
    let entrypoint = Path::new(&manifest.entrypoint);
    ensure!(
        !manifest.entrypoint.is_empty()
            && entrypoint
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
        "entrypoint must be package-relative"
    );
    ensure!(
        !manifest.components.is_empty(),
        "plugin provides no components"
    );
    let mut keys = BTreeSet::new();
    for spec in &manifest.components {
        ensure!(
            matches!(
                spec.kind.as_str(),
                "tool"
                    | "context"
                    | "lifecycle"
                    | "model_transport"
                    | "presentation"
                    | "auth"
                    | "thread_store"
                    | "attachment_store"
                    | FILE_SEARCH_KIND
            ),
            "unsupported component kind: {}",
            spec.kind
        );
        let supported_version = match spec.kind.as_str() {
            "thread_store" => THREAD_STORE_CONTRACT_VERSION,
            FILE_SEARCH_KIND => FILE_SEARCH_CONTRACT_VERSION,
            _ => 1,
        };
        ensure!(
            spec.contract_version == supported_version,
            "unsupported contract version for {}: expected {supported_version}, received {}",
            spec.kind,
            spec.contract_version
        );
        ensure!(
            !spec.name.is_empty() && spec.name.len() <= 128 && !spec.name.contains(':'),
            "invalid component name"
        );
        ensure!(
            keys.insert((&spec.kind, &spec.name)),
            "duplicate component in manifest"
        );
    }
    for (id, requirement) in &manifest.dependencies {
        validate_id(id)?;
        VersionReq::parse(requirement)?;
    }
    Ok(manifest)
}

pub(crate) fn read_settings(root: &Path) -> Result<ComponentSettings> {
    let path = root.join("config.json");
    if !path.try_exists()? {
        return Ok(ComponentSettings::default());
    }
    read_json(&path)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .with_context(|| format!("read {}", path.display()))?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 1024 * 1024,
        "component configuration exceeds 1 MiB"
    );
    serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
}

fn absolute(path: &Path) -> Result<PathBuf> {
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    })
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
