use super::PlatformPath;
use super::absolute;
use super::native;
use codex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct NativePath(#[serde(with = "native")] PathBuf);

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct AbsolutePath(#[serde(with = "absolute")] AbsolutePathBuf);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Containers {
    #[serde(with = "native::vec")]
    native_vec: Vec<PathBuf>,
    #[serde(with = "native::option")]
    native_option: Option<PathBuf>,
    #[serde(with = "native::option_vec")]
    native_option_vec: Option<Vec<PathBuf>>,
    #[serde(with = "absolute::vec")]
    absolute_vec: Vec<AbsolutePathBuf>,
    #[serde(with = "absolute::option")]
    absolute_option: Option<AbsolutePathBuf>,
    #[serde(with = "absolute::option_vec")]
    absolute_option_vec: Option<Vec<AbsolutePathBuf>>,
}

#[cfg(unix)]
fn non_unicode_path() -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(b"path_\xff\x80".to_vec()))
}

#[cfg(windows)]
fn non_unicode_path() -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_wide(&[
        112, 97, 116, 104, 95, 0xd800,
    ]))
}

fn absolute_non_unicode_path() -> AbsolutePathBuf {
    #[cfg(unix)]
    let path = PathBuf::from("/").join(non_unicode_path());
    #[cfg(windows)]
    let path = PathBuf::from(r"C:\").join(non_unicode_path());
    AbsolutePathBuf::from_absolute_path_checked(path).expect("absolute non-Unicode path")
}

#[test]
fn trusted_native_path_preserves_non_unicode_os_string() {
    let original = NativePath(non_unicode_path());
    assert!(original.0.to_str().is_none());
    assert!(serde_json::to_value(&original.0).is_err());
    let json = serde_json::to_string(&original).expect("trusted path encoding");
    let restored: NativePath = serde_json::from_str(&json).expect("trusted path decoding");
    assert_eq!(restored.0.as_os_str(), original.0.as_os_str());
}

#[test]
fn public_wrappers_preserve_pinned_wire_tag_and_native_units() {
    let path = non_unicode_path();
    #[cfg(unix)]
    let expected = serde_json::json!({ "UnixBytes": [112, 97, 116, 104, 95, 255, 128] });
    #[cfg(windows)]
    let expected = serde_json::json!({ "WindowsWide": [112, 97, 116, 104, 95, 55296] });

    let encoded = serde_json::to_value(crate::native_path::Borrowed(&path))
        .expect("public borrowed encoding");
    assert_eq!(encoded, expected);
    let restored: crate::native_path::Owned =
        serde_json::from_value(expected).expect("public owned decoding");
    assert_eq!(restored.0.as_os_str(), path.as_os_str());

    let absolute = absolute_non_unicode_path();
    let encoded = serde_json::to_value(crate::absolute_path::Borrowed(&absolute))
        .expect("public absolute borrowed encoding");
    let restored: crate::absolute_path::Owned =
        serde_json::from_value(encoded).expect("public absolute owned decoding");
    assert_eq!(restored.0, absolute);
}

#[test]
fn trusted_native_path_does_not_normalize_relative_or_empty_paths() {
    for path in ["", ".", "./one//../two/", "a\0b"] {
        let original = NativePath(PathBuf::from(path));
        let json = serde_json::to_string(&original).expect("trusted path encoding");
        let restored: NativePath = serde_json::from_str(&json).expect("trusted path decoding");
        assert_eq!(restored.0.as_os_str(), original.0.as_os_str());
    }
}

#[test]
fn trusted_absolute_path_preserves_non_unicode_os_string() {
    let original = AbsolutePath(absolute_non_unicode_path());
    assert!(serde_json::to_value(&original.0).is_err());
    let json = serde_json::to_string(&original).expect("trusted absolute encoding");
    let restored: AbsolutePath = serde_json::from_str(&json).expect("trusted absolute decoding");
    assert_eq!(
        restored.0.as_path().as_os_str(),
        original.0.as_path().as_os_str()
    );
}

#[test]
fn trusted_absolute_path_rejects_relative_and_empty_input() {
    for path in ["", "relative", "~/relative"] {
        let json =
            serde_json::to_string(&NativePath(PathBuf::from(path))).expect("native encoding");
        let error =
            serde_json::from_str::<AbsolutePath>(&json).expect_err("relative path rejected");
        assert!(error.to_string().contains("not absolute"));
    }
}

#[test]
fn trusted_absolute_path_rejects_representation_changes() {
    #[cfg(unix)]
    let path = PathBuf::from("/one//two/../three/");
    #[cfg(windows)]
    let path = PathBuf::from(r"C:\one\two\..\three\");
    let json = serde_json::to_string(&NativePath(path)).expect("native encoding");
    let error = serde_json::from_str::<AbsolutePath>(&json).expect_err("normalization rejected");
    assert!(error.to_string().contains("not normalized"));
}

#[test]
fn trusted_paths_reject_foreign_platform() {
    #[cfg(unix)]
    let foreign = PlatformPath::WindowsWide(vec![67, 58, 92, 0xd800]);
    #[cfg(windows)]
    let foreign = PlatformPath::UnixBytes(vec![47, 0xff]);
    let json = serde_json::to_string(&foreign).expect("tagged path encoding");
    for error in [
        serde_json::from_str::<NativePath>(&json).expect_err("foreign native path rejected"),
        serde_json::from_str::<AbsolutePath>(&json).expect_err("foreign absolute path rejected"),
    ] {
        assert!(error.to_string().contains("platform mismatch"));
    }
}

#[test]
fn trusted_path_containers_preserve_values_and_presence() {
    let native = non_unicode_path();
    let absolute = absolute_non_unicode_path();
    for original in [
        Containers {
            native_vec: vec![native.clone(), PathBuf::new()],
            native_option: Some(native.clone()),
            native_option_vec: Some(vec![native]),
            absolute_vec: vec![absolute.clone()],
            absolute_option: Some(absolute.clone()),
            absolute_option_vec: Some(vec![absolute]),
        },
        Containers {
            native_vec: vec![],
            native_option: None,
            native_option_vec: Some(vec![]),
            absolute_vec: vec![],
            absolute_option: None,
            absolute_option_vec: Some(vec![]),
        },
        Containers {
            native_vec: vec![],
            native_option: None,
            native_option_vec: None,
            absolute_vec: vec![],
            absolute_option: None,
            absolute_option_vec: None,
        },
    ] {
        let json = serde_json::to_string(&original).expect("container encoding");
        let restored: Containers = serde_json::from_str(&json).expect("container decoding");
        assert_eq!(restored, original);
        assert_eq!(
            serde_json::to_string(&restored).expect("restored encoding"),
            json
        );
    }
}
