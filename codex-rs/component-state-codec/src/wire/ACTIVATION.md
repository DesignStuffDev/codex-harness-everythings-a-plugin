# Trusted component state codec: shared package activated

These files are a separate transport for an already-authorized engine component.
They do not change provider-facing or persisted-rollout serialization. The host
must validate the selected component, connection epoch, and request owner before
accepting reconstructed state. Never use this codec for model output, user JSON,
or unauthenticated rollout records. The codec does not establish provenance merely
by decoding fields.

`codex-context-replay` uses this shared package for `ReplayInput.items` and
`RolloutReconstruction` history, Guardian checkpoint, and reference-context fields.
Storage-owned transport DTOs use the same typed adapters without importing replay. Explicit
external enum tags preserve native variants and avoid legacy rollout transforms.
Each envelope carries its own optional metadata, so an absent annotation does not
become an empty annotation when another record has metadata.

The graph preserves reasoning content; cell IDs, attempted calls and completeness;
function-output success; all three token-usage accumulators and rollout-budget
units; nested compaction and Guardian history; actual retained assistant events
including phase; recursive event messages and host analytics; permission entry
ordering and missing-path semantics; custom source/effort variants; optional JSON
null versus absence; and float bit patterns in scores, rate limits, and reviews.

## Activation and remaining integration

1. `component-state-codec` is registered in the workspace and dependency table. Its manifest
   contains only native protocol/history/domain helpers and serde dependencies.
   Add it to storage/replay callers and bump any existing transport contract that
   changes encoding. Internal remote DTOs remain private.
2. Activated the new protocol child file
   `protocol/src/models/executed_tool_calls/trusted_component.rs` with
   `pub mod trusted_component;` in `models/executed_tool_calls.rs`.
3. Explicitly re-exported `TrustedComponentExecutedToolCall` and
   `InvalidTrustedComponentExecutedToolCall` from that child through
   `protocol/src/models.rs`. No normal `Serialize`/`Deserialize` implementation
   changes are required. The child tests reside alongside its source.
4. Focused codec/protocol checks pass as described below. Replay stays unlinked.
   Storage process fidelity/regression gates, standalone rebuild/install acceptance,
   and replay owner/differential tests remain separate integration requirements.

The private executed-call restoration helper uses an explicitly named owned DTO.
It distinguishes raw arguments from native truncation records, preserving private
`omitted_calls` and `original_name_bytes` fields. It retains result sources and
metadata, including captured JSON null. Restoration rejects sources that violate
native capture bounds or deduplication; it never silently rewrites evidence. DTOs
carrying raw call data intentionally do not implement `Debug`. Ordinary provider
deserialization still strips provenance and does not trust truncation markers.

## Validation and remaining limitations

Source review compared 78 remote DTO definitions and 68 named-struct field/type
sets with native definitions. The shared main codec has 8 tests, the event codec
has 12, path primitives have 9, and transitive path records have 8. All **37 shared
codec tests pass on Linux**. All **3 protocol trusted-helper tests pass**, including
continued stripping by ordinary provider deserialization. Replay retains 2
aggregate contract tests that remain unlinked and unrun.

Commands used the coordinated `/workspace/toolchains/component-verification-env.sh`
(debug=0, no incremental, two build jobs):

- `just test -p codex-component-state-codec`: 37 passed, 0 skipped.
- `just test -p codex-protocol trusted_component`: 3 passed, 363 filtered out.
- `just fix -p codex-component-state-codec`: completed; remaining typed-serde lint
  recommendations were resolved or narrowly explained at the required signature.
- `just clippy -p codex-component-state-codec -p codex-protocol`: passed cleanly,
  including the final image-generation/model-context public adapter exports.

Exact command notes, test/lint logs and both JUnit reports are saved in
`/workspace/codec-validation`. Targeted Rust formatting completed afterward;
repository-wide formatting/Bazel lock refresh stays with the integration owner.
No browser test applies to this serialization-only package. No core/replay runtime
activation or full native-state parity is claimed by these checks.

**Universal native-state parity is not established.** The staged path codec now
uses explicit `UnixBytes(Vec<u8>)` / `WindowsWide(Vec<u16>)` tags for native
`PathBuf` values. It preserves empty paths, relative paths, lexical spelling,
non-UTF-8 Unix bytes, and Windows unpaired surrogates without filesystem access.
A worker must run on the host's native OS convention; a foreign platform tag is
rejected, not interpreted or converted. `AbsolutePathBuf` restoration additionally
requires an absolute path, uses the public checked constructor, and compares the
resulting OS string exactly so normalization cannot silently change evidence.
This requires no new native helper or export. `PathUri` uses its existing canonical
URI serde, which already preserves non-Unicode paths and foreign path conventions.

Direct session cwd/workspace roots, thread settings, hook source paths, rollout
paths, Guardian execution cwd, and sandbox writable roots use these adapters.
The staged transitive DTOs cover nested user inputs/local media, parsed commands,
image-generation saved paths, review locations, and file changes (including path
map keys and optional move/grant paths). Path-key maps use arrays of path/value
pairs with duplicate decoded keys rejected; JSON object keys cannot represent
this native domain. Source integration and both native-platform test runs remain
required; staged tests are not runtime parity evidence. Re-audit the reachable
type graph when upstream adds variants or fields; public `UserInput` is explicitly
non-exhaustive and unknown future variants must fail rather than lose data.

All source derives from the pinned OpenAI Codex baseline and this fork's current
native domain types; repository Apache-2.0 `LICENSE` and `NOTICE` apply. No third
party harness implementation was copied.
