# Native storage maintenance ownership

This checkpoint closes a lifecycle gap in the existing `thread_store:default`
version 2 implementation. It does not yet route `codex migrate-rollouts` through
component selection. The separate CLI still uses the native manual migration API.

## Admission and ownership

`run_rollout_maintenance` continues to acknowledge scheduling promptly. All
`LocalThreadStore` clones share one supervisor. Registering a job and fencing
admission use the same lock, so every accepted job belongs to shutdown. Finished
jobs are reaped on subsequent admission rather than accumulating indefinitely.

`begin_shutdown_store` rejects new maintenance and requests cooperative stop. It
does not reject already accepted writer operations or close their pools. A single
owned task drains accepted maintenance; cancelling or dropping a close waiter
does not cancel this drain. Worker panics and failed joins are retained for every
subsequent shutdown observer.

`shutdown_store`, called after writer users have been fenced, starts a separate
owned final stage that observes maintenance completion and closes the initialized
store-owned history SQLite pool. Constructing and abandoning its returned future
still leaves cleanup running. The builtin store does not close the host's shared
auxiliary StateRuntime. The standalone native plugin owns its own StateRuntime
and closes it after maintenance, writers, and the history pool.

## Cooperative stop and durability

Migration checks stop between rollout paths. Once a path starts, shutdown waits
for its publication, recovery, or error cleanup, including awaited blocking I/O.
Startup lock contention wakes on stop. A cancelled pass does not advance the
full-scan cursor over an unfinished prefix, so subsequent startup can resume.
Existing uninterrupted manual migration calls preserve their behavior.

Compression stops scanning and admitting new files, then joins every accepted
blocking job. It does not abort an async controller and leave its blocking jobs
detached. A stopped pass does not preserve a completed-run marker that would
suppress the next attempt.

The native plugin fences maintenance, releases fork reservations, drains accepted
RPC handlers, fences live writers, observes maintenance/history-pool shutdown,
and closes its private state runtime before acknowledging protocol shutdown.

Ordinary background domain failures retain the upstream warning-only policy.
Shutdown success means accepted workers were joined; it does not claim that all
eligible rollouts migrated or compressed successfully. A worker panic or failed
join makes close fail because completion cannot be confirmed. Failure of one job
does not skip joining the others.

The owning Tokio runtime must remain alive through graceful shutdown. The host's
existing bounded timeout remains authoritative: expiration forces process cleanup
and reports unknown operation completion. Forced termination does not imply a
successful durability fence, and this change introduces no stronger guarantee
for abrupt process/runtime destruction.

## Verification targets

The source adds deterministic tests for held jobs, abandoned shutdown waiters,
shared-clone admission, racing acceptance, retained panic/join failures,
abandoned history-pool close, cancellation between migration paths, and waking a
startup lock retry. The installed native process case blocks a real migration
inside one path using SQLite, starts close, verifies it remains pending, releases
the database, and checks canonical persisted history and cleanup.

Compression tests exercise held blocking jobs, cancellation/marker policy, and
join-error propagation. Execution results belong in the verification artifacts;
this document alone is not evidence that those tests passed.

The 2026-09-30 scoped gate ran 653 tests across rollout, state, thread-store,
thread-store-component, and the native storage plugin. The first run passed 652;
the new process fixture exposed its missing SQLite metadata row. A native manual
migration report identified `MissingSqliteMetadata`. After adding the same native
metadata indexing step used by the engine, the focused process case passed,
including the cancelled-pass cursor assertion. Production code did not change
between those two runs: all 653 distinct tests passed across the recorded runs.
See [the broad run](../verification/2026-09-30/storage-maintenance-regressions.log)
and [the corrected process case](../verification/2026-09-30/storage-maintenance-owned-close.log).
