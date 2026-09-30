#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;

use super::ComponentBinding;
use super::StreamFrame;
use crate::ComponentSpec;

#[cfg(target_os = "linux")]
#[path = "linux_parent_death_tests.rs"]
mod linux_parent_death;

const PLUGIN: &str = r#"
import json, os, pathlib, stat, subprocess, sys, time

def send(frame):
    print(json.dumps(frame), flush=True)

initialize = json.loads(sys.stdin.readline())
state = pathlib.Path(initialize['state_dir'])
state.joinpath('pid').write_text(str(os.getpid()))
mode = initialize['config']['mode']
if mode == 'handshake_stall':
    time.sleep(60)
if mode == 'bad_version':
    send({'type': 'ready', 'api_version': 999})
else:
    send({'type': 'ready', 'api_version': 1})
request = json.loads(sys.stdin.readline())
if mode == 'stall':
    time.sleep(60)
elif mode == 'descendant':
    child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])
    state.joinpath('descendant').write_text(str(child.pid))
    send({'type': 'event', 'id': 1, 'event': child.pid})
    time.sleep(60)
elif mode == 'stream':
    for index in range(3):
        send({'type': 'event', 'id': 1, 'event': {'index': index}})
elif mode == 'endless':
    while True:
        send({'type': 'event', 'id': 1, 'event': 'tick'})
        time.sleep(0.01)
elif mode == 'bad_id':
    send({'type': 'result', 'id': 2, 'result': 'wrong request'})
elif mode == 'bad_kind':
    send({'type': 'ready', 'id': 1, 'api_version': 1})
elif mode == 'missing_payload':
    send({'type': 'result', 'id': 1})
elif mode == 'error':
    send({'type': 'error', 'id': 1, 'message': 'fixture failure'})
elif mode == 'invalid_json':
    print('not-json', flush=True)
elif mode == 'oversized':
    print('x' * (4 * 1024 * 1024), flush=True)
elif mode == 'partial_eof':
    sys.stdout.write('{"type": "result"')
    sys.stdout.flush()
    sys.exit(0)
elif mode == 'graceful_partial':
    sys.stdout.write('{"type":"event","id":1,"event":"' + 'x' * 131072)
    sys.stdout.flush()
    state.joinpath('partial').write_text('sent')
    assert json.loads(sys.stdin.readline()) == {'type': 'shutdown'}
    state.joinpath('shutdown').write_text('received')
    sys.stdout.write('"}\n')
    sys.stdout.flush()
    send({'type': 'result', 'id': 1, 'result': {'drained': True}})
    assert sys.stdin.read() == '', 'duplicate shutdown frame'
    time.sleep(0.7)
    state.joinpath('cleanup').write_text('complete')
    sys.exit(0)
elif mode == 'eof':
    sys.exit(0)

if mode == 'persistent':
    counter_file = state.joinpath('counter')
    count = int(counter_file.read_text()) + 1 if counter_file.exists() else 1
    counter_file.write_text(str(count))
    result = {'count': count, 'pid': os.getpid()}
elif mode == 'echo':
    result = {'initialize': initialize, 'request': request, 'cwd': os.getcwd()}
elif mode == 'inspect_stderr':
    print('dummy-auth-diagnostic', file=sys.stderr, flush=True)
    diagnostic = os.fstat(2)
    null_device = os.stat(os.devnull)
    result = {'stderr_is_null': stat.S_ISCHR(diagnostic.st_mode) and diagnostic.st_rdev == null_device.st_rdev}
else:
    result = {'ok': True}
send({'type': 'result', 'id': 1, 'result': result})
shutdown = json.loads(sys.stdin.readline())
assert shutdown == {'type': 'shutdown'}, shutdown
state.joinpath('shutdown').write_text('received')
if mode == 'graceful_late':
    time.sleep(0.7)
    state.joinpath('cleanup').write_text('complete')
if mode == 'shutdown_stall':
    time.sleep(60)
if mode == 'bad_exit':
    sys.exit(17)
"#;

struct Fixture {
    _directory: TempDir,
    binding: ComponentBinding,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let directory = tempfile::tempdir().expect("temporary plugin directory");
        let script = directory.path().join("plugin.py");
        std::fs::write(&script, PLUGIN).expect("write plugin fixture");
        let python = ["python3", "python"]
            .into_iter()
            .find(|candidate| {
                std::process::Command::new(candidate)
                    .arg("--version")
                    .output()
                    .is_ok_and(|output| output.status.success())
            })
            .expect("process component tests require Python 3");
        let binding = ComponentBinding {
            plugin_id: "fixture".to_owned(),
            spec: ComponentSpec {
                kind: "tool".to_owned(),
                name: "fixture".to_owned(),
                contract_version: 1,
                metadata: Value::Null,
            },
            config: json!({"mode": mode}),
            package_dir: directory.path().to_path_buf(),
            state_dir: directory.path().join("state"),
            entrypoint: PathBuf::from(python),
            args: vec![script.to_string_lossy().into_owned()],
            timeout_ms: 5_000,
        };
        Self {
            _directory: directory,
            binding,
        }
    }

    async fn pid(&self) -> u32 {
        timeout(Duration::from_secs(3), async {
            loop {
                if let Ok(pid) = tokio::fs::read_to_string(self.binding.state_dir.join("pid")).await
                    && let Ok(pid) = pid.parse()
                {
                    return pid;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("plugin should start")
    }
}

#[tokio::test]
async fn invocation_passes_explicit_contract_and_shutdown() {
    let fixture = Fixture::new("echo");
    let params = json!({"text": "hello", "nested": [1, 2]});
    let result = fixture
        .binding
        .call("execute", params.clone())
        .await
        .unwrap();
    assert_eq!(
        result,
        json!({
            "initialize": {
                "type": "initialize", "api_version": 1, "plugin_id": "fixture",
                "config": {"mode": "echo"}, "state_dir": fixture.binding.state_dir,
            },
            "request": {
                "type": "request", "id": 1, "component": {"kind": "tool", "name": "fixture"},
                "method": "execute", "params": params,
            },
            "cwd": fixture.binding.package_dir,
        })
    );
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("shutdown")).unwrap(),
        "received"
    );
}

#[tokio::test]
async fn streaming_preserves_event_order_and_finishes_once() {
    let fixture = Fixture::new("stream");
    let mut stream = fixture
        .binding
        .stream("execute", Value::Null)
        .await
        .unwrap();
    let mut frames = Vec::new();
    for _ in 0..4 {
        frames.push(stream.next().await.unwrap());
    }
    assert_eq!(
        frames,
        vec![
            StreamFrame::Event(json!({"index": 0})),
            StreamFrame::Event(json!({"index": 1})),
            StreamFrame::Event(json!({"index": 2})),
            StreamFrame::Done(json!({"ok": true})),
        ]
    );
    assert!(
        stream
            .next()
            .await
            .unwrap_err()
            .to_string()
            .contains("finished")
    );
}

#[tokio::test]
async fn graceful_shutdown_preserves_partial_frame_and_waits_for_cleanup() {
    let fixture = Fixture::new("graceful_partial");
    let mut stream = fixture.binding.stream("launch", Value::Null).await.unwrap();
    let shutdown = stream.shutdown_handle();
    timeout(Duration::from_secs(3), async {
        while !fixture.binding.state_dir.join("partial").exists() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let event = {
        let next = stream.next();
        tokio::pin!(next);
        tokio::select! {
            result = &mut next => panic!("frame completed before shutdown: {result:?}"),
            _ = sleep(Duration::from_millis(20)) => {}
        }
        shutdown.request(Duration::from_secs(3));
        next.await.unwrap()
    };
    assert_eq!(event, StreamFrame::Event(json!("x".repeat(131072))));
    assert_eq!(
        stream.next().await.unwrap(),
        StreamFrame::Done(json!({"drained": true}))
    );
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("cleanup")).unwrap(),
        "complete"
    );
}

#[tokio::test]
async fn graceful_deadline_runs_without_polling_and_cannot_be_extended() {
    let fixture = Fixture::new("stall");
    let mut stream = fixture.binding.stream("launch", Value::Null).await.unwrap();
    let pid = fixture.pid().await;
    let shutdown = stream.shutdown_handle();
    shutdown.request(Duration::from_millis(40));
    shutdown.request(Duration::from_secs(60));
    sleep(Duration::from_millis(100)).await;
    let error = stream.next().await.unwrap_err();
    assert!(
        format!("{error:#}").contains("durability is unknown"),
        "{error:#}"
    );
    #[cfg(unix)]
    assert_reaped(pid).await;
    #[cfg(not(unix))]
    let _ = pid;
}

#[tokio::test]
async fn graceful_request_can_extend_cleanup_after_the_plugin_result() {
    let fixture = Fixture::new("graceful_late");
    let mut stream = fixture.binding.stream("launch", Value::Null).await.unwrap();
    timeout(Duration::from_secs(3), async {
        while !fixture.binding.state_dir.join("shutdown").exists() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    stream.shutdown_handle().request(Duration::from_secs(3));
    assert_eq!(
        stream.next().await.unwrap(),
        StreamFrame::Done(json!({"ok": true}))
    );
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("cleanup")).unwrap(),
        "complete"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn explicit_force_reaps_even_when_event_delivery_is_backpressured() {
    let fixture = Fixture::new("endless");
    let mut stream = fixture.binding.stream("launch", Value::Null).await.unwrap();
    let pid = fixture.pid().await;
    assert_eq!(
        stream.next().await.unwrap(),
        StreamFrame::Event(json!("tick"))
    );
    sleep(Duration::from_millis(50)).await;
    let shutdown = stream.shutdown_handle();
    shutdown.force_and_reap().await.unwrap();
    // An acknowledgement must mean wait completed, without relying on a later
    // runtime poll to run the fallback reaper after the caller returns.
    // SAFETY: signal zero only checks whether this process exists.
    assert_eq!(unsafe { libc::kill(pid as libc::pid_t, 0) }, -1);
    shutdown.force_and_reap().await.unwrap();
}

#[tokio::test]
async fn separate_invocations_keep_durable_state() {
    let fixture = Fixture::new("persistent");
    let first = fixture.binding.call("execute", Value::Null).await.unwrap();
    let second = fixture.binding.call("execute", Value::Null).await.unwrap();
    assert_eq!(
        (first["count"].clone(), second["count"].clone()),
        (json!(1), json!(2))
    );
    assert_ne!(first["pid"], second["pid"]);
}

#[tokio::test]
async fn sensitive_components_cannot_emit_inherited_stderr() {
    for kind in ["auth", "attachment_store"] {
        let mut fixture = Fixture::new("inspect_stderr");
        fixture.binding.spec.kind = kind.to_owned();
        assert_eq!(
            fixture.binding.call("resolve", Value::Null).await.unwrap(),
            json!({"stderr_is_null": true})
        );
    }
}

#[tokio::test]
async fn malformed_frames_and_plugin_failures_are_reported() {
    for (mode, expected) in [
        ("bad_version", "ready with api_version 1"),
        ("bad_id", "unexpected request id"),
        ("bad_kind", "unexpected component frame type"),
        ("missing_payload", "result payload missing"),
        ("error", "fixture failure"),
        ("invalid_json", "invalid component JSON frame"),
        ("oversized", "byte frame limit"),
        ("partial_eof", "unexpected EOF"),
        ("eof", "unexpected EOF"),
        ("bad_exit", "exited unsuccessfully"),
        ("shutdown_stall", "shutdown timed out"),
    ] {
        let fixture = Fixture::new(mode);
        let error = fixture
            .binding
            .call("execute", Value::Null)
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains(expected), "{mode}: {error:#}");
    }
}

#[tokio::test]
async fn oversized_requests_fail_without_waiting_for_a_response() {
    let fixture = Fixture::new("echo");
    let error = fixture
        .binding
        .call("execute", json!("x".repeat(super::MAX_FRAME_BYTES)))
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("request exceeds"));
}

#[cfg(unix)]
async fn assert_reaped(pid: u32) {
    timeout(Duration::from_secs(3), async {
        loop {
            // SAFETY: signal zero only checks whether this process still exists.
            if unsafe { libc::kill(pid as libc::pid_t, 0) } == -1 {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled plugin should be killed and reaped");
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_initialization_kills_and_reaps_child() {
    let fixture = Fixture::new("handshake_stall");
    let binding = fixture.binding.clone();
    let invocation = tokio::spawn(async move { binding.stream("execute", Value::Null).await });
    let pid = fixture.pid().await;
    invocation.abort();
    assert!(invocation.await.unwrap_err().is_cancelled());
    assert_reaped(pid).await;
}

#[cfg(unix)]
#[tokio::test]
async fn dropping_stream_and_cancelling_next_kill_and_reap_children() {
    let fixture = Fixture::new("stall");
    let stream = fixture
        .binding
        .stream("execute", Value::Null)
        .await
        .unwrap();
    let pid = fixture.pid().await;
    drop(stream);
    assert_reaped(pid).await;

    let fixture = Fixture::new("stall");
    let mut stream = fixture
        .binding
        .stream("execute", Value::Null)
        .await
        .unwrap();
    let pid = fixture.pid().await;
    assert!(
        timeout(Duration::from_millis(20), stream.next())
            .await
            .is_err()
    );
    assert_reaped(pid).await;
    assert!(stream.next().await.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn deadline_remains_active_while_consumer_is_idle() {
    let mut fixture = Fixture::new("endless");
    fixture.binding.timeout_ms = 200;
    let mut stream = fixture
        .binding
        .stream("execute", Value::Null)
        .await
        .unwrap();
    let pid = fixture.pid().await;
    sleep(Duration::from_millis(300)).await;
    assert_reaped(pid).await;
    let mut events = 0;
    loop {
        match stream.next().await {
            Ok(StreamFrame::Event(_)) => events += 1,
            Ok(StreamFrame::Done(_)) => panic!("endless plugin should time out"),
            Err(error) => {
                assert!(format!("{error:#}").contains("timed out"));
                break;
            }
        }
    }
    assert_eq!(events, 1);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancellation_terminates_plugin_process_group() {
    let fixture = Fixture::new("descendant");
    let mut stream = fixture
        .binding
        .stream("execute", Value::Null)
        .await
        .unwrap();
    let descendant = match stream.next().await.unwrap() {
        StreamFrame::Event(pid) => pid.as_u64().unwrap(),
        StreamFrame::Done(_) => panic!("expected descendant pid"),
    };
    let pid = fixture.pid().await;
    drop(stream);
    assert_reaped(pid).await;
    timeout(Duration::from_secs(3), async {
        loop {
            match tokio::fs::read_to_string(format!("/proc/{descendant}/stat")).await {
                // Grandchildren are reaped by their reparented owner; ensure they
                // are terminated even if container PID 1 delays that reap.
                Ok(stat) if stat.rsplit_once(") ").unwrap().1.starts_with('Z') => return,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
                Ok(_) | Err(_) => sleep(Duration::from_millis(10)).await,
            }
        }
    })
    .await
    .expect("plugin descendant should be terminated");
}
