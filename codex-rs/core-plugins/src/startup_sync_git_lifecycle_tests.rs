use super::run_git_command_with_timeout;
use pretty_assertions::assert_eq;
use std::process::Command;
use std::time::Duration;

#[test]
fn curated_git_drains_both_pipes_before_waiting_for_exit() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "dd if=/dev/zero bs=65536 count=2 2>/dev/null; { dd if=/dev/zero bs=65536 count=2 2>/dev/null; } >&2"]);
    let output = run_git_command_with_timeout(&mut command, "pipe fixture", Duration::from_secs(2))
        .expect("bounded output must not deadlock on full pipes");
    assert!(output.status.success());
    assert_eq!(output.stdout, vec![0; 131072]);
    assert_eq!(output.stderr, vec![0; 131072]);
}

#[test]
fn curated_git_parent_exit_does_not_release_pipe_holding_helper() {
    let fixture = tempfile::tempdir().unwrap();
    let marker = fixture.path().join("helper-survived");
    let ready = fixture.path().join("helper-ready");
    let mut command = Command::new("/bin/sh");
    command.env("MARKER", &marker).env("READY", &ready).args([
        "-c",
        r#"sh -c 'printf %s "$$" >"$READY.tmp"; mv "$READY.tmp" "$READY"; sleep 1; printf escaped >"$MARKER"' & while [ ! -f "$READY" ]; do sleep 0.01; done; exit 0"#,
    ]);
    let output = run_git_command_with_timeout(
        &mut command,
        "exited parent fixture",
        Duration::from_secs(3),
    )
    .expect("successful parent output remains available after helper cleanup");
    assert!(output.status.success());
    assert_helper_stopped(&ready).unwrap();
    assert!(
        !marker.exists(),
        "helper survived parent exit and held the pipe"
    );
}

#[test]
fn curated_git_timeout_stops_parent_and_pipe_holding_helper() {
    let fixture = tempfile::tempdir().unwrap();
    let marker = fixture.path().join("helper-survived");
    let ready = fixture.path().join("helper-ready");
    let mut command = Command::new("/bin/sh");
    command.env("MARKER", &marker).env("READY", &ready).args([
        "-c",
        r#"sh -c 'printf %s "$$" >"$READY.tmp"; mv "$READY.tmp" "$READY"; sleep 1; printf escaped >"$MARKER"' & wait"#,
    ]);
    let error =
        run_git_command_with_timeout(&mut command, "timeout fixture", Duration::from_millis(500))
            .expect_err("live parent must time out");
    // Exact normal timeout rejects cleanup-uncertain error suffixes.
    assert_eq!(error, "timeout fixture timed out after 0s");
    assert_helper_stopped(&ready).unwrap();
    assert!(!marker.exists(), "helper survived direct-parent timeout");
}

fn assert_helper_stopped(pid_file: &std::path::Path) -> anyhow::Result<()> {
    let pid = std::fs::read_to_string(pid_file)?.parse::<u32>()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let path = format!("/proc/{pid}/stat");
        match std::fs::read_to_string(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Ok(status)
                if status
                    .rsplit_once(") ")
                    .is_some_and(|(_, fields)| fields.starts_with("Z ")) =>
            {
                // Terminated, awaiting its adoptive parent's reap. This unit test
                // does not own grandchildren; unchanged strict runner acceptance
                // separately requires that no descendants remain at completion.
                return Ok(());
            }
            result => {
                assert!(
                    result.is_ok(),
                    "helper status could not be read: {result:?}"
                );
                assert!(
                    std::time::Instant::now() < deadline,
                    "helper {pid} remains live"
                );
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
