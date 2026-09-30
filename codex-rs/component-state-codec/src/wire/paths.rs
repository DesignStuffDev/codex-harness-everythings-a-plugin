//! Native path transport for a trusted component on the same operating system.
//!
//! This codec preserves OS strings without consulting the filesystem, current
//! directory, or home directory. It does not change provider or rollout serde.
//! PathUri already preserves encoded native paths and must use its own serde.

use codex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;

// External tags keep platform provenance explicit. Raw paths deliberately have
// no Debug implementation and never appear in codec validation errors.
#[derive(Serialize, Deserialize)]
enum PlatformPath {
    UnixBytes(Vec<u8>),
    WindowsWide(Vec<u16>),
}

struct NativePathWire;

impl NativePathWire {
    fn serialize<S: serde::Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            PlatformPath::UnixBytes(path.as_os_str().as_bytes().to_vec()).serialize(serializer)
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            PlatformPath::WindowsWide(path.as_os_str().encode_wide().collect())
                .serialize(serializer)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (path, serializer);
            Err(serde::ser::Error::custom(
                "unsupported trusted component path platform",
            ))
        }
    }

    fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
        match PlatformPath::deserialize(deserializer)? {
            #[cfg(unix)]
            PlatformPath::UnixBytes(bytes) => {
                use std::os::unix::ffi::OsStringExt;
                Ok(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
            }
            #[cfg(not(unix))]
            PlatformPath::UnixBytes(_) => Err(serde::de::Error::custom(
                "trusted component path platform mismatch",
            )),
            #[cfg(windows)]
            PlatformPath::WindowsWide(units) => {
                use std::os::windows::ffi::OsStringExt;
                Ok(PathBuf::from(std::ffi::OsString::from_wide(&units)))
            }
            #[cfg(not(windows))]
            PlatformPath::WindowsWide(_) => Err(serde::de::Error::custom(
                "trusted component path platform mismatch",
            )),
        }
    }
}

struct AbsolutePathWire;

impl AbsolutePathWire {
    fn serialize<S: serde::Serializer>(
        path: &AbsolutePathBuf,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        NativePathWire::serialize(path.as_path(), serializer)
    }

    fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<AbsolutePathBuf, D::Error> {
        let path = NativePathWire::deserialize(deserializer)?;
        // Do not let constructor expansion resolve relative paths using ambient
        // process state. A native AbsolutePathBuf is already absolute/normalized.
        if !path.is_absolute() {
            return Err(serde::de::Error::custom(
                "trusted component path is not absolute",
            ));
        }
        let absolute = AbsolutePathBuf::from_absolute_path_checked(&path)
            .map_err(|_| serde::de::Error::custom("invalid trusted component absolute path"))?;
        // Path equality can ignore lexical differences; compare the OS string
        // itself so normalization cannot silently change the transported value.
        if absolute.as_path().as_os_str() != path.as_os_str() {
            return Err(serde::de::Error::custom(
                "trusted component absolute path is not normalized",
            ));
        }
        Ok(absolute)
    }
}

remote_adapter!(native, PathBuf, NativePathWire, "NativePathWire");
remote_adapter!(
    absolute,
    AbsolutePathBuf,
    AbsolutePathWire,
    "AbsolutePathWire"
);

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
