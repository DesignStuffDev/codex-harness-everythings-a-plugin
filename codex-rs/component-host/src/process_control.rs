//! Invocation ownership before readiness and explicit force-and-reap control.

use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use super::ComponentStream;
use super::StreamFrame;
use crate::ComponentBinding;

pub(crate) const FORCE_REAP_GRACE: Duration = Duration::from_secs(2);

/// Control a component while initialization is pending. Dropping this owner or
/// its pending `ready` future still force-cancels through the fallback guard.
#[derive(Debug)]
pub struct ComponentStartup {
    receiver: mpsc::Receiver<StreamFrame>,
    started: oneshot::Receiver<()>,
    task: RunningTask,
    shutdown: ComponentShutdown,
}

impl ComponentStartup {
    pub fn shutdown_handle(&self) -> ComponentShutdown {
        self.shutdown.clone()
    }

    pub async fn ready(self) -> Result<ComponentStream> {
        if self.started.await.is_err() {
            self.task.join().await?;
            bail!("component exited before acknowledging initialization");
        }
        Ok(ComponentStream {
            receiver: self.receiver,
            task: Some(self.task),
            shutdown: self.shutdown,
        })
    }
}

/// Request cleanup while retaining the invocation and its current pending read.
#[derive(Clone, Debug)]
pub struct ComponentShutdown {
    graceful: watch::Sender<Option<Instant>>,
    force: watch::Sender<bool>,
    completed: watch::Receiver<Option<std::result::Result<(), String>>>,
}

impl ComponentShutdown {
    /// The first request fixes the cleanup budget; later requests do not extend it.
    pub fn request(&self, grace: Duration) {
        self.graceful.send_if_modified(|deadline| {
            if deadline.is_some() {
                return false;
            }
            *deadline = Some(Instant::now() + grace);
            true
        });
    }

    /// Force termination and await the owned child's actual `wait` completion.
    /// Keep the startup/read future alive until this returns. A two-second bound
    /// failing does not confirm reaping, and accepted-write durability is unknown.
    /// This acknowledges the direct child, not arbitrary orphaned descendants.
    pub async fn force_and_reap(&self) -> Result<()> {
        self.force.send_replace(true);
        let mut completed = self.completed.clone();
        let outcome = tokio::time::timeout(FORCE_REAP_GRACE, async {
            let outcome = completed
                .wait_for(Option::is_some)
                .await
                .context("component supervisor ended without a reap acknowledgement")?
                .clone()
                .context("component reap acknowledgement missing")?;
            outcome.map_err(anyhow::Error::msg)
        })
        .await
        .context("forced component termination did not confirm reaping within 2 seconds; durability is unknown")?;
        outcome.context(
            "forced component termination could not confirm reaping; durability is unknown",
        )
    }
}

pub(super) struct ControlReceivers {
    pub(super) graceful: watch::Receiver<Option<Instant>>,
    pub(super) force: watch::Receiver<bool>,
    pub(super) completed: watch::Sender<Option<std::result::Result<(), String>>>,
    pub(super) policy: InvocationPolicy,
}

#[derive(Clone, Copy)]
pub(super) enum InvocationPolicy {
    Bounded,
    Service { exit_grace: Duration },
}

impl ComponentBinding {
    /// Start supervision immediately and expose control before the handshake.
    /// Call `ready` to obtain the event stream. Requires an active Tokio runtime.
    /// On Linux the creating OS thread must outlive the child, as for `stream`.
    pub fn start_stream(&self, method: &str, params: Value) -> ComponentStartup {
        self.start_stream_with_policy(method, params, InvocationPolicy::Bounded)
    }

    /// Start one long-running invocation with a bounded startup and exit phase.
    /// `timeout_ms` bounds initialization and delivery of the initial request;
    /// after readiness there is no execution deadline. A result starts the
    /// `exit_grace` budget. Explicit graceful requests keep their own earlier
    /// deadline; a later request cannot extend an already-running exit budget.
    /// Dropping the startup/stream still cancels it, as for `start_stream`.
    pub fn start_service_stream(
        &self,
        method: &str,
        params: Value,
        exit_grace: Duration,
    ) -> ComponentStartup {
        self.start_stream_with_policy(method, params, InvocationPolicy::Service { exit_grace })
    }

    fn start_stream_with_policy(
        &self,
        method: &str,
        params: Value,
        policy: InvocationPolicy,
    ) -> ComponentStartup {
        let (sender, receiver) = mpsc::channel(1);
        let (started_sender, started) = oneshot::channel();
        let (graceful, graceful_receiver) = watch::channel(None);
        let (force, force_receiver) = watch::channel(false);
        let (completed_sender, completed) = watch::channel(None);
        let binding = self.clone();
        let method = method.to_owned();
        let task = RunningTask(Some(tokio::spawn(async move {
            super::supervise(
                binding,
                method,
                params,
                started_sender,
                sender,
                ControlReceivers {
                    graceful: graceful_receiver,
                    force: force_receiver,
                    completed: completed_sender,
                    policy,
                },
            )
            .await
        })));
        ComponentStartup {
            receiver,
            started,
            task,
            shutdown: ComponentShutdown {
                graceful,
                force,
                completed,
            },
        }
    }
}

#[derive(Debug)]
pub(super) struct RunningTask(Option<JoinHandle<Result<()>>>);

impl RunningTask {
    pub(super) async fn join(mut self) -> Result<()> {
        let result = self
            .0
            .as_mut()
            .context("component task already joined")?
            .await;
        self.0.take();
        result.context("component supervisor failed")?
    }
}

impl Drop for RunningTask {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}
