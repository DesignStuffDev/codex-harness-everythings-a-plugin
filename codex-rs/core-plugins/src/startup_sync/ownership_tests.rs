use super::*;
use crate::test_support::test_http_client_factory;
use pretty_assertions::assert_eq;
use std::os::unix::fs::PermissionsExt;
use std::time::Instant;
use wiremock::MockServer;

const SHA: &str = "1111111111111111111111111111111111111111";

fn fixture_git(root: &Path, pause: &str) -> anyhow::Result<PathBuf> {
    let git = root.join("fixture-git");
    // Only the selected native command waits; no descendants are created by
    // that wait (exec preserves the child PID). This is a test recovery fence.
    let script = format!(
        r#"#!/bin/sh
set -eu
if [ "$1" = '-c' ]; then shift 2; fi
if [ "$1" = 'ls-remote' ]; then printf '{SHA}\tHEAD\n'; exit 0; fi
repo="$2"
op="$3"
if [ "$op" = '{pause}' ]; then
  printf '%s\n' "$$" > '{pid}'
  exec /bin/sleep 60
fi
case "$op" in
 init) mkdir -p "$repo/.git" ;;
 reset) mkdir -p "$repo/.agents/plugins"; printf '{{}}' > "$repo/.agents/plugins/marketplace.json" ;;
 rev-parse) printf '{SHA}\n' ;;
 fetch|clean) : ;;
 *) exit 9 ;;
esac
"#,
        pid = root.join("fixture-pid").display()
    );
    std::fs::write(&git, script)?;
    std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755))?;
    Ok(git)
}

fn wait_for_fixture_pid(path: &Path) -> anyhow::Result<u32> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(value) = std::fs::read_to_string(path)
            && value.ends_with('\n')
            && let Ok(pid) = value.trim().parse()
        {
            return Ok(pid);
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "fixture child did not reach exec wait"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn wait_for_fixture_exec(path: &Path) -> anyhow::Result<u32> {
    let pid = wait_for_fixture_pid(path)?;
    let executable = std::fs::canonicalize("/bin/sleep")?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if std::fs::read_link(format!("/proc/{pid}/exe")).is_ok_and(|path| path == executable) {
            return Ok(pid);
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "fixture child did not exec sleep"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn independently_open_lock(home: &Path) -> anyhow::Result<File> {
    Ok(File::options()
        .write(true)
        .open(home.join(super::super::CURATED_PLUGINS_SYNC_LOCK_FILE))?)
}

// There is deliberately no production clear-on-reap API. These tests know their
// exact child execs sleep with no helpers; verify the child is gone before releasing
// their isolated registry. General direct-child observation is not a recovery fence.
fn release_fixture_after_external_fence(
    registry: &AttemptRegistry,
    pid: Option<u32>,
) -> anyhow::Result<()> {
    if let Some(pid) = pid {
        anyhow::ensure!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "fixture child still exists"
        );
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                pid,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        anyhow::ensure!(
            result == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD),
            "fixture direct child not reaped"
        );
    }
    let entries = { std::mem::take(&mut registry.lock().attempts) };
    for attempt in entries {
        let resources = { attempt.lock().resources.take() };
        drop(resources);
    }
    Ok(())
}

#[tokio::test]
async fn unknown_and_unwind_retain_real_attempt_and_stop_owned_child() {
    for (unwind, dropped_result) in [(false, false), (true, false), (false, true)] {
        let fixture = tempfile::tempdir().unwrap();
        let home = fixture.path().join("home");
        let git = fixture_git(fixture.path(), "init").unwrap();
        let pid_path = fixture.path().join("fixture-pid");
        let registry = Arc::new(AttemptRegistry::default());
        let server = MockServer::start().await;
        let endpoint = server.uri();
        let registered_paths = Arc::new(Mutex::new(Vec::new()));
        let paths = Arc::clone(&registered_paths);
        let hook: CommandHook = Arc::new(move |attempt, _, context| {
            if context != "git init curated plugins repo" {
                return Ok(());
            }
            wait_for_fixture_exec(&pid_path)
                .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
            *paths.lock().unwrap_or_else(PoisonError::into_inner) = attempt
                .lock()
                .resources
                .as_ref()
                .ok_or(SyncFailure::Quarantined)?
                .directories
                .iter()
                .map(|dir| dir.path().to_path_buf())
                .collect();
            if unwind {
                panic!("fixture observer unwind after child and pipe registration");
            }
            Err(SyncFailure::Transport(CommandFailure {
                cause: CommandFailureKind::Ordinary,
                cleanup: CommandCleanup::Unknown,
                message: "fixture original unknown".to_string(),
            }))
        });
        let run_registry = Arc::clone(&registry);
        let run_home = home.clone();
        let result = tokio::task::spawn_blocking(move || {
            run_registry.run_inner(&run_home, Some(hook), |attempt| {
                let result = attempt.run(
                    Some(&git),
                    &endpoint,
                    &endpoint,
                    &test_http_client_factory(),
                );
                if dropped_result {
                    drop(result);
                    Ok("observer incorrectly reported success".to_string())
                } else {
                    result
                }
            })
        })
        .await
        .unwrap();
        let error = result.unwrap_err();
        assert!(error.retains_resources());
        assert!(!error.permits_fallback());
        assert!(matches!(
            (&error, unwind),
            (SyncFailure::Unwind, true) | (SyncFailure::Transport(_), false)
        ));
        let original = error.to_string();
        drop(error); // The public observer is not the last owner.
        let attempt = Arc::clone(&registry.lock().attempts[0]);
        let generation = attempt.generation;
        let dirs = registered_paths.lock().unwrap().clone();
        assert_eq!(
            dirs.len(),
            2,
            "trusted Git and staged checkout registered before init"
        );
        assert!(dirs.iter().all(|dir| dir.is_dir()));
        let lock = independently_open_lock(&home).unwrap();
        assert!(matches!(
            lock.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "no HTTP or export fallback"
        );
        assert!(matches!(
            registry.run(&home, |_| panic!("quarantine admitted retry")),
            Err(SyncFailure::Quarantined)
        ));
        let pid = wait_for_fixture_pid(&fixture.path().join("fixture-pid")).unwrap();
        // Production finish performed this stop/reap; test recovery has not run yet.
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
        let commands = attempt.lock().resources.as_ref().unwrap().commands.clone();
        let command = commands.last().unwrap();
        assert_eq!(
            command.last_cleanup(),
            Some(CommandCleanup::DirectChildAndPipes)
        );
        let late = command
            .wait("late observation", Duration::ZERO, &AtomicBool::new(false))
            .unwrap_err();
        assert_eq!(late.cleanup, CommandCleanup::Unknown);
        assert_eq!(registry.lock().attempts[0].generation, generation);
        assert_eq!(
            attempt
                .lock()
                .outcome
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap_err()
                .to_string(),
            original
        );
        assert!(matches!(
            lock.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        release_fixture_after_external_fence(&registry, Some(pid)).unwrap();
        lock.try_lock().unwrap();
        assert!(dirs.iter().all(|dir| !dir.exists()));
    }
}

#[tokio::test]
async fn unknown_local_head_never_uses_sha_file_or_other_transport() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("home");
    std::fs::create_dir_all(home.join(".tmp/plugins/.git")).unwrap();
    std::fs::write(home.join(".tmp/plugins.sha"), SHA).unwrap();
    let git = fixture_git(fixture.path(), "rev-parse").unwrap();
    let pid_path = fixture.path().join("fixture-pid");
    let server = MockServer::start().await;
    let endpoint = server.uri();
    let registry = Arc::new(AttemptRegistry::default());
    let hook: CommandHook = Arc::new(move |_, _, context| {
        if context != "git rev-parse curated plugins HEAD" {
            return Ok(());
        }
        wait_for_fixture_exec(&pid_path)
            .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
        Err(SyncFailure::Transport(CommandFailure {
            cause: CommandFailureKind::Ordinary,
            cleanup: CommandCleanup::Unknown,
            message: "HEAD original unknown".to_string(),
        }))
    });
    let run_registry = Arc::clone(&registry);
    let run_home = home.clone();
    let error = tokio::task::spawn_blocking(move || {
        run_registry.run_inner(&run_home, Some(hook), |attempt| {
            attempt.run(
                Some(&git),
                &endpoint,
                &endpoint,
                &test_http_client_factory(),
            )
        })
    })
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(error.to_string(), "HEAD original unknown");
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(
        std::fs::read_to_string(home.join(".tmp/plugins.sha")).unwrap(),
        SHA
    );
    assert!(matches!(
        independently_open_lock(&home).unwrap().try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    let pid = wait_for_fixture_pid(&fixture.path().join("fixture-pid")).unwrap();
    release_fixture_after_external_fence(&registry, Some(pid)).unwrap();
}

#[tokio::test]
async fn real_sha_publication_error_stops_fallback_and_retains_lock() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("home");
    std::fs::create_dir_all(home.join(".tmp/plugins.sha")).unwrap();
    let git = fixture_git(fixture.path(), "never-pause").unwrap();
    let registry = Arc::new(AttemptRegistry::default());
    let server = MockServer::start().await;
    let endpoint = server.uri();
    let run_registry = Arc::clone(&registry);
    let run_home = home.clone();
    let error = tokio::task::spawn_blocking(move || {
        run_registry.run(&run_home, |attempt| {
            attempt.run(
                Some(&git),
                &endpoint,
                &endpoint,
                &test_http_client_factory(),
            )
        })
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(matches!(error, SyncFailure::Publication(_)), "{error}");
    assert!(!error.permits_fallback());
    assert!(
        home.join(".tmp/plugins/.agents/plugins/marketplace.json")
            .is_file(),
        "activation already occurred; prior snapshot is not asserted unchanged"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(matches!(
        independently_open_lock(&home).unwrap().try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    assert!(matches!(
        registry.run(&home, |_| panic!("publication failure retried")),
        Err(SyncFailure::Quarantined)
    ));
    release_fixture_after_external_fence(&registry, None).unwrap();
}

#[tokio::test]
async fn ordinary_nonzero_git_failure_keeps_http_export_fallback_and_releases_lock() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("home");
    let git = fixture.path().join("failed-git");
    std::fs::write(&git, "#!/bin/sh\nprintf ordinary-failure >&2\nexit 7\n").unwrap();
    std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755)).unwrap();
    let registry = Arc::new(AttemptRegistry::default());
    let server = MockServer::start().await;
    let api = server.uri();
    let export = format!("{api}/export");
    let run_registry = Arc::clone(&registry);
    let run_home = home.clone();
    let error = tokio::task::spawn_blocking(move || {
        run_registry.run(&run_home, |attempt| {
            attempt.run(Some(&git), &api, &export, &test_http_client_factory())
        })
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(!error.retains_resources(), "{error}");
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url.path(), "/repos/openai/plugins");
    assert_eq!(requests[1].url.path(), "/export");
    assert!(registry.lock().attempts.is_empty());
    independently_open_lock(&home).unwrap().try_lock().unwrap();
    assert_eq!(
        registry
            .run(&home, |_| Ok("retry admitted".to_string()))
            .unwrap(),
        "retry admitted"
    );
    assert!(
        !std::fs::read_dir(home.join(".tmp"))
            .unwrap()
            .flatten()
            .any(|entry| entry.file_type().unwrap().is_dir())
    );
}

#[test]
fn stale_attempt_completion_cannot_reset_new_generation_with_real_lock() {
    let fixture = tempfile::tempdir().unwrap();
    let registry = AttemptRegistry::default();
    let first = registry.reserve(fixture.path(), None).unwrap();
    first.acquire_lock(&registry).unwrap();
    registry
        .finish(
            &first,
            Err(SyncFailure::Ordinary("drained failure".to_string())),
        )
        .unwrap_err();
    let second = registry.reserve(fixture.path(), None).unwrap();
    second.acquire_lock(&registry).unwrap();
    assert_ne!(first.generation, second.generation);
    let stale = registry
        .finish(&first, Err(SyncFailure::Unwind))
        .unwrap_err();
    assert_eq!(stale.to_string(), "drained failure");
    assert_eq!(registry.lock().attempts[0].generation, second.generation);
    assert!(!second.quarantined.load(Ordering::Acquire));
    let lock = independently_open_lock(fixture.path()).unwrap();
    assert!(matches!(
        lock.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    registry
        .finish(&second, Ok("finished".to_string()))
        .unwrap();
    lock.try_lock().unwrap();
}
