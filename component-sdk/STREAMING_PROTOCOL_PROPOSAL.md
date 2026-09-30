# Negotiated large one-invocation component messages

Status: **transport direction accepted; implementation held until the storage-v2
checkpoint**. No streaming wire implementation is activated. The
active host's 52-test persistent-reaping gate is preserved. The concrete failure
is recorded in [the model size audit](../verification/2026-09-30/model-wire-audit.md):
native inference accepted 4,552,986-byte aggregate-history and 4,844,921-byte
inline-PNG requests that the selected model component rejected before delivery.

## Version and compatibility

Keep component manifest `api_version:1`. Introduce `model_transport` contract 2,
which requires the streaming transport below. Continue accepting explicit model
contract 1 with its existing 4 MiB complete-message limit; do not silently promote
or downgrade a selected contract. Legacy invocations receive exactly the current
initialize/request/event/result/error/shutdown wire shape, without new handshake
fields. A version-2 package on an old host fails manifest compatibility checks.

The transport is independent of model domain DTOs. Contract 2 must not be declared
production-complete until the event/error parity work in
[MODEL_TRANSPORT_V2_PROPOSAL.md](MODEL_TRANSPORT_V2_PROPOSAL.md) is integrated and
tested. In particular, framing alone does not fix skipped serde fields or typed
provider failures. The generic framing code need not know model event variants.

Only contract 2 uses these extra initialization fields:

```json
{
  "type":"initialize",
  "api_version":1,
  "plugin_id":"example.model",
  "config":{},
  "state_dir":"/absolute/plugin/state",
  "component":{"kind":"model_transport","name":"default","contract_version":2},
  "session":{"mode":"streaming","version":1}
}
```

The plugin replies with the ordinary `ready` fields and exact matching
`component` and `session` objects. Missing, unsupported or mismatched capability
fails before any request payload is sent. A startup failure follows the existing
explicit kill-and-reap path. No automatic native fallback, reconnection or retry
occurs. Initialize and ready remain bounded single physical frames; large plugin
configuration is outside this history/event change.

The Python builder embeds validated component contract declarations in its zipapp
and passes them to `Plugin.run`. A v2-only handler cannot be invoked through an
old host's legacy request. Python packaging accepts `model_transport:2` only when
the streaming SDK is included; this does not imply support for persistent storage
contract 2. Mixed packages validate the selected component explicitly.

## Physical and logical messages

After the handshake, each logical JSON envelope uses this sequence:

```json
{"type":"message_start","id":1,"bytes":12345678}
{"type":"chunk","id":1,"index":0,"data":"standard-base64"}
{"type":"end","id":1,"chunks":1}
```

The example byte count is illustrative; implementations enforce the actual sum.
The logical body is exactly one JSON value containing the existing invocation
envelope: `request`, `event`, `result` or `error`, including inner request `id:1`.
The host's first logical envelope contains the complete request and params; it
is its only application message. Plugin event/result/error envelopes have outer
IDs 1, 2, 3, and so forth. Outer IDs identify individual ordered messages, while
the inner ID correlates every response with the single invocation. Outer IDs are
independent in the two directions and must be contiguous positive `u64` values.

| Resource | Contract |
| --- | --- |
| Physical frame including LF | At most 4 MiB |
| Decoded bytes in a chunk | 1–192 KiB |
| Encoded base64 chunk | At most 262,144 bytes |
| Active logical assemblies | One per direction |
| Queued host events | One, as in the existing stream |
| Logical body | No transport-wide size cap; `u64` byte accounting checked for overflow |
| Temporary storage | Private spool, checked IO and exact byte count; no promise of a disk quota |

Unknown/duplicate physical fields, invalid base64, skipped/reused IDs, overlapping
starts, wrong chunk IDs/indexes, size overflow, excess/short byte counts, wrong end
counts and EOF with an active envelope fail the invocation. Deserialize physical
frames directly from bounded raw bytes so duplicate JSON fields are not silently
collapsed through a `Value` normalization step. Reject an empty/non-JSON logical
body and wrong inner type/ID before invoking or publishing it.

No event/result/error can interrupt another logical envelope. A service failure
before its start frame may be sent as the next error envelope. Once a start has
been emitted, an encoding/IO failure cannot splice a replacement terminal frame
into that body; the connection fails. The plugin emits at most one terminal
result or error. Model-specific `completed` and `backend_error` ordering remains
in the model adapter: it publishes either terminal model outcome only after the
exact next empty-object result and successful child shutdown, with no intervening
additional event.

`shutdown` remains a small raw physical control frame. The host may send it only
after the complete request end frame is flushed. It is valid while the plugin is
emitting an output envelope because it travels on the opposite pipe. Shutdown or
EOF on the request pipe during an incomplete request is malformed, not success.
There is still one invocation, not the persistent session's multiple-call API.

## Shared implementation boundary

Extract the existing persistent codec's bounded raw-frame reader, private spool,
assembly accounting and chunk sender into reusable private primitives. Preserve
ordinary persistent frame schemas, control bounds, duplicate checks and tests.
The streaming reader normalizes its `message_start` to a generic body start and
uses a one-assembly limit; its exact-next-ID check is stricter than the persistent
out-of-order ID tracker. The persistent API continues to expose its existing
`Header`, `Outgoing`, `MessageReader` and writer wrappers.

Reuse the shared spool/chunk machinery, not the persistent request-ID semantics:
multiple events belong to inner request ID 1 but have distinct outer IDs. The
staged broker's raw-frame/assembler/sender extraction can use the same primitives
later without activating any broker module now. Refresh the unapplied broker
artifacts after the extraction; do not duplicate a second spool codec.

The public Rust `ComponentBinding::stream`, `start_stream`, `call`, `ComponentStartup`,
`ComponentStream::next` and shutdown handles keep their signatures. The binding's
selected contract chooses legacy or negotiated transport internally. The generic
transport returns the same event/result values to its consumer. A typed model
backend failure travels as a normal `backend_error` event, followed by `result:{}`.
The model adapter decodes its domain DTO and captures its retry deadline on event
receipt, then holds the native error until `Done({})` validates terminal framing,
child exit/reaping and owned-work completion. Generic invocation `error` envelopes
remain transport/protocol failures. No model Rust error or domain downcast crosses
the generic host. Exact DTO and error-policy mappings are specified separately in
[MODEL_TRANSPORT_V2_DOMAIN.md](MODEL_TRANSPORT_V2_DOMAIN.md).

The minimal private extraction interface is:

```rust,ignore
// Existing session_wire names remain as wrappers/reexports. The Frame and
// Header enums keep their existing persistent wire meaning and variants.
async fn read_frame_bytes<R: AsyncBufRead + Unpin>(reader: &mut R)
    -> Result<Option<Vec<u8>>>;
impl MessageAssembler {
    fn new(max_active: usize) -> Self;
    async fn push(&mut self, frame: Frame) -> Result<Option<Incoming>>;
    fn finish(&self) -> Result<()>;
    fn has_seen(&self, id: u64) -> bool;
    fn is_active(&self, id: u64) -> bool;
}
impl Sending {
    fn from_payload(outgoing: Outgoing, control: bool, payload: Option<Payload>)
        -> Result<Self>;
    async fn next_frame(&mut self) -> Result<(Frame, bool)>;
    fn mark_sent(&self); // Only after the terminal physical frame is flushed.
}

// Owned JSON work is independent from any protocol's application DTOs.
impl JsonWorkOwner {
    fn start() -> (Self, JsonWorkClient);
    async fn stop_and_join(&mut self) -> Result<()>;
}
impl JsonWorkClient {
    async fn serialize(&self, value: Value) -> Result<Payload>;
    async fn decode(&self, payload: Payload) -> Result<Value>;
}
```

`max_active` is supplied by the trusted composition wrapper, not the peer. The
streaming wrapper uses one and exact-next-ID validation; persistent sessions use
their existing 64-assembly bound. `message_start` maps internally to the shared
`Frame::ResultStart` body carrier, without exposing that internal name on the
streaming wire. Its chunk/end frames reuse shared accounting. Sending performs
the reverse mapping. Direct deserialization of each negotiated physical schema
precedes this normalization, preserving strict field validation.

`JsonWorkOwner` runs a bounded-one-job service with independently owned blocking
handles. A client waiting future may disappear, but the worker remains owned
until joined and its result/spool is delivered or discarded. Stop closes
admission, signals cancellation, drops unstarted queued work and joins running
work; it does not abort the asynchronous controller holding the blocking handle.
The process supervisor owns this service through normal and forced cleanup.
Its fallback Drop signals stop and leaves the controller able to finish while
the runtime is alive; only an awaited successful join acknowledges completion.
The existing legacy `Payload` helpers may remain compatibility wrappers initially,
but the new streaming path cannot call their detached-worker implementation.

The Python handler API also stays `handler(params, RequestContext)` with
`context.emit(value)` and `watch_shutdown()`. A new isolated streaming codec uses
`TemporaryFile` and incremental JSON encoding, then bounded base64 chunks. Full
application values still materialize for handlers. Hold an output lock across
the entire logical envelope so concurrent emitters cannot interleave frames.

## Cancellation, deadlines and shutdown

Keep the existing whole-invocation deadline and start acknowledgement only after
the request's end frame is flushed. Cancellation before that acknowledgement
follows startup cancellation; it never retries a partly delivered request.
Dropping the stream or its pending next force-cancels the invocation. A first
graceful shutdown retains the exact in-progress frame/assembly future while the
opposite writer sends one shutdown frame; it must not lose partially read bytes.
Existing launch budgets and second-interrupt behavior remain unchanged.

The existing `Payload::from_value`/`into_value` helpers spawn blocking work that
can detach when their waiter is dropped. They cannot be used as-is for this new
path. Provide a private invocation-owned JSON-work supervisor outside the
cancellable protocol future. It retains each blocking serializer/parser handle
and spool, permits at most one active JSON operation, and checks cancellation
around at most 8 KiB reads/writes. Cancelling the protocol signals that owner;
cleanup joins running work (or confirms queued work never started) before
acknowledging successful cleanup. Dropping a waiting result must not release the
worker's ownership. A bounded forced-cleanup timeout explicitly reports joins or
reaping unconfirmed; it must not turn worker panics or IO failures into success.

The process supervisor owns direct-child kill/wait and this JSON-work owner.
Normal teardown joins both before its completed acknowledgement. An abruptly
destroyed runtime remains an unconfirmed forced fallback. No successful cleanup
claim covers arbitrary descendants or provider-side writes.

The Python parser/serializer runs inside the component process, so forced process
termination owns its lifetime. Its optional shutdown reader starts only after the
complete request, retaining the same prefetched raw bytes and single-reader rule
as today. Invalid shutdown control must not lead to a successful terminal result.

## Implementation and acceptance gates

1. Agree this transport and the model-v2 domain/error contract. Extract/test shared
   primitives without activating broker or changing persistent wire behavior.
2. Add Rust streaming codec plus owned JSON-work lifecycle and actual-process
   tests. Add the Python streaming SDK and zipapp tests independently against the
   same frames. Update version validation only with working adapters.
3. Verify old v1 packages and all host streaming, cancellation, launch shutdown,
   session, lease and reaping regressions. Test exact frame bounds, invalid frames,
   slow/backpressured peers, disk failures and cancellation during parse/spool.
4. Build the new host once. Separately build/install an external v2 model plugin,
   remove its build source, and reproduce both recorded failing requests through
   real engine turns and resumed history with exact content digests. Verify input
   and output envelopes above 16 MiB, large events followed by small ordered
   deltas, complete/result discipline, and the unchanged host binary hash.
5. Run model-domain parity/error checks and native regressions. Preserve the
   earlier v1 audit and acceptance as historical evidence; do not relabel it v2.
