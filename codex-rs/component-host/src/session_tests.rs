#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;

use crate::ComponentBinding;
use crate::ComponentSpec;

const SERVICE: &str = r#"
import base64, json, os, pathlib, sys, threading, time

initialize = json.loads(sys.stdin.readline())
state = pathlib.Path(initialize['state_dir'])
state.joinpath('pid').write_text(str(os.getpid()))
send_lock = threading.Lock()
state_lock = threading.Lock()
workers = []
leases = {}
assemblies = {}

def frame(value):
    with send_lock:
        print(json.dumps(value), flush=True)

def result(request, value):
    body = json.dumps(value).encode()
    ident = request['id'] + (1 if request['method'] == 'bad_id' else 0)
    frame({'type': 'result_start', 'id': ident, 'bytes': len(body)})
    count = 0
    for offset in range(0, len(body), 196608):
        frame({'type': 'chunk', 'id': ident, 'index': count, 'data': base64.b64encode(body[offset:offset+196608]).decode()})
        count += 1
    frame({'type': 'end', 'id': ident, 'chunks': count})

def wait_gate(name):
    while not state.joinpath(name).exists():
        time.sleep(0.005)

def handle(request, params):
    method = request['method']
    with state_lock:
        with state.joinpath('accepted').open('a') as record:
            record.write(method + '\n')
    if method == 'crash':
        os._exit(23)
    if method == 'wait':
        wait_gate('gate')
        with state_lock:
            with state.joinpath('committed').open('a') as record:
                record.write(str(request['id']) + '\n')
    elif method == 'prepare':
        with state_lock:
            event = leases.setdefault(params['lease'], threading.Event())
        state.joinpath('prepare_started').write_text('yes')
        if params.get('wait'):
            wait_gate('prepare_gate')
    elif method == 'release':
        with state_lock:
            event = leases.setdefault(params['lease'], threading.Event())
            event.set()
        state.joinpath('released').write_text('yes')
    elif method == 'delete':
        state.joinpath('delete_started').write_text('yes')
        with state_lock:
            event = leases.setdefault(params['lease'], threading.Event())
        event.wait()
    result(request, params)

ready = {'type': 'ready', 'api_version': 1, 'session': {'mode': 'multiplexed', 'version': 1}}
if initialize['config'].get('old_protocol'):
    del ready['session']
frame(ready)
for line in sys.stdin:
    message = json.loads(line)
    kind = message['type']
    if kind == 'request_start':
        assemblies[message['id']] = (message, bytearray())
    elif kind == 'chunk':
        assemblies[message['id']][1].extend(base64.b64decode(message['data']))
    elif kind == 'end':
        request, body = assemblies.pop(message['id'])
        worker = threading.Thread(target=handle, args=(request, json.loads(body)))
        worker.start()
        workers.append(worker)
    elif kind == 'shutdown':
        for worker in workers:
            worker.join()
        state.joinpath('shutdown').write_text('graceful')
        frame({'type': 'shutdown_complete'})
        break
"#;

struct Fixture {
    _root: TempDir,
    binding: ComponentBinding,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("service.py");
        std::fs::write(&script, SERVICE).unwrap();
        let python = ["python3", "python"]
            .into_iter()
            .find(|program| {
                std::process::Command::new(program)
                    .arg("--version")
                    .output()
                    .is_ok_and(|output| output.status.success())
            })
            .expect("persistent component tests require Python 3");
        let binding = ComponentBinding {
            plugin_id: "persistent.fixture".to_owned(),
            spec: ComponentSpec {
                kind: "thread_store".to_owned(),
                name: "default".to_owned(),
                contract_version: crate::THREAD_STORE_CONTRACT_VERSION,
                metadata: Value::Null,
            },
            config: json!({}),
            package_dir: root.path().to_path_buf(),
            state_dir: root.path().join("state"),
            entrypoint: PathBuf::from(python),
            args: vec![script.to_string_lossy().into_owned()],
            timeout_ms: 15_000,
        };
        Self {
            _root: root,
            binding,
        }
    }

    async fn wait_for(&self, file: &str, count: usize) {
        timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(contents) =
                    tokio::fs::read_to_string(self.binding.state_dir.join(file)).await
                    && contents.lines().count() >= count
                {
                    return;
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    }

    fn open_gate(&self, name: &str) {
        std::fs::write(self.binding.state_dir.join(name), "open").unwrap();
    }
}

#[tokio::test]
async fn persistent_process_multiplexes_calls_and_large_payloads() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let first = session.clone();
    let waiting = tokio::spawn(async move { first.call("wait", json!({"delayed":true})).await });
    fixture.wait_for("accepted", 1).await;
    let large = json!({"history": "x".repeat(5 * 1024 * 1024)});
    assert_eq!(session.call("echo", large.clone()).await.unwrap(), large);
    fixture.open_gate("gate");
    assert_eq!(waiting.await.unwrap().unwrap(), json!({"delayed":true}));
    session.close().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("shutdown")).unwrap(),
        "graceful"
    );
}

#[tokio::test]
async fn cancelled_waiter_keeps_write_owned_and_close_drains_it() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let caller = session.clone();
    let call = tokio::spawn(async move { caller.call("wait", json!({})).await });
    fixture.wait_for("accepted", 1).await;
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    assert_eq!(
        session.call("echo", json!("still-alive")).await.unwrap(),
        json!("still-alive")
    );
    let closer = session.clone();
    let mut closing = tokio::spawn(async move { closer.close().await });
    assert!(
        timeout(Duration::from_millis(30), &mut closing)
            .await
            .is_err()
    );
    fixture.open_gate("gate");
    closing.await.unwrap().unwrap();
    fixture.wait_for("committed", 1).await;
    assert!(session.call("echo", json!({})).await.is_err());
}

#[tokio::test]
async fn synchronous_close_fences_retained_clones_and_drains_without_a_close_waiter() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let retained = session.clone();
    let caller = session.clone();
    let pending = tokio::spawn(async move { caller.call("wait", json!({})).await });
    fixture.wait_for("accepted", 1).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());

    session.begin_close();
    session.begin_close();
    assert!(retained.call("echo", json!({})).await.is_err());
    drop(session);
    fixture.open_gate("gate");
    fixture.wait_for("committed", 1).await;
    fixture.wait_for("shutdown", 1).await;
    retained.close().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("accepted")).unwrap(),
        "wait\n"
    );
}

#[tokio::test]
async fn timed_out_waiter_does_not_kill_healthy_service_or_replay_write() {
    let mut fixture = Fixture::new();
    fixture.binding.timeout_ms = 250;
    let session = fixture.binding.connect().await.unwrap();
    let error = session.call("wait", json!({})).await.unwrap_err();
    assert!(format!("{error:#}").contains("outcome is unknown"));
    assert_eq!(session.call("echo", json!(42)).await.unwrap(), json!(42));
    fixture.open_gate("gate");
    session.close().await.unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("committed"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[tokio::test]
async fn fork_cleanup_bypasses_blocked_deletion_and_covers_cancelled_prepare() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let (_, lease) = session
        .call_with_cleanup(
            "prepare",
            json!({"lease":"first"}),
            "release",
            json!({"lease":"first"}),
        )
        .await
        .unwrap();
    let deleting = session.clone();
    let delete =
        tokio::spawn(async move { deleting.call("delete", json!({"lease":"first"})).await });
    fixture.wait_for("delete_started", 1).await;
    drop(lease);
    assert_eq!(delete.await.unwrap().unwrap(), json!({"lease":"first"}));

    std::fs::remove_file(fixture.binding.state_dir.join("prepare_started")).unwrap();
    std::fs::remove_file(fixture.binding.state_dir.join("released")).unwrap();
    let preparing = session.clone();
    let prepare = tokio::spawn(async move {
        preparing
            .call_with_cleanup(
                "prepare",
                json!({"lease":"second","wait":true}),
                "release",
                json!({"lease":"second"}),
            )
            .await
    });
    fixture.wait_for("prepare_started", 1).await;
    prepare.abort();
    assert!(prepare.await.unwrap_err().is_cancelled());
    fixture.wait_for("released", 1).await;
    fixture.open_gate("prepare_gate");
    session.close().await.unwrap();
}

#[tokio::test]
async fn abandoned_waiters_still_occupy_bounded_admission_until_completed() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let mut calls = Vec::new();
    for _ in 0..super::CALL_SLOTS {
        let caller = session.clone();
        calls.push(tokio::spawn(
            async move { caller.call("wait", json!({})).await },
        ));
    }
    fixture.wait_for("accepted", super::CALL_SLOTS).await;
    for call in calls {
        call.abort();
        assert!(call.await.unwrap_err().is_cancelled());
    }
    assert!(
        timeout(Duration::from_millis(30), session.call("echo", json!({})))
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(fixture.binding.state_dir.join("accepted"))
            .unwrap()
            .lines()
            .count(),
        super::CALL_SLOTS
    );
    fixture.open_gate("gate");
    assert_eq!(
        session
            .call("echo", json!("capacity-released"))
            .await
            .unwrap(),
        json!("capacity-released")
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn peer_failure_is_terminal_without_automatic_restart() {
    for method in ["crash", "bad_id"] {
        let fixture = Fixture::new();
        let session = fixture.binding.connect().await.unwrap();
        let pid = std::fs::read_to_string(fixture.binding.state_dir.join("pid")).unwrap();
        assert!(session.call(method, json!({})).await.is_err());
        assert!(session.call("echo", json!({})).await.is_err());
        assert!(session.close().await.is_err());
        assert_eq!(
            std::fs::read_to_string(fixture.binding.state_dir.join("pid")).unwrap(),
            pid
        );
    }
}

#[tokio::test]
async fn old_single_invocation_plugins_cannot_opt_in_accidentally() {
    let mut fixture = Fixture::new();
    fixture.binding.config = json!({"old_protocol":true});
    let error = fixture.binding.connect().await.unwrap_err();
    assert!(format!("{error:#}").contains("does not support multiplexed"));
}

#[tokio::test]
async fn invalid_methods_are_rejected_without_disrupting_accepted_calls() {
    let fixture = Fixture::new();
    let session = fixture.binding.connect().await.unwrap();
    let caller = session.clone();
    let pending = tokio::spawn(async move { caller.call("wait", json!({})).await });
    fixture.wait_for("accepted", 1).await;
    assert!(session.call("", json!({})).await.is_err());
    assert!(
        session
            .call_with_cleanup("prepare", json!({}), "", json!({}))
            .await
            .is_err()
    );
    assert_eq!(
        session.call("echo", json!("healthy")).await.unwrap(),
        json!("healthy")
    );
    fixture.open_gate("gate");
    pending.await.unwrap().unwrap();
    session.close().await.unwrap();
}

#[path = "session_reaping_tests.rs"]
mod reaping;
