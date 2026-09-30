# Selected model transport audit

The selected `model_transport:default` contract has a confirmed request-size
compatibility gap. The actual Codex CLI accepted the large native HTTP requests
below, but selecting the independently installed model component made the same
recovered input fail before the component received its inference request.

| Input fixture | Native HTTP request bytes | Native exit | Selected component exit |
| --- | ---: | ---: | ---: |
| 280 history messages, each about 16 KB | 4,552,986 | 0 | 1 |
| One user message with a 1200×1000 inline PNG | 4,844,921 | 0 | 1 |

Both selected runs reported:

```text
model transport component: plugin test.model-size method model.stream: encode component frame: component request exceeds 4194304 byte frame limit
```

The native fixture verified all 280 history markers and the exact image data URI.
Each case copied the same seeded rollout into two isolated homes before either
resume. The selected plugin completed initialization, received no oversized
request, and was reaped. No native inference fallback occurred. Package source
was removed after installation. Inference came from a local deterministic HTTP
fixture; no real model service or credentials were used. The histories were
controlled rollout fixtures, and isolated context limits were raised to prevent
compaction changing this transport-size comparison. This proves native transport
preservation, not a real model's context-window or image acceptance policy.

Evidence: [result and source hashes](model-wire-size-audit.json),
[exact commands and output paths](model-wire-size-audit-commands.json), and
[reproduction script](model-wire-size-reproduce.py). Full request bodies, PNG,
isolated homes, installed objects and stdout/stderr remain in
`/workspace/acceptance/model-wire-size-audit-20260930`.
The reproduction script must be copied into a fresh external working directory
before rerunning; it deliberately does not overwrite existing acceptance homes.

The running binaries were the preserved **storage-contract-v1 checkpoint**, not a
new storage-v2 build. Their hashes, sizes and modification times stayed unchanged:

- Codex: `3ff68d59d27a0434af9f0342b15749f9a2138c1fc074e0a79b5b3d39910de02a`.
- Manager: `4dd8ba7e78102f76349acc21ca1eb0772e9106dd1a80f9c757222bcd55742a76`.

No Cargo job or host rebuild was performed for this audit. Current source retains
the same selected-model request construction and per-call frame limit.

## Confirmed boundaries and remaining differences

1. **Aggregate request and single event size.**
   [component_model.rs](../../codex-rs/core/src/component_model.rs) serializes the
   complete normalized `ResponsesApiRequest` into one `model.stream` request.
   [process.rs](../../codex-rs/component-host/src/process.rs) bounds both physical
   request and response frames to 4 MiB; the newline and envelope consume part of
   this budget. The native
   [Responses endpoint](../../codex-rs/codex-api/src/endpoint/responses.rs)
   encodes the request directly without this component cap. Output items such as
   image-generation results, encrypted compaction content, or a large individual
   delta also encounter the response-frame cap. That output conclusion is from
   source inspection; this audit executed the two request cases above. Failure is
   explicit rather than truncation. Persistent storage chunking does not apply to
   this per-call model stream.

2. **Provider error categories.**
   Every component transport, protocol or error frame becomes terminal
   `ApiError::InvalidRequest` in `component_model::invalid`. Native
   [ApiError](../../codex-rs/codex-api/src/error.rs) includes context-window,
   content-filter, quota, retry, rate-limit, policy, flex-capacity and overload
   categories. Their retry hints and native policy effects cannot currently be
   expressed by the selected contract. This is an existing contract limitation,
   not an additional serialization-loss finding. No silent backend fallback
   should be introduced to hide it.

3. **Native provider setup is broader than normalized inference.**
   [ModelClientSession::stream](../../codex-rs/core/src/client.rs) constructs the
   selected request with `include_internal=false`. Native HTTP/WebSocket paths
   additionally perform provider/auth-dependent internal metadata, guardian,
   access-program, routing/header and connection setup. The selected branch keeps
   request contributors/interceptors and host telemetry but does not repeat all
   this setup. Backend credentials belong to the plugin; restoring native
   provider parity must use explicit contracts rather than copying host secrets.
   Provider catalogs, realtime and unary memory summaries remain other seams.

## Serde review: provider input versus host evidence

The current adapter maps every variant of
[`codex_api::ResponseEvent`](../../codex-rs/codex-api/src/common.rs): created,
item added/done, text and tool-input deltas, completed usage/end-turn, reasoning
summary/content events, server model, verification/moderation metadata, reasoning
inclusion, rate limits, model etag and safety buffering. The extension stream is
`Result<ResponseEvent, ApiError>`; this revision has no separate `ModelEvent`
enum. Unknown top-level component events fail explicitly. Malformed item events
also fail explicitly, whereas the native SSE decoder can ignore malformed provider
items. Providers must normalize into the native event contract before sending.

The review traced the
[`ResponseItem` and content types](../../codex-rs/protocol/src/models.rs): message
IDs/phase, agent messages, reasoning/encrypted content, function names/namespaces/
arguments/encrypted arguments/call IDs, local-shell action/status, custom calls
and outputs, tool-search execution/status/arguments/tools, web-search actions,
image-generation result/revised prompt, configuration and compaction items,
text/image/audio/encrypted content, and file/inline image references. The adapter
uses the native Responses serialization and provider-side item deserialization.
It does not add a second loss of ordinary typed fields at this seam. This was a
source audit, not exhaustive runtime coverage of every event variant.

Specific asymmetric serde fields need different treatment from trusted storage:

- `InternalChatMessageMetadataPassthrough.cell_id`, `executed_tool_calls` and
  `tool_calls_complete` deliberately use `skip_deserializing`: inbound model
  messages must not invent host-recorded tool execution evidence. Native SSE uses
  the same `ResponseItem` decoder. The trusted storage-v2 round-trip fix must not
  be copied into this provider boundary without preserving that distinction.
- `FunctionCallOutputPayload` serializes only its string/structured body and
  decodes `success=None`. This is the native model-provider wire contract;
  internal tool-success evidence has a separate owner. Native and selected model
  request serialization have the same behavior.
- Request-side reasoning-content suppression, optional/default fields, unknown
  item fallback and content-kind recovery follow upstream serde behavior. Provider
  and capability normalization may remove fields from the request copy; it does
  not rewrite the host's in-memory history.
- `SafetyBuffering.show_buffering_ui` is skipped by native serde. The component
  decoder already restores this explicit event field, while `retry_model` maps
  to native `faster_model` normally.
- `TokenUsage.codex_rollout_budget_units` is **deserializable but omitted by
  native serialization**. A plugin that explicitly sends the field is handled by
  the current decoder. However, a future native provider plugin cannot encode
  completed events by blindly serializing native `TokenUsage`, because it would
  drop budget units used by
  [rollout_budget.rs](../../codex-rs/core/src/rollout_budget.rs). An explicit model
  event encoder/DTO must carry this field and resolved buffering state. This is
  an encoder requirement, not evidence that the current decoder drops a supplied
  budget value.
- Independent review also identified an **output-event encoder trap**:
  `ResponseItem::Reasoning.content` ordinary serialization omits Text-only
  content and `Some([])`, while native provider decoding accepts it and the
  engine's event mapping/timing consume it. The current selected decoder accepts
  explicitly supplied content. A native plugin encoder must restore the original
  reasoning content rather than reuse provider-request serialization unchanged.
  This is distinct from intentional request-side suppression and from rejecting
  forged tool execution evidence.

## Bounded follow-up

Use a versioned model contract to carry an uncapped logical native request and
ordered logical events in bounded physical chunks. Preserve explicit provider
normalization, distinguish host evidence from provider claims, add typed provider
errors, and own serialization/cancellation through process reaping. The proposed
sequence and acceptance gates are in
[MODEL_TRANSPORT_V2_PROPOSAL.md](../../component-sdk/MODEL_TRANSPORT_V2_PROPOSAL.md).
No selected-model implementation was changed by this audit.

The SDK native packaging command was corrected to specify
`--contract-version 2` for the current thread store. Generic assembly still
defaults to 1 because other component contracts remain at version 1; component
API version 1 and attachment-store contract 1 are valid. Historical manifests and
storage-v1 acceptance records were retained unchanged.
