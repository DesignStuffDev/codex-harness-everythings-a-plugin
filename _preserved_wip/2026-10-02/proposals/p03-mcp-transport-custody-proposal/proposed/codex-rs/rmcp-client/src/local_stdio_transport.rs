//! Protocol framing and child lifetime for locally spawned MCP servers.
//!
//! Process creation is platform-specific; protocol selection and shutdown are shared.

use std::future::Future;
use std::io;
use std::sync::Arc;

use codex_utils_pty::Command;
use futures::FutureExt;
use rmcp::service::RoleClient;
use rmcp::service::RxJsonRpcMessage;
use rmcp::service::TxJsonRpcMessage;
use rmcp::transport::Transport;
use rmcp::transport::async_rw::AsyncRwTransport;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;

use crate::bounded_stdio_transport::BoundedStdioTransport;
use crate::local_process_owner::LocalProcessOwner;
use crate::protocol_mode::McpProtocolMode;

pub(super) struct LocalStdioTransport {
    process: LocalProcessOwner,
    transport: StdioTransport,
}

enum StdioTransport {
    /// Preserve rmcp's existing framing for servers using the initialize handshake.
    Legacy(AsyncRwTransport<RoleClient, ChildStdout, ChildStdin>),
    /// Bound frames and skip messages unknown to the client during 2026-07-28 discovery.
    V20260728(BoundedStdioTransport),
}

impl LocalStdioTransport {
    pub(super) fn spawn(
        command: Command,
        program_name: String,
        protocol_mode: McpProtocolMode,
        local_process_observer: Option<Arc<dyn Fn(LocalProcessOwner) + Send + Sync>>,
    ) -> io::Result<Self> {
        let child = command.spawn()?;
        let (process, stdin, stdout) = LocalProcessOwner::new(
            child, program_name.clone(), local_process_observer,
        )?;
        let transport = match protocol_mode {
            McpProtocolMode::Legacy => StdioTransport::Legacy(AsyncRwTransport::new(stdout, stdin)),
            McpProtocolMode::V20260728 => {
                StdioTransport::V20260728(BoundedStdioTransport::new(stdin, stdout, program_name))
            }
        };
        Ok(Self { process, transport })
    }

    pub(super) fn id(&self) -> Option<u32> {
        self.process.id()
    }

    pub(super) fn process_owner(&self) -> LocalProcessOwner {
        self.process.clone()
    }
}

impl Transport<RoleClient> for LocalStdioTransport {
    type Error = io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        match &mut self.transport {
            StdioTransport::Legacy(transport) => transport.send(item).boxed(),
            StdioTransport::V20260728(transport) => transport.send(item).boxed(),
        }
    }

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<RoleClient>>> + Send {
        match &mut self.transport {
            StdioTransport::Legacy(transport) => transport.receive().boxed(),
            StdioTransport::V20260728(transport) => transport.receive().boxed(),
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        self.process.begin_shutdown();
        let framing = match &mut self.transport {
            StdioTransport::Legacy(transport) => transport.close().await,
            StdioTransport::V20260728(transport) => transport.close().await,
        };
        // Preserve framing errors without skipping process reaping or stderr.
        self.process.record_framing(framing);
        self.process.wait_closed().await
    }
}

impl Drop for LocalStdioTransport {
    fn drop(&mut self) {
        self.process.transport_dropped();
    }
}
