//! Staged session ownership for an explicitly selected replay component.
//! A callable Arc never owns terminal lifetime: the separate nonclone owner does.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use codex_component_host::ComponentBinding;
use codex_context_replay::ReplayInput;
use codex_context_replay_component::ProcessReplay;
use codex_context_replay_component::ReplayComponentError;
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

type Status = Option<Result<(), ReplayComponentError>>;

/// Captured only in the host, never supplied or restored by component JSON.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReplayTicket {
    thread_id: ThreadId,
    session_id: SessionId,
    epoch: Uuid,
}

struct Shared {
    live: Mutex<bool>,
    process: OnceLock<Arc<ProcessReplay>>,
    closing: CancellationToken,
    ready: watch::Sender<Status>,
    completion: watch::Sender<Status>,
}

impl Shared {
    fn revoke(&self) {
        let mut live = self.live.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        *live = false;
        if let Some(process) = self.process.get() {
            process.begin_close();
        }
        self.closing.cancel();
    }
}

/// One immutable selected connection per session incarnation.
pub(crate) struct SessionReplay {
    owner: ReplayTicket,
    shared: Arc<Shared>,
}

/// Terminal ownership starts before acquisition and transfers into the session loop.
/// Drop revokes immediately; the independent supervisor owns bounded cleanup.
pub(crate) struct ReplayOwner {
    replay: Arc<SessionReplay>,
}

impl Drop for ReplayOwner {
    fn drop(&mut self) {
        self.replay.begin_close();
    }
}

impl ReplayOwner {
    pub(crate) async fn close(self) -> CodexResult<()> {
        self.replay.close().await
    }
}

/// Held only together with the session state lock and never across an await.
pub(crate) struct ReplayCommitGuard<'a> {
    _live: MutexGuard<'a, bool>,
}

impl SessionReplay {
    /// No await occurs before the caller receives the terminal owner. Acquisition
    /// runs on a long-lived runtime worker and survives cancellation of readiness.
    pub(crate) fn start(
        binding: ComponentBinding,
        thread_id: ThreadId,
        session_id: SessionId,
    ) -> (Arc<Self>, ReplayOwner) {
        let (ready, _) = watch::channel(None);
        let (completion, _) = watch::channel(None);
        let shared = Arc::new(Shared {
            live: Mutex::new(true),
            process: OnceLock::new(),
            closing: CancellationToken::new(),
            ready,
            completion,
        });
        let replay = Arc::new(Self {
            owner: ReplayTicket { thread_id, session_id, epoch: Uuid::new_v4() },
            shared: Arc::clone(&shared),
        });
        let acquiring = Arc::clone(&shared);
        let supervisor = tokio::spawn(async move {
            if acquiring.closing.is_cancelled() {
                return Ok(());
            }
            let process = Arc::new(ProcessReplay::connect(binding).await?);
            acquiring.process.set(Arc::clone(&process))
                .map_err(|_| ReplayComponentError::Startup)?;
            let live = *acquiring.live.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            acquiring.ready.send_replace(Some(if live { Ok(()) } else { Err(ReplayComponentError::Startup) }));
            acquiring.closing.cancelled().await;
            process.close().await
        });
        // Observe task panics and complete both wait channels. A retained service
        // Arc cannot keep admission open after terminal ownership is revoked.
        tokio::spawn(async move {
            let outcome = supervisor.await.unwrap_or(Err(ReplayComponentError::Shutdown));
            shared.revoke();
            let closed = match shared.process.get() {
                Some(process) => process.close().await,
                None => Ok(()),
            };
            shared.ready.send_if_modified(|ready| {
                if ready.is_none() {
                    *ready = Some(Err(ReplayComponentError::Startup));
                    true
                } else {
                    false
                }
            });
            shared.completion.send_replace(Some(outcome.and(closed)));
        });
        let owner = ReplayOwner { replay: Arc::clone(&replay) };
        (replay, owner)
    }

    pub(crate) async fn ready(&self) -> CodexResult<()> {
        wait_for(self.shared.ready.subscribe()).await
    }

    pub(crate) async fn reconstruct(
        &self,
        input: ReplayInput,
    ) -> CodexResult<(ReplayTicket, super::rollout_reconstruction::RolloutReconstruction)> {
        // Validate before acquisition of data; the final commit validates again.
        drop(self.commit_guard(&self.owner, self.owner.thread_id, self.owner.session_id)?);
        let process = self.shared.process.get().ok_or_else(|| {
            CodexErr::Fatal("context replay component is not ready".to_owned())
        })?;
        let output = process.reconstruct(input).await.map_err(component_error)?;
        Ok((self.owner, output.into()))
    }

    /// Lock order: SessionState, then this gate. Invalidation takes only this
    /// gate or takes it under SessionState. There is no inverse acquisition.
    pub(crate) fn commit_guard(
        &self,
        ticket: &ReplayTicket,
        thread_id: ThreadId,
        session_id: SessionId,
    ) -> CodexResult<ReplayCommitGuard<'_>> {
        let live = self.shared.live.lock().map_err(|_| {
            CodexErr::Fatal("context replay ownership is unavailable".to_owned())
        })?;
        if !*live || ticket != &self.owner || ticket.thread_id != thread_id || ticket.session_id != session_id {
            return Err(CodexErr::Fatal("context replay owner is no longer current".to_owned()));
        }
        Ok(ReplayCommitGuard { _live: live })
    }

    pub(crate) fn begin_close(&self) {
        self.shared.revoke();
    }

    pub(crate) async fn close(&self) -> CodexResult<()> {
        self.begin_close();
        wait_for(self.shared.completion.subscribe()).await
    }
}

async fn wait_for(mut status: watch::Receiver<Status>) -> CodexResult<()> {
    loop {
        if let Some(result) = *status.borrow_and_update() {
            return result.map_err(component_error);
        }
        status.changed().await.map_err(|_| {
            CodexErr::Fatal("context replay supervisor stopped".to_owned())
        })?;
    }
}

fn component_error(error: ReplayComponentError) -> CodexErr {
    CodexErr::Fatal(error.to_string())
}

impl From<codex_context_replay::RolloutReconstruction>
    for super::rollout_reconstruction::RolloutReconstruction
{
    fn from(value: codex_context_replay::RolloutReconstruction) -> Self {
        Self {
            history: value.history,
            retained_context: value.retained_context,
            guardian_history: value.guardian_history,
            last_started_turn_id: value.last_started_turn_id,
            previous_turn_settings: value.previous_turn_settings,
            reference_context_item: value.reference_context_item,
            world_state_baseline: value.world_state_baseline.map(|snapshot| {
                crate::context::world_state::WorldStateSnapshot::from(&snapshot.into_object())
            }),
            window_number: value.window_number,
            first_window_id: value.first_window_id,
            previous_window_id: value.previous_window_id,
            window_id: value.window_id,
        }
    }
}
