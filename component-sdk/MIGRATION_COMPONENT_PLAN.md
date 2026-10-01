# Manual rollout migration component contract — P01 verified

P01 source checkpoint: `5d2f2a026ae6ce0c42cf56bb2f3b01971e4bf0bb`.
This extends extraction of **existing native storage maintenance**, rather than
adding a new demonstration tool. `migrate-rollouts` selects the installed backend;
its native implementation can be built and installed separately as package 0.2.0
with unchanged host binaries. Storage contract 2 gains optional migration contract 1.

[Validation and original failures](../verification/2026-10-01/P01_TEST_EVIDENCE.md)
record 625 distinct passing focused Rust cases across runs, independent source
build/install, old-package compatibility, actual legacy migration and cold resume,
plus Chromium GUI approval/cancel/recovery and manager-only SIGINT. The fixture
uses deterministic inference with real Codex, App Server, storage and native tools;
in-app Browser, live-provider, cross-platform and whole-workspace gates remain open.
Earlier maintenance evidence remains in
[STORAGE_MAINTENANCE_LIFECYCLE.md](STORAGE_MAINTENANCE_LIFECYCLE.md).
See [execution state](../EXECUTION_STATE.md) for the next extraction.

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
Cancellation is cooperative: an in-flight path publication finishes its native
journal/data/index publication and safe cleanup before stopping. Do not drop the
publication future when cancellation arrives. The store retains and joins that
accepted work; cancellation prevents admission of the remaining paths.
Use the existing path publication/recovery algorithm and its native Manual
telemetry trigger, not the startup cursor/skip policy.

## Additive version 2 wire surface

Add `manual_rollout_migration: Option<u32>` to capabilities with
`#[serde(default)]`. Missing or null maps to `None` (unsupported); `Some(1)`
advertises the supported manual-migration subcontract. An unknown version disables
manual migration only: ordinary storage-v2 operations remain usable. Older built
version 2 implementations therefore remain compatible. Storage contract 2 and its
initialization/handshake are unchanged. The selected implementation must explicitly
advertise migration version 1; unsupported migration fails the command without
opening an alternative native store.

Add typed ordinary requests/responses to the existing CALL protocol:

- StartMigration `{ contract_version: 1, lease_id, options }` -> acknowledgement;
- MigrationSnapshot `{ lease_id }` -> snapshot;
- MigrationReport `{ lease_id }` -> native-compatible report or typed error;
- CancelMigration `{ lease_id }` -> stop-request acknowledgement.

The service independently validates StartMigration's `contract_version` before
accepting native migration work, even after capability negotiation.

Add reserved control method `thread_store/release_migration` with
ReleaseMigration `{ lease_id }`. Reserve control capacity before sending
StartMigration using `call_with_cleanup`. The control is
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

Completed reports remain owned until explicit release. Enforce a separate maximum
of **32 retained migration leases**, independent of ordinary RPC admission and
reserved control capacity: completion of the start RPC does not free its retained
run/report lease. Admission must reject excess retained leases. Failed starts and
early releases must be removed once their counterpart settles, as with existing
fork leases; retained tombstones must not create an unbounded side registry. Large
reports use the existing persistent spool codec, not a new line/frame size cap.

Recent duplicate cleanup acknowledgements are retained for the last 128 lease
IDs. Lease IDs are unique for the entire connection lifetime and must never be
reused or replayed. This is bounded duplicate protection, not durable idempotency
or reconnect/retry support. The service admits at most 32 native runs/reports and
96 total pending entries, including failed-start and early-release tombstones.
An unmatched release waits at most 30 seconds before returning an explicit error;
its fence remains until the matching start is rejected or the connection closes.
Entry exhaustion closes migration admission rather than risking a late mutation.

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

- DryRun: no StateRuntime/database/metadata initialization and no background work,
  for either native or selected execution;
- Apply: initialize metadata as the current command does, independently of the
  engine's runtime feature flag. For a selected component, the worker owns this
  StateRuntime; the CLI host must not open a duplicate runtime. The native path
  keeps its native initialization owner.

Both paths set `startup_migration: false` and `startup_compression: false`;
manual execution must not incidentally schedule either startup job.

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
Second Ctrl-C/SIGTERM or the 210-second outer cleanup deadline may force shutdown
and must report unknown completion. The first SIGTERM uses the same cooperative
shutdown path as Ctrl-C.
The signal handler/registration must exist before starting the selected worker.
On all errors and early returns, retain the store guard and observe whole-store
shutdown; preserve both operation and cleanup failures.

Native migration metrics use the process-global OTEL provider, which is absent in
the selected worker. Reuse the shared public native migration telemetry recorder
in the CLI host **only for the selected-worker path**, preserving low-cardinality
result/outcome recording and the command's host process metrics. Native execution
already records these metrics and must not also use the host-side recorder.
The CLI owns these command-level metrics for selected runs; an external backend
must not export a second copy of them. Do not claim child metrics are exported,
or record a cancelled/failed run as a
completed migration. Telemetry parity and absence of duplicate native recording
remain P01 verification gates.

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
report retention then removal, the independent 32-lease limit and capacity recovery,
and whole-store close. Actual CLI tests should
compare native and selected stdout/exit behavior, assert unsupported old-version-2
capabilities and unknown migration versions fail without native fallback while
ordinary storage2 remains available. Verify StartMigration rejects an unsupported
subcontract version before native work. Verify selected Apply owns its StateRuntime
in the worker, dry-run opens no database, both startup flags remain false, and
telemetry records once in the appropriate process. Deselect to recover native behavior
without rebuilding the host. Use the packaged native storage component installed
outside the harness source for this acceptance. No browser-specific change is
introduced by this CLI/runtime phase.
