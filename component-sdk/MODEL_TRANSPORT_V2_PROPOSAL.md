# Proposed model transport contract v2

This is a follow-up design, not an implemented or available protocol. The current
selected model contract remains version 1. The
[runtime audit](../verification/2026-09-30/model-wire-audit.md) reproduced native
requests above 4 MiB that contract 1 rejects.

The agreed framing proposal is in
[STREAMING_PROTOCOL_PROPOSAL.md](STREAMING_PROTOCOL_PROPOSAL.md). The exact event,
backend error, trust and retry-deadline schema proposed for the separate domain
crate is in [MODEL_TRANSPORT_V2_DOMAIN.md](MODEL_TRANSPORT_V2_DOMAIN.md).

## Wire and ownership

1. Negotiate/select model contract version 2 explicitly. Keep component manifest
   API version independent from the model contract version. A version-1 plugin
   must never receive unnegotiated chunk frames. Existing v1 support can remain
   explicitly bounded while v2 receives its own adapter and SDK support.
2. Encode each request and each ordered event as a logical JSON payload with
   start/chunk/end frames, declared byte length, contiguous indexes and exact end
   counts. Retain a bounded physical frame and chunk size, and a bounded number
   of active payloads. Do not impose an arbitrary aggregate logical history cap;
   preserve existing native context policy. Spool large payloads to private
   temporary files and fail explicitly on unavailable storage.
3. Reuse audited framing/spooling primitives where practical, but add the model
   event sequence explicitly. The existing persistent session API is
   request/result based and cannot substitute for streaming events unchanged.
   Preserve native event order; reject an event after completed, missing terminal
   frames, duplicate/out-of-order payloads and malformed data. Publish completion
   only after the terminal result is validated.
4. Use bounded backpressure between component output and engine consumers.
   Cancellation must stop inference, join owned serialization/decoding work,
   remove spools and reap the child. Dropping a consumer must not detach an
   unbounded blocking parse/serialization task. Forced teardown must not report
   confirmed completion or durability.

## Data contracts

- Preserve the native normalized Responses request, including multimodal tool
  outputs and full resumed history. Do not shorten images, split semantic history
  items or omit fields just to fit a frame. Provider capability policy remains an
  explicit owner.
- Implement a dedicated native event encoder/decoder with exhaustive mapping.
  Carry `TokenUsage.codex_rollout_budget_units` and resolved
  `SafetyBuffering.show_buffering_ui` explicitly. Retain provider-side rejection
  of forged tool execution provenance; trusted storage DTOs are a different
  boundary.
- Define safe typed backend errors that preserve native context-window,
  content-filter, quota, retry/rate-limit hints, policy, capacity and overload
  behavior. Validate limits and redact private diagnostics. Do not add automatic
  fallback to a different backend.
- Document provider auth/routing/guardian/access-program differences separately.
  Achieving byte-complete normalized inference does not itself extract all
  provider management or authorize sharing host credentials.

## Delivery and acceptance gates

First add protocol fixtures and native event parity tests, then the v2 adapter and
independent SDK example. Validate a separately built/installed plugin against an
unchanged host after that host's initial v2 build. Required runtime cases include:

- The audit's aggregate history and inline PNG cases, plus logical payloads above
  16 MiB, with exact input preserved and real engine turns/resume completed.
- Large output items and small deltas in order; reasoning, tool execution,
  compaction, usage/budget, buffering, moderation and end-turn parity.
- Typed retry/context-window errors reaching existing native policies without a
  silent backend switch.
- Slow readers/writers, partial payloads, disk failures, cancellation during
  serialization and inference, orderly shutdown and direct-child reaping.
- Existing v1 behavior and explicit version incompatibility; no reinterpretation
  of historical acceptance as v2 evidence.

This work should be reviewed and gated separately from the active storage-v2 and
persistent-session lifecycle fixes.
