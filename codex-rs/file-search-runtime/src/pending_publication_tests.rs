use super::*;
use pretty_assertions::assert_eq;

#[test]
fn cancellation_before_control_publication_forwards_to_real_native_and_keeps_sibling() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let root = fixture();
        let (backend, provider, scope) = setup(65536);
        let sibling_reporter = Arc::new(Reporter::default());
        let sibling = scope
            .open(request(root.path()), sibling_reporter.clone())
            .await
            .unwrap();
        let (entered, entering) = oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let mut allow_native = Release(Some(release));
        let blocking = tokio::task::spawn_blocking(move || {
            let _ = entered.send(());
            let _ = blocked.recv();
        });
        timeout(WAIT, entering).await.unwrap().unwrap();
        let (entered, entering) = oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let mut publish = Release(Some(release));
        *lock(&backend.gate) = Some(Arc::new(Gate {
            entered: Mutex::new(Some(entered)),
            release: Mutex::new(blocked),
        }));
        let pending = scope
            .begin_open(request(root.path()), Arc::new(Reporter::default()))
            .unwrap();
        let control = pending.control();
        timeout(WAIT, entering).await.unwrap().unwrap();
        *lock(&backend.gate) = None;
        let actual_control = lock(&backend.controls).last().unwrap().clone();
        assert_eq!(actual_control.requests.load(Ordering::SeqCst), 0);
        control.request_cancel();
        assert_eq!(
            actual_control.requests.load(Ordering::SeqCst),
            0,
            "pending control has not been returned to the facade yet"
        );
        publish.now();
        timeout(WAIT, async {
            while actual_control.requests.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        query(&sibling, &sibling_reporter, 1).await;
        let early = timeout(Duration::from_millis(30), control.cancel_and_wait()).await;
        let charged = lock(&provider.inner.state).sessions;
        allow_native.now();
        blocking.await.unwrap();
        // The native backend may have skipped admission before reaching its
        // queued constructor. Either proven no-work or actual join is retained;
        // quota cannot be refunded while an accepted constructor is blocked.
        let receipt = timeout(WAIT, control.cancel_and_wait()).await.unwrap();
        assert_eq!(receipt.operation, Ok(()));
        assert!(matches!(
            receipt.cleanup,
            StartCleanup::NotAdmitted | StartCleanup::Confirmed
        ));
        if early.is_err() {
            assert_eq!(charged, 2);
        }
        let error = timeout(WAIT, pending.finish())
            .await
            .unwrap()
            .err()
            .unwrap();
        assert_eq!(error.operation.kind(), SearchErrorKind::ClosedLease);
        assert_eq!(error.cleanup, receipt.cleanup);
        assert_eq!(control.cancel_and_wait().await, receipt);
        query(&sibling, &sibling_reporter, 2).await;
        assert_eq!(sibling.close().await, joined());
        assert_eq!(provider.shutdown().await, joined());
    });
}
