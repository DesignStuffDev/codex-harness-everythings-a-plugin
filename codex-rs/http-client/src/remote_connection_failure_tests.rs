use super::*;
use crate::RouteAwareClientPoolError;
use pretty_assertions::assert_eq;
use std::error::Error;
use std::io;

#[test]
fn remote_queries_preserve_independent_native_classification() {
    let classes = [
        None,
        Some(RouteFailureClass::ProxyResolutionUnavailable),
        Some(RouteFailureClass::ConnectTimeout),
        Some(RouteFailureClass::ProxyAuthenticationRequired),
        Some(RouteFailureClass::TlsError),
        Some(RouteFailureClass::InvalidProxyConfig),
        Some(RouteFailureClass::UnsupportedProxyScheme),
        Some(RouteFailureClass::ResolverError),
    ];
    for failure_class in classes {
        for status in [None, Some(StatusCode::PROXY_AUTHENTICATION_REQUIRED)] {
            for flags in 0..32 {
                let expected = RemoteConnectionFailure {
                    status,
                    failure_class,
                    is_timeout: flags & 1 != 0,
                    is_connect: flags & 2 != 0,
                    is_builder: flags & 4 != 0,
                    is_body: flags & 8 != 0,
                    is_request: flags & 16 != 0,
                };
                let mut error = RouteAwareRequestError::from(expected);
                assert_eq!(RemoteConnectionFailure::capture(&error), expected);
                assert!(error.source().is_none());
                assert!(error.url_mut().is_none());
                assert_eq!(
                    RemoteConnectionFailure::capture(&error.without_url()),
                    expected
                );
            }
        }
    }
}

#[test]
fn capture_removes_private_diagnostics_and_source_chains() {
    let native = RouteAwareRequestError::Route(RouteAwareClientPoolError::Resolve(io::Error::other(
        "proxy resolution for https://user:secret@example.com/?token=private failed",
    )));
    assert!(native.source().is_some());
    let failure = RemoteConnectionFailure::capture(&native);
    assert_eq!(
        failure,
        RemoteConnectionFailure {
            status: None,
            failure_class: Some(RouteFailureClass::ProxyResolutionUnavailable),
            is_timeout: false,
            is_connect: false,
            is_builder: false,
            is_body: false,
            is_request: false,
        }
    );
    let reconstructed = RouteAwareRequestError::from(failure);
    assert_eq!(
        reconstructed.to_string(),
        "connection failed (proxy_resolution_unavailable)"
    );
    assert!(reconstructed.source().is_none());
    assert!(!format!("{reconstructed:?}").contains("secret"));
}

#[test]
fn capture_preserves_real_http_status_and_builder_queries_without_network_io() {
    let response = reqwest::Response::from(
        http::Response::builder()
            .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
            .body("")
            .expect("valid response"),
    );
    let native = RouteAwareRequestError::from(response.error_for_status().unwrap_err());
    let failure = RemoteConnectionFailure::capture(&native);
    assert_eq!(failure.status, Some(StatusCode::PROXY_AUTHENTICATION_REQUIRED));
    assert_eq!(
        failure.failure_class,
        Some(RouteFailureClass::ProxyAuthenticationRequired)
    );
    let reconstructed = RouteAwareRequestError::from(failure);
    assert_eq!(RemoteConnectionFailure::capture(&reconstructed), failure);
    assert_eq!(
        reconstructed.to_string(),
        "connection failed (proxy_407): HTTP 407"
    );

    let error = reqwest::Client::new()
        .get("https://example.com/?token=secret")
        .header("invalid-header", "invalid\nvalue")
        .build()
        .unwrap_err();
    let failure = RemoteConnectionFailure::capture(&error.into());
    assert!(failure.is_builder);
    let mut reconstructed = RouteAwareRequestError::from(failure);
    assert_eq!(RemoteConnectionFailure::capture(&reconstructed), failure);
    assert!(reconstructed.url_mut().is_none());
    assert!(!reconstructed.to_string().contains("secret"));
}
