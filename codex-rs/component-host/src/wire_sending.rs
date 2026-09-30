//! Shared chunk sender with explicit flush ownership.
use super::*;

pub(crate) struct Sending {
    work: Option<crate::payload_work::JsonWorkClient>,
    outgoing: Outgoing,
    payload: Option<Payload>,
    reader: Option<tokio::fs::File>,
    started: bool,
    chunks: u64,
    transferred: u64,
}

impl Sending {
    fn validate(outgoing: &Outgoing) -> Result<()> {
        if let Header::Request {
            component, method, ..
        } = &outgoing.header
        {
            validate_component(component, method)?;
        }
        match &outgoing.header {
            Header::Request { id, .. } | Header::Result { id } | Header::Error { id, .. } => {
                ensure!(*id > 0, "session message ID must be positive")
            }
            Header::Shutdown | Header::ShutdownComplete => {}
        }
        if let Header::Error { message, .. } = &outgoing.header {
            ensure!(
                message.len() < MAX_FRAME_BYTES,
                "session error exceeds wire limit"
            );
        }
        Ok(())
    }
    pub(super) async fn new(mut outgoing: Outgoing, control: bool) -> Result<Self> {
        Self::validate(&outgoing)?;
        let payload = if matches!(
            outgoing.header,
            Header::Request { .. } | Header::Result { .. }
        ) {
            let payload = Payload::from_value(std::mem::take(&mut outgoing.value)).await?;
            Some(payload)
        } else {
            None
        };
        Self::from_payload(outgoing, payload, control)
    }
    pub(crate) fn from_payload(
        outgoing: Outgoing,
        payload: Option<Payload>,
        control: bool,
    ) -> Result<Self> {
        Self::from_parts(outgoing, payload, control, None)
    }
    pub(crate) fn with_work(
        outgoing: Outgoing,
        payload: Payload,
        work: crate::payload_work::JsonWorkClient,
    ) -> Result<Self> {
        Self::from_parts(outgoing, Some(payload), false, Some(work))
    }
    fn from_parts(
        outgoing: Outgoing,
        payload: Option<Payload>,
        control: bool,
        work: Option<crate::payload_work::JsonWorkClient>,
    ) -> Result<Self> {
        Self::validate(&outgoing)?;
        let control = control
            || matches!(
                outgoing.header,
                Header::Request {
                    is_control: true,
                    ..
                }
            );
        ensure!(
            !control
                || payload
                    .as_ref()
                    .is_none_or(|payload| payload.bytes <= MAX_CONTROL_BYTES),
            "session control payload exceeds limit"
        );
        let reader = payload
            .as_ref()
            .filter(|_| work.is_none())
            .map(|payload| payload.file.reopen().map(tokio::fs::File::from_std))
            .transpose()?;
        Ok(Self {
            work,
            outgoing,
            payload,
            reader,
            started: false,
            chunks: 0,
            transferred: 0,
        })
    }

    // One step is at most one physical frame, allowing urgent controls between chunks.
    pub(crate) async fn next_frame(&mut self) -> Result<(Frame, bool)> {
        let id = match &self.outgoing.header {
            Header::Request { id, .. } | Header::Result { id } | Header::Error { id, .. } => *id,
            Header::Shutdown | Header::ShutdownComplete => 0,
        };
        let complete;
        let frame;
        if !self.started {
            let bytes = self.payload.as_ref().map_or(0, |payload| payload.bytes);
            frame = match &self.outgoing.header {
                Header::Request {
                    id,
                    component,
                    method,
                    is_control,
                } => Frame::RequestStart {
                    id: *id,
                    component: component.clone(),
                    method: method.clone(),
                    is_control: *is_control,
                    bytes,
                },
                Header::Result { id } => Frame::ResultStart { id: *id, bytes },
                Header::Error { id, message } => Frame::Error {
                    id: *id,
                    message: message.clone(),
                },
                Header::Shutdown => Frame::Shutdown,
                Header::ShutdownComplete => Frame::ShutdownComplete,
            };
            self.started = true;
            complete = self.payload.is_none();
        } else if self.transferred
            < self
                .payload
                .as_ref()
                .context("missing outgoing payload")?
                .bytes
        {
            let chunk = if let Some(work) = &self.work {
                let payload = self.payload.as_ref().context("missing outgoing payload")?;
                work.read(payload, self.transferred, CHUNK_BYTES).await?
            } else {
                let mut chunk = vec![0; CHUNK_BYTES];
                let bytes = self
                    .reader
                    .as_mut()
                    .context("missing spool reader")?
                    .read(&mut chunk)
                    .await?;
                chunk.truncate(bytes);
                chunk
            };
            let bytes = chunk.len();
            ensure!(bytes > 0, "truncated outgoing payload spool");
            frame = Frame::Chunk {
                id,
                index: self.chunks,
                data: STANDARD.encode(chunk),
            };
            self.transferred += bytes as u64;
            self.chunks += 1;
            complete = false;
        } else {
            frame = Frame::End {
                id,
                chunks: self.chunks,
            };
            complete = true;
        }
        Ok((frame, complete))
    }
    pub(crate) fn mark_sent(&self) {
        if let Some(sent) = &self.outgoing.sent {
            sent.store(true, Ordering::Release);
        }
    }
    pub(super) async fn step<W: AsyncWrite + Unpin>(&mut self, writer: &mut W) -> Result<bool> {
        let (frame, complete) = self.next_frame().await?;
        write_frame(writer, frame).await?;
        if complete {
            writer.flush().await?;
            self.mark_sent();
        }
        Ok(complete)
    }
}
