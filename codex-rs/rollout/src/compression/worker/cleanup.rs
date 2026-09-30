//! Best-effort removal of old interrupted-compression temporary files.
use super::*;
use std::ffi::OsStr;

pub(super) async fn cleanup_stale_temps(
    codex_home: &Path,
    trigger: RolloutCompressionTrigger,
    cancellation: &watch::Receiver<bool>,
) -> io::Result<bool> {
    let mut errors = false;
    for root in [
        codex_home.join(SESSIONS_SUBDIR),
        codex_home.join(ARCHIVED_SESSIONS_SUBDIR),
    ] {
        if cancellation_requested(cancellation) {
            break;
        }
        errors |= cleanup_stale_temps_in_root(root.as_path(), trigger, cancellation).await?;
    }
    Ok(errors)
}

async fn cleanup_stale_temps_in_root(
    root: &Path,
    trigger: RolloutCompressionTrigger,
    cancellation: &watch::Receiver<bool>,
) -> io::Result<bool> {
    let mut errors = false;
    if !tokio::fs::try_exists(root)
        .await
        .inspect_err(|err| {
            errors = true;
            FailureMetric::TempCleanup(trigger).record("check_root", err);
        })
        .unwrap_or(false)
    {
        return Ok(errors);
    }
    let mut stack = vec![root.to_path_buf()];
    'cleanup: while let Some(dir) = stack.pop() {
        if cancellation_requested(cancellation) {
            break;
        }
        let mut read_dir = match tokio::fs::read_dir(dir.as_path()).await {
            Ok(read_dir) => read_dir,
            Err(err) => {
                errors = true;
                FailureMetric::TempCleanup(trigger).record("read_directory", &err);
                warn!(
                    "failed to read rollout temp cleanup directory {}: {err}",
                    dir.display()
                );
                continue;
            }
        };
        while let Some(entry) = read_dir.next_entry().await? {
            if cancellation_requested(cancellation) {
                break 'cleanup;
            }
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(file_type) => file_type,
                Err(err) => {
                    errors = true;
                    FailureMetric::TempCleanup(trigger).record("read_file_type", &err);
                    warn!(
                        "failed to read rollout temp cleanup file type {}: {err}",
                        path.display()
                    );
                    continue;
                }
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if file_type.is_file()
                && path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.ends_with(TEMP_SUFFIX))
            {
                let stale = entry
                    .metadata()
                    .await
                    .and_then(|metadata| metadata.modified())
                    .inspect_err(|err| {
                        errors = true;
                        FailureMetric::TempCleanup(trigger).record("read_metadata", err);
                    })
                    .ok()
                    .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                    .is_some_and(|age| age >= TEMP_FILE_STALE_AFTER);
                if !stale {
                    continue;
                }
                if cancellation_requested(cancellation) {
                    break 'cleanup;
                }
                match tokio::fs::remove_file(path.as_path()).await {
                    Ok(()) => metrics::temp_cleanup(trigger, "removed"),
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => {
                        errors = true;
                        FailureMetric::TempCleanup(trigger).record("remove_temp", &err);
                        warn!(
                            "failed to remove stale rollout temp {}: {err}",
                            path.display()
                        );
                    }
                }
            }
        }
    }
    Ok(errors)
}
