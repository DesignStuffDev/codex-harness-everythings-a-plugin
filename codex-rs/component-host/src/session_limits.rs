use std::fmt;
use std::io;
use std::io::Write;
use std::num::NonZeroU64;

use anyhow::Result;
use anyhow::ensure;
use serde_json::Value;

pub(super) const CONTROL_BYTES: u64 = 64 * 1024;

/// Optional limits on serialized JSON payloads in a persistent component session.
/// Defaults preserve unbounded ordinary payloads. Cleanup requests and replies
/// retain their separate 64 KiB limit. Limits do not change the wire protocol.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SessionPayloadLimits {
    /// Ordinary request bodies only; reserved controls retain their fixed cap.
    pub request_bytes: Option<NonZeroU64>,
    /// If configured, must be at least 64 KiB so valid control replies still fit.
    /// Constructors reject smaller limits rather than silently increasing them.
    pub reply_bytes: Option<NonZeroU64>,
}

/// The logical payload rejected by a locally configured session limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadKind {
    Request,
    Reply,
}

/// A local payload rejection. Remote transport error strings are not converted
/// into this type, and diagnostics never include payload contents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PayloadLimitExceeded {
    pub kind: PayloadKind,
    pub limit_bytes: u64,
}

impl fmt::Display for PayloadLimitExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            PayloadKind::Request => "request",
            PayloadKind::Reply => "reply",
        };
        write!(
            formatter,
            "component session {kind} payload exceeds {} byte limit",
            self.limit_bytes
        )
    }
}

impl std::error::Error for PayloadLimitExceeded {}

impl SessionPayloadLimits {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            self.reply_bytes
                .is_none_or(|limit| limit.get() >= CONTROL_BYTES),
            "component session reply limit must be at least 64 KiB to admit control replies"
        );
        Ok(())
    }

    pub(super) fn check_request(&self, value: &Value) -> Result<()> {
        match self.request_bytes {
            Some(limit) => check_value(value, PayloadKind::Request, limit.get()),
            None => Ok(()),
        }
    }

    pub(super) fn check_reply(&self, value: &Value, is_control: bool) -> Result<()> {
        if is_control {
            let limit = self
                .reply_bytes
                .map_or(CONTROL_BYTES, |limit| limit.get().min(CONTROL_BYTES));
            check_value(value, PayloadKind::Reply, limit)
        } else {
            match self.reply_bytes {
                Some(limit) => check_value(value, PayloadKind::Reply, limit.get()),
                None => Ok(()),
            }
        }
    }

    pub(super) fn check_incoming(&self, kind: PayloadKind, bytes: u64) -> Result<()> {
        let limit = match kind {
            PayloadKind::Request => self.request_bytes,
            PayloadKind::Reply => self.reply_bytes,
        };
        if let Some(limit) = limit
            && bytes > limit.get()
        {
            return Err(PayloadLimitExceeded {
                kind,
                limit_bytes: limit.get(),
            }
            .into());
        }
        Ok(())
    }

    pub(super) fn check_control(value: &Value) -> Result<()> {
        check_value(value, PayloadKind::Request, CONTROL_BYTES)
    }
}

fn check_value(value: &Value, kind: PayloadKind, limit_bytes: u64) -> Result<()> {
    let error = PayloadLimitExceeded { kind, limit_bytes };
    if lower_bound_fits(value, limit_bytes).is_none() {
        return Err(error.into());
    }
    let mut counter = LimitedCounter {
        remaining: limit_bytes,
        exceeded: false,
    };
    let serialized = serde_json::to_writer(&mut counter, value);
    if counter.exceeded {
        return Err(error.into());
    }
    serialized?;
    Ok(())
}

enum Children<'a> {
    Array(std::slice::Iter<'a, Value>),
    Object(serde_json::map::Iter<'a>),
}

// A serializer scans strings to find escapes before writing their contents.
// Reject obviously oversized values first using string lengths and container
// punctuation. Visit children incrementally so both traversal and its stack are
// bounded by the byte budget, including for wide or deeply nested values.
fn lower_bound_fits(mut value: &Value, mut remaining: u64) -> Option<()> {
    let mut charge = |bytes: usize| {
        remaining = remaining.checked_sub(bytes as u64)?;
        Some(())
    };
    let mut pending = Vec::new();
    loop {
        match value {
            Value::Null => charge(4)?,
            Value::Bool(true) => charge(4)?,
            Value::Bool(false) => charge(5)?,
            Value::Number(_) => charge(1)?,
            Value::String(text) => {
                charge(2)?;
                charge(text.len())?;
            }
            Value::Array(values) => {
                charge(2)?;
                charge(values.len().saturating_sub(1))?;
                pending.push(Children::Array(values.iter()));
            }
            Value::Object(values) => {
                charge(2)?;
                charge(values.len().saturating_sub(1))?;
                pending.push(Children::Object(values.iter()));
            }
        }
        loop {
            let Some(children) = pending.last_mut() else {
                return Some(());
            };
            let next = match children {
                Children::Array(values) => values.next().map(|value| (None, value)),
                Children::Object(values) => values.next().map(|(key, value)| (Some(key), value)),
            };
            if let Some((key, child)) = next {
                if let Some(key) = key {
                    charge(3)?;
                    charge(key.len())?;
                }
                value = child;
                break;
            }
            pending.pop();
        }
    }
}

struct LimitedCounter {
    remaining: u64,
    exceeded: bool,
}

impl Write for LimitedCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(remaining) = self.remaining.checked_sub(bytes.len() as u64) else {
            self.exceeded = true;
            return Err(io::Error::other("session payload byte budget exhausted"));
        };
        self.remaining = remaining;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "session_limits_tests.rs"]
mod tests;
