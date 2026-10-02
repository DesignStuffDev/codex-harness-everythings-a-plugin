use super::*;
use pretty_assertions::assert_eq;
use std::sync::mpsc;

#[test]
fn repeated_stop_preserves_first_absolute_deadline_and_joins_watchdog() -> io::Result<()> {
    let clock = Clock::default();
    let first = clock.begin_with(
        Duration::from_secs(/*secs*/ 1),
        Duration::from_secs(/*secs*/ 2),
        || {},
    );
    let second = clock.begin_with(
        Duration::from_secs(/*secs*/ 50),
        Duration::from_secs(/*secs*/ 60),
        || {},
    );
    assert_eq!(first.graceful(), second.graceful());
    assert_eq!(first.hard(), second.hard());
    clock.finish()?;
    clock.finish()?;
    Ok(())
}

#[test]
fn independent_expiry_remains_failure_after_watchdog_join() -> io::Result<()> {
    let (expired_tx, expired_rx) = mpsc::channel();
    let watchdog = Watchdog::start(
        Duration::from_millis(/*millis*/ 5),
        Duration::from_millis(/*millis*/ 15),
        move || {
            let _ = expired_tx.send(());
        },
    )?;
    let received = expired_rx.recv_timeout(Duration::from_secs(/*secs*/ 5));
    let outcome = watchdog.finish();
    assert!(received.is_ok());
    assert_eq!(
        outcome.err().map(|error| error.kind()),
        Some(io::ErrorKind::TimedOut)
    );
    assert!(
        watchdog
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .worker
            .is_none()
    );
    Ok(())
}

#[test]
fn disarming_wakes_and_joins_without_waiting_for_hard_timeout() -> io::Result<()> {
    let (expired_tx, expired_rx) = mpsc::channel();
    let watchdog = Watchdog::start(
        Duration::from_secs(/*secs*/ 60),
        Duration::from_secs(/*secs*/ 65),
        move || {
            let _ = expired_tx.send(());
        },
    )?;
    watchdog.finish()?;
    assert!(expired_rx.try_recv().is_err());
    assert!(
        watchdog
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .worker
            .is_none()
    );
    Ok(())
}

#[test]
fn watchdog_expires_before_blocked_runtime_teardown_can_finish() -> io::Result<()> {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (dropping_tx, dropping_rx) = mpsc::channel();
    let (dropped_tx, dropped_rx) = mpsc::channel();
    let native = Arc::new(Mutex::new(None));
    let runtime_native = Arc::clone(&native);
    let runtime_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let handle = runtime.spawn_blocking(move || {
            entered_tx.send(()).unwrap();
            let _ = release_rx.recv();
        });
        *runtime_native.lock().unwrap() = Some(handle);
        dropping_tx.send(()).unwrap();
        drop(runtime);
        dropped_tx.send(()).unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    dropping_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let (expired_tx, expired_rx) = mpsc::channel();
    let clock = Clock::default();
    clock.begin_with(
        Duration::from_millis(5),
        Duration::from_millis(15),
        move || {
            let _ = expired_tx.send(());
        },
    );
    let expired = expired_rx.recv_timeout(Duration::from_secs(5));
    let premature_runtime_completion = dropped_rx.try_recv();
    // Always release the held native work before assertions or runtime join.
    let _ = release_tx.send(());
    let dropped = dropped_rx.recv_timeout(Duration::from_secs(5));
    let runtime_result = runtime_thread.join();
    let clock_result = clock.finish();
    assert!(expired.is_ok());
    assert_eq!(premature_runtime_completion, Err(mpsc::TryRecvError::Empty));
    assert!(dropped.is_ok());
    assert!(runtime_result.is_ok());
    assert_eq!(
        clock_result.err().map(|error| error.kind()),
        Some(io::ErrorKind::TimedOut)
    );
    let mut handle = native.lock().unwrap().take().unwrap();
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(matches!(
        std::future::Future::poll(std::pin::Pin::new(&mut handle), &mut context),
        std::task::Poll::Ready(Ok(()))
    ));
    Ok(())
}
