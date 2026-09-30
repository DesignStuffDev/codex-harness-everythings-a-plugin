# Native context engine — staged extraction

This package is **unlinked and unbuilt**. It does not change the active harness.
The package lifts the actual native history implementation; it is not a wrapper
around `codex-core` or a replacement that only appends messages to a vector.

The source baseline is OpenAI Codex
`d42056091aded7feb1d88ac7e83972108b2aa478`. The repository's Apache-2.0 `LICENSE`,
`NOTICE`, and `UPSTREAM_PROVENANCE.md` apply. `SOURCE_MAP.json` records extraction
inputs and method fingerprints; `SEMANTICS_PROVENANCE.md` records classification
sources, including this fork's already-existing component-context recognition.

## Boundary

| Owned here | Native behavior retained |
| --- | --- |
| `ContextManager` | Shared immutable history snapshots, retained sources and input order, reviewer transcript/policy, history/reset/user/reviewer revisions, token usage, reference context and world-state baseline |
| Recording and normalization | Per-output history budgets, full original rollout annotation, call/output pairing, orphan handling, modality filtering |
| Review and rollback | Guardian evidence capture and migration, compatibility checkpoints, original authorization provenance, user-turn rollback and mixed context trimming |
| Token estimates | Native text/image/audio/serialization estimates and caches, reasoning adjustments and model-reported usage |
| Semantics and legacy compaction | Native contextual/authorization classification, event parsing, retained excerpts, summary selection and truncation |
| `ReplayHistory` implementation | Full native history semantics for the pure `codex-context-replay` reducer, with no core callback |

`ReviewPolicy` contains already-resolved review and inherited-message decisions.
The host still resolves session source and feature policy, composes model-specific
`BaseInstructions`, prepares images and metadata, and renders extension world
state. The engine receives their owned results. Its baseline accessor/setter lets
that renderer preserve the native diff/checkpoint order without importing runtime
contributors or mutable host configuration into this package.

The optional `snapshot-api` feature implements the existing
`ConversationHistorySnapshot` interface for the future core alias. Standalone
replay builds omit this feature and therefore do not import the extension API's
tool/runtime dependency closure. The default package has no `codex-core`, model
transport, session, or process dependency.

## Replay versus live state

`codex-context-replay-native-plugin` is the first staged process implementation.
Each `context_replay.reconstruct` call creates a fresh manager and returns the full
reconstruction and companion metadata. A cancelled waiter cannot commit a result
to a departed host session. The transport supervises accepted work; graceful
shutdown drains it and the host's deadline bounds forced process termination.

This does **not** move the running session's authoritative `ContextManager` into a
plugin yet. The complete owner, revision, mutation, mirror and durability design
is in `LIVE_STATE_PROTOCOL_DESIGN.md`. In particular, the native reviewer revision
counter is process-local: any future process-owned live state must pair it with a
host-owned epoch so a restarted plugin cannot reuse a cached Guardian identity.

## Required activation and evidence

1. Finish and review the shared trusted-state codec in `component-state-codec`. It
   must preserve in-memory state that ordinary model/rollout JSON omits. Link only
   its explicit native executed-tool-call restoration export; leave provider serde
   unchanged. Owner/epoch checks belong outside the codec.
2. Register staged crates and workspace dependencies, add matching Bazel targets,
   and refresh the Bazel lock. No active manifest or core alias changed in this
   stage. Add `context_replay` v1 to manifest validation and sensitive-state stderr
   handling when the process contract is activated.
3. Run the copied semantic/token tests, trusted codec round-trips, and the staged
   `core/src/context_manager/component_engine_parity_tests.rs` and
   `core/src/session/context_engine_replay_parity_tests.rs` differentials **before**
   replacing native aliases. Differential assertions cover complete history,
   evidence, companion metadata, normalization and token results, including both
   reviewer policies and legacy/paginated replay.
4. Wire the selected process at the existing asynchronous replay seam, before
   acquiring `SessionState` for committing the returned reconstruction. Absence
   keeps the native path; selected-provider errors propagate without fallback.
   Validate session ownership at commit and close the component during teardown.
5. Separately export/build/install the native worker outside the checkout and prove
   real resume and in-memory fork behavior with unchanged host binary hashes.
   Exercise large chunked histories, exact internal metadata, cancellation,
   process loss, shutdown, invalid data, deselection and concurrent session
   isolation. Run native compaction/resume/fork and Guardian authorization checks.

Source comparison has been performed, but no Rust build, differential test,
process test, or standalone acceptance test has run for these staged packages.
Formatting, focused lint/test commands and required broader regression checks
remain part of the integration owner's coordinated build gate.
