use super::*;
use crate::RolloutItem;
use crate::RolloutLine;
use codex_protocol::ThreadId;
use codex_protocol::protocol::SessionMeta;
use codex_protocol::protocol::SessionMetaLine;
use pretty_assertions::assert_eq;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tempfile::TempDir;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Releases blocked native jobs even when an assertion panics.
struct Gates(Arc<(Mutex<[bool; 2]>, Condvar)>);

impl Gates {
    fn new() -> Self {
        Self(Arc::new((Mutex::new([false; 2]), Condvar::new())))
    }

    fn release(&self, index: usize) {
        let (open, wake) = &*self.0;
        open.lock().unwrap()[index] = true;
        wake.notify_all();
    }
}

impl Drop for Gates {
    fn drop(&mut self) {
        let (open, wake) = &*self.0;
        *open.lock().unwrap() = [true; 2];
        wake.notify_all();
    }
}

fn cold_rollouts(home: &Path, count: usize) -> anyhow::Result<Vec<PathBuf>> {
    let folder = home.join(SESSIONS_SUBDIR).join("2025/01/03");
    std::fs::create_dir_all(&folder)?;
    (0..count)
        .map(|index| {
            let id = uuid::Uuid::from_u128(index as u128 + 1);
            let thread_id = ThreadId::from_string(&id.to_string())?;
            let path = folder.join(format!("rollout-2025-01-03T12-00-00-{id}.jsonl"));
            let line = RolloutLine {
                timestamp: "2025-01-03T12:00:00Z".to_owned(),
                ordinal: None,
                item: RolloutItem::SessionMeta(SessionMetaLine {
                    meta: SessionMeta {
                        id: thread_id,
                        ..Default::default()
                    },
                    git: None,
                }),
            };
            std::fs::write(&path, format!("{}\n", serde_json::to_string(&line)?))?;
            let modified = SystemTime::now() - MIN_ROLLOUT_AGE - Duration::from_secs(60);
            std::fs::File::options()
                .write(true)
                .open(&path)?
                .set_times(std::fs::FileTimes::new().set_modified(modified))?;
            Ok(path)
        })
        .collect()
}

fn held_compressor(
    gates: &Gates,
    started: tokio::sync::mpsc::UnboundedSender<usize>,
    panic_index: Option<usize>,
) -> Arc<Compressor> {
    let gates = Arc::clone(&gates.0);
    let sequence = AtomicUsize::new(0);
    Arc::new(move |path, locks, thread_id, trigger| {
        let index = sequence.fetch_add(1, Ordering::SeqCst);
        started.send(index).unwrap();
        if index < 2 {
            let (open, wake) = &*gates;
            let mut open = open.lock().unwrap();
            while !open[index] {
                open = wake.wait(open).unwrap();
            }
        }
        assert_ne!(
            panic_index,
            Some(index),
            "injected native blocking worker panic"
        );
        // The accepted operation still executes the real compression algorithm.
        compress_rollout_if_cold_blocking(path, locks, thread_id, trigger)
    })
}

async fn wait_for_both_jobs(
    started: &mut tokio::sync::mpsc::UnboundedReceiver<usize>,
) -> anyhow::Result<()> {
    let first = tokio::time::timeout(TEST_TIMEOUT, started.recv()).await?;
    let second = tokio::time::timeout(TEST_TIMEOUT, started.recv()).await?;
    let mut indices = [first, second];
    indices.sort();
    assert_eq!(indices, [Some(0), Some(1)]);
    Ok(())
}

#[tokio::test]
async fn cancellation_stops_admission_and_waits_for_both_native_file_jobs() -> anyhow::Result<()> {
    let home = TempDir::new()?;
    let paths = cold_rollouts(home.path(), 4)?;
    let gates = Gates::new();
    let (started, mut observed) = tokio::sync::mpsc::unbounded_channel();
    let compressor = held_compressor(&gates, started, /*panic_index*/ None);
    let (cancel, cancellation) = watch::channel(/*init*/ false);
    let mut worker = tokio::spawn(run_with_compressor(
        home.path().to_path_buf(),
        RolloutCompressionTrigger::Startup,
        cancellation,
        compressor,
    ));
    wait_for_both_jobs(&mut observed).await?;
    cancel.send_replace(true);
    gates.release(0);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut worker)
            .await
            .is_err()
    );
    assert!(crate::try_acquire_rollout_maintenance_lock(home.path())?.is_none());
    gates.release(1);
    tokio::time::timeout(TEST_TIMEOUT, &mut worker).await???;
    assert!(
        observed.try_recv().is_err(),
        "cancelled pass admitted another file"
    );
    assert_eq!(paths.iter().filter(|path| path.exists()).count(), 2);
    assert_eq!(
        paths
            .iter()
            .filter(|path| super::super::path::compressed_rollout_path(path).exists())
            .count(),
        2
    );
    assert!(!home.path().join(".tmp").join(RUN_MARKER_FILE_NAME).exists());
    assert!(crate::try_acquire_rollout_maintenance_lock(home.path())?.is_some());
    Ok(())
}

#[tokio::test]
async fn failed_blocking_job_is_reported_only_after_the_other_job_finishes() -> anyhow::Result<()> {
    let home = TempDir::new()?;
    let paths = cold_rollouts(home.path(), 4)?;
    let gates = Gates::new();
    let (started, mut observed) = tokio::sync::mpsc::unbounded_channel();
    let compressor = held_compressor(&gates, started, /*panic_index*/ Some(0));
    let (keep_open, cancellation) = watch::channel(/*init*/ false);
    let mut worker = tokio::spawn(run_with_compressor(
        home.path().to_path_buf(),
        RolloutCompressionTrigger::Startup,
        cancellation,
        compressor,
    ));
    wait_for_both_jobs(&mut observed).await?;
    gates.release(0);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut worker)
            .await
            .is_err()
    );
    assert!(crate::try_acquire_rollout_maintenance_lock(home.path())?.is_none());
    gates.release(1);
    let error = tokio::time::timeout(TEST_TIMEOUT, &mut worker)
        .await??
        .unwrap_err();
    assert!(matches!(error, RolloutCompressionWorkerError::TaskJoin(error) if error.is_panic()));
    assert!(
        observed.try_recv().is_err(),
        "failed pass admitted another file"
    );
    assert_eq!(paths.iter().filter(|path| path.exists()).count(), 3);
    assert_eq!(
        paths
            .iter()
            .filter(|path| super::super::path::compressed_rollout_path(path).exists())
            .count(),
        1
    );
    assert!(!home.path().join(".tmp").join(RUN_MARKER_FILE_NAME).exists());
    assert!(crate::try_acquire_rollout_maintenance_lock(home.path())?.is_some());
    drop(keep_open);
    Ok(())
}

#[tokio::test]
async fn already_cancelled_or_closed_owner_never_claims_a_run_marker() -> anyhow::Result<()> {
    for closed in [false, true] {
        let home = TempDir::new()?;
        let paths = cold_rollouts(home.path(), 1)?;
        let (sender, cancellation) = watch::channel(!closed);
        let keep_open = (!closed).then_some(sender);
        super::super::run_rollout_compression_worker(
            home.path().to_path_buf(),
            RolloutCompressionTrigger::Rpc,
            cancellation,
        )
        .await?;
        assert!(paths[0].exists());
        assert!(!home.path().join(".tmp").join(RUN_MARKER_FILE_NAME).exists());
        drop(keep_open);
    }
    Ok(())
}

#[tokio::test]
async fn native_filesystem_failure_remains_an_operation_error() -> anyhow::Result<()> {
    let home = TempDir::new()?;
    let invalid_home = home.path().join("file-not-directory");
    std::fs::write(&invalid_home, "fixture")?;
    let (keep_open, cancellation) = watch::channel(/*init*/ false);
    let error = super::super::run_rollout_compression_worker(
        invalid_home,
        RolloutCompressionTrigger::Rpc,
        cancellation,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, RolloutCompressionWorkerError::Operation(_)));
    drop(keep_open);
    Ok(())
}
