use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Output;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use codex_protocol::ResponseItemId;
use codex_protocol::ThreadId;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::AgentMessageEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionMeta;
use codex_protocol::protocol::SessionMetaLine;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::TurnCompleteEvent;
use codex_protocol::protocol::TurnStartedEvent;
use codex_protocol::protocol::UserMessageEvent;
use codex_rollout::RolloutItem;
use codex_rollout::RolloutLine;
use codex_state::SqliteConfig;
use codex_utils_absolute_path::AbsolutePathBuf;
#[cfg(unix)]
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

pub const FIRST_ID: &str = "11111111-1111-4111-8111-111111111111";
pub const SECOND_ID: &str = "22222222-2222-4222-8222-222222222222";
pub const BROKEN_ID: &str = "33333333-3333-4333-8333-333333333333";
pub const PLUGIN_ID: &str = "test.cli-native-migration";
pub const TIMEOUT: Duration = Duration::from_secs(30);

pub fn sqlite(home: &Path) -> Result<SqliteConfig> {
    Ok(SqliteConfig::from_sqlite_home(
        AbsolutePathBuf::from_absolute_path(home)?,
    ))
}

pub fn rollout(home: &Path, id: &str) -> PathBuf {
    home.join("sessions/2025/01/03")
        .join(format!("rollout-2025-01-03T12-00-00-{id}.jsonl"))
}

pub fn seed(home: &Path, cwd: &Path, id: &str) -> Result<PathBuf> {
    let path = rollout(home, id);
    fs::create_dir_all(path.parent().context("rollout directory")?)?;
    let mut file = fs::File::create(&path)?;
    let thread_id = ThreadId::from_string(id)?;
    let turn_id = format!("migration-{id}");
    let timestamp = "2025-01-03T12:00:00Z";
    // Identical input bytes in both homes make bytes_processed part of parity.
    for item in [
        RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                session_id: thread_id.into(),
                id: thread_id,
                timestamp: timestamp.to_owned(),
                cwd: cwd.to_path_buf(),
                originator: "cli-migration-test".to_owned(),
                cli_version: "0.0.0".to_owned(),
                source: SessionSource::Cli,
                model_provider: Some("openai".to_owned()),
                history_mode: ThreadHistoryMode::Legacy,
                ..Default::default()
            },
            git: None,
        }),
        RolloutItem::EventMsg(EventMsg::TurnStarted(TurnStartedEvent {
            turn_id: turn_id.clone(),
            root_turn_id: None,
            trace_id: None,
            started_at: Some(1_735_905_600),
            model_context_window: None,
            collaboration_mode_kind: Default::default(),
        })),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "Preserve this migration question".to_owned(),
            ..Default::default()
        })),
        // Legacy display events and model-visible response items are separate
        // records. A resumable turn needs both, not only the displayed messages.
        RolloutItem::ResponseItem(
            ResponseItem::Message {
                id: Some(ResponseItemId::with_suffix("msg", format!("{id}-user"))),
                role: "user".to_owned(),
                content: vec![ContentItem::InputText {
                    text: "Preserve this migration question".to_owned(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            }
            .into(),
        ),
        RolloutItem::ResponseItem(
            ResponseItem::Message {
                id: Some(ResponseItemId::with_suffix(
                    "msg",
                    format!("{id}-assistant"),
                )),
                role: "assistant".to_owned(),
                content: vec![ContentItem::OutputText {
                    text: "Preserve this migration answer".to_owned(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            }
            .into(),
        ),
        RolloutItem::EventMsg(EventMsg::AgentMessage(AgentMessageEvent {
            message: "Preserve this migration answer".to_owned(),
            phase: None,
            memory_citation: None,
            delivery: None,
            questions: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id,
            last_agent_message: Some("Preserve this migration answer".to_owned()),
            error: None,
            started_at: Some(1_735_905_600),
            completed_at: Some(1_735_905_601),
            duration_ms: Some(1_000),
            time_to_first_token_ms: None,
        })),
    ] {
        let line = RolloutLine {
            timestamp: timestamp.to_owned(),
            ordinal: None,
            item,
        };
        writeln!(file, "{}", serde_json::to_string(&line)?)?;
    }
    Ok(path)
}

pub fn command(home: &Path) -> Result<tokio::process::Command> {
    let mut command = tokio::process::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    command
        .env("CODEX_HOME", home)
        .env(codex_state::SQLITE_HOME_ENV, home)
        .current_dir(home)
        .args(["migrate-rollouts", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    Ok(command)
}

pub async fn run(home: &Path, args: &[&str]) -> Result<Output> {
    tokio::time::timeout(TIMEOUT, command(home)?.args(args).output())
        .await
        .context("migration CLI did not terminate")?
        .context("run migration CLI")
}

pub fn report(output: &Output, home: &Path) -> Result<Value> {
    let mut value: Value = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "expected complete JSON report, status {}, stdout {}, stderr {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    normalize(&mut value, home);
    Ok(value)
}

pub fn normalize(value: &mut Value, home: &Path) {
    match value {
        Value::String(text) => *text = text.replace(&*home.to_string_lossy(), "<home>"),
        Value::Array(items) => items.iter_mut().for_each(|item| normalize(item, home)),
        Value::Object(fields) => fields.values_mut().for_each(|item| normalize(item, home)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

pub fn assert_no_metadata(home: &Path) -> Result<()> {
    for database in sqlite(home)?.runtime_db_paths() {
        assert!(
            !database.path.exists(),
            "created {}",
            database.path.display()
        );
    }
    assert!(!home.join("thread-writer-locks").exists());
    Ok(())
}

pub fn install_native(home: &Path) -> Result<()> {
    let package = tempfile::tempdir()?;
    let executable = format!("storage{}", std::env::consts::EXE_SUFFIX);
    fs::copy(
        codex_utils_cargo_bin::cargo_bin("codex-thread-store-local-plugin")?,
        package.path().join(&executable),
    )?;
    #[cfg(unix)]
    let (entrypoint, args) = {
        use std::os::unix::fs::PermissionsExt;
        let wrapper = package.path().join("storage-wrapper.sh");
        fs::write(
            &wrapper,
            "#!/bin/sh\nprintf '%s\\n' \"$$\" > \"$1\"\nshift\nexec \"${0%/*}/storage\" \"$@\"\n",
        )?;
        fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755))?;
        (
            "storage-wrapper.sh".to_owned(),
            vec![home.join("worker.pid")],
        )
    };
    #[cfg(not(unix))]
    let (entrypoint, args) = (executable, Vec::<PathBuf>::new());
    install(home, package.path(), &entrypoint, &args)?;
    // Dropping the source package leaves only the manager's installed copy.
    Ok(())
}

pub fn install(home: &Path, package: &Path, entrypoint: &str, args: &[PathBuf]) -> Result<()> {
    fs::write(
        package.join("codex-component.json"),
        serde_json::to_vec(&json!({
            "api_version":1,"id":PLUGIN_ID,"version":"1.0.0","entrypoint":entrypoint,
            "args":args,"components":[{"kind":"thread_store","name":"default","contract_version":2}]
        }))?,
    )?;
    codex_component_host::install(home, package)?;
    codex_component_host::select(home, "thread_store", "default", Some(PLUGIN_ID))?;
    Ok(())
}

#[cfg(unix)]
pub fn signal(pid: u32, signal: libc::c_int) -> Result<()> {
    let pid = libc::pid_t::try_from(pid)?;
    // SAFETY: signal only the explicitly owned test child identified by its PID.
    if unsafe { libc::kill(pid, signal) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(unix)]
pub async fn assert_reaped(home: &Path) -> Result<()> {
    let pid: libc::pid_t = fs::read_to_string(home.join("worker.pid"))?
        .trim()
        .parse()?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            // SAFETY: signal 0 observes existence without changing the process.
            if unsafe { libc::kill(pid, 0) } != 0 {
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::ESRCH)
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("migration worker remained alive or unreaped")?;
    Ok(())
}

#[cfg(unix)]
pub async fn wait_for_file(path: &Path) -> Result<()> {
    tokio::time::timeout(TIMEOUT, async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .with_context(|| format!("did not observe {}", path.display()))?;
    Ok(())
}
