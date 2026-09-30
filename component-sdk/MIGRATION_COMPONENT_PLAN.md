# Manual rollout migration component contract (proposed)

This is the agreed next phase, not an implemented interface. The current
maintenance lifecycle checkpoint is described in
[STORAGE_MAINTENANCE_LIFECYCLE.md](STORAGE_MAINTENANCE_LIFECYCLE.md). Its regression
gate must finish before these source changes begin.

## Native contract

Extend the existing object-safe `ThreadStore` interface, without adding a core
dependency or exposing `LocalThreadStore` to the CLI's orchestration:

```rust
fn supports_manual_rollout_migration(&self) -> bool { false }
fn start_rollout_migration(
    &self,
    options: RolloutMigrationOptions,
) -> ThreadStoreFuture<'_, Box<dyn RolloutMigrationRun>>;

/// Owns observation/control of one accepted migration. Drop requests release;
/// the backend still owns accepted work until it has joined safe cleanup.
trait RolloutMigrationRun: Send + Sync {
    fn snapshot(&self) -> ThreadStoreFuture<'_, RolloutMigrationSnapshot>;
    fn report(&self) -> ThreadStoreFuture<'_, RolloutMigrationReport>;
    fn cancel(&self) -> ThreadStoreFuture<'_, ()>;
    fn close(self: Box<Self>) -> ThreadStoreFuture<'static, ()>;
}
```

The default start implementation returns `Unsupported` with stable operation
`manual_rollout_migration`. Snapshot is an immediate consistent observation;
it does not itself wait for progress. `cancel` acknowledges an idempotent stop
request, while `close` acknowledges joined cleanup and disposal of the run.
Dropping any ordinary waiter does not abort a backend operation. Dropping the run
queues its pre-reserved release request; it does not claim cleanup completed.

`RolloutMigrationSnapshot` contains:

- `revision: u64`, incremented for each observable transition;
- `phase: Scanning | Running | Completed | Cancelled`;
- `processed_paths: u64` and `total_paths: Option<u64>` (unknown during discovery);
- cumulative counts for Eligible, Migrated, AlreadyPaginated, SkippedEmpty,
  SkippedBusy, and Failed.

A Scanning snapshot with zero processed paths is not terminal. Running with zero
paths is also not inferred to be terminal. `Completed` means the worker settled;
an operation-level failure is returned by `report`, and a successful report can
still contain failed per-file outcomes, exactly as it does today. Cancelled means
the current path finished its safe cleanup and no remaining paths were started.
Its report is the completed prefix. Reporting an active job returns Conflict.

Keep the existing options unchanged: DryRun/Apply, repeated thread IDs, and the
aggregate I/O rate limit, including native overflow/positive-range validation.
Keep the existing public report/outcome/status/failure-reason shapes. The process
wire uses a separate faithful mirror for native paths through the version 2
state codec; do not derive ordinary path serialization as the transport format.

The local implementation registers the actual native migration future with the
shared maintenance supervisor. It owns the report/progress state and stop signal;
its observer handle does not own a cancellable transaction future. Whole-store
shutdown stops and joins manual jobs alongside startup migration/compression.
Use the existing path publication/recovery algorithm and its native Manual
telemetry trigger, not the startup cursor/skip policy.

## Additive version 2 wire surface

Add `manual_rollout_migration: bool` to capabilities with `#[serde(default)]`.
Older built version 2 implementations remain compatible and negotiate false.
Do not add a required field to initialization or change the version 2 handshake.
The selected implementation must explicitly advertise support; lack of support
fails the command without opening an alternative native store.

Add typed ordinary requests/responses to the existing CALL protocol:

- StartMigration `{ lease_id, options }` -> acknowledgement;
- MigrationSnapshot `{ lease_id }` -> snapshot;
- MigrationReport `{ lease_id }` -> native-compatible report or typed error;
- CancelMigration `{ lease_id }` -> stop-request acknowledgement.

Add reserved control method ReleaseMigration `{ lease_id }`. Reserve control
capacity before sending StartMigration using `call_with_cleanup`. The control is
fenced after the start request's final wire frame, but the service must still
handle release-before-start because independent request handlers can run in a
different order. The service must not hold its registry mutex while awaiting a
job or native start. Its lease state machine is:

```text
absent -> Preparing -> Running -> Finished(result)
            |             |              |
release ----+-------------+--------------+-> Releasing -> removed after join
release before start -> Released tombstone -> native start settles -> removed
start failure -> Failed tombstone -> guaranteed release -> removed
```

A release-before-start request is remembered; the later start either avoids
native work entirely or immediately stops and joins work already accepted.
An acknowledgement of ReleaseMigration means cleanup has joined. If start never
settles because the connection dies, whole-store supervision and the existing
outer shutdown deadline determine the outcome. A late or lost start reply must
not cause a second migration to start or leak its lease. Do not automatically
retry/replay a mutating RPC after unknown completion.

Completed reports remain owned until explicit release. Request/control admission
bounds the number of live leases; ensure failed starts and early releases are
removed once their counterpart settles, as with existing fork leases. Large
reports use the existing persistent spool codec, not a new line/frame size cap.

The adapter's run handle forwards the typed methods. Its destructor queues the
reserved control. Explicit cancel uses the ordinary short request, while the
already reserved release remains available if that waiter is abandoned or normal
capacity is unavailable. Explicit close consumes the handle, sends the reserved
control, and waits for its acknowledgement. A timeout means unknown cleanup,
never a successful report or retry permission.

## CLI composition and presentation

The entrypoint remains `cli/src/migrate_rollouts.rs::run`; preserve config loading,
root overrides, OTEL setup/process-start reporting, arguments, and exit behavior.
Choose `thread_store:default` before constructing any native fallback. Use a
dedicated manual-maintenance composition path with both startup flags false:

- DryRun: no StateRuntime/metadata initialization, no background work;
- Apply: initialize metadata as the current command does, independently of the
  engine's runtime feature flag.

The existing core runtime factory is unsuitable: its native branch schedules
startup maintenance, and it uses runtime feature decisions. Prefer a small CLI
composition helper using the component catalog/adapter and the existing native
constructor. Do not expand codex-core for this new command path.

Keep JSON stdout exactly the existing `RolloutMigrationReport`. Keep verbose and
human report formatting, failure summaries, and report-before-error behavior for
per-file failures. Poll snapshot at the existing 250 ms TTY interval; use cumulative
counts rather than replaying dropped deltas, preserving accurate progress if
intermediate display frames are coalesced. Plain progress retains its 1,000-path
threshold and emits a final update. Zero-file completion follows explicit phase.

Keep native local storage-byte measurements before initialization and after
migration, as presently used only for Apply + non-JSON. They are a compatibility
measurement of the negotiated shared local storage paths, not a generic backend
size estimate. If a future backend lacks that compatibility, negotiate an explicit
measurement capability instead of assuming its physical layout.

First Ctrl-C requests cooperative cancellation and waits for the current path's
cleanup; exit nonzero and do not call a partial report a completed migration.
Second Ctrl-C/outer deadline may force shutdown and must report unknown completion.
The signal handler/registration must exist before starting the selected worker.
On all errors and early returns, retain the store guard and observe whole-store
shutdown; preserve both operation and cleanup failures.

Native migration metrics currently use the process-global OTEL provider. In the
selected worker that provider is absent. Preserve the command's host process
metrics and explicitly solve/report native migration metrics delivery (shared
low-cardinality result recording in the host is the smallest option); do not
silently claim child metrics are exported or duplicate native metrics.

## Source patch outline and acceptance

1. `thread-store/src/` adds public run/snapshot contract and trait defaults;
   native `local/rollout_migration` adds supervised manual job state around the
   existing algorithm. Retain private cancellation safe points/cursor separation.
2. `thread-store-component` adds the optional capability, wire mirrors, typed
   adapter/run lease, and service registry. Native package advertises the actual
   trait capability; existing generic handler/shutdown ownership remains in use.
3. CLI composes the selected dedicated store, reads cumulative progress, keeps
   existing render/report functions, and owns signal/cleanup lifetime.
4. SDK/contract documentation describes the additive capability and RPCs. Update
   the component inventory only after real selected CLI acceptance passes.

Required focused evidence includes native/component parity for dry-run and apply,
thread filtering, rate-limit validation, pending-journal recovery, compressed and
archived rollouts, busy writer/global-maintenance contention, per-file failures,
large reports, and lossless native paths. Verify dry-run storage bytes/metadata
remain unchanged and no startup work ran. Prove cancelled mid-path publication is
joined and resumable, cumulative progress does not undercount coalesced updates,
0/0 is not prematurely completed, and close joins manual jobs.

Real installed-process tests must cover cancellation before admission, released
before native start completes, lost/late start reply, failed start plus release,
report retention then removal, and whole-store close. Actual CLI tests should
compare native and selected stdout/exit behavior, assert unsupported old-version-2
capabilities fail without native fallback, and deselect to recover native behavior
without rebuilding the host. Use the packaged native storage component installed
outside the harness source for this acceptance. No browser-specific change is
introduced by this CLI/runtime phase.
