# P02 narrow follow-on: owned cancellation during Preparing

Status: the first native/process/service/runtime/AS/TUI Preparing milestone is implemented
and verified; this is not an API freeze or whole-search completion. Required source API2,
worker0.2/wire1, public Stop admission, installed legacy/session Stop and GUI clear-query
cancellation now have separate accepted evidence. Normal native/search/storage/GUI regression
and real manager-only SIGINT after unhold passed. Exact counts, versions and retained failures
are in [EXECUTION_STATE.md](../EXECUTION_STATE.md). Additional GUI retirement triggers,
pending-Open launcher shutdown, private storage search and the wider P02 services remain queued.
The original audit below is historical context; do not reimplement completed source.

Prepared after parent reported TUI focused23 and full5620 passes (4 ignored),
unchanged source and subreaper0; those are parent-owned results, not new tests run
by this audit. The current run is finishing full-CLI/installed TUI and GUI checkpoint first.
No change below should interrupt that frozen checkpoint.

The detailed source audit and proposed contract are retained below. The implementation
direction is the explicit per-start ticket/control/receipt described here; final
capability wording and source-SDK revision must be checked with implementation.
This is the durable follow-on queue, not completed cancellation behavior.

## Explicit scope and exit

Give one admitted Preparing start a synchronous cancellation capability and an
independently observable retained cleanup receipt. Healthy cancellation of A must
leave sibling B and the selected provider usable. No ready session/query/poll/wire
shape changes, global provider replacement, per-keystroke task fan-out, or private
rollout-storage access belong in this slice.

Cancellation request and cleanup completion are separate facts. An uninterruptible
filesystem/allocator/blocking constructor cannot be declared joined by a timer.
The receipt remains NotAdmitted/Confirmed/Unconfirmed with original operation
failure retained, and uncertainty keeps reservation/quarantine semantics. Actual
transport loss may still invoke the existing provider-wide failure containment;
do not promise sibling survival through a broken process connection.

Exit requires production native construction and actual process worker proof,
not only a convenient facade test double. A new host cannot repair the older
independent03 worker's constructor cancellation: preserve its old-wire regression,
then build/install a separately sourced updated worker for new capability proof.

## Ordered queue and exact ownership boundaries

0. **Finish current checkpoint first.** Full frozen CLI plus current
   independent03 package through real PTY, actual desktop GUI regression, source
   evidence, archive and authorized publication. Keep all failed first runs.
   No parallel contract edits during these gates.
1. **Agree the one shared ticket/control contract (one owner).** Required neutral
   `SearchBackend::begin_open(SearchOpen)->Result<PendingSearchStart,SearchStartError>`;
   non-Clone ticket `control()` plus consuming `finish()`; explicit
   `SearchStartControl::{request_cancel,cancel_and_wait}` and immutable
   `SearchStartCancellationOutcome { operation:Result<(),SearchError>,cleanup:StartCleanup }`.
   Existing open remains convenience over required begin+finish, not a default
   no-op/drop-future adapter. Mirror in FileSearchScope. Include the small public
   construction API needed by independent Rust backend authors; private ticket
   fields alone must not make the SDK impossible to implement externally.
2. **Implement native start ownership and guard fixes (native owner).** Expose the
   already-reserved NativeSession before awaiting FileSearchOwner creation, pass
   a distinct private per-start cancellation signal into that actual constructor,
   and retain its result/join owners. Native caller/sibling external flags must
   never be flipped. Add before-spawn completion guard to the FileSearchOwner
   async result owner; publish failure/Unconfirmed before releasing tracker token.
   Keep actual bounded allocation checks and native matching behavior unchanged.
3. **Implement process + service start control (process owner).** Expose the
   existing reserved Lease stopping control; keep ordered OPEN admission and
   its pre-reserved DeferredControl RELEASE. In the worker, retain/publish backend
   pending control before await and forward a latched close to it outside locks.
   Add service_start guard BEFORE spawn. Control cancel must work for Preparing,
   not just a later ready SearchBackendSession. No additional cancel wire RPC.
4. **Implement facade pending ticket (runtime owner, after contract agreement).**
   Reserve before publication; keep one owner/control/result per bounded lease.
   Cancel before backend admission skips begin and proves local NotAdmitted;
   cancel during control publication is latched and forwarded; cancel before
   public session handoff closes the same lease. Ready-before-cancel returns the
   real session but surviving control can close only that same lease. No callbacks
   after the fence; callback/drain join still required for Confirmed. Preserve
   guards when a Tokio task is dropped before its first poll.
5. **Adapt the nine controlled fixture implementations and run the core slice
   (one owner serializes Rust).** Required trait addition intentionally forces each
   implementation to supply actual control/ownership semantics. Compile/default
   convenience users first; then actual lifecycle tests. Preserve original failed
   runs and exact source manifests. Do not port fake cancellation helpers as proof.
6. **Adopt tickets at real AS/TUI callers (consumer owner).** Wire existing
   one-shot token/long-session Starting observer and TUI generation/reconnect/root
   fence to the exact pending control; the retained coordinator/request owner still
   consumes original finish and cancellation receipt. Presentation cancellation
   never releases quota itself. Do not hold connection/manager locks across
   callbacks/hooks/awaits. Avoid duplicate cancel tasks/DeferredControl releases.
7. **Package/version and real-host acceptance (integration and packaging owners).** Export
   updated source, build in a new independent target, park source, assemble new
   package version, freeze host, install/select and exercise real Preparing
   cancellation with a concurrent sibling search. Reuse unchanged subreaper and
   strict PID absence. Repeat old independent03 normal-query parity separately.
   Run current AS/UI failure, streaming/query, /cd, removal and shutdown gates.
8. **Checkpoint only what passed.** Update component coverage/roadmap/state and
   SDK lifecycle/version docs with original paths/symbols, exact tested sources,
   new package hashes and honest legacy capability limits. Publish verified
   milestone under existing authorization. Unverified fixture mechanisms or
   forced-cleanup uncertainty do not become a completed P02 claim.

## Affected source files at the audited revision

New private module names are proposals; existing paths below are exact.

| Layer | Production source to change | Purpose |
| --- | --- | --- |
| Neutral SDK | `codex-rs/file-search-api/src/backend.rs`, `lib.rs`; new `startup.rs` | Required begin_open, ticket/control/cancel receipt + public exports; std boxed futures, no Tokio/Core dependency. Reuse StartCleanup from error.rs. |
| Native backend | `codex-rs/file-search/src/native_backend.rs`, `native_backend_session.rs` | Split reserve/ticket from retained run_session; expose and forward private start cancellation without changing sibling/provider authority. |
| Native owner | `codex-rs/file-search/src/async_owner.rs`; optional new private `async_owner_start.rs` | Retained pending constructor, private cancellation propagation and before-spawn result-owner guard. Inspect `FileSearchOwner::create_backend/create_inner`, `OwnerInner::started`. |
| Process adapter | `codex-rs/file-search-component/src/process.rs`, `process_open.rs`, `process_state.rs` | Synchronous begin reservation, cancellation control over same stopping watch, retained OPEN+RELEASE receipt. |
| Worker service | `codex-rs/file-search-component/src/service_open.rs`, `service_lease.rs` | Store/forward backend Preparing control, latched close race and pre-spawn completion guard. `service.rs` only if new state exports require it. |
| Facade | `codex-rs/file-search-runtime/src/startup.rs`, `scope.rs`, `state.rs`, `session.rs`, `lib.rs` | Pending public owner/control/receipt, cancellation-aware publication and handoff, same quota/first-error rules. New private module only if it keeps modules focused. |
| App Server | `codex-rs/app-server/src/fuzzy_file_search.rs`, `fuzzy_file_search/publisher.rs`, `request_processors/search/connection.rs`; lifecycle.rs only if watcher ownership changes | Existing cancellation_flag/Starting observer gains same-start control; existing request/session IDs and notification fence remain. No public protocol DTO change is inherently required. |
| TUI | `codex-rs/tui/src/file_search/coordinator.rs`, `runtime.rs`, `state.rs` as needed | Coordinator stores/requests exact pending control when latest incarnation retires; bounded one owner/intent/mailbox and retry loop stay. |
| Packaging/docs | `component-sdk/rust_component_package.py`, `tests/file_search/build_worker.py`, `codex-rs/file-search-local-plugin/README.md`, `component-sdk/FILE_SEARCH_WIRE_PLAN.md`, `FILE_SEARCH_COMPOSITION_PLAN.md` | Explicit package/capability export and SDK/wire compatibility guidance, updated-worker independent proof. No unreviewed import/rebuild of whole harness. |

Exact existing SearchBackend implementers: **2 production +9 controlled fixtures**.
Production: `file-search/src/native_backend.rs`, `file-search-component/src/process.rs`.
Fixtures requiring required-method adaptation:

- `file-search-component/src/service_tests.rs`
- `file-search-runtime/src/test_support.rs`
- `file-search-runtime/src/cli_lifecycle_tests.rs`
- `file-search-runtime/src/cause_race_tests.rs`
- `file-search-runtime/src/startup_cancel_tests.rs`
- `app-server/src/request_processors/search/runtime_tests.rs`
- `app-server/src/in_process_search_lifecycle_tests.rs`
- `app-server-client/src/file_search_tests.rs`
- `tui/src/file_search/test_support.rs`

Add behavior tests beside the affected owners: neutral contract tests only for
public ticket/drop/receipt behavior; native_backend_tests and async_owner tests for
real construction; process_startup_tests + service_tests for actual retained
protocol/service owners; runtime startup_cancel_tests/cause_race_tests for sibling
scope/receipt races; AS runtime_tests and TUI coordinator_tests for actual caller
fences. `file-search-local-plugin/tests/native_process.rs` must gain updated native
process coverage. Add real installed acceptance only after a trustworthy gate can
hold native construction without substituting a fake backend or leaking test-only
scheduling switches into default production configuration.

## Version and compatibility decision recommendation

The four version planes are distinct:

| Plane | Current fact | Recommended treatment for this slice |
| --- | --- | --- |
| Host component manifest API | component-api COMPONENT_API_VERSION=1, ComponentSpec.metadata is an existing Value field | Keep1. Do not change global component manifest/storage contracts. |
| Search JSON wire | FILE_SEARCH_CONTRACT_VERSION=1; strict deny_unknown_fields Initialize DTOs; OPEN/RELEASE already identity-scoped | Keep1 while fields/methods/receipts remain unchanged. Do not add unnegotiated fields to InitializeResponse: that would break old strict clients. |
| Rust source SDK | file-search-api inherits the workspace version (currently all these crates use the source workspace version), no stable Rust dynamic ABI | Required begin_open is a source-breaking implementer change. Record an explicit SDK source-contract revision/changelog and require updated source SDK for rebuilt custom backends. Do not bump the entire Codex workspace merely to rename this slice or claim old implementers compile unchanged. Final publishable crate semver policy remains a platform release decision. |
| Native installable package | independent03 is manifest version0.1.0; exporter and build_worker currently hardcode/default0.1.0, metadata{} | Produce new immutable package version0.2.0 with separate build proof; leave03 untouched. Accept old package for documented legacy normal-query compatibility, never claim newer cancellation behavior for it. |

For explicit discovery, prefer additive **existing manifest metadata** over an
unnegotiated strict wire field: proposed `metadata.preparing_cancel_receipt=1`
on the file_search/default ComponentSpec. Define this precisely as cooperative
constructor cancellation plus typed retained receipt, not a guaranteed maximum
wall-clock join time. Absence means legacy/unknown, not false success. New selection
can carry this capability alongside its immutable binding and report it in evidence;
it must not silently reject all existing03 packages or silently strengthen their
claim. Runtime still requests/observes available RELEASE with uncertainty on loss.

`rust_component_package.assemble` currently hardcodes metadata{}; implement a
bounded explicitly supplied component-metadata option (or a search-specific SDK
package definition), validate it and preserve it in the inventory/provenance.
Do not edit installed immutable objects afterward just to make a test pass.
Existing old hosts already allow ComponentSpec.metadata, so normal old-host/new-
package wire compatibility remains testable. Catalog's exact contract-version
allowlist need not change unless the final design actually changes wire schema.
This metadata key and source SDK revision are recommendations to agree before
implementation, not an already-supported capability or a frozen API promise.

## Non-negotiable regression and cancellation matrix

- Cancel before task first poll, before backend reservation, during control
  publication, while constructor blocked, simultaneous readiness, and after the
  public ready session transfer; all target exactly one lease.
- Drop pending ticket/finish observer/cancel observer separately; accepted owners
  remain bounded and observable, no duplicate task/release, no orphaned callback.
- Clean cancellation has operationOk plus actual cleanup proof; real constructor
  or release error survives synthetic ClosedLease; joined failure refunds;
  Unconfirmed remains immutable and quarantined. Deadline alone is not Joined.
- Same provider/sibling query continues after healthy A cancellation. Actual
  process loss follows explicitly documented provider containment, not a false
  sibling-success promise.
- Runtime shutdown drops unpolled tasks: all three result-owner layers retain or
  publish failure/Unconfirmed before tracker tokens release. Test real executor
  teardown, not a helper that just sets the field it then asserts.
- Updated separately built worker, unchanged frozen host, real AS/TUI/GUI query
  behavior, cancellation and normal shutdown. Old03 separate compatibility gate.
  No model backend/desktop proof from native fixture or terminal-observer tests.

---

## Detailed source audit and proposal: cancellation of one Preparing file-search start

Status: independent source audit and proposed API, 2026-10-01. Not implemented,
compiled, tested, or an agreed SDK/version freeze. Repository observed:
`/workspace/codex-harness-everythings-a-plugin`. No checkout changes, Rust, Git,
product commands or servers were run for this audit.

## Observed gap

- `codex-rs/file-search-api/src/backend.rs:186` exposes only `open`'s eventual
  session result. There is no neutral control capability for that one pending
  start. Provider shutdown is broader authority.
- Runtime `startup.rs:94` retains and awaits `backend.open`. `session.rs:164`
  fences its lease, but can call `backend.request_close` only after an actual
  session is published. `session.rs:234` waits for `start_finished`. Dropping the
  public waiter requests close through AbandonedStart; it does not provide an
  independently observable pending-start receipt or reach a blocked backend.
- Process `process_open.rs:45` already reserves a lease, owns a CancelOnDrop
  watch, and retains `run_open`. Its `admit` and reply observation select that
  lease's stopping watch, then consume the paired DeferredControl release reply.
  This usable Preparing control is hidden inside the returned open future.
- Worker `service_open.rs:35` reserves Preparing synchronously, but its task
  then waits for `backend.open`. `service_lease.rs:55` cannot forward closure
  until a session exists; its join at line116 waits for `start_finished`.
- Native `native_backend.rs:238` awaits FileSearchOwner::create_backend before
  calling NativeSession::started. NativeSession already exists, but requesting
  its close while Preparing does not reach the pending FileSearchOwner creation.
  `async_owner.rs:266` retains a Preparing entry and a blocking constructor.

A TUI select that merely drops or replaces `scope.open` is insufficient. It can
fence presentation, but cannot establish that accepted startup was cancelled or
joined. Closing the entire shared provider to cancel one start is not an
acceptable healthy-path substitute.

## Recommended neutral and facade surface

Use an explicit synchronously admitted pending ticket. Proposed signatures:

```rust
pub trait SearchBackend: Send + Sync {
    fn begin_open(&self, request: SearchOpen)
        -> Result<PendingSearchStart, SearchStartError>;
    // Keep open(request) as a convenience implemented using begin_open + finish.
    // Existing provider request_shutdown/shutdown remain separate authority.
}

pub struct PendingSearchStart { /* private public owner and retained reply */ }
impl PendingSearchStart {
    pub fn control(&self) -> Arc<dyn SearchStartControl>;
    pub fn finish(self) -> SearchStartFuture<'static>;
}

pub trait SearchStartControl: Send + Sync {
    fn request_cancel(&self);
    fn cancel_and_wait(&self) -> SearchStartCancellationFuture<'static>;
}

pub struct SearchStartCancellationOutcome {
    pub operation: Result<(), SearchError>,
    pub cleanup: StartCleanup,
}
// SearchStartCancellationFuture = boxed Send future of the outcome above.

impl FileSearchScope {
    pub fn begin_open(&self, request: SearchOpen,
        reporter: Arc<dyn SessionReporter>)
        -> Result<PendingFileSearchStart, SearchStartError>;
}
impl PendingFileSearchStart {
    pub fn control(&self) -> Arc<dyn SearchStartControl>;
    pub fn finish(self) -> RuntimeSearchStartFuture<'static>;
    // RuntimeSearchStartFuture yields Result<FileSearchSession, SearchStartError>.
}
```

The exact exported type spellings can be adjusted together before implementation.
The essential contract is required `begin_open`, an immediately available
single-start cancellation capability, a consuming session handoff, and a typed
immutable cleanup outcome. The facade's existing async open remains convenience
syntax over this path, preserving callers that do not need pending control.

`begin_open` validates and reserves bounded metadata/release capacity before
publishing the ticket. It does not await construction, transport replies, or a
join. Immediate rejection is NotAdmitted. An accepted local Preparing ticket
does not itself prove remote/native admission; that distinction must come from
the selected implementation's actual receipt, never elapsed time or a phase name.

The ticket is non-Clone and has a public drop guard. `finish(self)` retains that
guard until an uninterrupted transfer to the public session; dropping the
ticket or its observing future requests cancellation. Internal tasks retain
implementation state, never the public ticket/drop guard. The control capability
does not itself defer cancellation when the last public pending owner is lost.
It retains only enough identity/receipt state to request or observe this lease.
If readiness wins, a surviving control still targets that same resulting lease;
it never acquires authority over the scope/provider or a replacement start.

No no-op compatibility adapter: do not implement begin_open by spawning an old
uncancellable open and returning a control that only drops its observer or
fabricates Confirmed. No default cancellation implementation may silently do
nothing. Old implementations must implement this contract or remain explicitly
outside the new capability guarantee.

## State transitions and race rules

1. Reserve the facade Preparing entry and task token under existing admission
   locks. Construct its completion guard before spawn. Single-start cancellation
   synchronously latches `closing` and fences callbacks/query/session handoff.
2. Obtain/publish the backend pending control without holding facade state locks
   across external hooks. If cancellation preceded task execution, avoid backend
   admission and finish locally with proven NotAdmitted. If it raced control
   creation/publication, inspect the latched intent and forward it immediately.
3. Request cancellation outside state locks. The retained startup owner continues
   observing the selected backend's result; cancellation is not future abortion.
   Both the control hook and owned receipt path need panic containment.
4. Preserve the final handoff check: cancellation linearized before session
   transfer yields SearchStartError with its real cleanup receipt. If transfer
   linearized first, cancellation closes the same now-ready session normally.
   A ready oneshot message alone is not proof the caller received its session.
5. Repeated controls/observers reuse one retained cancellation/cleanup outcome.
   They create no new cleanup tasks, release writes, or quota reservations.
6. Keep operation cause separate. Clean requested cancellation has operation Ok
   in its cancellation outcome; the unsuccessful open reports ClosedLease.
   A real constructor/release failure wins over synthetic ClosedLease. Suppress
   ClosedLease in provider error accounting only for prior explicit intent plus
   NotAdmitted/Confirmed evidence, never spontaneous failure or Unconfirmed.

## Process and worker integration

Move the existing process reservation/control exposure into begin_open. The
control targets that lease's stopping watch. `run_open` still owns both admitted
OPEN and DeferredControl, preserves admission/release ordering, and drains the
actual RELEASE reply. Before transport admission cancellation may prove
NotAdmitted; after admission it must consume actual cleanup proof.

The wire already has RELEASE for the exact provider/lease/epoch while Preparing.
Do not add a redundant cancel RPC. Update worker service_open to retain the
backend pending control and service_lease to forward close during Preparing.
Without this server-side change, the new host can send RELEASE promptly but the
old worker still waits for native construction before replying.

## Native construction ownership

Expose an internal FileSearchOwner pending ticket before awaiting its constructor.
NativeSearchBackend forwards cancellation to this exact ticket/private flag.
The owner retains the blocking constructor and, if construction succeeds after
cancellation, immediately fences and joins every created thread/callback. Check
private cancellation between admission/construction steps where practical.

Never cancel the entire FileSearchOwner/provider to implement one session's
request. Never mutate an external flag shared by sibling callers. A fresh private
flag may belong to this admitted ticket only. Allocation/worker failures that race
cancellation must retain their real cause. Native kernel/filesystem calls cannot
be forcibly joined by dropping an async future or declaring a timer expired.

## Completion guards and bounds

Runtime startup already has a pre-spawn StartCompletionGuard; retain and adapt
that pattern. Worker service_open currently moves only a token into its async
body at line53: task drop before first poll leaves start_finished false. Add a
whole preconstructed completion guard that records owner loss/uncertainty,
finishes the startup predicate, wakes closure and retains the failed receipt
before releasing the tracker token. Do not construct it inside the task body.

The analogous FileSearchOwner startup-result task at async_owner.rs:329 also has
only a token and depends on OwnerInner::started to leave Preparing. Guard its
unpolled/aborted path as well. Audit newly touched close/result-owner tasks for
the same rule. Guard drop must use poison-tolerant locks, avoid callback/logging
I/O and avoid a second panic during unwinding. A lost owner is Unconfirmed; no
guard is permitted to claim a join it did not observe.

Retain one charged Preparing slot, one startup owner, bounded input, one paired
release route, and one immutable cancellation receipt. NotAdmitted requires
actual no-work proof plus drainage of local accepted metadata/tasks. Confirmed
requires native workers, callbacks and retained handlers joined. Unconfirmed
keeps quota/ownership accounted. Late cleanup may support a separate recovery
receipt; it must not rewrite the old Unconfirmed receipt. Late real failures
still propagate into existing scope/provider error ledgers.

Bounded ownership is not a promise of bounded successful native shutdown time.
If a composition chooses a cancellation deadline, expiry can return Unconfirmed
while retaining the owner/reservation. It cannot abort a blocking constructor
or refund capacity. Current runtime escalates any Unconfirmed to provider
shutdown: preserve that documented failure-containment policy for actual backend
loss, or explicitly design per-lease quarantine for a merely local observer
deadline. Never claim sibling survival after real shared transport loss. Healthy
single-lease cancellation must leave siblings and provider usable.

## Deterministic gates before runtime acceptance

- Cancel before backend admission: no backend open; NotAdmitted; no leaked slot.
- Two starts in one scope plus another scope: gate one real accepted constructor,
  cancel only its control, and exercise/query both siblings throughout.
- Gate control publication, cancel between reservation and publication, then
  release the gate: latched intent reaches that exact pending owner.
- Race readiness against cancellation on both sides of session handoff; prove no
  callback/update admission after the winning fence and no accidental new lease.
- Drop the pending ticket, finish observer and cancel observer independently;
  retained owner still drains, repeated receipt identical, admission slot stays
  charged until proof. Repeated cancel emits only one release.
- Cancel concurrent with real ResourceExhausted/SearchFailed; retain cause plus
  Confirmed or independent Unconfirmed. A late error still reaches owner ledger.
- Exact unpolled-task/runtime-drop regressions for runtime, worker-service and
  native-owner guards; none may strand a permanently Preparing receipt or refund
  unconfirmed quota. Also test panicking control hooks and independent close path.
- Native production owner with deterministic constructor gate/private signal:
  cancellation reaches Preparing before constructor result, release produces
  joined cleanup, sibling private/external flags remain untouched. Controlled
  backends test facade logic only and must not be labelled native/process proof.
- Real process/service gate for OPEN then RELEASE-before-ready, exact identity,
  real startup error, lost/forced transport, stale epoch and repeated observers.
  Assert bounded retained leases and unchanged old-wire lifecycle behavior.
- Real updated independently built worker through the unchanged selected host:
  cancel a Preparing start, prove worker reuse/sibling search and normal joined
  shutdown under the unchanged subreaper; rerun TUI query/root/exit regression.

## Compatibility and sequencing

Required Rust begin_open changes the SDK implementation contract. Update native,
process, worker-service adapters and controlled fixtures together, then consumers.
Do not call this backward compatible merely because `open` remains a convenience.
Decide crate/SDK versioning and advertised capability before an API freeze.

Existing wire RELEASE remains unchanged. Preserve old-wire parity. The frozen
independent03 worker is older code; a new host alone does not give it prompt
Preparing cancellation. Keep its existing acceptance evidence labelled with its
old hash/guarantees. Separately export, build, package, install and test an updated
worker for the new cancellation claim, with its own provenance and frozen-host
evidence. If support must be distinguished at discovery, specify an explicit
capability/version negotiation; do not infer support from the unchanged v1 method
name. This proposal does not settle that negotiation or the final v1/SDK freeze.

Recommended sequence: agree the typed ticket/receipt and capability contract;
implement neutral/native/process/service ownership together; pass focused gates;
adopt in TUI/App Server consumers; build a new separate worker and run real-host
regression/acceptance. This proposal is not implemented; its preservation in the repository does not
constitute cancellation acceptance.

## Additional implementation audits, 2026-10-01

The [native owner/constructor review](FILE_SEARCH_PREPARING_NATIVE_REVIEW.md) and
[process/service review](FILE_SEARCH_PREPARING_PROCESS_REVIEW.md) refine the next
steps against exact source fingerprints. These are implementation audits, not
runtime proof. Retain original NotAdmitted classification independently of a
generic close receipt, guard close-result tasks as well as startup tasks before
spawn, preserve real failure causes through cancellation, and fence session
handoff after readiness has been queued. The reviews record deterministic tests
and legacy compatibility constraints; staged changes still need integration and
actual native/process/installed-host acceptance.

The neutral public pending ticket/control/receipt primitives are now implemented
at source `d22cea88aa23e35a619d996fc731122450e51313`. All 19 API tests passed,
including eight carrier ownership/handoff cases. This additive SDK support leaves
SearchBackend unchanged and does not supply actual constructor cancellation.
Continue the ordered native/process/service/runtime steps above.
