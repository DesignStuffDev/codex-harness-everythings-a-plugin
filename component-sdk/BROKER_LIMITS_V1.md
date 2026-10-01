# P03 checked broker-limit API

This slice validates explicit configuration values only. It does not change manifests, register a broker, grant authority, allocate payloads, reserve calls/connections, or create workers. The declaration catalog guard remains active. Types are additive exports of the existing custom component-api crate, not extraction of native Codex behavior.

## Checked API

`BrokerConnectionLimitsSpec` is unchecked, public-field serde input:

```rust
pub struct BrokerConnectionLimitsSpec {
    pub max_in_flight: u32,
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub max_spool_bytes: u64,
    pub max_decoded_nodes: u64,
    pub max_decoded_string_bytes: u64,
    pub max_decode_jobs: u32,
    pub max_encode_jobs: u32,
    pub max_ordinary_codec_jobs: u32,
}
pub struct BrokerConnectionLimits { /* private validated storage */ }
impl BrokerConnectionLimits {
    pub fn new(spec: BrokerConnectionLimitsSpec) -> Result<Self, BrokerLimitsError>;
    pub fn as_spec(&self) -> &BrokerConnectionLimitsSpec;
    pub fn validate_for(&self, process: &BrokerProcessLimits) -> Result<(), BrokerLimitsError>;
}
```

`BrokerProcessLimitsSpec` is likewise unchecked public-field input:

```rust
pub struct BrokerProcessLimitsSpec {
    pub max_broker_connections: u32,
    pub max_in_flight: u32,
    pub max_spool_bytes: u64,
    pub max_decoded_nodes: u64,
    pub max_decoded_string_bytes: u64,
    pub max_blocking_jobs: u32,
}
pub struct BrokerProcessLimits { /* private validated storage */ }
impl BrokerProcessLimits {
    pub fn new(spec: BrokerProcessLimitsSpec) -> Result<Self, BrokerLimitsError>;
    pub fn as_spec(&self) -> &BrokerProcessLimitsSpec;
    pub fn physical_frame_staging_bytes(&self) -> u64;
}
```

Both wrappers deserialize through checked `TryFrom<...Spec>` and serialize through `From<...Limits> for ...Spec`, preserving the flat field shape. Both raw specs reject unknown/duplicate/missing fields. No `Default` exists. Only checked wrappers may enter later runtime constructors; public spec fields remain explicitly unchecked. These types have no standalone version field; future versioned broker envelopes will contain them, subject to a separate signature review.

`BrokerLimitsError` variants are `InvalidConnectionCount`, `InvalidCallCount`, `ZeroBudget`, `InvalidCodecConcurrency`, `InsufficientProgressWorkers`, `SizeOverflow`, `SpoolCannotFitCall`, `ProcessCannotFitCall`. Canonical messages contain no supplied values. Three exported structural ceiling constants are `MAX_BROKER_CONNECTIONS = 16`, `MAX_BROKER_CALLS_PER_CONNECTION = 16`, `MAX_BROKER_BLOCKING_JOBS = 8`.

## Exactly enforced structural invariants

| Setting | Validated constraint |
| --- | --- |
| Process broker connections | 1–16 |
| Connection reverse calls | 1–16 |
| Process reverse calls | 1–(configured connections × 16), at most256 |
| Every byte/node/string budget | Positive u64; finite logical ceiling, not measured RSS |
| Each connection reverse decode/encode job ceiling | 1–connection call ceiling |
| Connection ordinary codec jobs | 1–32, aligned with current ordinary parent `CALL_SLOTS` |
| Shared running codec jobs | 3–8; enough configured slots for three future progress classes |
| Maximum call spool fit | Checked maximum request bytes + maximum response bytes must fit connection spool |
| `connection.validate_for(process)` | Same checked maximum call byte sum must fit process spool |

Overflow is rejected before the sum is used. The multiplication for process calls occurs only after validating connections≤16. Frame staging arithmetic returns configured connections × 2 × existing4MiB MAX_FRAME_BYTES, at most128MiB; it is an estimate for one maximum physical frame per direction per broker connection, not a charged allocation, complete process memory bound or proof of transport buffering behavior. No static constant-only test is added for that arithmetic.

The recommended initial composition is64 process calls with2 decode and2 encode slots per connection, within the shared8-job maximum. Those recommendations are not defaults or DTO hard maxima. The DTO intentionally permits256 configured process calls and up to16 per connection codec class; a future shared runtime must independently restrict actual running work, reserve each progress class, and charge queued work to retained records. Construction does none of those things. `validate_for` does not promise that every configured aggregate may be consumed simultaneously, nor does it validate decoded payload shape.

Connection permits must eventually be reserved before spawn/handshake and remain charged through idle/Starting/draining/quarantined states. Reverse-only spooling/node/string quotas must be enforced independently from ordinary history transport. These obligations remain later implementation; plain connection/history behavior is untouched here.

## Test and compatibility boundary

Four authored tests exercise checked JSON roundtrips, accepted structural maxima, invalid/zero/concurrency ceilings, exact call fit/overflow/process insufficiency, and raw duplicate/missing/unknown fields for both wrappers. All ten API tests passed (four budget tests and six existing declaration tests), with zero retries. Lint passed unchanged and two files were mechanically formatted afterward. See [exact evidence](../verification/2026-10-01/P03_BROKER_LIMITS_EVIDENCE.md). No new dependency or lock update is introduced.
