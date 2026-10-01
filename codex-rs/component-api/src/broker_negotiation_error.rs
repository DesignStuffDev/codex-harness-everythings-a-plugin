use std::fmt;

/// Fixed validation failures; these never include supplied names or handles.
/// Offer/acknowledgement variants are shared with the following DTO slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrokerNegotiationError {
    UnsupportedVersion,
    InvalidConnectionHandle,
    InvalidAuthorityHandle,
    TooManyServices,
    InvalidServiceName,
    InvalidServiceVersion,
    InvalidOperationCount,
    InvalidOperationName,
    DuplicateService,
    DuplicateOperation,
    InvalidOperationLimit,
    OperationLimitExceedsConnection,
    UndeclaredService,
    MissingRequiredService,
    OperationSetMismatch,
    AcknowledgementMismatch,
}

impl fmt::Display for BrokerNegotiationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedVersion => "unsupported broker version",
            Self::InvalidConnectionHandle => "invalid broker connection handle",
            Self::InvalidAuthorityHandle => "invalid service authority handle",
            Self::TooManyServices => "too many broker services",
            Self::InvalidServiceName => "invalid broker service name",
            Self::InvalidServiceVersion => "invalid broker service version",
            Self::InvalidOperationCount => "invalid broker operation count",
            Self::InvalidOperationName => "invalid broker operation name",
            Self::DuplicateService => "duplicate broker service and version",
            Self::DuplicateOperation => "duplicate broker operation",
            Self::InvalidOperationLimit => "broker operation byte limits must be positive",
            Self::OperationLimitExceedsConnection => {
                "operation byte limit exceeds connection limit"
            }
            Self::UndeclaredService => "broker service was not declared",
            Self::MissingRequiredService => "required broker service is missing",
            Self::OperationSetMismatch => "broker operation set differs from declaration",
            Self::AcknowledgementMismatch => "broker acknowledgement differs from offer",
        })
    }
}

impl std::error::Error for BrokerNegotiationError {}
