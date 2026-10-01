//! Primary attempt custody. Errors returned to callers are non-owning observations.
#[cfg(target_os = "linux")]
use codex_utils_pty::CommandCleanup;
#[cfg(target_os = "linux")]
use codex_utils_pty::CommandFailure;
#[cfg(target_os = "linux")]
use codex_utils_pty::CommandFailureKind;
#[cfg(target_os = "linux")]
use codex_utils_pty::OwnedBackgroundCommand;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tempfile::TempDir;

#[derive(Clone, Debug)]
pub(crate) enum SyncFailure {
    Ordinary(String),
    #[cfg(target_os = "linux")]
    Transport(CommandFailure),
    Publication(String),
    #[cfg(not(target_os = "linux"))]
    LegacyUnverified(String),
    Unwind,
    Quarantined,
}
impl SyncFailure {
    pub(crate) fn permits_fallback(&self) -> bool {
        match self {
            Self::Ordinary(_) => true,
            // Compatibility only; this disposition asserts no cleanup proof.
            #[cfg(not(target_os = "linux"))]
            Self::LegacyUnverified(_) => true,
            #[cfg(target_os = "linux")]
            Self::Transport(error) => {
                error.cleanup != CommandCleanup::Unknown
                    && error.cause != CommandFailureKind::Cancelled
            }
            _ => false,
        }
    }
    pub(crate) fn retains_resources(&self) -> bool {
        match self {
            Self::Ordinary(_) => false,
            #[cfg(not(target_os = "linux"))]
            Self::LegacyUnverified(_) => false,
            #[cfg(target_os = "linux")]
            Self::Transport(error) => error.cleanup == CommandCleanup::Unknown,
            _ => true,
        }
    }
}
impl From<String> for SyncFailure {
    fn from(message: String) -> Self {
        Self::Ordinary(message)
    }
}
impl std::fmt::Display for SyncFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ordinary(message) | Self::Publication(message) => f.write_str(message),
            #[cfg(target_os = "linux")]
            Self::Transport(error) => std::fmt::Display::fmt(error, f),
            #[cfg(not(target_os = "linux"))]
            Self::LegacyUnverified(message) => f.write_str(message),
            Self::Unwind => f.write_str("curated sync worker unwound; cleanup outcome unknown; attempt quarantined"),
            Self::Quarantined => f.write_str("curated sync is quarantined; external termination or fencing is required before recovery"),
        }
    }
}
impl std::error::Error for SyncFailure {}

#[derive(Default)]
struct Resources {
    // Resources drop in declaration order. The stable lock must be last.
    #[cfg(target_os = "linux")]
    commands: Vec<Arc<OwnedBackgroundCommand>>,
    directories: Vec<TempDir>,
    lock: Option<File>,
}
struct AttemptState {
    resources: Option<Resources>,
    outcome: Option<Result<String, SyncFailure>>,
    uncertainty: Option<SyncFailure>,
}
pub(super) struct SyncAttempt {
    pub(super) home: PathBuf,
    generation: u64,
    quarantined: AtomicBool,
    state: Mutex<AttemptState>,
    #[cfg(all(test, target_os = "linux"))]
    hook: Option<CommandHook>,
}
#[cfg(all(test, target_os = "linux"))]
pub(super) type CommandHook = Arc<
    dyn Fn(&SyncAttempt, &Arc<OwnedBackgroundCommand>, &str) -> Result<(), SyncFailure>
        + Send
        + Sync,
>;

#[derive(Default)]
struct RegistryState {
    next: u64,
    attempts: Vec<Arc<SyncAttempt>>,
}
#[derive(Default)]
pub(super) struct AttemptRegistry {
    state: Mutex<RegistryState>,
}
pub(super) static ATTEMPTS: AttemptRegistry = AttemptRegistry {
    state: Mutex::new(RegistryState {
        next: 0,
        attempts: Vec::new(),
    }),
};

impl AttemptRegistry {
    fn lock(&self) -> MutexGuard<'_, RegistryState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    fn blocked(&self, home: &Path) -> bool {
        self.lock()
            .attempts
            .iter()
            .any(|attempt| attempt.home == home && attempt.quarantined.load(Ordering::Acquire))
    }
    fn reserve(
        &self,
        home: &Path,
        #[cfg(all(test, target_os = "linux"))] hook: Option<CommandHook>,
    ) -> Result<Arc<SyncAttempt>, SyncFailure> {
        std::fs::create_dir_all(home.join(".tmp"))
            .map_err(|err| format!("failed to create curated plugins sync directory: {err}"))?;
        let home = home
            .canonicalize()
            .map_err(|err| format!("failed to resolve curated plugins home: {err}"))?;
        let mut state = self.lock();
        if state
            .attempts
            .iter()
            .any(|attempt| attempt.home == home && attempt.quarantined.load(Ordering::Acquire))
        {
            return Err(SyncFailure::Quarantined);
        }
        state.next = state.next.checked_add(1).ok_or(SyncFailure::Quarantined)?;
        let attempt = Arc::new(SyncAttempt {
            home,
            generation: state.next,
            quarantined: AtomicBool::new(false),
            state: Mutex::new(AttemptState {
                resources: Some(Resources::default()),
                outcome: None,
                uncertainty: None,
            }),
            #[cfg(all(test, target_os = "linux"))]
            hook,
        });
        state.attempts.push(Arc::clone(&attempt));
        Ok(attempt)
    }
    pub(super) fn run(
        &self,
        home: &Path,
        run: impl FnOnce(&Arc<SyncAttempt>) -> Result<String, SyncFailure>,
    ) -> Result<String, SyncFailure> {
        self.run_inner(
            home,
            #[cfg(all(test, target_os = "linux"))]
            None,
            run,
        )
    }
    fn run_inner(
        &self,
        home: &Path,
        #[cfg(all(test, target_os = "linux"))] hook: Option<CommandHook>,
        run: impl FnOnce(&Arc<SyncAttempt>) -> Result<String, SyncFailure>,
    ) -> Result<String, SyncFailure> {
        let attempt = self.reserve(
            home,
            #[cfg(all(test, target_os = "linux"))]
            hook,
        )?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            attempt.acquire_lock(self)?;
            run(&attempt)
        }))
        .unwrap_or(Err(SyncFailure::Unwind));
        self.finish(&attempt, result)
    }
    fn finish(
        &self,
        attempt: &Arc<SyncAttempt>,
        result: Result<String, SyncFailure>,
    ) -> Result<String, SyncFailure> {
        let result = match &attempt.lock().uncertainty {
            Some(error) => Err(error.clone()),
            None => result,
        };
        let retain = result.as_ref().is_err_and(SyncFailure::retains_resources);
        #[cfg(target_os = "linux")]
        let mut commands_to_stop = Vec::new();
        let (outcome, resources) = {
            let mut state = attempt.lock();
            // Completion is exact-generation and immutable, including its original Unknown.
            if let Some(outcome) = &state.outcome {
                return outcome.clone();
            }
            if retain {
                attempt.quarantined.store(true, Ordering::Release);
                #[cfg(target_os = "linux")]
                if let Some(resources) = &state.resources {
                    commands_to_stop = resources.commands.clone();
                }
            }
            state.outcome = Some(result.clone());
            (result, if retain { None } else { state.resources.take() })
        };
        #[cfg(target_os = "linux")]
        for command in commands_to_stop {
            // Latch uncertainty before supplementary cleanup. A later direct reap
            // never releases this attempt or changes its original outcome.
            command.quarantine("curated sync attempt quarantined".to_string());
            command.stop_and_observe("quarantined curated Git command", Duration::ZERO);
        }
        if !retain {
            let removed = {
                let mut state = self.lock();
                state
                    .attempts
                    .iter()
                    .position(|entry| {
                        entry.generation == attempt.generation && Arc::ptr_eq(entry, attempt)
                    })
                    .map(|index| state.attempts.remove(index))
            };
            drop(removed);
        }
        // TempDir filesystem cleanup and lock release never run under the registry mutex.
        drop(resources);
        outcome
    }
}
impl SyncAttempt {
    fn lock(&self) -> MutexGuard<'_, AttemptState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    fn acquire_lock(&self, registry: &AttemptRegistry) -> Result<(), SyncFailure> {
        let file = File::options()
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.home.join(super::CURATED_PLUGINS_SYNC_LOCK_FILE))
            .map_err(|err| format!("failed to open curated plugins sync lock: {err}"))?;
        loop {
            if registry.blocked(&self.home) {
                return Err(SyncFailure::Quarantined);
            }
            match file.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(std::fs::TryLockError::Error(err)) => {
                    return Err(format!("failed to lock curated plugins sync: {err}").into());
                }
            }
        }
        // Quarantine does not release its lock, so acquiring it excludes an older
        // same-home worker transitioning to quarantine after the admission check.
        let mut state = self.lock();
        let resources = state.resources.as_mut().ok_or(SyncFailure::Quarantined)?;
        resources.lock = Some(file);
        Ok(())
    }
    pub(super) fn keep_directory(&self, directory: TempDir) -> Result<PathBuf, SyncFailure> {
        let path = directory.path().to_path_buf();
        self.lock()
            .resources
            .as_mut()
            .ok_or(SyncFailure::Quarantined)?
            .directories
            .push(directory);
        Ok(path)
    }
    fn observe_failure(&self, error: SyncFailure) -> SyncFailure {
        let mut state = self.lock();
        if error.retains_resources() {
            self.quarantined.store(true, Ordering::Release);
            state.uncertainty.get_or_insert_with(|| error.clone());
        }
        state.uncertainty.clone().unwrap_or(error)
    }
    #[cfg(target_os = "linux")]
    pub(super) fn command(
        &self,
        command: &mut std::process::Command,
        context: &str,
        timeout: Duration,
    ) -> Result<std::process::Output, SyncFailure> {
        if let Some(error) = &self.lock().uncertainty {
            return Err(error.clone());
        }
        let result = (|| {
            let owner = OwnedBackgroundCommand::register();
            // Register with the whole attempt before anything can spawn.
            self.lock()
                .resources
                .as_mut()
                .ok_or(SyncFailure::Quarantined)?
                .commands
                .push(Arc::clone(&owner));
            owner
                .spawn(command, context, &AtomicBool::new(false))
                .map_err(SyncFailure::Transport)?;
            #[cfg(all(test, target_os = "linux"))]
            if let Some(hook) = &self.hook
                && let Err(error) = hook(self, &owner, context)
            {
                let failure = match error {
                    SyncFailure::Transport(mut failure) => {
                        failure.cleanup = CommandCleanup::Unknown;
                        failure
                    }
                    error => CommandFailure {
                        cause: CommandFailureKind::Ordinary,
                        cleanup: CommandCleanup::Unknown,
                        message: error.to_string(),
                    },
                };
                owner.quarantine(failure.message.clone());
                return Err(SyncFailure::Transport(failure));
            }
            owner
                .wait(context, timeout, &AtomicBool::new(false))
                .map_err(SyncFailure::Transport)
        })();
        result.map_err(|error| self.observe_failure(error))
    }
    #[cfg(not(target_os = "linux"))]
    pub(super) fn command(
        &self,
        command: &mut std::process::Command,
        context: &str,
        timeout: Duration,
    ) -> Result<std::process::Output, SyncFailure> {
        // The existing non-Linux runner is unchanged. Linux process custody does
        // not establish equivalent non-Linux cleanup or host-shutdown guarantees.
        super::run_git_command_with_timeout(command, context, timeout)
            .map_err(|error| self.observe_failure(SyncFailure::LegacyUnverified(error)))
    }
}

#[cfg(all(test, target_os = "linux"))]
#[path = "ownership_tests.rs"]
mod tests;
