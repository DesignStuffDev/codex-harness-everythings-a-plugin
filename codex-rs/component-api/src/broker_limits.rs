//! Checked broker-only limits. These do not narrow ordinary history transport.
//! Construction validates configuration; it does not reserve resources or workers.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::MAX_FRAME_BYTES;

/// Structural ceiling, not an implicitly selected runtime configuration.
pub const MAX_BROKER_CONNECTIONS: u32 = 16;
pub const MAX_BROKER_CALLS_PER_CONNECTION: u32 = 16;
pub const MAX_BROKER_BLOCKING_JOBS: u32 = 8;

/// Unchecked input for a connection's reverse-work budget; validate before use.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerConnectionLimitsSpec {
    pub max_in_flight: u32,
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub max_spool_bytes: u64,
    pub max_decoded_nodes: u64,
    pub max_decoded_string_bytes: u64,
    pub max_decode_jobs: u32,
    pub max_encode_jobs: u32,
    pub max_ordinary_codec_jobs: u32,
}

/// Immutable validated connection limits; deserialize through the same checks.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    try_from = "BrokerConnectionLimitsSpec",
    into = "BrokerConnectionLimitsSpec"
)]
pub struct BrokerConnectionLimits {
    spec: BrokerConnectionLimitsSpec,
}

impl BrokerConnectionLimits {
    pub fn new(spec: BrokerConnectionLimitsSpec) -> Result<Self, BrokerLimitsError> {
        if spec.max_in_flight == 0 || spec.max_in_flight > MAX_BROKER_CALLS_PER_CONNECTION {
            return Err(BrokerLimitsError::InvalidCallCount);
        }
        if [
            spec.max_request_bytes,
            spec.max_response_bytes,
            spec.max_spool_bytes,
            spec.max_decoded_nodes,
            spec.max_decoded_string_bytes,
        ]
        .contains(&0)
        {
            return Err(BrokerLimitsError::ZeroBudget);
        }
        if spec.max_decode_jobs == 0
            || spec.max_encode_jobs == 0
            || spec.max_decode_jobs > spec.max_in_flight
            || spec.max_encode_jobs > spec.max_in_flight
            || spec.max_ordinary_codec_jobs == 0
            || spec.max_ordinary_codec_jobs > 32
        {
            return Err(BrokerLimitsError::InvalidCodecConcurrency);
        }
        let bytes = spec
            .max_request_bytes
            .checked_add(spec.max_response_bytes)
            .ok_or(BrokerLimitsError::SizeOverflow)?;
        if bytes > spec.max_spool_bytes {
            return Err(BrokerLimitsError::SpoolCannotFitCall);
        }
        Ok(Self { spec })
    }

    pub fn as_spec(&self) -> &BrokerConnectionLimitsSpec {
        &self.spec
    }

    /// Check that the process spool budget can fit one maximum call.
    /// The future runtime must enforce concurrent capacity in both live ledgers;
    /// this validation does not reserve a call or allocate either payload.
    pub fn validate_for(&self, process: &BrokerProcessLimits) -> Result<(), BrokerLimitsError> {
        let bytes = self
            .spec
            .max_request_bytes
            .checked_add(self.spec.max_response_bytes)
            .ok_or(BrokerLimitsError::SizeOverflow)?;
        if bytes > process.spec.max_spool_bytes {
            return Err(BrokerLimitsError::ProcessCannotFitCall);
        }
        Ok(())
    }
}

impl TryFrom<BrokerConnectionLimitsSpec> for BrokerConnectionLimits {
    type Error = BrokerLimitsError;
    fn try_from(spec: BrokerConnectionLimitsSpec) -> Result<Self, Self::Error> {
        Self::new(spec)
    }
}

impl From<BrokerConnectionLimits> for BrokerConnectionLimitsSpec {
    fn from(limits: BrokerConnectionLimits) -> Self {
        limits.spec
    }
}

/// Unchecked shared process budget. Do not construct a fresh ledger per connection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerProcessLimitsSpec {
    pub max_broker_connections: u32,
    pub max_in_flight: u32,
    pub max_spool_bytes: u64,
    pub max_decoded_nodes: u64,
    pub max_decoded_string_bytes: u64,
    pub max_blocking_jobs: u32,
}

/// Immutable validated process limits for a future shared runtime ledger.
/// That runtime must reserve connection permits before spawn/handshake and retain
/// them through idle readiness, draining and uncertain quarantined ownership.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "BrokerProcessLimitsSpec", into = "BrokerProcessLimitsSpec")]
pub struct BrokerProcessLimits {
    spec: BrokerProcessLimitsSpec,
}

impl BrokerProcessLimits {
    pub fn new(spec: BrokerProcessLimitsSpec) -> Result<Self, BrokerLimitsError> {
        if spec.max_broker_connections == 0 || spec.max_broker_connections > MAX_BROKER_CONNECTIONS
        {
            return Err(BrokerLimitsError::InvalidConnectionCount);
        }
        if spec.max_in_flight == 0
            || spec.max_in_flight > spec.max_broker_connections * MAX_BROKER_CALLS_PER_CONNECTION
        {
            return Err(BrokerLimitsError::InvalidCallCount);
        }
        if [
            spec.max_spool_bytes,
            spec.max_decoded_nodes,
            spec.max_decoded_string_bytes,
        ]
        .contains(&0)
        {
            return Err(BrokerLimitsError::ZeroBudget);
        }
        // Require capacity for ordinary codec, reverse decode and reverse encode.
        // A future runtime must separately reserve each progress class; this
        // configuration check does not schedule or reserve workers.
        if spec.max_blocking_jobs < 3 {
            return Err(BrokerLimitsError::InsufficientProgressWorkers);
        }
        if spec.max_blocking_jobs > MAX_BROKER_BLOCKING_JOBS {
            return Err(BrokerLimitsError::InvalidCodecConcurrency);
        }
        Ok(Self { spec })
    }

    pub fn as_spec(&self) -> &BrokerProcessLimitsSpec {
        &self.spec
    }

    /// Budget for one maximum physical frame per direction per connection.
    /// This is separate from logical reverse spools and is not an RSS claim.
    pub fn physical_frame_staging_bytes(&self) -> u64 {
        u64::from(self.spec.max_broker_connections) * 2 * MAX_FRAME_BYTES as u64
    }
}

impl TryFrom<BrokerProcessLimitsSpec> for BrokerProcessLimits {
    type Error = BrokerLimitsError;
    fn try_from(spec: BrokerProcessLimitsSpec) -> Result<Self, Self::Error> {
        Self::new(spec)
    }
}

impl From<BrokerProcessLimits> for BrokerProcessLimitsSpec {
    fn from(limits: BrokerProcessLimits) -> Self {
        limits.spec
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrokerLimitsError {
    InvalidConnectionCount,
    InvalidCallCount,
    ZeroBudget,
    InvalidCodecConcurrency,
    InsufficientProgressWorkers,
    SizeOverflow,
    SpoolCannotFitCall,
    ProcessCannotFitCall,
}

impl fmt::Display for BrokerLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConnectionCount => "broker connection count must be 1 to 16",
            Self::InvalidCallCount => "invalid broker call capacity",
            Self::ZeroBudget => "broker budgets must be positive",
            Self::InvalidCodecConcurrency => "invalid broker codec concurrency",
            Self::InsufficientProgressWorkers => {
                "broker process requires three codec progress workers"
            }
            Self::SizeOverflow => "broker payload size sum overflowed",
            Self::SpoolCannotFitCall => "connection spool budget cannot fit one maximum call",
            Self::ProcessCannotFitCall => "process spool budget cannot fit one maximum call",
        })
    }
}

impl std::error::Error for BrokerLimitsError {}
