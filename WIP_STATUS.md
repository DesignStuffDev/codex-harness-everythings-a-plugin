# Preserved next-component work: unbuilt and unverified

This branch preserves the newer, uncommitted implementation recovered from
`/workspace/codex-harness-next-components`. It is a development checkpoint, not a
release or a replacement for the recovered baseline on
[`main`](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/tree/main).
Use `main` for that baseline and its current recovery validation.

The newer changes listed here have not been compiled, integrated into the
baseline, or validated through the real Codex runtime. Test source being present
does not mean those tests passed. The branch may require interface, dependency,
lockfile, Bazel and lifecycle repairs before it builds.

## Newer implementation preserved here

- **Model streaming transport v2:** shared framed/chunked transport and payload
  worker ownership in `codex-rs/component-host/src/{streaming_wire,wire_assembly,
  wire_sending,payload_work}.rs`, with catalog/process/API changes and staged
  streaming process tests. This work aims to remove the legacy model transport's
  single-frame limit while retaining cancellation and terminal-response rules.
- **Typed model domain and error bridge:** new `codex-rs/model-transport-api`,
  changes to `core/src/component_model.rs`, and supporting HTTP/backend/exec-server
  error and retry metadata handling. These are intended to preserve native event
  and failure semantics across the independently installed model boundary.
  Completeness and behavior have not been demonstrated.
- **Python SDK support for that transport:** `_streaming.py`, `model.py`,
  `model_errors.py`, and registration/packaging changes under
  `component-sdk/codex_component_sdk`. The earlier SDK wheel acceptance does not
  validate these newer SDK files.
- **Session startup and shutdown ownership:** staged
  `core/src/session/runtime_lifecycle.rs`, `runtime_cleanup.rs`, and lifecycle
  tests, with changes to session construction, startup, handlers, suspension and
  thread-manager handoff. The goal is retained, once-only cleanup across startup
  errors and abandoned waiters before further context extraction. There is no
  runtime proof of those new guarantees on this branch.
- **Native maintenance and manual migration:** changes to the native store's
  maintenance owner and thread-manager startup, a new `migration_run.rs`, native
  manual migration implementation/tests, and component migration wire/adapter
  support. Native-default startup ownership, the full CLI migration path and
  end-to-end capability/lifecycle behavior still need integration and checks.

These changes are source work toward the existing component plans. They do not
establish additional completed extraction beyond the verified baseline. Other
previously staged context/replay, broker and catalog material remains subject to
its own integration gates; its presence is not activation or runtime evidence.

## How to read copied evidence

`COMPONENTS.md`, `VALIDATION.md` and most files under `verification/2026-09-30`
were copied from the earlier storage-contract-v2 and presentation-lifecycle
checkpoint. Their executable hashes and successful runtime observations apply to
that older checkpoint only. They do not establish that this branch builds, that
its new model transport works, or that its new session/migration paths are safe.

The main branch also contains later recovery work, test portability corrections,
and validation evidence that this isolated branch has not incorporated. A
comparison between the two preservation branches therefore includes both newer
WIP implementation and main-only recovery files; do not treat every difference
as a proposed replacement or deletion.

The root README below the WIP notice is the retained upstream README. Its official
installers download upstream Codex, not this fork or this WIP implementation.

## Next integration gates

Reconcile the shared contracts and workspace/build metadata, compile the affected
packages, and run meaningful transport, lifecycle, cancellation, storage and
native-regression tests. Then rebuild a host once and independently build/install
compatible external packages to exercise real model, session and GUI behavior
without rebuilding that host. Keep the older successful checkpoint available
until the newer behavior is verified.

The full project remains incomplete: all native Codex functional subsystems have
not yet become independently installable and replaceable. This is our own
Codex-based harness and presentation plugin, not a modification of the installed
official desktop app. Upstream Apache-2.0 licensing and notices remain in
`LICENSE`, `NOTICE` and `UPSTREAM_PROVENANCE.md`; no DeepSeek or Cordis code is used.
