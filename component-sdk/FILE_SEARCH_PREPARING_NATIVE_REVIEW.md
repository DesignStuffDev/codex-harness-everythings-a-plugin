# Native Preparing cancellation: read-only implementation audit

2026-10-01. Repository `/workspace/codex-harness-everythings-a-plugin`.
Read the root AGENTS.md and `component-sdk/FILE_SEARCH_PREPARING_CANCELLATION_PLAN.md`.
This is a proposed bounded implementation split, not an implemented or tested change.
No checkout edits, Rust commands, Git commands, or product processes were run.

## Contract assumptions to agree before backend edits

The staged SDK primitive is a public non-Clone `PendingSearchStart` containing an
`Arc<dyn SearchStartControl>` and a `SearchStartFuture<'static>`. Its public guard
requests cancellation if the ticket or its consuming finish observer is dropped.
`SearchStartControl::request_cancel` is synchronous/nonblocking; its
`cancel_and_wait` observes one retained typed outcome with separate operation and
`StartCleanup`. A surviving control targets the same lease after ready handoff.

The native backend must implement the required `begin_open`, not adapt its old
uncancellable future by aborting it. Admission and quota reservation are synchronous
and bounded. Native construction and joining remain owned after every public waiter
is gone. Holding a control must not suppress cancellation from dropping the last
public pending owner or public session.

## Actual source chain and gaps

| Source | Actual behavior and required change |
| --- | --- |
| `file-search-api/src/backend.rs:186` | Only `open` is currently available. Required begin/control is a Rust implementer change. |
| `file-search/src/native_backend.rs:75` | Admission currently happens only when the returned async body is polled. Move the prepare/quota/register/spawn part into synchronous `begin_open`; preserve max sessions, three aggregate resource charges, identities, and preconstructed SessionTaskGuard. |
| `file-search/src/native_backend.rs:237` | `run_session` awaits `owner.create_backend` before publishing a ManagedFileSearchSession. This is the propagation gap. |
| `file-search/src/native_backend_session.rs:140,205` | NativeSession exists before construction, but `request_close` forwards only through `state.native`. It needs a retained pending native control, with latched closing checked during publication and forwarding outside the state lock. |
| `file-search/src/async_owner.rs:98,266` | Entry Preparing retains only `released: bool`. Admission must also allocate/retain a private constructor shutdown signal before spawning. Split synchronous admission from the ready observer. |
| `file-search/src/async_owner.rs:325,329` | Blocking constructor is retained by an async observer whose only guard is a token. Unpolled/drop-before-result can leave Preparing forever. A pre-spawn completion guard must publish loss and quarantine before its token drops. |
| `file-search/src/async_owner.rs:611` | Preparing release flips a boolean but cannot interrupt construction. It must latch the private signal here. |
| `file-search/src/async_owner.rs:654,666` | The retained close-result observer has the same unpolled-task gap, leaving Closing forever. Include it in this ownership fix, with a distinct close guard and first native failure preservation. |
| `file-search/src/native_session.rs:206,250` | The constructor receives an optional external cancel flag and allocates private shutdown too late. Accept a separately owned private signal internally; legacy external cancellation semantics remain unchanged. |
| `file-search/src/lifecycle.rs:25,94` | Constructor synchronously waits for supervisor readiness; supervisor constructs pool, index, walker before sending ready. Add cooperative checkpoints around these actual steps and retain RAII join ownership for partial construction. |
| `file-search/src/lib.rs:312` and `matcher.rs:41` | Existing walker/matcher already observe private shutdown and the legacy external flag independently. Reuse that distinction. |

## Smallest sound native design

### A. Owner pending ticket and private signal

Introduce a crate-private native owner ticket (suggested name
`PendingNativeSearchStart`) returned by synchronous `begin_create_backend`.
It contains an identity-scoped cancellation control and consuming ready observer;
its control should refer to owner/id/retained receipt, not a clone that accidentally
keeps a public managed lease alive. Existing public create/create_bounded remain
wrappers over the same internal admission/observer path so ownership semantics do
not diverge.

Under `OwnerInner.state`, reserve exactly one Entry and tracker token, allocate a
fresh `Arc<AtomicBool>` private shutdown flag, and insert it into Entry before any
spawn. This flag belongs only to this admitted identity. Internal constructor
arguments should name the two signals clearly (for example a small internal
`NativeCancellation { external, private_shutdown }` value); never overwrite or
store through the caller's `cancel_flag`. Existing sync public constructors allocate
their own new private signal and otherwise preserve their signatures.

`prepare_close` always latches the private shutdown signal. If Preparing, retain
released=true and wait for the actual constructor result; if Ready, existing
request_close also wakes the WorkSender. Private signal writes alone must not be
used as the final ready-session wake mechanism because matcher waiting is queue based.

Add constructor cancellation checkpoints before native work and before supervisor
spawn. If no native worker was admitted, a cancelled constructor may return
ClosedLease + NotAdmitted, only after local constructor-owned values are destroyed.
If native workers were created, cleanup can be Confirmed only after their actual
joins and destruction of retained callback/handler ownership. Never fabricate it
because a timeout elapsed or a future was dropped.

### B. Preserve actual construction receipts

Once a real validation/allocation/spawn failure is observed, retain that cause even
if cancellation races it. Cancellation must not replace ResourceExhausted or a real
SearchFailed with ClosedLease.

For partial construction, cooperative checkpoints belong inside `supervise`, where
PoolThreads/WalkerThread already own OS handles. Check before pool build, after pool
build, after index creation, and before walker/matcher admission where safe. Exiting
after pool creation must drop the owning pool/index before joining PoolThreads;
review all early returns for that existing local declaration/drop order. If an index
exists, use its actual shutdown path and join the pool. No external callback is
invoked just to represent a clean cancellation.

`lifecycle::start` must distinguish a clean cancellation before readiness from an
unexpected supervisor exit. On ready-channel closure, consume actual close outcome:
real operation failure wins; only outcome Ok plus observed private cancellation can
yield ClosedLease with joined cleanup. Ready racing cancellation can still produce
a session internally, but the owner/backend handoff fence must close it before
presenting success if cancellation linearized first.

Owner error accounting currently records every construction error. Suppress synthetic
ClosedLease from a cancelled constructor only when Entry already records explicit
release and cleanup is NotAdmitted/Confirmed. Keep the unsuccessful open error for
the caller, but the cancellation receipt/provider shutdown operation is Ok. Preserve
all other errors and Unconfirmed diagnostics.

Do not derive every cancellation outcome from a generic joined close receipt:
CloseCleanup::Joined loses the distinction between actual no-native-admission and
joined admitted work. Retain the original start cleanup class in the per-start receipt
(or a start-specific result cell) alongside the immutable close receipt. That allows
repeat `cancel_and_wait` calls to give the same honest NotAdmitted/Confirmed result.

### C. Native backend control and handoff

`NativeSearchBackend::begin_open` synchronously validates bounds, reserves existing
provider resources, inserts NativeSession, constructs SessionTaskGuard before spawn,
and returns the SDK PendingSearchStart with a control referencing that exact session.
It must not hold a provider lock during external hooks or constructor work.

The lifecycle task checks closing before native admission. If it wins here, complete
the start with NotAdmitted and refund only after local accepted task ownership is
drained. Otherwise obtain owner pending ticket, publish its control into NativeSession,
then forward any latched close outside the session lock before awaiting finish.
Publication and close races must have one shared state fence: either closer sees the
control or publisher sees closing; neither is allowed to miss cancellation.

The ready handoff check must be linearized with request_close. A oneshot containing
ready is not proof the public caller received a usable session. A final locked state
check before returning the public lease decides whether handoff or cancellation won.
After successful handoff, the old control remains valid only for this exact same lease.
It never gains authority over sibling sessions, scopes, or the provider.

### D. Owner-loss guards

Construct startup and close completion guards before scheduling their async tasks.
They own identity, owner, retained cells, private signal as needed, and tracker token.
On unarmed normal completion they do nothing; on task loss they synchronously fence,
publish ForcedShutdown/SearchFailed with Unconfirmed, mark Quarantined, preserve the
first known real operation failure, notify observers, then release their tracker token.
Locks are poison tolerant. Guard Drop must not call user code, join, log I/O, or panic.

The current `OwnerInner::started` treats any phase other than unreleased Preparing
as a reason to enter Closing. Once guards can create Quarantined before late result
delivery, this broad fallback becomes wrong. Handle states explicitly: a late success
may still need retained cleanup, but it cannot rewrite an already published
Unconfirmed receipt or automatically refund quarantined capacity. A late real failure
must still reach the bounded provider/owner error ledger. Keep any independent later
recovery receipt distinct. A guard is proof of owner loss, not proof of native join.

Runtime destruction may make spawned cleanup impossible. Report that uncertainty;
do not claim the added guard makes an arbitrary blocking OS call interruptible or
turn runtime destruction into a clean shutdown. Normal users must still await provider
shutdown before destroying the runtime.

## Deterministic tests that establish behavior

1. **Pending ticket before first poll:** begin on a current-thread runtime; cancel
   before lifecycle task polls. Assert no native admission, operation Ok with
   NotAdmitted cancellation outcome, open ClosedLease, repeated outcome identical,
   released quota, and a fresh sibling query still works.
2. **Real native constructor queued:** occupy the sole Tokio blocking worker using
   the existing async_owner test pattern. Admit A, obtain its control, cancel while
   constructor is queued, and prove private flag is latched before unblocking. No
   cleanup receipt or capacity refund before its retained result owner completes.
   Unblock, observe actual receipt, then query/reuse. This covers queued construction,
   not pool/index midpoint cancellation.
3. **Constructor midpoint:** use a per-instance cfg(test) construction hook (never
   process-global mutable hooks) just after pool workers exist, before ready. Gate A;
   cancel A; prove its private signal observed and receipt stays pending while gate
   is closed. Allow progress and assert actual thread/reporter drops plus Confirmed.
   Exercise already-ready sibling B and another pending start while A drains.
4. **Shared external flag isolation:** start two legacy owner sessions using the same
   Arc<AtomicBool>, cancel only A through owner control/drop, and assert the shared
   flag remains false, B still queries, and A joins. Also assert distinct private
   identities; never substitute a public shared flag as cancellation mechanism.
5. **Publication race:** gate between internal owner admission and NativeSession
   control publication, request cancel, then release. The exact A control gets
   cancellation; sibling B is unchanged. No accepted owner remains permanently
   Preparing and there is no duplicate close task.
6. **Ready/handoff race:** gate after native readiness before result delivery, cancel
   on each side of the handoff fence. Before fence: open fails and wait joins; after
   fence: control closes same returned session. No callback/query admission after
   winning close fence; late control cannot cancel a replacement identity.
7. **Real cause race:** gate genuine bounded allocation/spawn/resource failure plus
   cancellation. Assert complete typed error/outcome equality, real cause survives,
   cleanup is independently correct, provider shutdown retains real error, and
   confirmed failure refunds capacity.
8. **Abandonment:** independently drop ticket, finish observer, cancellation observer,
   and retained controls; each accepted owner completes once and repeated observers
   see the same receipt. Holding a control does not suppress public-owner cancellation.
9. **Actual unpolled task destruction:** use a current-thread runtime, synchronously
   admit owner startup/close tasks, then destroy it before first polling the relevant
   retained observer. Use bounded blocking gates that are always released before
   asserting. Observe receipts from a second runtime/retained handle: Unconfirmed,
   no indefinite Preparing/Closing wait, quota quarantined, no false Joined. This
   tests actual executor drop, not manually dropping a helper guard.
10. **Late result after uncertain receipt:** owner loss then late constructor cleanup
    must not change the original cancellation/close receipt or recycle its charged
    slot; a later real error appears in the ledger. Do not label still-live descendant
    processes as acceptable or replace lifecycle assertions with timeout success.

Retain existing pool partial-start/panic join tests, owner reporter teardown tests,
bounded failure/reuse tests, query identity tests, and the actual separately built
process worker/unchanged host acceptance. Native tests do not establish process RELEASE
forwarding, SDK compatibility, TUI/GUI behavior, or external package installability.

## Reviewable implementation split

1. Agree SDK primitive and typed receipt, including documented NotAdmitted semantics
   for local accepted-but-not-native-admitted work. Stage API tests separately.
2. Owner private signal + synchronous internal ticket + both result-owner guards,
   with focused owner/runtime-destruction tests. Prefer a sibling module for new
   owner ticket/guard logic: async_owner.rs is already 738 lines.
3. Native constructor cooperative checkpoints + real partial-start/join tests in
   a new sibling native startup test module. Keep legacy sync external flag contract.
4. Native backend begin/control/publication/handoff adoption and sibling/cause races.
   Coordinate native_backend.rs and native_backend_session.rs ownership with one
   implementer; do not independently edit the same state machine in parallel.
5. Root integrates process/service/runtime adapters and versioned package metadata,
   then focused regressions, scoped fix, fmt, frozen host, independent package build,
   installed-process and real TUI/GUI acceptance. No extra wire method is needed.

Steps 2–4 are one coherent native contract transition but can be reviewed as bounded
patches. Do not publish only a new API whose native adapter cannot implement actual
Preparing cancellation. No compile/test result is asserted by this audit.

## Source fingerprints observed

```text
0b99d712ac0825ae881a3f7ee7872e22a485b78e0fbe011ef1e30aff96c5d3ce  file-search/src/async_owner.rs
c73acf84ee839b3c54b0ad0a902b9ade9b60387e75e8a55f68dfd4f93440fa69  file-search/src/native_backend.rs
22c86b4dd2036d743d148fdd964b207a066401deebb9bd7e6126333e234851fe  file-search/src/native_backend_session.rs
594bdab6fbeb160786c76783ca5ccc589e29dfd5ad45ce98c806c5ab42b392a3  file-search/src/native_session.rs
1e11c8780477dfa98fd863580a56c589c05f7334a35fc7754eca99841bfd3360  file-search/src/lifecycle.rs
847a7bc11b14499dd7df1f74cb15018340f656d81b50c4b5907ad9e77482babb  file-search/src/lib.rs
8672f9a95466f7d114b38c9c0d98159b50b120d711b84b7f9db1f07f5913937b  file-search/src/matcher.rs
e93e5b02c22bef3d2491f7fa04b25e7e4117b3fa987e1183f98acb5d9bf3ae16  file-search-api/src/backend.rs
```
