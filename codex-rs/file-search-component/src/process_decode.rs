//! Identity-bound conversion of process replies; diagnostics never echo payloads.

use codex_component_host::PayloadLimitExceeded;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::FILE_SEARCH_CONTRACT_VERSION;
use crate::Reply;
use crate::WireReply;

pub(super) fn transport_error(error: &anyhow::Error) -> SearchError {
    if error.downcast_ref::<PayloadLimitExceeded>().is_some() {
        SearchError::new(
            SearchErrorKind::ResourceExhausted,
            "file-search transport payload limit exceeded",
        )
    } else {
        SearchError::new(
            SearchErrorKind::TransportLost,
            "file-search component transport failed; no operation is replayed",
        )
    }
}

pub(super) fn malformed() -> SearchError {
    SearchError::new(
        SearchErrorKind::TransportLost,
        "file-search component returned an invalid or mismatched reply",
    )
}

pub(super) fn decode<I, T, E>(
    value: Value,
    method: &str,
    identity: &I,
) -> Result<Result<T, E>, SearchError>
where
    I: DeserializeOwned + PartialEq,
    T: DeserializeOwned,
    E: DeserializeOwned,
{
    let reply: WireReply<I, T, E> = serde_json::from_value(value).map_err(|_| malformed())?;
    if reply.contract_version != FILE_SEARCH_CONTRACT_VERSION
        || reply.method != method
        || &reply.identity != identity
    {
        return Err(malformed());
    }
    Ok(match reply.reply {
        Reply::Ok { result } => Ok(result),
        Reply::Error { error } => Err(error),
    })
}

pub(super) fn encode<T: serde::Serialize>(value: &T) -> Result<Value, SearchError> {
    serde_json::to_value(value).map_err(|_| {
        SearchError::new(
            SearchErrorKind::InvalidInput,
            "file-search request could not be encoded",
        )
    })
}
