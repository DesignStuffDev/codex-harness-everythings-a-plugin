use std::io::Write;
use std::num::NonZeroU64;

use codex_component_host::SessionPayloadLimits;
use codex_file_search_api::FileSearchOptions;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchError;
use serde::Deserialize;
use serde::Serialize;

use crate::WireBudget;
use crate::WirePoll;
use crate::contract::exhausted;
use crate::contract::invalid;

pub const MAX_LEASES: u32 = 16;
pub const MAX_QUERY_BYTES: u32 = 64 * 1024;
pub const MAX_ROOTS_OPTIONS_BYTES: u32 = 256 * 1024;
pub const MAX_MATCHES: u32 = 1024;
pub const MAX_SNAPSHOT_BYTES: u32 = 1024 * 1024;
pub const MAX_POLL_WAIT_MS: u32 = 1000;
pub const REQUEST_BYTES: u64 = 512 * 1024;
pub const REPLY_BYTES: u64 = 1024 * 1024 + 64 * 1024;

pub fn payload_limits() -> SessionPayloadLimits {
    SessionPayloadLimits {
        request_bytes: NonZeroU64::new(REQUEST_BYTES),
        reply_bytes: NonZeroU64::new(REPLY_BYTES),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceLimits {
    pub max_leases: u32,
    pub max_pending_polls_per_lease: u32,
    pub max_query_utf8_bytes: u32,
    pub max_roots_options_bytes: u32,
    pub max_matches: u32,
    pub max_frame_bytes: u32,
    pub max_poll_wait_ms: u32,
    pub resources: WireBudget,
}

impl ServiceLimits {
    pub fn new(resources: SearchBudget) -> Self {
        Self {
            max_leases: MAX_LEASES,
            max_pending_polls_per_lease: 1,
            max_query_utf8_bytes: MAX_QUERY_BYTES,
            max_roots_options_bytes: MAX_ROOTS_OPTIONS_BYTES,
            max_matches: MAX_MATCHES,
            max_frame_bytes: MAX_SNAPSHOT_BYTES,
            max_poll_wait_ms: MAX_POLL_WAIT_MS,
            resources: WireBudget::from_native(resources),
        }
    }

    pub fn validate(&self) -> Result<(), SearchError> {
        for (value, ceiling) in [
            (self.max_leases, MAX_LEASES),
            (self.max_query_utf8_bytes, MAX_QUERY_BYTES),
            (self.max_roots_options_bytes, MAX_ROOTS_OPTIONS_BYTES),
            (self.max_matches, MAX_MATCHES),
            (self.max_frame_bytes, MAX_SNAPSHOT_BYTES),
            (self.max_poll_wait_ms, MAX_POLL_WAIT_MS),
        ] {
            if value == 0 || value > ceiling {
                return Err(invalid("file-search negotiated ceiling is invalid"));
            }
        }
        if self.max_pending_polls_per_lease != 1 {
            return Err(invalid("file-search requires one retained poll per lease"));
        }
        self.resources.into_native()?;
        Ok(())
    }

    pub fn negotiate(&self, requested: &Self) -> Result<Self, SearchError> {
        self.validate()?;
        requested.validate()?;
        let mut result = self.clone();
        result.max_leases = result.max_leases.min(requested.max_leases);
        result.max_query_utf8_bytes = result
            .max_query_utf8_bytes
            .min(requested.max_query_utf8_bytes);
        result.max_roots_options_bytes = result
            .max_roots_options_bytes
            .min(requested.max_roots_options_bytes);
        result.max_matches = result.max_matches.min(requested.max_matches);
        result.max_frame_bytes = result.max_frame_bytes.min(requested.max_frame_bytes);
        result.max_poll_wait_ms = result.max_poll_wait_ms.min(requested.max_poll_wait_ms);
        result.resources.max_index_entries.0 = result
            .resources
            .max_index_entries
            .0
            .min(requested.resources.max_index_entries.0);
        result.resources.max_index_bytes.0 = result
            .resources
            .max_index_bytes
            .0
            .min(requested.resources.max_index_bytes.0);
        result.resources.max_worker_threads.0 = result
            .resources
            .max_worker_threads
            .0
            .min(requested.resources.max_worker_threads.0);
        Ok(result)
    }

    pub fn validate_negotiated(&self, requested: &Self) -> Result<(), SearchError> {
        if self.negotiate(requested)? != *self {
            return Err(invalid("file-search service exceeded requested ceilings"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseLimits {
    pub max_query_utf8_bytes: u32,
    pub max_matches: u32,
    pub max_frame_bytes: u32,
    pub max_poll_wait_ms: u32,
    pub max_pending_polls: u32,
    pub max_in_flight_updates: u32,
}

impl From<&ServiceLimits> for LeaseLimits {
    fn from(limits: &ServiceLimits) -> Self {
        Self {
            max_query_utf8_bytes: limits.max_query_utf8_bytes,
            max_matches: limits.max_matches,
            max_frame_bytes: limits.max_frame_bytes,
            max_poll_wait_ms: limits.max_poll_wait_ms,
            max_pending_polls: 1,
            max_in_flight_updates: 1,
        }
    }
}

impl LeaseLimits {
    pub fn validate_against(&self, service: &ServiceLimits) -> Result<(), SearchError> {
        service.validate()?;
        if self != &Self::from(service) {
            return Err(invalid("file-search lease changed negotiated ceilings"));
        }
        Ok(())
    }
}

pub fn validate_wire_poll(
    poll: &WirePoll,
    options: &FileSearchOptions,
    limits: &LeaseLimits,
) -> Result<(), SearchError> {
    if let WirePoll::Changed { frame } = poll {
        check_size(frame, u64::from(limits.max_frame_bytes))?;
        if frame.query.len() > limits.max_query_utf8_bytes as usize {
            return Err(exhausted(
                "file-search frame query exceeds negotiated ceiling",
            ));
        }
        if let Some(snapshot) = &frame.snapshot
            && (snapshot.matches.len() > options.limit.get()
                || snapshot.matches.len() > limits.max_matches as usize)
        {
            return Err(exhausted(
                "file-search frame exceeds negotiated match ceiling",
            ));
        }
    }
    Ok(())
}

pub(crate) fn check_size<T: Serialize>(value: &T, maximum: u64) -> Result<(), SearchError> {
    let mut output = Counter {
        remaining: maximum,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut output, value);
    if output.exceeded {
        return Err(exhausted(
            "file-search serialized value exceeds negotiated ceiling",
        ));
    }
    result.map_err(|_| invalid("file-search value cannot be encoded"))
}

struct Counter {
    remaining: u64,
    exceeded: bool,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let Some(remaining) = self.remaining.checked_sub(bytes.len() as u64) else {
            self.exceeded = true;
            return Err(std::io::Error::other(
                "file-search serialized value is too large",
            ));
        };
        self.remaining = remaining;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
