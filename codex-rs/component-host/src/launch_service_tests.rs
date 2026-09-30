#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use codex_component_host::ComponentBinding;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio::time::sleep;
use tokio::time::timeout;

use super::fixture;

const SERVICE: &str = r#"
import json, os, pathlib, sys, time
initialize = json.loads(sys.stdin.readline())
state = pathlib.Path(initialize['state_dir'])
state.joinpath('pid').write_text(str(os.getpid()))
mode = initialize['config']['mode']
def send(value):
    print(json.dumps(value), flush=True)
def gate(name):
    while not state.joinpath(name).exists():
        time.sleep(0.005)
if mode == 'startup_stall':
    gate('never')
send({'type': 'ready', 'api_version': 2 if mode == 'bad_handshake' else 1})
if mode == 'bad_handshake':
    gate('never')
request = json.loads(sys.stdin.readline())
state.joinpath('request').write_text('received')
if mode == 'malformed':
    print('{', flush=True)
    gate('never')
if mode == 'error':
    send({'type':'error', 'id':1, 'message':'fixture error'})
    gate('never')
if mode == 'idle':
    gate('finish')
if mode == 'interrupt':
    assert json.loads(sys.stdin.readline()) == {'type':'shutdown'}
    state.joinpath('shutdown').write_text('received')
    gate('finish')
send({'type':'result', 'id':1, 'result':{'drained':True}})
if mode != 'interrupt':
    assert json.loads(sys.stdin.readline()) == {'type':'shutdown'}
    state.joinpath('shutdown').write_text('received')
if mode in ('cleanup', 'cleanup_stall'):
    gate('cleanup')
assert sys.stdin.read() == '', 'shutdown must be sent once'
state.joinpath('finished').write_text('yes')
sys.exit(23 if mode == 'failed_exit' else 0)
"#;

async fn wait_for(path: &Path) {
    timeout(Duration::from_secs(5), async {
        while !path.exists() {
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

fn assert_reaped(binding: &ComponentBinding) {
    #[cfg(unix)]
    {
        let pid: libc::pid_t = std::fs::read_to_string(binding.state_dir.join("pid"))
            .unwrap()
            .parse()
            .unwrap();
        // SAFETY: signal zero only probes the exact fixture process.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
    #[cfg(not(unix))]
    let _ = binding;
}

#[tokio::test]
async fn launch_survives_its_startup_timeout_after_readiness() {
    let (_directory, binding) = fixture(SERVICE, json!({"mode":"idle"}), /*timeout_ms*/ 300);
    let owned = binding.clone();
    let mut running = tokio::spawn(async move {
        super::super::run(&owned, Value::Null, Duration::from_secs(2), || {
            std::future::pending()
        })
        .await
    });
    wait_for(&binding.state_dir.join("request")).await;
    assert!(
        timeout(Duration::from_millis(600), &mut running)
            .await
            .is_err()
    );
    std::fs::write(binding.state_dir.join("finish"), "yes").unwrap();
    timeout(Duration::from_secs(3), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_reaped(&binding);
}

#[tokio::test]
async fn interrupt_near_startup_limit_receives_its_full_cleanup_budget() {
    let (_directory, binding) = fixture(
        SERVICE,
        json!({"mode":"interrupt"}),
        /*timeout_ms*/ 300,
    );
    let owned = binding.clone();
    let request = binding.state_dir.join("request");
    let mut first = true;
    let mut running = tokio::spawn(async move {
        super::super::run(&owned, Value::Null, Duration::from_secs(2), || {
            let interrupt: Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>> = if first {
                first = false;
                let request = request.clone();
                Box::pin(async move {
                    wait_for(&request).await;
                    sleep(Duration::from_millis(150)).await;
                    Ok(())
                })
            } else {
                Box::pin(std::future::pending())
            };
            interrupt
        })
        .await
    });
    wait_for(&binding.state_dir.join("shutdown")).await;
    assert!(
        timeout(Duration::from_millis(600), &mut running)
            .await
            .is_err()
    );
    std::fs::write(binding.state_dir.join("finish"), "yes").unwrap();
    timeout(Duration::from_secs(3), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_reaped(&binding);
}

#[tokio::test]
async fn normal_result_allows_presentation_cleanup_beyond_half_a_second() {
    let (_directory, binding) =
        fixture(SERVICE, json!({"mode":"cleanup"}), /*timeout_ms*/ 300);
    let owned = binding.clone();
    let mut running = tokio::spawn(async move {
        super::super::run(&owned, Value::Null, Duration::from_secs(2), || {
            std::future::pending()
        })
        .await
    });
    wait_for(&binding.state_dir.join("shutdown")).await;
    assert!(
        timeout(Duration::from_millis(800), &mut running)
            .await
            .is_err()
    );
    std::fs::write(binding.state_dir.join("cleanup"), "yes").unwrap();
    timeout(Duration::from_secs(3), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_reaped(&binding);
}

#[tokio::test]
async fn launch_failure_paths_without_interrupt_report_uncertainty_after_reaping() {
    for (mode, expected) in [
        ("bad_handshake", "handshake requires ready"),
        ("startup_stall", "startup timed out"),
        ("malformed", "invalid component JSON frame"),
        ("error", "fixture error"),
        ("failed_exit", "exited unsuccessfully"),
        ("cleanup_stall", "shutdown timed out"),
    ] {
        let (_directory, binding) = fixture(SERVICE, json!({"mode":mode}), /*timeout_ms*/ 300);
        let error = timeout(
            Duration::from_secs(5),
            super::super::run(&binding, Value::Null, Duration::from_millis(100), || {
                std::future::pending()
            }),
        )
        .await
        .unwrap()
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains(expected), "mode {mode}: {message}");
        assert!(message.contains("durability is unknown"), "{message}");
        assert_reaped(&binding);
    }
}
