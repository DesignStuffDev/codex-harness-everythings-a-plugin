//! Actual native traversal/matching through the generic process adapter. This
//! test builds a worker in the workspace; independent packaging is a later gate.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::sync::Mutex;
use std::time::Duration;

use codex_component_host::ComponentBinding;
use codex_component_host::ComponentCatalog;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::FileSearchSnapshot;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBackendSession;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchPhase;
use codex_file_search_api::SearchPoll;
use codex_file_search_api::SearchQuery;
use codex_file_search_component::InitializeRequest;
use codex_file_search_component::ProcessSearchBackend;
use codex_file_search_component::ProviderIdentity;
use codex_file_search_component::ServiceLimits;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    base: PathBuf,
    binding: ComponentBinding,
    home: PathBuf,
    #[cfg(target_os = "linux")]
    started_pids: Mutex<Vec<libc::pid_t>>,
}

impl Fixture {
    fn new(config: Value) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().join("project");
        std::fs::create_dir_all(base.join("sub")).unwrap();
        std::fs::create_dir(base.join(".git")).unwrap();
        for file in [
            "alpha.txt",
            "naïve.txt",
            "sub/alpha_child.txt",
            "sub/beta.txt",
            "ignored_alpha.txt",
        ] {
            std::fs::write(base.join(file), "native process fixture").unwrap();
        }
        std::fs::write(base.join(".gitignore"), "ignored_alpha.txt\n").unwrap();
        let package = temp.path().join("package");
        std::fs::create_dir(&package).unwrap();
        let executable = if cfg!(windows) {
            "search-worker.exe"
        } else {
            "search-worker"
        };
        std::fs::copy(
            codex_utils_cargo_bin::cargo_bin("codex-file-search-local-plugin").unwrap(),
            package.join(executable),
        )
        .unwrap();
        std::fs::write(package.join("codex-component.json"), serde_json::to_vec(&json!({
            "api_version":1,"id":"native.file-search-local","version":"0.1.0",
            "entrypoint":executable,"components":[{"kind":"file_search","name":"default","contract_version":1}]
        })).unwrap()).unwrap();
        let home = temp.path().join("home");
        assert_eq!(
            codex_component_host::install(&home, &package).unwrap(),
            "native.file-search-local"
        );
        let mut settings = ComponentCatalog::load(&home).unwrap().settings().clone();
        settings.config.insert(
            "native.file-search-local".to_owned(),
            if config.is_null() { json!({}) } else { config },
        );
        std::fs::write(
            home.join("components/config.json"),
            serde_json::to_vec(&settings).unwrap(),
        )
        .unwrap();
        codex_component_host::select(
            &home,
            "file_search",
            "default",
            Some("native.file-search-local"),
        )
        .unwrap();
        let mut binding = ComponentCatalog::load(&home)
            .unwrap()
            .selected("file_search", "default")
            .unwrap();
        binding.timeout_ms = 15_000;
        assert_ne!(binding.package_dir, package);
        // Installation must retain the executable independently of its input.
        // This removes only this test's temporary source package.
        std::fs::remove_dir_all(package).unwrap();
        Self {
            _temp: temp,
            base,
            binding,
            home,
            #[cfg(target_os = "linux")]
            started_pids: Mutex::new(Vec::new()),
        }
    }

    async fn connect(&self) -> Arc<ProcessSearchBackend> {
        let backend = ProcessSearchBackend::connect(
            self.binding.clone(),
            InitializeRequest {
                contract_version: 1,
                identity: ProviderIdentity {
                    provider_id: uuid::Uuid::new_v4().to_string(),
                },
                base_dir: self.base.clone(),
                requested_limits: ServiceLimits::new(SearchBudget {
                    max_index_entries: nz(8192),
                    max_index_bytes: nz(64 * 1024 * 1024),
                    max_worker_threads: nz(52),
                }),
            },
        )
        .await
        .unwrap();
        #[cfg(target_os = "linux")]
        {
            let pids = self.worker_pids();
            assert_eq!(
                pids.len(),
                1,
                "exact installed worker must be live after initialization"
            );
            self.started_pids.lock().unwrap().extend(pids);
        }
        backend
    }

    fn request(&self, roots: Vec<PathBuf>) -> SearchOpen {
        SearchOpen {
            roots,
            options: FileSearchOptions {
                limit: nz(20),
                exclude: Vec::new(),
                threads: nz(2),
                compute_indices: true,
                respect_gitignore: true,
            },
            // Test allocation, not a production default. Native admission must
            // independently prove its fixed storage fits this explicit budget.
            budget: SearchBudget {
                max_index_entries: nz(4096),
                max_index_bytes: nz(32 * 1024 * 1024),
                max_worker_threads: nz(26),
            },
        }
    }

    #[cfg(target_os = "linux")]
    fn worker_pids(&self) -> Vec<libc::pid_t> {
        std::fs::read_dir("/proc")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let pid = entry.file_name().to_str()?.parse().ok()?;
                let executable = std::fs::read_link(entry.path().join("exe")).ok()?;
                (executable == self.binding.entrypoint).then_some(pid)
            })
            .collect()
    }

    fn assert_reaped(&self) {
        #[cfg(target_os = "linux")]
        {
            assert!(
                self.worker_pids().is_empty(),
                "installed worker must be absent after shutdown"
            );
            for pid in self.started_pids.lock().unwrap().iter().copied() {
                // SAFETY: signal zero probes only a PID observed running this
                // fixture's unique installed executable before shutdown.
                assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::ESRCH),
                    "worker PID {pid} must be absent, including zombies"
                );
            }
        }
    }
}

fn nz(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
fn joined() -> SearchCloseOutcome {
    SearchCloseOutcome {
        operation: Ok(()),
        cleanup: CloseCleanup::Joined,
    }
}

async fn settled(session: &dyn SearchBackendSession, id: u64, text: &str) -> FileSearchSnapshot {
    let identity = NonZeroU64::new(id).unwrap();
    assert_eq!(
        session
            .update_query(SearchQuery {
                id: identity,
                text: text.to_owned()
            })
            .await
            .unwrap()
            .id,
        identity
    );
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut revision = 0;
        loop {
            match session
                .next_snapshot(revision, Duration::from_millis(100))
                .await
            {
                Ok(SearchPoll::Changed(frame)) => {
                    revision = frame.revision;
                    assert_eq!((frame.query_id, frame.query.as_str()), (id, text));
                    match frame.phase {
                        SearchPhase::Idle => {
                            return frame.snapshot.expect("idle requires a current snapshot");
                        }
                        SearchPhase::Running => {}
                        phase => panic!("native query did not settle successfully: {phase:?}"),
                    }
                }
                Ok(SearchPoll::Unchanged { .. }) => {}
                Err(error) if error.kind() == SearchErrorKind::StaleEpoch => {
                    tokio::task::yield_now().await
                }
                Err(error) => panic!("native poll failed: {error}"),
            }
        }
    })
    .await
    .expect("native search must settle")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn process_results_match_existing_native_scoring_ignore_and_highlights() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let request = fixture.request(vec![fixture.base.clone()]);
    let expected = codex_file_search::run(
        "alpha",
        request.roots.clone(),
        request.options.clone(),
        /*cancel_flag*/ None,
    )
    .unwrap();
    let session = backend.open(request).await.unwrap();
    let actual = settled(session.as_ref(), 1, "alpha").await;
    assert_eq!(
        (actual.matches, actual.total_match_count),
        (expected.matches, expected.total_match_count)
    );
    assert!(actual.walk_complete);
    assert_eq!(session.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn separate_provider_cwds_preserve_relative_roots_without_changing_host_cwd() {
    let original = std::env::current_dir().unwrap();
    let first = Fixture::new(Value::Null);
    let second = Fixture::new(json!({"native_snapshot_bytes": 2 * 1024 * 1024}));
    std::fs::write(
        second.base.join("only_second.txt"),
        "isolated second provider",
    )
    .unwrap();
    let (first_backend, second_backend) = tokio::join!(first.connect(), second.connect());
    let first_session = first_backend
        .open(first.request(vec![PathBuf::from(".")]))
        .await
        .unwrap();
    let second_session = second_backend
        .open(second.request(vec![PathBuf::from(".")]))
        .await
        .unwrap();
    let first_result = settled(first_session.as_ref(), 1, "only_second").await;
    let second_result = settled(second_session.as_ref(), 1, "only_second").await;
    assert!(first_result.matches.is_empty());
    assert_eq!(second_result.matches.len(), 1);
    assert_eq!(
        (
            second_result.matches[0].root.as_path(),
            second_result.matches[0].path.as_path()
        ),
        (Path::new("."), Path::new("only_second.txt"))
    );
    assert_eq!(std::env::current_dir().unwrap(), original);
    assert_eq!(first_session.close().await, joined());
    assert_eq!(first_backend.shutdown().await, joined());
    let still_live = settled(second_session.as_ref(), 2, "alpha").await;
    assert!(!still_live.matches.is_empty());
    assert_eq!(second_session.close().await, joined());
    assert_eq!(second_backend.shutdown().await, joined());
    assert_eq!(std::env::current_dir().unwrap(), original);
    first.assert_reaped();
    second.assert_reaped();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_session_close_does_not_cancel_siblings_and_joins_before_quota_reuse() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let first = backend
        .open(fixture.request(vec![fixture.base.clone()]))
        .await
        .unwrap();
    let second = backend
        .open(fixture.request(vec![fixture.base.clone()]))
        .await
        .unwrap();
    assert_eq!(first.close().await, joined());
    let replacement = backend
        .open(fixture.request(vec![fixture.base.clone()]))
        .await
        .unwrap();
    assert!(
        !settled(second.as_ref(), 1, "alpha")
            .await
            .matches
            .is_empty()
    );
    assert!(
        !settled(replacement.as_ref(), 1, "beta")
            .await
            .matches
            .is_empty()
    );
    assert_eq!(replacement.close().await, joined());
    assert_eq!(second.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    fixture.assert_reaped();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_plugin_configuration_fails_without_native_fallback() {
    let fixture = Fixture::new(json!({"unknown-option": "private-fixture-value"}));
    let result = ProcessSearchBackend::connect(
        fixture.binding.clone(),
        InitializeRequest {
            contract_version: 1,
            identity: ProviderIdentity {
                provider_id: uuid::Uuid::new_v4().to_string(),
            },
            base_dir: fixture.base.clone(),
            requested_limits: codex_file_search_local_plugin::service_ceilings().unwrap(),
        },
    )
    .await;
    let error = result
        .err()
        .expect("unsupported config must fail selected initialization");
    assert_eq!(error.operation.kind(), SearchErrorKind::InvalidInput);
    assert_eq!(
        error.cleanup,
        codex_file_search_api::StartCleanup::Confirmed
    );
    assert!(!error.operation.message().contains("private-fixture-value"));
    fixture.assert_reaped();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exhausted_native_index_is_failure_with_joined_cleanup_not_a_partial_idle() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let mut request = fixture.request(vec![fixture.base.clone()]);
    request.budget.max_index_entries = nz(1);
    match backend.open(request).await {
        Err(error) => {
            assert_eq!(error.operation.kind(), SearchErrorKind::ResourceExhausted);
            assert_eq!(
                error.cleanup,
                codex_file_search_api::StartCleanup::Confirmed
            );
        }
        Ok(session) => {
            let update = session
                .update_query(SearchQuery {
                    id: NonZeroU64::new(1).unwrap(),
                    text: "alpha".to_owned(),
                })
                .await;
            if update.is_ok() {
                tokio::time::timeout(Duration::from_secs(10), async {
                    let mut cursor = 0;
                    loop {
                        match session
                            .next_snapshot(cursor, Duration::from_millis(100))
                            .await
                        {
                            Ok(SearchPoll::Changed(frame)) => {
                                cursor = frame.revision;
                                match frame.phase {
                                    SearchPhase::Failed(error) => {
                                        assert_eq!(
                                            error.kind(),
                                            SearchErrorKind::ResourceExhausted
                                        );
                                        break;
                                    }
                                    SearchPhase::Running => {}
                                    phase => panic!(
                                        "exhausted index must not appear complete: {phase:?}"
                                    ),
                                }
                            }
                            Ok(SearchPoll::Unchanged { .. }) => {}
                            Err(error) if error.kind() == SearchErrorKind::StaleEpoch => {
                                tokio::task::yield_now().await
                            }
                            Err(_) => break,
                        }
                    }
                })
                .await
                .expect("bounded native failure must become observable");
            }
            let outcome = session.close().await;
            assert_eq!(
                outcome.operation.unwrap_err().kind(),
                SearchErrorKind::ResourceExhausted
            );
            assert_eq!(outcome.cleanup, CloseCleanup::Joined);
        }
    }
    let outcome = backend.shutdown().await;
    assert_eq!(
        outcome.operation.unwrap_err().kind(),
        SearchErrorKind::ResourceExhausted
    );
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    fixture.assert_reaped();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn removal_affects_future_selection_while_existing_provider_remains_owned() {
    let fixture = Fixture::new(json!({}));
    let backend = fixture.connect().await;
    let session = backend
        .open(fixture.request(vec![fixture.base.clone()]))
        .await
        .unwrap();
    codex_component_host::remove(&fixture.home, "native.file-search-local").unwrap();
    assert!(
        ComponentCatalog::load(&fixture.home)
            .unwrap()
            .selected("file_search", "default")
            .is_none()
    );
    assert!(
        !settled(session.as_ref(), 1, "alpha")
            .await
            .matches
            .is_empty()
    );
    assert_eq!(session.close().await, joined());
    assert_eq!(backend.shutdown().await, joined());
    assert!(fixture.binding.entrypoint.is_file());
    fixture.assert_reaped();
}
