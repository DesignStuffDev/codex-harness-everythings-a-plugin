use crate::FileMatch;
use crate::FileSearchSnapshot;
use crate::MatchType;
use crate::ProviderLimits;
use crate::ScopeLimits;
use crate::SearchBudget;
use crate::SearchError;
use crate::SearchErrorKind;
use crate::SearchFrame;
use crate::SearchPhase;
use crate::SearchPoll;
use crate::SearchQuery;
use crate::SearchStartError;
use crate::StartCleanup;
use pretty_assertions::assert_eq;
use std::error::Error;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::path::PathBuf;

#[test]
fn existing_match_and_snapshot_json_is_preserved() -> Result<(), Box<dyn Error>> {
    let mut matched = FileMatch {
        score: 42,
        path: PathBuf::from("apple.txt"),
        match_type: MatchType::File,
        root: PathBuf::from("/project"),
        indices: None,
    };
    assert_eq!(
        serde_json::to_string(&matched)?,
        r#"{"score":42,"path":"apple.txt","match_type":"file","root":"/project"}"#,
    );
    matched.indices = Some(vec![0, 2]);
    matched.match_type = MatchType::Directory;
    let snapshot = FileSearchSnapshot {
        query_id: 3,
        query: "ap".into(),
        matches: vec![matched],
        total_match_count: 1,
        scanned_file_count: 8,
        walk_complete: true,
    };
    assert_eq!(
        serde_json::to_string(&snapshot)?,
        r#"{"query_id":3,"query":"ap","matches":[{"score":42,"path":"apple.txt","match_type":"directory","root":"/project","indices":[0,2]}],"total_match_count":1,"scanned_file_count":8,"walk_complete":true}"#,
    );
    Ok(())
}

#[test]
fn diagnostics_are_bounded_without_splitting_utf8() -> Result<(), Box<dyn Error>> {
    let prefix = "x".repeat(SearchError::MAX_MESSAGE_BYTES - 1);
    let diagnostic = format!("{prefix}💡 details");
    let error = SearchError::new(SearchErrorKind::SearchFailed, &diagnostic);
    assert_eq!(error.message(), prefix);
    assert_eq!(
        serde_json::from_str::<SearchError>(&serde_json::to_string(&error)?)?,
        error,
    );
    // Untrusted deserialization cannot bypass the retained diagnostic bound.
    let oversized = serde_json::json!({"kind": "search_failed", "message": diagnostic});
    assert!(serde_json::from_value::<SearchError>(oversized).is_err());
    Ok(())
}

#[test]
fn startup_cleanup_evidence_survives_round_trip() -> Result<(), Box<dyn Error>> {
    for cleanup in [
        StartCleanup::NotAdmitted,
        StartCleanup::Confirmed,
        StartCleanup::Unconfirmed(SearchError::new(
            SearchErrorKind::TransportLost,
            "release receipt was lost",
        )),
    ] {
        let error = SearchStartError {
            operation: SearchError::new(SearchErrorKind::ClosedLease, "closed during startup"),
            cleanup,
        };
        assert_eq!(
            serde_json::from_str::<SearchStartError>(&serde_json::to_string(&error)?)?,
            error,
        );
        assert_eq!(error.source().map(ToString::to_string), Some(error.operation.to_string()));
    }
    Ok(())
}

#[test]
fn zero_budget_fields_cannot_bypass_validation_through_serde() {
    for field in ["max_index_entries", "max_index_bytes", "max_worker_threads"] {
        let mut encoded = serde_json::json!({
            "max_index_entries": 1,
            "max_index_bytes": 1,
            "max_worker_threads": 1,
        });
        encoded[field] = serde_json::json!(0);
        assert!(serde_json::from_value::<SearchBudget>(encoded).is_err());
    }
}

#[test]
fn resource_allocations_are_rejected_instead_of_clamped() -> Result<(), Box<dyn Error>> {
    let ceiling = SearchBudget {
        max_index_entries: NonZeroUsize::new(5).ok_or("nonzero entries")?,
        max_index_bytes: NonZeroUsize::new(40).ok_or("nonzero bytes")?,
        max_worker_threads: NonZeroUsize::new(4).ok_or("nonzero workers")?,
    };
    ceiling.validate_within(&ceiling)?;
    for allocation in [
        SearchBudget { max_index_entries: NonZeroUsize::new(6).ok_or("entries")?, ..ceiling },
        SearchBudget { max_index_bytes: NonZeroUsize::new(41).ok_or("bytes")?, ..ceiling },
        SearchBudget { max_worker_threads: NonZeroUsize::new(5).ok_or("workers")?, ..ceiling },
    ] {
        assert_eq!(
            allocation.validate_within(&ceiling),
            Err(SearchError::new(
                SearchErrorKind::ResourceExhausted,
                "file-search resource allocation exceeds the provider ceiling",
            )),
        );
    }
    let provider = ProviderLimits {
        max_scopes: NonZeroUsize::new(2).ok_or("scopes")?,
        max_sessions: NonZeroUsize::new(3).ok_or("sessions")?,
        resources: ceiling,
    };
    let scope = ScopeLimits { max_sessions: NonZeroUsize::new(4).ok_or("scope sessions")? };
    assert_eq!(
        scope.validate_within(&provider),
        Err(SearchError::new(
            SearchErrorKind::ResourceExhausted,
            "file-search scope allowance exceeds the provider session ceiling",
        )),
    );
    Ok(())
}

#[test]
fn query_admission_checks_identity_and_utf8_bytes() -> Result<(), Box<dyn Error>> {
    let mut query = SearchQuery {
        id: NonZeroU64::new(3).ok_or("query id")?,
        text: "éé".into(),
    };
    let size_limit = NonZeroUsize::new(4).ok_or("query bytes")?;
    query.validate_after(/*previous_id*/ 2, size_limit)?;
    assert_eq!(
        query.validate_after(/*previous_id*/ 3, size_limit),
        Err(SearchError::new(
            SearchErrorKind::StaleEpoch,
            "file-search query identity must strictly increase",
        )),
    );
    query.text.push('é');
    assert_eq!(
        query.validate_after(/*previous_id*/ 2, size_limit),
        Err(SearchError::new(
            SearchErrorKind::ResourceExhausted,
            "file-search query exceeds the UTF-8 byte limit",
        )),
    );
    query.text.clear();
    query.validate_after(/*previous_id*/ 2, size_limit)?;
    Ok(())
}

#[test]
fn frame_rejects_relabeling_and_missing_idle_snapshot() -> Result<(), Box<dyn Error>> {
    let snapshot = FileSearchSnapshot {
        query_id: 3,
        query: "apple".into(),
        walk_complete: true,
        ..FileSearchSnapshot::default()
    };
    let mut frame = SearchFrame {
        revision: 7,
        query_id: 3,
        query: "apple".into(),
        snapshot: Some(snapshot),
        phase: SearchPhase::Idle,
    };
    frame.validate()?;
    // Same text can be a later a/b/a query; identity must still match.
    frame.query_id = 5;
    assert!(frame.validate().is_err());
    frame.query_id = 3;
    frame.query = "banana".into();
    assert!(frame.validate().is_err());
    frame.query = "apple".into();
    frame.snapshot = None;
    assert!(frame.validate().is_err());
    frame.phase = SearchPhase::Cancelled;
    frame.validate()?;
    frame.phase = SearchPhase::Running;
    frame.query_id = 0;
    assert!(frame.validate().is_err());
    frame.query.clear();
    frame.validate()?;
    Ok(())
}

#[test]
fn idle_requires_a_complete_index_and_snapshot_counts_must_agree() -> Result<(), Box<dyn Error>> {
    let matched = FileMatch {
        score: 1,
        path: PathBuf::from("apple"),
        match_type: MatchType::File,
        root: PathBuf::from("/project"),
        indices: None,
    };
    let mut snapshot = FileSearchSnapshot {
        query_id: 1,
        query: "apple".into(),
        matches: vec![matched],
        total_match_count: 1,
        scanned_file_count: 1,
        walk_complete: false,
    };
    let mut frame = SearchFrame {
        revision: 1,
        query_id: 1,
        query: "apple".into(),
        snapshot: Some(snapshot.clone()),
        phase: SearchPhase::Running,
    };
    frame.validate()?;
    frame.phase = SearchPhase::Idle;
    assert!(frame.validate().is_err());
    snapshot.walk_complete = true;
    frame.snapshot = Some(snapshot.clone());
    frame.validate()?;
    snapshot.total_match_count = 0;
    frame.snapshot = Some(snapshot.clone());
    assert!(frame.validate().is_err());
    snapshot.total_match_count = 2;
    frame.snapshot = Some(snapshot);
    assert!(frame.validate().is_err());
    Ok(())
}

#[test]
fn poll_accepts_revision_gaps_but_rejects_stale_or_fabricated_change() -> Result<(), Box<dyn Error>> {
    let changed = SearchPoll::Changed(SearchFrame {
        revision: 9,
        query_id: 1,
        query: "apple".into(),
        snapshot: None,
        phase: SearchPhase::Running,
    });
    changed.validate_after(/*after_revision*/ 2)?;
    assert!(changed.validate_after(/*after_revision*/ 9).is_err());
    assert!(changed.validate_after(/*after_revision*/ u64::MAX).is_err());
    let unchanged = SearchPoll::Unchanged { revision: 9 };
    unchanged.validate_after(/*after_revision*/ 9)?;
    assert!(unchanged.validate_after(/*after_revision*/ 2).is_err());
    Ok(())
}
