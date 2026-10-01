//! Persistent component server plumbing; implementations own their handler tasks.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
use codex_component_api::COMPONENT_API_VERSION;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use tokio::io::AsyncBufRead;
use tokio::io::BufReader;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::process::read_frame;
use crate::process::write_frame;
use crate::session::CALL_SLOTS;
use crate::session_failure::SessionFailure;
use crate::session_limits::SessionPayloadLimits;
use crate::session_wire::Header;
use crate::session_wire::MAX_PENDING;
use crate::session_wire::MessageReader;
use crate::session_wire::Outgoing;
use crate::session_wire::SessionComponent;
use crate::session_wire::write_messages;

/// Explicit host configuration for one persistent process. Configuration may
/// contain credentials, so this type deliberately does not implement Debug.
#[derive(Clone, Deserialize)]
pub struct SessionInitialization {
    pub api_version: u32,
    pub plugin_id: String,
    pub config: Value,
    pub state_dir: PathBuf,
}

#[derive(Clone)]
enum End {
    Shutdown,
    Disconnected,
    Failure(SessionFailure),
}

struct ServerState {
    end: Mutex<Option<End>>,
    disconnected: AtomicBool,
    regular_slots: Arc<Semaphore>,
    control_slots: Arc<Semaphore>,
}

/// One fully received call. Responding transfers ownership to the writer;
/// dropping a request without responding returns an explicit transport error.
pub struct ComponentServerRequest {
    pub id: u64,
    pub component: SessionComponent,
    pub method: String,
    pub params: Value,
    pub is_control: bool,
    response: Option<mpsc::OwnedPermit<Outgoing>>,
    _slot: OwnedSemaphorePermit,
    limits: SessionPayloadLimits,
    response_is_control: bool,
}

impl std::fmt::Debug for ComponentServerRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComponentServerRequest")
            .field("id", &self.id)
            .field("component", &self.component)
            .field("method", &self.method)
            .field("is_control", &self.is_control)
            .finish_non_exhaustive()
    }
}

impl ComponentServerRequest {
    /// Check a successful logical JSON reply before consuming this request.
    /// A component can use this to substitute its own typed resource-exhaustion
    /// envelope. These local errors are never reconstructed from wire text.
    pub fn validate_response(&self, value: &Value) -> Result<()> {
        // The public request metadata is mutable; the reserved response lane is
        // fixed at admission and cannot be broadened by editing that metadata.
        self.limits.check_reply(value, self.response_is_control)
    }

    /// Complete the request. Application errors belong in the service's typed
    /// result envelope; Err is reserved for malformed requests/transport failures.
    pub async fn respond(mut self, result: std::result::Result<Value, String>) -> Result<()> {
        let overflow = result
            .as_ref()
            .ok()
            .and_then(|value| self.validate_response(value).err());
        let response = self
            .response
            .take()
            .context("component request already answered")?;
        let result = if overflow.is_some() {
            // Use the already reserved lane, leave the writer healthy, and do
            // not expose payload data or pretend a legacy Error is typed.
            Err("component response exceeds configured payload limit".to_owned())
        } else {
            result
        };
        let (header, value) = match result {
            Ok(value) => (Header::Result { id: self.id }, value),
            Err(message) => (
                Header::Error {
                    id: self.id,
                    message,
                },
                Value::Null,
            ),
        };
        response.send(Outgoing {
            header,
            value,
            after_sent: None,
            sent: None,
        });
        match overflow {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for ComponentServerRequest {
    fn drop(&mut self) {
        if let Some(response) = self.response.take() {
            response.send(Outgoing {
                header: Header::Error {
                    id: self.id,
                    message: "component handler abandoned request; outcome is unknown".to_owned(),
                },
                value: Value::Null,
                after_sent: None,
                sent: None,
            });
        }
    }
}

/// A server with an independently owned reader. `next` is cancellation-safe and
/// can be selected alongside completed handlers. Run handlers concurrently so
/// control requests can release leases held by blocked ordinary operations.
pub struct ComponentServer {
    initialization: SessionInitialization,
    incoming: mpsc::Receiver<ComponentServerRequest>,
    regular: Option<mpsc::Sender<Outgoing>>,
    control: Option<mpsc::Sender<Outgoing>>,
    reader: Option<JoinHandle<()>>,
    writer: Option<JoinHandle<Result<()>>>,
    state: Arc<ServerState>,
}

impl ComponentServer {
    pub async fn stdio() -> Result<Self> {
        Self::stdio_with_limits(SessionPayloadLimits::default()).await
    }

    /// Enforce local serialized-body limits without changing the handshake or
    /// wire schema. Invalid limits are rejected before consuming initialization.
    pub async fn stdio_with_limits(limits: SessionPayloadLimits) -> Result<Self> {
        limits.validate()?;
        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();
        let initialize = read_frame(&mut stdin).await?;
        ensure!(
            initialize["type"] == "initialize"
                && initialize["session"] == json!({"mode":"multiplexed","version":1}),
            "persistent component requires multiplexed session version 1"
        );
        let initialization: SessionInitialization = serde_json::from_value(initialize)?;
        ensure!(
            initialization.api_version == COMPONENT_API_VERSION,
            "incompatible component API version"
        );
        ensure!(
            initialization.config.is_object(),
            "component configuration must be an object"
        );
        write_frame(
            &mut stdout,
            json!({"type":"ready","api_version":COMPONENT_API_VERSION,
            "session":{"mode":"multiplexed","version":1}}),
        )
        .await?;
        let (regular, regular_rx) = mpsc::channel(CALL_SLOTS);
        let (control, control_rx) = mpsc::channel(CALL_SLOTS);
        let (incoming, receiver) = mpsc::channel(MAX_PENDING);
        let state = Arc::new(ServerState {
            end: Mutex::new(None),
            disconnected: AtomicBool::new(false),
            regular_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
            control_slots: Arc::new(Semaphore::new(CALL_SLOTS)),
        });
        let reader_state = Arc::clone(&state);
        let responses = (regular.clone(), control.clone());
        let reader = tokio::spawn(async move {
            let outcome = read_requests(
                stdin,
                incoming,
                responses,
                Arc::clone(&reader_state),
                limits,
            )
            .await;
            let end =
                outcome.unwrap_or_else(|error| End::Failure(SessionFailure::from_error(error)));
            reader_state
                .disconnected
                .store(!matches!(end, End::Shutdown), Ordering::Release);
            *reader_state
                .end
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(end);
        });
        let writer = tokio::spawn(write_messages(stdout, regular_rx, control_rx, limits));
        Ok(Self {
            initialization,
            incoming: receiver,
            regular: Some(regular),
            control: Some(control),
            reader: Some(reader),
            writer: Some(writer),
            state,
        })
    }

    pub fn initialization(&self) -> &SessionInitialization {
        &self.initialization
    }

    /// None marks explicit shutdown or EOF. Check `was_disconnected` before
    /// releasing all service leases, draining accepted handlers, and finishing.
    pub async fn next(&mut self) -> Result<Option<ComponentServerRequest>> {
        // Both waits are cancellation-safe: the tasks own all parsing/writing.
        // A failed writer must wake service shutdown even if stdin stays open.
        let request = if let Some(writer) = self.writer.as_mut() {
            tokio::select! {
                request = self.incoming.recv() => request,
                result = writer => {
                    self.writer.take();
                    if let Some(reader) = self.reader.take() { reader.abort(); }
                    self.state.disconnected.store(true, Ordering::Release);
                    let failure = match result {
                        Ok(Ok(())) => SessionFailure::message("component response writer stopped unexpectedly"),
                        Ok(Err(error)) => SessionFailure::from_error(error.context("component response writer failed")),
                        Err(error) => SessionFailure::message(format!("component response writer task failed: {error}")),
                    };
                    let mut end = self.state.end.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    let failure = match end.as_ref() {
                        Some(End::Failure(primary)) => SessionFailure::from_error(primary.clone().into_error()
                            .context(format!("additional writer failure: {}", failure.into_error()))),
                        Some(End::Shutdown | End::Disconnected) | None => failure,
                    };
                    *end = Some(End::Failure(failure.clone()));
                    return Err(failure.into_error());
                }
            }
        } else {
            self.incoming.recv().await
        };
        if let Some(request) = request {
            return Ok(Some(request));
        }
        if let Some(reader) = self.reader.as_mut() {
            let result = reader.await;
            self.reader.take();
            result.context("component server reader failed")?;
        }
        match self
            .state
            .end
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            Some(End::Shutdown | End::Disconnected) => Ok(None),
            Some(End::Failure(failure)) => Err(failure
                .clone()
                .into_error()
                .context("component server transport failed")),
            None => bail!("component server reader stopped unexpectedly"),
        }
    }

    pub fn was_disconnected(&self) -> bool {
        self.state.disconnected.load(Ordering::Acquire)
    }

    /// Call only after releasing service leases and draining owned handler tasks.
    /// This flushes queued responses before acknowledging graceful shutdown.
    pub async fn finish(mut self) -> Result<()> {
        ensure!(
            self.state.regular_slots.available_permits() == CALL_SLOTS
                && self.state.control_slots.available_permits() == CALL_SLOTS,
            "component handlers must finish before the server"
        );
        let end = self
            .state
            .end
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .context("component server still accepting requests")?;
        if matches!(end, End::Shutdown) {
            self.regular
                .as_ref()
                .context("component writer missing")?
                .send(Outgoing {
                    header: Header::ShutdownComplete,
                    value: Value::Null,
                    after_sent: None,
                    sent: None,
                })
                .await
                .context("component response writer closed")?;
        }
        self.regular.take();
        self.control.take();
        if let Some(reader) = self.reader.as_mut() {
            let result = reader.await;
            self.reader.take();
            result.context("component server reader failed")?;
        }
        if let Some(writer) = self.writer.as_mut() {
            let result = writer.await;
            self.writer.take();
            let result = result
                .context("component server writer failed")
                .and_then(std::convert::identity);
            if let Err(error) = result {
                return Err(match end {
                    End::Failure(primary) => primary.into_error().context(format!(
                        "additional writer failure during finish: {error:#}"
                    )),
                    End::Shutdown | End::Disconnected => error,
                });
            }
        }
        match end {
            End::Shutdown => Ok(()),
            End::Disconnected => bail!("component host disconnected"),
            End::Failure(failure) => Err(failure
                .into_error()
                .context("component host protocol failed")),
        }
    }
}

impl Drop for ComponentServer {
    fn drop(&mut self) {
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
        if let Some(writer) = self.writer.take() {
            writer.abort();
        }
    }
}

async fn read_requests<R: AsyncBufRead + Unpin>(
    stdin: R,
    incoming: mpsc::Sender<ComponentServerRequest>,
    responses: (mpsc::Sender<Outgoing>, mpsc::Sender<Outgoing>),
    state: Arc<ServerState>,
    limits: SessionPayloadLimits,
) -> Result<End> {
    let mut reader = MessageReader::new(stdin).with_limits(limits);
    while let Some(message) = reader.next().await? {
        match message.header {
            Header::Request {
                id,
                component,
                method,
                is_control,
            } => {
                let (slots, responses) = if is_control {
                    (&state.control_slots, &responses.1)
                } else {
                    (&state.regular_slots, &responses.0)
                };
                let slot = Arc::clone(slots)
                    .try_acquire_owned()
                    .context("too many active component requests")?;
                let response = responses
                    .clone()
                    .try_reserve_owned()
                    .context("component peer exceeded bounded response capacity")?;
                let params = message
                    .payload
                    .context("component request payload missing")?
                    .into_value()
                    .await?;
                incoming
                    .try_send(ComponentServerRequest {
                        id,
                        component,
                        method,
                        params,
                        is_control,
                        response: Some(response),
                        _slot: slot,
                        limits,
                        response_is_control: is_control,
                    })
                    .map_err(|_| anyhow::anyhow!("component request consumer closed or full"))?;
            }
            Header::Shutdown => return Ok(End::Shutdown),
            Header::Result { .. } | Header::Error { .. } | Header::ShutdownComplete => {
                bail!("unexpected frame from component host");
            }
        }
    }
    Ok(End::Disconnected)
}

#[cfg(test)]
#[path = "session_server_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "session_server_limit_tests.rs"]
mod limit_tests;
