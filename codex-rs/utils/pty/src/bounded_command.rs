//! Registered ownership for bounded Linux commands. Unknown cleanup is sticky.
use crate::child::reaper::ChildToReap;
use crate::child::reaper::{self};
use std::io::Read;
use std::io::{self};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::ChildStderr;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::time::Instant;

const OUTPUT_LIMIT: usize = 1024 * 1024;
const CLEANUP_BUDGET: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandFailureKind {
    Ordinary,
    Cancelled,
    Timeout,
    IdentityLost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandCleanup {
    NotStarted,
    /// Direct child reaped and both pipes reached EOF after group control.
    /// This is not a universal descendant fence or recovery of earlier Unknown.
    DirectChildAndPipes,
    Unknown,
}
#[derive(Clone, Debug)]
pub struct CommandFailure {
    pub cause: CommandFailureKind,
    pub cleanup: CommandCleanup,
    pub message: String,
}
impl CommandFailure {
    fn new(cause: CommandFailureKind, cleanup: CommandCleanup, message: String) -> Self {
        Self {
            cause,
            cleanup,
            message,
        }
    }
}
impl std::fmt::Display for CommandFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CommandFailure {}

struct OwnedGroup {
    child: Child,
    owns_child: bool,
    reaper: Sender<ChildToReap>,
}
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        if self.owns_child {
            let _ = crate::process_group::kill_process_group(self.child.id());
            let _ = self
                .reaper
                .send(ChildToReap::Native(self.child.id() as libc::pid_t));
        }
    }
}
#[derive(Default)]
struct CommandState {
    #[cfg(test)]
    force_signal_error: bool,
    started: bool,
    retired: bool,
    group: Option<OwnedGroup>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    first_failure: Option<CommandFailure>,
    unknown: Option<CommandFailure>,
    last_cleanup: Option<CommandCleanup>,
}

// Primary ownership is installed before spawn. Unknown entries have no production
// removal API: later direct-child/pipe observations are not a stronger group fence.
static COMMAND_OWNERS: Mutex<Vec<Arc<OwnedBackgroundCommand>>> = Mutex::new(Vec::new());
/// Single-use Linux command slot registered before spawn. The primary registry
/// retains uncertain child/pipe obligations independently of returned Arc/error
/// observers. Dropping an observer does not stop an active command or recover
/// unknown cleanup. Callers must separately retain every resource the command may
/// access (locks, repositories, temporary directories) and block retry/fallback
/// after Unknown. `wait` preserves the original Unknown; `stop_and_observe` can
/// supply later direct-child/pipe evidence but never authorizes resource release.
/// No production recovery/reset is provided; external termination or fencing is
/// required. This API makes no all-descendant or non-Linux custody guarantee.
pub struct OwnedBackgroundCommand {
    state: Mutex<CommandState>,
}
impl OwnedBackgroundCommand {
    /// Installs primary custody before a caller can spawn into this single-use slot.
    pub fn register() -> Arc<Self> {
        let owner = Arc::new(Self {
            state: Mutex::new(CommandState::default()),
        });
        COMMAND_OWNERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Arc::clone(&owner));
        owner
    }
    fn lock(&self) -> MutexGuard<'_, CommandState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    fn retire_confirmed(self: &Arc<Self>) {
        let removed = {
            let mut owners = COMMAND_OWNERS
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            owners
                .iter()
                .position(|owner| Arc::ptr_eq(owner, self))
                .map(|index| owners.remove(index))
        };
        drop(removed);
    }
    pub fn spawn(
        self: &Arc<Self>,
        command: &mut Command,
        context: &str,
        cancelled: &AtomicBool,
    ) -> Result<(), CommandFailure> {
        let mut state = self.lock();
        if let Some(error) = &state.unknown {
            return Err(error.clone());
        }
        if state.started {
            let cleanup = if state.retired {
                CommandCleanup::NotStarted
            } else {
                CommandCleanup::Unknown
            };
            let error = CommandFailure::new(
                CommandFailureKind::Ordinary,
                cleanup,
                format!("{context} command owner already used"),
            );
            if cleanup == CommandCleanup::Unknown {
                state.first_failure.get_or_insert_with(|| error.clone());
                state.unknown = Some(error.clone());
            }
            return Err(error);
        }
        state.started = true;
        let result = (|| {
            if cancelled.load(Ordering::Acquire) {
                return Err(CommandFailure::new(
                    CommandFailureKind::Cancelled,
                    CommandCleanup::NotStarted,
                    format!("{context} cancelled before spawn"),
                ));
            }
            let reaper = reaper::sender().map_err(|error| {
                CommandFailure::new(
                    CommandFailureKind::Ordinary,
                    CommandCleanup::NotStarted,
                    format!("failed to reserve child reaper: {error}"),
                )
            })?;
            command.process_group(0);
            let child = command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| {
                    CommandFailure::new(
                        CommandFailureKind::Ordinary,
                        CommandCleanup::NotStarted,
                        format!("failed to run {context}: {error}"),
                    )
                })?;
            // Child and its pipe handles enter the registered slot before any hook/poll.
            state.group = Some(OwnedGroup {
                child,
                owns_child: true,
                reaper,
            });
            let Some(group) = state.group.as_mut() else {
                return Err(CommandFailure::new(
                    CommandFailureKind::Ordinary,
                    CommandCleanup::Unknown,
                    format!("{context} lost registered child; cleanup outcome unknown"),
                ));
            };
            let stdout = group.child.stdout.take();
            let stderr = group.child.stderr.take();
            state.stdout = stdout;
            state.stderr = stderr;
            if state.stdout.is_none() || state.stderr.is_none() {
                return Err(CommandFailure::new(
                    CommandFailureKind::Ordinary,
                    CommandCleanup::Unknown,
                    format!("missing piped output for {context}; cleanup outcome unknown"),
                ));
            }
            Ok(())
        })();
        if let Err(error) = &result {
            state.first_failure = Some(error.clone());
            if error.cleanup == CommandCleanup::Unknown {
                state.unknown = Some(error.clone());
            }
            state.last_cleanup = Some(error.cleanup);
        }
        let retire = result
            .as_ref()
            .err()
            .is_some_and(|error| error.cleanup == CommandCleanup::NotStarted);
        if retire {
            state.retired = true;
            self.retire_confirmed();
        }
        drop(state);
        result
    }
    pub fn wait(
        self: &Arc<Self>,
        context: &str,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Output, CommandFailure> {
        let mut state = self.lock();
        if state.retired {
            return Err(state.first_failure.clone().unwrap_or_else(|| {
                CommandFailure::new(
                    CommandFailureKind::Ordinary,
                    CommandCleanup::NotStarted,
                    format!("{context} command already completed"),
                )
            }));
        }
        #[cfg(test)]
        let force_signal_error = state.force_signal_error;
        let result = match &mut *state {
            CommandState {
                group: Some(group),
                stdout: Some(stdout),
                stderr: Some(stderr),
                ..
            } if group.owns_child => poll_group(
                group,
                stdout,
                stderr,
                context,
                timeout,
                cancelled,
                #[cfg(test)]
                force_signal_error,
            ),
            _ => Err(CommandFailure::new(
                CommandFailureKind::IdentityLost,
                CommandCleanup::Unknown,
                format!("{context} has no waitable owned command; cleanup outcome unknown"),
            )),
        };
        let cleanup = result
            .as_ref()
            .err()
            .map_or(CommandCleanup::DirectChildAndPipes, |error| error.cleanup);
        state.last_cleanup = Some(cleanup);
        if let Err(error) = &result {
            state.first_failure.get_or_insert_with(|| error.clone());
        }
        if let Err(error) = &result
            && error.cleanup == CommandCleanup::Unknown
        {
            state.unknown.get_or_insert_with(|| error.clone());
        }
        let unknown = state.unknown.clone();
        if cleanup == CommandCleanup::DirectChildAndPipes {
            state.group.take();
            state.stdout.take();
            state.stderr.take();
        }
        if cleanup == CommandCleanup::DirectChildAndPipes && unknown.is_none() {
            state.retired = true;
            self.retire_confirmed();
        }
        drop(state);
        match unknown {
            Some(error) => Err(error),
            None => result,
        }
    }
    /// Records a non-owning failure without releasing the registered obligation.
    /// Used for a caller's lost/aborted observation; it is deliberately irreversible.
    pub fn quarantine(&self, message: String) {
        let mut state = self.lock();
        if state.retired {
            return;
        }
        let error = CommandFailure::new(
            CommandFailureKind::Ordinary,
            CommandCleanup::Unknown,
            message,
        );
        state.first_failure.get_or_insert_with(|| error.clone());
        state.unknown.get_or_insert(error);
    }
    pub fn original_failure(&self) -> Option<CommandFailure> {
        self.lock().first_failure.clone()
    }
    pub fn last_cleanup(&self) -> Option<CommandCleanup> {
        self.lock().last_cleanup
    }
    /// May stop/reap an identity still owned by this slot, but never clears an
    /// earlier Unknown registration. Direct-child observation is not recovery.
    pub fn stop_and_observe(self: &Arc<Self>, context: &str, timeout: Duration) -> CommandCleanup {
        {
            let state = self.lock();
            if state.group.is_none() {
                return state.last_cleanup.unwrap_or(CommandCleanup::Unknown);
            }
        }
        let _ = self.wait(context, timeout, &AtomicBool::new(true));
        self.last_cleanup().unwrap_or(CommandCleanup::Unknown)
    }
}

pub fn run_bounded_background_command(
    command: &mut Command,
    context: &str,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<Output, String> {
    let owner = OwnedBackgroundCommand::register();
    owner
        .spawn(command, context, cancelled)
        .map_err(|error| error.to_string())?;
    owner
        .wait(context, timeout, cancelled)
        .map_err(|error| error.to_string())
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

fn poll_group(
    group: &mut OwnedGroup,
    mut stdout_pipe: &mut ChildStdout,
    mut stderr_pipe: &mut ChildStderr,
    context: &str,
    timeout: Duration,
    cancelled: &AtomicBool,
    #[cfg(test)] force_signal_error: bool,
) -> Result<Output, CommandFailure> {
    let mut failure = set_nonblocking(stdout_pipe)
        .and_then(|()| set_nonblocking(stderr_pipe))
        .err()
        .map(|error| format!("failed to prepare {context} output: {error}"));
    let prepared = failure.is_none();
    let deadline = Instant::now() + timeout;
    let mut cleanup_deadline = None;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut cause = CommandFailureKind::Ordinary;
    let mut control_uncertain = false;
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    loop {
        if failure.is_none() && cancelled.load(Ordering::Acquire) {
            cause = CommandFailureKind::Cancelled;
            failure = Some(format!("{context} cancelled"));
        }
        if failure.is_none() && Instant::now() >= deadline {
            cause = CommandFailureKind::Timeout;
            failure = Some(format!("{context} timed out after {}s", timeout.as_secs()));
        }
        let exited = match exited_without_reaping(group.child.id()) {
            Ok(exited) => exited,
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    // Ownership was lost to an external waiter. Do not signal
                    // a numeric ID that could now identify a different process.
                    group.owns_child = false;
                    return Err(CommandFailure::new(
                        CommandFailureKind::IdentityLost,
                        CommandCleanup::Unknown,
                        format!("lost child ownership for {context}: {error}"),
                    ));
                }
                control_uncertain = true;
                failure.get_or_insert_with(|| format!("failed to inspect {context}: {error}"));
                false
            }
        };
        // Even successful leader exit must not release still-running helpers.
        // Signal before wait(), while the zombie leader still reserves its ID.
        if cleanup_deadline.is_none() && (failure.is_some() || exited) {
            #[cfg(test)]
            let signal_result = if force_signal_error {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "fixture denied group signal",
                ))
            } else {
                crate::process_group::kill_process_group(group.child.id())
            };
            #[cfg(not(test))]
            let signal_result = crate::process_group::kill_process_group(group.child.id());
            if let Err(error) = signal_result {
                control_uncertain = true;
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
                        // A read error is not EOF evidence; retain the pipe and ownership.
                        control_uncertain = true;
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
                    return Err(CommandFailure::new(
                        cause,
                        CommandCleanup::Unknown,
                        format!("failed to reap {context}: {error}; cleanup outcome unknown"),
                    ));
                }
            };
            group.owns_child = false;
            return match failure {
                Some(error) => Err(CommandFailure::new(
                    cause,
                    if control_uncertain {
                        CommandCleanup::Unknown
                    } else {
                        CommandCleanup::DirectChildAndPipes
                    },
                    error,
                )),
                None => Ok(Output {
                    status,
                    stdout,
                    stderr,
                }),
            };
        }
        if cleanup_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(CommandFailure::new(
                cause,
                CommandCleanup::Unknown,
                format!(
                    "{}; cleanup could not confirm child exit and pipe closure; descendants may remain",
                    failure.unwrap_or_else(|| format!("{context} cleanup timed out"))
                ),
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
#[path = "bounded_command_tests.rs"]
mod tests;
