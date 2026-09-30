//! Server retry advice captured once so passing an error between callers does not restart it.

use http::HeaderMap;
use http::header::RETRY_AFTER;
use std::time::Duration;
use std::time::SystemTime;
use tokio::time::Instant;

/// The earliest time a server advised making a follow-up request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryAfter(Instant);

impl RetryAfter {
    /// Captures an interval measured from receipt of the server's advice.
    pub fn from_delay(delay: Duration) -> Option<Self> {
        Instant::now().checked_add(delay).map(Self)
    }

    /// Captures an absolute deadline as monotonic advice without restarting its delay.
    ///
    /// Expired advice remains present with zero remaining delay. Wall time is sampled before
    /// monotonic time, so a scheduling pause between the samples cannot shorten the advice.
    /// Nanoseconds are retained; an unrepresentable monotonic deadline returns `None`.
    pub fn from_system_time(deadline: SystemTime) -> Option<Self> {
        let wall_now = SystemTime::now();
        let now = Instant::now();
        Self::from_system_time_at(deadline, wall_now, now)
    }

    /// Exports the already-running deadline for another process sharing the system clock.
    ///
    /// Monotonic time is sampled before wall time so the sampling interval cannot shorten the
    /// advice. Expired deadlines retain their elapsed time and precision is not rounded. A
    /// clock adjustment during transit can affect the receiving process's deadline; once that
    /// process calls [`Self::from_system_time`], subsequent wall-clock changes have no effect.
    pub fn to_system_time(self) -> Option<SystemTime> {
        let now = Instant::now();
        let wall_now = SystemTime::now();
        self.to_system_time_at(now, wall_now)
    }

    fn from_system_time_at(deadline: SystemTime, wall_now: SystemTime, now: Instant) -> Option<Self> {
        now.checked_add(deadline.duration_since(wall_now).unwrap_or_default())
            .map(Self)
    }

    fn to_system_time_at(self, now: Instant, wall_now: SystemTime) -> Option<SystemTime> {
        match self.0.checked_duration_since(now) {
            Some(remaining) => wall_now.checked_add(remaining),
            None => wall_now.checked_sub(now.duration_since(self.0)),
        }
    }

    /// Reads either nonnegative delay seconds or an HTTP date from `Retry-After`.
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        Self::from_header(headers.get(RETRY_AFTER)?.to_str().ok()?)
    }

    /// Reads a `Retry-After` value, including values carried inside a streamed error.
    pub fn from_header(value: &str) -> Option<Self> {
        // Sample wall time first so a scheduling pause cannot shorten the server's deadline.
        let now = SystemTime::now();
        let received_at = Instant::now();
        let value = value.trim();
        let delay = if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
            Duration::from_secs(value.parse().ok()?)
        } else {
            httpdate::parse_http_date(value)
                .ok()?
                .duration_since(now)
                .unwrap_or_default()
        };
        received_at.checked_add(delay).map(Self)
    }

    /// Returns the original deadline for callers that pass advice on or sleep until it.
    pub fn deadline(self) -> Instant {
        self.0
    }

    /// Returns the remaining delay, or zero after the deadline has passed.
    pub fn remaining_delay(self) -> Duration {
        self.0.saturating_duration_since(Instant::now())
    }
}

#[cfg(test)]
#[path = "retry_after_tests.rs"]
mod tests;
