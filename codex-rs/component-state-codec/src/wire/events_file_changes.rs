//! Lossless native file changes: map keys are paths, never JSON object strings.

use codex_protocol::protocol::FileChange;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "FileChange")]
pub(crate) enum FileChangeWire {
    Add {
        content: String,
    },
    Delete {
        content: String,
    },
    Update {
        unified_diff: String,
        #[serde(with = "super::paths::native::option")]
        move_path: Option<PathBuf>,
    },
}

remote_adapter!(file_change, FileChange, FileChangeWire, "FileChangeWire");

pub(crate) mod file_changes {
    use super::*;

    #[derive(serde::Serialize)]
    struct PathRef<'a>(#[serde(with = "super::super::paths::native")] &'a PathBuf);
    #[derive(serde::Deserialize)]
    struct OwnedPath(#[serde(with = "super::super::paths::native")] PathBuf);

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &HashMap<PathBuf, FileChange>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(
            value
                .iter()
                .map(|(path, change)| (PathRef(path), file_change::Borrowed(change))),
        )
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<PathBuf, FileChange>, D::Error> {
        let entries = <Vec<(OwnedPath, file_change::Owned)> as serde::Deserialize>::deserialize(
            deserializer,
        )?;
        let mut changes = HashMap::with_capacity(entries.len());
        for (path, change) in entries {
            if changes.insert(path.0, change.0).is_some() {
                return Err(serde::de::Error::custom(
                    "duplicate trusted file change path",
                ));
            }
        }
        Ok(changes)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::approvals::ApplyPatchApprovalRequestEvent")]
pub(crate) struct ApplyPatchApprovalRequestEventWire {
    call_id: String,
    turn_id: String,
    started_at_ms: i64,
    #[serde(with = "file_changes")]
    changes: HashMap<PathBuf, FileChange>,
    reason: Option<String>,
    #[serde(with = "super::paths::native::option")]
    grant_root: Option<PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::PatchApplyBeginEvent")]
pub(crate) struct PatchApplyBeginEventWire {
    call_id: String,
    turn_id: String,
    auto_approved: bool,
    #[serde(with = "file_changes")]
    changes: HashMap<PathBuf, FileChange>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::PatchApplyUpdatedEvent")]
pub(crate) struct PatchApplyUpdatedEventWire {
    call_id: String,
    #[serde(with = "file_changes")]
    changes: HashMap<PathBuf, FileChange>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::PatchApplyEndEvent")]
pub(crate) struct PatchApplyEndEventWire {
    call_id: String,
    turn_id: String,
    stdout: String,
    stderr: String,
    success: bool,
    #[serde(with = "file_changes")]
    changes: HashMap<PathBuf, FileChange>,
    status: codex_protocol::protocol::PatchApplyStatus,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::items::FileChangeItem")]
pub(crate) struct FileChangeItemWire {
    id: String,
    #[serde(with = "file_changes")]
    changes: HashMap<PathBuf, FileChange>,
    status: Option<codex_protocol::protocol::PatchApplyStatus>,
    auto_approved: Option<bool>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::ReviewCodeLocation")]
pub(crate) struct ReviewCodeLocationWire {
    #[serde(with = "super::paths::native")]
    absolute_file_path: PathBuf,
    line_range: codex_protocol::protocol::ReviewLineRange,
}
