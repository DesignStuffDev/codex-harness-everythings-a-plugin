use crate::HttpErrorHeaders;
use crate::WireError;
use codex_protocol::protocol::MisalignmentErrorDetails;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Absolute same-machine retry advice. Capture once before waiting for process exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryDeadline {
    pub unix_seconds: u64,
    pub subsec_nanos: u32,
}

impl RetryDeadline {
    pub fn from_system_time(deadline: SystemTime) -> Result<Self, WireError> {
        let duration = deadline.duration_since(UNIX_EPOCH).map_err(|_| WireError::Deadline)?;
        Ok(Self { unix_seconds: duration.as_secs(), subsec_nanos: duration.subsec_nanos() })
    }

    pub fn to_system_time(self) -> Result<SystemTime, WireError> {
        if self.subsec_nanos >= 1_000_000_000 {
            return Err(WireError::Deadline);
        }
        UNIX_EPOCH.checked_add(Duration::new(self.unix_seconds, self.subsec_nanos))
            .ok_or(WireError::Deadline)
    }
}

/// All current backend API outcomes. Diagnostics are intentional public values.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackendError {
    Api { status: u16, message: String },
    Stream { message: String },
    ContentFilter,
    ContextWindowExceeded,
    QuotaExceeded,
    UsageNotIncluded,
    Retryable { message: String, retry_at: Option<RetryDeadline> },
    RateLimitExceeded { message: String, retry_at: Option<RetryDeadline> },
    RateLimit { message: String },
    InvalidRequest { message: String },
    InvalidPrompt { message: String },
    CyberPolicy { message: String },
    BioPolicy { message: String },
    MisalignmentPolicyViolation { message: String, misalignment: Option<MisalignmentErrorDetails> },
    FlexUnavailable,
    ServerOverloaded { retry_at: Option<RetryDeadline> },
    Transport { transport: TransportFailure },
}

/// Transport errors retain native policy classification rather than display text.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransportFailure {
    Policy { reason: NetworkDenial },
    Http {
        status: u16,
        url: Option<String>,
        headers: Option<HttpErrorHeaders>,
        body: Option<String>,
        retry_at: Option<RetryDeadline>,
    },
    RetryLimit,
    Timeout,
    Connection { connection: ConnectionFailure },
    Network { message: String },
    Build { message: String },
    ResponseTooLarge { max_bytes: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkDenial { Unavailable, Destination, Revoked, UnsupportedTransport }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteFailure {
    ProxyResolutionUnavailable,
    ConnectTimeout,
    ProxyAuthenticationRequired,
    TlsError,
    InvalidProxyConfig,
    UnsupportedProxyScheme,
    ResolverError,
}

/// Opaque connection sources are replaced by a canonical diagnostic and flags.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionFailure {
    pub message: String,
    pub status: Option<u16>,
    pub failure_class: Option<RouteFailure>,
    pub is_timeout: bool,
    pub is_connect: bool,
    pub is_builder: bool,
    pub is_body: bool,
    pub is_request: bool,
}

impl BackendError {
    pub fn from_value(value: serde_json::Value) -> Result<Self, WireError> {
        let error: Self = serde_json::from_value(value).map_err(|_| WireError::BackendError)?;
        error.validate()?;
        Ok(error)
    }

    pub fn validate(&self) -> Result<(), WireError> {
        match self {
            Self::Api { status, .. } => validate_status(*status),
            Self::Retryable { retry_at, .. }
            | Self::RateLimitExceeded { retry_at, .. }
            | Self::ServerOverloaded { retry_at } => validate_deadline(*retry_at),
            Self::Transport { transport } => transport.validate(),
            Self::Stream { .. }
            | Self::ContentFilter
            | Self::ContextWindowExceeded
            | Self::QuotaExceeded
            | Self::UsageNotIncluded
            | Self::RateLimit { .. }
            | Self::InvalidRequest { .. }
            | Self::InvalidPrompt { .. }
            | Self::CyberPolicy { .. }
            | Self::BioPolicy { .. }
            | Self::MisalignmentPolicyViolation { .. }
            | Self::FlexUnavailable => Ok(()),
        }
    }
}

impl TransportFailure {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            Self::Http { status, url, headers, retry_at, .. } => {
                validate_status(*status)?;
                validate_deadline(*retry_at)?;
                if let Some(url) = url { crate::http_metadata::validate_public_url(url)?; }
                if let Some(headers) = headers { headers.validate()?; }
                Ok(())
            }
            Self::Connection { connection } => {
                if let Some(status) = connection.status { validate_status(status)?; }
                Ok(())
            }
            Self::ResponseTooLarge { max_bytes } => {
                usize::try_from(*max_bytes).map(|_| ()).map_err(|_| WireError::IntegerRange)
            }
            Self::Policy { .. }
            | Self::RetryLimit
            | Self::Timeout
            | Self::Network { .. }
            | Self::Build { .. } => Ok(()),
        }
    }
}

fn validate_status(status: u16) -> Result<(), WireError> {
    http::StatusCode::from_u16(status).map(|_| ()).map_err(|_| WireError::Status)
}

fn validate_deadline(deadline: Option<RetryDeadline>) -> Result<(), WireError> {
    deadline.map(RetryDeadline::to_system_time).transpose().map(|_| ())
}

impl std::fmt::Debug for BackendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("BackendError").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for TransportFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("TransportFailure").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for ConnectionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ConnectionFailure").finish_non_exhaustive()
    }
}
