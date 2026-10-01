use std::sync::Arc;

use codex_file_search_api::SearchCloseOutcome;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::INITIALIZE_METHOD;
use crate::LeaseIdentity;
use crate::LeaseRequest;
use crate::OPEN_METHOD;
use crate::POLL_METHOD;
use crate::PendingServiceReply;
use crate::ProviderRequest;
use crate::RELEASE_METHOD;
use crate::SHUTDOWN_METHOD;
use crate::SearchService;
use crate::UPDATE_METHOD;
use crate::WireCloseOutcome;
use crate::contract::invalid;
use crate::contract::reply;
use crate::contract::version;
use crate::service::Inner;
use crate::service::closed_error;
use crate::service::lock;
use crate::service::observe_close;
use crate::service_lease::Lease;

/// Transport lane is fixed by the received frame, never by request parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceLane {
    Ordinary,
    Control,
}

impl From<bool> for ServiceLane {
    fn from(is_control: bool) -> Self {
        if is_control {
            Self::Control
        } else {
            Self::Ordinary
        }
    }
}

impl SearchService {
    /// Dispatch a fully decoded logical message BEFORE spawning its response
    /// handler. `lane` must be the actual transport lane, not user input.
    /// Malformed schemas use transport errors; domain failures stay typed JSON.
    pub fn admit(
        &self,
        method: &str,
        lane: ServiceLane,
        params: Value,
    ) -> Result<PendingServiceReply, String> {
        let control = matches!(method, RELEASE_METHOD | SHUTDOWN_METHOD);
        if lane != ServiceLane::from(control) {
            return Err("file-search method used the wrong transport lane".to_owned());
        }
        match method {
            INITIALIZE_METHOD => Ok(self.inner.initialize(decode(params)?)),
            OPEN_METHOD => Ok(self.inner.open(decode(params)?)),
            UPDATE_METHOD => Ok(self.inner.update(decode(params)?)),
            POLL_METHOD => Ok(self.inner.poll(decode(params)?)),
            RELEASE_METHOD => Ok(self.inner.release(decode(params)?)),
            SHUTDOWN_METHOD => Ok(self.inner.shutdown_reply(decode(params)?)),
            _ => Err("unknown file-search method".to_owned()),
        }
    }
}

fn decode<T: DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|_| "malformed file-search request".to_owned())
}

impl Inner {
    pub(crate) fn lookup(
        &self,
        contract_version: u32,
        identity: &LeaseIdentity,
    ) -> Result<Arc<Lease>, SearchError> {
        version(contract_version)?;
        identity.validate()?;
        let state = lock(&self.state);
        if state.identity.as_ref().map(|value| &value.provider_id) != Some(&identity.provider_id) {
            return Err(invalid("file-search provider identity does not match"));
        }
        if state.closing {
            return Err(closed_error());
        }
        if let Some(lease) = state.leases.get(&identity.session_epoch.0)
            && lease.identity == *identity
        {
            return Ok(Arc::clone(lease));
        }
        Err(missing(identity.session_epoch.0, state.last_epoch))
    }

    fn release(self: &Arc<Self>, request: LeaseRequest) -> PendingServiceReply {
        let identity = request.identity;
        let result = (|| {
            version(request.contract_version)?;
            identity.validate()?;
            let state = lock(&self.state);
            if state.identity.as_ref().map(|value| &value.provider_id)
                != Some(&identity.provider_id)
            {
                return Err(invalid("file-search provider identity does not match"));
            }
            if let Some(lease) = state.leases.get(&identity.session_epoch.0)
                && lease.identity == identity
            {
                return Ok(Release::Active(Arc::clone(lease)));
            }
            if let Some((_, receipt)) = state.retired.iter().find(|(key, _)| *key == identity) {
                return Ok(Release::Retired(receipt.clone()));
            }
            Err(missing(identity.session_epoch.0, state.last_epoch))
        })();
        match result {
            Ok(Release::Active(lease)) => {
                lease.request_close();
                PendingServiceReply {
                    future: Box::pin(async move {
                        reply(
                            RELEASE_METHOD,
                            identity,
                            Ok::<_, SearchError>(WireCloseOutcome::from(lease.outcome().await)),
                        )
                    }),
                }
            }
            Ok(Release::Retired(outcome)) => PendingServiceReply::ready(reply(
                RELEASE_METHOD,
                identity,
                Ok::<_, SearchError>(WireCloseOutcome::from(outcome)),
            )),
            Err(error) => PendingServiceReply::ready(reply(
                RELEASE_METHOD,
                identity,
                Err::<WireCloseOutcome, _>(error),
            )),
        }
    }

    fn shutdown_reply(self: &Arc<Self>, request: ProviderRequest) -> PendingServiceReply {
        let result = (|| {
            version(request.contract_version)?;
            request.identity.validate()?;
            if lock(&self.state).identity.as_ref() != Some(&request.identity) {
                return Err(invalid("file-search provider identity does not match"));
            }
            Ok(())
        })();
        if let Err(error) = result {
            return PendingServiceReply::ready(reply(
                SHUTDOWN_METHOD,
                request.identity,
                Err::<WireCloseOutcome, _>(error),
            ));
        }
        self.request_shutdown();
        let receiver = self.closed.subscribe();
        // This observer is deliberately not a tracked operation: shutdown owns
        // and waits the tracker; tracking this waiter would deadlock its owner.
        PendingServiceReply {
            future: Box::pin(async move {
                reply(
                    SHUTDOWN_METHOD,
                    request.identity,
                    Ok::<_, SearchError>(WireCloseOutcome::from(observe_close(receiver).await)),
                )
            }),
        }
    }
}

enum Release {
    Active(Arc<Lease>),
    Retired(SearchCloseOutcome),
}

fn missing(epoch: u64, last_epoch: u64) -> SearchError {
    if epoch <= last_epoch {
        SearchError::new(
            SearchErrorKind::ClosedLease,
            "file-search lease is closed or its bounded receipt was retired",
        )
    } else {
        SearchError::new(
            SearchErrorKind::UnknownLease,
            "file-search lease was not admitted",
        )
    }
}
