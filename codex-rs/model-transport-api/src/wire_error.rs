use std::fmt;

/// A category-only validation failure that never formats model or error contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    Request,
    Event,
    Item,
    BackendError,
    Status,
    Header,
    Url,
    Deadline,
    IntegerRange,
}

impl fmt::Display for WireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Request => "invalid model request schema",
            Self::Event => "invalid model event schema",
            Self::Item => "invalid provider response item",
            Self::BackendError => "invalid model backend error schema",
            Self::Status => "invalid backend HTTP status",
            Self::Header => "invalid backend HTTP metadata",
            Self::Url => "invalid public backend URL",
            Self::Deadline => "invalid backend retry deadline",
            Self::IntegerRange => "backend integer exceeds native range",
        })
    }
}

impl std::error::Error for WireError {}
