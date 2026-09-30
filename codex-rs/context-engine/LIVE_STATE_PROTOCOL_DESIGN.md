# Live context state: boundary required after replay

Status: source-backed design, **not an implemented or registered live contract**.
The first `context_replay` component is stateless reconstruction. This document
keeps that extraction usable for a later replacement of live context ownership;
it does not claim that replay alone replaces context management.

## Actual native state and callers

`core/src/context_manager/history.rs` owns annotated history, optional reviewer
transcript, retained context, reviewer mode, inherited-input retention policy,
history/reset/user/reviewer revisions, token information, reference context and a
world-state baseline. Those exact fields now exist in the staged native engine.

`core/src/state/session.rs` also owns cancellation and surrounding session state.
`replace_annotated_history` cancels `history_reset` when evidence is invalidated
and clears auto-compaction prefill. These host effects are not fields to hide in
the component. A future mutation reply must tell the host whether review
invalidation occurred, using the native `replace_compacted` result, and publish
the new revision/mirror before callers observe the transition.

`core/src/session/mod.rs::record_prepared_conversation_items` currently prepares
image/MCP metadata, assigns input order, mutates native history under the session
lock, then persists the original annotated envelopes and emits raw response
events. History may contain truncated outputs while the rollout retains original
payloads. A live adapter must return the mutated **original envelopes** as well as
the resulting history snapshot; persisting the truncated mirror would regress
replay and forks. Native MCP attribution is acknowledged only after persistence.

`replace_compacted_history` holds the settings persistence guard, mutates history,
then writes the compacted checkpoint, world state, reference context and current
settings event in that order. Context update recording persists generated model
items before world-state changes and records the reference context before
advancing its baseline. A process API must preserve these multi-step orders and
must not hold a `SessionState` mutex across IPC.

## Proposed owned state and identities

Each host session should create one component-owned context with a host-generated
owner identity and incarnation epoch. Every mutation, read snapshot and response
must carry that identity and a monotonic operation/revision value. A new process,
restored context or fork gets a new epoch; a late response from another epoch can
never update a mirror or satisfy a Guardian cache entry. This is a requirement,
not a currently exported Rust type or method.

The lossless state image must include:

- Original annotated response items, complete internal message/tool metadata,
  retained source/version identities and tool-specific truncation budgets.
- Full retained context, order allocator and omission/completeness information;
  optional reviewer transcript with its native generation and review policy.
- Native history/reset/user revisions, and reviewer evidence revision scoped by
  the host epoch. Restoring a process-local integer alone is insufficient.
- `TokenUsageInfo`, including rollout-budget units, reference context and owned
  world-state baseline; inherited-user retention policy.

No borrowed iterators, `Arc` identity, locks, runtime configuration or cancellation
tokens cross the boundary. The trusted component codec is distinct from provider
serde. In particular it restores internal executed-call provenance only for a
selected trusted component after native invariant validation. It does not grant
model responses new deserialization privileges.

An immutable host mirror should serve existing synchronous
`ConversationHistorySnapshot` readers. It must be published atomically from one
acknowledged state version, including evidence and reviewer mode. Remote methods
must not masquerade as synchronous iterators or block under the session lock.
Snapshot readers must reject a lost/revoked owner instead of silently returning
the last mirror as current authoritative state.

## Operations, ordering and cancellation

The live contract still needs an explicit schema. Its minimum semantic groups are
initialization/restoration, ordered append and retained-input events, usage
updates, reference/world-state updates, compaction replacement, rollback/reset,
prompt projection, immutable checkpoint/fork snapshot, and close. Prefer complete
native transactions over an RPC for every private helper. Prompt projection must
remain read-only with respect to the authoritative raw history, matching the
native clone-and-normalize behavior.

One context serializes accepted mutations. Each includes the expected owner and
prior revision; mismatches fail before mutation. Caller cancellation abandons the
waiter, not accepted state work. A supervised operation must finish or report an
unknown outcome if the process is lost. The current persistent transport provides
no replay, reconnection or deduplication cache; a live adapter must not retry a
possibly accepted mutation merely because its response was lost.

The host must stop admitting session work before closing the component. Accepted
mutations and persistence effects must be drained in their established order,
then the component can acknowledge close. A timeout kills the child and invalidates
the owner epoch and every derived mirror. Recovery may reconstruct a new owner
from a known durable rollout checkpoint; it must disclose loss of non-durable
accepted state and must not claim exact continuation from stale in-memory data.

The commit/persistence handshake requires another call-site audit before live
activation. Native in-memory mutation preceding rollout persistence is existing
behavior, but moving it out of process introduces a new response-loss window.
The host must define who retains accepted original envelopes through that window
and when their durable acknowledgement advances. The current pure replay service
does not encounter or solve this mutation transaction problem.

## Required acceptance

- Compare native and component state after every append, truncation, retention,
  usage update, compaction, rollback, normalization and checkpoint operation.
- Preserve incomplete/missing annotation handling, authorization exemptions,
  retained-source versions, MCP metadata and omitted evidence exactly.
- Test in-memory fork snapshots and durable legacy/paginated resumes with text
  reasoning, cell completion, executed-call provenance, output success and token
  budgets that ordinary provider serde intentionally omits.
- Race cancellation and shutdown at admission, mutation completion, response
  delivery and persistence acknowledgement. Check next-turn progress and no
  unauthorized duplicate mutation, stale mirror or cache identity reuse.
- Run two sessions, forks and process restarts with deliberately overlapping
  local revision values; owner epochs must prevent cross-session evidence reuse.
- Build and install a compatible implementation outside the checkout, remove its
  source package, and exercise the same behavior without changing the host hash.

Browser testing does not apply to this runtime-only contract. Existing web
presentation regressions remain the presentation owner's separate acceptance.
