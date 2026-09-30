//! Native compression-pass lifecycle and scan ownership.
mod cleanup;
mod file;
mod scan;

use cleanup::cleanup_stale_temps;
use file::compress_rollout_if_cold_blocking;
use scan::compress_rollouts_in_root;

use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use tokio::sync::watch;

use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::ARCHIVED_SESSIONS_SUBDIR;
use crate::SESSIONS_SUBDIR;

use super::RolloutCompressionTrigger;
use super::RolloutCompressionWorkerError;
use super::RolloutFile;
use super::error_metrics::FailureMetric;
use super::metrics;

const TEMP_SUFFIX: &str = ".tmp";
const COMPRESSION_LEVEL: i32 = 3;
const MIN_ROLLOUT_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const RUN_MARKER_STALE_AFTER: Duration = Duration::from_secs(6 * 60 * 60);
const TEMP_FILE_STALE_AFTER: Duration = RUN_MARKER_STALE_AFTER;
const WORKER_MAX_RUNTIME: Duration = Duration::from_secs(5 * 60 * 60);
const RUN_MARKER_FILE_NAME: &str = "rollout-compression.lock";
const MAX_CONCURRENT_COMPRESSION_JOBS: usize = 2;
const MAX_METADATA_WARNINGS_PER_RUN: usize = 5;

#[derive(Default)]
struct CompressionStats {
    scanned: usize,
    compressed: usize,
    skipped: usize,
    failed: usize,
    scan_errors: bool,
    metadata_read_failures: usize,
    cleanup_errors: bool,
    time_budget_exhausted: bool,
    cancelled: bool,
    task_join: Option<tokio::task::JoinError>,
}

pub(super) struct CompressionRunMarker {
    path: PathBuf,
    remove_on_drop: bool,
}

impl CompressionRunMarker {
    pub(super) fn try_claim(codex_home: &Path) -> io::Result<Option<Self>> {
        let marker_dir = codex_home.join(".tmp");
        std::fs::create_dir_all(marker_dir.as_path())?;
        let path = marker_dir.join(RUN_MARKER_FILE_NAME);
        match create_run_marker_file(path.as_path()) {
            Ok(()) => return Ok(Some(Self::new(path))),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err),
        }

        let stale = std::fs::metadata(path.as_path())
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age >= RUN_MARKER_STALE_AFTER);
        if !stale {
            return Ok(None);
        }
        match std::fs::remove_file(path.as_path()) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        match create_run_marker_file(path.as_path()) {
            Ok(()) => Ok(Some(Self::new(path))),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn new(path: PathBuf) -> Self {
        Self {
            path,
            remove_on_drop: true,
        }
    }

    pub(super) fn persist(mut self) {
        self.remove_on_drop = false;
    }
}

impl Drop for CompressionRunMarker {
    fn drop(&mut self) {
        if self.remove_on_drop {
            let _ = std::fs::remove_file(self.path.as_path());
        }
    }
}

pub(super) fn spawn(codex_home: PathBuf, trigger: RolloutCompressionTrigger) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        metrics::run(trigger, "skipped_no_runtime");
        warn!(
            "failed to start rollout compression worker for {}: no Tokio runtime",
            codex_home.display()
        );
        return;
    };
    handle.spawn(async move {
        if let Err(err) = run(codex_home.clone(), trigger).await {
            warn!(
                "rollout compression worker failed for {}: {err}",
                codex_home.display()
            );
        }
    });
}

type Compressor = dyn Fn(
        &Path,
        &Arc<crate::WriterLockCoordinator>,
        codex_protocol::ThreadId,
        RolloutCompressionTrigger,
    ) -> io::Result<CompressionMeasurement>
    + Send
    + Sync;

pub(super) async fn run(codex_home: PathBuf, trigger: RolloutCompressionTrigger) -> io::Result<()> {
    let (keep_open, cancellation) = watch::channel(/*init*/ false);
    let result = run_controlled(codex_home, trigger, cancellation).await;
    drop(keep_open);
    result.map_err(|error| match error {
        RolloutCompressionWorkerError::Operation(error) => error,
        RolloutCompressionWorkerError::TaskJoin(error) => io::Error::other(error),
    })
}

pub(super) async fn run_controlled(
    codex_home: PathBuf,
    trigger: RolloutCompressionTrigger,
    cancellation: watch::Receiver<bool>,
) -> Result<(), RolloutCompressionWorkerError> {
    run_with_compressor(
        codex_home,
        trigger,
        cancellation,
        Arc::new(compress_rollout_if_cold_blocking),
    )
    .await
}

fn cancellation_requested(cancellation: &watch::Receiver<bool>) -> bool {
    *cancellation.borrow() || cancellation.has_changed().is_err()
}

fn stop_requested(
    stats: &mut CompressionStats,
    cancellation: &watch::Receiver<bool>,
    started_at: Instant,
) -> bool {
    stats.cancelled |= cancellation_requested(cancellation);
    stats.time_budget_exhausted |= started_at.elapsed() >= WORKER_MAX_RUNTIME;
    stats.cancelled || stats.time_budget_exhausted || stats.task_join.is_some()
}

async fn run_with_compressor(
    codex_home: PathBuf,
    trigger: RolloutCompressionTrigger,
    cancellation: watch::Receiver<bool>,
    compressor: Arc<Compressor>,
) -> Result<(), RolloutCompressionWorkerError> {
    if cancellation_requested(&cancellation) {
        metrics::run(trigger, "cancelled");
        return Ok(());
    }
    let Some(_maintenance_guard) =
        crate::try_acquire_rollout_maintenance_lock(codex_home.as_path())
            .inspect_err(|err| FailureMetric::Run(trigger).record("maintenance_lock", err))?
    else {
        metrics::run(trigger, "skipped_maintenance");
        debug!(
            "rollout maintenance is already running for {}",
            codex_home.display()
        );
        return Ok(());
    };
    let marker = match CompressionRunMarker::try_claim(codex_home.as_path()) {
        Ok(Some(marker)) => marker,
        Ok(None) => {
            metrics::run(trigger, "skipped_already_running");
            debug!(
                "rollout compression worker recently ran or is already running for {}",
                codex_home.display()
            );
            return Ok(());
        }
        Err(err) => {
            FailureMetric::Run(trigger).record("run_marker", &err);
            return Err(err.into());
        }
    };

    metrics::run(trigger, "started");
    let started_at = Instant::now();
    let writer_locks = Arc::new(crate::WriterLockCoordinator::new(&codex_home));
    let mut stage = "temp_cleanup";
    let mut stats = CompressionStats::default();
    let result = async {
        stats.cleanup_errors =
            cleanup_stale_temps(codex_home.as_path(), trigger, &cancellation).await?;
        stage = "scan";
        for root in [
            codex_home.join(ARCHIVED_SESSIONS_SUBDIR),
            codex_home.join(SESSIONS_SUBDIR),
        ] {
            if stop_requested(&mut stats, &cancellation, started_at) {
                break;
            }
            compress_rollouts_in_root(
                root.as_path(),
                started_at,
                &mut stats,
                &writer_locks,
                trigger,
                &cancellation,
                &compressor,
            )
            .await?;
        }
        Ok::<_, io::Error>(())
    }
    .await;
    if let Some(error) = stats.task_join.take() {
        FailureMetric::Run(trigger)
            .record("task_join", &io::Error::other("compression task failed"));
        metrics::run_duration(trigger, "failed", started_at.elapsed());
        return Err(RolloutCompressionWorkerError::TaskJoin(error));
    }
    match result {
        Ok(()) => {}
        Err(err) => {
            FailureMetric::Run(trigger).record(stage, &err);
            metrics::run_duration(trigger, "failed", started_at.elapsed());
            return Err(err.into());
        }
    };
    info!(
        metadata_read_failures = stats.metadata_read_failures,
        "rollout compression worker finished: scanned={}, compressed={}, skipped={}, failed={}",
        stats.scanned,
        stats.compressed,
        stats.skipped,
        stats.failed
    );
    // The final read guard serializes marker publication with cancellation send.
    // There are no awaits while it is held: a later signal follows completion.
    let terminal_signal = cancellation.borrow();
    stats.cancelled |= *terminal_signal || cancellation.has_changed().is_err();
    // Keep the existing completed outcome: it means the pass returned, not
    // that every directory was scanned or every file was compressed.
    let completion = if stats.cancelled {
        "cancelled"
    } else if stats.time_budget_exhausted {
        "time_budget"
    } else {
        "scan_finished"
    };
    let tags = [
        ("status", "completed"),
        ("trigger", trigger.tag()),
        ("completion_reason", completion),
        (
            "file_errors",
            if stats.failed > 0 { "true" } else { "false" },
        ),
        (
            "scan_errors",
            if stats.scan_errors { "true" } else { "false" },
        ),
        (
            "cleanup_errors",
            if stats.cleanup_errors {
                "true"
            } else {
                "false"
            },
        ),
    ];
    metrics::counter(metrics::RUN_COUNTER, &tags);
    metrics::duration_histogram(metrics::RUN_DURATION_HISTOGRAM, started_at.elapsed(), &tags);
    if !stats.cancelled {
        marker.persist();
    }
    Ok(())
}

fn create_run_marker_file(path: &Path) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    writeln!(
        file,
        "pid={} started_at={:?}",
        std::process::id(),
        SystemTime::now()
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompressionOutcome {
    Compressed,
    SkippedNotCold,
    SkippedBusy,
    SkippedChanged,
    SkippedAlreadyCompressed,
}

impl CompressionOutcome {
    fn tag(self) -> &'static str {
        match self {
            CompressionOutcome::Compressed => "compressed",
            CompressionOutcome::SkippedNotCold => "skipped_not_cold",
            CompressionOutcome::SkippedBusy => "skipped_busy",
            CompressionOutcome::SkippedChanged => "skipped_changed",
            CompressionOutcome::SkippedAlreadyCompressed => "skipped_already_compressed",
        }
    }
}

struct CompressionMeasurement {
    outcome: CompressionOutcome,
    source_bytes: Option<u64>,
    compressed_bytes: Option<u64>,
}

impl CompressionMeasurement {
    fn new(
        outcome: CompressionOutcome,
        source_bytes: Option<u64>,
        compressed_bytes: Option<u64>,
    ) -> Self {
        Self {
            outcome,
            source_bytes,
            compressed_bytes,
        }
    }
}

#[cfg(test)]
#[path = "worker/worker_tests.rs"]
mod tests;
