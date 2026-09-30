#![cfg(unix)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::time::timeout;

use super::Fixture;

const SERVICE: &str = r#"
import json, os, pathlib, sys, threading

initialize = json.loads(sys.stdin.readline())
state = pathlib.Path(initialize['state_dir'])
mode = initialize['config']['mode']
pending_pid = state.joinpath('pending_pid')
pending_pid.write_text(str(os.getpid()))
pending_pid.replace(state.joinpath('pid'))

def frame(value):
    print(json.dumps(value), flush=True)

def stall():
    threading.Event().wait()

if mode == 'stall_handshake':
    stall()
if mode == 'reject_handshake':
    frame({'type': 'ready', 'api_version': 1})
    stall()

frame({'type': 'ready', 'api_version': 1,
       'session': {'mode': 'multiplexed', 'version': 1}})
if mode == 'writer_failure':
    os.close(0)
    state.joinpath('stdin_closed').write_text('yes')
    stall()

for line in sys.stdin:
    message = json.loads(line)
    if message['type'] == 'end':
        state.joinpath('accepted').write_text('yes')
        if mode == 'reader_failure':
            print('{', flush=True)
            stall()
        if mode == 'crash':
            os._exit(23)
    if message['type'] == 'shutdown':
        state.joinpath('shutdown').write_text('yes')
        if mode == 'stall_shutdown':
            stall()
        frame({'type': 'shutdown_complete'})
        if mode == 'stall_exit':
            stall()
        sys.exit(23 if mode == 'failed_exit' else 0)
"#;

fn fixture(mode: &str) -> Fixture {
    let mut fixture = Fixture::new();
    std::fs::write(fixture.binding.package_dir.join("service.py"), SERVICE).unwrap();
    fixture.binding.config = json!({"mode": mode});
    fixture.binding.timeout_ms = 1_000;
    fixture
}

fn pid(fixture: &Fixture) -> libc::pid_t {
    std::fs::read_to_string(fixture.binding.state_dir.join("pid"))
        .unwrap()
        .parse()
        .unwrap()
}

fn child_is_absent(pid: libc::pid_t) -> bool {
    // SAFETY: signal 0 probes only the exact fixture child; it sends no signal.
    let status = unsafe { libc::kill(pid, 0) };
    status == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

fn assert_reaped(fixture: &Fixture) {
    let pid = pid(fixture);
    assert!(
        child_is_absent(pid),
        "persistent fixture {pid} remained live or zombie after cleanup returned"
    );
}

#[tokio::test]
async fn startup_rejection_and_deadline_reap_before_returning_error() {
    for (mode, expected) in [
        ("reject_handshake", "does not support multiplexed"),
        ("stall_handshake", "handshake timed out"),
    ] {
        let fixture = fixture(mode);
        let error = timeout(Duration::from_secs(5), fixture.binding.connect())
            .await
            .unwrap()
            .unwrap_err();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
        assert_reaped(&fixture);
    }
}

#[tokio::test]
async fn cancelled_startup_keeps_its_owner_until_the_child_is_reaped() {
    let mut fixture = fixture("stall_handshake");
    fixture.binding.timeout_ms = 15_000;
    let binding = fixture.binding.clone();
    let caller = tokio::spawn(async move { binding.connect().await });
    fixture.wait_for("pid", 1).await;
    let pid = pid(&fixture);
    assert!(!child_is_absent(pid));
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    // There is no public completion after cancellation. Its independent owner
    // still runs while the runtime is alive, and must actually reap, not just kill.
    timeout(Duration::from_secs(5), async {
        while !child_is_absent(pid) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn reader_writer_and_peer_failures_reap_before_terminal_completion() {
    for mode in ["reader_failure", "writer_failure", "crash"] {
        let mut fixture = fixture(mode);
        fixture.binding.timeout_ms = 5_000;
        let session = fixture.binding.connect().await.unwrap();
        if mode == "writer_failure" {
            fixture.wait_for("stdin_closed", 1).await;
        }
        let error = timeout(
            Duration::from_secs(5),
            session.call("work", json!({"body": "x".repeat(512 * 1024)})),
        )
        .await
        .unwrap()
        .unwrap_err();
        assert!(
            format!("{error:#}").contains("connection failed"),
            "{error:#}"
        );
        assert_reaped(&fixture);
        let close_error = session.close().await.unwrap_err();
        assert_eq!(close_error.to_string(), error.to_string());
    }
}

#[tokio::test]
async fn shutdown_deadline_failed_exit_and_success_all_return_after_reaping() {
    for (mode, expected) in [
        ("stall_shutdown", Some("shutdown timed out")),
        ("stall_exit", Some("exit timed out")),
        ("failed_exit", Some("exited unsuccessfully")),
        ("normal", None),
    ] {
        let fixture = fixture(mode);
        let session = fixture.binding.connect().await.unwrap();
        let outcome = timeout(Duration::from_secs(5), session.close())
            .await
            .unwrap();
        match expected {
            Some(expected) => {
                let error = outcome.unwrap_err();
                let text = format!("{error:#}");
                assert!(text.contains(expected), "{text}");
                assert!(text.contains("outcomes may be unknown"), "{text}");
            }
            None => outcome.unwrap(),
        }
        assert_reaped(&fixture);
    }
}
