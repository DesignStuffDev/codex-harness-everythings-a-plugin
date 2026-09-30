//! Keep presentation output and its cleanup supervisor alive across the first interrupt.

use std::future::Future;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use codex_component_host::ComponentBinding;
use codex_component_host::StreamFrame;
use serde_json::Value;
use tokio::time::Instant;

pub(crate) const SHUTDOWN_GRACE: Duration = Duration::from_secs(210);
const UNKNOWN_DURABILITY: &str = "accepted writes may not have completed and durability is unknown";

pub(crate) async fn run<F, I>(
    binding: &ComponentBinding,
    params: Value,
    grace: Duration,
    mut next_interrupt: F,
) -> Result<()>
where
    F: FnMut() -> I,
    I: Future<Output = std::io::Result<()>>,
{
    let outcome: Result<()> = async {
        let mut interrupt = Box::pin(next_interrupt());
        tokio::select! {
            // Register the signal listener before spawning the component, including
            // plugins that never acknowledge initialization.
            biased;
            result = &mut interrupt => {
                result.with_context(|| format!("interrupt listener failed during presentation startup; {UNKNOWN_DURABILITY}"))?;
                bail!("forced termination during presentation startup before readiness; {UNKNOWN_DURABILITY}");
            }
            _ = std::future::ready(()) => {}
        }
        let startup = binding.start_service_stream("launch", params, grace);
        let shutdown = startup.shutdown_handle();
        let mut stream = {
            let ready = startup.ready();
            tokio::pin!(ready);
            tokio::select! {
                result = &mut interrupt => {
                    shutdown.force_and_reap().await.with_context(|| format!("forced termination during presentation startup before readiness; {UNKNOWN_DURABILITY}"))?;
                    result.with_context(|| format!("interrupt listener failed during presentation startup; {UNKNOWN_DURABILITY}"))?;
                    bail!("forced termination during presentation startup before readiness; {UNKNOWN_DURABILITY}");
                }
                result = &mut ready => result?,
            }
        };
        let mut shutdown_deadline = None;
        loop {
            let frame = {
                // Cancelling `next` force-kills its component. Retain this exact
                // future while requesting graceful shutdown, even mid-frame.
                let next = stream.next();
                tokio::pin!(next);
                loop {
                    tokio::select! {
                        result = &mut next => break result,
                        result = &mut interrupt => {
                            if let Err(error) = result {
                                shutdown.force_and_reap().await?;
                                return Err(error).with_context(|| format!("interrupt listener failed; {UNKNOWN_DURABILITY}"));
                            }
                            if shutdown_deadline.is_some() {
                                shutdown.force_and_reap().await.with_context(|| format!("forced termination after a second interrupt; {UNKNOWN_DURABILITY}"))?;
                                bail!("forced termination after a second interrupt; {UNKNOWN_DURABILITY}");
                            }
                            shutdown_deadline = Some(Instant::now() + grace);
                            shutdown.request(grace);
                            eprintln!("Stopping presentation; allowing up to {grace:?} for accepted writes and storage cleanup. Press Ctrl+C again to force termination.");
                            interrupt = Box::pin(next_interrupt());
                        }
                        _ = async {
                            if let Some(deadline) = shutdown_deadline {
                                tokio::time::sleep_until(deadline).await;
                            } else {
                                std::future::pending::<()>().await;
                            }
                        } => {
                            shutdown.force_and_reap().await.with_context(|| format!("forced termination after the graceful shutdown deadline; {UNKNOWN_DURABILITY}"))?;
                            bail!("forced termination after the graceful shutdown deadline; {UNKNOWN_DURABILITY}");
                        }
                    }
                }
            };
            let frame = if shutdown_deadline.is_some() {
                frame.with_context(|| {
                    format!("presentation cleanup was not confirmed; {UNKNOWN_DURABILITY}")
                })?
            } else {
                frame?
            };
            match frame {
                StreamFrame::Event(event) => println!("{event}"),
                StreamFrame::Done(result) => {
                    println!("{result}");
                    return Ok(());
                }
            }
        }
    }.await;
    outcome.with_context(|| format!("presentation cleanup was not confirmed; {UNKNOWN_DURABILITY}"))
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
