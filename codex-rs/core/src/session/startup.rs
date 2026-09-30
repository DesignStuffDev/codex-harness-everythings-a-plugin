//! Owns persistence acquisition and constructed services until loop handoff.

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::Weak;

use codex_protocol::protocol::Op;
use codex_thread_store::LiveThreadInitGuard;
use tokio::runtime::Handle;
use tokio::sync::Mutex;

use super::SessionIo;
use super::runtime_lifecycle::SessionRuntimeOwner;
use super::runtime_lifecycle::ShutdownOutcome;
use super::runtime_lifecycle::ShutdownTask;
use super::runtime_lifecycle::ShutdownWait;
use super::session::Session;

#[derive(Default)]
pub(crate) struct SessionStartup {
    pub(crate) persistence: Mutex<LiveThreadInitGuard>,
    pub(crate) session: OnceLock<Weak<Session>>,
    pub(crate) io: OnceLock<SessionIo>,
    cleanup: ShutdownTask,
}

impl SessionStartup {
    pub(crate) async fn cleanup(self: &Arc<Self>) {
        let session = self.session.get().and_then(Weak::upgrade);
        let runtime = session.as_ref().map_or_else(Handle::current, |session| {
            session.services.runtime_handle.clone()
        });
        if let Err(error) = self.begin_cleanup(session, &runtime).wait().await {
            tracing::warn!(%error, "failed to clean up session startup");
        }
    }

    fn begin_cleanup(
        self: &Arc<Self>,
        session: Option<Arc<Session>>,
        runtime: &Handle,
    ) -> ShutdownWait {
        let startup = Arc::clone(self);
        self.cleanup.start(runtime, async move {
            if let Some(io) = startup.io.get() {
                // The loop already owns normal persistence and its terminal supervisor.
                let _ = io.submit(Op::Interrupt).await;
                let _ = io.shutdown_and_wait().await;
                Ok(())
            } else {
                let outcome = match session {
                    Some(session) => {
                        super::runtime_lifecycle::shutdown_runtime(&session)
                            .wait()
                            .await
                    }
                    None => Ok(()),
                };
                // Producers stop before the uncommitted writer is discarded. This
                // worker also retains any acquisition whose original waiter was lost.
                let mut persistence = std::mem::take(&mut *startup.persistence.lock().await);
                persistence.discard().await;
                outcome
            }
        })
    }
}

/// Constructed access and terminal ownership must travel together until handoff.
pub(crate) struct InitializedSession {
    pub(crate) session: Arc<Session>,
    pub(crate) launch: SessionLaunchGuard,
}

#[must_use = "session startup must transfer ownership or be cleaned up"]
pub(crate) struct SessionLaunchGuard {
    startup: Arc<SessionStartup>,
    session: Option<Arc<Session>>,
    runtime: Handle,
    armed: bool,
}

impl SessionLaunchGuard {
    pub(super) fn new(startup: Arc<SessionStartup>) -> Self {
        Self {
            startup,
            session: None,
            runtime: Handle::current(),
            armed: true,
        }
    }

    pub(super) fn constructed(&mut self, session: Arc<Session>) {
        let _ = self.startup.session.set(Arc::downgrade(&session));
        self.session = Some(session);
    }

    pub(super) async fn cleanup(mut self) -> ShutdownOutcome {
        let cleanup = self
            .startup
            .begin_cleanup(self.session.take(), &self.runtime);
        self.armed = false;
        cleanup.wait().await
    }

    pub(super) async fn into_runtime_owner(mut self) -> SessionRuntimeOwner {
        // Cancellation while taking this lock still leaves the launch guard armed.
        // After commit, transfer contains no further await or fallible work.
        self.startup.persistence.lock().await.commit();
        let owner =
            SessionRuntimeOwner::new(self.session.take().expect("constructed session is owned"));
        self.armed = false;
        owner
    }
}

impl Drop for SessionLaunchGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self
                .startup
                .begin_cleanup(self.session.take(), &self.runtime);
        }
    }
}
