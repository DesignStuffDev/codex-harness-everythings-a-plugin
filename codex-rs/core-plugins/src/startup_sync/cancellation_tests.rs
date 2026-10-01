use super::tests::fixture_git;
use super::tests::independently_open_lock;
use super::tests::wait_for_fixture_exec;
use super::*;
use crate::test_support::test_http_client_factory;
use pretty_assertions::assert_eq;
use std::os::unix::fs::MetadataExt;
use std::sync::mpsc;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::Request;
use wiremock::ResponseTemplate;
use wiremock::matchers::path;

#[test]
fn stopped_attempt_is_rejected_before_creating_home_or_resources() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("unused-home");
    let registry = AttemptRegistry::default();
    let control = Arc::new(SyncControl::default());
    control.request_stop(Instant::now());
    let error = registry
        .run_controlled(&home, control, |_| panic!("stopped work admitted"))
        .unwrap_err();
    assert!(matches!(
        error,
        SyncFailure::Stopped {
            publication_may_have_occurred: false
        }
    ));
    assert!(!error.retains_resources());
    assert!(!error.permits_fallback());
    assert!(registry.lock().attempts.is_empty());
    assert!(!home.exists());
}

#[tokio::test]
async fn stop_wakes_real_lock_wait_without_git_fallback_or_lock_replacement() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("home");
    std::fs::create_dir_all(home.join(".tmp")).unwrap();
    let lock_path = home.join(super::super::CURATED_PLUGINS_SYNC_LOCK_FILE);
    let held = File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .unwrap();
    held.lock().unwrap();
    let inode = held.metadata().unwrap().ino();
    let registry = Arc::new(AttemptRegistry::default());
    let control = Arc::new(SyncControl::default());
    let server = MockServer::start().await;
    let endpoint = server.uri();
    let git = fixture_git(fixture.path(), "never-pause").unwrap();
    let run_registry = Arc::clone(&registry);
    let run_control = Arc::clone(&control);
    let run_home = home.clone();
    let (waiting_tx, waiting_rx) = mpsc::channel();
    let task = tokio::task::spawn_blocking(move || {
        let attempt = run_registry.reserve_controlled(&run_home, run_control, None)?;
        *attempt
            .lock_waiting
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(waiting_tx);
        let result = attempt.acquire_lock(&run_registry).and_then(|()| {
            attempt.run(
                Some(&git),
                &endpoint,
                &endpoint,
                &test_http_client_factory(),
            )
        });
        run_registry.finish(&attempt, result)
    });
    let waiting = waiting_rx.recv_timeout(Duration::from_secs(3));
    control.request_stop(Instant::now() + Duration::from_secs(1));
    let error = task.await.unwrap().unwrap_err();
    assert!(
        waiting.is_ok(),
        "worker never observed WouldBlock on the real held lock"
    );
    assert!(matches!(
        error,
        SyncFailure::Stopped {
            publication_may_have_occurred: false
        }
    ));
    assert!(registry.lock().attempts.is_empty());
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(std::fs::metadata(&lock_path).unwrap().ino(), inode);
    assert!(matches!(
        independently_open_lock(&home).unwrap().try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    drop(held);
    independently_open_lock(&home).unwrap().try_lock().unwrap();
    assert!(!home.join(".tmp/plugins").exists());
}

#[tokio::test]
async fn same_owner_control_stops_registered_git_and_prevents_fallback() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("home");
    let git = fixture_git(fixture.path(), "init").unwrap();
    let pid_path = fixture.path().join("fixture-pid");
    let registry = Arc::new(AttemptRegistry::default());
    let control = Arc::new(SyncControl::default());
    let hook_control = Arc::clone(&control);
    let (ready_tx, ready_rx) = mpsc::channel();
    let hook: CommandHook = Arc::new(move |attempt, _, context| {
        if context == "git init curated plugins repo" {
            if !Arc::ptr_eq(&attempt.control, &hook_control) {
                return Err(SyncFailure::Unwind);
            }
            let pid = wait_for_fixture_exec(&pid_path)
                .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
            let _ = ready_tx.send(pid);
        }
        Ok(())
    });
    let server = MockServer::start().await;
    let endpoint = server.uri();
    let run_registry = Arc::clone(&registry);
    let run_control = Arc::clone(&control);
    let run_home = home.clone();
    let task = tokio::task::spawn_blocking(move || {
        run_registry.run_inner_controlled(&run_home, run_control, Some(hook), |attempt| {
            attempt.run(
                Some(&git),
                &endpoint,
                &endpoint,
                &test_http_client_factory(),
            )
        })
    });
    let pid = ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    control.request_stop(Instant::now() + Duration::from_secs(3));
    let error = task.await.unwrap().unwrap_err();
    assert!(
        matches!(&error, SyncFailure::Transport(failure) if failure.cause == CommandFailureKind::Cancelled && failure.cleanup == CommandCleanup::DirectChildAndPipes)
    );
    assert!(!error.retains_resources());
    assert!(!error.permits_fallback());
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    assert!(registry.lock().attempts.is_empty());
    independently_open_lock(&home).unwrap().try_lock().unwrap();
    assert!(!home.join(".tmp/plugins").exists());
}

#[tokio::test]
async fn stop_after_http_failure_prevents_export_admission() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("home");
    let control = Arc::new(SyncControl::default());
    let response_control = Arc::clone(&control);
    let server = MockServer::start().await;
    Mock::given(path("/repos/openai/plugins"))
        .respond_with(move |_: &Request| {
            response_control.request_stop(Instant::now() + Duration::from_secs(1));
            ResponseTemplate::new(503)
        })
        .expect(1)
        .mount(&server)
        .await;
    let endpoint = server.uri();
    let registry = Arc::new(AttemptRegistry::default());
    let run_registry = Arc::clone(&registry);
    let run_home = home.clone();
    let error = tokio::task::spawn_blocking(move || {
        run_registry.run_controlled(&run_home, control, |attempt| {
            attempt.run(
                None,
                &endpoint,
                &format!("{endpoint}/export"),
                &test_http_client_factory(),
            )
        })
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(matches!(
        error,
        SyncFailure::Stopped {
            publication_may_have_occurred: false
        }
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(registry.lock().attempts.is_empty());
    independently_open_lock(&home).unwrap().try_lock().unwrap();
}

#[tokio::test]
async fn stop_inside_admitted_publication_finishes_sha_pair_then_blocks_next_stage() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().join("home");
    let git = fixture_git(fixture.path(), "never-pause").unwrap();
    let control = Arc::new(SyncControl::default());
    let publication_control = Arc::clone(&control);
    let registry = Arc::new(AttemptRegistry::default());
    let server = MockServer::start().await;
    let endpoint = server.uri();
    let run_registry = Arc::clone(&registry);
    let run_home = home.clone();
    let error = tokio::task::spawn_blocking(move || {
        run_registry.run_controlled(&run_home, control, |attempt| {
            *attempt
                .after_activation
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(Box::new(move || {
                publication_control.request_stop(Instant::now())
            }));
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
    assert!(matches!(
        error,
        SyncFailure::Stopped {
            publication_may_have_occurred: true
        }
    ));
    assert!(
        home.join(".tmp/plugins/.agents/plugins/marketplace.json")
            .is_file()
    );
    assert_eq!(
        std::fs::read_to_string(home.join(".tmp/plugins.sha"))
            .unwrap()
            .trim(),
        "1111111111111111111111111111111111111111"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(
        registry.lock().attempts.is_empty(),
        "completed publication pair is not uncertainty"
    );
    independently_open_lock(&home).unwrap().try_lock().unwrap();
}
