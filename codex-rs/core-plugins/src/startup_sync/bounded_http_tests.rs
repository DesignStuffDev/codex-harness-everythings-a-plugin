use super::super::StartupSyncHttpClient;
use super::super::SyncControl;
use super::super::SyncFailure;
use super::super::ownership::AttemptRegistry;
use super::HttpLimits;
use super::fetch_bytes;
use super::fetch_bytes_observing;
use super::fetch_text;
use crate::test_support::test_http_client_factory;
use anyhow::Context;
use http::Method;
use pretty_assertions::assert_eq;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

enum Reply {
    StalledHeaders,
    StalledBody,
    DeclaredOversize,
    Encoded {
        content_type: &'static str,
        body: Vec<u8>,
    },
    Chunked {
        status: u16,
        chunks: Vec<Vec<u8>>,
        delay: Duration,
    },
}

// Every fixture owns its native server thread. Drop releases and joins it even
// if a regression assertion unwinds; no accepted socket waits indefinitely.
struct Server {
    url: String,
    paths: Arc<Mutex<Vec<String>>>,
    ready: mpsc::Receiver<()>,
    stopped: Arc<AtomicBool>,
    handle: Option<JoinHandle<io::Result<()>>>,
}

impl Server {
    fn start(reply: Reply) -> anyhow::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let url = format!("http://{}", listener.local_addr()?);
        let paths = Arc::new(Mutex::new(Vec::new()));
        let server_paths = Arc::clone(&paths);
        let stopped = Arc::new(AtomicBool::new(false));
        let server_stopped = Arc::clone(&stopped);
        let (ready_tx, ready) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("curated-http-fixture".to_string())
            .spawn(move || {
                while !server_stopped.load(Ordering::Acquire) {
                    let (mut stream, _) = match listener.accept() {
                        Ok(accepted) => accepted,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(error) => return Err(error),
                    };
                    stream.set_read_timeout(Some(Duration::from_millis(25)))?;
                    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
                    let path = read_request(&mut stream, &server_stopped)?;
                    if let Some(path) = path {
                        server_paths
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .push(path);
                        if let Err(error) = respond(&mut stream, &reply, &server_stopped, &ready_tx)
                            && !matches!(
                                error.kind(),
                                io::ErrorKind::BrokenPipe
                                    | io::ErrorKind::ConnectionReset
                                    | io::ErrorKind::ConnectionAborted
                            )
                        {
                            return Err(error);
                        }
                    }
                }
                Ok(())
            })?;
        Ok(Self {
            url,
            paths,
            ready,
            stopped,
            handle: Some(handle),
        })
    }

    fn wait_until_stalled(&self) -> anyhow::Result<()> {
        self.ready
            .recv_timeout(Duration::from_secs(3))
            .context("HTTP fixture did not reach its response boundary")
    }

    fn paths(&self) -> Vec<String> {
        self.paths
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn shutdown(mut self) -> anyhow::Result<()> {
        self.stopped.store(true, Ordering::Release);
        let handle = self.handle.take().context("missing server handle")?;
        match handle.join() {
            Ok(result) => result.context("HTTP fixture server failed"),
            Err(_) => anyhow::bail!("HTTP fixture server panicked"),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_request(stream: &mut TcpStream, stopped: &AtomicBool) -> io::Result<Option<String>> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut request = Vec::new();
    let mut buffer = [0; 1024];
    while !stopped.load(Ordering::Acquire) && Instant::now() < deadline {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(None),
            Ok(count) => {
                request.extend_from_slice(&buffer[..count]);
                if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    return Ok(String::from_utf8_lossy(&request)
                        .split_whitespace()
                        .nth(1)
                        .map(str::to_string));
                }
                if request.len() > 16 * 1024 {
                    return Err(io::Error::other("oversized fixture request"));
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

fn respond(
    stream: &mut TcpStream,
    reply: &Reply,
    stopped: &AtomicBool,
    ready: &mpsc::Sender<()>,
) -> io::Result<()> {
    match reply {
        Reply::StalledHeaders => {}
        Reply::StalledBody => {
            stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n3\r\nabc\r\n")?;
            stream.flush()?;
        }
        Reply::DeclaredOversize => {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 65\r\nConnection: close\r\n\r\n")?;
            stream.flush()?;
        }
        Reply::Encoded { content_type, body } => {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            return stream.write_all(body);
        }
        Reply::Chunked {
            status,
            chunks,
            delay,
        } => {
            write!(
                stream,
                "HTTP/1.1 {status} Fixture\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
            )?;
            for chunk in chunks {
                write!(stream, "{:x}\r\n", chunk.len())?;
                stream.write_all(chunk)?;
                stream.write_all(b"\r\n")?;
                stream.flush()?;
                let _ = ready.send(());
                let until = Instant::now() + *delay;
                while Instant::now() < until && !stopped.load(Ordering::Acquire) {
                    std::thread::sleep(
                        Duration::from_millis(5)
                            .min(until.saturating_duration_since(Instant::now())),
                    );
                }
                if stopped.load(Ordering::Acquire) {
                    return Ok(());
                }
            }
            return stream.write_all(b"0\r\n\r\n");
        }
    }
    let _ = ready.send(());
    // Hold the accepted connection open at the selected boundary. EOF is allowed
    // after cancellation. A fixture ceiling also bounds cleanup if cancellation
    // regresses, independently of the collector's own deadline implementation.
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut byte = [0];
    while !stopped.load(Ordering::Acquire) && Instant::now() < deadline {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

struct Worker {
    control: Arc<SyncControl>,
    handle: Option<JoinHandle<Result<Vec<u8>, SyncFailure>>>,
}

impl Worker {
    fn start_text(
        home: PathBuf,
        control: Arc<SyncControl>,
        url: String,
        limits: HttpLimits,
    ) -> anyhow::Result<Self> {
        let run_control = Arc::clone(&control);
        let handle = std::thread::Builder::new()
            .name("curated-http-text-test-worker".to_string())
            .spawn(move || {
                let registry = AttemptRegistry::default();
                registry
                    .run_controlled(&home, run_control, |attempt| {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
                        let client = StartupSyncHttpClient::new(&test_http_client_factory());
                        let request = client.request(Method::GET, &url);
                        runtime.block_on(fetch_text(
                            attempt,
                            request,
                            &url,
                            "fixture text request",
                            limits,
                        ))
                    })
                    .map(String::into_bytes)
            })?;
        Ok(Self {
            control,
            handle: Some(handle),
        })
    }

    fn start(
        home: PathBuf,
        control: Arc<SyncControl>,
        url: String,
        limits: HttpLimits,
        pipeline: bool,
    ) -> anyhow::Result<Self> {
        Self::start_observing(
            home, control, url, limits, pipeline, /*observed_chunk*/ None,
        )
    }

    fn start_observing(
        home: PathBuf,
        control: Arc<SyncControl>,
        url: String,
        limits: HttpLimits,
        pipeline: bool,
        observed_chunk: Option<mpsc::Sender<()>>,
    ) -> anyhow::Result<Self> {
        let run_control = Arc::clone(&control);
        let handle = std::thread::Builder::new()
            .name("curated-http-test-worker".to_string())
            .spawn(move || {
                let registry = AttemptRegistry::default();
                let mut body = None;
                let result = registry.run_controlled(&home, run_control, |attempt| {
                    if pipeline {
                        return attempt.run(
                            /*git_binary*/ None,
                            &url,
                            &format!("{url}/export"),
                            &test_http_client_factory(),
                        );
                    }
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| SyncFailure::Ordinary(error.to_string()))?;
                    let client = StartupSyncHttpClient::new(&test_http_client_factory());
                    let request = client.request(Method::GET, &url);
                    let bytes = match observed_chunk {
                        Some(sender) => {
                            let observer = || {
                                let _ = sender.send(());
                            };
                            runtime.block_on(fetch_bytes_observing(
                                attempt,
                                request,
                                &url,
                                "fixture HTTP request",
                                limits,
                                &observer,
                            ))
                        }
                        None => runtime.block_on(fetch_bytes(
                            attempt,
                            request,
                            &url,
                            "fixture HTTP request",
                            limits,
                        )),
                    }?;
                    body = Some(bytes);
                    Ok("fetched".to_string())
                });
                result.map(|version| body.unwrap_or_else(|| version.into_bytes()))
            })?;
        Ok(Self {
            control,
            handle: Some(handle),
        })
    }

    fn join(mut self) -> anyhow::Result<Result<Vec<u8>, SyncFailure>> {
        let handle = self.handle.take().context("missing HTTP worker handle")?;
        match handle.join() {
            Ok(result) => Ok(result),
            Err(_) => anyhow::bail!("HTTP fixture worker panicked"),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.control.request_stop(Instant::now());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn limits(body_bytes: usize, diagnostic_bytes: usize, timeout: Duration) -> HttpLimits {
    HttpLimits {
        body_bytes,
        diagnostic_bytes,
        timeout,
    }
}

fn assert_lock_released(home: &Path) -> anyhow::Result<()> {
    let file = File::options()
        .write(true)
        .open(home.join(super::super::CURATED_PLUGINS_SYNC_LOCK_FILE))?;
    file.try_lock()
        .context("HTTP attempt retained its stable lock after completion")?;
    Ok(())
}

#[test]
fn cancellation_interrupts_stalled_headers_and_stalled_chunk_body() -> anyhow::Result<()> {
    for reply in [Reply::StalledHeaders, Reply::StalledBody] {
        let fixture = tempfile::tempdir()?;
        let observe_body = matches!(&reply, Reply::StalledBody);
        let server = Server::start(reply)?;
        let control = Arc::new(SyncControl::default());
        let (observed_tx, observed_rx) = mpsc::channel();
        let worker = Worker::start_observing(
            fixture.path().to_path_buf(),
            Arc::clone(&control),
            format!("{}/fixture", server.url),
            limits(
                /*body_bytes*/ 64,
                /*diagnostic_bytes*/ 16,
                Duration::from_secs(3),
            ),
            /*pipeline*/ false,
            observe_body.then_some(observed_tx),
        )?;
        server.wait_until_stalled()?;
        if observe_body {
            observed_rx
                .recv_timeout(Duration::from_secs(3))
                .context("collector did not observe the first body chunk before cancellation")?;
        }
        let stopped_at = Instant::now();
        control.request_stop(stopped_at + Duration::from_secs(1));
        let result = worker.join()?;
        assert!(
            stopped_at.elapsed() < Duration::from_secs(1),
            "stop waited for the HTTP timeout"
        );
        let error = result
            .err()
            .context("stalled HTTP request unexpectedly succeeded")?;
        assert!(matches!(
            error,
            SyncFailure::Stopped {
                publication_may_have_occurred: false
            }
        ));
        assert!(!error.permits_fallback());
        assert!(!error.retains_resources());
        assert_lock_released(fixture.path())?;
        assert_eq!(server.paths(), vec!["/fixture".to_string()]);
        server.shutdown()?;
    }
    Ok(())
}

#[test]
fn declared_oversized_content_length_is_rejected_before_body_arrives() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let server = Server::start(Reply::DeclaredOversize)?;
    let worker = Worker::start(
        fixture.path().to_path_buf(),
        Arc::new(SyncControl::default()),
        format!("{}/fixture", server.url),
        limits(
            /*body_bytes*/ 64,
            /*diagnostic_bytes*/ 16,
            Duration::from_secs(3),
        ),
        /*pipeline*/ false,
    )?;
    server.wait_until_stalled()?;
    let headers_at = Instant::now();
    let error = worker
        .join()?
        .err()
        .context("declared oversized response unexpectedly succeeded")?;
    assert!(
        headers_at.elapsed() < Duration::from_secs(1),
        "collector waited for the stalled body despite its declared size"
    );
    assert!(matches!(&error, SyncFailure::Ordinary(_)), "{error}");
    let message = error.to_string();
    assert!(
        message.contains("declared") && message.contains("64") && message.contains("limit"),
        "{message}"
    );
    assert_lock_released(fixture.path())?;
    assert_eq!(server.paths(), vec!["/fixture".to_string()]);
    server.shutdown()?;
    Ok(())
}

#[test]
fn chunked_success_and_error_bodies_are_bounded_without_content_length() -> anyhow::Result<()> {
    for status in [200, 503] {
        let fixture = tempfile::tempdir()?;
        let server = Server::start(Reply::Chunked {
            status,
            chunks: vec![
                b"prefix!!".to_vec(),
                vec![b'x'; 64],
                b"TAIL_MUST_NOT_APPEAR".to_vec(),
            ],
            delay: Duration::ZERO,
        })?;
        let worker = Worker::start(
            fixture.path().to_path_buf(),
            Arc::new(SyncControl::default()),
            format!("{}/fixture", server.url),
            limits(
                /*body_bytes*/ 16,
                /*diagnostic_bytes*/ 64,
                Duration::from_secs(3),
            ),
            /*pipeline*/ false,
        )?;
        let error = worker
            .join()?
            .err()
            .context("oversized chunked response unexpectedly succeeded")?;
        assert!(matches!(&error, SyncFailure::Ordinary(_)), "{error}");
        let message = error.to_string();
        assert!(
            message.contains("16") && message.contains("limit"),
            "observed body budget was not reported: {message}"
        );
        assert!(
            message.len() < 512,
            "body escaped its diagnostic cap: {} bytes",
            message.len()
        );
        assert!(!message.contains("TAIL_MUST_NOT_APPEAR"), "{message}");
        assert_lock_released(fixture.path())?;
        server.shutdown()?;
    }
    Ok(())
}

#[test]
fn chunked_body_at_the_exact_observed_limit_is_preserved() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let server = Server::start(Reply::Chunked {
        status: 200,
        chunks: vec![b"12345678".to_vec(), b"abcdefgh".to_vec()],
        delay: Duration::ZERO,
    })?;
    let worker = Worker::start(
        fixture.path().to_path_buf(),
        Arc::new(SyncControl::default()),
        format!("{}/fixture", server.url),
        limits(
            /*body_bytes*/ 16,
            /*diagnostic_bytes*/ 8,
            Duration::from_secs(3),
        ),
        /*pipeline*/ false,
    )?;
    let body = worker.join()??;
    assert_eq!(body, b"12345678abcdefgh");
    assert_lock_released(fixture.path())?;
    server.shutdown()?;
    Ok(())
}

#[test]
fn bounded_text_preserves_response_charset_and_bom_override() -> anyhow::Result<()> {
    for wire_body in [b"caf\xe9".to_vec(), b"\xef\xbb\xbfcaf\xc3\xa9".to_vec()] {
        let fixture = tempfile::tempdir()?;
        let server = Server::start(Reply::Encoded {
            content_type: "text/plain; charset=windows-1252",
            body: wire_body.clone(),
        })?;
        let url = format!("{}/fixture", server.url);
        let raw_worker = Worker::start(
            fixture.path().to_path_buf(),
            Arc::new(SyncControl::default()),
            url.clone(),
            limits(
                /*body_bytes*/ 64,
                /*diagnostic_bytes*/ 16,
                Duration::from_secs(3),
            ),
            /*pipeline*/ false,
        )?;
        assert_eq!(
            raw_worker.join()??,
            wire_body,
            "byte collection must preserve the encoded payload"
        );
        assert_lock_released(fixture.path())?;
        let text_worker = Worker::start_text(
            fixture.path().to_path_buf(),
            Arc::new(SyncControl::default()),
            url,
            limits(
                /*body_bytes*/ 64,
                /*diagnostic_bytes*/ 16,
                Duration::from_secs(3),
            ),
        )?;
        let decoded = String::from_utf8(text_worker.join()??)?;
        assert_eq!(
            decoded, "café",
            "charset decoding must honor a UTF-8 BOM over the legacy header"
        );
        assert_lock_released(fixture.path())?;
        assert_eq!(
            server.paths(),
            vec!["/fixture".to_string(), "/fixture".to_string()]
        );
        server.shutdown()?;
    }
    Ok(())
}

#[test]
fn error_diagnostics_retain_only_the_configured_body_prefix() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let server = Server::start(Reply::Chunked {
        status: 503,
        chunks: vec![b"FIRST123NEVER_INCLUDE_THIS_TAIL".to_vec()],
        delay: Duration::ZERO,
    })?;
    let worker = Worker::start(
        fixture.path().to_path_buf(),
        Arc::new(SyncControl::default()),
        format!("{}/fixture", server.url),
        limits(
            /*body_bytes*/ 64,
            /*diagnostic_bytes*/ 8,
            Duration::from_secs(3),
        ),
        /*pipeline*/ false,
    )?;
    let error = worker
        .join()?
        .err()
        .context("503 response unexpectedly succeeded")?;
    let message = error.to_string();
    assert!(message.contains("FIRST123"), "{message}");
    assert!(!message.contains("NEVER_INCLUDE"), "{message}");
    assert!(message.len() < 512, "{message}");
    server.shutdown()?;
    Ok(())
}

#[test]
fn continuous_chunks_do_not_reset_the_absolute_request_deadline() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let server = Server::start(Reply::Chunked {
        status: 200,
        chunks: vec![b"a".to_vec(); 40],
        delay: Duration::from_millis(50),
    })?;
    let worker = Worker::start(
        fixture.path().to_path_buf(),
        Arc::new(SyncControl::default()),
        format!("{}/fixture", server.url),
        limits(
            /*body_bytes*/ 128,
            /*diagnostic_bytes*/ 16,
            Duration::from_millis(250),
        ),
        /*pipeline*/ false,
    )?;
    server.wait_until_stalled()?;
    let started = Instant::now();
    let error = worker
        .join()?
        .err()
        .context("continuous chunks escaped the original deadline")?;
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "deadline was extended by chunks: {elapsed:?}"
    );
    assert!(matches!(&error, SyncFailure::Ordinary(_)), "{error}");
    let message = error.to_string();
    assert!(
        message.contains("timed out")
            || message.contains("timeout")
            || message.contains("deadline"),
        "{message}"
    );
    assert_lock_released(fixture.path())?;
    server.shutdown()?;
    Ok(())
}

#[test]
fn stopped_pipeline_http_request_never_admits_export_or_publication() -> anyhow::Result<()> {
    let fixture = tempfile::tempdir()?;
    let server = Server::start(Reply::StalledHeaders)?;
    let control = Arc::new(SyncControl::default());
    let worker = Worker::start(
        fixture.path().to_path_buf(),
        Arc::clone(&control),
        server.url.clone(),
        limits(
            /*body_bytes*/ 64,
            /*diagnostic_bytes*/ 16,
            Duration::from_secs(3),
        ),
        /*pipeline*/ true,
    )?;
    server.wait_until_stalled()?;
    let stopped_at = Instant::now();
    control.request_stop(stopped_at + Duration::from_secs(1));
    let error = worker
        .join()?
        .err()
        .context("stopped pipeline unexpectedly succeeded")?;
    assert!(stopped_at.elapsed() < Duration::from_secs(1));
    assert!(matches!(
        error,
        SyncFailure::Stopped {
            publication_may_have_occurred: false
        }
    ));
    assert_eq!(server.paths(), vec!["/repos/openai/plugins".to_string()]);
    assert!(!fixture.path().join(".tmp/plugins").exists());
    assert!(!fixture.path().join(".tmp/plugins.sha").exists());
    assert_lock_released(fixture.path())?;
    server.shutdown()?;
    Ok(())
}
