# Context and history component: next extraction plan

Status: **source audit and proposed contract only**. No history replacement, new
component kind, or tests described below have been implemented by this plan. The
existing `context` component contributes bounded contextual messages; it does not
replace `ContextManager`. This document audits the current working tree descended
from the revision recorded in [UPSTREAM_PROVENANCE.md](UPSTREAM_PROVENANCE.md).

The recommended next step is a native history-engine crate with explicit owned
inputs, followed by an independently packaged implementation using persistent
component sessions. Start with the pure algorithms to establish equivalence, but
do not count that milestone as replacement of the live history service.

## Verified current ownership and interfaces

| Source | Existing responsibility and constraint |
| --- | --- |
| [`core/src/context_manager/history.rs`](codex-rs/core/src/context_manager/history.rs) | Concrete, crate-private `ContextManager`; no backend trait or factory. Stores annotated model history, review history, retained evidence, revisions, token information, reference settings, and world-state baseline. `Clone` shares immutable `Arc` payloads until mutation. |
| [`core/src/state/session.rs`](codex-rs/core/src/state/session.rs) | `SessionState.history` owns that concrete manager. `replace_annotated_history` also cancels `history_reset` when evidence is invalidated and clears auto-compaction prefill. Token ledger, rate limits, window IDs, reasoning-effort pin, and previous-turn settings live alongside history, not inside it. |
| [`core/src/context_manager/normalize.rs`](codex-rs/core/src/context_manager/normalize.rs) | Repairs call/output pairs, removes orphan outputs, and removes unsupported media. Missing-output IDs use the fixed UUID namespace `0x90d38d3e6a5b4d52bfe22f1e634bfac4`; changing it alters model inputs and prompt caches. |
| [`core/src/context_manager/updates.rs`](codex-rs/core/src/context_manager/updates.rs) | Merges adjacent compatible contextual fragments, preserves separate-message requests, role, and content-kind annotations. This is rendering, not ownership of settings or history. |
| [`core/src/context_manager/history_user_authorization.rs`](codex-rs/core/src/context_manager/history_user_authorization.rs) | Captures and restores retained user/assistant evidence and provenance. Contextual messages and user authorization have different meanings. Rollback and legacy checkpoint migration must preserve that distinction. |
| [`core/src/session/mod.rs`](codex-rs/core/src/session/mod.rs) | `record_conversation_items` prepares/stamps items, assigns acceptance order and provenance, updates live history, persists full originals, then emits raw items. `replace_compacted_history`, context updates, usage recording, and snapshot access coordinate several owners. |
| [`core/src/session/rollout_reconstruction.rs`](codex-rs/core/src/session/rollout_reconstruction.rs) | `reconstruct_history_from_rollout` reconstructs history and resume metadata together. Despite its current async `Session` method signature, its body has no `.await` and does not read `self`; its dependencies are explicit rollout input plus fields of `TurnContext`. This is a strong pure extraction candidate. |
| [`core/src/session/guardian_checkpoint.rs`](codex-rs/core/src/session/guardian_checkpoint.rs) | `guardian_fork_history` takes one state-lock snapshot covering history/evidence, window IDs, latest usage record, world state, and reference context. It creates an in-memory replay bundle, not a separate durable store. |
| [`core/src/context/world_state/mod.rs`](codex-rs/core/src/context/world_state/mod.rs) | `WorldState` owns heterogeneous renderers. `WorldStateSnapshot` is a serializable map with merge-patch operations. Renderers can reconcile old model messages when no exact baseline exists. A map alone cannot reproduce all current rendering behavior. |
| [`guardian-context/src/history.rs`](codex-rs/guardian-context/src/history.rs) | `TranscriptHistory` owns bounded independent review history and its generation. It has explicit checkpoint behavior; serializing its private deque as an invented wire ABI is unnecessary. |
| [`history`](codex-rs/history) and [`ext/extension-api`](codex-rs/ext/extension-api) | Existing `ResponseItemEnvelope`, `CodexHarnessMetadata`, `RetainedContext`, `GuardianHistoryCheckpoint`, `CompactedItem`, and `ConversationHistorySnapshot` are reusable contracts. The borrowed snapshot trait is useful inside the host but is not an interprocess ABI. |

The extraction must not import `codex-core` into its implementation. Current core
dependencies that need narrowing are `TurnContext`, `ManagedFeatures`,
`GuardianContextMode`, contextual-message classification, `parse_turn_item`,
Guardian text truncation, and `WorldState` renderers. Use a resolved initialization
policy, rendered base-instruction text, typed classification/provenance, and
serializable world-state snapshots instead of passing session/configuration objects.
Move genuinely shared classification/rendering helpers into a suitable lower-level
crate; do not reimplement them independently in both engine and adapter.

## State that must remain coherent

The new engine state must represent every current `ContextManager` field:

| State | Ownership and exact behavior to preserve |
| --- | --- |
| Ordered `ResponseItemEnvelope` items | Active model window; preserve metadata, IDs, turns, content annotations, and source revisions. Live tool output can be truncated while persisted originals remain full-sized. |
| Optional `TranscriptHistory` | Independent/legacy Guardian view, including bounded retention and generation. Compaction can remove this backup only under the native promotion/completeness conditions. |
| `RetainedContext` | Accepted user inputs, verified answers, delivered assistant evidence, source completeness, acceptance order, and rollback provenance. Never infer user authorization from a plugin-added contextual fragment. |
| Review mode and inherited-message flag | Resolved from session source and managed features; checkpoint restoration can select compatibility behavior. They travel with snapshots and must not be changed by an ordinary append. |
| `history_version`, `reset_version`, `user_message_revision` | Distinct counters. Ordinary appends do not turn the history-generation counter into a general transaction counter. Compaction differs from destructive reset. Add a separate RPC revision rather than reusing these counters. |
| Guardian review-context revision | Currently process-unique through a host atomic counter, including across resumed roots. Allocate its uniqueness in the host or add an equivalent host incarnation identity; a restarted plugin counter must not revive cached evidence. |
| `Option<TokenUsageInfo>` | Local/model-window accounting. Preserve saturating arithmetic and context-window saturation. Do not substitute it for the separately persisted per-response, per-turn, and per-thread `TokenUsageRecord` ledger. |
| `Option<TurnContextItem>` | Baseline for subsequent settings diffs. Missing baseline triggers full reinjection. Mixed developer-bundle rollback can clear it. |
| `Option<WorldStateSnapshot>` | Exact comparison baseline; compaction can retain only extension metadata. Clearing it is not equivalent to an empty persisted snapshot. |

Host-owned session effects remain explicit dependencies: `history_reset` cancellation,
`AutoCompactWindow` and its IDs/prefill, `ReasoningEffortPin`, previous-turn settings,
MCP attribution/resource-origin checkpoints, media preparation, rollout durability,
event publication, and approval policy. An engine result requests a typed effect;
the host commits that effect against the same accepted history revision.

Three important current ordering facts must be preserved before improving them:

1. Live history changes before `persist_rollout_items` completes. Persistence failure
   is logged and returned as `false`; the current code does not provide an atomic
   memory-plus-rollout transaction. The extraction must not claim stronger durability.
2. Compaction takes the settings persistence lock, captures related state, replaces
   history, and persists the compaction before its world-state/reference companions.
   Later accepted settings must not overtake that checkpoint.
3. Context updates persist model-visible messages before `WorldState` and
   `TurnContext`. Replay distinguishes a baseline never set from one explicitly
   cleared; coalescing those cases changes reinjection behavior.

## Pure components that can be separated first

These are proposed boundaries, not existing plugin methods:

| Candidate | Owned inputs → outputs | State/lifecycle and acceptance |
| --- | --- | --- |
| Token estimator | `ResponseItem`/history plus rendered base-instruction text → nonnegative saturating estimate | No authoritative state. The original-image LRU is an expendable cache. Preserve media heuristics, encrypted reasoning/output handling, exclusion of transport metadata, server-reasoning inclusion, and charging locally appended items after the latest model item. |
| Prompt normalization | Annotated items plus input modalities → normalized annotated items | Operates on a snapshot, not the live window. Preserve synthetic IDs, named external outputs, pairing, unsupported-media markers, metadata, and ordering. |
| Fragment assembly | Ordered rendered fragments plus separate-message flags → annotated model messages | No settings ownership or inference of policy. Preserve role boundaries and content kinds. This alone is not history extraction. |
| Replay reducer | `RolloutItem` sequence, history mode, resolved review policy, and truncation policy → current `RolloutReconstruction`-equivalent data | No I/O required by the current reducer. Preserve newest valid checkpoint selection, reverse user-turn segmentation, incomplete-turn handling, retained evidence, world-state replay, and all resume/window metadata. |

For token estimation, remove the `TurnContext` dependency by resolving model
instructions before invocation; the existing `estimate_token_count_with_base_instructions`
already demonstrates the narrower input. For replay, retain legacy conversion of
`InterAgentCommunication` and legacy compaction-without-replacement behavior. Do not
upload or migrate old media as a side effect: existing
`Session::apply_rollout_reconstruction` performs preparation separately with an inline
store and retains the persisted originals.

A first external policy plugin can replace one of these algorithms in the actual
native path, with the native algorithm as the default. This is useful acceptance
evidence for that algorithm, not completion of the stateful manager replacement.

## Proposed minimal stateful service contract

The following names are **proposals requiring agreement before implementation**.
Suggested package split: a small history contract crate, a native history engine
crate, a process adapter, and a separately buildable native history plugin. Exact
package names and manifest kind `history:default` are not registered today.

Use one persistent engine connection per running Codex session. Session/thread IDs
and a new host incarnation identify its owner. Do not expose `Session`, locks,
Rust references, extension registries, credentials, or a general host callback.
The component's state directory may cache derived state; existing thread storage
remains the canonical durable source for restart and fork.

| Proposed method | Required input | Required result/semantics |
| --- | --- | --- |
| `history.open` | Owner identity, contract/state schema, resolved review policy, and new-session seed or canonical replay bundle | Main branch plus RPC revision and coherent initial snapshot. Explicitly reject unsupported state schema. No silent fallback if a selected plugin fails. |
| `history.apply` | Branch, expected RPC revision, owner-scoped operation ID, ordered typed command batch | Next revision, complete command results, host effects, and the newly committed snapshot/checkpoint reference. One batch is atomic within the component. Retrying an already committed operation cannot apply it twice. |
| `history.read` | Branch, exact revision, requested view | Frozen owned DTO for raw/annotated history, Guardian view, token information, or checkpoint. A stale revision must fail explicitly; never return data silently taken from a newer revision. |
| `history.project` | Exact frozen snapshot or branch revision, modalities, rendered base instructions, server-reasoning flag | Prompt items and estimates without mutating live history. This replaces the semantics of consuming a cloned manager in `for_prompt`. |
| `history.fork` | Source branch and exact revision, host-generated branch identity | Independent working branch for compaction/replay trials. Mutating it cannot alter the parent or any previously materialized snapshot. Pair creation with reserved cleanup. |
| `history.release` | Owner-scoped branch identity | Idempotent branch cleanup, including release racing creation. Never release the live main branch through a stale lease. |

Connection shutdown uses the existing persistent transport close operation. It is
not a substitute for session checkpoint/durability work. If a separate domain flush
becomes necessary, define and test its guarantee rather than inferring one from a
transport acknowledgement.

`history.apply` needs typed variants covering current operations, rather than a
generic JSON mutation escape hatch: reserve acceptance order; append live annotated
items; replay original items; record retained event; restore review/retained context;
set reference/world baseline; update/set/saturate token information; remove the oldest
paired item in a working branch; replace compacted history; destructive reset; and
drop the last N user instruction turns. Append returns provenance amendments for
the full originals that the host persists. Compaction returns whether review
invalidation is required. Replay returns history and companion hydration metadata
together, as `RolloutReconstruction` does now.

Reuse existing serde protocol/history structures where suitable. Add an explicit
state schema for fields not represented by existing durable checkpoints. Export
`TranscriptHistory` through its checkpoint contract, not private implementation
fields. Represent world-state maps as owned objects; native section renderers and
their settings inputs must either move into a pure renderer crate or be executed by
the host against the same frozen revision. Do not serialize boxed fragment objects.

The host materializes one immutable `ConversationHistorySnapshot` implementation
from each accepted DTO so existing synchronous, borrowed consumers retain snapshot
semantics without performing hidden RPC. This cache is a projection, not a second
native engine. Native compaction trials currently mutate `clone_history()` results;
they need explicit working branches or owned pure transformations. Replacing
`clone_history()` with a shared live remote handle would be incorrect.

## Admission, cancellation, restart, and trust

- Introduce an ordered history coordinator. Do not await plugin RPC while holding
  `SessionState`'s mutex. Capture a revision under the lock, serialize admitted
  history mutations, await the component, then commit related host state under the
  lock. Recheck settings/turn ownership when necessary. Preserve the existing
  settings-persistence ordering; document a single lock/admission order to avoid
  checkpoint or cleanup deadlocks.
- Cancellation before admission performs no operation. After admission, the
  coordinator owns result handling even if the turn waiter disappears. It must
  commit or reconcile the accepted mutation before allowing dependent history work.
  Read/projection cancellation may discard its result. Abandoned working branches
  release through reserved cleanup capacity, including late creation responses.
- On transport loss, mark the engine unavailable and stop new history-dependent
  turns. No blind retry and no automatic native fallback. Recover with the same
  compatible implementation from the last acknowledged canonical state plus
  host-owned accepted operations, or restart from persisted rollout under existing
  session-recovery semantics. Fence the old owner before opening a new connection.
- RPC operation IDs/revision reconciliation are new service responsibilities.
  Connection request IDs alone are insufficient. Bound any deduplication ledger by
  explicit host acknowledgement/checkpoint progress; define outcome-unknown errors
  where reconciliation is impossible. No claim of exactly-once execution across
  process crashes is justified without that protocol.
- Keep native review/authorization classification and host-issued provenance
  validation. A component result cannot mint acceptance order, source identity,
  verified answers, stronger permissions, or developer policy. Retained evidence
  must derive from host-accepted inputs/checkpoints. Selected native replacements
  are trusted executable code; these contracts do not create an OS sandbox.
- Initial activation is on session startup/restart. Removal affects new sessions;
  an existing session pins its installed implementation until normal close.
  Compatibility must include state schema and canonical export/replay support,
  not only the integer transport version.

## Existing runtime support and remaining prerequisites

The runtime owner confirmed the current guarantees against
[PERSISTENT_PROTOCOL.md](component-sdk/PERSISTENT_PROTOCOL.md): persistent unary
calls, 32 ordinary slots, 32 reserved cleanup slots, FIFO ordinary writes but
concurrent service handlers/out-of-order responses, and explicit drain/close. An
accepted operation survives waiter cancellation. There is no automatic retry,
reconnect, or deduplication. Forced close leaves outcomes unknown.

Physical frames are limited to 4 MiB, with 192 KiB decoded chunks and spooling of
large logical values. Ordinary logical bodies have no hard size cap and are still
fully materialized as `serde_json::Value`; spooling alone does not make full-history
RPC memory-bounded. Before production activation, define snapshot pagination or
bounded canonical export/import with a frozen revision and explicit resource limits.
Test histories exceeding one frame without truncating legitimate model history.

The [dependency broker](component-sdk/DEPENDENCY_BROKER_PROPOSAL.md) is a separately
accepted design under implementation, not an available prerequisite to assume. This
first history service can be a leaf: the host supplies resolved settings, snapshots,
rollout inputs, and native model/compaction results. It needs no credentials or
network access. A later plugin owning compaction inference would require explicitly
granted model services, cancellation propagation, and owner scope through that broker.

## Implementation and acceptance sequence

1. Extract the native pure estimator, normalizer, and replay reducer with the
   existing semantic tests. Establish owned contracts and move coupled tests/docs.
   Keep unchanged default call sites passing explicit inputs. Do not mix altered
   compaction policy with the initial extraction.
2. Extract native mutable engine state and its operations; replace concrete
   `ContextManager` ownership through an interface. Preserve cheap immutable
   snapshots, independent working copies, host effects, and startup replay. Run the
   same contract corpus against the native backend before adding a process adapter.
3. Agree service operation/revision/recovery and bounded snapshot contracts, then
   implement process adapter and independently buildable native plugin. The native
   in-process default and absent-plugin behavior remain covered.
4. Build the plugin from an exported source/package directory outside the harness
   checkout. Record host SHA-256 first, install/select using the existing manager,
   remove the build package/source access, run real native sessions, then confirm
   the host hash is unchanged. Exercise custom replacement behavior that changes
   actual history projection or policy, not an appended demo message.
5. Repeat defaults and selected-plugin regressions; test explicit deselection,
   incompatible schema, unavailable process, crash/ambiguous operation recovery,
   cancellation, removal, and restart. Report each covered boundary and remaining
   world-state/compaction ownership gaps separately.

Required existing regression anchors include:

- [`context_manager/history_tests.rs`](codex-rs/core/src/context_manager/history_tests.rs):
  snapshot copy-on-write, contextual-user exclusion, original metadata/provenance,
  world-state deduplication, call/output normalization, exact synthetic IDs, media
  support, retained evidence, rollback of queued input, truncation, token estimates.
- [`session/rollout_reconstruction_tests.rs`](codex-rs/core/src/session/rollout_reconstruction_tests.rs):
  bounded versus full replay; completed/incomplete/inter-agent user turns; latest
  checkpoint companions; missing baselines; legacy compaction; accepted answer order.
- [`session/guardian_checkpoint_tests.rs`](codex-rs/core/src/session/guardian_checkpoint_tests.rs),
  [`session/retained_context_tests.rs`](codex-rs/core/src/session/retained_context_tests.rs),
  and [`context/world_state/world_state_tests.rs`](codex-rs/core/src/context/world_state/world_state_tests.rs):
  coherent reviewer evidence and settings snapshots across compaction/reset.
- [`tests/suite/compact.rs`](codex-rs/core/tests/suite/compact.rs),
  [`compact_resume_fork.rs`](codex-rs/core/tests/suite/compact_resume_fork.rs), and
  [`token_usage_rollout.rs`](codex-rs/core/tests/suite/token_usage_rollout.rs):
  actual model request bodies, multiple compactions, resume/fork equivalence,
  smaller-model switching, and per-response/turn/thread accounting.

New acceptance must also interrupt a turn during an admitted append, during
compaction replacement, and during snapshot/branch creation; then prove no lost or
duplicate accepted items, no surviving branch lease, correct next-turn behavior, and
unchanged approval evidence. Hold an old snapshot while mutating the current window
to prove it remains frozen. Compare native and external outputs over the same legacy
and paginated rollout corpus, including malformed checkpoints and large media.

Use `just test` with focused packages/filters under the repository workflow, then
the required broader regression gate after integration. These are planned checks,
not reported passes. This phase changes the CLI/runtime; browser testing is
inapplicable unless a presentation flow is changed.
