//! A selected process implements the neutral backend without reconnect or replay.

use std::sync::Arc;
use std::sync::Mutex;

use codex_component_host::ComponentBinding;
use codex_component_host::ComponentSessionOptions;
use codex_component_host::DeferredControl;
use codex_component_host::SessionWorkingDirectory;
use codex_file_search_api::CloseCleanup;
use codex_file_search_api::PendingSearchStart;
use codex_file_search_api::SearchBackend;
use codex_file_search_api::SearchCloseFuture;
use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_api::SearchOpen;
use codex_file_search_api::SearchStartError;
use codex_file_search_api::SearchStartFuture;
use codex_file_search_api::StartCleanup;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::FILE_SEARCH_CONTRACT_VERSION;
use crate::INITIALIZE_METHOD;
use crate::InitializeRequest;
use crate::InitializeResponse;
use crate::ProviderIdentity;
use crate::ProviderRequest;
use crate::SHUTDOWN_METHOD;
use crate::WireCloseOutcome;
use crate::payload_limits;

#[path = "process_decode.rs"]
mod decode;
#[path = "process_open.rs"]
mod open;
#[path = "process_session.rs"]
mod session;
#[path = "process_start.rs"]
mod start;
#[path = "process_state.rs"]
mod state;

use decode::decode;
use decode::encode;
use decode::malformed;
use decode::transport_error;
use state::CancelOnDrop;
use state::Provider;
use state::ProviderState;
use state::closed;
use state::lock;
use state::lost;
use state::observe;
use state::uncertain;

/// One immutable selected implementation and process incarnation. Public-handle
/// drop requests shutdown; retained tasks do not prevent that drop from running.
pub struct ProcessSearchBackend {
    inner: Arc<Provider>,
}

impl ProcessSearchBackend {
    /// Reserve one bounded start and expose cancellation before awaiting ready.
    /// The retained owner uses this lease's existing paired OPEN/RELEASE route.
    /// A selected worker's constructor responsiveness remains its own contract;
    /// old workers are not upgraded by exposing this host-side control.
    pub fn begin_open(&self, request: SearchOpen) -> Result<PendingSearchStart, SearchStartError> {
        open::begin_open(Arc::clone(&self.inner), request)
    }

    /// Immutable ceilings accepted by this selected provider. Runtime scope
    /// accounting must use these values, not assume requested limits survived
    /// negotiation unchanged.
    pub fn negotiated_limits(&self) -> &crate::ServiceLimits {
        &self.inner.limits
    }

    /// Connect using an explicit immutable child cwd and negotiate bounded
    /// search leases. Cancellation leaves startup owned until its cleanup has
    /// been observed. A failed selected implementation never falls back.
    pub async fn connect(
        binding: ComponentBinding,
        initialization: InitializeRequest,
    ) -> Result<Arc<Self>, SearchStartError> {
        initialization.validate().map_err(not_admitted)?;
        if binding.spec.kind != "file_search"
            || binding.spec.contract_version != FILE_SEARCH_CONTRACT_VERSION
        {
            return Err(not_admitted(SearchError::new(
                SearchErrorKind::UnsupportedVersion,
                "selected component does not implement file_search1",
            )));
        }
        let (cancel, cancelled) = watch::channel(false);
        let mut cancellation = CancelOnDrop(Some(cancel));
        let (sender, receiver) = oneshot::channel();
        tokio::spawn(async move {
            let result = connect_owned(binding, initialization, cancelled).await;
            // If its observer disappeared, dropping the public backend here
            // requests shutdown of the retained provider owner.
            let _ = sender.send(result);
        });
        let result = receiver.await.unwrap_or_else(|_| {
            Err(SearchStartError {
                operation: lost(),
                cleanup: StartCleanup::Unconfirmed(lost()),
            })
        });
        cancellation.0.take();
        result
    }
}

impl Drop for ProcessSearchBackend {
    fn drop(&mut self) {
        self.inner.request_shutdown();
    }
}

impl SearchBackend for ProcessSearchBackend {
    fn open(&self, request: SearchOpen) -> SearchStartFuture<'_> {
        match self.begin_open(request) {
            Ok(pending) => pending.finish(),
            Err(error) => Box::pin(async move { Err(error) }),
        }
    }

    fn request_shutdown(&self) {
        self.inner.request_shutdown();
    }

    fn shutdown(&self) -> SearchCloseFuture<'_> {
        self.inner.request_shutdown();
        Box::pin(observe(self.inner.completed.subscribe()))
    }
}

pub(super) fn not_admitted(operation: SearchError) -> SearchStartError {
    SearchStartError {
        operation,
        cleanup: StartCleanup::NotAdmitted,
    }
}

async fn connect_owned(
    binding: ComponentBinding,
    initialization: InitializeRequest,
    mut cancelled: watch::Receiver<bool>,
) -> Result<Arc<ProcessSearchBackend>, SearchStartError> {
    // Await the retained host startup rather than dropping it on cancellation:
    // its failure receipt currently has no typed joined-cleanup discriminator.
    let transport = binding
        .connect_with_options(ComponentSessionOptions {
            payload_limits: payload_limits(),
            working_directory: SessionWorkingDirectory::ExplicitAbsolute(
                initialization.base_dir.clone(),
            ),
        })
        .await
        .map_err(|error| SearchStartError {
            operation: transport_error(&error),
            cleanup: StartCleanup::Unconfirmed(lost()),
        })?;
    let initialize_params = encode(&initialization).map_err(not_admitted);
    let shutdown_params = encode(&ProviderRequest {
        contract_version: FILE_SEARCH_CONTRACT_VERSION,
        identity: initialization.identity.clone(),
    })
    .map_err(not_admitted);
    let admitted = match (initialize_params, shutdown_params) {
        (Ok(initialize_params), Ok(shutdown_params)) => tokio::select! {
            biased;
            _ = cancelled.wait_for(|cancelled| *cancelled) => Err(not_admitted(closed())),
            result = transport.start_with_cleanup(INITIALIZE_METHOD, initialize_params, SHUTDOWN_METHOD, shutdown_params) => {
                result.map_err(|error| not_admitted(transport_error(&error)))
            }
        },
        (Err(error), _) | (_, Err(error)) => Err(error),
    };
    let (reply, cleanup) = match admitted {
        Ok(handles) => handles,
        Err(mut error) => {
            if transport.close().await.is_err() {
                error.cleanup = StartCleanup::Unconfirmed(lost());
            }
            return Err(error);
        }
    };
    // Both handles are owned before this first post-admission await.
    let initialized = tokio::select! {
        biased;
        _ = cancelled.wait_for(|cancelled| *cancelled) => Err(closed()),
        result = reply.wait() => match result {
            Ok(value) => decode::<_, InitializeResponse, SearchError>(value, INITIALIZE_METHOD, &initialization.identity)
                .and_then(std::convert::identity),
            Err(error) => Err(transport_error(&error)),
        }
    }.and_then(|response| {
        response.limits.validate_negotiated(&initialization.requested_limits)?;
        let expected_platform = if cfg!(windows) { "windows_wide" } else { "unix_bytes" };
        if response.encoding != "file_search1" || response.native_path_platform != expected_platform {
            return Err(malformed());
        }
        Ok(response)
    });
    let response = match initialized {
        Ok(response) => response,
        Err(operation) => {
            let outcome = release_provider(cleanup, &initialization.identity).await;
            let cleanup = if transport.close().await.is_ok() {
                outcome.cleanup.into()
            } else {
                StartCleanup::Unconfirmed(lost())
            };
            return Err(SearchStartError { operation, cleanup });
        }
    };
    let (stopping, _) = watch::channel(false);
    let (completed, _) = watch::channel(None);
    let inner = Arc::new(Provider {
        transport,
        identity: initialization.identity,
        limits: response.limits,
        admission: tokio::sync::Mutex::new(()),
        state: Mutex::new(ProviderState::default()),
        stopping,
        completed,
    });
    let owner = Arc::clone(&inner);
    let mut guard = ProviderOwnerGuard(Some(Arc::clone(&owner)));
    tokio::spawn(async move {
        run_provider(owner, cleanup).await;
        guard.0.take();
    });
    let backend = Arc::new(ProcessSearchBackend { inner });
    if *cancelled.borrow() {
        let outcome = backend.shutdown().await;
        return Err(SearchStartError {
            operation: closed(),
            cleanup: outcome.cleanup.into(),
        });
    }
    Ok(backend)
}

async fn release_provider(
    cleanup: DeferredControl,
    identity: &ProviderIdentity,
) -> SearchCloseOutcome {
    match cleanup.release_reply().await {
        Ok(value) => {
            match decode::<_, WireCloseOutcome, SearchError>(value, SHUTDOWN_METHOD, identity) {
                Ok(Ok(outcome)) => outcome.into(),
                Ok(Err(error)) | Err(error) => uncertain(error),
            }
        }
        Err(error) => uncertain(transport_error(&error)),
    }
}

async fn run_provider(owner: Arc<Provider>, cleanup: DeferredControl) {
    let mut stopping = owner.stopping.subscribe();
    let _ = stopping.wait_for(|stopping| *stopping).await;
    owner.request_shutdown();
    // Domain shutdown can wake startup/poll handlers while paired per-lease
    // releases drain; it must not wait for the admission mutex or regular slots.
    let mut outcome = release_provider(cleanup, &owner.identity).await;
    let leases = lock(&owner.state).leases.clone();
    for lease in leases {
        let closed = observe(lease.completed.subscribe()).await;
        if outcome.operation.is_ok() {
            outcome.operation = closed.operation;
        }
        if let CloseCleanup::Unconfirmed(error) = closed.cleanup {
            outcome.cleanup = CloseCleanup::Unconfirmed(error);
        }
    }
    if let Err(error) = owner.transport.close().await {
        let error = transport_error(&error);
        if outcome.operation.is_ok() {
            outcome.operation = Err(error.clone());
        }
        outcome.cleanup = CloseCleanup::Unconfirmed(error);
    }
    if let Some(error) = lock(&owner.state).first_error.clone() {
        outcome.operation = Err(error);
    }
    owner.completed.send_replace(Some(outcome));
}

struct ProviderOwnerGuard(Option<Arc<Provider>>);

impl Drop for ProviderOwnerGuard {
    fn drop(&mut self) {
        if let Some(owner) = self.0.take() {
            owner.request_shutdown();
            owner.transport.begin_close();
            owner.completed.send_replace(Some(uncertain(lost())));
        }
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
