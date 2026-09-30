//! Native cold-file publication algorithm, unchanged by cancellation ownership.
use super::super::path;
use super::*;
use std::fs::File;
use std::fs::FileTimes;
use std::fs::Permissions;

enum ColdFileState {
    Cold(FileState),
    NotCold(Option<FileState>),
}

pub(super) fn compress_rollout_if_cold_blocking(
    path: &Path,
    writer_locks: &Arc<crate::WriterLockCoordinator>,
    thread_id: codex_protocol::ThreadId,
    trigger: RolloutCompressionTrigger,
) -> io::Result<CompressionMeasurement> {
    let before = match cold_file_state(path)
        .inspect_err(|err| FailureMetric::File(trigger).record("read_metadata", err))?
    {
        ColdFileState::Cold(state) => state,
        ColdFileState::NotCold(state) => {
            return Ok(CompressionMeasurement::new(
                CompressionOutcome::SkippedNotCold,
                state.map(|state| state.len),
                /*compressed_bytes*/ None,
            ));
        }
    };
    let source_bytes = Some(before.len);
    let compressed_path = path::compressed_rollout_path(path);
    if compressed_path.exists() {
        return Ok(CompressionMeasurement::new(
            CompressionOutcome::SkippedAlreadyCompressed,
            source_bytes,
            /*compressed_bytes*/ None,
        ));
    }

    let temp_dir = compressed_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(temp_dir)
        .inspect_err(|err| FailureMetric::File(trigger).record("prepare_directory", err))?;
    let mut temp_file = tempfile::Builder::new()
        .prefix("rollout-compress-")
        .suffix(TEMP_SUFFIX)
        .tempfile_in(temp_dir)
        .inspect_err(|err| FailureMetric::File(trigger).record("create_temp", err))?;
    encode_zstd_to_writer(path, temp_file.as_file_mut())
        .inspect_err(|err| FailureMetric::File(trigger).record("encode_and_write", err))?;
    temp_file
        .as_file_mut()
        .flush()
        .inspect_err(|err| FailureMetric::File(trigger).record("flush", err))?;
    verify_zstd(temp_file.path())
        .inspect_err(|err| FailureMetric::File(trigger).record("verify", err))?;
    if !same_file_state(path, &before)
        .inspect_err(|err| FailureMetric::File(trigger).record("recheck_source", err))?
    {
        return Ok(CompressionMeasurement::new(
            CompressionOutcome::SkippedChanged,
            source_bytes,
            /*compressed_bytes*/ None,
        ));
    }
    set_file_metadata(temp_file.as_file(), before.modified, &before.permissions)
        .inspect_err(|err| FailureMetric::File(trigger).record("set_metadata", err))?;
    temp_file
        .as_file()
        .sync_all()
        .inspect_err(|err| FailureMetric::File(trigger).record("sync", err))?;
    let compressed_bytes = temp_file
        .as_file()
        .metadata()
        .inspect_err(|err| FailureMetric::File(trigger).record("read_metadata", err))?
        .len();

    // Encoding and verification do not block writers. Coordination prevents writer
    // acquisition while we recheck, publish, and remove the original file.
    let Some(_publication_guard) = writer_locks
        .try_acquire_for_publication(thread_id)
        .inspect_err(|err| FailureMetric::File(trigger).record("writer_lock", err))?
    else {
        return Ok(CompressionMeasurement::new(
            CompressionOutcome::SkippedBusy,
            source_bytes,
            /*compressed_bytes*/ None,
        ));
    };
    if !same_file_state(path, &before)
        .inspect_err(|err| FailureMetric::File(trigger).record("recheck_source", err))?
    {
        return Ok(CompressionMeasurement::new(
            CompressionOutcome::SkippedChanged,
            source_bytes,
            /*compressed_bytes*/ None,
        ));
    }

    match temp_file.persist_noclobber(compressed_path.as_path()) {
        Ok(_) => {}
        Err(err) if err.error.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(CompressionMeasurement::new(
                CompressionOutcome::SkippedAlreadyCompressed,
                source_bytes,
                /*compressed_bytes*/ None,
            ));
        }
        Err(err) => {
            FailureMetric::File(trigger).record("publish", &err.error);
            return Err(err.error);
        }
    }
    if !same_file_state(path, &before)
        .inspect_err(|err| FailureMetric::File(trigger).record("recheck_source", err))?
    {
        let _ = std::fs::remove_file(compressed_path.as_path());
        return Ok(CompressionMeasurement::new(
            CompressionOutcome::SkippedChanged,
            source_bytes,
            /*compressed_bytes*/ None,
        ));
    }
    std::fs::remove_file(path)
        .inspect_err(|err| FailureMetric::File(trigger).record("remove_source", err))?;
    Ok(CompressionMeasurement::new(
        CompressionOutcome::Compressed,
        source_bytes,
        Some(compressed_bytes),
    ))
}

struct FileState {
    len: u64,
    modified: SystemTime,
    permissions: Permissions,
}

fn cold_file_state(path: &Path) -> io::Result<ColdFileState> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(ColdFileState::NotCold(None));
        }
        Err(err) => return Err(err),
    };
    if !metadata.is_file() {
        return Ok(ColdFileState::NotCold(None));
    }
    let modified = metadata.modified()?;
    let state = FileState {
        len: metadata.len(),
        modified,
        permissions: metadata.permissions(),
    };
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    if age < MIN_ROLLOUT_AGE {
        return Ok(ColdFileState::NotCold(Some(state)));
    }
    Ok(ColdFileState::Cold(state))
}

fn same_file_state(path: &Path, expected: &FileState) -> io::Result<bool> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len() == expected.len
            && metadata.modified()? == expected.modified
            && metadata.permissions() == expected.permissions),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

fn encode_zstd_to_writer(source: &Path, output: impl Write) -> io::Result<()> {
    let mut input = File::open(source)?;
    let mut encoder = zstd::stream::write::Encoder::new(output, COMPRESSION_LEVEL)?;
    // Preserve fast byte-bound checks for paginated history without decoding the whole file.
    encoder.set_pledged_src_size(Some(input.metadata()?.len()))?;
    io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

fn verify_zstd(path: &Path) -> io::Result<()> {
    let input = File::open(path)?;
    let mut decoder = zstd::stream::read::Decoder::new(input)?;
    let mut sink = io::sink();
    io::copy(&mut decoder, &mut sink)?;
    Ok(())
}

fn set_file_metadata(
    file: &File,
    modified: SystemTime,
    permissions: &Permissions,
) -> io::Result<()> {
    file.set_times(FileTimes::new().set_modified(modified))?;
    file.set_permissions(permissions.clone())
}
