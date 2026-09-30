# Model transport v2 domain contract proposal

Status: design only. Root accepted chunked streaming transport direction; active
host, model and SDK changes remain held until the storage-v2 acceptance checkpoint.
This specifies the domain half of
[STREAMING_PROTOCOL_PROPOSAL.md](STREAMING_PROTOCOL_PROPOSAL.md). Framing alone
does not establish model replacement parity.

## Version and crate ownership

Use manifest `api_version:1`, component
`{kind:"model_transport",name:"default",contract_version:2}` and negotiated
`session:{mode:"streaming",version:1}`. Preserve explicit model contract 1 unchanged.
Contract 2 does not downgrade after handshake, protocol or backend failure.

Add `codex-model-transport-api` as a separate domain crate. It owns request/event/
error DTOs, validation, native conversions and language-neutral JSON fixtures.
It must not depend on `codex-core` or `codex-component-host`. The process host and
`codex-component-api` must not depend on this domain crate. A model adapter composes
the generic host and the domain API. This keeps model/backend policy out of the
process supervisor and avoids a domain error downcast through `anyhow` strings.

Recommended module split: `request`, `event`, `usage`, `backend_error`,
`http_error_metadata`, and native conversion modules with separate tests. DTOs
use serde/JSON and the existing public protocol payload types where their serde
contract is symmetric. A `native` feature owns conversions to `codex-api`,
`codex-http-client` and `http` types; no optional dependency may introduce a cycle.
Native plugin authors can opt into those conversions instead of implementing a
second private model event encoder.

Public domain entry points should have this shape (names provisional):

```rust,ignore
pub const MODEL_TRANSPORT_CONTRACT_VERSION: u32 = 2;
pub struct ModelRequest { pub thread_id: String, pub request: serde_json::Value }
pub enum ModelEvent { /* exhaustive event DTOs below, plus BackendError */ }
pub enum DecodedEvent { Response(ResponseEvent), BackendError(ApiError) }

// Behind the native conversion feature:
pub fn encode_request(thread_id: ThreadId, request: ResponsesApiRequest)
    -> Result<ModelRequest, WireError>;
pub fn encode_event(event: ResponseEvent) -> Result<ModelEvent, WireError>;
pub fn encode_backend_error(error: ApiError)
    -> Result<ModelEvent, WireError>;
pub fn decode_event(event: ModelEvent)
    -> Result<DecodedEvent, WireError>;
```

`WireError` diagnostics describe a field/category without formatting its input.
Wire DTO `Debug` implementations must omit prompt, tool argument, error body,
misalignment explanation/steer and other private values. The exact clock trait
stays private with deterministic test snapshots; public helpers capture their
clock internally. The wire deadline itself is public and language independent.

## Request contract

Method remains `model.stream`; params remain
`{thread_id:string,request:ResponsesApiRequest}`. The nested request is the
existing normalized Responses JSON shape, not a separately invented prompt.
Native encoding uses the existing request serializer. Plugin implementations
receive its JSON and use their backend-specific decoder. The current native
request type is Serialize-only, so this proposal does not add an unsafe
round-trip Deserialize implementation to that upstream type.

Preserve model, instructions, input items, tools, tool choice/parallel mode,
reasoning, output format, streaming options, included fields, tier/cache metadata
and any explicit access-program fields supplied by the request owner. Capability
normalization and credential-dependent enrichment remain the host/provider
owner's decision. This contract does not authorize handing host credentials or
forged host tool evidence to a model plugin.

No new logical request/history or image size cap is introduced. The framing layer
owns bounded physical chunks and private temporary storage. JSON values still
materialize for native/Python handlers; this is not a promise of constant-memory
application decoding. Model context and individual context-fragment limits retain
their existing owners.

## Exhaustive native event mapping

Events retain the current snake_case names and field names, making their intent
recognizable to v1 authors. Contract 2's typed envelope rejects unknown event
variants and unknown top-level fields; optional fields below may be absent or
null unless otherwise stated. Nested provider/protocol payloads retain upstream
decoder compatibility rather than gaining a blanket strict-field policy.

| Event `type` | Fields and native conversion |
| --- | --- |
| `created` | `response_id: string?` → `Created` |
| `output_item_added` | `item: ResponseItem JSON` → provider-trust decoder → `OutputItemAdded` |
| `output_item_done` | `item: ResponseItem JSON` → provider-trust decoder → `OutputItemDone` |
| `output_text_delta` | `delta: string` → `OutputTextDelta` |
| `tool_call_input_delta` | `item_id: string`, `call_id: string?`, `delta: string` |
| `completed` | `response_id: string`, `token_usage: Usage?`, `usage_metadata: ResponseUsageMetadata?`, `end_turn: bool?` |
| `reasoning_summary_delta` | `delta: string`, `summary_index: i64` |
| `reasoning_summary_done` | `item_id: string`, `text: string`, `summary_index: i64` |
| `reasoning_content_delta` | `delta: string`, `content_index: i64` |
| `reasoning_summary_part_added` | `summary_index: i64` |
| `server_model` | `model: string` |
| `model_verifications` | `verifications: ModelVerification[]` |
| `turn_moderation_metadata` | `metadata: TurnModerationMetadataEvent` (including its inner `metadata` field, as v1) |
| `server_reasoning_included` | `included: bool` |
| `rate_limits` | `rate_limits: RateLimitSnapshot` |
| `models_etag` | `etag: string` |
| `safety_buffering` | `buffering: Buffering` below |
| `backend_error` | `error: BackendError` below; terminal model outcome, not a native `ResponseEvent` |

Native conversion must exhaustively match `ResponseEvent`; no wildcard success
branch. Upstream additions should cause a compilation failure in its encoder.
Its item encoder must also review each `ResponseItem` variant rather than assume
provider-request serialization is a faithful native event encoder.

`Usage` is an explicit DTO with all native fields:

```text
input_tokens: i64
cached_input_tokens: i64
cache_write_input_tokens: i64 = 0
output_tokens: i64
reasoning_output_tokens: i64
total_tokens: i64
codex_rollout_budget_units: JSON number? = null
```

Native `TokenUsage` skips budget units when serializing, so copy every field
explicitly in both conversions. Preserve native signed counters; do not add
positivity rules absent from native usage policy. Budget-unit validation remains
the existing finite/nonnegative rollout-budget policy, with JSON syntax rejecting
NaN/infinity. Preserve every valid `serde_json::Number`, including a negative
number: core applies its sign check only when a rollout budget is configured.
The DTO must not impose an unconditional negative-value rejection.
`ResponseUsageMetadata.amount` and `.metadata` remain unchanged.

`Buffering` explicitly contains `use_cases:string[]`, `reasons:string[]`,
`retry_model:string?`, and required `show_buffering_ui:bool`. Map `retry_model`
to native `faster_model`. Native serde skips the presentation flag; copy it
explicitly. Requiring it in v2 avoids silently replacing a resolved native
decision with false. Existing v1's optional/default-false behavior stays unchanged.

## Provider trust decoding

`ResponseItem` remains model/provider input at this boundary. Decode item JSON
using native provider serde, not the trusted storage-v2 codec. In particular:

- `cell_id`, `executed_tool_calls` and `tool_calls_complete` must still be ignored
  on incoming model items. A component response cannot assert the host executed
  a tool. Host-generated tool calls/results retain their normal owner.
- Model-visible tool output body follows native `FunctionCallOutputPayload`
  serialization; its internal `success` bit is not a provider-wire field.
- Keep native message phase, IDs, namespaced/encrypted tool arguments, local shell,
  custom tools, search/image calls, multimodal output, reasoning and compaction
  fields. Their provider decoder defaults and forward-compatible `Other` item
  behavior remain native. Unknown outer event types still fail explicitly.

**Native event encoding must restore `ResponseItem::Reasoning.content`
explicitly.** Its ordinary provider-request serializer omits `Some(content)`
unless at least one `ReasoningText` entry exists. However, native provider decode
accepts `Text` entries, and `core/event_mapping.rs` and `core/turn_timing.rs`
consume them. Blind serialization of a native output event therefore loses
Text-only content and converts `Some([])` into absence. A dedicated output-item
encoder can reuse ordinary serialization for other fields, then replace
`content` from the original native value, preserving `None`, `Some([])`, Text,
ReasoningText and mixed content. This restoration is for plugin-to-host event
encoding only; native request serialization retains its existing suppression.
It grants no ability to restore forged execution provenance on decode.

A native plugin encoder receiving an already-decoded native event must use these
conversions. It must not serialize an internal trusted persistence structure and
label it a provider response. Storage provenance preservation and model output
validation are different contracts.

## Terminal backend error lifecycle

Encode an intentional backend failure as a normal logical event:

```json
{"type":"event","id":1,"event":{"type":"backend_error","error":{"kind":"context_window_exceeded"}}}
```

It must be followed immediately by the normal logical result:

```json
{"type":"result","id":1,"result":{}}
```

The model adapter validates and converts the error on receipt, capturing any
retry deadline then. It retains that `ApiError` while awaiting the exact next
`Done({})` from the generic stream. `Done` is available only after protocol
shutdown, child exit/reaping and owned work completion. Only then does the model
adapter yield the backend error once and finish.

`completed` uses the same terminal discipline. An event after either terminal
model event, nonempty/nonobject result, duplicate terminal event, result without
a model terminal event, transport error, missing result, failed exit or cleanup
failure is a terminal protocol failure. Do not publish the original backend error
if terminal transport validation failed: doing so could misclassify uncertain
execution as a retryable backend condition. Cancellation discards the pending
backend/completion outcome and follows generic teardown; it must not publish a
late success or trigger a model retry after cancellation.

The generic outer `error` envelope remains an invocation/protocol failure. It
does not carry backend policy classifications. The host knows nothing about
`backend_error`. No inference request is retried by the generic transport. Native
model policy may retry the **same selected component** after its typed backend
error, within existing budgets. `responses_websocket_enabled` is already false
when a component is selected, so native HTTP fallback must remain disabled.

The Python SDK should put model helpers in a separate `codex_component_sdk.model`
module. A model-handler wrapper may catch an explicit `BackendError` value, emit
the validated `backend_error` event and return `{}` through the ordinary runtime.
Unexpected Python exceptions still become the existing generic safe invocation
error, never an inferred retryable backend error. Generic `Plugin`, framing and
non-model handlers need no model variant dispatch. The wrapper must reject
emissions after its terminal model event; it cannot acknowledge a valid outcome
after malformed shutdown control. Native plugin authors receive equivalent
domain encoder helpers, while their process runtime still owns terminal flush
and shutdown.

## Exact backend error DTO

`BackendError` is a snake_case tagged enum with `kind` as discriminator. Its
variants below are exhaustive for current native `ApiError`, including an
explicit nested transport enum. No `other` or stringify-everything fallback.

| `kind` | Additional fields | Native `ApiError` |
| --- | --- | --- |
| `api` | `status:u16`, `message:string` | `Api` |
| `stream` | `message:string` | `Stream` |
| `content_filter` | none | `ContentFilter` |
| `context_window_exceeded` | none | `ContextWindowExceeded` |
| `quota_exceeded` | none | `QuotaExceeded` |
| `usage_not_included` | none | `UsageNotIncluded` |
| `retryable` | `message:string`, `retry_at:RetryDeadline?` | `Retryable` |
| `rate_limit_exceeded` | `message:string`, `retry_at:RetryDeadline?` | `RateLimitExceeded` |
| `rate_limit` | `message:string` | `RateLimit` |
| `invalid_request` | `message:string` | `InvalidRequest` |
| `invalid_prompt` | `message:string` | `InvalidPrompt` |
| `cyber_policy` | `message:string` | `CyberPolicy` |
| `bio_policy` | `message:string` | `BioPolicy` |
| `misalignment_policy_violation` | `message:string`, `misalignment:MisalignmentErrorDetails?` | `MisalignmentPolicyViolation` |
| `flex_unavailable` | none | `FlexUnavailable` |
| `server_overloaded` | `retry_at:RetryDeadline?` | `ServerOverloaded` |
| `transport` | `transport:TransportFailure` | `Transport` |

`TransportFailure` also has a strict `kind` discriminator:

| `kind` | Additional fields | Native conversion |
| --- | --- | --- |
| `policy` | `reason: unavailable\|destination\|revoked\|unsupported_transport` | Exact `NetworkPolicyDenied` variant |
| `http` | `status:u16`, `url:string?`, `headers:HttpErrorHeaders?`, `body:string?`, `retry_at:RetryDeadline?` | `TransportError::Http` |
| `retry_limit` | none | `RetryLimit` |
| `timeout` | none | `Timeout` |
| `connection` | `connection:ConnectionFailure` below | Reconstructed `Connection(HttpError)` |
| `network` | `message:string` | `Network` |
| `build` | `message:string` | `Build` |
| `response_too_large` | `max_bytes:u64` | Checked conversion to native `usize` |

Validate status with native `http::StatusCode`, integer widths and enum variants.
Do not silently coerce booleans/floats to integer counters or error statuses.
Do not truncate an error body before native policy parsing; that would change
classification. Logical message framing already handles large bodies. Public
messages/body are intentional backend diagnostics, not opaque exception dumps.
SDK helpers must return fixed errors for unexpected exceptions and must not log
error DTO contents. Sensitive misalignment details remain live-only under the
existing native error/event persistence policy; add regression coverage.

### HTTP metadata and native policy

Preserve the native `api_bridge::map_api_error` path after reconstructing the
typed error. It distinguishes 400 invalid-image/prompt/policy, 403 policy,
429 quota/usage-limit/flex conditions, 500 internal failures and 503 overload/
slow-down responses. Provider-specific mapping, including Bedrock expired
signatures, still runs after generic conversion. Do not pre-flatten HTTP status
and body into `ApiError::Api` or `Stream`, which have different mappings.

`HttpErrorHeaders` is a map from lowercase header name to string value, limited
to metadata consumed by current native mapping. It is not an arbitrary response
header tunnel. Preserve the first native HeaderMap value, matching native `get`:

- `x-request-id`, `x-oai-request-id`, `cf-ray`,
  `x-openai-authorization-error`, `x-error-json`;
- `x-codex-active-limit`, `x-codex-promo-message`,
  `x-codex-rate-limit-reached-type`, `x-codex-credits-has-credits`,
  `x-codex-credits-unlimited`, `x-codex-credits-balance`;
- the selected active-limit family's `x-{normalized-limit}-limit-name` and
  `x-{normalized-limit}-{primary|secondary}-{used-percent|window-minutes|reset-at}`,
  using exactly the normalization/default in native `rate_limits.rs`.

The native encoder filters unused headers; the decoder rejects unexpected header
names and invalid header values. Binary/non-UTF8 values are omitted because the
native consumer already ignores them. Authorization, cookies and unrelated
headers are never forwarded by this helper. Retry-After is carried only as the
captured deadline, not reparsed from a relative raw header at the host. URLs are
optional public diagnostics: the native encoder strips user-info, query and
fragment; decoding rejects those private URL fields. URL redaction intentionally
changes diagnostics, not error classification or routing.

Preserve `headers:null` versus `headers:{}` even when filtering removes every
entry. Native `api_bridge` invokes rate-limit snapshot parsing only for a present
HeaderMap, and an empty map still produces the default Codex snapshot. Collapsing
an empty present map to None changes `UsageLimitReached` metadata.

For `x-error-json`, project only the consumed `error.code` into a minimal encoded
JSON object; omit malformed values. Do not forward unrelated fields inside that
base64 header merely because its outer name is permitted. Validate/reproject the
same field on decode so native identity-error classification remains available
without turning the metadata field into a general private diagnostic tunnel.

### Opaque connection error reconstruction

`HttpError` is `RouteAwareRequestError`, which can own a reqwest error and nested
runtime sources. It cannot be reconstructed from a display string. Mapping it
to `Stream` loses `CodexErrorDetails::ConnectionFailed` and its distinct optional
connection retry policy.

Add a narrow, generic `RemoteConnectionFailure` representation to the HTTP error
owner (not a model enum in the host). `ConnectionFailure` carries a canonical
public message, optional status, optional native `RouteFailureClass`, and named
classification booleans `is_timeout`, `is_connect`, `is_builder`, `is_body`,
`is_request`. Multiple flags may be true, as with native reqwest errors. Its
native reconstructed variant implements those existing queries, `failure_class`
and `status`; it has no local source chain or URL. `without_url` is a no-op and
`url_mut` returns None. Explicitly preserve all seven native route classes.

Native encoding takes classification from the actual methods and emits a
canonical safe connection diagnostic instead of serializing private source
objects. This deliberately preserves policy/classification rather than promising
the same Rust error identity or source chain. The resulting error remains
`ApiError::Transport(TransportError::Connection(...))`, so native connection
retry policy and status reporting continue to recognize it.

This is a separately reviewable HTTP API addition. Do not fabricate a reqwest
error, perform network IO to manufacture one, or disguise the connection as a
request-build error. If that bridge is withheld, connection-error parity remains
an explicit activation blocker rather than an undocumented lossy fallback.

## Retry deadline across IPC

Wire form is an absolute Unix deadline:

```text
RetryDeadline { unix_seconds:u64, subsec_nanos:u32 } // nanos < 1_000_000_000
```

The plugin captures native retry advice when received and converts that already
running deadline to this absolute form once when encoding. Delay/header advice
must not be restarted before writing. The host converts the absolute time to a
native `RetryAfter` once on backend-error receipt, before terminal result or reap
waits. The same object then passes through `api_bridge`, model policy and retry
sleep. Expired advice stays `Some` with zero remaining delay; it must not become
None and accidentally restore local backoff.

The HTTP utility should own checked snapshot conversion helpers for its opaque
`RetryAfter(Instant)`, rather than exposing private Instant representation in
the wire. Use paired wall/monotonic samples, checked arithmetic and native
`Instant::checked_add`; invalid/out-of-range advice is a protocol error. Do not
silently clamp valid native advice to a new arbitrary maximum delay. Record the
sampling precision/rounding choice in helper tests.

These plugins are local processes on the host machine, so wall-clock timestamps
share the same system clock. A wall-clock adjustment between plugin encoding and
host capture is an explicit limitation of the cross-process conversion; once
captured at the host, subsequent clock changes cannot restart or extend the
monotonic deadline. This contract is not a remote-clock synchronization protocol.
If exact cross-process monotonic timing across wall-clock changes is required,
negotiate a platform clock identity before activation instead of pretending
serialized Tokio `Instant` or a fresh relative delay is portable.

## Validation and test matrix

| Layer | Required checks |
| --- | --- |
| Domain event conversion | Every native `ResponseEvent` variant, optional-field absent/present, usage budget number including negative values with/without configured native budget, buffering true/false/retry model, full rate/moderation/usage metadata |
| Faithful native item encoding | Text-only reasoning in both output_item_added/done, empty/None/mixed reasoning content, compared against direct native events and their event/timing consumers; request serialization suppression unchanged |
| Provider trust | All tool/item/content variants; incoming forged cell/call records discarded; host-generated request metadata policy unchanged; native model output body semantics preserved |
| Error conversion | Every `ApiError` and nested transport variant; exhaustive native matches; status/usize/deadline overflow; unknown discriminants and fields; no raw exception/source disclosure |
| Native error policy | Compare native and reconstructed `CodexErrorDetails`, retryability, status and metadata for 400/403/429/500/503, usage limits, quota, flex, policy and Bedrock signature fixtures; headers absent versus present-empty after filtering |
| Opaque connection bridge | Classification flags/classes/status survive; source identity intentionally absent; existing connection retry branch reached; no synthetic network operation |
| Deadlines | Encode delay, pause before receive, pause before terminal result, pause before reap; remaining time continues decreasing at every stage; expired advice remains present; invalid arithmetic fails |
| Terminal model lifecycle | Backend error→result→successful reap yields typed error once; event/EOF/error/nonzero exit after backend error yields protocol failure; cancellation cannot publish pending error or completion |
| Retry integration | Existing same-provider retry budgets/hints; selected component remains selected through retries; no native HTTP fallback; content-filter guidance and context-window behavior follow existing native policy |
| Large real process | Independent zipapp/native package with deleted source; >16 MiB request and event exact preservation; audit aggregate/image cases; ordered small deltas after large item; malformed/backpressured/cancelled payloads |
| Compatibility | Existing contract1 SDK and manager shutdown checks unchanged; old host rejects contract2; negotiation mismatch fails before request; unsupported persistent contract2 not accidentally enabled in Python |

Source references: `codex-api/src/{common,error,api_bridge}.rs`,
`codex-api/src/{sse/responses,rate_limits}.rs`,
`http-client/src/{retry_after,error,route_aware_client_pool,network_policy}.rs`,
`protocol/src/{models,protocol,error,response_usage}.rs`,
`core/src/{component_model,client,responses_retry,rollout_budget}.rs` and
`model-provider/src/amazon_bedrock/error.rs` at the pinned imported revision plus
the current component changes. No Rust build or source implementation is claimed
by this design document.
