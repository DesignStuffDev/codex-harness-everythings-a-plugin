//! Application-await cancellation and observed HTTP-body budgets.
//! Client construction, DNS and ordinary Runtime teardown can exceed these
//! cooperative deadlines; the registered native worker remains their owner.
use super::SyncFailure;
use super::ownership::SyncAttempt;
use codex_http_client::HttpError;
use codex_http_client::HttpResponse;
use codex_http_client::RequestBuilder;
use std::future::Future;
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Copy)]
pub(super) struct HttpLimits {
    pub(super) body_bytes: usize,
    pub(super) diagnostic_bytes: usize,
    pub(super) timeout: Duration,
}
pub(super) const METADATA_LIMITS: HttpLimits = HttpLimits {
    body_bytes: 1024 * 1024,
    diagnostic_bytes: 8 * 1024,
    timeout: Duration::from_secs(30),
};
pub(super) const ARCHIVE_LIMITS: HttpLimits = HttpLimits {
    body_bytes: 64 * 1024 * 1024,
    ..METADATA_LIMITS
};
const POLL_INTERVAL: Duration = Duration::from_millis(5);

async fn await_io<T>(
    attempt: &SyncAttempt,
    deadline: Instant,
    context: &str,
    future: impl Future<Output = Result<T, HttpError>>,
) -> Result<T, SyncFailure> {
    attempt.admit_stage()?;
    tokio::pin!(future);
    loop {
        attempt.admit_stage()?;
        if Instant::now() >= deadline {
            return Err(format!("{context} timed out across request and response body").into());
        }
        let next = (Instant::now() + POLL_INTERVAL).min(deadline);
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(tokio::time::Instant::from_std(next)) => {},
            result = &mut future => {
                attempt.admit_stage()?;
                if Instant::now() >= deadline { return Err(format!("{context} timed out across request and response body").into()); }
                return result.map_err(|error| format!("failed to {context}: {}", bounded_text(error.to_string().as_bytes(), 8 * 1024)).into());
            }
        }
    }
}

fn bounded_text(bytes: &[u8], cap: usize) -> String {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(cap)]);
    let mut end = text.len().min(cap);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

pub(super) fn diagnostic_url(url: &str) -> String {
    bounded_text(url.as_bytes(), 1024)
}

pub(super) async fn fetch_bytes(
    attempt: &SyncAttempt,
    request: RequestBuilder,
    url: &str,
    context: &str,
    limits: HttpLimits,
) -> Result<Vec<u8>, SyncFailure> {
    collect(
        attempt,
        request,
        url,
        context,
        limits,
        #[cfg(test)] /*observer*/ None,
    )
    .await
    .map(http::Response::into_body)
}

#[cfg(test)]
async fn fetch_bytes_observing(
    attempt: &SyncAttempt,
    request: RequestBuilder,
    url: &str,
    context: &str,
    limits: HttpLimits,
    observer: &dyn Fn(),
) -> Result<Vec<u8>, SyncFailure> {
    collect(attempt, request, url, context, limits, Some(observer))
        .await
        .map(http::Response::into_body)
}

pub(super) async fn fetch_text(
    attempt: &SyncAttempt,
    request: RequestBuilder,
    url: &str,
    context: &str,
    limits: HttpLimits,
) -> Result<String, SyncFailure> {
    let buffered = collect(
        attempt,
        request,
        url,
        context,
        limits,
        #[cfg(test)] /*observer*/ None,
    )
    .await?;
    attempt.admit_stage()?;
    // Preserve the existing charset, BOM and replacement decoding only after
    // policy-aware network consumption has satisfied the observed byte cap.
    // This response contains owned local bytes; decoding creates no request.
    let text = HttpResponse::from(buffered).text().await.map_err(|error| {
        SyncFailure::Ordinary(format!(
            "failed to decode {context}: {}",
            bounded_text(error.to_string().as_bytes(), 8 * 1024)
        ))
    })?;
    attempt.admit_stage()?;
    Ok(text)
}

async fn collect(
    attempt: &SyncAttempt,
    request: RequestBuilder,
    url: &str,
    context: &str,
    limits: HttpLimits,
    #[cfg(test)] observer: Option<&dyn Fn()>,
) -> Result<http::Response<Vec<u8>>, SyncFailure> {
    let deadline = Instant::now() + limits.timeout;
    let description = format!("{context} from {}", diagnostic_url(url));
    // This function owns the response. Returning on stop/limit drops the whole
    // response/session, not only a borrowed chunk future. The enclosing worker
    // still performs ordinary client and Runtime teardown before proceeding.
    let mut response = await_io(attempt, deadline, &description, request.send()).await?;
    let status = response.status();
    let headers = response.headers().clone();
    if response
        .content_length()
        .is_some_and(|length| length > limits.body_bytes as u64)
    {
        return Err(format!(
            "{description} failed with status {status}: declared body exceeds {} byte limit",
            limits.body_bytes
        )
        .into());
    }
    let mut body = Vec::new();
    let mut observed = 0usize;
    loop {
        let chunk = await_io(attempt, deadline, &description, response.chunk()).await?;
        let Some(chunk) = chunk else {
            break;
        };
        observed = observed.checked_add(chunk.len()).filter(|size| *size <= limits.body_bytes)
            .ok_or_else(|| SyncFailure::Ordinary(format!("{description} failed with status {status}: observed body exceeds {} byte limit", limits.body_bytes)))?;
        if status.is_success() {
            body.extend_from_slice(&chunk);
        } else {
            let retained = chunk
                .len()
                .min(limits.diagnostic_bytes.saturating_sub(body.len()));
            body.extend_from_slice(&chunk[..retained]);
            if observed > limits.diagnostic_bytes {
                return Err(format!("{description} failed with status {status}: {} [truncated at {} diagnostic bytes]", bounded_text(&body, limits.diagnostic_bytes), limits.diagnostic_bytes).into());
            }
        }
        #[cfg(test)]
        if !chunk.is_empty()
            && let Some(observer) = observer
        {
            observer();
        }
    }
    attempt.admit_stage()?;
    if !status.is_success() {
        return Err(format!(
            "{description} failed with status {status}: {}",
            bounded_text(&body, limits.diagnostic_bytes)
        )
        .into());
    }
    let mut buffered = http::Response::new(body);
    *buffered.status_mut() = status;
    *buffered.headers_mut() = headers;
    Ok(buffered)
}

#[cfg(test)]
#[path = "bounded_http_tests.rs"]
mod tests;
