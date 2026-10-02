use super::*;
use pretty_assertions::assert_eq;
use std::sync::mpsc;

#[test]
fn repeated_stop_preserves_first_absolute_deadline_and_joins_watchdog() -> io::Result<()> {
    let clock = Clock::default();
    let first = clock.begin_with(Duration::from_secs(/*secs*/ 1), Duration::from_secs(/*secs*/ 2), || {});
    let second = clock.begin_with(Duration::from_secs(/*secs*/ 50), Duration::from_secs(/*secs*/ 60), || {});
    assert_eq!(first.graceful(), second.graceful());
    assert_eq!(first.hard(), second.hard());
    clock.finish()?;
    clock.finish()?;
    Ok(())
}

#[test]
fn independent_expiry_remains_failure_after_watchdog_join() -> io::Result<()> {
    let (expired_tx, expired_rx) = mpsc::channel();
    let watchdog = Watchdog::start(Duration::from_millis(/*millis*/ 5), Duration::from_millis(/*millis*/ 15), move || {
        let _ = expired_tx.send(());
    })?;
    let received = expired_rx.recv_timeout(Duration::from_secs(/*secs*/ 5));
    let outcome = watchdog.finish();
    assert!(received.is_ok());
    assert_eq!(outcome.err().map(|error| error.kind()), Some(io::ErrorKind::TimedOut));
    assert!(watchdog.state.lock().unwrap_or_else(PoisonError::into_inner).worker.is_none());
    Ok(())
}

#[test]
fn disarming_wakes_and_joins_without_waiting_for_hard_timeout() -> io::Result<()> {
    let (expired_tx, expired_rx) = mpsc::channel();
    let watchdog = Watchdog::start(Duration::from_secs(/*secs*/ 60), Duration::from_secs(/*secs*/ 65), move || {
        let _ = expired_tx.send(());
    })?;
    watchdog.finish()?;
    assert!(expired_rx.try_recv().is_err());
    assert!(watchdog.state.lock().unwrap_or_else(PoisonError::into_inner).worker.is_none());
    Ok(())
}
