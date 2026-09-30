# Context extraction: next integration gates

This is a source-backed activation proposal. The context engine, replay reducer,
process adapter, native worker, and core lifecycle helper remain unlinked. No
context runtime pass, replay differential pass, or independent worker build is
claimed. The shared state codec is active: its 37 Linux tests and three protocol
helper tests passed. Evidence is in
`../../verification/2026-09-30/component-state-codec-results.json`.

## Dependency and state review

The staged dependency direction is:

```text
protocol/history -> component-state-codec -> context-replay
context-replay + native context helpers -> context-engine
context-replay + component-host -> context-replay-component
context-engine + context-replay + component-host -> native replay worker
core -> selected process adapter, or original native reducer
```

The shared codec has no replay/engine/core dependency. Replay's `contract.rs`
references only public shared adapters for rollout items, response envelopes,
Guardian checkpoints, and reference turn context. No local `wire` module remains.
The engine's `snapshot-api` feature adds the existing extension snapshot trait
only for host comparisons; the worker uses the default engine without core or
that optional feature. No synchronous native history method performs IPC.

The reducer's `ReplayHistory` implementation executes native retention,
authorization classification, normalization, rollback, legacy compaction and token
logic locally. Its owned input resolves history mode, truncation policy, independent
review policy and inherited-input retention. Output includes complete history,
retained evidence, Guardian checkpoint, last started turn, previous settings,
reference context, world-state baseline, and all context-window identities.

The worker owns fresh ContextManager state for each request and runs the complete
reducer on a blocking task while transport I/O remains independent. It has no
durable state or host dependency grant. The host owns session lifecycle, settings,
media rehydration, persistence, review invalidation and final state publication.
This boundary replaces reconstruction only. Live context transactions remain
specified separately in `LIVE_STATE_PROTOCOL_DESIGN.md`.

## Gate 0: finish the active storage and transport checkpoint

Complete storage contract-v2 native-versus-process fidelity tests, regression
checks and independent package acceptance using the shared codec. Complete the
persistent transport forced termination/reap fix and test that its completion
signal follows bounded process cleanup, including failed acquisition. The staged
replay owner retains acquisition even after caller cancellation; it cannot repair
an underlying transport completion that precedes reap.

No context activation should overlap that active source/build checkpoint. Keep
one coordinated Cargo owner and the agreed resource profile. The storage wrapper
and helper tests provide prerequisites, not replay runtime evidence.

## Gate 1: compile the native library boundary before selecting a process

Register four existing package manifests and workspace dependencies:
`context-replay`, `context-engine`, `context-replay-component`, and
`context-replay-native-plugin`. The replay manifest already depends on the shared
codec. Keep production core imports and the catalog unchanged at this step.
Refresh Cargo/Bazel metadata and verify the existing new per-crate BUILD targets.
There are no new include-str/include-bytes resources in these four packages.

Run `just test -p codex-context-replay -p codex-context-engine`. The staged source
contains four replay tests (two world-state and two aggregate-codec tests) and 64
engine test declarations; runtime discovery, compilation and results must verify
those counts. Fix extraction compile errors without simplifying native semantics.
The known ResponseItemId fixture constructors were corrected using the real
native `with_suffix` API, including worker and core differential fixtures.

## Gate 2: compare both implementations in the same host tests

Add test-only core dependencies on replay and `context-engine` with `snapshot-api`.
Declare only the two staged differential modules in
`core/src/context_manager/mod.rs` and `core/src/session/mod.rs`. Retain the original
core ContextManager/reducer and all production aliases as the oracle.

Run:

```sh
just test -p codex-core --lib -E 'test(component_engine_parity_tests) | test(context_engine_replay_parity_tests)'
```

The staged three tests compare native/extracted history, retained authorization,
review snapshots, token estimates, truncation, compaction and rollback, plus the
full reconstruction companion output across legacy/paginated corpora. The replay
comparison passes input and output through the actual shared codec. Distinct
process-unique Guardian evidence identities must remain distinct; comparing their
literal counters across two live implementations would assert the wrong property.
Broaden these corpora where differences expose missing native paths before aliasing
any production type.

## Gate 3: exercise the installed native worker independently of core selection

Register `context_replay/default` contract v1 in catalog validation. Add it to both
process transports' sensitive-state stderr suppression and reject it in the
management CLI's generic one-shot `call`. These are static interface changes;
install/select/reset/list continue to work normally.

Run `just test -p codex-context-replay-native-plugin`. The three staged process
tests install an actual compiled package, delete its original package directory,
compare native outputs for a history larger than a physical frame, recover after
invalid input, and preserve independent connections across later deselection.
Ensure Bazel runfiles expose the native binary to `cargo_bin`; add the worker to
core's `extra_binaries` when core process tests start using it.

Add actual-child cancellation/process-loss tests at this gate: accepted pure work
must remain supervised after waiter cancellation, next-request behavior must be
defined, and close/failure completion must satisfy Gate 0's reap contract. No
automatic retry, reconnect or native fallback is authorized by this contract.

## Gate 4: apply the explicit session-owner seam and fail closed

Prepare/apply the core patch described in
`../core/src/session/CONTEXT_REPLAY_ACTIVATION.md` after shared-seam approval.
Use actual `Session::new` and its existing isolation-aware immutable catalog.
Transfer the nonclone ReplayOwner through startup into the submission-loop task;
retained Session/service Arcs cannot own terminal lifetime.

Change `apply_rollout_reconstruction` and `record_initial_history` to Results and
propagate selected-component errors to `Session::new`. Keep the original reducer
on the unselected path. Revoke under SessionState before teardown awaits and hold
the host-generated owner/epoch gate together with SessionState while committing
all companion state. Revalidate any later prefill update. No IPC occurs under that
state lock. Update native tests to handle Results explicitly.

An additional startup ownership seam needs agreement before this patch is applied:
reconstruction currently follows SessionConfigured, MCP installation and prewarm
startup, but `Session::new`'s error arm only discards pending persistence. The
existing SessionStartup holder performs full runtime cleanup for managed starts;
those starts are isolated and suppress global replay selection. Ordinary selected
starts therefore need an explicit retained constructed-session owner that joins
`shutdown_session_runtime` on failure/cancellation, with idempotent teardown so
managed cleanup cannot duplicate thread-stop callbacks. The staged ReplayOwner
alone owns only the replay connection, not all those already-started services.
Preserve native event ordering and cover this newly fallible point explicitly.

Add integration tests for selected real resume and in-memory fork; malformed or
terminated workers; cancellation during acquisition and reconstruction; an
early-published retained Session Arc; failed constructor cleanup; submission-loop
termination; owner revocation racing a completed result; immutable selection and
isolated sessions. Compare complete native state, including reasoning variants,
executed-call evidence, completion flags, success, token budgets and permissions.
Check persistence and emitted terminal events after failed startup, not only error
strings or process existence.

Run the new owner/process tests plus existing native regressions in the same
compiled binaries. Source-backed existing groups include:

```sh
just test -p codex-core --test all -E 'test(suite::compact_resume_fork::) | test(suite::abort_lifecycle::) | test(suite::startup_cancellation::)'
```

Also select relevant existing `guardian_authorization`, `guardian_retained_context`,
`approvals`, `stream_no_completed`, model-request/streaming and rollback tests.
Keep original defaults in those regressions; separately exercise selected replay
through the same behavioral triggers. Run required broader core/protocol/workspace
checks at the coherent integration checkpoint, with failures and skips recorded.

## Gate 5: prove installability and preserve presentation behavior

Build the host once, record executable hash/size/mtime, export the production Rust
dependency closure with `component-sdk/rust_component_package.py`, build the worker
outside the checkout, assemble/install/select its package, remove the source/package
directories, then perform real host resume/fork/next-turn and shutdown flows.
Compare host hashes before and after every install/runtime step. A copied workspace
worker binary or exported source tree alone is not this acceptance.

Recheck the existing web presentation's real-engine recovery flow during this
integration cycle using the available browser workflow. Codec/library checks are
runtime-only; the GUI regression belongs to the actual presentation integration.
Preserve Apache-2.0 provenance and record artifacts, exact commands and remaining
coverage. Successful stateless reconstruction still does not establish replacement
of live context ownership, compaction scheduling or token-budget policy.
