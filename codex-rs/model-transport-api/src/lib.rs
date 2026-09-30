//! Domain contract for independently installed model inference implementations.
//!
//! This crate owns model values, provider-trust decoding and backend error policy
//! data. The generic process host owns framing, cancellation and process cleanup.
//! Native conversions are optional and introduce no host or engine dependency.

mod backend_error;
mod event;
mod http_metadata;
mod request;
mod usage;
mod wire_error;

#[cfg(feature = "native")]
mod native_error;
#[cfg(feature = "native")]
mod native_event;

pub use backend_error::BackendError;
pub use backend_error::ConnectionFailure;
pub use backend_error::NetworkDenial;
pub use backend_error::RetryDeadline;
pub use backend_error::RouteFailure;
pub use backend_error::TransportFailure;
pub use event::Buffering;
pub use event::ModelEvent;
pub use http_metadata::HttpErrorHeaders;
pub use request::ModelRequest;
pub use usage::Usage;
pub use wire_error::WireError;

#[cfg(feature = "native")]
pub use native_error::encode_backend_error;
#[cfg(feature = "native")]
pub use native_event::DecodedEvent;
#[cfg(feature = "native")]
pub use native_event::decode_event;
#[cfg(feature = "native")]
pub use native_event::encode_event;
#[cfg(feature = "native")]
pub use request::encode_request;

/// Model payload version; distinct from manifest API and streaming frame versions.
pub const MODEL_TRANSPORT_CONTRACT_VERSION: u32 = 2;

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
#[cfg(all(test, feature = "native"))]
#[path = "native_event_tests.rs"]
mod native_event_tests;
#[cfg(all(test, feature = "native"))]
#[path = "native_error_tests.rs"]
mod native_error_tests;
