use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_rollout::RolloutConfig;
use pretty_assertions::assert_eq;
use serde_json::Value;

use super::support::*;

// A pre-migration storage2 implementation. It accepts only open, advertises no
// new optional capability, and records initialization without touching native DBs.
const UNSUPPORTED_STORE: &str = r#"#!/usr/bin/env python3
import base64, json, os, pathlib, sys
initial = json.loads(sys.stdin.readline())
state = pathlib.Path(initial['state_dir'])
pathlib.Path(sys.argv[1]).write_text(str(os.getpid()))
def frame(value):
    print(json.dumps(value), flush=True)
frame({'type':'ready', 'api_version':1, 'session':{'mode':'multiplexed','version':1}})
pending = {}
for line in sys.stdin:
    value = json.loads(line)
    kind = value['type']
    if kind == 'request_start':
        pending[value['id']] = [value, bytearray()]
    elif kind == 'chunk':
        pending[value['id']][1].extend(base64.b64decode(value['data']))
    elif kind == 'end':
        request, data = pending.pop(value['id'])
        assert request['method'] == 'thread_store/open', request
        params = json.loads(data)
        state.joinpath('open.json').write_text(json.dumps(params))
        capabilities = {'contract_version':2, 'default_history_mode':'paginated',
            'thread_sections':False, 'thread_attachments':False, 'projects':False,
            'paginated_history_lists':True, 'rollout_maintenance':False,
            'rollout_path_reads':False, 'shared_local_sqlite':params['paths']}
        body = json.dumps(capabilities).encode()
        frame({'type':'result_start','id':request['id'],'bytes':len(body)})
        frame({'type':'chunk','id':request['id'],'index':0,'data':base64.b64encode(body).decode()})
        frame({'type':'end','id':request['id'],'chunks':1})
    elif kind == 'shutdown':
        state.joinpath('shutdown').write_text('joined')
        frame({'type':'shutdown_complete'})
        break
    else:
        raise AssertionError(value)
"#;

#[tokio::test]
async fn unsupported_selected_storage_never_falls_back_or_initializes_native_metadata() -> Result<()>
{
    for args in [vec![], vec!["--apply"]] {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        let path = seed(&home, root.path(), FIRST_ID)?;
        let original = fs::read(&path)?;
        let package = tempfile::tempdir()?;
        let entrypoint = package.path().join("unsupported.py");
        fs::write(&entrypoint, UNSUPPORTED_STORE)?;
        fs::set_permissions(entrypoint, fs::Permissions::from_mode(0o755))?;
        install(
            &home,
            package.path(),
            "unsupported.py",
            &[home.join("worker.pid")],
        )?;
        drop(package);
        let output = run(&home, &args).await?;
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr)?;
        assert!(error.contains("manual_rollout_migration"), "{error}");
        let state = home.join("components/state").join(PLUGIN_ID);
        let initialization: Value = serde_json::from_slice(&fs::read(state.join("open.json"))?)?;
        assert_eq!(initialization["state_db_enabled"], !args.is_empty());
        assert_eq!(initialization["startup_migration"], false);
        assert_eq!(initialization["startup_compression"], false);
        assert_eq!(fs::read_to_string(state.join("shutdown"))?, "joined");
        assert_eq!(fs::read(&path)?, original);
        assert_no_metadata(&home)?;
        assert_reaped(&home).await?;
    }
    Ok(())
}

async fn history_with_metadata(home: &Path) -> Result<sqlx::SqlitePool> {
    let config = RolloutConfig {
        codex_home: home.to_path_buf(),
        sqlite: sqlite(home)?,
        cwd: home.to_path_buf(),
        model_provider_id: "openai".to_owned(),
        generate_memories: false,
    };
    let metadata = codex_rollout::state_db::try_init(&config).await?;
    let indexed = metadata
        .get_thread(ThreadId::from_string(FIRST_ID)?)
        .await?;
    metadata.close().await;
    assert_eq!(
        indexed
            .context("valid rollout was not backfilled")?
            .history_mode,
        ThreadHistoryMode::Legacy
    );
    codex_state::open_thread_history_db(&config.sqlite).await
}

async fn wait_for_stopping(stderr: &Path) -> Result<()> {
    tokio::time::timeout(TIMEOUT, async {
        while !fs::read_to_string(stderr)?.contains("Stopping migration;") {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Ok::<_, std::io::Error>(())
    })
    .await
    .context("CLI did not acknowledge the first interrupt")??;
    Ok(())
}

fn diagnostics(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter(|line| line.starts_with("Stopping migration;") || line.starts_with("Error:"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn first_interrupt_joins_native_and_selected_inflight_publication() -> Result<()> {
    for selected in [false, true] {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        let path = seed(&home, root.path(), FIRST_ID)?;
        if selected {
            install_native(&home)?;
        }
        let history = history_with_metadata(&home).await?;
        let transaction = history.begin_with("BEGIN IMMEDIATE").await?;
        let stderr = home.join("cli.stderr");
        let mut child = command(&home)?
            .arg("--apply")
            .stderr(fs::File::create(&stderr)?)
            .spawn()?;
        let writer = home
            .join("thread-writer-locks")
            .join(format!("{FIRST_ID}.lock"));
        wait_for_file(&writer).await?;
        signal(child.id().context("CLI process id")?, libc::SIGINT)?;
        wait_for_stopping(&stderr).await?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        let early_exit = child.try_wait()?;
        // Always release our lock before checking whether cleanup returned early.
        transaction.rollback().await?;
        let mut output = tokio::time::timeout(TIMEOUT, child.wait_with_output()).await??;
        output.stderr = fs::read(&stderr)?;
        history.close().await;
        assert!(
            early_exit.is_none(),
            "CLI exited before accepted publication could finish"
        );
        assert_eq!(output.status.code(), Some(1));
        insta::allow_duplicates! {
            insta::assert_snapshot!(diagnostics(&output), @r"
            Stopping migration; waiting for accepted work and storage cleanup...
            Error: rollout migration cancelled after joined cleanup
            ");
        }
        let result = report(&output, &home)?;
        assert_eq!(result["outcomes"].as_array().expect("outcomes").len(), 1);
        assert_eq!(result["outcomes"][0]["status"], "migrated");
        assert!(!writer.exists());
        assert_eq!(
            codex_rollout::read_session_meta_line(&path)
                .await?
                .meta
                .history_mode,
            ThreadHistoryMode::Paginated
        );
        if selected {
            assert_reaped(&home).await?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn repeated_interrupt_reports_uncertainty_and_selected_store_can_recover() -> Result<()> {
    let root = tempfile::tempdir()?;
    let home = root.path().join("home");
    let path = seed(&home, root.path(), FIRST_ID)?;
    let original = fs::read(&path)?;
    install_native(&home)?;
    let history = history_with_metadata(&home).await?;
    let transaction = history.begin_with("BEGIN IMMEDIATE").await?;
    let stderr = home.join("cli.stderr");
    let child = command(&home)?
        .arg("--apply")
        .stderr(fs::File::create(&stderr)?)
        .spawn()?;
    let pid = child.id().context("CLI process id")?;
    let writer = home
        .join("thread-writer-locks")
        .join(format!("{FIRST_ID}.lock"));
    wait_for_file(&writer).await?;
    signal(pid, libc::SIGINT)?;
    wait_for_stopping(&stderr).await?;
    signal(pid, libc::SIGTERM)?;
    let result = tokio::time::timeout(TIMEOUT, child.wait_with_output()).await;
    transaction.rollback().await?;
    history.close().await;
    let mut output = result.context("forced migration did not terminate")??;
    output.stderr = fs::read(&stderr)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "forced stop must not print a completed report"
    );
    insta::assert_snapshot!(diagnostics(&output), @r"
    Stopping migration; waiting for accepted work and storage cleanup...
    Error: forced migration shutdown after repeated signal; durability is uncertain
    ");
    assert_reaped(&home).await?;
    assert_eq!(fs::read(&path)?, original);
    let recovered = run(&home, &["--apply"]).await?;
    assert!(
        recovered.status.success(),
        "{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    assert_eq!(
        report(&recovered, &home)?["outcomes"][0]["status"],
        "migrated"
    );
    assert!(!writer.exists());
    assert_reaped(&home).await?;
    Ok(())
}
