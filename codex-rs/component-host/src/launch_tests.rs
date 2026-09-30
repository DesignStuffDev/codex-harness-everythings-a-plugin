#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use codex_component_host::ComponentBinding;
use codex_component_host::ComponentSpec;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

const PLUGIN: &str = r#"
import json, os, pathlib, sys, time
initialize = json.loads(sys.stdin.readline())
state = pathlib.Path(initialize['state_dir'])
state.joinpath('pid').write_text(str(os.getpid()))
if initialize['config']['startup_stall']:
    time.sleep(60)
print(json.dumps({'type': 'ready', 'api_version': 1}), flush=True)
request = json.loads(sys.stdin.readline())
state.joinpath('request').write_text('received')
assert json.loads(sys.stdin.readline()) == {'type': 'shutdown'}
state.joinpath('shutdown').write_text('received')
time.sleep(60)
"#;

#[tokio::test]
async fn launch_deadline_force_terminates_with_explicit_unknown_durability() {
    assert_interrupted_child(false).await;
}

#[tokio::test]
async fn launch_interrupt_during_handshake_cleans_up_and_reports_uncertainty() {
    assert_interrupted_child(true).await;
}

async fn assert_interrupted_child(startup_stall: bool) {
    let (_directory, binding) = fixture(
        PLUGIN,
        json!({"startup_stall": startup_stall}),
        /*timeout_ms*/ 5_000,
    );
    let marker = binding
        .state_dir
        .join(if startup_stall { "pid" } else { "request" });
    let mut first = true;
    let error = super::run(&binding, Value::Null, Duration::from_millis(100), || {
        let interrupt: Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>> = if first {
            first = false;
            let marker = marker.clone();
            Box::pin(async move {
                while !marker.exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok(())
            })
        } else {
            Box::pin(std::future::pending())
        };
        interrupt
    })
    .await
    .unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("durability is unknown"), "{message}");
    assert!(message.contains("accepted writes"), "{message}");
    if startup_stall {
        assert!(message.contains("before readiness"), "{message}");
        assert!(!binding.state_dir.join("request").exists());
    } else {
        assert!(binding.state_dir.join("shutdown").exists());
    }
    #[cfg(unix)]
    {
        let pid: libc::pid_t = std::fs::read_to_string(binding.state_dir.join("pid"))
            .unwrap()
            .parse()
            .unwrap();
        // No polling allowance: returning from launch must acknowledge wait,
        // before the manager runtime can shut down and lose a detached reaper.
        // SAFETY: signal zero checks existence without affecting the child.
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "child must already be reaped"
        );
    }
}

fn fixture(source: &str, config: Value, timeout_ms: u64) -> (tempfile::TempDir, ComponentBinding) {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("plugin.py");
    std::fs::write(&script, source).unwrap();
    let python = ["python3", "python"]
        .into_iter()
        .find(|candidate| {
            std::process::Command::new(candidate)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        })
        .expect("component process tests require Python 3");
    let binding = ComponentBinding {
        plugin_id: "launch-fixture".into(),
        spec: ComponentSpec {
            kind: "presentation".into(),
            name: "fixture".into(),
            contract_version: 1,
            metadata: Value::Null,
        },
        config,
        package_dir: directory.path().to_owned(),
        state_dir: directory.path().join("state"),
        entrypoint: python.into(),
        args: vec![script.to_string_lossy().into_owned()],
        timeout_ms,
    };
    (directory, binding)
}

#[path = "launch_service_tests.rs"]
mod service;
