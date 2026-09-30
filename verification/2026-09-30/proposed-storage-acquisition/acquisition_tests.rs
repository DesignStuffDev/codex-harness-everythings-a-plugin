//! Delay real native storage replies after the native writer has been acquired.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_thread_store::AppendThreadItemsParams;
use codex_thread_store::LiveThread;
use codex_thread_store::LiveThreadInitGuard;
use codex_thread_store::LoadThreadHistoryParams;
use codex_thread_store::PersistContext;
use codex_thread_store::ResumeThreadParams;
use codex_thread_store::ThreadStore;
use codex_thread_store_component::ProcessThreadStore;

use crate::support::Fixture;
use crate::support::message;

const PROXY: &str = r#"
import base64, json, pathlib, subprocess, sys, threading, time
native, mode, operation, directory = sys.argv[1:]
state = pathlib.Path(directory)
child = subprocess.Popen([native], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
target = None
assemblies = {}
send_lock = threading.Lock()
replies = []
def emit(lines):
    with send_lock:
        sys.stdout.writelines(lines)
        sys.stdout.flush()
def delayed(lines):
    while not state.joinpath('release').exists():
        time.sleep(0.005)
    emit(lines)
def requests():
    global target
    for line in sys.stdin:
        value = json.loads(line)
        if value['type'] == 'request_start':
            assemblies[value['id']] = (value, bytearray())
        elif value['type'] == 'chunk':
            assemblies[value['id']][1].extend(base64.b64decode(value['data']))
        elif value['type'] == 'end':
            request, body = assemblies.pop(value['id'])
            if request['method'] == 'thread_store/call' and json.loads(body)['operation'] == operation:
                with state.joinpath('requests').open('a') as log:
                    log.write(operation + '\n')
                if target is None:
                    target = value['id']
        child.stdin.write(line)
        child.stdin.flush()
    child.stdin.close()
threading.Thread(target=requests, daemon=True).start()
held = []
for line in child.stdout:
    value = json.loads(line)
    if value.get('id') == target and target is not None:
        held.append(line)
        if value['type'] != 'end':
            continue
        state.joinpath('native_completed').write_text('writer acquired')
        if mode == 'delay':
            reply = threading.Thread(target=delayed, args=(held,))
            reply.start()
            replies.append(reply)
        else:
            body = json.dumps({'status':'ok','value':{'operation':'flush_thread','value':None}}).encode()
            for replacement in [
                {'type':'result_start','id':target,'bytes':len(body)},
                {'type':'chunk','id':target,'index':0,'data':base64.b64encode(body).decode()},
                {'type':'end','id':target,'chunks':1},
            ]:
                emit([json.dumps(replacement) + '\n'])
        held = []
        target = None
    else:
        emit([line])
code = child.wait()
for reply in replies:
    reply.join()
state.joinpath('exited').write_text(str(code))
sys.exit(code)
"#;

struct DelayedReply {
    fixture: Fixture,
    binding: codex_component_host::ComponentBinding,
    state: PathBuf,
}

impl DelayedReply {
    fn new(mode: &str, operation: &str) -> Result<Self> {
        let fixture = Fixture::install();
        let state = fixture._root.path().join("proxy-state");
        std::fs::create_dir(&state)?;
        let script = fixture._root.path().join("proxy.py");
        std::fs::write(&script, PROXY)?;
        let python = ["python3", "python"].into_iter().find(|program| {
            std::process::Command::new(program).arg("--version").output()
                .is_ok_and(|output| output.status.success())
        }).context("native storage proxy tests require Python 3")?;
        let mut binding = fixture.binding.clone();
        let native = binding.package_dir.join(&binding.entrypoint);
        binding.entrypoint = PathBuf::from(python);
        binding.args = vec![script.to_string_lossy().into_owned(), native.to_string_lossy().into_owned(),
            mode.to_owned(), operation.to_owned(), state.to_string_lossy().into_owned()];
        binding.timeout_ms = 2_000;
        Ok(Self { fixture, binding, state })
    }

    async fn open(&self) -> Result<Arc<ProcessThreadStore>> {
        Ok(Arc::new(ProcessThreadStore::connect(self.binding.clone(), self.fixture.initialization.clone()).await?))
    }

    async fn wait_for_native_acquisition(&self) -> Result<()> {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !self.state.join("native_completed").exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }).await.context("native acquisition response")?;
        Ok(())
    }
}

#[tokio::test]
async fn late_native_acquisition_times_out_fences_clones_and_releases_writer() -> Result<()> {
    for operation in ["create_thread", "resume_thread"] {
        let proxy = DelayedReply::new("delay", operation)?;
        let store = proxy.open().await?;
        let retained = Arc::clone(&store);
        let id = ThreadId::new();
        let params = proxy.fixture.params(id, ThreadHistoryMode::Legacy);
        let resume = if operation == "resume_thread" {
            store.create_thread(params.clone()).await?;
            store.append_items(AppendThreadItemsParams { thread_id: id, items: vec![message("durable seed")] }).await?;
            store.persist_thread(id, PersistContext::Standard).await?;
            store.shutdown_thread(id).await?;
            Some(ResumeThreadParams {
                thread_id: id, rollout_path: None, include_archived: true,
                history: Some(Arc::new(store.load_history(LoadThreadHistoryParams { thread_id: id, include_archived: true }).await?.items)),
                metadata: params.metadata.clone(),
            })
        } else { None };
        let resumed = resume.clone();
        let creating = params.clone();
        let owned_store: Arc<dyn ThreadStore> = store.clone();
        let acquiring = tokio::spawn(async move {
            let mut owner = LiveThreadInitGuard::default();
            owner.acquire(async move {
                match resumed {
                    Some(params) => LiveThread::resume(owned_store, ThreadHistoryMode::Legacy, params).await,
                    None => LiveThread::create(owned_store, creating).await,
                }
            }).await
        });
        proxy.wait_for_native_acquisition().await?;
        let result = tokio::time::timeout(Duration::from_secs(4), acquiring).await??;
        let error = result.err().context("acquisition must honor its configured deadline")?;
        let retry = retained.read_pending_thread_metadata(id).await;
        std::fs::write(proxy.state.join("release"), "continue")?;
        tokio::time::timeout(Duration::from_secs(3), store.shutdown_store()).await??;
        // A separate recovered process can acquire the writer only after the
        // previous process has drained. Neither timed-out operation is replayed.
        let recovered = proxy.fixture.open().await;
        match resume {
            Some(params) => recovered.resume_thread(params).await?,
            None => recovered.create_thread(params).await?,
        }
        recovered.discard_thread(id).await?;
        recovered.shutdown_store().await?;
        assert!(retry.is_err(), "a timed-out acquisition must fence every retained clone");
        assert!(error.to_string().contains("selected store is closing"), "{error}");
        assert_eq!(std::fs::read_to_string(proxy.state.join("requests"))?.lines().count(), 1);
        assert_eq!(std::fs::read_to_string(proxy.state.join("exited"))?, "0");
    }
    Ok(())
}

#[tokio::test]
async fn mismatched_native_acquisition_reply_fences_store_and_releases_writer() -> Result<()> {
    let proxy = DelayedReply::new("wrong_reply", "create_thread")?;
    let store = proxy.open().await?;
    let id = ThreadId::new();
    let params = proxy.fixture.params(id, ThreadHistoryMode::Legacy);
    let error = store.create_thread(params.clone()).await.err().context("mismatched reply accepted")?;
    let retry = store.read_pending_thread_metadata(id).await;
    store.shutdown_store().await?;
    let recovered = proxy.fixture.open().await;
    recovered.create_thread(params).await?;
    recovered.discard_thread(id).await?;
    recovered.shutdown_store().await?;
    assert!(retry.is_err(), "unknown acquisition result must close this store");
    assert!(error.to_string().contains("selected store is closing"), "{error}");
    Ok(())
}

#[tokio::test]
async fn confirmed_native_acquisition_errors_leave_store_usable() -> Result<()> {
    let fixture = Fixture::install();
    let store = fixture.open().await;
    let id = ThreadId::new();
    let params = fixture.params(id, ThreadHistoryMode::Legacy);
    store.create_thread(params.clone()).await?;
    assert!(store.create_thread(params.clone()).await.is_err());
    assert!(store.resume_thread(ResumeThreadParams {
        thread_id: id, rollout_path: None, history: Some(Arc::new(Vec::new())),
        include_archived: true, metadata: params.metadata,
    }).await.is_err());
    let other = ThreadId::new();
    store.create_thread(fixture.params(other, ThreadHistoryMode::Legacy)).await?;
    store.discard_thread(id).await?;
    store.discard_thread(other).await?;
    store.shutdown_store().await?;
    Ok(())
}
