//! Policy rejection, real resource failure and lexical path contract checks.

use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn spare_capacity_inputs_preserve_real_root_exclusion_and_query_semantics() {
    let root = fixture();
    let mut root_path = PathBuf::with_capacity(1024 * 1024);
    root_path.push(root.path());
    let mut roots = Vec::with_capacity(128);
    roots.push(root_path);
    let mut excluded = String::with_capacity(1024 * 1024);
    excluded.push_str("beta*");
    let mut excludes = Vec::with_capacity(128);
    excludes.push(excluded);
    let mut request = request(root.path());
    request.roots = roots;
    request.options.exclude = excludes;
    // Admission measures logical inputs. Retaining the caller's large spare
    // allocations is not required to preserve their authorized root or text.
    let mut policy = limits(1);
    policy.max_roots_options_bytes = nonzero(
        std::mem::size_of::<PathBuf>()
            + std::mem::size_of::<String>()
            + root.path().as_os_str().as_encoded_bytes().len()
            + "beta*".len(),
    );
    policy.max_query_bytes = nonzero(5);
    let backend = backend(policy);
    let session = open(&backend, request).await;
    let mut text = String::with_capacity(1024 * 1024);
    text.push_str("alpha");
    let accepted = session
        .update_query(SearchQuery {
            id: NonZeroU64::new(1).unwrap(),
            text,
        })
        .await
        .unwrap();
    assert_eq!(accepted.id.get(), 1);
    let mut cursor = 0;
    let snapshot = settled(session.as_ref(), 1, "alpha", &mut cursor).await;
    assert_eq!(snapshot.matches.len(), 1);
    assert_eq!(snapshot.matches[0].path, Path::new("alpha.txt"));
    assert_eq!(
        snapshot.matches[0].root.as_os_str(),
        root.path().as_os_str()
    );
    update(session.as_ref(), 2, "beta").await;
    let excluded = settled(session.as_ref(), 2, "beta", &mut cursor).await;
    assert!(excluded.matches.is_empty());
    assert_eq!(excluded.total_match_count, 0);
    update(session.as_ref(), 3, "alpha").await;
    let restored = settled(session.as_ref(), 3, "alpha", &mut cursor).await;
    assert_eq!(restored.matches, snapshot.matches);
    assert_eq!(close(session.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}

#[tokio::test]
async fn rejected_query_and_poll_inputs_do_not_advance_identity_or_poison_shutdown() {
    let root = fixture();
    let mut policy = limits(1);
    policy.max_query_bytes = nonzero(3);
    let backend = backend(policy);
    let session = open(&backend, request(root.path())).await;
    assert_eq!(
        session
            .update_query(query(1, "😃"))
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::ResourceExhausted
    );
    update(session.as_ref(), 1, "中").await;
    let mut cursor = 0;
    let snapshot = settled(session.as_ref(), 1, "中", &mut cursor).await;
    assert_eq!(snapshot.matches[0].path, Path::new("中.txt"));
    assert_eq!(
        session
            .update_query(query(1, "中"))
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::StaleEpoch
    );
    assert_eq!(
        session
            .next_snapshot(u64::MAX, Duration::ZERO)
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::InvalidInput
    );
    assert_eq!(
        session
            .next_snapshot(cursor, RETAINED_WAIT + Duration::from_secs(1))
            .await
            .unwrap_err()
            .kind(),
        SearchErrorKind::InvalidInput
    );
    assert_eq!(
        session.next_snapshot(cursor, Duration::ZERO).await.unwrap(),
        SearchPoll::Unchanged { revision: cursor }
    );
    update(session.as_ref(), 2, "中").await;
    settled(session.as_ref(), 2, "中", &mut cursor).await;
    assert_eq!(close(session.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}

#[tokio::test]
async fn rejected_open_policy_inputs_do_not_consume_ownership_or_poison_shutdown() {
    let root = fixture();
    let backend = backend(limits(1));
    let mut cases = Vec::new();
    let mut empty = request(root.path());
    empty.roots.clear();
    cases.push((empty, SearchErrorKind::InvalidInput));
    let mut excluded = request(root.path());
    excluded.options.exclude.push("[".to_owned());
    cases.push((excluded, SearchErrorKind::InvalidInput));
    let mut workers = request(root.path());
    workers.budget.max_worker_threads = nonzero(1);
    cases.push((workers, SearchErrorKind::ResourceExhausted));
    let mut storage = request(root.path());
    storage.budget.max_index_bytes = nonzero(1);
    cases.push((storage, SearchErrorKind::ResourceExhausted));
    let mut matches = request(root.path());
    matches.options.limit = nonzero(33);
    cases.push((matches, SearchErrorKind::ResourceExhausted));
    let mut allocation = request(root.path());
    allocation.budget.max_index_entries = nonzero(65);
    cases.push((allocation, SearchErrorKind::ResourceExhausted));
    for (request, expected) in cases {
        let rejected = backend.open(request).await.err().unwrap();
        assert_eq!(rejected.operation.kind(), expected);
        assert_eq!(rejected.cleanup, StartCleanup::NotAdmitted);
    }
    let session = open(&backend, request(root.path())).await;
    update(session.as_ref(), 1, "alpha").await;
    settled(session.as_ref(), 1, "alpha", &mut 0).await;
    assert_eq!(close(session.as_ref()).await.operation, Ok(()));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));

    let mut policy = limits(1);
    policy.max_roots_options_bytes = nonzero(1);
    let limited =
        crate::NativeSearchBackend::new(std::env::current_dir().unwrap(), policy).unwrap();
    let rejected = limited.open(request(root.path())).await.err().unwrap();
    assert_eq!(
        rejected.operation.kind(),
        SearchErrorKind::ResourceExhausted
    );
    assert_eq!(rejected.cleanup, StartCleanup::NotAdmitted);
    assert_eq!(shutdown(&limited).await.operation, Ok(()));
}

#[tokio::test]
async fn real_entry_exhaustion_retains_failure_but_joined_cleanup_returns_capacity() {
    let root = fixture();
    let empty = tempfile::tempdir().unwrap();
    let mut policy = limits(1);
    policy.resources.max_index_entries = nonzero(1);
    let backend = backend(policy);
    let mut exhausted_request = request(root.path());
    exhausted_request.budget.max_index_entries = nonzero(1);
    let operation = match timeout(WAIT, backend.open(exhausted_request))
        .await
        .unwrap()
    {
        Ok(session) => {
            let failure = failed(session.as_ref()).await;
            let outcome = close(session.as_ref()).await;
            assert_eq!(outcome.operation, Err(failure.clone()));
            assert_eq!(close(session.as_ref()).await, outcome);
            failure
        }
        Err(error) => {
            assert_eq!(error.cleanup, StartCleanup::Confirmed);
            error.operation
        }
    };
    assert_eq!(operation.kind(), SearchErrorKind::ResourceExhausted);

    let mut replacement_request = request(empty.path());
    replacement_request.budget.max_index_entries = nonzero(1);
    let replacement = open(&backend, replacement_request).await;
    update(replacement.as_ref(), 1, "absent").await;
    let snapshot = settled(replacement.as_ref(), 1, "absent", &mut 0).await;
    assert!(snapshot.matches.is_empty());
    assert_eq!(snapshot.total_match_count, 0);
    assert_eq!(close(replacement.as_ref()).await.operation, Ok(()));
    let outcome = shutdown(&backend).await;
    assert_eq!(outcome.operation, Err(operation));
    assert_eq!(shutdown(&backend).await, outcome);
}

#[tokio::test]
async fn real_native_output_exhaustion_is_typed_and_never_reports_successful_idle() {
    let root = fixture();
    let mut policy = limits(1);
    policy.max_snapshot_bytes = nonzero(1);
    let backend = backend(policy);
    let operation = match timeout(WAIT, backend.open(request(root.path())))
        .await
        .unwrap()
    {
        Ok(session) => {
            let failure = failed(session.as_ref()).await;
            let outcome = close(session.as_ref()).await;
            assert_eq!(outcome.operation, Err(failure.clone()));
            assert_eq!(close(session.as_ref()).await, outcome);
            failure
        }
        Err(error) => {
            assert_eq!(error.cleanup, StartCleanup::Confirmed);
            error.operation
        }
    };
    assert_eq!(operation.kind(), SearchErrorKind::ResourceExhausted);
    let outcome = shutdown(&backend).await;
    assert_eq!(outcome.operation, Err(operation));
    assert_eq!(shutdown(&backend).await, outcome);
}

#[tokio::test]
async fn dropped_close_and_shutdown_observers_leave_retained_joined_receipts() {
    let root = fixture();
    let backend = backend(limits(1));
    let session = open(&backend, request(root.path())).await;
    update(session.as_ref(), 1, "alpha").await;
    settled(session.as_ref(), 1, "alpha", &mut 0).await;
    let mut observer = session.close();
    poll_fn(|context| {
        // Either readiness is legal; dropping this observer never owns cleanup.
        let _ = observer.as_mut().poll(context);
        Poll::Ready(())
    })
    .await;
    drop(observer);
    let closed = close(session.as_ref()).await;
    assert_eq!(closed.operation, Ok(()));
    assert_eq!(close(session.as_ref()).await, closed);

    let mut observer = backend.shutdown();
    poll_fn(|context| {
        let _ = observer.as_mut().poll(context);
        Poll::Ready(())
    })
    .await;
    drop(observer);
    let stopped = shutdown(&backend).await;
    assert_eq!(stopped.operation, Ok(()));
    assert_eq!(shutdown(&backend).await, stopped);
    let rejected = backend.open(request(root.path())).await.err().unwrap();
    assert_eq!(rejected.operation.kind(), SearchErrorKind::ClosedLease);
    assert_eq!(rejected.cleanup, StartCleanup::NotAdmitted);
}

#[tokio::test]
async fn relative_and_absolute_lexical_roots_keep_their_spelling_without_chdir() {
    let original_cwd = std::env::current_dir().unwrap();
    let root = tempfile::Builder::new()
        .prefix("native-backend-roots-")
        .tempdir_in(&original_cwd)
        .unwrap();
    write_files(root.path());
    let name = root.path().file_name().unwrap();
    let relative = Path::new(".").join(name).join(".");
    let absolute = original_cwd.join(name).join(".");
    let backend = backend(limits(2));
    for spelling in [relative, absolute] {
        let session = open(&backend, request(&spelling)).await;
        update(session.as_ref(), 1, "alpha").await;
        let snapshot = settled(session.as_ref(), 1, "alpha", &mut 0).await;
        assert_eq!(snapshot.matches.len(), 1);
        let matched = &snapshot.matches[0];
        assert_eq!(matched.path, Path::new("alpha.txt"));
        assert_eq!(matched.root.as_os_str(), spelling.as_os_str());
        assert!(matched.full_path().is_file());
        assert_eq!(
            std::env::current_dir().unwrap().as_os_str(),
            original_cwd.as_os_str()
        );
        assert_eq!(close(session.as_ref()).await.operation, Ok(()));
    }
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
    assert_eq!(
        std::env::current_dir().unwrap().as_os_str(),
        original_cwd.as_os_str()
    );
}

#[tokio::test]
async fn constructor_rejects_relative_mismatched_or_unnormalized_base_and_bad_poll_policy() {
    let cwd = std::env::current_dir().unwrap();
    let other = tempfile::tempdir().unwrap();
    for base in [
        PathBuf::from("."),
        other.path().to_path_buf(),
        cwd.join("."),
    ] {
        let error = NativeSearchBackend::new(base, limits(1)).err().unwrap();
        assert_eq!(error.kind(), SearchErrorKind::InvalidInput);
    }
    for wait in [Duration::ZERO, Duration::from_secs(61)] {
        let mut policy = limits(1);
        policy.max_poll_wait = wait;
        let error = NativeSearchBackend::new(cwd.clone(), policy).err().unwrap();
        assert_eq!(error.kind(), SearchErrorKind::InvalidInput);
    }
    assert_eq!(
        std::env::current_dir().unwrap().as_os_str(),
        cwd.as_os_str()
    );
    let backend = backend(limits(1));
    assert_eq!(shutdown(&backend).await.operation, Ok(()));
}
