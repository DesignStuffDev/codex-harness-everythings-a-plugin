//! Awaited, stateless reconstruction through one explicitly selected component.
//!
//! Each engine session owns its connection. The immutable catalog/binding fixes
//! code and configuration for that connection; installation changes apply to new
//! sessions. No synchronous history API performs IPC and no failure falls back to
//! a different implementation. The host validates the session owner and epoch
//! before committing the returned owned reconstruction to live session state.

use codex_component_host::ComponentBinding;
use codex_component_host::ComponentCatalog;
use codex_component_host::ComponentSession;
use codex_context_replay::RECONSTRUCT_METHOD;
use codex_context_replay::REPLAY_COMPONENT_KIND;
use codex_context_replay::REPLAY_COMPONENT_NAME;
use codex_context_replay::REPLAY_CONTRACT_VERSION;
use codex_context_replay::ReplayInput;
use codex_context_replay::RolloutReconstruction;

/// An immutable startup selection and its supervised persistent connection.
///
/// Replay requests are pure. Dropping a request future abandons its waiter, but
/// the transport retains accepted work until response, drain, or process exit.
/// Close is explicit; neither cancellation nor a lost response causes a retry.
pub struct ProcessReplay {
    plugin_id: String,
    session: ComponentSession,
}

/// Bounded diagnostics deliberately omit rollout data and component messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayComponentError {
    InvalidBinding,
    Startup,
    EncodeInput,
    Call,
    DecodeOutput,
    Shutdown,
}

impl std::fmt::Display for ReplayComponentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidBinding => "invalid context replay component binding",
            Self::Startup => "context replay component startup failed",
            Self::EncodeInput => "context replay input encoding failed",
            Self::Call => "context replay component request failed",
            Self::DecodeOutput => "context replay component returned invalid state",
            Self::Shutdown => "context replay component shutdown failed",
        })
    }
}

impl std::error::Error for ReplayComponentError {}

impl ProcessReplay {
    /// Absence preserves the caller's native path; an invalid selected provider
    /// is an error. The caller must retain this object for its session lifetime.
    pub async fn from_catalog(
        catalog: &ComponentCatalog,
    ) -> Result<Option<Self>, ReplayComponentError> {
        match catalog.selected(REPLAY_COMPONENT_KIND, REPLAY_COMPONENT_NAME) {
            Some(binding) => Self::connect(binding).await.map(Some),
            None => Ok(None),
        }
    }

    pub async fn connect(binding: ComponentBinding) -> Result<Self, ReplayComponentError> {
        if binding.spec.kind != REPLAY_COMPONENT_KIND
            || binding.spec.name != REPLAY_COMPONENT_NAME
            || binding.spec.contract_version != REPLAY_CONTRACT_VERSION
        {
            return Err(ReplayComponentError::InvalidBinding);
        }
        let session = binding
            .connect()
            .await
            .map_err(|_| ReplayComponentError::Startup)?;
        Ok(Self {
            plugin_id: binding.plugin_id,
            session,
        })
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// The dedicated trusted-state codec preserves fields that ordinary model
    /// JSON intentionally strips. The selected implementation is trusted code;
    /// this method must never be used to deserialize provider/model responses.
    pub async fn reconstruct(
        &self,
        input: ReplayInput,
    ) -> Result<RolloutReconstruction, ReplayComponentError> {
        let params =
            serde_json::to_value(input).map_err(|_| ReplayComponentError::EncodeInput)?;
        let response = self
            .session
            .call(RECONSTRUCT_METHOD, params)
            .await
            .map_err(|_| ReplayComponentError::Call)?;
        serde_json::from_value(response).map_err(|_| ReplayComponentError::DecodeOutput)
    }

    /// Stop admission synchronously before awaiting surrounding host shutdown.
    pub fn begin_close(&self) {
        self.session.begin_close();
    }

    /// Drain accepted work and require the transport shutdown acknowledgement.
    /// The binding's timeout bounds shutdown and forced termination; a failed
    /// close is an error even though replay itself has no durable side effects.
    pub async fn close(&self) -> Result<(), ReplayComponentError> {
        self.session
            .close()
            .await
            .map_err(|_| ReplayComponentError::Shutdown)
    }
}
