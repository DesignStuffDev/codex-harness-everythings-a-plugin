use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;

use crate::ComponentCatalog;
use crate::ComponentSettings;
use crate::catalog::read_manifest;
use crate::catalog::read_settings;
use crate::catalog::validate_id;

struct RegistryLock(PathBuf);

static NEXT_OBJECT: AtomicU64 = AtomicU64::new(0);

impl RegistryLock {
    fn acquire(root: &Path) -> Result<Self> {
        fs::create_dir_all(root)?;
        let path = root.join("registry.lock");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .context("component registry is locked; another management command may be running")?;
        let lock = Self(path);
        writeln!(file, "{}", std::process::id())?;
        Ok(lock)
    }
}

impl Drop for RegistryLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Installs a local package and explicitly enables it, without executing plugin code.
/// Every installation gets a distinct immutable object. Remove the activation
/// before upgrading; old objects remain available to already-running catalogs.
pub fn install(codex_home: &Path, package: &Path) -> Result<String> {
    let manifest = read_manifest(package)?;
    let root = codex_home.join("components");
    let _lock = RegistryLock::acquire(&root)?;
    let mut settings = read_settings(&root)?;
    ensure!(
        !settings.installed.contains_key(&manifest.id) && !settings.enabled.contains(&manifest.id),
        "plugin is already installed: {}",
        manifest.id
    );
    let objects = root.join("objects");
    fs::create_dir_all(&objects)?;
    let source = package.canonicalize()?;
    ensure!(
        !objects.canonicalize()?.starts_with(&source),
        "installation directory cannot be inside the source package"
    );
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let sequence = NEXT_OBJECT.fetch_add(1, Ordering::Relaxed);
    let object = format!(
        "{}-{}-{timestamp:x}-{sequence:x}",
        manifest.id,
        std::process::id()
    );
    let destination = objects.join(&object);
    let staging = objects.join(format!(".install-{object}"));
    ensure!(
        !destination.try_exists()? && !staging.try_exists()?,
        "component object already exists; retry installation"
    );
    let installation = (|| {
        copy_package(package, &staging, &mut 0)?;
        fs::rename(&staging, &destination)?;
        settings.installed.insert(manifest.id.clone(), object);
        settings.enabled.push(manifest.id.clone());
        ComponentCatalog::from_settings(&root, settings.clone())?;
        write_settings(&root, &settings)
    })();
    if installation.is_err() {
        let _ = fs::remove_dir_all(&staging);
        let _ = fs::remove_dir_all(&destination);
    }
    installation?;
    Ok(manifest.id)
}

/// Removes activation and the installed-object mapping for future host startups.
/// Package objects and durable state are retained so active catalogs continue to
/// work. Reclaiming cached code requires a future stopped-host garbage collector.
pub fn remove(codex_home: &Path, id: &str) -> Result<()> {
    validate_id(id)?;
    let root = codex_home.join("components");
    let _lock = RegistryLock::acquire(&root)?;
    let mut settings = read_settings(&root)?;
    ensure!(
        settings.installed.contains_key(id) || settings.enabled.iter().any(|enabled| enabled == id),
        "plugin is not installed: {id}"
    );
    settings.enabled.retain(|enabled| enabled != id);
    settings.selections.retain(|_, selected| selected != id);
    settings.installed.remove(id);
    ComponentCatalog::from_settings(&root, settings.clone())
        .context("cannot remove plugin required by another enabled plugin")?;
    write_settings(&root, &settings)
}

/// Selects a replacement for one named contract, or restores its native default.
pub fn select(codex_home: &Path, kind: &str, name: &str, plugin_id: Option<&str>) -> Result<()> {
    let root = codex_home.join("components");
    let _lock = RegistryLock::acquire(&root)?;
    let mut settings = read_settings(&root)?;
    let key = format!("{kind}:{name}");
    match plugin_id {
        Some(id) => {
            settings.selections.insert(key, id.to_owned());
        }
        None => {
            settings.selections.remove(&key);
        }
    }
    ComponentCatalog::from_settings(&root, settings.clone())?;
    write_settings(&root, &settings)
}

fn write_settings(root: &Path, settings: &ComponentSettings) -> Result<()> {
    let temporary = root.join(format!(".config-{}.json", std::process::id()));
    let mut file = File::create(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(settings)?)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(temporary, root.join("config.json"))?;
    Ok(())
}

fn copy_package(source: &Path, target: &Path, total_bytes: &mut u64) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "component packages cannot contain symlinks"
    );
    if metadata.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            ensure!(
                entry.file_name() != ".git",
                "embedded repositories are not component packages"
            );
            copy_package(&entry.path(), &target.join(entry.file_name()), total_bytes)?;
        }
    } else {
        ensure!(
            metadata.is_file(),
            "component packages must contain regular files"
        );
        *total_bytes = total_bytes.saturating_add(metadata.len());
        ensure!(
            *total_bytes <= 1024 * 1024 * 1024,
            "component package exceeds 1 GiB"
        );
        fs::copy(source, target)?;
    }
    Ok(())
}
