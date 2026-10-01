use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use codex_component_host::ComponentBinding;
use codex_component_host::ComponentSpec;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreShutdownGuard;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

use super::ProcessThreadStore;
use crate::LocalStoragePaths;
use crate::StorageCapabilities;
use crate::StorageInitialization;

const SERVICE: &str = r#"
import base64, json, pathlib, sys, threading, time
initial = json.loads(sys.stdin.readline())
state = pathlib.Path(initial['state_dir'])
config = initial['config']
workers = []
assemblies = {}
lock = threading.Lock()
def frame(value):
    with lock:
        print(json.dumps(value), flush=True)
def wait_for(name):
    while not state.joinpath(name).exists():
        time.sleep(0.005)
def open_store(request):
    state.joinpath('open_started').write_text('yes')
    if config['mode'] == 'wait_open':
        wait_for('open_gate')
    if config['mode'] == 'open_error':
        frame({'type':'error', 'id':request['id'], 'message':'fixture refused open'})
        return
    value = config['capabilities']
    if config['mode'] == 'migration_absent':
        value.pop('manual_rollout_migration', None)
    if config['mode'] == 'migration_future':
        value['manual_rollout_migration'] = 99
    if config['mode'] == 'decode':
        value = {}
    if config['mode'] == 'contract':
        value['contract_version'] = 9
    if config['mode'] == 'paths':
        value['shared_local_sqlite']['codex_home'] = config['wrong_home']
    body = json.dumps(value).encode()
    frame({'type':'result_start', 'id':request['id'], 'bytes':len(body)})
    frame({'type':'chunk', 'id':request['id'], 'index':0, 'data':base64.b64encode(body).decode()})
    frame({'type':'end', 'id':request['id'], 'chunks':1})
frame({'type':'ready', 'api_version':1, 'session':{'mode':'multiplexed','version':1}})
for line in sys.stdin:
    value = json.loads(line)
    if value['type'] == 'request_start':
        assemblies[value['id']] = value
    elif value['type'] == 'end':
        request = assemblies.pop(value['id'])
        assert request['method'] == 'thread_store/open'
        worker = threading.Thread(target=open_store, args=(request,))
        worker.start()
        workers.append(worker)
    elif value['type'] == 'shutdown':
        state.joinpath('shutdown_started').write_text('yes')
        wait_for('shutdown_gate')
        for worker in workers:
            worker.join()
        frame({'type':'shutdown_complete'})
        state.joinpath('exited').write_text('yes')
        break
"#;

struct Fixture {
    _root: TempDir,
    binding: ComponentBinding,
    initialization: StorageInitialization,
}

impl Fixture {
    fn new(mode: &str) -> Result<Self> {
        let root = tempfile::tempdir()?;
        let script = root.path().join("service.py");
        std::fs::write(&script, SERVICE)?;
        let python = ["python3", "python"]
            .into_iter()
            .find(|program| {
                std::process::Command::new(program)
                    .arg("--version")
                    .output()
                    .is_ok_and(|output| output.status.success())
            })
            .context("adapter process tests require Python 3")?;
        let paths = LocalStoragePaths {
            codex_home: root.path().join("home"),
            sqlite_home: root.path().join("sqlite"),
        };
        let capabilities = StorageCapabilities {
            contract_version: 2,
            default_history_mode: ThreadHistoryMode::Paginated,
            thread_sections: false,
            thread_attachments: false,
            projects: false,
            paginated_history_lists: true,
            rollout_maintenance: false,
            manual_rollout_migration: None,
            rollout_path_reads: false,
            shared_local_sqlite: paths.clone(),
        };
        let binding = ComponentBinding {
            plugin_id: "storage.lifecycle".to_owned(),
            spec: ComponentSpec {
                kind: "thread_store".to_owned(),
                name: "default".to_owned(),
                contract_version: 2,
                metadata: Value::Null,
            },
            config: json!({"mode":mode, "capabilities":capabilities, "wrong_home": codex_component_state_codec::native_path::Borrowed(&root.path().join("wrong-home"))}),
            package_dir: root.path().to_path_buf(),
            state_dir: root.path().join("state"),
            entrypoint: PathBuf::from(python),
            args: vec![script.to_string_lossy().into_owned()],
            timeout_ms: 5_000,
        };
        let initialization = StorageInitialization {
            contract_version: 2,
            paths,
            default_model_provider_id: "fixture".to_owned(),
            state_db_enabled: false,
            startup_migration: false,
            startup_compression: false,
        };
        Ok(Self {
            _root: root,
            binding,
            initialization,
        })
    }

    fn connect(&self) -> tokio::task::JoinHandle<Result<ProcessThreadStore>> {
        let binding = self.binding.clone();
        let initialization = self.initialization.clone();
        tokio::spawn(async move { ProcessThreadStore::connect(binding, initialization).await })
    }

    async fn wait_for(&self, name: &str) -> Result<()> {
        tokio::time::timeout(Duration::from_secs(3), async {
            while !self.binding.state_dir.join(name).exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .with_context(|| format!("wait for storage fixture {name}"))?;
        Ok(())
    }

    fn release(&self, name: &str) -> Result<()> {
        std::fs::write(self.binding.state_dir.join(name), "continue")?;
        Ok(())
    }
}

#[tokio::test]
async fn invalid_open_waits_for_service_cleanup_before_returning() -> Result<()> {
    for (mode, expected) in [
        ("decode", "missing field"),
        ("contract", "unsupported thread-store service contract"),
        ("paths", "does not preserve the configured host state paths"),
        ("open_error", "fixture refused open"),
    ] {
        let fixture = Fixture::new(mode)?;
        let connecting = fixture.connect();
        fixture.wait_for("shutdown_started").await?;
        assert!(
            !connecting.is_finished(),
            "initialization must await the cleanup acknowledgement"
        );
        fixture.release("shutdown_gate")?;
        let error = connecting
            .await?
            .err()
            .context("invalid initialization succeeded")?;
        assert!(format!("{error:#}").contains(expected), "{mode}: {error:#}");
        assert!(fixture.binding.state_dir.join("exited").exists());
    }
    Ok(())
}

#[tokio::test]
async fn cancelled_connect_still_drains_accepted_open() -> Result<()> {
    let fixture = Fixture::new("wait_open")?;
    let connecting = fixture.connect();
    fixture.wait_for("open_started").await?;
    connecting.abort();
    assert!(
        connecting
            .await
            .err()
            .context("cancelled connect completed")?
            .is_cancelled()
    );
    fixture.release("open_gate")?;
    fixture.wait_for("shutdown_started").await?;
    fixture.release("shutdown_gate")?;
    fixture.wait_for("exited").await?;
    Ok(())
}

#[tokio::test]
async fn initialization_cleanup_failure_preserves_both_errors() -> Result<()> {
    let mut fixture = Fixture::new("contract")?;
    fixture.binding.timeout_ms = 1_000;
    let error = fixture
        .connect()
        .await?
        .err()
        .context("invalid initialization succeeded")?;
    let error = format!("{error:#}");
    assert!(
        error.contains("unsupported thread-store service contract"),
        "{error}"
    );
    assert!(
        error.contains("storage initialization cleanup failed"),
        "{error}"
    );
    assert!(error.contains("outcomes are unknown"), "{error}");
    Ok(())
}

#[tokio::test]
async fn shutdown_owner_closes_process_with_retained_store_and_unpolled_future() -> Result<()> {
    for begin_explicitly in [false, true] {
        let fixture = Fixture::new("valid")?;
        let retained: Arc<dyn ThreadStore> = Arc::new(fixture.connect().await??);
        let owner = ThreadStoreShutdownGuard::new(Arc::clone(&retained));
        if begin_explicitly {
            drop(owner.shutdown());
            // The signal precedes polling or dropping either the returned
            // completion future or the owner itself.
            assert!(
                retained
                    .read_pending_thread_metadata(ThreadId::new())
                    .await
                    .is_err()
            );
        }
        drop(owner);
        fixture.wait_for("shutdown_started").await?;
        assert!(
            retained
                .read_pending_thread_metadata(ThreadId::new())
                .await
                .is_err()
        );
        fixture.release("shutdown_gate")?;
        retained.shutdown_store().await?;
        assert!(fixture.binding.state_dir.join("exited").exists());
    }
    Ok(())
}

#[tokio::test]
async fn absent_or_newer_migration_contract_keeps_ordinary_storage_compatible() -> Result<()> {
    for mode in ["migration_absent", "migration_future"] {
        let fixture = Fixture::new(mode)?;
        let store = fixture.connect().await??;
        assert_eq!(store.default_history_mode(), ThreadHistoryMode::Paginated);
        assert!(!store.supports_manual_rollout_migration());
        let result = store
            .start_rollout_migration(codex_thread_store::RolloutMigrationOptions {
                mode: codex_thread_store::RolloutMigrationMode::DryRun,
                thread_ids: Vec::new(),
                max_mib_per_second: None,
            })
            .await;
        assert!(matches!(
            result,
            Err(codex_thread_store::ThreadStoreError::Unsupported {
                operation: "manual_rollout_migration"
            })
        ));
        // The fixture rejects any non-open RPC: unsupported migration did not invoke it.
        fixture.release("shutdown_gate")?;
        store.shutdown_process().await?;
        assert!(fixture.binding.state_dir.join("exited").exists());
    }
    Ok(())
}
