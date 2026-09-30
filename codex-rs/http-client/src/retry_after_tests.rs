use super::*;
use http::HeaderValue;
use pretty_assertions::assert_eq;

/// Delta seconds start at receipt and remain present once their deadline expires.
#[tokio::test(start_paused = true)]
async fn delta_seconds_count_down_to_zero() {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_static("10"));
    let advice = RetryAfter::from_headers(&headers).expect("valid advice");
    let deadline = Instant::now() + Duration::from_secs(10);
    assert_eq!(advice.deadline(), deadline);
    assert_eq!(advice.remaining_delay(), Duration::from_secs(10));
    tokio::time::advance(Duration::from_secs(4)).await;
    assert_eq!(advice.remaining_delay(), Duration::from_secs(6));
    tokio::time::advance(Duration::from_secs(7)).await;
    assert_eq!(advice.remaining_delay(), Duration::ZERO);
}

/// HTTP dates share the monotonic clock; expired dates and explicit zero are still advice.
#[tokio::test(start_paused = true)]
async fn http_dates_and_invalid_advice() {
    let value = httpdate::fmt_http_date(SystemTime::now() + Duration::from_secs(60));
    let date = httpdate::parse_http_date(&value).expect("formatted date");
    let before = SystemTime::now();
    let advice = RetryAfter::from_header(&value).expect("valid date");
    let after = SystemTime::now();
    let remaining = advice.remaining_delay();
    assert!(
        (date.duration_since(after).unwrap()..=date.duration_since(before).unwrap())
            .contains(&remaining)
    );
    tokio::time::advance(Duration::from_secs(3)).await;
    assert_eq!(advice.remaining_delay(), remaining - Duration::from_secs(3));

    for value in ["0", "Thu, 01 Jan 1970 00:00:00 GMT"] {
        assert_eq!(
            RetryAfter::from_header(value).map(RetryAfter::remaining_delay),
            Some(Duration::ZERO)
        );
    }
    for value in ["", "-1", "+1", "1.5", "invalid", "18446744073709551615"] {
        assert_eq!(RetryAfter::from_header(value), None, "{value}");
    }
}

#[tokio::test(start_paused = true)]
async fn absolute_deadline_retains_elapsed_time_and_nanoseconds() {
    let started = Instant::now();
    let wall_started = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let delay = Duration::new(10, 123_456_789);
    let advice = RetryAfter::from_delay(delay).expect("valid advice");
    tokio::time::advance(Duration::from_secs(4)).await;

    let encoded = advice
        .to_system_time_at(Instant::now(), wall_started + Duration::from_secs(4))
        .expect("representable wall time");
    assert_eq!(encoded, wall_started + delay);

    // Two seconds in transit must consume the original advice, not start it again.
    tokio::time::advance(Duration::from_secs(2)).await;
    let decoded = RetryAfter::from_system_time_at(
        encoded,
        wall_started + Duration::from_secs(6),
        Instant::now(),
    );
    assert_eq!(decoded, Some(advice));
    assert_eq!(decoded.unwrap().deadline(), started + delay);
    assert_eq!(
        decoded.unwrap().remaining_delay(),
        Duration::new(4, 123_456_789)
    );
}

#[tokio::test(start_paused = true)]
async fn expired_absolute_advice_remains_present() {
    let wall_started = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    let advice = RetryAfter::from_delay(Duration::from_secs(2)).expect("valid advice");
    tokio::time::advance(Duration::from_secs(3)).await;
    let encoded = advice
        .to_system_time_at(Instant::now(), wall_started + Duration::from_secs(3))
        .expect("representable wall time");
    assert_eq!(encoded, wall_started + Duration::from_secs(2));
    let captured = RetryAfter::from_system_time_at(
        encoded,
        wall_started + Duration::from_secs(3),
        Instant::now(),
    );
    assert_eq!(captured.map(RetryAfter::remaining_delay), Some(Duration::ZERO));
    assert_eq!(
        RetryAfter::from_system_time(SystemTime::UNIX_EPOCH).map(RetryAfter::remaining_delay),
        Some(Duration::ZERO)
    );
}

#[test]
fn public_absolute_deadline_conversion_uses_conservative_sample_order() {
    let deadline = SystemTime::now() + Duration::new(60, 123_456_789);
    let advice = RetryAfter::from_system_time(deadline).expect("valid advice");
    let exported = advice.to_system_time().expect("representable wall time");
    // Both conversions can add sampling latency but do not round away nanoseconds or
    // shorten the deadline under an unchanged system clock.
    assert!(exported >= deadline);
}

#[cfg(target_os = "linux")]
#[test]
fn absolute_deadline_conversion_checks_platform_overflow() {
    // Linux SystemTime can represent this value, but adding its remaining duration
    // to a positive monotonic instant cannot fit in the native timespec.
    let largest_seconds = SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(i64::MAX as u64))
        .expect("Linux SystemTime supports signed 64-bit seconds");
    let now = Instant::now();
    assert_eq!(
        RetryAfter::from_system_time_at(largest_seconds, SystemTime::UNIX_EPOCH, now),
        None
    );
    let advice = RetryAfter::from_delay(Duration::from_secs(1)).expect("valid advice");
    assert_eq!(advice.to_system_time_at(now, largest_seconds), None);
}
