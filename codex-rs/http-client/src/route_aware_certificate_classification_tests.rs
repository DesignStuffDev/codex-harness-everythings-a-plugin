use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use pretty_assertions::assert_eq;

use super::ClientRouteClass;
use super::HttpClientBuilder;
use super::HttpClientFactory;
use super::Method;
use super::OutboundProxyPolicy;
use super::OutboundProxyRoute;
use super::RouteAwareClientPool;
use super::RouteAwareClientPoolError;
use super::RouteAwareRequestError;
use super::RouteFailureClass;

#[tokio::test]
async fn forced_rustls_rejects_and_classifies_real_untrusted_certificate_handshake() {
    assert_untrusted_certificate_handshake_is_classified(
        HttpClientBuilder::new().with_rustls_tls(),
    )
    .await;
}

#[test]
fn nested_io_preserves_typed_rustls_certificate_classification() {
    let error = io::Error::other(io::Error::new(
        io::ErrorKind::InvalidData,
        rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer),
    ));
    let error = RouteAwareRequestError::Route(RouteAwareClientPoolError::Resolve(error));
    assert_eq!(error.failure_class(), Some(RouteFailureClass::TlsError));
}

#[test]
fn certificate_words_without_a_tls_error_do_not_gain_tls_classification() {
    let error = io::Error::other(io::Error::new(
        io::ErrorKind::InvalidData,
        "certificate unknown issuer hostname expired revoked",
    ));
    let error = RouteAwareRequestError::Route(RouteAwareClientPoolError::Resolve(error));
    assert_eq!(
        error.failure_class(),
        Some(RouteFailureClass::ProxyResolutionUnavailable)
    );
}

// Shared by the preserved transport-default regression and the explicit Rustls
// regression. Both retain the same real server, rejection, timeout and cleanup.
pub(super) async fn assert_untrusted_certificate_handshake_is_classified(
    client_builder: HttpClientBuilder,
) {
    codex_utils_rustls_provider::ensure_rustls_crypto_provider();
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .expect("self-signed certificate should generate");
    let private_key = rustls::pki_types::PrivateKeyDer::Pkcs8(
        rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der()),
    );
    let configuration = Arc::new(
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate.cert.der().clone()], private_key)
            .expect("TLS server should be configured"),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("TLS server should bind");
    let address = listener
        .local_addr()
        .expect("TLS server should have an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("TLS server should accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("TLS handshake timeout");
        let mut connection = rustls::ServerConnection::new(configuration)
            .expect("TLS server connection should be created");
        let _ = connection.complete_io(&mut stream);
    });
    let pool = RouteAwareClientPool::with_builder(
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        ClientRouteClass::Api,
        client_builder.without_request_logging(),
    );

    let mut request = reqwest::Request::new(
        Method::GET,
        reqwest::Url::parse(&format!("https://localhost:{}/", address.port())).unwrap(),
    );
    *request.timeout_mut() = Some(Duration::from_secs(3));
    let error = pool
        .send_with_resolver(request, |_| async { Ok(OutboundProxyRoute::Direct) })
        .await
        .expect_err("self-signed server certificate must not be trusted");
    drop(std::net::TcpStream::connect(address));
    server.join().expect("TLS server should finish");

    assert_eq!(
        error.failure_class(),
        Some(RouteFailureClass::TlsError),
        "unexpected certificate error: {error:?}"
    );
}
