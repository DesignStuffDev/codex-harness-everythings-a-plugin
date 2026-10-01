//! Declarations request host services; they never grant host authority.

use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;

/// Version of the optional manifest declaration, separate from service versions.
pub const SERVICE_REQUIREMENTS_VERSION: u32 = 1;
/// The manifest and broker offer share this descriptor-count ceiling.
pub const MAX_HOST_SERVICES: usize = 32;
pub const MAX_SERVICE_OPERATIONS: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServicePresence {
    Required,
    Optional,
}

/// A checked service declaration. Operations are exact, not a wildcard grant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawHostServiceRequirement")]
pub struct HostServiceRequirement {
    name: String,
    version: u32,
    operations: Vec<String>,
    presence: ServicePresence,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHostServiceRequirement {
    name: String,
    version: u32,
    operations: Vec<String>,
    presence: ServicePresence,
}

impl HostServiceRequirement {
    pub fn new(
        name: String,
        version: u32,
        operations: Vec<String>,
        presence: ServicePresence,
    ) -> Result<Self, ServiceRequirementsError> {
        if name.is_empty() || name.len() > 128 {
            return Err(ServiceRequirementsError::InvalidServiceName);
        }
        if version == 0 {
            return Err(ServiceRequirementsError::InvalidServiceVersion);
        }
        if operations.is_empty() || operations.len() > MAX_SERVICE_OPERATIONS {
            return Err(ServiceRequirementsError::InvalidOperationCount);
        }
        let mut seen = BTreeSet::new();
        for operation in &operations {
            if operation.is_empty() || operation.len() > 256 {
                return Err(ServiceRequirementsError::InvalidOperationName);
            }
            if !seen.insert(operation) {
                return Err(ServiceRequirementsError::DuplicateOperation);
            }
        }
        Ok(Self {
            name,
            version,
            operations,
            presence,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn operations(&self) -> &[String] {
        &self.operations
    }
    pub fn presence(&self) -> ServicePresence {
        self.presence
    }
}

impl TryFrom<RawHostServiceRequirement> for HostServiceRequirement {
    type Error = ServiceRequirementsError;

    fn try_from(raw: RawHostServiceRequirement) -> Result<Self, Self::Error> {
        Self::new(raw.name, raw.version, raw.operations, raw.presence)
    }
}

/// Optional checked manifest field. Omit it for a plain component package.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawServiceRequirementsV1")]
pub struct ServiceRequirementsV1 {
    version: u32,
    services: Vec<HostServiceRequirement>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawServiceRequirementsV1 {
    version: u32,
    services: Vec<HostServiceRequirement>,
}

impl ServiceRequirementsV1 {
    pub fn new(services: Vec<HostServiceRequirement>) -> Result<Self, ServiceRequirementsError> {
        if services.is_empty() || services.len() > MAX_HOST_SERVICES {
            return Err(ServiceRequirementsError::InvalidServiceCount);
        }
        let mut seen = BTreeSet::new();
        for service in &services {
            if !seen.insert((service.name(), service.version())) {
                return Err(ServiceRequirementsError::DuplicateService);
            }
        }
        Ok(Self {
            version: SERVICE_REQUIREMENTS_VERSION,
            services,
        })
    }

    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn services(&self) -> &[HostServiceRequirement] {
        &self.services
    }
}

impl TryFrom<RawServiceRequirementsV1> for ServiceRequirementsV1 {
    type Error = ServiceRequirementsError;

    fn try_from(raw: RawServiceRequirementsV1) -> Result<Self, Self::Error> {
        if raw.version != SERVICE_REQUIREMENTS_VERSION {
            return Err(ServiceRequirementsError::UnsupportedDeclarationVersion);
        }
        Self::new(raw.services)
    }
}

/// Canonical validation failures do not echo package-controlled names or values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceRequirementsError {
    UnsupportedDeclarationVersion,
    InvalidServiceCount,
    InvalidServiceName,
    InvalidServiceVersion,
    InvalidOperationCount,
    InvalidOperationName,
    DuplicateService,
    DuplicateOperation,
}

impl fmt::Display for ServiceRequirementsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedDeclarationVersion => "unsupported service declaration version",
            Self::InvalidServiceCount => "service declarations require 1 to 32 services",
            Self::InvalidServiceName => "service names require 1 to 128 UTF-8 bytes",
            Self::InvalidServiceVersion => "service versions must be positive",
            Self::InvalidOperationCount => "services require 1 to 32 operations",
            Self::InvalidOperationName => "operation names require 1 to 256 UTF-8 bytes",
            Self::DuplicateService => "duplicate service and version declaration",
            Self::DuplicateOperation => "duplicate service operation",
        })
    }
}

impl std::error::Error for ServiceRequirementsError {}
