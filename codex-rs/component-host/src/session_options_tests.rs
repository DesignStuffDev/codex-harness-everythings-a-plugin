use std::num::NonZeroU64;
use std::path::PathBuf;

use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;

use super::SERVICE;
use crate::ComponentBinding;
use crate::ComponentCatalog;
use crate::ComponentSessionOptions;
use crate::SessionPayloadLimits;
use crate::SessionWorkingDirectory;

struct PackageFixture {
    root: TempDir,
    binding: ComponentBinding,
}

impl PackageFixture {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("component launch with spaces ")
            .tempdir()
            .unwrap();
        let package = root.path().join("package source");
        let sources = root.path().join("zip app sources");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::create_dir_all(&sources).unwrap();
        let source = SERVICE.replace(
            "    if method == 'wait':",
            r#"
    if method == 'location':
        executable = pathlib.Path(sys.argv[0]).resolve()
        result(request, {
            'cwd': str(pathlib.Path.cwd()),
            'entrypoint': str(executable),
            'asset': executable.with_name('package asset.txt').read_text(),
            'relative': pathlib.Path(params['relative']).read_text(),
        })
        return
    if method == 'wait':"#,
        );
        std::fs::write(sources.join("__main__.py"), source).unwrap();
        let python = ["python3", "python"]
            .into_iter()
            .find(|program| {
                std::process::Command::new(program)
                    .arg("--version")
                    .output()
                    .is_ok_and(|output| output.status.success())
            })
            .expect("persistent launch tests require Python 3");
        let output = std::process::Command::new(python)
            .args(["-m", "zipapp"])
            .arg(&sources)
            .arg("-o")
            .arg(package.join("plugin.pyz"))
            .arg("-p")
            .arg(format!("/usr/bin/env {python}"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "zipapp fixture creation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::write(package.join("package asset.txt"), "from installed package").unwrap();
        std::fs::write(package.join("relative.txt"), "package directory").unwrap();
        std::fs::write(package.join("codex-component.json"), serde_json::to_vec(&json!({
            "api_version":1, "id":"launch.fixture", "version":"1.0.0", "entrypoint":"plugin.pyz",
            "components":[{"kind":"tool", "name":"launch", "contract_version":1}],
        })).unwrap()).unwrap();
        let home = root.path().join("component home with spaces");
        crate::install(&home, &package).unwrap();
        let mut binding = ComponentCatalog::load(&home)
            .unwrap()
            .components("tool")
            .pop()
            .unwrap();
        binding.timeout_ms = 15_000;
        assert!(binding.entrypoint.is_absolute());
        assert!(binding.entrypoint.starts_with(&binding.package_dir));
        Self { root, binding }
    }

    fn workspace(&self, name: &str) -> PathBuf {
        let path = self.root.path().join(name);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("relative.txt"), name).unwrap();
        // A wrong cwd-based asset lookup would return this instead of the
        // immutable installed package's asset.
        std::fs::write(path.join("package asset.txt"), "wrong workspace asset").unwrap();
        path.canonicalize().unwrap()
    }
}

#[tokio::test]
async fn existing_connect_wrappers_keep_installed_package_directory() {
    let fixture = PackageFixture::new();
    for mode in ["default", "limits"] {
        let session = if mode == "default" {
            fixture.binding.connect().await.unwrap()
        } else {
            fixture
                .binding
                .connect_with_limits(SessionPayloadLimits {
                    request_bytes: NonZeroU64::new(1024),
                    reply_bytes: NonZeroU64::new(64 * 1024),
                })
                .await
                .unwrap()
        };
        assert_eq!(
            session
                .call("location", json!({"relative":"relative.txt"}))
                .await
                .unwrap(),
            json!({
                "cwd":fixture.binding.package_dir, "entrypoint":fixture.binding.entrypoint,
                "asset":"from installed package", "relative":"package directory",
            })
        );
        session.close().await.unwrap();
    }
}

#[tokio::test]
async fn simultaneous_explicit_directories_keep_host_and_package_identity_unchanged() {
    let fixture = PackageFixture::new();
    let host_cwd = std::env::current_dir().unwrap();
    let first_cwd = fixture.workspace("first search workspace");
    let second_cwd = fixture.workspace("second search workspace");
    let options = |path| ComponentSessionOptions {
        payload_limits: SessionPayloadLimits {
            request_bytes: NonZeroU64::new(1024),
            reply_bytes: NonZeroU64::new(64 * 1024),
        },
        working_directory: SessionWorkingDirectory::ExplicitAbsolute(path),
    };
    let (first, second) = tokio::join!(
        fixture
            .binding
            .connect_with_options(options(first_cwd.clone())),
        fixture
            .binding
            .connect_with_options(options(second_cwd.clone())),
    );
    let first = first.unwrap();
    let second = second.unwrap();
    for (session, cwd, contents) in [
        (&first, first_cwd, "first search workspace"),
        (&second, second_cwd, "second search workspace"),
    ] {
        assert_eq!(
            session
                .call("location", json!({"relative":"relative.txt"}))
                .await
                .unwrap(),
            json!({
                "cwd":cwd, "entrypoint":fixture.binding.entrypoint,
                "asset":"from installed package", "relative":contents,
            })
        );
    }
    let (first_closed, second_closed) = tokio::join!(first.close(), second.close());
    first_closed.unwrap();
    second_closed.unwrap();
    assert_eq!(std::env::current_dir().unwrap(), host_cwd);
}

#[tokio::test]
async fn invalid_explicit_directories_and_relative_entrypoint_fail_before_launch() {
    let fixture = PackageFixture::new();
    for path in [
        PathBuf::from("relative cwd"),
        fixture.root.path().join("missing directory"),
        fixture.binding.package_dir.join("package asset.txt"),
    ] {
        assert!(
            fixture
                .binding
                .connect_with_options(ComponentSessionOptions {
                    working_directory: SessionWorkingDirectory::ExplicitAbsolute(path),
                    ..ComponentSessionOptions::default()
                })
                .await
                .is_err()
        );
        assert!(!fixture.binding.state_dir.exists());
    }
    let mut binding = fixture.binding.clone();
    binding.entrypoint = PathBuf::from("plugin.pyz");
    let error = binding
        .connect_with_options(ComponentSessionOptions {
            working_directory: SessionWorkingDirectory::ExplicitAbsolute(
                fixture.workspace("valid directory"),
            ),
            ..ComponentSessionOptions::default()
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("absolute entrypoint"));
    assert!(!fixture.binding.state_dir.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn rejected_handshake_with_explicit_directory_reaps_before_return() {
    let mut fixture = PackageFixture::new();
    fixture.binding.config = json!({"old_protocol":true});
    let error = fixture
        .binding
        .connect_with_options(ComponentSessionOptions {
            working_directory: SessionWorkingDirectory::ExplicitAbsolute(
                fixture.workspace("rejected startup workspace"),
            ),
            ..ComponentSessionOptions::default()
        })
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("does not support multiplexed"));
    let pid: libc::pid_t = std::fs::read_to_string(fixture.binding.state_dir.join("pid"))
        .unwrap()
        .parse()
        .unwrap();
    // SAFETY: signal zero probes the exact child without sending a signal.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}
