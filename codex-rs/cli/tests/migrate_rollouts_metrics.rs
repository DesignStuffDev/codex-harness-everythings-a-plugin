//! Real CLI exports must preserve native migration counters across component selection.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
#[cfg(unix)]
use std::process::Output;

use anyhow::Context;
use anyhow::Result;
use pretty_assertions::assert_eq;
use serde_json::Value;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;

// Shared CLI fixtures also contain helpers used only by the lifecycle target.
#[allow(dead_code)]
#[path = "migrate_rollouts/support.rs"]
mod support;

use support::*;

const RUN: &str = "codex.rollout_migration.run";
const THREAD: &str = "codex.rollout_migration.thread";
type Tags = BTreeMap<String, String>;
type Counters = BTreeMap<(String, Tags), u64>;

#[derive(Clone, Copy, Debug)]
enum Case {
    DryRun,
    FilteredApply,
    PartialFailure,
    Busy,
}

impl Case {
    fn args(self) -> Vec<&'static str> {
        match self {
            Self::DryRun | Self::PartialFailure => vec![],
            Self::FilteredApply => vec!["--apply", "--thread", FIRST_ID],
            Self::Busy => vec!["--apply"],
        }
    }

    fn expected(self) -> Counters {
        match self {
            Self::DryRun => expected("dry_run", "all", "success", &[("eligible", 2, None)]),
            Self::FilteredApply => {
                expected("apply", "selected", "success", &[("migrated", 1, None)])
            }
            Self::PartialFailure => expected(
                "dry_run",
                "all",
                "partial_failure",
                &[
                    ("eligible", 2, None),
                    ("failed", 1, Some("invalid_session_metadata")),
                ],
            ),
            Self::Busy => expected("apply", "all", "error", &[]),
        }
    }
}

fn expected(
    mode: &str,
    scope: &str,
    result: &str,
    outcomes: &[(&str, u64, Option<&str>)],
) -> Counters {
    let common = Tags::from([
        ("trigger".to_owned(), "manual".to_owned()),
        ("mode".to_owned(), mode.to_owned()),
        ("scope".to_owned(), scope.to_owned()),
    ]);
    let mut run = common.clone();
    run.insert("result".to_owned(), result.to_owned());
    // Exactly one increment is essential: host + native instrumentation would be two.
    let mut counters = Counters::from([((RUN.to_owned(), run), 1)]);
    for (status, count, reason) in outcomes {
        let mut tags = common.clone();
        tags.insert("status".to_owned(), (*status).to_owned());
        if let Some(reason) = reason {
            tags.insert("failure_reason".to_owned(), (*reason).to_owned());
        }
        counters.insert((THREAD.to_owned(), tags), *count);
    }
    counters
}

async fn collector(home: &Path) -> Result<MockServer> {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::path("/metrics"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    fs::write(
        home.join("config.toml"),
        format!(
            "cli_auth_credentials_store = \"file\"\nanalytics.enabled = true\n\
         [otel]\nmetrics_exporter = {{ otlp-http = {{ endpoint = \"{}/metrics\", protocol = \"json\" }} }}\n",
            server.uri(),
        ),
    )?;
    Ok(server)
}

fn metric_command(home: &Path) -> Result<tokio::process::Command> {
    let mut command = command(home)?;
    // Export on shutdown, avoiding periodic timing assumptions. Only the child
    // receives this setting; injected proxy/security settings remain untouched.
    command.env("OTEL_METRIC_EXPORT_INTERVAL", "60000");
    Ok(command)
}

async fn exported_counters(server: &MockServer) -> Result<Counters> {
    let requests = server.received_requests().await.context("OTLP requests")?;
    let mut counters = Counters::new();
    for request in requests {
        let body: Value = serde_json::from_slice(&request.body)?;
        for metric in body["resourceMetrics"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|resource| resource["scopeMetrics"].as_array().into_iter().flatten())
            .flat_map(|scope| scope["metrics"].as_array().into_iter().flatten())
        {
            let Some(name @ (RUN | THREAD)) = metric["name"].as_str() else {
                continue;
            };
            let temporality = &metric["sum"]["aggregationTemporality"];
            anyhow::ensure!(
                temporality.as_u64() == Some(1)
                    || temporality.as_str() == Some("AGGREGATION_TEMPORALITY_DELTA"),
                "counter aggregation must be Delta before summing exports: {metric}"
            );
            for point in metric["sum"]["dataPoints"]
                .as_array()
                .context("counter data points")?
            {
                let mut tags = Tags::new();
                for attribute in point["attributes"].as_array().context("counter tags")? {
                    let key = attribute["key"].as_str().context("tag key")?;
                    let value = attribute["value"]["stringValue"]
                        .as_str()
                        .context("tag value")?;
                    anyhow::ensure!(
                        tags.insert(key.to_owned(), value.to_owned()).is_none(),
                        "duplicate tag {key}"
                    );
                }
                let count = match &point["asInt"] {
                    Value::Number(count) => count.as_u64().context("unsigned metric count")?,
                    Value::String(count) => count.parse()?,
                    other => anyhow::bail!("unexpected counter value {other}"),
                };
                *counters.entry((name.to_owned(), tags)).or_default() += count;
            }
        }
    }
    anyhow::ensure!(
        !counters.is_empty(),
        "no migration metrics reached the OTLP collector"
    );
    Ok(counters)
}

#[tokio::test]
async fn native_and_selected_export_one_run_and_matching_thread_outcomes() -> Result<()> {
    for case in [
        Case::DryRun,
        Case::FilteredApply,
        Case::PartialFailure,
        Case::Busy,
    ] {
        let root = tempfile::tempdir()?;
        let mut exports = Vec::new();
        for selected in [false, true] {
            let home = root
                .path()
                .join(if selected { "selected" } else { "native" });
            for id in [FIRST_ID, SECOND_ID] {
                seed(&home, root.path(), id)?;
            }
            if matches!(case, Case::PartialFailure) {
                let broken = seed(&home, root.path(), BROKEN_ID)?;
                fs::write(broken, "invalid rollout metadata\n")?;
            }
            if selected {
                install_native(&home)?;
            }
            let server = collector(&home).await?;
            let _maintenance = if matches!(case, Case::Busy) {
                Some(
                    codex_rollout::try_acquire_rollout_maintenance_lock(&home)?
                        .context("fixture must hold the native maintenance lock")?,
                )
            } else {
                None
            };
            let output =
                tokio::time::timeout(TIMEOUT, metric_command(&home)?.args(case.args()).output())
                    .await??;
            assert_eq!(
                output.status.success(),
                matches!(case, Case::DryRun | Case::FilteredApply),
                "{case:?}, selected={selected}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let actual = exported_counters(&server).await?;
            assert_eq!(actual, case.expected(), "{case:?}, selected={selected}");
            exports.push(actual);
            #[cfg(unix)]
            if selected {
                assert_reaped(&home).await?;
            }
        }
        assert_eq!(
            exports[0], exports[1],
            "{case:?}: native/selected export parity"
        );
    }
    Ok(())
}

#[cfg(unix)]
async fn cancel_during_publication(home: &Path) -> Result<Output> {
    use codex_protocol::ThreadId;
    use codex_protocol::protocol::ThreadHistoryMode;
    use codex_rollout::RolloutConfig;
    use std::time::Duration;

    let sqlite = sqlite(home)?;
    let metadata = codex_rollout::state_db::try_init(&RolloutConfig {
        codex_home: home.to_path_buf(),
        sqlite: sqlite.clone(),
        cwd: home.to_path_buf(),
        model_provider_id: "openai".to_owned(),
        generate_memories: false,
    })
    .await?;
    let indexed = metadata
        .get_thread(ThreadId::from_string(FIRST_ID)?)
        .await?;
    metadata.close().await;
    assert_eq!(
        indexed.context("valid rollout metadata")?.history_mode,
        ThreadHistoryMode::Legacy
    );
    let history = codex_state::open_thread_history_db(&sqlite).await?;
    let transaction = history.begin_with("BEGIN IMMEDIATE").await?;
    let stderr = home.join("metrics-cancel.stderr");
    let mut child = metric_command(home)?
        .arg("--apply")
        .stderr(fs::File::create(&stderr)?)
        .spawn()?;
    let writer = home
        .join("thread-writer-locks")
        .join(format!("{FIRST_ID}.lock"));
    wait_for_file(&writer).await?;
    signal(child.id().context("CLI process id")?, libc::SIGINT)?;
    tokio::time::timeout(TIMEOUT, async {
        while !fs::read_to_string(&stderr)?.contains("Stopping migration;") {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok::<_, std::io::Error>(())
    })
    .await??;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let early_exit = child.try_wait()?;
    transaction.rollback().await?;
    let mut output = tokio::time::timeout(TIMEOUT, child.wait_with_output()).await??;
    output.stderr = fs::read(stderr)?;
    history.close().await;
    assert!(
        early_exit.is_none(),
        "publication must still be owned when cancellation is acknowledged"
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report(&output, home)?["outcomes"][0]["status"], "migrated");
    assert!(!writer.exists());
    Ok(output)
}

#[cfg(unix)]
#[tokio::test]
async fn cancelled_native_and_selected_runs_export_matching_completion() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut exports = Vec::new();
    for selected in [false, true] {
        let home = root
            .path()
            .join(if selected { "selected" } else { "native" });
        seed(&home, root.path(), FIRST_ID)?;
        if selected {
            install_native(&home)?;
        }
        let server = collector(&home).await?;
        cancel_during_publication(&home).await?;
        let actual = exported_counters(&server).await?;
        assert_eq!(
            actual,
            expected("apply", "all", "cancelled", &[("migrated", 1, None)])
        );
        exports.push(actual);
        if selected {
            assert_reaped(&home).await?;
        }
    }
    assert_eq!(exports[0], exports[1]);
    Ok(())
}
