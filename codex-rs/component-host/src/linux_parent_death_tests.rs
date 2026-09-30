#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio::time::sleep;
use tokio::time::timeout;

const STATE_ENV: &str = "CODEX_COMPONENT_PARENT_DEATH_STATE";
const MODE_ENV: &str = "CODEX_COMPONENT_PARENT_DEATH_MODE";
const PLUGIN: &str = r#"
import json, os, pathlib, sys, time
initialize = json.loads(sys.stdin.readline())
ready = {'type':'ready','api_version':1}
if 'session' in initialize:
    ready['session'] = initialize['session']
print(json.dumps(ready), flush=True)
for line in sys.stdin:
    request = json.loads(line)
    if request['type'] in ('request','end'):
        pathlib.Path(initialize['state_dir'], 'accepted').write_text(json.dumps({
            'pid':os.getpid(), 'parent':os.getppid(), 'group':os.getpgrp(),
            'parent_group':os.getpgid(os.getppid())}))
        time.sleep(60)
        break
"#;

/// This helper is a real intermediate engine process. The outer test kills it
/// without unwinding Rust or polling the blocked component handler again.
#[tokio::test]
#[ignore = "subprocess helper invoked only by parent-death regression"]
async fn helper_parent() {
    let Some(state) = std::env::var_os(STATE_ENV) else {
        return;
    };
    let mode = std::env::var(MODE_ENV).unwrap();
    let mut fixture = super::Fixture::new("stall");
    std::fs::write(&fixture.binding.args[0], PLUGIN).unwrap();
    fixture.binding.state_dir = PathBuf::from(state);
    fixture.binding.timeout_ms = 60_000;
    if mode == "persistent" {
        let session = fixture.binding.connect().await.unwrap();
        let _ = session.call("block", json!({})).await;
    } else {
        let _ = fixture.binding.call("block", json!({})).await;
    }
    panic!("blocked child unexpectedly completed before parent termination");
}

struct OrphanCleanup(i32);

impl Drop for OrphanCleanup {
    fn drop(&mut self) {
        // A failing regression must not leave its fixture process running.
        unsafe {
            libc::kill(self.0, libc::SIGKILL);
        }
    }
}

#[tokio::test]
async fn killed_parent_terminates_blocked_components_in_separate_process_groups() {
    for mode in ["one-shot", "persistent"] {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        let mut parent = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "process::tests::linux_parent_death::helper_parent",
                "--ignored",
                "--nocapture",
            ])
            .env(STATE_ENV, &state)
            .env(MODE_ENV, mode)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let parent_pid = parent.id().unwrap();
        let accepted: Value = timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(bytes) = tokio::fs::read(state.join("accepted")).await
                    && let Ok(value) = serde_json::from_slice(&bytes)
                {
                    break value;
                }
                assert!(
                    parent.try_wait().unwrap().is_none(),
                    "parent exited before blocked child started"
                );
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let child_pid = accepted["pid"].as_i64().unwrap() as i32;
        let _cleanup = OrphanCleanup(child_pid);
        assert_eq!(accepted["parent"], parent_pid);
        assert_eq!(accepted["group"], child_pid);
        assert_ne!(accepted["group"], accepted["parent_group"]);

        parent.kill().await.unwrap();
        timeout(Duration::from_secs(3), async {
            loop {
                // Once orphaned, reaping belongs to the new adopter. A zombie
                // is terminated and cannot execute even if container PID 1 waits.
                match tokio::fs::read_to_string(format!("/proc/{child_pid}/status")).await {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                    Ok(status)
                        if status
                            .lines()
                            .any(|line| line.starts_with("State:") && line.contains('Z')) =>
                    {
                        break;
                    }
                    Ok(_) => {}
                    Err(error) => panic!("inspect child termination: {error}"),
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("blocked component must terminate when its owning process is killed");
    }
}
