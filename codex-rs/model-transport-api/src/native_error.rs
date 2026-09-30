use crate::BackendError;
use crate::ConnectionFailure;
use crate::HttpErrorHeaders;
use crate::ModelEvent;
use crate::NetworkDenial;
use crate::RetryDeadline;
use crate::RouteFailure;
use crate::TransportFailure;
use crate::WireError;
use codex_api::ApiError;
use codex_api::TransportError;
use codex_http_client::NetworkPolicyDenied;
use codex_http_client::RemoteConnectionFailure;
use codex_http_client::RetryAfter;
use codex_http_client::RouteFailureClass;
use http::StatusCode;

/// Preserve typed backend policy and captured retry advice without opaque sources.
pub fn encode_backend_error(error: ApiError) -> Result<ModelEvent, WireError> {
    let error = match error {
        ApiError::Transport(transport) => BackendError::Transport { transport: encode_transport(transport)? },
        ApiError::Api { status, message } => BackendError::Api { status: status.as_u16(), message },
        ApiError::Stream(message) => BackendError::Stream { message },
        ApiError::ContentFilter => BackendError::ContentFilter,
        ApiError::ContextWindowExceeded => BackendError::ContextWindowExceeded,
        ApiError::QuotaExceeded => BackendError::QuotaExceeded,
        ApiError::UsageNotIncluded => BackendError::UsageNotIncluded,
        ApiError::Retryable { message, retry_after } => BackendError::Retryable { message, retry_at: encode_retry(retry_after)? },
        ApiError::RateLimitExceeded { message, retry_after } => BackendError::RateLimitExceeded { message, retry_at: encode_retry(retry_after)? },
        ApiError::RateLimit(message) => BackendError::RateLimit { message },
        ApiError::InvalidRequest { message } => BackendError::InvalidRequest { message },
        ApiError::InvalidPrompt { message } => BackendError::InvalidPrompt { message },
        ApiError::CyberPolicy { message } => BackendError::CyberPolicy { message },
        ApiError::BioPolicy { message } => BackendError::BioPolicy { message },
        ApiError::MisalignmentPolicyViolation { message, misalignment } => BackendError::MisalignmentPolicyViolation { message, misalignment },
        ApiError::FlexUnavailable => BackendError::FlexUnavailable,
        ApiError::ServerOverloaded { retry_after } => BackendError::ServerOverloaded { retry_at: encode_retry(retry_after)? },
    };
    error.validate()?;
    Ok(ModelEvent::BackendError { error })
}

pub(crate) fn decode_backend_error(error: BackendError) -> Result<ApiError, WireError> {
    error.validate()?;
    Ok(match error {
        BackendError::Transport { transport } => ApiError::Transport(decode_transport(transport)?),
        BackendError::Api { status, message } => ApiError::Api { status: status_code(status)?, message },
        BackendError::Stream { message } => ApiError::Stream(message),
        BackendError::ContentFilter => ApiError::ContentFilter,
        BackendError::ContextWindowExceeded => ApiError::ContextWindowExceeded,
        BackendError::QuotaExceeded => ApiError::QuotaExceeded,
        BackendError::UsageNotIncluded => ApiError::UsageNotIncluded,
        BackendError::Retryable { message, retry_at } => ApiError::Retryable { message, retry_after: decode_retry(retry_at)? },
        BackendError::RateLimitExceeded { message, retry_at } => ApiError::RateLimitExceeded { message, retry_after: decode_retry(retry_at)? },
        BackendError::RateLimit { message } => ApiError::RateLimit(message),
        BackendError::InvalidRequest { message } => ApiError::InvalidRequest { message },
        BackendError::InvalidPrompt { message } => ApiError::InvalidPrompt { message },
        BackendError::CyberPolicy { message } => ApiError::CyberPolicy { message },
        BackendError::BioPolicy { message } => ApiError::BioPolicy { message },
        BackendError::MisalignmentPolicyViolation { message, misalignment } => ApiError::MisalignmentPolicyViolation { message, misalignment },
        BackendError::FlexUnavailable => ApiError::FlexUnavailable,
        BackendError::ServerOverloaded { retry_at } => ApiError::ServerOverloaded { retry_after: decode_retry(retry_at)? },
    })
}

fn encode_transport(error: TransportError) -> Result<TransportFailure, WireError> {
    Ok(match error {
        TransportError::Policy(reason) => TransportFailure::Policy { reason: match reason {
            NetworkPolicyDenied::Unavailable => NetworkDenial::Unavailable,
            NetworkPolicyDenied::Destination => NetworkDenial::Destination,
            NetworkPolicyDenied::Revoked => NetworkDenial::Revoked,
            NetworkPolicyDenied::UnsupportedTransport => NetworkDenial::UnsupportedTransport,
        } },
        TransportError::Http { status, url, headers, body, retry_after } => TransportFailure::Http {
            status: status.as_u16(),
            url: url.map(crate::http_metadata::redact_url).transpose()?,
            headers: headers.map(HttpErrorHeaders::from_native), body,
            retry_at: encode_retry(retry_after)?,
        },
        TransportError::RetryLimit => TransportFailure::RetryLimit,
        TransportError::Timeout => TransportFailure::Timeout,
        TransportError::Connection(error) => {
            let captured = RemoteConnectionFailure::capture(&error);
            TransportFailure::Connection { connection: ConnectionFailure {
                message: captured.to_string(), status: captured.status.map(|status| status.as_u16()),
                failure_class: captured.failure_class.map(encode_class),
                is_timeout: captured.is_timeout, is_connect: captured.is_connect,
                is_builder: captured.is_builder, is_body: captured.is_body, is_request: captured.is_request,
            } }
        }
        TransportError::Network(message) => TransportFailure::Network { message },
        TransportError::Build(message) => TransportFailure::Build { message },
        TransportError::ResponseTooLarge { max_bytes } => TransportFailure::ResponseTooLarge {
            max_bytes: u64::try_from(max_bytes).map_err(|_| WireError::IntegerRange)?,
        },
    })
}

fn decode_transport(error: TransportFailure) -> Result<TransportError, WireError> {
    Ok(match error {
        TransportFailure::Policy { reason } => TransportError::Policy(match reason {
            NetworkDenial::Unavailable => NetworkPolicyDenied::Unavailable,
            NetworkDenial::Destination => NetworkPolicyDenied::Destination,
            NetworkDenial::Revoked => NetworkPolicyDenied::Revoked,
            NetworkDenial::UnsupportedTransport => NetworkPolicyDenied::UnsupportedTransport,
        }),
        TransportFailure::Http { status, url, headers, body, retry_at } => TransportError::Http {
            status: status_code(status)?, url,
            headers: headers.map(HttpErrorHeaders::into_native).transpose()?, body,
            retry_after: decode_retry(retry_at)?,
        },
        TransportFailure::RetryLimit => TransportError::RetryLimit,
        TransportFailure::Timeout => TransportError::Timeout,
        TransportFailure::Connection { connection } => TransportError::Connection(RemoteConnectionFailure {
            status: connection.status.map(status_code).transpose()?,
            failure_class: connection.failure_class.map(decode_class),
            is_timeout: connection.is_timeout, is_connect: connection.is_connect,
            is_builder: connection.is_builder, is_body: connection.is_body, is_request: connection.is_request,
        }.into()),
        TransportFailure::Network { message } => TransportError::Network(message),
        TransportFailure::Build { message } => TransportError::Build(message),
        TransportFailure::ResponseTooLarge { max_bytes } => TransportError::ResponseTooLarge {
            max_bytes: usize::try_from(max_bytes).map_err(|_| WireError::IntegerRange)?,
        },
    })
}

fn status_code(status: u16) -> Result<StatusCode, WireError> {
    StatusCode::from_u16(status).map_err(|_| WireError::Status)
}

fn encode_retry(advice: Option<RetryAfter>) -> Result<Option<RetryDeadline>, WireError> {
    advice.map(|advice| RetryDeadline::from_system_time(advice.to_system_time().ok_or(WireError::Deadline)?)).transpose()
}

fn decode_retry(deadline: Option<RetryDeadline>) -> Result<Option<RetryAfter>, WireError> {
    deadline.map(|deadline| RetryAfter::from_system_time(deadline.to_system_time()?).ok_or(WireError::Deadline)).transpose()
}

fn encode_class(class: RouteFailureClass) -> RouteFailure {
    match class {
        RouteFailureClass::ProxyResolutionUnavailable => RouteFailure::ProxyResolutionUnavailable,
        RouteFailureClass::ConnectTimeout => RouteFailure::ConnectTimeout,
        RouteFailureClass::ProxyAuthenticationRequired => RouteFailure::ProxyAuthenticationRequired,
        RouteFailureClass::TlsError => RouteFailure::TlsError,
        RouteFailureClass::InvalidProxyConfig => RouteFailure::InvalidProxyConfig,
        RouteFailureClass::UnsupportedProxyScheme => RouteFailure::UnsupportedProxyScheme,
        RouteFailureClass::ResolverError => RouteFailure::ResolverError,
    }
}

fn decode_class(class: RouteFailure) -> RouteFailureClass {
    match class {
        RouteFailure::ProxyResolutionUnavailable => RouteFailureClass::ProxyResolutionUnavailable,
        RouteFailure::ConnectTimeout => RouteFailureClass::ConnectTimeout,
        RouteFailure::ProxyAuthenticationRequired => RouteFailureClass::ProxyAuthenticationRequired,
        RouteFailure::TlsError => RouteFailureClass::TlsError,
        RouteFailure::InvalidProxyConfig => RouteFailureClass::InvalidProxyConfig,
        RouteFailure::UnsupportedProxyScheme => RouteFailureClass::UnsupportedProxyScheme,
        RouteFailure::ResolverError => RouteFailureClass::ResolverError,
    }
}
