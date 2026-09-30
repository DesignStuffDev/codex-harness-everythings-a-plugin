//! Native rollout discovery and bounded blocking-job ownership.
use super::*;
use tokio::task::JoinSet;

pub(super) async fn compress_rollouts_in_root(
    root: &Path,
    started_at: Instant,
    stats: &mut CompressionStats,
    writer_locks: &Arc<crate::WriterLockCoordinator>,
    trigger: RolloutCompressionTrigger,
    cancellation: &watch::Receiver<bool>,
    compressor: &Arc<Compressor>,
) -> io::Result<()> {
    if !tokio::fs::try_exists(root)
        .await
        .inspect_err(|err| {
            stats.scan_errors = true;
            FailureMetric::Scan(trigger).record("check_root", err);
        })
        .unwrap_or(false)
    {
        return Ok(());
    }
    let mut stack = vec![root.to_path_buf()];
    let mut jobs = JoinSet::new();
    'scan: while let Some(dir) = stack.pop() {
        if stop_requested(stats, cancellation, started_at) {
            break;
        }
        let mut read_dir = match tokio::fs::read_dir(dir.as_path()).await {
            Ok(read_dir) => read_dir,
            Err(err) => {
                stats.scan_errors = true;
                FailureMetric::Scan(trigger).record("read_directory", &err);
                warn!(
                    "failed to read rollout compression directory {}: {err}",
                    dir.display()
                );
                continue;
            }
        };
        loop {
            if stop_requested(stats, cancellation, started_at) {
                break 'scan;
            }
            let entry = match read_dir.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(err) => {
                    drain_compression_jobs(&mut jobs, stats, trigger).await;
                    return Err(err);
                }
            };
            if stop_requested(stats, cancellation, started_at) {
                break 'scan;
            }
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(file_type) => file_type,
                Err(err) => {
                    stats.scan_errors = true;
                    FailureMetric::Scan(trigger).record("read_file_type", &err);
                    warn!(
                        "failed to read rollout compression file type {}: {err}",
                        path.display()
                    );
                    continue;
                }
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let Some(rollout_file) = RolloutFile::from_path(path) else {
                continue;
            };
            if rollout_file.is_compressed() {
                continue;
            }
            let path = rollout_file.into_path();
            let Some(rollout_id) = crate::rollout_id_from_path(path.as_path()) else {
                stats.scan_errors = true;
                stats.skipped = stats.skipped.saturating_add(1);
                metrics::file(trigger, "skipped_unreadable_meta");
                continue;
            };
            let thread_id = match crate::read_session_meta_line(path.as_path()).await {
                Ok(metadata) => metadata.meta.id,
                Err(err) => {
                    stats.scan_errors = true;
                    let reason = err
                        .get_ref()
                        .and_then(|error| error.downcast_ref::<crate::list::MetadataReadError>())
                        .map_or("io", |error| error.reason);
                    let error_kind = super::super::error_metrics::error_kind(&err);
                    metrics::counter(
                        "codex.rollout_compression.scan",
                        &[
                            ("outcome", "failed"),
                            ("stage", "read_metadata"),
                            ("error_kind", error_kind),
                            ("trigger", trigger.tag()),
                            ("reason", reason),
                        ],
                    );
                    stats.metadata_read_failures = stats.metadata_read_failures.saturating_add(1);
                    if stats.metadata_read_failures <= MAX_METADATA_WARNINGS_PER_RUN {
                        let metadata = tokio::fs::metadata(&path).await.ok();
                        let file_size_bytes = metadata.as_ref().map(std::fs::Metadata::len);
                        let mtime_age_seconds = metadata
                            .and_then(|metadata| metadata.modified().ok())
                            .and_then(|modified| modified.elapsed().ok())
                            .map(|age| age.as_secs());
                        // Only bounded labels and parsed IDs, never paths or error messages.
                        warn!(
                            trigger = trigger.tag(),
                            %rollout_id,
                            reason,
                            error_kind,
                            file_size_bytes,
                            mtime_age_seconds,
                            "skipping rollout compression because session metadata could not be read"
                        );
                    }
                    stats.skipped = stats.skipped.saturating_add(1);
                    metrics::file(trigger, "skipped_unreadable_meta");
                    continue;
                }
            };
            stats.scanned = stats.scanned.saturating_add(1);
            metrics::file(trigger, "scanned");
            while jobs.len() >= MAX_CONCURRENT_COMPRESSION_JOBS {
                collect_next_compression_job(&mut jobs, stats, trigger).await;
                if stop_requested(stats, cancellation, started_at) {
                    break 'scan;
                }
            }
            if stop_requested(stats, cancellation, started_at) {
                break 'scan;
            }
            {
                // Serialize admission with cancellation send; never hold this
                // synchronous watch guard across a filesystem or task await.
                let admission = cancellation.borrow();
                if *admission || cancellation.has_changed().is_err() {
                    stats.cancelled = true;
                    break 'scan;
                }
                let compressor = Arc::clone(compressor);
                let writer_locks = Arc::clone(writer_locks);
                jobs.spawn_blocking(move || {
                    let started_at = Instant::now();
                    let result = compressor(path.as_path(), &writer_locks, thread_id, trigger);
                    let duration = started_at.elapsed();
                    (path, duration, result)
                });
            }
        }
    }
    drain_compression_jobs(&mut jobs, stats, trigger).await;
    Ok(())
}

type CompressionJobResult = (PathBuf, Duration, io::Result<CompressionMeasurement>);

async fn drain_compression_jobs(
    jobs: &mut JoinSet<CompressionJobResult>,
    stats: &mut CompressionStats,
    trigger: RolloutCompressionTrigger,
) {
    while !jobs.is_empty() {
        collect_next_compression_job(jobs, stats, trigger).await;
    }
}

async fn collect_next_compression_job(
    jobs: &mut JoinSet<CompressionJobResult>,
    stats: &mut CompressionStats,
    trigger: RolloutCompressionTrigger,
) {
    let Some(result) = jobs.join_next().await else {
        return;
    };
    match result {
        Ok((_, duration, Ok(measurement))) => {
            let outcome = measurement.outcome;
            match outcome {
                CompressionOutcome::Compressed => {
                    stats.compressed = stats.compressed.saturating_add(1);
                }
                CompressionOutcome::SkippedNotCold
                | CompressionOutcome::SkippedBusy
                | CompressionOutcome::SkippedChanged
                | CompressionOutcome::SkippedAlreadyCompressed => {
                    stats.skipped = stats.skipped.saturating_add(1);
                }
            }
            metrics::file(trigger, outcome.tag());
            metrics::file_duration(trigger, outcome.tag(), duration);
            if let Some(source_bytes) = measurement.source_bytes {
                metrics::source_bytes(trigger, outcome.tag(), source_bytes);
            }
            if let Some(compressed_bytes) = measurement.compressed_bytes {
                metrics::compressed_bytes(trigger, outcome.tag(), compressed_bytes);
                if let Some(source_bytes) = measurement.source_bytes {
                    metrics::compression_ratio(
                        trigger,
                        outcome.tag(),
                        source_bytes,
                        compressed_bytes,
                    );
                }
            }
        }
        Ok((path, duration, Err(err))) => {
            stats.failed = stats.failed.saturating_add(1);
            // The failing operation records its stage before returning the error.
            metrics::file_duration(trigger, "failed", duration);
            warn!("failed to compress rollout {}: {err}", path.display());
        }
        Err(err) => {
            stats.failed = stats.failed.saturating_add(1);
            warn!("rollout compression task failed: {err}");
            FailureMetric::File(trigger)
                .record("task_join", &io::Error::other("compression task failed"));
            if stats.task_join.is_none() {
                stats.task_join = Some(err);
            }
        }
    }
}
