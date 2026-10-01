use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn bounded_rejection_receipts_do_not_invalidate_an_older_live_lease() {
    let backend = backend(joined(), 1);
    let (service, provider) = service(backend).await;
    let request = open(&provider, 1);
    let active = request.identity.clone();
    let _: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
            .expect("open"),
    )
    .await;
    let mut oldest = None;
    for epoch in 2..=68 {
        let request = open(&provider, epoch);
        if epoch == 2 {
            oldest = Some(request.identity.clone());
        }
        let rejected: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
            service
                .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
                .expect("rejection"),
        )
        .await;
        assert!(
            matches!(rejected.reply, Reply::Error { error } if error.cleanup == StartCleanup::NotAdmitted && error.operation.kind() == SearchErrorKind::ResourceExhausted)
        );
    }
    assert_eq!(
        lock(&service.inner.state).retired.len(),
        crate::service::RETAINED_RECEIPTS
    );
    let oldest = oldest.expect("old identity");
    let expired: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &oldest)).await;
    assert!(
        matches!(expired.reply, Reply::Error { error } if error.kind() == SearchErrorKind::ClosedLease)
    );
    let closed: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &active)).await;
    assert!(matches!(closed.reply, Reply::Ok { result } if result.cleanup == CloseCleanup::Joined));
    let mut replay = open(&provider, 2);
    replay.identity = oldest;
    let rejected: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(replay))
            .expect("replay"),
    )
    .await;
    assert!(
        matches!(rejected.reply, Reply::Error { error } if error.operation.kind() == SearchErrorKind::StaleEpoch)
    );
    assert_eq!(service.shutdown().await.cleanup, CloseCleanup::Joined);
}

#[tokio::test]
async fn actual_encoded_frame_overflow_returns_domain_exhaustion_and_joins_owner() {
    let backend = Arc::new(Backend {
        starts: Arc::new(Semaphore::new(1)),
        entered: Arc::new(Semaphore::new(0)),
        begin_calls: AtomicUsize::new(0),
        start_plan: StartPlan::Ready,
        controls: Mutex::new(Vec::new()),
        shutdown_cleanup: CloseCleanup::Joined,
        session: Arc::new(Session::new(
            joined(),
            /*polls*/ 1,
            Some(SearchFrame {
                revision: 1,
                query_id: 0,
                query: String::new(),
                phase: SearchPhase::Running,
                snapshot: Some(FileSearchSnapshot {
                    query_id: 0,
                    query: String::new(),
                    total_match_count: 1,
                    scanned_file_count: 1,
                    walk_complete: false,
                    matches: vec![FileMatch {
                        score: 1,
                        path: std::path::PathBuf::from("x".repeat(300_000)),
                        root: std::path::PathBuf::from("."),
                        match_type: MatchType::File,
                        indices: None,
                    }],
                }),
            }),
        )),
    });
    let (service, provider) = service(Arc::clone(&backend)).await;
    let request = open(&provider, 1);
    let identity = request.identity.clone();
    let _: WireReply<LeaseIdentity, OpenResponse, SearchStartError> = receive(
        service
            .admit(OPEN_METHOD, ServiceLane::Ordinary, json(request))
            .expect("open"),
    )
    .await;
    let response: WireReply<LeaseIdentity, WirePoll> = receive(
        service
            .admit(
                POLL_METHOD,
                ServiceLane::Ordinary,
                json(PollRequest {
                    contract_version: FILE_SEARCH_CONTRACT_VERSION,
                    identity: identity.clone(),
                    after_revision: WireU64(0),
                    wait_ms: 0,
                }),
            )
            .expect("poll"),
    )
    .await;
    assert!(
        matches!(response.reply, Reply::Error { error } if error.kind() == SearchErrorKind::ResourceExhausted)
    );
    let closed: WireReply<LeaseIdentity, WireCloseOutcome> =
        receive(release(&service, &identity)).await;
    assert!(
        matches!(closed.reply, Reply::Ok { result } if result.cleanup == CloseCleanup::Joined && matches!(&result.operation, OperationOutcome::Error { error } if error.kind() == SearchErrorKind::ResourceExhausted))
    );
    assert_eq!(backend.session.close_calls.load(Ordering::SeqCst), 1);
    let outcome = service.shutdown().await;
    assert_eq!(outcome.cleanup, CloseCleanup::Joined);
    assert!(
        matches!(outcome.operation, Err(error) if error.kind() == SearchErrorKind::ResourceExhausted)
    );
}
