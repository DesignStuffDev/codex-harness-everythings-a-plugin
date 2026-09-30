//! Trusted process transport DTOs. These deliberately bypass public rollout omissions.
//! External enum tags retain arbitrary-precision JSON values without Serde content buffering.

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::SandboxPolicy")]
pub(crate) enum SandboxPolicyWire {
    DangerFullAccess,
    ReadOnly {
        network_access: bool,
    },
    ExternalSandbox {
        network_access: codex_protocol::protocol::NetworkAccess,
    },
    WorkspaceWrite {
        #[serde(with = "super::paths::absolute::vec")]
        writable_roots: Vec<codex_utils_absolute_path::AbsolutePathBuf>,
        network_access: bool,
        exclude_tmpdir_env_var: bool,
        exclude_slash_tmp: bool,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::permissions::FileSystemPath")]
pub(crate) enum FileSystemPathWire {
    Path {
        path: codex_utils_path_uri::PathUri,
    },
    GlobPattern {
        pattern: String,
    },
    Special {
        value: codex_protocol::permissions::FileSystemSpecialPath,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::permissions::FileSystemSandboxEntry")]
pub(crate) struct FileSystemSandboxEntryWire {
    #[serde(with = "FileSystemPathWire")]
    path: codex_protocol::permissions::FileSystemPath,
    access: codex_protocol::permissions::FileSystemAccessMode,
    missing_path_behavior:
        Option<codex_protocol::permissions::FileSystemSandboxEntryMissingPathBehavior>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::models::FileSystemPermissions")]
pub(crate) struct FileSystemPermissionsWire {
    #[serde(with = "entries")]
    entries: Vec<codex_protocol::permissions::FileSystemSandboxEntry>,
    glob_scan_max_depth: Option<std::num::NonZeroUsize>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::models::ManagedFileSystemPermissions")]
pub(crate) enum ManagedFileSystemPermissionsWire {
    Restricted {
        #[serde(with = "entries")]
        entries: Vec<codex_protocol::permissions::FileSystemSandboxEntry>,
        glob_scan_max_depth: Option<std::num::NonZeroUsize>,
    },
    Unrestricted,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::models::PermissionProfile")]
pub(crate) enum PermissionProfileWire {
    Managed {
        #[serde(with = "ManagedFileSystemPermissionsWire")]
        file_system: codex_protocol::models::ManagedFileSystemPermissions,
        network: codex_protocol::permissions::NetworkSandboxPolicy,
    },
    Disabled,
    External {
        network: codex_protocol::permissions::NetworkSandboxPolicy,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::models::AdditionalPermissionProfile")]
pub(crate) struct AdditionalPermissionProfileWire {
    network: Option<codex_protocol::models::NetworkPermissions>,
    #[serde(with = "optional_file_system")]
    file_system: Option<codex_protocol::models::FileSystemPermissions>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::request_permissions::RequestPermissionProfile")]
pub(crate) struct RequestPermissionProfileWire {
    network: Option<codex_protocol::models::NetworkPermissions>,
    #[serde(with = "optional_file_system")]
    file_system: Option<codex_protocol::models::FileSystemPermissions>,
}

pub(crate) mod entries {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::FileSystemSandboxEntryWire")]
        &'a codex_protocol::permissions::FileSystemSandboxEntry,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::FileSystemSandboxEntryWire")]
        codex_protocol::permissions::FileSystemSandboxEntry,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &[codex_protocol::permissions::FileSystemSandboxEntry],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(value.iter().map(Borrowed))
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<codex_protocol::permissions::FileSystemSandboxEntry>, D::Error> {
        let value = <Vec<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.into_iter().map(|owned| owned.0).collect())
    }
}

pub(crate) mod optional_file_system {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::FileSystemPermissionsWire")]
        &'a codex_protocol::models::FileSystemPermissions,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::FileSystemPermissionsWire")]
        codex_protocol::models::FileSystemPermissions,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::models::FileSystemPermissions>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::models::FileSystemPermissions>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_permission_profile {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::PermissionProfileWire")]
        &'a codex_protocol::models::PermissionProfile,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::PermissionProfileWire")] codex_protocol::models::PermissionProfile,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::models::PermissionProfile>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::models::PermissionProfile>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

pub(crate) mod optional_additional {
    #[derive(serde::Serialize)]
    struct Borrowed<'a>(
        #[serde(with = "super::AdditionalPermissionProfileWire")]
        &'a codex_protocol::models::AdditionalPermissionProfile,
    );
    #[derive(serde::Deserialize)]
    struct Owned(
        #[serde(with = "super::AdditionalPermissionProfileWire")]
        codex_protocol::models::AdditionalPermissionProfile,
    );

    pub(crate) fn serialize<S: serde::Serializer>(
        value: &Option<codex_protocol::models::AdditionalPermissionProfile>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&value.as_ref().map(Borrowed), serializer)
    }

    pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<codex_protocol::models::AdditionalPermissionProfile>, D::Error> {
        let value = <Option<Owned> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(value.map(|owned| owned.0))
    }
}

remote_adapter!(
    permission_profile,
    codex_protocol::models::PermissionProfile,
    PermissionProfileWire,
    "PermissionProfileWire"
);
