# P02 Preparing cancellation: process/service integration audit

Read-only source audit, 2026-10-01. No checkout edits, compilation, tests or Git
operations performed. Source inventory is in
[the original source fingerprint snapshot](FILE_SEARCH_PREPARING_PROCESS_REVIEW_SOURCE.json). The original cloud environment
was observed connected/running at enforced revision 45. This is a design/review
artifact, not runtime proof.

## Contract conclusions to settle before implementation

1. The additive `PendingSearchStart::new(control, future)` primitive is suitable
   for external Rust authors, provided the supplied future retains ownership and
   performs the backend-specific final handoff fence. The generic ticket cannot
   itself know whether that particular lease closed after a result was queued.
   Its Drop guard must remain armed through `finish` and disarm only on actual
   successful session transfer, not when a oneshot sender enqueues `Ok`.
2. `begin_open` must become required on SearchBackend; `open` can be convenience
   over begin/finish. Do not supply an adapter that simply spawns old uncancellable
   open. Immediate rejections admit no work; the new process implementation can
   do existing bounded input validation and local reservation synchronously.
3. A process control must retain *this lease*, not ProcessSession: retaining a
   public ProcessSession would suppress its last-public-handle Drop close.
   `request_cancel` calls this Lease's existing request_close. `cancel_and_wait`
   requests cancellation synchronously then returns an independently droppable
   observation of one retained outcome; it does not create another RELEASE owner.
4. Do not derive every cancellation cleanup as `close.cleanup.into()`. Joined
   alone discards the difference between a cancelled local request never admitted
   to transport, a remote explicit NotAdmitted receipt, and constructed work
   joined after cancellation. Retain that admission/startup receipt alongside the
   existing close receipt; use NotAdmitted only when local retained tasks/routes
   are drained as well. Runtime startup already distinguishes this at
   `file-search-runtime/src/startup.rs:151`.
5. Define how control hook panics surface. Worker must catch a panic in synchronous
   `begin_open`, `request_cancel`, creation of the cancel receipt future, and
   polling that future. A panic should not erase the original cause or prevent an
   independent owned close/drain path. Public ticket Drop must not double-panic.
   Catching a hook panic does not by itself establish cleanup.
6. No wire addition is needed. OPEN and identity-scoped RELEASE already support
   Preparing. Keep strict Initialize/Open DTO fields and contract version 1
   unchanged; newly promised native behavior requires the separate updated worker
   package and explicit capability/version evidence in the canonical plan.

## Exact process ownership and recommended change

| Existing site | Current behavior | Minimal coherent change |
| --- | --- | --- |
| `file-search-component/src/process.rs:111` | SearchBackend::open wraps async process_open::open. | Implement required begin_open and delegate to synchronous reservation/ticket constructor. Keep provider startup/shutdown authority separate. |
| `process_open.rs:45–73` | Input bounded, Lease reserved, CancelOnDrop and LeaseOwnerGuard created when the open future is first polled; retained run_open spawned. | Move input validation/reservation before ticket publication. Build control, oneshot owner and pre-spawn guard synchronously. Return ticket with owned `'static` result observer. Local guard must cover any panic before spawn completes. |
| `process_open.rs:74–81` | Observer returns queued result and disarms CancelOnDrop; no post-receive closing check. | After receiving success, check this Lease's closing state at the handoff linearization point. If cancellation won, request/join the same lease and return ClosedLease plus real receipt. Do not return a ready session merely because sender queued one. |
| `process_open.rs:88–139` | Provider admission mutex serializes epoch + bounded start_with_cleanup admission; stopping selected before and during admission. | Preserve exact ordering. A cancelled waiter for this mutex must send neither OPEN nor RELEASE; other starts can still proceed. Do not hold admission mutex through any response/join. |
| `process_open.rs:141–241` | One run_open owns both OPEN reply and DeferredControl, observes stopping, releases once, drains operations. | Keep that one owner. Publish precise immutable cancellation/start receipt when it completes; the new control only triggers and observes it. Preserve real earlier startup cause over synthetic ClosedLease and later release cause. |
| `process_state.rs:172–214` | request_close fences state + watch; finish removes lease only on Joined, retains first error and publishes close outcome. | Add immutable start/cancel receipt information, ensure first publication cannot be overwritten by a guard or late completion, and preserve quarantined capacity on Unconfirmed. |
| `process_state.rs:228–241` | LeaseOwnerGuard exists before spawn, fences provider and reports uncertainty on owner loss. | Retain it; extend its terminal publication to the new receipt, without claiming NotAdmitted just because identity/response is absent. |
| `process_session.rs:43–65` | Last public session Drop closes lease; close observes retained receipt. | Keep it. A surviving start-control must close the same ready lease without keeping its public session alive or affecting a newer lease. |

Important branches:

- `run_open:142–152` publishes close operation Ok/Joined for an admission error,
  while its open result carries the actual SearchStartError. A cancellation
  observer built only from close loses non-cancellation admission failure. Retain
  the operation/start receipt explicitly and normalize only an expected explicit
  ClosedLease, never ResourceExhausted or transport/codec failure.
- `run_open:222–229` currently preserves a remote NotAdmitted open error while
  ignoring a separate RELEASE failure in the lease close result. The new typed
  cancellation receipt must not promote lost local/transport cleanup to known
  success. Preserve the known native no-work fact separately from uncertainty
  about the accepted host route; test this branch before claiming exact receipts.
- `run_open:197–200` sends a public session. If the receiver vanished, sending
  drops it and its Drop sets stopping. Preserve that last-owner behavior.
- Component host `session.rs:230–263` guarantees there is no await between actual
  start admission and return of both handles. `DeferredControl:192–212` ensures
  drop queues a retained release and timeout is unknown, not joined. No change to
  the generic transport is required for the first search integration.

## Exact worker-service ownership and recommended change

`service_runner.rs:41–55` correctly admits messages synchronously in reader order
before spawning response handlers. Preserve this. `service_admit.rs:100–145`
already finds a Preparing lease and requests close using exact provider/lease/
epoch identity. It also retains bounded retired receipts. Do not move admission
into an asynchronous response handler or add a cancel message.

### Startup and publication

1. `service_open.rs:35–52` already reserves a lease and TaskTracker token before
   spawn. Construct a complete `StartCompletionGuard` **before** line 53, owning
   lease + token; merely moving a token into the async body is insufficient.
2. At task entry, inspect the lease's closing flag. If it was closed before any
   backend reservation, skip `begin_open` and finish with explicit no-backend-work
   proof. Do not call the backend just to wait for it to notice cancellation.
3. Invoke `backend.begin_open` without a lease/registry mutex and catch panic. On
   success extract/clone its control, publish it into a new
   `LeaseState.pending_start: Option<Arc<dyn SearchStartControl>>`, and sample
   the latched closing state under the same state mutex.
4. If closing was already latched, invoke that exact pending control outside the
   mutex immediately. If cancellation arrives after publication,
   request_close obtains that control under the same mutex and invokes it after
   releasing the lock. This closes both sides of the publication race.
5. Await the pending ticket's consuming finish in the retained startup owner.
   Never select/drop that future as the cancellation mechanism. Capture real
   failure/cleanup in LeaseState, set start_finished, and notify.
6. On success, publish the ready session under the lease lock; if closing won,
   forward to the session and join instead of acknowledging ready. Keep the
   current first-cause vs expected ClosedLease discrimination. A wire OPEN reply
   and its later processing are not a public-session handoff in the host.
7. Clear the stored pending control when its role has safely transferred to the
   ready session or a terminal result. Do not allow unnecessary retained control
   cycles after cleanup; the outer public control remains identity-scoped.

### Closure, independent drain and guard completion

`service_lease.rs:55–75` already snapshots the ready session outside lock before
calling its external request_close hook. Snapshot and call pending_start likewise.
Repeated requests must latch once, while allowing a late-published control/session
to receive the already-latched intent. External hook calls stay outside all locks.

The current join at `service_lease.rs:107–165` waits for start_finished, then only
closes a ready session. It needs a way to drive/observe the pending control's
retained cancel receipt independently when the nonblocking request hook failed;
otherwise a panicking hook can leave the startup future permanently gated. Keep
one cancellation-receipt owner rather than spawning a task per request. Observe
both original finish and cancellation cleanup; neither can be abandoned merely
because the other returned first. If a receipt is Unconfirmed, fence containment
and quarantine as specified, without manufacturing join of the startup owner.

There is an additional exact guard gap: `service_lease.rs:77` spawns its close
owner without a preconstructed guard. Fixing only startup does not guarantee a
terminal close receipt when the runtime drops both tasks before their first poll.
Add a pre-spawn close completion guard as part of this slice. Guard Drop must use
poison-tolerant locks, retain first failure, publish immutable Unconfirmed, keep
the reservation charged, and release its token only after publishing terminal
state. It must not call untrusted hooks, block, or depend on a newly spawned task
executing during runtime teardown. A later genuine recovery may release capacity
but must not rewrite the original uncertain receipt.

The service currently uses `closed.send_replace` in its close owner. Use a
first-publication helper or equivalent lock discipline for guard-vs-normal
completion so an already observed Unconfirmed cannot become Joined later.

## Adjacent pre-first-poll gaps: track explicitly

These are source observations, not new test failures. They should be a small
separate guard-focused patch if adding them would make the initial start-control
integration too large; do not claim whole-service runtime-teardown coverage until
they have gates:

- `service_operations.rs:73/149`: update/poll publish active flags before spawn,
  but construct OperationGuard at lines75/151 inside the future. An unpolled task
  can leave the active flag true forever. Move the guard outside spawn and record
  dropped operation failure instead of merely clearing a flag.
- `service_provider.rs:75`: initialization publishes identity and limits but
  its completion state is not guarded before spawn. An unpolled initializer can
  leave initialized false and no initialization cleanup receipt.
- `service_provider.rs:164`: provider shutdown's owner is unguarded. Runtime
  teardown can leave the provider closed watch unset even after all task tokens
  drop. A terminal provider receipt requires its own before-spawn guard.

The runtime facade already contains useful reference patterns:
`startup.rs:90/210` StartCompletionGuard and `session.rs:189/320`
CloseCompletionGuard. Reuse semantics, not a whole cross-crate implementation;
its locks and owner/registry accounting differ from this service.

## Test inventory and additions

Existing tests must keep their actual assertions or be split when intentional
semantics changed; a fixture expectation update is not native runtime proof.

| Existing file/tests | Preserve/add |
| --- | --- |
| `process_tests.rs` abandoned_open_keeps_allocation_until_reserved_release_joins | Add public-ticket drop and independently dropped finish/cancel observers; retain one RELEASE and charged capacity until actual receipt. Existing abort-open case remains. |
| `process_tests.rs` ordered_admission_does_not_wait_for_the_previous_open_response | Add begin-open cancellation while waiting for admission gate and prove no wire OPEN/RELEASE for cancelled start while another lease continues. |
| `process_startup_tests.rs` two cancelled_startup_preserves_genuine_release_failure cases | Exercise new per-start control, not only whole-provider shutdown, preserve original ResourceExhausted plus Joined/Unconfirmed respectively. |
| `process_startup_tests.rs` earlier_genuine_start_failure_is_not_replaced_by_release_failure | Keep first-cause priority in open, cancellation receipt and provider ledger. |
| `service_tests.rs` release_finds_preparing_before_backend_task_runs_and_joins_late_start | Split into early release/no backend entry/NotAdmitted and a separate already-entered construction/late ready/join case. Current test waits for backend entry after release; it would hang if copied unchanged into the desired skip-before-admission behavior. |
| `service_startup_tests.rs` explicit_release/shutdown/spontaneous_closed/genuine_failure/unconfirmed cases | Adapt controlled backend to a real pending control and preserve full-object receipt equality. Do not normalize spontaneous ClosedLease or Unconfirmed. Explicit changed cleanup discrimination must be documented. |
| `service_boundary_tests.rs` bounded_rejection_receipts_do_not_invalidate_an_older_live_lease | Retain old lease validity while repeated cancellation/rejection churn retires other bounded receipts. |

New deterministic gates:

1. Direct synchronous begin returns control before finish is polled. Cancel before
   owner first poll: no backend admission, no wire work, local owners complete.
2. Gate inside backend begin before it publishes control; another thread requests
   RELEASE. Resume begin and prove exactly that control is cancelled outside the
   state lock. A control that safely re-enters diagnostic state catches lock use.
3. A and B in one provider, A still Preparing and B querying. Cancel A only; B
   succeeds throughout, no provider shutdown. Add C after A's joined refund.
4. Cancellation on both sides of ready result handoff; no ready session returned
   when cancellation wins, no newer lease or sibling authority acquired.
5. Repeated cancellation and dropped cancellation observers return identical
   receipts without another release, startup task or quota reservation.
6. Real ResourceExhausted/start/release cause racing cancellation remains the
   operation cause. Test remote NotAdmitted + lost RELEASE separately from normal
   no-work rejection. Unconfirmed remains immutable and charged.
7. Panic in request_cancel with an independently functioning cancel_and_wait path;
   original retained failure remains while actual cleanup can still join. Panic
   in both routes yields uncertainty, never a fabricated successful close.
8. Actual current-thread runtime teardown: initialize service first, enter the
   runtime without driving tasks, admit OPEN/RELEASE, then drop the runtime. Read
   stored startup and close receipts under another observer runtime, assert
   Unconfirmed and charged capacity, not merely that tracker tokens disappeared.
   Include teardown after first startup poll and close-owner-before-first-poll.
9. Actual process test peer verifies exactly ordered OPEN then one RELEASE-before-
   ready for the same identity; stale epoch rejection, loss and forced transport
   uncertainty retain existing behavior. Current Python fixture holds all opens
   with one gate: extend with a per-lease/epoch gate for the sibling test rather
   than pretending a global gate proves isolation.

Controlled service backend tests prove service ownership only. Python stdio peer
tests prove process adapter ownership only. Production native-owner tests and a
new separately built native worker are required for the end-to-end cancellation
claim; old independent03 cannot acquire new constructor semantics from a new host.

## Reviewable patch split

1. **Neutral additive primitive**, no backend behavior change: public ticket,
   control/future/outcome, Drop/finish/panic documentation and focused contract
   tests. This is infrastructure, not Preparing cancellation acceptance.
2. **Completion guards**, small service-focused change: startup + close owners,
   immutable publication, exact unpolled-runtime regressions. Keep adjacent
   operation/initialization/provider guard changes in their own reviewable patch
   if necessary, with explicit remaining gaps.
3. **Required begin contract + process/service adoption**, synchronized with the
   native and facade owner changes and fixture adapters. Process ticket and
   cancellation receipt first, worker pending-control publication/forwarding and
   independent drain second. Required trait migration should never land with a
   silently uncancellable temporary default. Approximately three process files
   plus two worker files and focused tests, rather than growing service.rs.
4. **Real consumers and package evidence**, after owner gates: AS/TUI cancellation
   sites, native package 0.2.0 metadata/provenance, old03 compatibility, frozen host
   + separately built updated worker and existing GUI/CLI lifecycle regression.

All Rust execution remains parent-owned and serialized. Source hashes in this
audit are provenance only; they do not imply a tested source revision.
