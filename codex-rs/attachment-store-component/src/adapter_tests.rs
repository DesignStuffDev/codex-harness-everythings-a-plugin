use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use codex_attachment_store_api::AttachmentMetadata;
use codex_attachment_store_api::AttachmentStore;
use codex_attachment_store_api::AttachmentStoreErrorKind;
use codex_attachment_store_api::ResolveRequest;
use codex_attachment_store_api::UploadRequest;
use codex_attachment_store_api::UploadResult;
use codex_component_host::ComponentBinding;
use codex_component_host::ComponentSpec;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

use super::ComponentAttachmentStore;

const PLUGIN: &str = r#"
import json, os, pathlib, stat, sys, time
def send(value): print(json.dumps(value), flush=True)
initial = json.loads(sys.stdin.readline())
state = pathlib.Path(initial['state_dir'])
mode = initial['config']['mode']
send({'type':'ready','api_version':1})
request = json.loads(sys.stdin.readline())
params = request['params']
state.joinpath('request.json').write_text(json.dumps(request))
if request['method'] == 'upload':
    path = pathlib.Path(params['blob']['path'])
    data = path.read_bytes()
    state.joinpath('observed.json').write_text(json.dumps({'size':len(data), 'sum':sum(data), 'directory_mode':stat.S_IMODE(path.parent.stat().st_mode), 'file_writable':bool(stat.S_IMODE(path.stat().st_mode)&0o222), 'directory':str(path.parent),'pid':os.getpid()}))
    if mode == 'stall': time.sleep(60)
    if mode == 'mutate':
        path.chmod(0o600); path.write_bytes(b'replaced by plugin')
    result = {'ok':{'kind':'file','file_id':'provider-file-1'}} if mode == 'file' else {'ok':{'kind':'inline'}}
else:
    result = {'ok':{'metadata':{'file_name':'saved.png','size_bytes':17},'url_lifetime':None}}
    if mode != 'metadata':
        result['ok']['metadata']['file_url'] = 'https://files.example/image?token=private'
        if mode == 'permanent': result['ok']['url_lifetime'] = {'kind':'permanent'}
        elif mode != 'missing_lifetime': result['ok']['url_lifetime'] = {'kind':'expires','expires_at_unix_ms':int(time.time()*1000)+(params['download_url_ttl_ms'] or 0)+(10000 if mode == 'valid' else 100 if mode == 'shutdown_delay' else -1)}
if mode == 'typed': result = {'error':'not_found'}
if mode == 'error':
    send({'type':'error','id':1,'message':'secret attachment bytes and https://private?token=secret'})
else:
    send({'type':'result','id':1,'result':result})
    assert json.loads(sys.stdin.readline()) == {'type':'shutdown'}
    if mode == 'shutdown_delay': time.sleep(0.25)
"#;

struct Fixture {
    directory: TempDir,
    binding: ComponentBinding,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let directory = tempfile::tempdir().expect("external fixture directory");
        let script = directory.path().join("store.py");
        fs::write(&script, PLUGIN).expect("write store fixture");
        let python = ["python3", "python"]
            .into_iter()
            .find(|name| {
                std::process::Command::new(name)
                    .arg("--version")
                    .output()
                    .is_ok_and(|output| output.status.success())
            })
            .expect("attachment component tests require Python");
        let binding = ComponentBinding {
            plugin_id: "attachment-fixture".to_owned(),
            spec: ComponentSpec {
                kind: "attachment_store".to_owned(),
                name: "default".to_owned(),
                contract_version: 1,
                metadata: Value::Null,
            },
            config: json!({"mode":mode}),
            package_dir: directory.path().to_path_buf(),
            state_dir: directory.path().join("state"),
            entrypoint: PathBuf::from(python),
            args: vec![script.to_string_lossy().into_owned()],
            timeout_ms: 5_000,
        };
        Self { directory, binding }
    }

    fn store(&self) -> ComponentAttachmentStore {
        ComponentAttachmentStore::new(self.binding.clone()).expect("valid binding")
    }

    fn recorded(&self, name: &str) -> Value {
        serde_json::from_slice(
            &fs::read(self.binding.state_dir.join(name)).expect("fixture record"),
        )
        .expect("record JSON")
    }

    fn transfers(&self) -> usize {
        fs::read_dir(self.directory.path().join("state"))
            .expect("state directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".attachment-transfer-")
            })
            .count()
    }
}

fn upload(data: Vec<u8>) -> UploadRequest {
    UploadRequest {
        thread_id: "thread-1".to_owned(),
        file_name: Some("private.png".to_owned()),
        data,
    }
}

#[tokio::test]
async fn uploads_larger_than_frame_use_private_staging_and_preserve_exact_bytes() {
    let fixture = Fixture::new("inline");
    let bytes: Vec<u8> = (0..5 * 1024 * 1024)
        .map(|index| (index % 256) as u8)
        .collect();
    let result = fixture
        .store()
        .upload(upload(bytes.clone()))
        .await
        .expect("upload");
    assert_eq!(
        result,
        UploadResult::Inline {
            bytes: bytes.clone()
        }
    );
    let observed = fixture.recorded("observed.json");
    assert_eq!(observed["size"], bytes.len());
    assert_eq!(
        observed["sum"],
        bytes.iter().map(|byte| u64::from(*byte)).sum::<u64>()
    );
    #[cfg(unix)]
    assert_eq!(
        (
            observed["directory_mode"].clone(),
            observed["file_writable"].clone()
        ),
        (json!(0o700), json!(false))
    );
    assert_eq!(fixture.transfers(), 0);
    assert!(fixture.recorded("request.json").to_string().len() < 4096);
}

#[tokio::test]
async fn inline_result_keeps_host_owned_bytes_even_if_plugin_changes_staging() {
    let fixture = Fixture::new("mutate");
    let result = fixture
        .store()
        .upload(upload(b"original".to_vec()))
        .await
        .expect("upload");
    assert_eq!(
        result,
        UploadResult::Inline {
            bytes: b"original".to_vec()
        }
    );
    assert_eq!(fixture.transfers(), 0);
}

#[tokio::test]
async fn file_reference_and_typed_errors_cross_adapter_without_backend_details() {
    let fixture = Fixture::new("file");
    assert_eq!(
        fixture
            .store()
            .upload(upload(vec![1, 2]))
            .await
            .expect("upload"),
        UploadResult::File {
            file_id: "provider-file-1".to_owned()
        }
    );
    for (mode, kind) in [
        ("typed", AttachmentStoreErrorKind::NotFound),
        ("error", AttachmentStoreErrorKind::Backend),
    ] {
        let fixture = Fixture::new(mode);
        let error = fixture
            .store()
            .upload(upload(b"private".to_vec()))
            .await
            .expect_err("failed upload");
        assert_eq!(error.kind(), kind);
        assert!(!error.to_string().contains("secret"));
        assert!(!error.to_string().contains("https"));
        assert_eq!(fixture.transfers(), 0);
    }
}

#[tokio::test]
async fn metadata_only_resolve_omits_urls_and_rounds_up_ttl_requests() {
    let fixture = Fixture::new("metadata");
    assert_eq!(
        fixture
            .store()
            .resolve(ResolveRequest {
                file_id: "file-1",
                download_url_ttl: None
            })
            .await
            .expect("metadata"),
        AttachmentMetadata {
            file_name: Some("saved.png".to_owned()),
            size_bytes: Some(17),
            ..AttachmentMetadata::default()
        }
    );
    let fixture = Fixture::new("valid");
    let resolved = fixture
        .store()
        .resolve(ResolveRequest {
            file_id: "file-1",
            download_url_ttl: Some(Duration::from_nanos(1)),
        })
        .await
        .expect("fresh URL");
    assert_eq!(
        fixture.recorded("request.json")["params"]["download_url_ttl_ms"],
        1
    );
    assert_eq!(
        resolved.file_url,
        Some("https://files.example/image?token=private".to_owned())
    );
}

#[tokio::test]
async fn invalid_lifetimes_are_rejected_and_permanent_urls_are_explicit() {
    for (mode, ttl) in [
        ("valid", None),
        ("expired", Some(Duration::from_secs(5))),
        ("shutdown_delay", Some(Duration::from_secs(5))),
        ("missing_lifetime", Some(Duration::from_secs(5))),
        ("metadata", Some(Duration::ZERO)),
    ] {
        let fixture = Fixture::new(mode);
        let error = fixture
            .store()
            .resolve(ResolveRequest {
                file_id: "file-1",
                download_url_ttl: ttl,
            })
            .await
            .expect_err("invalid URL lifetime");
        assert_eq!(error.kind(), AttachmentStoreErrorKind::InvalidAttachment);
        assert!(!format!("{error:?}").contains("private"));
    }
    let fixture = Fixture::new("permanent");
    assert!(
        fixture
            .store()
            .resolve(ResolveRequest {
                file_id: "file-1",
                download_url_ttl: Some(Duration::from_secs(3600))
            })
            .await
            .expect("permanent URL")
            .file_url
            .is_some()
    );
}

#[tokio::test]
async fn cancellation_terminates_invocation_and_removes_staging() {
    let fixture = Fixture::new("stall");
    let store = fixture.store();
    let task = tokio::spawn(async move { store.upload(upload(vec![9; 1024])).await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !fixture.binding.state_dir.join("observed.json").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("plugin reaches upload");
    let observed = fixture.recorded("observed.json");
    task.abort();
    assert!(task.await.expect_err("canceled task").is_cancelled());
    tokio::time::timeout(Duration::from_secs(3), async {
        while fixture.transfers() != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        #[cfg(target_os = "linux")]
        while PathBuf::from(format!("/proc/{}", observed["pid"])).exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child and staging are cleaned up");
}

#[test]
fn cancellation_while_staging_is_queued_never_creates_transfer_or_starts_plugin() {
    let fixture = Fixture::new("inline");
    let store = fixture.store();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .expect("runtime");
    let (release, blocked) = std::sync::mpsc::channel();
    let (started, started_rx) = std::sync::mpsc::channel();
    let blocker = runtime.spawn_blocking(move || {
        started.send(()).expect("notify blocked pool");
        blocked.recv().expect("release blocked pool");
    });
    started_rx.recv().expect("blocking pool occupied");
    runtime.block_on(async {
        let mut upload = store.upload(upload(vec![9; 1024 * 1024]));
        std::future::poll_fn(|cx| {
            assert!(upload.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(upload);
    });
    release.send(()).expect("release staging worker");
    runtime.block_on(blocker).expect("worker released");
    // Dropping the runtime waits for the detached blocking staging worker too.
    drop(runtime);
    assert!(
        !fixture.binding.state_dir.exists(),
        "cancelled queued staging must not perform filesystem or plugin work"
    );
}
