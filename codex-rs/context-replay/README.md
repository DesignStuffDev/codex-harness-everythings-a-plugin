# Native context replay extraction

Status: staged source, **not a workspace member, linked backend, or verified plugin**.
No runtime behavior changes until the integration owner opens the next source wave.

`reconstruct` contains the native rollout selection/reconstruction algorithm extracted
from `core/src/session/rollout_reconstruction.rs`. It accepts owned `ReplayInput`:
rollout items, history mode, truncation policy, and resolved independent-review and
inherited-user-retention decisions. Its output includes annotated history, retained
evidence, Guardian checkpoint, previous-turn settings, reference context, world state,
last started turn, and the complete compaction-window identity.

The crate does not depend on `codex-core`, configuration, session locks, I/O, or an
async runtime. Selection and RFC 7386 world-state replay preserve native algorithms.
They remain subject to differential verification after wiring. Original code is
Apache-2.0; source revision/license/NOTICE are preserved at the repository root in
`UPSTREAM_PROVENANCE.md`, `LICENSE`, and `NOTICE` as applicable.

## Native history dependency

`ReplayHistory` supplies the native item-history semantics. It is an in-process
algorithm interface, never synchronous process RPC. The staged
`codex-context-engine` implements it with the lifted native `ContextManager`,
retention/authorization algorithms, reviewer migration, output truncation,
user-turn rollback and legacy-compaction helpers. Its default dependency closure
does not include core or the extension runtime. The earlier core bridge remains a
staged comparison aid; the native worker does not depend on it.

`context-replay-component` stages the awaited host adapter and
`context-replay-native-plugin` stages the persistent native worker. Neither is
registered or wired into the active harness. `context_replay/default` version 1
uses method `context_replay.reconstruct` with owned `ReplayInput` and complete
`RolloutReconstruction`. Each request reconstructs fresh state; live context
ownership remains a later contract.

## Trusted component wire state

The request and response use the shared `codex-component-state-codec`, not ordinary model
or persisted-rollout JSON. The latter intentionally omits internal tool provenance,
cell completion, some reasoning and budget metadata, and compatibility encodings
may rewrite variants. Crossing the component boundary must add no loss to an
in-memory fork or resume snapshot. The codec and its transitive native DTO audit
are staged and must pass exact round-trip and native differential tests before
activation. Existing provider-facing serde remains unchanged.

Restoration of private executed-tool-call invariants requires the separately named
trusted helper staged under `protocol/src/models/executed_tool_calls/`; its future
export is an explicit activation change. Only selected trusted component state
may use this codec. Owner/epoch validation is a host obligation outside the codec;
external model responses must keep their existing stripping behavior.

## Next integration steps

1. Register the four staged context crates in workspace members/dependencies and
   link the narrowly scoped trusted protocol helper, under the integration owner's
   source/build gate. Add all audited codec dependencies and refresh Bazel metadata.
2. Link `context_manager/component_engine_parity_tests.rs` and
   `session/context_engine_replay_parity_tests.rs`. They compare the complete
   results against unchanged core-native implementations, including trusted wire
   round-trips, reviewer policies and legacy/paginated replay. Run them before
   aliasing production core types to the extracted engine. No test has run yet.
3. Run copied authorization, Guardian, token/media/truncation and rollback checks.
   Keep shared classification and provenance behavior single-sourced at activation.
4. At the existing async reconstruction seam, await a selected persistent replay
   service or invoke this pure native reducer. Do not perform process calls from
   `ReplayHistory` methods or while holding `SessionState` locks. The staged adapter
   must be explicitly selected and closed; its manifest kind is not yet registered.
5. Verify the extracted reducer with focused `just test` and native replay/compaction
   regressions, then separately build/install the native implementation outside the
   checkout and prove real resume/fork behavior with an unchanged host hash.

The process activation checklist is in
[`context-replay-native-plugin/ACTIVATION.md`](../context-replay-native-plugin/ACTIVATION.md).
The broader extraction audit is in
[`CONTEXT_COMPONENT_PLAN.md`](../../CONTEXT_COMPONENT_PLAN.md); native live-state
ownership and remaining transaction decisions are in
[`context-engine/LIVE_STATE_PROTOCOL_DESIGN.md`](../context-engine/LIVE_STATE_PROTOCOL_DESIGN.md).
