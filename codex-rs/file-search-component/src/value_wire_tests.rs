#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

fn sample(roots: &[PathBuf]) -> SearchPoll {
    SearchPoll::Changed(SearchFrame {
        revision: (1 << 53) + 23,
        query_id: u64::MAX - 1,
        query: "résumé \"a\"".to_owned(),
        snapshot: Some(FileSearchSnapshot {
            query_id: u64::MAX - 1,
            query: "résumé \"a\"".to_owned(),
            matches: vec![FileMatch {
                score: 823,
                path: PathBuf::from("résumé.txt"),
                match_type: MatchType::File,
                root: roots[1].clone(),
                indices: Some(vec![0, 2, 4]),
            }],
            total_match_count: 8,
            scanned_file_count: 12,
            walk_complete: true,
        }),
        phase: SearchPhase::Idle,
    })
}

#[test]
fn decimal_wire_retains_high_bits_and_rejects_ambiguous_spellings() {
    for value in [0, (1 << 53) + 1, u64::MAX] {
        let encoded = serde_json::to_value(WireU64(value)).unwrap();
        assert_eq!(encoded, json!(value.to_string()));
        assert_eq!(
            serde_json::from_value::<WireU64>(encoded).unwrap(),
            WireU64(value)
        );
    }
    for text in [
        "",
        "00",
        "01",
        "-1",
        "+1",
        " 1",
        "1 ",
        "1.0",
        "1e2",
        "１",
        "18446744073709551616",
    ] {
        assert!(
            serde_json::from_value::<WireU64>(json!(text)).is_err(),
            "{text:?}"
        );
    }
    assert!(serde_json::from_value::<WireU64>(json!(1)).is_err());
}

#[test]
fn frame_round_trip_preserves_exact_root_spelling_and_query_identity() {
    let roots = vec![PathBuf::from("root"), PathBuf::from("root//")];
    let original = sample(&roots);
    let wire = WirePoll::from_native(original.clone(), &roots).unwrap();
    let encoded = serde_json::to_value(wire).unwrap();
    assert_eq!(
        encoded["frame"]["snapshot"]["matches"][0]["root_index"],
        json!(1)
    );
    assert_eq!(
        encoded["frame"]["query_epoch"],
        json!((u64::MAX - 1).to_string())
    );
    assert_eq!(encoded["frame"]["phase"], json!({"state":"idle"}));
    let decoded: WirePoll = serde_json::from_value(encoded).unwrap();
    let SearchPoll::Changed(frame) = decoded.into_native(&roots).unwrap() else {
        panic!("changed frame expected")
    };
    assert_eq!(
        frame.snapshot.as_ref().unwrap().matches[0].root.as_os_str(),
        roots[1].as_os_str()
    );
    assert_eq!(SearchPoll::Changed(frame), original);
    // Component equality alone would accept this normalized-equivalent root.
    assert!(WirePoll::from_native(original, &roots[..1]).is_err());
}

#[cfg(unix)]
#[test]
fn nested_paths_round_trip_non_utf8_bytes_and_reject_foreign_encoding() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let roots = vec![
        PathBuf::from("."),
        PathBuf::from(OsString::from_vec(b"r\xff".to_vec())),
    ];
    let mut original = sample(&roots);
    let SearchPoll::Changed(frame) = &mut original else {
        panic!("changed frame expected")
    };
    frame.snapshot.as_mut().unwrap().matches[0].path =
        PathBuf::from(OsString::from_vec(b"f\xfe".to_vec()));
    let encoded =
        serde_json::to_value(WirePoll::from_native(original.clone(), &roots).unwrap()).unwrap();
    assert_eq!(
        encoded["frame"]["snapshot"]["matches"][0]["path"],
        json!({"UnixBytes":[102,254]})
    );
    assert_eq!(
        serde_json::from_value::<WirePoll>(encoded.clone())
            .unwrap()
            .into_native(&roots)
            .unwrap(),
        original
    );
    let mut foreign = encoded;
    foreign["frame"]["snapshot"]["matches"][0]["path"] = json!({"WindowsWide":[102,254]});
    assert!(serde_json::from_value::<WirePoll>(foreign).is_err());
}

#[test]
fn invalid_result_paths_roots_and_indices_are_rejected_in_both_directions() {
    let roots = vec![PathBuf::from("a"), PathBuf::from("b")];
    let original = sample(&roots);
    let WirePoll::Changed { frame } = WirePoll::from_native(original.clone(), &roots).unwrap()
    else {
        panic!("changed frame expected")
    };
    for path in [
        PathBuf::from("../outside"),
        std::env::current_dir().unwrap().join("outside"),
    ] {
        let mut malformed = frame.clone();
        malformed.snapshot.as_mut().unwrap().matches[0].path = path.clone();
        assert!(
            WirePoll::Changed { frame: malformed }
                .into_native(&roots)
                .is_err()
        );
        let SearchPoll::Changed(mut native) = original.clone() else {
            panic!("changed frame expected")
        };
        native.snapshot.as_mut().unwrap().matches[0].path = path;
        assert!(WirePoll::from_native(SearchPoll::Changed(native), &roots).is_err());
    }
    let mut malformed = frame.clone();
    malformed.snapshot.as_mut().unwrap().matches[0].root_index = 2;
    assert!(
        WirePoll::Changed { frame: malformed }
            .into_native(&roots)
            .is_err()
    );
    for indices in [vec![1, 1], vec![2, 1]] {
        let mut malformed = frame.clone();
        malformed.snapshot.as_mut().unwrap().matches[0].indices = Some(indices);
        assert!(
            WirePoll::Changed { frame: malformed }
                .into_native(&roots)
                .is_err()
        );
    }
    let mut root_entry = frame;
    root_entry.snapshot.as_mut().unwrap().matches[0].path = PathBuf::new();
    assert!(
        WirePoll::Changed { frame: root_entry }
            .into_native(&roots)
            .is_ok()
    );
}

#[test]
fn impossible_completion_and_counts_fail_instead_of_becoming_idle() {
    let roots = vec![PathBuf::from("a"), PathBuf::from("b")];
    let WirePoll::Changed { frame } = WirePoll::from_native(sample(&roots), &roots).unwrap() else {
        panic!("changed frame expected")
    };
    let mut incomplete = frame.clone();
    incomplete.snapshot.as_mut().unwrap().walk_complete = false;
    assert!(
        WirePoll::Changed { frame: incomplete }
            .into_native(&roots)
            .is_err()
    );
    let mut inconsistent = frame.clone();
    inconsistent.snapshot.as_mut().unwrap().scanned_file_count = WireU64(0);
    assert!(
        WirePoll::Changed {
            frame: inconsistent
        }
        .into_native(&roots)
        .is_err()
    );
    let mut missing = frame;
    missing.snapshot = None;
    assert!(
        WirePoll::Changed { frame: missing }
            .into_native(&roots)
            .is_err()
    );
    let SearchPoll::Changed(mut native) = sample(&roots) else {
        panic!("changed frame expected")
    };
    native.snapshot.as_mut().unwrap().query_id -= 1;
    assert!(WirePoll::from_native(SearchPoll::Changed(native), &roots).is_err());
}

#[test]
fn budget_and_options_reject_zero_and_preserve_supported_policy() {
    let options = FileSearchOptions {
        exclude: vec!["**/.cache/**".to_owned()],
        compute_indices: true,
        respect_gitignore: false,
        ..FileSearchOptions::default()
    };
    let wire = WireOptions::from_native(&options).unwrap();
    assert_eq!(
        WireOptions::from_native(&wire.clone().into_native().unwrap()).unwrap(),
        wire
    );
    for bad in [
        WireOptions {
            limit: 0,
            ..wire.clone()
        },
        WireOptions { threads: 0, ..wire },
    ] {
        assert!(bad.into_native().is_err());
    }
    let budget = SearchBudget {
        max_index_entries: NonZeroUsize::new(17).unwrap(),
        max_index_bytes: NonZeroUsize::new(2048).unwrap(),
        max_worker_threads: NonZeroUsize::new(6).unwrap(),
    };
    let wire = WireBudget::from_native(budget);
    assert_eq!(wire.into_native().unwrap(), budget);
    assert!(
        WireBudget {
            max_index_bytes: WireU64(0),
            ..wire
        }
        .into_native()
        .is_err()
    );
}
