use std::fmt;

use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;

use crate::BrokerNegotiationError;

/// Checked connection identifier, not evidence of live host authority.
#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(try_from = "String")]
pub struct BrokerConnectionHandle(String);

impl BrokerConnectionHandle {
    pub fn new(value: String) -> Result<Self, BrokerNegotiationError> {
        if value.is_empty() || value.len() > 256 {
            return Err(BrokerNegotiationError::InvalidConnectionHandle);
        }
        Ok(Self(value))
    }
}

impl TryFrom<String> for BrokerConnectionHandle {
    type Error = BrokerNegotiationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for BrokerConnectionHandle {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl fmt::Debug for BrokerConnectionHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BrokerConnectionHandle([redacted])")
    }
}

/// Checked grant identifier; constructing or decoding it does not issue a grant.
#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(try_from = "String")]
pub struct ServiceAuthorityHandle(String);

impl ServiceAuthorityHandle {
    pub fn new(value: String) -> Result<Self, BrokerNegotiationError> {
        if value.is_empty() || value.len() > 256 {
            return Err(BrokerNegotiationError::InvalidAuthorityHandle);
        }
        Ok(Self(value))
    }
}

impl TryFrom<String> for ServiceAuthorityHandle {
    type Error = BrokerNegotiationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for ServiceAuthorityHandle {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl fmt::Debug for ServiceAuthorityHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ServiceAuthorityHandle([redacted])")
    }
}
