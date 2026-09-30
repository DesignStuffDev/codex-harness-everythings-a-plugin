//! Public transport classification captured without private error sources or request URLs.

use std::fmt;

use http::StatusCode;

use crate::RouteAwareRequestError;
use crate::RouteFailureClass;

/// A connection failure received across a process boundary.
///
/// The fields retain the native HTTP error queries used by retry and reporting policy. Several
/// flags may be true at once. This representation deliberately has no request URL, arbitrary
/// diagnostic string, or local error source chain; its display text is derived from public data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoteConnectionFailure {
    pub status: Option<StatusCode>,
    pub failure_class: Option<RouteFailureClass>,
    pub is_timeout: bool,
    pub is_connect: bool,
    pub is_builder: bool,
    pub is_body: bool,
    pub is_request: bool,
}

impl RemoteConnectionFailure {
    /// Captures policy-relevant queries without copying the error's private diagnostics.
    pub fn capture(error: &RouteAwareRequestError) -> Self {
        Self {
            status: error.status(),
            failure_class: error.failure_class(),
            is_timeout: error.is_timeout(),
            is_connect: error.is_connect(),
            is_builder: error.is_builder(),
            is_body: error.is_body(),
            is_request: error.is_request(),
        }
    }
}

impl fmt::Display for RemoteConnectionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("connection failed")?;
        if let Some(failure_class) = self.failure_class {
            write!(formatter, " ({failure_class})")?;
        }
        if let Some(status) = self.status {
            write!(formatter, ": HTTP {}", status.as_u16())?;
        }
        Ok(())
    }
}

impl std::error::Error for RemoteConnectionFailure {}

#[cfg(test)]
#[path = "remote_connection_failure_tests.rs"]
mod tests;
