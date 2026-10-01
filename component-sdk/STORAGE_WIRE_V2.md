# Storage state transport correction

The subsequent [P01 manual-migration contract](MIGRATION_COMPONENT_PLAN.md) adds
an optional, independently negotiated migration version 1 capability. It leaves
storage contract 2 unchanged: absent or unknown migration support disables that
operation, without disabling ordinary storage or selecting a different backend.
See that contract and the execution state for P01 validation status.

Status: implementation underway; the version-one runtime checkpoint remains
historical evidence for the specific cases it exercised. It does not establish
complete state fidelity. A native-versus-process persistence regression reproduced
the original failure before the correction; command, log and JUnit are retained
under `verification/2026-09-30/storage-wire-fidelity-before.*`.
The corrected raw durable comparison now passes in both history modes. All 27
distinct storage cases passed across the initial run and a corrected test-setup
rerun; see `verification/2026-09-30/storage-wire-v2-regressions.json`. Maintenance
lifecycle integration and independent version-two acceptance remain separate gates.

## Problem and required behavior

The initial storage adapter derives its JSON transport directly from native
storage DTOs. Several nested types use provider-facing or persisted-history
serialization. Those serializers are intentionally different from an in-memory
component boundary:

- Native response metadata writes `cell_id`, `executed_tool_calls`, and
  `tool_calls_complete`, but ordinary deserialization does not trust them. The
  process adapter consequently removes those facts *before* the native writer
  can persist them. Direct native append preserves them in the raw rollout.
- Pending metadata can retain token-budget units in memory that ordinary JSON
  omits. Process reads must return exactly the state the native store retains.
- Native path arguments can contain non-UTF-8 OS strings. Ordinary `PathBuf`
  JSON rejects them before the selected store receives the request.
- Rollout serialization can transform retained assistant delivery events for
  older persisted-history readers. That transformation belongs to the native
  persistence implementation, not the preceding process hop.

The corrected boundary transports owned native values without applying an extra
provider or persistence round trip. LocalThreadStore still applies its existing
write policy, projections, storage encoding and read validation. In particular,
the worker must **not** decode raw disk history using the trusted component
codec: it invokes the existing native read methods, then transports their typed
results. Missing or deliberately distrusted evidence stays missing.

## Shared implementation and compatibility

`codex-component-state-codec` is the shared domain codec for storage and future
context replay. It depends on domain types, not core, replay algorithms or the
process transport. Its narrowly exported serde adapters preserve native history,
annotations, token accounting, permission data and platform-native paths. The
underlying DTOs and ordinary serializers remain unchanged.

Storage uses private remote DTO serializers at its request/reply boundary. It
must audit every nested request and response type rather than fixing only append.
The selected component is trusted executable code; serialization itself does
not establish authority. Existing immutable selection, request correlation,
ownership and lifecycle checks still apply.

This changes the storage wire contract to **version 2**. Process framing remains
API version 1. The updated host must reject version-one storage packages before
invocation, rather than silently interpreting them as lossless implementations.
Other component contracts remain unchanged. Native package assembly supports an
explicit `--contract-version`; storage packages require `--contract-version 2`.

## Acceptance before claiming the correction

1. Demonstrate the original failure using the same host-recorded evidence and
   native writer in both in-process and installed-process modes. Compare raw
   persisted JSON values, avoiding the intentionally distrustful history decoder.
2. Pass shared codec round trips and verify ordinary provider decoding continues
   stripping untrusted provenance. Check pending state and non-UTF-8 path calls.
3. Run storage lifecycle, acquisition, fork, recovery and large-history tests;
   run relevant core/App Server regressions and the broad workspace gate.
4. Rebuild the host once, independently build/install the corrected native
   package, and repeat CLI and GUI acceptance with unchanged host hashes,
   including manager Ctrl+C during an active turn and cold recovery.

Old checkpoint packages and reports are retained with their original version
and scope. They must not be relabeled as version-two verification.
