//! Accessor ownership checks with the real search facade and a controlled
//! backend. These fixtures do not start or substitute the embedded App Server.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_file_search_api::CloseCleanup;
use codex_file_search_api::PendingSearchStart;
use codex_file_search_api::ProviderLimits;
use codex_file_search_api::ScopeLimits;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchBudget;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_runtime::FileSearchProvider;
use codex_file_search_runtime::FileSearchScopeFactory;
use codex_file_search_runtime::RuntimePolicy;
use pretty_assertions::assert_eq;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::AppServerClient;
use crate::ClientCommand;
use crate::InProcessAppServerClient;

struct Backend {
    closing: AtomicBool,
    joined: Semaphore,
}

impl SearchBackend for Backend {
    fn begin_open(&self, _: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        panic!("accessor tests do not admit backend sessions");
    }

    fn request_shutdown(&self) {
        self.closing.store(true, Ordering::Release);
    }

    fn shutdown(&self) -> SearchCloseFuture<'_> {
        Box::pin(async move {
            self.joined.add_permits(1);
            SearchCloseOutcome {
                operation: Ok(()),
                cleanup: CloseCleanup::Joined,
            }
        })
    }
}

fn positive(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("positive fixture bound")
}

fn provider(backend: &Arc<Backend>) -> FileSearchProvider {
    FileSearchProvider::from_backend(
        backend.clone(),
        RuntimePolicy {
            provider: ProviderLimits {
                max_scopes: positive(2),
                max_sessions: positive(1),
                resources: SearchBudget {
                    max_index_entries: positive(1),
                    max_index_bytes: positive(1024),
                    max_worker_threads: positive(1),
                },
            },
            max_query_bytes: positive(64),
            max_roots_options_bytes: positive(1024),
            max_matches: positive(1),
            max_frame_retained_bytes: positive(4096),
            poll_wait: Duration::from_millis(10),
        },
    )
    .expect("fixture search provider")
}

fn client(factory: Option<FileSearchScopeFactory>) -> InProcessAppServerClient {
    let (command_tx, mut command_rx) = mpsc::channel(1);
    let (_event_tx, event_rx) = mpsc::unbounded_channel();
    let worker_handle = tokio::spawn(async move {
        let Some(ClientCommand::Shutdown { response_tx }) = command_rx.recv().await else {
            panic!("expected synthetic client shutdown");
        };
        response_tx.send(Ok(())).expect("shutdown observer");
    });
    InProcessAppServerClient {
        command_tx,
        event_rx,
        worker_handle,
        shutdown_runtime: tokio::runtime::Handle::current(),
        file_search_factory: factory,
    }
}

#[tokio::test]
async fn embedded_missing_factory_stays_an_error_through_the_client_enum() {
    let embedded = client(None);
    let error = embedded
        .file_search_scope_factory()
        .err()
        .expect("missing embedded service is an error");
    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    let client = AppServerClient::InProcess(embedded);
    let forwarded = client
        .file_search_scope_factory()
        .err()
        .expect("an embedded error must not become a remote-style None");
    assert_eq!(forwarded.kind(), error.kind());
    assert_eq!(forwarded.to_string(), error.to_string());
    client.shutdown().await.expect("synthetic worker shutdown");
}

#[tokio::test]
async fn client_and_returned_factories_cannot_keep_the_provider_alive() {
    let backend = Arc::new(Backend {
        closing: AtomicBool::new(false),
        joined: Semaphore::new(0),
    });
    let provider = provider(&backend);
    let client = AppServerClient::InProcess(client(Some(provider.scope_factory())));
    let factory = client
        .file_search_scope_factory()
        .expect("embedded capability")
        .expect("embedded provider exists");
    let scope = factory
        .new_scope(ScopeLimits {
            max_sessions: positive(1),
        })
        .expect("shared provider admits a scope");
    assert!(provider.scope_factory().owns_scope(&scope));

    drop(provider);

    assert!(backend.closing.load(Ordering::Acquire));
    assert!(client.file_search_scope_factory().is_err());
    let rejected = factory
        .new_scope(ScopeLimits {
            max_sessions: positive(1),
        })
        .err()
        .expect("returned weak factory cannot extend provider admission");
    assert_eq!(rejected.kind(), SearchErrorKind::ClosedLease);
    timeout(Duration::from_secs(2), backend.joined.acquire())
        .await
        .expect("retained shutdown runs despite factory clones")
        .expect("join observation")
        .forget();
    assert_eq!(
        timeout(Duration::from_secs(2), scope.shutdown())
            .await
            .expect("scope cleanup"),
        SearchCloseOutcome {
            operation: Ok(()),
            cleanup: CloseCleanup::Joined,
        }
    );
    client.shutdown().await.expect("synthetic worker shutdown");
}
