use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use codex_file_search_api::SearchError;
use codex_file_search_api::SearchQuery;
use futures::FutureExt;
use tokio::sync::oneshot;

use crate::POLL_METHOD;
use crate::PendingServiceReply;
use crate::PollRequest;
use crate::UPDATE_METHOD;
use crate::UpdateRequest;
use crate::UpdateResponse;
use crate::WirePoll;
use crate::WireU64;
use crate::contract::exhausted;
use crate::contract::invalid;
use crate::contract::reply;
use crate::service::Inner;
use crate::service::closed_error;
use crate::service::lock;
use crate::service::panic_error;
use crate::service_lease::Operation;
use crate::service_lease::OperationGuard;

impl Inner {
    pub(crate) fn update(self: &Arc<Self>, request: UpdateRequest) -> PendingServiceReply {
        let token = self.tasks.token();
        let admitted = (|| {
            let lease = self.lookup(request.contract_version, &request.identity)?;
            let query = SearchQuery {
                id: NonZeroU64::new(request.query_epoch.0)
                    .ok_or_else(|| invalid("file-search query epoch must be positive"))?,
                text: request.query,
            };
            let session = {
                let mut state = lock(&lease.state);
                if state.closing {
                    return Err(closed_error());
                }
                if state.updating {
                    return Err(exhausted("file-search update is already retained"));
                }
                let maximum = NonZeroUsize::new(lease.limits.max_query_utf8_bytes as usize)
                    .ok_or_else(|| invalid("file-search query limit is zero"))?;
                query.validate_after(state.query_id, maximum)?;
                let session = state
                    .session
                    .clone()
                    .ok_or_else(|| invalid("file-search lease is still preparing"))?;
                state.query_id = query.id.get();
                state.query.clone_from(&query.text);
                state.updating = true;
                session
            };
            Ok((lease, session, query))
        })();
        let (lease, session, query) = match admitted {
            Ok(value) => value,
            Err(error) => {
                return PendingServiceReply::ready(reply(
                    UPDATE_METHOD,
                    request.identity,
                    Err::<UpdateResponse, SearchError>(error),
                ));
            }
        };
        let (sender, receiver) = oneshot::channel();
        self.runtime.spawn(async move {
            let _token = token;
            let guard = OperationGuard {
                lease: Arc::clone(&lease),
                operation: Operation::Update,
            };
            let id = query.id;
            let result = match AssertUnwindSafe(async { session.update_query(query).await })
                .catch_unwind()
                .await
            {
                Ok(Ok(accepted)) if accepted.id == id => Ok(UpdateResponse {
                    accepted_query_epoch: WireU64(id.get()),
                }),
                Ok(Ok(_)) => Err(invalid(
                    "file-search backend acknowledged a different query epoch",
                )),
                Ok(Err(error)) => Err(error),
                Err(_) => Err(panic_error()),
            };
            if let Err(error) = &result
                && !(error.kind() == codex_file_search_api::SearchErrorKind::ClosedLease
                    && lock(&lease.state).closing)
            {
                lease.retain_failure(error.clone());
            }
            drop(guard);
            if result.is_err() {
                lease.request_close();
            }
            let _ = sender.send(reply(UPDATE_METHOD, request.identity, result));
        });
        PendingServiceReply {
            future: Box::pin(async move {
                receiver
                    .await
                    .unwrap_or_else(|_| Err("file-search update owner disappeared".to_owned()))
            }),
        }
    }

    pub(crate) fn poll(self: &Arc<Self>, request: PollRequest) -> PendingServiceReply {
        let token = self.tasks.token();
        let admitted = (|| {
            let lease = self.lookup(request.contract_version, &request.identity)?;
            let session = {
                let mut state = lock(&lease.state);
                if state.closing {
                    return Err(closed_error());
                }
                if state.polling {
                    return Err(exhausted("file-search poll is already retained"));
                }
                if request.wait_ms > lease.limits.max_poll_wait_ms {
                    return Err(invalid("file-search poll wait exceeds negotiated ceiling"));
                }
                let session = state
                    .session
                    .clone()
                    .ok_or_else(|| invalid("file-search lease is still preparing"))?;
                state.polling = true;
                session
            };
            Ok((lease, session))
        })();
        let (lease, session) = match admitted {
            Ok(value) => value,
            Err(error) => {
                return PendingServiceReply::ready(reply(
                    POLL_METHOD,
                    request.identity,
                    Err::<WirePoll, SearchError>(error),
                ));
            }
        };
        let (sender, receiver) = oneshot::channel();
        self.runtime.spawn(async move {
            let _token = token;
            let guard = OperationGuard {
                lease: Arc::clone(&lease),
                operation: Operation::Poll,
            };
            let result = match AssertUnwindSafe(async {
                let poll = session
                    .next_snapshot(
                        request.after_revision.0,
                        Duration::from_millis(u64::from(request.wait_ms)),
                    )
                    .await?;
                poll.validate_after(request.after_revision.0)?;
                if let codex_file_search_api::SearchPoll::Changed(frame) = &poll
                    && (frame.query.len() > lease.limits.max_query_utf8_bytes as usize
                        || frame.snapshot.as_ref().is_some_and(|snapshot| {
                            snapshot.matches.len() > lease.options.limit.get()
                                || snapshot.matches.len() > lease.limits.max_matches as usize
                        }))
                {
                    return Err(exhausted(
                        "file-search native frame exceeds negotiated ceiling",
                    ));
                }
                let wire = WirePoll::from_native(poll, &lease.roots)?;
                crate::validate_wire_poll(&wire, &lease.options, &lease.limits)?;
                if let WirePoll::Changed { frame } = &wire {
                    let state = lock(&lease.state);
                    if frame.query_epoch.0 > state.query_id
                        || (frame.query_epoch.0 == state.query_id && frame.query != state.query)
                    {
                        return Err(invalid("file-search backend returned an unadmitted query"));
                    }
                    // Older valid generations may race a newly admitted update.
                    // Never relabel them: the client delivery fence discards them.
                }
                Ok(wire)
            })
            .catch_unwind()
            .await
            {
                Ok(result) => result,
                Err(_) => Err(panic_error()),
            };
            if let Err(error) = &result
                && !(error.kind() == codex_file_search_api::SearchErrorKind::ClosedLease
                    && lock(&lease.state).closing)
            {
                lease.retain_failure(error.clone());
            }
            drop(guard);
            if result.is_err() {
                lease.request_close();
            }
            let _ = sender.send(reply(POLL_METHOD, request.identity, result));
        });
        PendingServiceReply {
            future: Box::pin(async move {
                receiver
                    .await
                    .unwrap_or_else(|_| Err("file-search poll owner disappeared".to_owned()))
            }),
        }
    }
}
