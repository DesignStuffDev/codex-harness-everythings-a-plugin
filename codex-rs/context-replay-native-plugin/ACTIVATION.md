# Native replay component: saved source and activation gate

This worker, host adapter, native context engine and replay reducer are **staged,
unlinked and unverified**. Their shared `component-state-codec` dependency is now
active with 37 codec and three protocol-helper tests passing on Linux. No context
runtime replacement, independent build or completed context subsystem transformation
is claimed. Activation waits for the storage/lifecycle checkpoint and shared-seam
approval. See `../context-engine/NEXT_INTEGRATION.md` for the proposed test gates.

## Agreed contract

- Kind/name/version: `context_replay` / `default` / `1`.
- Method: `context_replay.reconstruct`.
- Input: complete owned `ReplayInput`, using the trusted-state codec.
- Output: complete `RolloutReconstruction`, using the same trusted-state codec.
- One immutable startup selection and persistent connection per engine session.
- The worker reconstructs fresh native state per request, without core callbacks,
  host dependency grants, durable storage, or live context mutation.
- Dropping the caller's future abandons the waiter. Accepted reconstruction stays
  supervised. Explicit close drains accepted work before shutdown acknowledgement;
  the configured deadline bounds forced termination. There is no fallback or retry.
- Physical frames are chunked by `component-host`. Full owned history/JSON values
  remain materialized; this is not a constant-memory replay implementation.

The adapter's errors are static and bounded. At activation add `context_replay` to
the host's sensitive-state stderr suppression, as approved by the integration
owner. The management CLI's generic one-shot diagnostic `call` must reject this
contract, both because it requires persistent transport and because its output is
trusted internal history. Install/select/reset/list remain normal management APIs.

## Exact source integration still required

1. Register `context-replay`, `context-engine`, `context-replay-component`, and
   `context-replay-native-plugin` in workspace members/dependencies. Existing new
   per-crate Bazel files describe their targets but are not usable until Cargo/Bazel
   metadata is regenerated. The shared codec already has its audited direct
   dependencies and workspace registration. Refresh `Cargo.lock` and `MODULE.bazel.lock` under the
   coordinated build slot.
2. The explicit trusted executed-tool-call module and reexports in `protocol`
   are activated and tested. Every existing provider serializer/deserializer
   remains unchanged. Preserve that boundary when linking the replay callers.
3. Add the engine with `snapshot-api` to core's test dependencies and link the two
   new differential test modules. Do not alias production history types until the
   native-versus-extracted comparisons pass. The standalone worker deliberately
   uses the default engine without this feature and without core.
4. Register `context_replay` v1 in component manifest validation. Add stderr/CLI
   protections above. The native worker integration tests use actual installation
   and therefore cannot pass against an unregistered kind.
5. Capture selection during session startup, acquire `ProcessReplay` before replay,
   await it from `Session::apply_rollout_reconstruction`, then commit all companion
   metadata at the existing session-state seam. Validate the captured session
   owner/epoch before applying a result. No IPC under `SessionState` locks and no
   synchronous `ReplayHistory` RPC. Preserve the unselected native path and close
   the session-owned process during teardown and failed startup cleanup.
6. Run formatting, scoped lint/tests and required native regressions. Source-only
   tests currently cover semantic/authorization parity, token estimates, complete
   reconstruction, trusted-state round-trips, actual installed worker requests,
   large chunked histories, invalid-request recovery and isolated selections.
   Cancellation/process-loss/real resume-fork host acceptance still needs activated
   integration tests; current test sources are not passing evidence.

The live context transaction/mirror/epoch design remains in
`../context-engine/LIVE_STATE_PROTOCOL_DESIGN.md`. Do not infer live context
replacement from successful stateless replay.

## Independent build/install acceptance after activation

Build the host once, record its executable hash/size/mtime and retain it unchanged.
Use fresh directories outside the checkout for the following source export and
package; `REPO`, `SOURCE_EXPORT`, `BINARY`, `PACKAGE`, `MANAGER` and `TEST_CODEX_HOME`
below are task-specific absolute paths, not required environment variables.

```sh
python3 "$REPO/component-sdk/rust_component_package.py" export \
  --repo "$REPO" --package codex-context-replay-native-plugin \
  --output "$SOURCE_EXPORT"
cargo build --offline --manifest-path "$SOURCE_EXPORT/codex-rs/Cargo.toml" \
  -p codex-context-replay-native-plugin
python3 "$REPO/component-sdk/rust_component_package.py" assemble \
  --repo "$REPO" --binary "$BINARY" --output "$PACKAGE" \
  --id codex.context-replay-native --kind context_replay --name default
"$MANAGER" --codex-home "$TEST_CODEX_HOME" install "$PACKAGE"
"$MANAGER" --codex-home "$TEST_CODEX_HOME" select \
  context_replay default codex.context-replay-native
```

Run the real host resume/fork cases, not generic CLI `call`. Remove the external
source/package after installation and prove installed immutable code still works.
Compare exact internal reasoning, cell/executed-call provenance, success, budget
and companion state with native behavior. Exercise fresh sessions after reset,
multiple simultaneous owners, cancellation, malformed output, abrupt process loss
and graceful shutdown. Record executable hashes before and after every install and
runtime step. An exported tree, copied workspace executable or successful source
build alone does not establish independent plugin acceptance.

No commands in this acceptance section have been run for context replay yet.
