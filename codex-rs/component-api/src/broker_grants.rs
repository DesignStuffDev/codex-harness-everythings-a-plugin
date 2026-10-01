//! Checked grant descriptions. Host runtime state must separately verify authority.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::BrokerNegotiationError;
use crate::MAX_SERVICE_OPERATIONS;
use crate::ServiceAuthorityHandle;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawGrantedOperation")]
pub struct GrantedOperationV1 {
    name: String,
    max_request_bytes: u64,
    max_response_bytes: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGrantedOperation {
    name: String,
    max_request_bytes: u64,
    max_response_bytes: u64,
}

impl GrantedOperationV1 {
    pub fn new(
        name: String,
        max_request_bytes: u64,
        max_response_bytes: u64,
    ) -> Result<Self, BrokerNegotiationError> {
        if name.is_empty() || name.len() > 256 {
            return Err(BrokerNegotiationError::InvalidOperationName);
        }
        if max_request_bytes == 0 || max_response_bytes == 0 {
            return Err(BrokerNegotiationError::InvalidOperationLimit);
        }
        Ok(Self {
            name,
            max_request_bytes,
            max_response_bytes,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn max_request_bytes(&self) -> u64 {
        self.max_request_bytes
    }
    pub fn max_response_bytes(&self) -> u64 {
        self.max_response_bytes
    }
}

impl TryFrom<RawGrantedOperation> for GrantedOperationV1 {
    type Error = BrokerNegotiationError;
    fn try_from(raw: RawGrantedOperation) -> Result<Self, Self::Error> {
        Self::new(raw.name, raw.max_request_bytes, raw.max_response_bytes)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawServiceGrant")]
pub struct ServiceGrantV1 {
    name: String,
    version: u32,
    operations: Vec<GrantedOperationV1>,
    authority: ServiceAuthorityHandle,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawServiceGrant {
    name: String,
    version: u32,
    operations: Vec<GrantedOperationV1>,
    authority: ServiceAuthorityHandle,
}

impl ServiceGrantV1 {
    pub fn new(
        name: String,
        version: u32,
        mut operations: Vec<GrantedOperationV1>,
        authority: ServiceAuthorityHandle,
    ) -> Result<Self, BrokerNegotiationError> {
        if name.is_empty() || name.len() > 128 {
            return Err(BrokerNegotiationError::InvalidServiceName);
        }
        if version == 0 {
            return Err(BrokerNegotiationError::InvalidServiceVersion);
        }
        if operations.is_empty() || operations.len() > MAX_SERVICE_OPERATIONS {
            return Err(BrokerNegotiationError::InvalidOperationCount);
        }
        {
            let mut seen = BTreeSet::new();
            for operation in &operations {
                if !seen.insert(operation.name()) {
                    return Err(BrokerNegotiationError::DuplicateOperation);
                }
            }
        }
        operations.sort_by(|left, right| left.name().cmp(right.name()));
        Ok(Self {
            name,
            version,
            operations,
            authority,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn operations(&self) -> &[GrantedOperationV1] {
        &self.operations
    }
    pub fn authority(&self) -> &ServiceAuthorityHandle {
        &self.authority
    }
}

impl TryFrom<RawServiceGrant> for ServiceGrantV1 {
    type Error = BrokerNegotiationError;
    fn try_from(raw: RawServiceGrant) -> Result<Self, Self::Error> {
        Self::new(raw.name, raw.version, raw.operations, raw.authority)
    }
}
