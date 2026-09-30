//! Shared physical writer for ordinary calls and negotiated reverse dependencies.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use serde_json::Value;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::broker_api::DependencyContext;
use crate::broker_api::DependencyError;
use crate::broker_api::DependencyScope;
pub(super) use crate::broker_spool::BrokerWriterControl;
use crate::broker_spool::WriterCancelled;
use crate::broker_spool::spool;
use crate::broker_wire::BrokerRequestHeader;
use crate::broker_wire::MAX_BROKER_PENDING;
use crate::broker_wire::broker_frame_value;
use crate::broker_wire::cancel_frame_value;
use crate::broker_wire::scoped_frame_value;
use crate::process::write_frame;
use crate::session_wire::Header;
use crate::session_wire::MAX_PENDING;
use crate::session_wire::Outgoing;
use crate::session_wire::Sending;
use crate::session_wire::SessionComponent;

pub(super) struct ScopedOutgoing {
    pub message: Outgoing,
    pub scope: Option<DependencyScope>,
}

pub(super) enum BrokerOutgoing {
    Request {
        header: BrokerRequestHeader,
        params: Value,
        sent: Arc<AtomicBool>,
    },
    Result {
        id: u64,
        result: Value,
        authority: Option<DependencyContext>,
    },
    Error {
        id: u64,
        error: DependencyError,
    },
}

/// Completion acknowledges a terminal physical frame flush, not just queue admission.
pub(super) struct BrokerResponse {
    pub message: BrokerOutgoing,
    pub completed: oneshot::Sender<std::result::Result<(), String>>,
}

struct ResponseCompletion(Option<oneshot::Sender<std::result::Result<(), String>>>);

impl Drop for ResponseCompletion {
    fn drop(&mut self) {
        if let Some(completed) = self.0.take() {
            let _ = completed.send(Err("dependency response flush was not confirmed".to_owned()));
        }
    }
}

struct ResponseSending {
    sending: BrokerSending,
    completion: ResponseCompletion,
}

impl ResponseSending {
    async fn new(response: BrokerResponse, control: &BrokerWriterControl) -> Result<Self> {
        let completion = ResponseCompletion(Some(response.completed));
        let sending = BrokerSending::new(response.message, control).await?;
        Ok(Self {
            sending,
            completion,
        })
    }

    async fn step<W: AsyncWrite + Unpin>(
        &mut self,
        writer: &mut W,
        control: &BrokerWriterControl,
    ) -> Result<bool> {
        let complete = self.sending.step(writer, control).await?;
        if complete && let Some(completed) = self.completion.0.take() {
            let _ = completed.send(Ok(()));
        }
        Ok(complete)
    }
}

async fn prepare(
    mut outgoing: Outgoing,
    control: bool,
    writer_control: &BrokerWriterControl,
) -> Result<Sending> {
    let payload = if matches!(
        outgoing.header,
        Header::Request { .. } | Header::Result { .. }
    ) {
        Some(spool(std::mem::take(&mut outgoing.value), writer_control).await?)
    } else {
        None
    };
    Sending::from_payload(outgoing, payload, control)
}

async fn send_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    value: Value,
    control: &BrokerWriterControl,
) -> Result<()> {
    tokio::select! {
        biased;
        _ = control.cancelled() => Err(WriterCancelled.into()),
        result = write_frame(writer, value) => result,
    }
}

/// One reserved cancellation slot per accepted broker request, never retried.
pub(super) struct BrokerCancellation {
    pub id: u64,
    pub after_sent: Arc<AtomicBool>,
}

struct BrokerSending {
    sending: Sending,
    id: u64,
    request: Option<BrokerRequestHeader>,
    failure: Option<DependencyError>,
    authority: Option<DependencyContext>,
}

impl BrokerSending {
    async fn new(outgoing: BrokerOutgoing, control: &BrokerWriterControl) -> Result<Self> {
        let (id, header, value, request, failure, authority, sent) = match outgoing {
            BrokerOutgoing::Request {
                header,
                params,
                sent,
            } => {
                let ordinary = Header::Request {
                    id: header.id,
                    component: SessionComponent {
                        kind: "dependency".to_owned(),
                        name: header.service.clone(),
                    },
                    method: header.method.clone(),
                    is_control: false,
                };
                (
                    header.id,
                    ordinary,
                    params,
                    Some(header),
                    None,
                    None,
                    Some(sent),
                )
            }
            BrokerOutgoing::Result {
                id,
                result,
                authority,
            } => (
                id,
                Header::Result { id },
                result,
                None,
                None,
                authority,
                None,
            ),
            BrokerOutgoing::Error { id, error } => (
                id,
                Header::Error {
                    id,
                    message: String::new(),
                },
                Value::Null,
                None,
                Some(error),
                None,
                None,
            ),
        };
        let sending = prepare(
            Outgoing {
                header,
                value,
                after_sent: None,
                sent,
            },
            /*control*/ false,
            control,
        )
        .await?;
        Ok(Self {
            sending,
            id,
            request,
            failure,
            authority,
        })
    }

    async fn step<W: AsyncWrite + Unpin>(
        &mut self,
        writer: &mut W,
        control: &BrokerWriterControl,
    ) -> Result<bool> {
        if control.is_cancelled() {
            return Err(WriterCancelled.into());
        }
        let authority_error = self
            .authority
            .as_ref()
            .and_then(|authority| authority.check_authority().err());
        let (mut frame, mut complete) = if authority_error.is_none() {
            self.sending.next_frame().await?
        } else {
            (
                crate::session_wire::Frame::Error {
                    id: self.id,
                    message: String::new(),
                },
                true,
            )
        };
        // Authority can change while a large spool is read. Check again immediately
        // before writing; final host catalog publication still has its own owner fence.
        if let Some(error) = authority_error.or_else(|| {
            self.authority
                .as_ref()
                .and_then(|authority| authority.check_authority().err())
        }) {
            self.failure = Some(error);
            frame = crate::session_wire::Frame::Error {
                id: self.id,
                message: String::new(),
            };
            complete = true;
        }
        let value = broker_frame_value(frame, self.request.as_ref(), self.failure.as_ref())?;
        // This shared bounded writer flushes the physical frame before returning.
        // The bounded spool read above is never cancelled mid-read. Control may
        // interrupt pipe IO only afterwards, failing the delivery acknowledgement.
        send_frame(writer, value, control).await?;
        if complete {
            self.sending.mark_sent();
        }
        Ok(complete)
    }
}

struct OrdinarySending {
    sending: Sending,
    scope: Option<DependencyScope>,
    terminal: bool,
}

impl OrdinarySending {
    async fn new(
        outgoing: ScopedOutgoing,
        control: bool,
        writer_control: &BrokerWriterControl,
    ) -> Result<Self> {
        let terminal = matches!(
            outgoing.message.header,
            Header::Shutdown | Header::ShutdownComplete
        );
        Ok(Self {
            sending: prepare(outgoing.message, control, writer_control).await?,
            scope: outgoing.scope,
            terminal,
        })
    }

    async fn step<W: AsyncWrite + Unpin>(
        &mut self,
        writer: &mut W,
        control: &BrokerWriterControl,
    ) -> Result<bool> {
        if control.is_cancelled() {
            return Err(WriterCancelled.into());
        }
        let (frame, complete) = self.sending.next_frame().await?;
        send_frame(
            writer,
            scoped_frame_value(frame, self.scope.as_ref())?,
            control,
        )
        .await?;
        if complete {
            self.sending.mark_sent();
        }
        Ok(complete)
    }
}

/// Queue capacities are reserved by the caller: 32 ordinary/control and 16 per
/// broker lane. This task is the sole writer and never waits for handler work.
#[allow(clippy::too_many_arguments)]
pub(super) async fn write_broker_messages<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut regular: mpsc::Receiver<ScopedOutgoing>,
    mut control: mpsc::Receiver<Outgoing>,
    mut requests: mpsc::Receiver<BrokerOutgoing>,
    mut responses: mpsc::Receiver<BrokerResponse>,
    mut cancellations: mpsc::Receiver<BrokerCancellation>,
    writer_control: BrokerWriterControl,
) -> Result<()> {
    let result: Result<()> = async {
        let mut regular_open = true;
        let mut control_open = true;
        let mut requests_open = true;
        let mut responses_open = true;
        let mut cancellations_open = true;
        let mut controls = VecDeque::<Outgoing>::new();
        let mut cancels = VecDeque::<BrokerCancellation>::new();
        let mut ordinary: Option<OrdinarySending> = None;
        let mut request: Option<BrokerSending> = None;
        let mut response: Option<ResponseSending> = None;
        let mut broker_turn = true;
        let mut response_turn = true;
        loop {
            if writer_control.is_cancelled() { return Err(WriterCancelled.into()); }
            while cancellations_open && cancels.len() < MAX_BROKER_PENDING {
                match cancellations.try_recv() {
                    Ok(cancel) => cancels.push_back(cancel),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => cancellations_open = false,
                }
            }
            // A full deferred queue disables recv polling. Still observe EOF
            // once its physical channel is empty, or impossible fences can hang.
            if cancellations.is_closed() && cancellations.is_empty() {
                cancellations_open = false;
            }
            if let Some(position) = cancels.iter().position(|cancel| cancel.after_sent.load(Ordering::Acquire)) {
                let cancel = cancels.remove(position).context("missing queued dependency cancellation")?;
                send_frame(&mut writer, cancel_frame_value(cancel.id)?, &writer_control).await?;
                continue;
            }
            if response.is_none() && responses_open {
                match responses.try_recv() {
                    Ok(message) => {
                        ensure!(!matches!(message.message, BrokerOutgoing::Request { .. }), "dependency response lane received request");
                        response = Some(ResponseSending::new(message, &writer_control).await?);
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {},
                    Err(mpsc::error::TryRecvError::Disconnected) => responses_open = false,
                }
            }
            while control_open && controls.len() < MAX_PENDING {
                match control.try_recv() {
                    Ok(message) => controls.push_back(message),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => control_open = false,
                }
            }
            if control.is_closed() && control.is_empty() {
                control_open = false;
            }
            // A response gets the first frame, then yields an opportunity to another
            // lane. Its logical body is unbounded, so draining it would starve cleanup.
            if response_turn && let Some(sending) = &mut response {
                if sending.step(&mut writer, &writer_control).await? {
                    response = None;
                }
                response_turn = false;
                continue;
            }
            if let Some(position) = controls.iter().position(|message| message.after_sent.as_ref()
                .is_none_or(|sent| sent.load(Ordering::Acquire))
                && ((ordinary.is_none() && request.is_none() && response.is_none())
                    || !matches!(message.header, Header::Shutdown | Header::ShutdownComplete)))
            {
                let message = controls.remove(position).context("missing queued component control")?;
                let mut sending = OrdinarySending::new(ScopedOutgoing { message, scope: None }, /*control*/ true, &writer_control).await?;
                while !sending.step(&mut writer, &writer_control).await? {}
                response_turn = true;
                continue;
            }
            if ordinary.is_none() && regular_open {
                match regular.try_recv() {
                    Ok(message) => ordinary = Some(OrdinarySending::new(message, /*control*/ false, &writer_control).await?),
                    Err(mpsc::error::TryRecvError::Empty) => {},
                    Err(mpsc::error::TryRecvError::Disconnected) => regular_open = false,
                }
            }
            if request.is_none() && requests_open {
                match requests.try_recv() {
                    Ok(message) => {
                        ensure!(matches!(message, BrokerOutgoing::Request { .. }), "dependency request lane received response");
                        request = Some(BrokerSending::new(message, &writer_control).await?);
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {},
                    Err(mpsc::error::TryRecvError::Disconnected) => requests_open = false,
                }
            }
            if ordinary.as_ref().is_some_and(|message| message.terminal) && response.is_some() {
                response_turn = true;
                continue;
            }
            if request.is_some() && (broker_turn || ordinary.is_none()
                || ordinary.as_ref().is_some_and(|message| message.terminal))
            {
                if request.as_mut().context("missing active dependency request")?.step(&mut writer, &writer_control).await? {
                    request = None;
                }
                broker_turn = false;
                response_turn = true;
                continue;
            }
            if let Some(sending) = &mut ordinary {
                if sending.step(&mut writer, &writer_control).await? {
                    ordinary = None;
                }
                broker_turn = true;
                response_turn = true;
                continue;
            }
            if let Some(sending) = &mut response {
                if sending.step(&mut writer, &writer_control).await? {
                    response = None;
                }
                continue;
            }
            if !regular_open && !control_open && !requests_open && !responses_open && !cancellations_open {
                ensure!(controls.is_empty() && cancels.is_empty(), "cleanup or cancellation depends on an unsent request");
                tokio::select! {
                    biased;
                    _ = writer_control.cancelled() => return Err(WriterCancelled.into()),
                    result = writer.shutdown() => result?,
                }
                return Ok(());
            }
            tokio::select! {
                biased;
                _ = writer_control.cancelled() => return Err(WriterCancelled.into()),
                cancel = cancellations.recv(), if cancellations_open && cancels.len() < MAX_BROKER_PENDING => match cancel {
                    Some(cancel) => cancels.push_back(cancel),
                    None => cancellations_open = false,
                },
                message = responses.recv(), if responses_open => match message {
                    Some(message) => {
                        ensure!(!matches!(message.message, BrokerOutgoing::Request { .. }), "dependency response lane received request");
                        response = Some(ResponseSending::new(message, &writer_control).await?);
                    }
                    None => responses_open = false,
                },
                message = control.recv(), if control_open && controls.len() < MAX_PENDING => match message {
                    Some(message) => controls.push_back(message),
                    None => control_open = false,
                },
                message = requests.recv(), if requests_open => match message {
                    Some(message) => {
                        ensure!(matches!(message, BrokerOutgoing::Request { .. }), "dependency request lane received response");
                        request = Some(BrokerSending::new(message, &writer_control).await?);
                    }
                    None => requests_open = false,
                },
                message = regular.recv(), if regular_open => match message {
                    Some(message) => ordinary = Some(OrdinarySending::new(message, /*control*/ false, &writer_control).await?),
                    None => regular_open = false,
                },
            }
        }
    }.await;
    match result {
        Err(error) if error.is::<WriterCancelled>() => Ok(()),
        result => result,
    }
}

#[cfg(test)]
#[path = "broker_writer_tests.rs"]
mod tests;
