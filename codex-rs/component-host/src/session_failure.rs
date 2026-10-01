//! Retain locally observed failure types across cloned terminal notifications.

use crate::session_limits::PayloadLimitExceeded;

#[derive(Clone, Debug)]
pub(super) struct SessionFailure {
    diagnostic: String,
    payload_limit: Option<PayloadLimitExceeded>,
}

impl SessionFailure {
    pub fn message(diagnostic: impl Into<String>) -> Self {
        Self {
            diagnostic: diagnostic.into(),
            payload_limit: None,
        }
    }

    pub fn from_error(error: anyhow::Error) -> Self {
        Self {
            diagnostic: format!("{error:#}"),
            payload_limit: error.downcast_ref::<PayloadLimitExceeded>().copied(),
        }
    }

    pub fn into_error(self) -> anyhow::Error {
        match self.payload_limit {
            Some(limit) => anyhow::Error::new(limit).context(self.diagnostic),
            None => anyhow::Error::msg(self.diagnostic),
        }
    }
}
