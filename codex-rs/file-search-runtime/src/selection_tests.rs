#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::future::pending;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use codex_file_search::Cli;
use codex_file_search::NativeBackendLimits;
use codex_file_search::Reporter;
use codex_file_search_api::FileMatch;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::StartCleanup;
use codex_file_search_component::ServiceLimits;
use pretty_assertions::assert_eq;
use serde_json::json;

use crate::CliSearchPolicy;
use crate::RuntimePolicy;
use crate::SelectionContext;
use crate::SelectionPolicy;
use crate::run_cli_with_context;
use crate::select_provider;

fn count(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
fn policy() -> CliSearchPolicy {
    // Small real-tree test allocation, deliberately not a CLI deployment default.
    let budget = SearchBudget {
        max_index_entries: count(32),
        max_index_bytes: count(8 * 1024 * 1024),
        max_worker_threads: count(4),
    };
    CliSearchPolicy {
        session_budget: budget,
        selection: SelectionPolicy {
            runtime: RuntimePolicy {
                provider: ProviderLimits {
                    max_scopes: count(1),
                    max_sessions: count(1),
                    resources: budget,
                },
                max_query_bytes: count(4096),
                max_roots_options_bytes: count(32 * 1024),
                max_matches: count(16),
                max_frame_retained_bytes: count(128 * 1024),
                poll_wait: Duration::from_millis(50),
            },
            native: NativeBackendLimits {
                max_sessions: count(1),
                resources: budget,
                max_query_bytes: count(4096),
                max_roots_options_bytes: count(32 * 1024),
                max_matches: count(16),
                max_snapshot_bytes: count(128 * 1024),
                max_poll_wait: Duration::from_millis(50),
            },
            process: ServiceLimits::new(budget),
        },
    }
}
fn context(home: &Path) -> SelectionContext {
    SelectionContext {
        codex_home: home.to_owned(),
        base_dir: std::env::current_dir().unwrap(),
    }
}
fn cli(root: &Path, pattern: Option<&str>) -> Cli {
    Cli {
        json: false,
        limit: count(1),
        cwd: Some(root.to_owned()),
        compute_indices: true,
        threads: count(1),
        exclude: vec!["ignored*".into()],
        pattern: pattern.map(str::to_owned),
    }
}
#[derive(Default)]
struct Output {
    matches: Vec<FileMatch>,
    truncated: Option<(usize, usize)>,
    listing: Vec<PathBuf>,
}
#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Output>>);
impl Reporter for Recorder {
    fn report_match(&self, matched: &FileMatch) {
        self.0.lock().unwrap().matches.push(matched.clone());
    }
    fn warn_matches_truncated(&self, total: usize, shown: usize) {
        self.0.lock().unwrap().truncated = Some((total, shown));
    }
    fn warn_no_search_pattern(&self, root: &Path) {
        self.0.lock().unwrap().listing.push(root.to_owned());
    }
}

#[tokio::test]
async fn selected_native_cli_preserves_matches_indices_exclusion_and_truncation() {
    let home = tempfile::tempdir().unwrap();
    let tree = tempfile::tempdir().unwrap();
    for name in ["match-a.rs", "match-b.rs", "ignored-match.rs", "other.txt"] {
        std::fs::write(tree.path().join(name), "fixture").unwrap();
    }
    let expected = codex_file_search::run(
        "ma",
        vec![tree.path().to_owned()],
        FileSearchOptions {
            limit: count(1),
            threads: count(1),
            exclude: vec!["ignored*".into()],
            compute_indices: true,
            respect_gitignore: true,
        },
        None,
    )
    .unwrap();
    let reporter = Recorder::default();
    tokio::time::timeout(
        Duration::from_secs(10),
        run_cli_with_context(
            cli(tree.path(), Some("ma")),
            reporter.clone(),
            context(home.path()),
            policy(),
            pending(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let output = reporter.0.lock().unwrap();
    assert_eq!(output.matches, expected.matches);
    assert_eq!(
        output.truncated,
        Some((expected.total_match_count, expected.matches.len()))
    );
    assert_eq!(output.listing, Vec::<PathBuf>::new());
}

#[tokio::test]
async fn no_pattern_does_not_validate_context_policy_or_selected_catalog() {
    let tree = tempfile::tempdir().unwrap();
    let mut invalid_policy = policy();
    invalid_policy.selection.runtime.poll_wait = Duration::ZERO;
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join("components")).unwrap();
    std::fs::write(
        home.path().join("components/config.json"),
        "{ malformed configuration",
    )
    .unwrap();
    let context = SelectionContext {
        codex_home: home.path().to_owned(),
        base_dir: "relative-invalid-base".into(),
    };
    let reporter = Recorder::default();
    run_cli_with_context(
        cli(tree.path(), None),
        reporter.clone(),
        context,
        invalid_policy,
        pending(),
    )
    .await
    .unwrap();
    let output = reporter.0.lock().unwrap();
    assert_eq!(output.listing, vec![tree.path().to_owned()]);
    assert!(output.matches.is_empty());
    assert_eq!(output.truncated, None);
}

#[tokio::test]
async fn an_invalid_explicit_selection_cannot_fall_back_to_native() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join("components")).unwrap();
    std::fs::write(
        home.path().join("components/config.json"),
        serde_json::to_vec(&json!({
            "selections": {"file_search:default": "missing.provider"}
        }))
        .unwrap(),
    )
    .unwrap();
    let error = select_provider(context(home.path()), policy().selection)
        .await
        .err()
        .expect("invalid selection must fail");
    assert_eq!(error.operation.kind(), SearchErrorKind::InvalidInput);
    assert_eq!(error.cleanup, StartCleanup::NotAdmitted);
}

#[cfg(unix)]
#[tokio::test]
async fn a_selected_executable_failure_cannot_fall_back_to_native() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let package = home.path().join("components/packages/broken.search");
    std::fs::create_dir_all(&package).unwrap();
    let executable = package.join("worker");
    std::fs::write(&executable, "#!/bin/sh\nexit 19\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(
        package.join("codex-component.json"),
        serde_json::to_vec(&json!({
            "api_version": 1, "id":"broken.search", "version":"1.0.0", "entrypoint":"worker",
            "components":[{"kind":"file_search","name":"default","contract_version":1}]
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        home.path().join("components/config.json"),
        serde_json::to_vec(&json!({
            "enabled":["broken.search"], "selections":{"file_search:default":"broken.search"}
        }))
        .unwrap(),
    )
    .unwrap();
    let error = tokio::time::timeout(
        Duration::from_secs(10),
        select_provider(context(home.path()), policy().selection),
    )
    .await
    .unwrap()
    .err()
    .expect("selected executable failure must fail");
    assert_eq!(error.operation.kind(), SearchErrorKind::TransportLost);
    assert!(matches!(error.cleanup, StartCleanup::Unconfirmed(_)));
}

#[tokio::test]
async fn cli_interruption_observes_cleanup_and_emits_no_matches() {
    let home = tempfile::tempdir().unwrap();
    let tree = tempfile::tempdir().unwrap();
    std::fs::write(tree.path().join("match.rs"), "fixture").unwrap();
    let reporter = Recorder::default();
    let error = tokio::time::timeout(
        Duration::from_secs(10),
        run_cli_with_context(
            cli(tree.path(), Some("ma")),
            reporter.clone(),
            context(home.path()),
            policy(),
            async { Ok(()) },
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.to_string().contains("interrupted"));
    assert!(!error.to_string().contains("cleanup unconfirmed"));
    assert!(reporter.0.lock().unwrap().matches.is_empty());
}

#[test]
fn process_negotiation_reduces_shared_units_and_preserves_distinct_byte_caps() {
    let mut configured = policy().selection;
    configured.runtime.provider.max_sessions = count(8);
    configured.runtime.max_query_bytes = count(5000);
    configured.runtime.max_matches = count(20);
    let mut negotiated = configured.process.clone();
    negotiated.max_leases = 2;
    negotiated.max_query_utf8_bytes = 3000;
    negotiated.max_matches = 9;
    negotiated.max_poll_wait_ms = 10;
    negotiated.max_roots_options_bytes = 700;
    negotiated.max_frame_bytes = 900;
    let effective = configured.for_process(&negotiated).unwrap();
    assert_eq!(effective.provider.max_sessions.get(), 2);
    assert_eq!(effective.max_query_bytes.get(), 3000);
    assert_eq!(effective.max_matches.get(), 9);
    assert_eq!(effective.poll_wait, Duration::from_millis(10));
    assert_eq!(
        effective.max_roots_options_bytes,
        configured.runtime.max_roots_options_bytes
    );
    assert_eq!(
        effective.max_frame_retained_bytes,
        configured.runtime.max_frame_retained_bytes
    );
}

#[test]
fn standalone_threads_account_for_all_workers_and_reject_overflow() {
    let default = crate::standalone_policy(count(2)).unwrap();
    assert_eq!(default.session_budget.max_worker_threads.get(), 6);
    assert_eq!(
        default
            .selection
            .runtime
            .provider
            .resources
            .max_worker_threads
            .get(),
        64
    );
    assert_eq!(
        crate::standalone_policy(count(31))
            .unwrap()
            .session_budget
            .max_worker_threads
            .get(),
        64
    );
    assert_eq!(
        crate::standalone_policy(count(32)).unwrap_err().kind(),
        SearchErrorKind::ResourceExhausted
    );
    assert_eq!(
        crate::standalone_policy(count(usize::MAX))
            .unwrap_err()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
}
