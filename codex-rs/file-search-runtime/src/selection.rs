use std::sync::Arc;

use codex_component_api::FILE_SEARCH_KIND;
use codex_component_host::ComponentCatalog;
use codex_file_search::NativeSearchBackend;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::StartCleanup;
use codex_file_search_component::FILE_SEARCH_CONTRACT_VERSION;
use codex_file_search_component::InitializeRequest;
use codex_file_search_component::ProcessSearchBackend;
use codex_file_search_component::ProviderIdentity;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::FileSearchProvider;
use crate::RuntimePolicy;
use crate::SelectionContext;
use crate::SelectionPolicy;
use crate::selection_guard::BackendGuard;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedImplementation {
    Native,
    Component { plugin_id: String },
}

/// One public provider owner and its immutable activation/negotiation result.
/// Only this composition shuts the provider down; its scopes have independent
/// lifetimes and cannot reselect or restart the backend.
pub struct SelectedSearchProvider {
    pub provider: FileSearchProvider,
    pub effective_policy: RuntimePolicy,
    pub implementation: SelectedImplementation,
}

/// Select `file_search/default` from this explicit home's catalog. A missing
/// selection uses native; an invalid catalog or failed selected implementation
/// returns an error without fallback. The owned initializer/drain survives an
/// abandoned observer. Drop is a cleanup request, never a joined receipt.
pub async fn select_provider(
    context: SelectionContext,
    policy: SelectionPolicy,
) -> Result<SelectedSearchProvider, SearchStartError> {
    context.validate().map_err(not_admitted)?;
    policy.validate().map_err(not_admitted)?;
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| {
        not_admitted(SearchError::new(
            SearchErrorKind::InvalidInput,
            "file-search selection requires an active Tokio runtime",
        ))
    })?;
    let (cancel, cancelled) = watch::channel(false);
    let mut guard = CancelOnDrop(Some(cancel));
    let (sender, receiver) = oneshot::channel();
    runtime.spawn(async move {
        let result = select_owned(context, policy, cancelled).await;
        // An undeliverable public provider is dropped here, which fences and
        // retains its cleanup independently of scopes or internal tasks.
        let _ = sender.send(result);
    });
    let result = receiver.await.unwrap_or_else(|_| {
        Err(SearchStartError {
            operation: lost(),
            cleanup: StartCleanup::Unconfirmed(lost()),
        })
    });
    guard.0.take();
    result
}

async fn select_owned(
    context: SelectionContext,
    policy: SelectionPolicy,
    cancelled: watch::Receiver<bool>,
) -> Result<SelectedSearchProvider, SearchStartError> {
    if *cancelled.borrow() {
        return Err(not_admitted(closed()));
    }
    let catalog = ComponentCatalog::load(&context.codex_home).map_err(|_| not_admitted(
        SearchError::new(SearchErrorKind::InvalidInput, "file-search component catalog could not be loaded; check component settings and package manifests")
    ))?;
    let selected = catalog.selected(FILE_SEARCH_KIND, "default");
    if *cancelled.borrow() {
        return Err(not_admitted(closed()));
    }
    let (backend, effective, implementation): (Arc<dyn SearchBackend>, _, _) = match selected {
        Some(binding) => {
            let implementation = SelectedImplementation::Component {
                plugin_id: binding.plugin_id.clone(),
            };
            let backend = ProcessSearchBackend::connect(
                binding,
                InitializeRequest {
                    contract_version: FILE_SEARCH_CONTRACT_VERSION,
                    identity: ProviderIdentity {
                        provider_id: uuid::Uuid::new_v4().hyphenated().to_string(),
                    },
                    base_dir: context.base_dir,
                    requested_limits: policy.process.clone(),
                },
            )
            .await?;
            let guard = BackendGuard::new(backend.clone());
            let effective = match policy.for_process(backend.negotiated_limits()) {
                Ok(effective) => effective,
                Err(operation) => return Err(failed_with_cleanup(operation, guard).await),
            };
            // Transfer between adjacent synchronous owners without an await.
            guard.disarm();
            (backend, effective, implementation)
        }
        None => {
            let effective = policy.for_native().map_err(not_admitted)?;
            let backend =
                NativeSearchBackend::new(context.base_dir, policy.native).map_err(not_admitted)?;
            (Arc::new(backend), effective, SelectedImplementation::Native)
        }
    };
    let guard = BackendGuard::new(Arc::clone(&backend));
    if *cancelled.borrow() {
        return Err(failed_with_cleanup(closed(), guard).await);
    }
    let provider = match FileSearchProvider::from_backend(backend, effective) {
        Ok(provider) => provider,
        Err(operation) => return Err(failed_with_cleanup(operation, guard).await),
    };
    guard.disarm();
    Ok(SelectedSearchProvider {
        provider,
        effective_policy: effective,
        implementation,
    })
}
async fn failed_with_cleanup(operation: SearchError, guard: BackendGuard) -> SearchStartError {
    let outcome = guard.shutdown().await;
    // Cancellation is a fence, not a reason to erase a real failure discovered
    // while draining an already-started implementation.
    let operation = if operation.kind() == SearchErrorKind::ClosedLease {
        match outcome.operation {
            Err(error) => error,
            Ok(()) => match &outcome.cleanup {
                codex_file_search_api::CloseCleanup::Unconfirmed(error) => error.clone(),
                codex_file_search_api::CloseCleanup::Joined => operation,
            },
        }
    } else {
        operation
    };
    SearchStartError {
        operation,
        cleanup: outcome.cleanup.into(),
    }
}
fn not_admitted(operation: SearchError) -> SearchStartError {
    SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    }
}
fn closed() -> SearchError {
    SearchError::new(
        SearchErrorKind::ClosedLease,
        "file-search selection observer was cancelled",
    )
}
fn lost() -> SearchError {
    SearchError::new(
        SearchErrorKind::TransportLost,
        "file-search selection owner ended before its cleanup receipt",
    )
}
struct CancelOnDrop(Option<watch::Sender<bool>>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            sender.send_replace(true);
        }
    }
}
