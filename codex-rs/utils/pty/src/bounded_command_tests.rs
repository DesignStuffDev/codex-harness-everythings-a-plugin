use super::run_bounded_background_command;
use pretty_assertions::assert_eq;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

#[test]
fn cancelled_command_does_not_spawn() {
    let fixture = tempfile::tempdir().unwrap();
    let marker = fixture.path().join("spawned");
    let mut command = Command::new("/bin/sh");
    command
        .env("MARKER", &marker)
        .args(["-c", r#"printf ran >"$MARKER""#]);
    let error = run_bounded_background_command(
        &mut command,
        "cancelled fixture",
        Duration::from_secs(3),
        &AtomicBool::new(true),
    )
    .unwrap_err();
    assert!(error.contains("cancelled before spawn"), "{error}");
    assert!(!marker.exists());
}

#[test]
fn cancellation_reaps_the_owned_leader() {
    let fixture = tempfile::tempdir().unwrap();
    let pid_file = fixture.path().join("pid");
    let cancelled = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let cancel = scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while std::fs::read_to_string(&pid_file)
                .ok()
                .is_none_or(|value| value.parse::<u32>().is_err())
            {
                assert!(Instant::now() < deadline, "fixture never started");
                std::thread::sleep(Duration::from_millis(5));
            }
            cancelled.store(true, Ordering::Release);
        });
        let mut command = Command::new("/bin/sh");
        command
            .env("PID_FILE", &pid_file)
            .args(["-c", r#"printf '%s' "$$" >"$PID_FILE"; sleep 60"#]);
        let error = run_bounded_background_command(
            &mut command,
            "running cancellation fixture",
            Duration::from_secs(10),
            &cancelled,
        )
        .unwrap_err();
        // Exact normal cancellation rejects cleanup-uncertain error suffixes.
        assert_eq!(error, "running cancellation fixture cancelled");
        cancel.join().unwrap();
    });
    let pid = std::fs::read_to_string(pid_file)
        .unwrap()
        .parse::<libc::pid_t>()
        .unwrap();
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // SAFETY: WNOWAIT observes only this test's former child without stealing
    // ownership from the shared reaper if a regression transfers it there.
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    let error = std::io::Error::last_os_error();
    assert_eq!(
        result, -1,
        "normal cancellation must already have reaped its leader"
    );
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
}

#[test]
fn excessive_output_is_an_error_and_nonzero_status_is_preserved() {
    let mut huge = Command::new("/bin/sh");
    huge.args(["-c", "dd if=/dev/zero bs=65536 count=32 2>/dev/null"]);
    let error = run_bounded_background_command(
        &mut huge,
        "overflow fixture",
        Duration::from_secs(5),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(error.contains("output exceeds"), "{error}");
    let mut nonzero = Command::new("/bin/sh");
    nonzero.args(["-c", "printf diagnostic >&2; exit 7"]);
    let output = run_bounded_background_command(
        &mut nonzero,
        "status fixture",
        Duration::from_secs(3),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stderr, b"diagnostic");
}
