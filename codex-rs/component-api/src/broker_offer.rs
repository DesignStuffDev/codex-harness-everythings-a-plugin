//! Checked negotiation data only; a matching acknowledgement grants no live authority.
//! Parse original bounded handshake bytes; these constructors do not bound serde allocation.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::BrokerConnectionHandle;
use crate::BrokerConnectionLimits;
use crate::BrokerNegotiationError;
use crate::GrantedOperationV1;
use crate::MAX_HOST_SERVICES;
use crate::ServiceGrantV1;
use crate::ServicePresence;
use crate::ServiceRequirementsV1;

/// Broker extension version, distinct from manifest declarations and service versions.
pub const BROKER_EXTENSION_VERSION: u32 = 1;

/// Canonical checked offer. Construction neither selects services nor reserves resources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawBrokerOfferV1")]
pub struct BrokerOfferV1 {
    version: u32,
    connection: BrokerConnectionHandle,
    limits: BrokerConnectionLimits,
    services: Vec<ServiceGrantV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBrokerOfferV1 {
    version: u32,
    connection: BrokerConnectionHandle,
    limits: BrokerConnectionLimits,
    services: Vec<ServiceGrantV1>,
}

impl BrokerOfferV1 {
    pub fn new(
        connection: BrokerConnectionHandle,
        limits: BrokerConnectionLimits,
        mut services: Vec<ServiceGrantV1>,
    ) -> Result<Self, BrokerNegotiationError> {
        if services.len() > MAX_HOST_SERVICES {
            return Err(BrokerNegotiationError::TooManyServices);
        }
        let mut seen = BTreeSet::new();
        for service in &services {
            if !seen.insert((service.name(), service.version())) {
                return Err(BrokerNegotiationError::DuplicateService);
            }
            for operation in service.operations() {
                if operation.max_request_bytes() > limits.as_spec().max_request_bytes
                    || operation.max_response_bytes() > limits.as_spec().max_response_bytes
                {
                    return Err(BrokerNegotiationError::OperationLimitExceedsConnection);
                }
            }
        }
        drop(seen);
        services.sort_by(|left, right| {
            (left.name(), left.version()).cmp(&(right.name(), right.version()))
        });
        Ok(Self {
            version: BROKER_EXTENSION_VERSION,
            connection,
            limits,
            services,
        })
    }

    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn connection(&self) -> &BrokerConnectionHandle {
        &self.connection
    }
    pub fn limits(&self) -> &BrokerConnectionLimits {
        &self.limits
    }
    pub fn services(&self) -> &[ServiceGrantV1] {
        &self.services
    }

    /// Require exact requested operation sets. Optional services may be absent entirely.
    pub fn validate_requirements(
        &self,
        requirements: &ServiceRequirementsV1,
    ) -> Result<(), BrokerNegotiationError> {
        for service in &self.services {
            let declaration = requirements
                .services()
                .iter()
                .find(|declaration| {
                    declaration.name() == service.name()
                        && declaration.version() == service.version()
                })
                .ok_or(BrokerNegotiationError::UndeclaredService)?;
            let requested: BTreeSet<_> = declaration
                .operations()
                .iter()
                .map(String::as_str)
                .collect();
            let granted: BTreeSet<_> = service
                .operations()
                .iter()
                .map(GrantedOperationV1::name)
                .collect();
            if requested != granted {
                return Err(BrokerNegotiationError::OperationSetMismatch);
            }
        }
        for declaration in requirements.services() {
            if declaration.presence() == ServicePresence::Required
                && !self.services.iter().any(|service| {
                    service.name() == declaration.name()
                        && service.version() == declaration.version()
                })
            {
                return Err(BrokerNegotiationError::MissingRequiredService);
            }
        }
        Ok(())
    }

    /// Compare every canonical field, including handles, directional caps and all limits.
    /// Live ownership, revocation and handshake-envelope validation remain host obligations.
    pub fn validate_acknowledgement(
        &self,
        acknowledgement: &BrokerAcknowledgementV1,
    ) -> Result<(), BrokerNegotiationError> {
        if self != &acknowledgement.offer {
            return Err(BrokerNegotiationError::AcknowledgementMismatch);
        }
        Ok(())
    }
}

impl TryFrom<RawBrokerOfferV1> for BrokerOfferV1 {
    type Error = BrokerNegotiationError;

    fn try_from(raw: RawBrokerOfferV1) -> Result<Self, Self::Error> {
        if raw.version != BROKER_EXTENSION_VERSION {
            return Err(BrokerNegotiationError::UnsupportedVersion);
        }
        Self::new(raw.connection, raw.limits, raw.services)
    }
}

/// Exact checked offer echo. Transparent decoding reuses the checked offer shape.
/// Callers must deserialize original bounded bytes to preserve duplicate-field rejection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BrokerAcknowledgementV1 {
    offer: BrokerOfferV1,
}

impl BrokerAcknowledgementV1 {
    pub fn from_offer(offer: &BrokerOfferV1) -> Self {
        Self {
            offer: offer.clone(),
        }
    }
}
