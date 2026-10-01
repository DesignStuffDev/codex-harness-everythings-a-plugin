#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Adversarial protocol peer, not a native-search implementation. These tests
//! prove the process adapter's ownership and validation over real stdio.

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codex_component_host::ComponentBinding;
use codex_component_host::ComponentSpec;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchOpen;
use serde_json::Value;
use tempfile::TempDir;

use crate::FILE_SEARCH_CONTRACT_VERSION;
use crate::InitializeRequest;
use crate::ProcessSearchBackend;
use crate::ProviderIdentity;
use crate::ServiceLimits;

pub(super) struct Fixture {
    _root: TempDir,
    pub binding: ComponentBinding,
    pub initialization: InitializeRequest,
}

impl Fixture {
    pub fn new(config: Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("service.py");
        std::fs::write(&script, SCRIPT).unwrap();
        let python = std::process::Command::new("python3")
            .args(["-c", "import sys; print(sys.executable)"])
            .output()
            .expect("process tests require Python 3");
        assert!(python.status.success());
        let entrypoint = PathBuf::from(String::from_utf8(python.stdout).unwrap().trim());
        let base_dir = root.path().join("search-context");
        std::fs::create_dir(&base_dir).unwrap();
        let binding = ComponentBinding {
            plugin_id: "search.fixture".into(),
            spec: ComponentSpec {
                service_requirements: None,
                kind: "file_search".into(),
                name: "default".into(),
                contract_version: 1,
                metadata: Value::Null,
            },
            config,
            package_dir: root.path().to_owned(),
            state_dir: root.path().join("state"),
            entrypoint,
            args: vec![script.to_str().unwrap().to_owned()],
            timeout_ms: 10_000,
        };
        let initialization = InitializeRequest {
            contract_version: FILE_SEARCH_CONTRACT_VERSION,
            identity: ProviderIdentity {
                provider_id: uuid::Uuid::new_v4().to_string(),
            },
            base_dir,
            requested_limits: ServiceLimits::new(SearchBudget {
                max_index_entries: NonZeroUsize::new(160).unwrap(),
                max_index_bytes: NonZeroUsize::new(160_000).unwrap(),
                max_worker_threads: NonZeroUsize::new(160).unwrap(),
            }),
        };
        Self {
            _root: root,
            binding,
            initialization,
        }
    }

    pub async fn connect(&self) -> Arc<ProcessSearchBackend> {
        ProcessSearchBackend::connect(self.binding.clone(), self.initialization.clone())
            .await
            .unwrap()
    }

    pub fn request(&self) -> SearchOpen {
        SearchOpen {
            roots: vec![PathBuf::from(".")],
            options: FileSearchOptions::default(),
            budget: SearchBudget {
                max_index_entries: NonZeroUsize::new(10).unwrap(),
                max_index_bytes: NonZeroUsize::new(10_000).unwrap(),
                max_worker_threads: NonZeroUsize::new(10).unwrap(),
            },
        }
    }

    pub async fn wait(&self, marker: &str, count: usize) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if self.records(marker).lines().count() >= count {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    }

    pub fn records(&self, marker: &str) -> String {
        std::fs::read_to_string(self.binding.state_dir.join(marker)).unwrap_or_default()
    }

    pub fn gate(&self, marker: &str) {
        std::fs::write(self.binding.state_dir.join(marker), "open").unwrap();
    }

    pub fn assert_reaped(&self) {
        assert_eq!(self.records("shutdown"), "graceful");
        #[cfg(unix)]
        {
            let pid: libc::pid_t = self.records("pid").parse().unwrap();
            // SAFETY: signal zero probes this fixture's exact child process.
            assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
        }
    }
}

const SCRIPT: &str = r#"
import base64, json, os, pathlib, sys, threading, time
init = json.loads(sys.stdin.readline())
state = pathlib.Path(init['state_dir'])
config = init['config']
state.joinpath('pid').write_text(str(os.getpid()))
state.joinpath('cwd').write_text(os.getcwd())
send_lock = threading.Lock()
lock = threading.RLock()
assemblies, leases, workers = {}, {}, []
limits = None

def record(name, text='yes'):
    with lock:
        with state.joinpath(name).open('a') as out: out.write(text + '\n')

def frame(value):
    with send_lock: print(json.dumps(value), flush=True)

def reply(request, params, value, error=None):
    identity = dict(params['identity'])
    if config.get('bad_identity') and request['method'].endswith('next_snapshot'):
        identity['session_epoch'] = '999'
    outcome = {'status':'ok','result':value} if error is None else {'status':'error','error':error}
    envelope = {'contract_version':1,'method':request['method'],'identity':identity,'reply':outcome}
    body = json.dumps(envelope).encode()
    frame({'type':'result_start','id':request['id'],'bytes':len(body)})
    chunks = 0
    for offset in range(0,len(body),196608):
        frame({'type':'chunk','id':request['id'],'index':chunks,'data':base64.b64encode(body[offset:offset+196608]).decode()})
        chunks += 1
    frame({'type':'end','id':request['id'],'chunks':chunks})

def wait_gate(name, stop=None):
    while not state.joinpath(name).exists() and not (stop and stop.is_set()): time.sleep(0.005)

def close_outcome(unconfirmed=False):
    operation = {'status':'ok'}
    if config.get('operation_error'):
        operation = {'status':'error','error':{'kind':'search_failed','message':'original search failure'}}
    if config.get('resource_error'):
        operation = {'status':'error','error':{'kind':'resource_exhausted','message':'original resource exhaustion'}}
    cleanup = {'status':'joined'}
    if unconfirmed:
        cleanup = {'status':'unconfirmed','error':{'kind':'transport_lost','message':'cleanup proof unavailable'}}
    return {'operation':operation,'cleanup':cleanup}

def handle(request, params, lease):
    global limits
    method = request['method'].split('/')[-1]
    if method == 'initialize':
        limits = params['requested_limits']
        reply(request,params,{'limits':limits,'native_path_platform':'windows_wide' if os.name=='nt' else 'unix_bytes','encoding':'file_search1'})
    elif method == 'open':
        if config.get('hold_open') or config.get('hold_open_epoch') == params['identity']['session_epoch']:
            wait_gate('open_gate',lease['stop'])
        lease_limits = {key:limits[key] for key in ['max_query_utf8_bytes','max_matches','max_frame_bytes','max_poll_wait_ms']}
        lease_limits.update(max_pending_polls=1,max_in_flight_updates=1)
        if config.get('open_error'):
            error = {'operation':{'kind':'closed_lease' if config.get('open_closed_error') else 'search_failed','message':'earlier startup failure'},'cleanup':{'status':'not_admitted' if config.get('open_not_admitted') else 'confirmed'}}
            reply(request,params,None,error)
        else:
            reply(request,params,{'initial_cursor':'0','limits':lease_limits,'budget':params['budget']})
        lease['open_done'].set()
    elif method == 'update_query':
        if config.get('hold_update'): wait_gate('update_gate',lease['stop'])
        with lock:
            lease['query'] = params['query']
            lease['query_id'] = params['query_epoch']
        reply(request,params,{'accepted_query_epoch':params['query_epoch']})
        lease['update_done'].set()
        record('update_done')
    elif method == 'next_snapshot':
        if config.get('hold_poll'): wait_gate('poll_gate',lease['stop'])
        with lock:
            query, query_id = lease['query'], lease['query_id']
        snapshot = {'matches':[{'score':7,'root_index':0,'path':{'UnixBytes':list(b'file.txt')} if os.name!='nt' else {'WindowsWide':[ord(c) for c in 'file.txt']},'match_type':'file','indices':None}],'total_match_count':'1','scanned_file_count':'1','walk_complete':True}
        if config.get('old_query'): query,query_id = '', '0'
        result = {'status':'changed','frame':{'revision':str(int(params['after_revision'])+1),'query_epoch':query_id,'query':query,'phase':{'state':'idle'},'snapshot':snapshot}}
        reply(request,params,result)
        lease['poll_done'].set()
        record('poll_done')
    elif method == 'release':
        lease['stop'].set()
        record('released_epochs',params['identity']['session_epoch'])
        record('release_started')
        for event in ['open_done','update_done','poll_done']: lease[event].wait()
        if config.get('hold_release'): wait_gate('release_gate')
        reply(request,params,close_outcome(config.get('unconfirmed',False)))
        record('released')
    elif method == 'shutdown':
        with lock: owned = list(leases.values())
        for item in owned: item['stop'].set()
        for item in owned:
            for event in ['open_done','update_done','poll_done']: item[event].wait()
        reply(request,params,close_outcome())

frame({'type':'ready','api_version':1,'session':{'mode':'multiplexed','version':1}})
for line in sys.stdin:
    message = json.loads(line)
    if message['type'] == 'request_start': assemblies[message['id']] = (message,bytearray())
    elif message['type'] == 'chunk': assemblies[message['id']][1].extend(base64.b64decode(message['data']))
    elif message['type'] == 'end':
        request,body = assemblies.pop(message['id'])
        params = json.loads(body)
        method = request['method'].split('/')[-1]
        lease = None
        with lock:
            if method == 'open':
                lease = {name:threading.Event() for name in ['stop','open_done','update_done','poll_done']}
                lease['update_done'].set(); lease['poll_done'].set()
                lease.update(query='',query_id='0',identity=dict(params['identity']))
                leases[params['identity']['lease_id']] = lease
                record('epochs',params['identity']['session_epoch'])
            elif 'lease_id' in params['identity']:
                lease = leases.get(params['identity']['lease_id'])
                if lease is None or lease['identity'] != params['identity']:
                    reply(request,params,None,{'kind':'invalid_input','message':'fixture lease identity mismatch'})
                    record('identity_rejected')
                    continue
            # Only operations admitted before the close fence may drain normally.
            # An update to a wrongly cancelled sibling must not appear successful.
            if method in ['update_query','next_snapshot'] and lease['stop'].is_set():
                reply(request,params,None,{'kind':'closed_lease','message':'fixture lease is closed'})
                record('late_operation_rejected')
                continue
            if method == 'update_query': lease['update_done'].clear()
            if method == 'next_snapshot': lease['poll_done'].clear()
            record(method)
            worker = threading.Thread(target=handle,args=(request,params,lease))
            workers.append(worker)
            worker.start()
    elif message['type'] == 'shutdown':
        for worker in workers: worker.join()
        state.joinpath('shutdown').write_text('graceful')
        frame({'type':'shutdown_complete'})
        break
"#;
