//! Linux bounded background commands with owned process groups and output pipes.
//!
//! The leader stays waitable until its process group has been signalled. This
//! pins its PID/PGID and avoids signalling a recycled identifier after reaping.

use std::io::Read;
use std::io::{self};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::sync::mpsc::Sender;

use crate::child::reaper::ChildToReap;
use crate::child::reaper::{self};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

const OUTPUT_LIMIT: usize = 1024 * 1024;
const CLEANUP_BUDGET: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

struct OwnedGroup {
    child: Child,
    owns_child: bool,
    reaper: Sender<ChildToReap>,
}

impl Drop for OwnedGroup {
    fn drop(&mut self) {
        if !self.owns_child {
            return;
        }
        // Signal before transferring exclusive unreaped PID ownership. Reserve
        // the shared reaper before spawn, so Drop never starts or joins a thread.
        let _ = crate::process_group::kill_process_group(self.child.id());
        let _ = self
            .reaper
            .send(ChildToReap::Native(self.child.id() as libc::pid_t));
    }
}

fn set_nonblocking(pipe: &impl AsRawFd) -> io::Result<()> {
    let descriptor = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags == -1
        || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn exited_without_reaping(pid: u32) -> io::Result<bool> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid,
            &mut info,
            libc::WEXITED | libc::WNOWAIT | libc::WNOHANG,
        )
    };
    if result == -1 {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
    Ok(unsafe { info.si_pid() } != 0)
}

fn read_available(
    pipe: &mut (impl Read + ?Sized),
    output: &mut Vec<u8>,
) -> io::Result<(bool, bool)> {
    let mut chunk = [0; 8192];
    let mut overflow = false;
    // Bound work per pipe so one prolific writer cannot starve cancellation
    // or the other pipe. Retain bounded output but continue draining excess.
    for _ in 0..8 {
        match pipe.read(&mut chunk) {
            Ok(0) => return Ok((true, overflow)),
            Ok(size) => {
                let retain = size.min(OUTPUT_LIMIT.saturating_sub(output.len()));
                output.extend_from_slice(&chunk[..retain]);
                overflow |= retain != size;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok((false, overflow))
}

/// Runs one already-configured command synchronously in its own Linux process group.
/// Preserves the supplied program, arguments, environment and working directory;
/// the caller remains responsible for trust, credentials and network policy.
/// Captures at most 1 MiB per pipe and allows two seconds for forced cleanup.
/// This function does not require a Tokio runtime.
/// Cancellation, timeout and oversized output are errors, never success. The
/// supplied cancellation belongs to the caller; transport failure must not
/// clear it or grant a later fallback permission to run.
pub fn run_bounded_background_command(
    command: &mut Command,
    context: &str,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<Output, String> {
    if cancelled.load(Ordering::Acquire) {
        return Err(format!("{context} cancelled before spawn"));
    }
    let reaper =
        reaper::sender().map_err(|error| format!("failed to reserve child reaper: {error}"))?;
    command.process_group(0);
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to run {context}: {error}"))?;
    let mut group = OwnedGroup {
        child,
        owns_child: true,
        reaper,
    };
    let mut stdout_pipe =
        group.child.stdout.take().ok_or_else(|| {
            format!("missing piped stdout for {context}; cleanup outcome unknown")
        })?;
    let mut stderr_pipe =
        group.child.stderr.take().ok_or_else(|| {
            format!("missing piped stderr for {context}; cleanup outcome unknown")
        })?;
    let mut failure = set_nonblocking(&stdout_pipe)
        .and_then(|()| set_nonblocking(&stderr_pipe))
        .err()
        .map(|error| format!("failed to prepare {context} output: {error}"));
    let prepared = failure.is_none();
    let deadline = Instant::now() + timeout;
    let mut cleanup_deadline = None;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut stdout_closed = !prepared;
    let mut stderr_closed = !prepared;
    loop {
        if failure.is_none() && cancelled.load(Ordering::Acquire) {
            failure = Some(format!("{context} cancelled"));
        }
        if failure.is_none() && Instant::now() >= deadline {
            failure = Some(format!("{context} timed out after {}s", timeout.as_secs()));
        }
        let exited = match exited_without_reaping(group.child.id()) {
            Ok(exited) => exited,
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    // Ownership was lost to an external waiter. Do not signal
                    // a numeric ID that could now identify a different process.
                    group.owns_child = false;
                    return Err(format!("lost child ownership for {context}: {error}"));
                }
                failure.get_or_insert_with(|| format!("failed to inspect {context}: {error}"));
                false
            }
        };
        // Even successful leader exit must not release still-running helpers.
        // Signal before wait(), while the zombie leader still reserves its ID.
        if cleanup_deadline.is_none() && (failure.is_some() || exited) {
            if let Err(error) = crate::process_group::kill_process_group(group.child.id()) {
                failure.get_or_insert_with(|| format!("failed to stop {context} group: {error}"));
            }
            cleanup_deadline = Some(Instant::now() + CLEANUP_BUDGET);
        }
        if prepared {
            for (pipe, output, closed) in [
                (
                    &mut stdout_pipe as &mut dyn Read,
                    &mut stdout,
                    &mut stdout_closed,
                ),
                (
                    &mut stderr_pipe as &mut dyn Read,
                    &mut stderr,
                    &mut stderr_closed,
                ),
            ] {
                if *closed {
                    continue;
                }
                match read_available(pipe, output) {
                    Ok((eof, overflow)) => {
                        *closed = eof;
                        if overflow {
                            failure.get_or_insert_with(|| {
                                format!("{context} output exceeds {OUTPUT_LIMIT} bytes per pipe")
                            });
                        }
                    }
                    Err(error) => {
                        *closed = true;
                        failure.get_or_insert_with(|| {
                            format!("failed to read {context} output: {error}")
                        });
                    }
                }
            }
        }
        if exited && stdout_closed && stderr_closed {
            // waitid(WNOWAIT) confirmed termination; this wait reaps exactly
            // our direct child and cannot wait for a still-running leader.
            let status = match group.child.wait() {
                Ok(status) => status,
                Err(error) => {
                    if error.raw_os_error() == Some(libc::ECHILD) {
                        // Another waiter may have raced the WNOWAIT observation.
                        // Never let Drop signal an identifier we no longer own.
                        group.owns_child = false;
                    }
                    return Err(format!(
                        "failed to reap {context}: {error}; cleanup outcome unknown"
                    ));
                }
            };
            group.owns_child = false;
            return match failure {
                Some(error) => Err(error),
                None => Ok(Output {
                    status,
                    stdout,
                    stderr,
                }),
            };
        }
        if cleanup_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(format!(
                "{}; cleanup could not confirm child exit and pipe closure; descendants may remain",
                failure.unwrap_or_else(|| format!("{context} cleanup timed out"))
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
#[path = "bounded_command_tests.rs"]
mod tests;
