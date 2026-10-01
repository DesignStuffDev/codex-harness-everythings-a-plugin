# File-search component wire and owned service plan

Implementation status: the neutral API baseline is published at `98eeb3e`.
The new `codex-file-search-component` source now defines the version-1 DTOs,
lossless value conversion, bounded provider/lease service, persistent process
adapter and generic stdio runner. These additions are authored, not yet compiled
or runtime-verified at this checkpoint. Native worker packaging and host/client
composition remain separate integration work; these adapters alone do not prove
installed native replacement.

The implementation uses the concrete schemas in
`codex-rs/file-search-component/src/contract.rs`, `value_wire.rs`, `limits.rs`
and `outcome.rs`. It retains synchronous Preparing admission before any spawned
handler, serialized client open admission, immutable per-provider startup cwd,
64 bounded terminal/rejection receipts, and distinct operation/cleanup receipts.
`SearchBackendFactory` receives explicit base/config/state/negotiated limits;
`run_stdio` supplies transport composition without a Core or native matcher
 dependency. Initial service tests exercise production reservation/close ownership
using controllable backend completion; process tests exercise real stdio peers.
Neither is a claim that the native filesystem worker or real installed client is
verified. Root records the actual run results in execution/evidence documents.

The design and startup ordering audit below explain the decisions. Where a
proposal example differs in spelling, the typed source schemas above are the
implementation authority. Nested `SearchError` retains the neutral API's existing
serde behavior; outer new DTOs reject unknown fields.

# P02B file_search1 wire and lease-service implementation proposal

Status: read-only design, 2026-10-01. Neutral API baseline supplied by root:
`98eeb3e`. Inspected current primary checkout and concurrent transport-limit WIP;
no compilation, tests, Git operations or repository edits were performed here.
Nothing in this document is evidence of installed file-search replacement.

## Decisions and smallest implementation slice

Use `file_search:default`, contract version 1, on the existing multiplexed
component session. Add `codex-file-search-component` with DTOs, validation,
process backend and generic service runner, depending on the neutral API,
component host, component path codec, serde/JSON, uuid and Tokio. Do not depend
on Core, history, protocol, native matcher or configuration implementation.
The native plugin supplies an actual `SearchBackend` factory. Runtime composition
selects the implementation once and injects scopes separately.

Implement DTO conversion/validation and service admission before connecting
presentation clients. Actual native/process tests must follow; DTO tests alone
cannot justify installed-backend or lifecycle claims.

Use the four planned lease methods, plus one provider initialize/shutdown pair.
The extra pair negotiates aggregate ceilings once and returns typed whole-provider
cleanup evidence before the transport shutdown handshake. Without it, a generic
transport close cannot express failed-but-joined backend shutdown.

Additive transport prerequisite: `DeferredControl::release_reply(self) ->
Result<Value>`. Existing `release` delegates and discards to preserve storage.
Root has approved this and assigned it to the transport agent. Search must decode
the returned domain outcome; outer successful transport delivery is not a join.

## Actual reusable code and differences

- `component-host/src/session.rs::start_with_cleanup` now returns
  `(PendingComponentReply, DeferredControl)` immediately after paired admission.
  The caller can retain cleanup through response failure. Ordinary and control
  capacities are each 32. Deferred control is sent only after its paired request
  is fully written; dropping it queues cleanup but does not prove completion.
- `session_reply.rs::PendingComponentReply::wait` times out observation, not the
  accepted operation. Never retry a timed-out open/query automatically.
- `session.rs::DeferredControl::release` currently discards the response Value.
  The additive response-bearing form is essential for `SearchCloseOutcome`.
- `session_server.rs::ComponentServer` owns reader/writer tasks and yields fully
  decoded calls through a bounded queue. The runner owns handler tasks. Admit
  each request synchronously before spawning its handler, then run handlers
  concurrently so release can unblock a poll/start. `finish` requires all request
  slots released and flushes replies before `ShutdownComplete`.
- `thread-store-local-plugin/src/lib.rs::run_stdio` is the useful shutdown-order
  precedent: fence service work, drain handlers/owners, then `server.finish()`.
  Search must return typed operation and cleanup results separately; do not copy
  storage's flattened string result as the search contract.
- `thread-store-component/src/migration_service.rs` retains Preparing/Ready/
  Closing work, failed starts and release-before-start fences. It separately caps
  leases, metadata and retired receipts. Reuse the ownership principle, not the
  whole storage-dependent module or its string-only outcome. Its 128-entry retired
  cache alone cannot establish never-reused IDs after eviction; search also needs
  monotonic admission epochs.
- Migration/fork adapters still use `call_with_cleanup`; search uses the newer
  split admission primitive. No storage migration is required for this step.
- `component-api` has no file-search kind/version constant yet; catalog's kind
  whitelist and manager `Call` persistent-only guard need additions. Preserve
  component API version 1, storage contract 2, and their existing defaults.
- Current transport WIP exposes opt-in `SessionPayloadLimits` and local typed
  `PayloadLimitExceeded`. Search opts in on both client and server. Do not infer
  typed errors from remote diagnostic text. Existing storage stays uncapped.

## Identity, integer and authority rules

`provider_id` and `lease_id` are host-created canonical lower-case UUID-v4 strings
(36 bytes). `provider_id` is fresh for each process/backend incarnation. Every
reply repeats its identity and method; validate these before converting results.
A transport request ID is additional correlation, not a lease identity.

`session_epoch` is a positive u64 increasing across all lease opens on this
provider. `query_epoch` is the neutral API's positive query ID; zero is reserved
for initial frames. Revision zero is the initial observation cursor. Checked
increments never wrap. Use canonical decimal strings for these u64 values and
resource byte/count ceilings: this avoids JavaScript's 53-bit JSON-number limit
in separately written SDK clients. Reject signs, whitespace, leading zeroes
except the exact string `"0"`, and overflow. Convert to usize with checked
conversion at the native boundary. Small fixed bounds/options can be u32 JSON
numbers. Do not silently round or clamp.

The host serializes open admission, not open completion: hold a narrow admission
mutex while choosing an epoch and awaiting `start_with_cleanup`, release it as
soon as handles return, then observe the reply independently. A cancelled
pre-admission attempt can leave an unused epoch; gaps are legal. Neither close
nor poll takes this admission mutex. This ensures admitted wire opens arrive in
increasing order without blocking cleanup behind native startup.

`SearchOpen` carries no scope identity. Keep scope ownership in the host runtime:
a scope owns a set of backend leases; scope closure releases only that set;
provider closure fences all scopes and the one backend. Do not invent a wire
scope UUID that the current API cannot supply. Worker aggregate accounting and
host per-scope accounting are complementary. Idle scopes consume host max_scopes
metadata but no worker lease entries.

These are trusted selected component connections. IDs and valid decoded receipts
do not grant authority. Bind them to the selected immutable package, process
connection and current provider/lease epochs. The worker receives only explicit
roots; preserving symlink traversal is not an OS sandbox or proof of physical
root confinement.

## Encoding and exact shared records

All new records use snake_case and `deny_unknown_fields`. They are private
component DTOs, not changes to public App Server JSON. All optional data below is
explicit null, unless a method-specific schema says otherwise.

```
ProviderIdentity = { provider_id: UuidV4 }
LeaseIdentity = {
  provider_id: UuidV4, lease_id: UuidV4, session_epoch: PositiveDecimalU64
}
WireBudget = {
  max_index_entries: PositiveDecimalU64,
  max_index_bytes: PositiveDecimalU64,
  max_worker_threads: PositiveDecimalU64
}
WireOptions = {
  limit: u32, threads: u32, exclude: [string],
  compute_indices: bool, respect_gitignore: bool
}
SearchError = { kind: existing snake_case SearchErrorKind, message: string }
```

Each path uses the already accepted path codec: `{"UnixBytes":[u8...]}` or
`{"WindowsWide":[u16...]}`. Use `codex_component_path_codec::native_path` and its
vector adapter; do not lossy-convert into strings or import state/history codecs.
Errors never contain roots, queries, raw native error chains or credentials.
SearchError's existing 2,048 UTF-8 byte bound applies independently to each error.

Generic reply format:

```
{
  contract_version: 1,
  method: "file_search/<method>",
  identity: ProviderIdentity | LeaseIdentity,
  reply: { status: "ok", result: T }
       | { status: "error", error: E }
}
```

`E` is SearchError except open (SearchStartError). Release/shutdown successful
`T` is a close outcome that itself can describe operation/cleanup failure:

```
CloseOutcome = {
  operation: { status: "ok" }
           | { status: "error", error: SearchError },
  cleanup: { status: "joined" }
         | { status: "unconfirmed", error: SearchError }
}
StartError = {
  operation: SearchError,
  cleanup: { status: "not_admitted" }
         | { status: "confirmed" }
         | { status: "unconfirmed", error: SearchError }
}
```

Use explicit wire structs/conversion for operation result; do not accidentally
freeze serde's generic `{"Ok":null}`/`{"Err":...}` enum representation as this
language-neutral wire. Neutral API serde remains unchanged. Overall success
requires operation ok AND cleanup joined. A failed-but-joined session releases
its quota and still reports its retained operation error. Unconfirmed cleanup
retains quota until independent whole-provider recovery proves release; even a
successful operation is not enough. First operation failure must survive a later
cleanup error. Repeated host close observers get the locally retained outcome.

## Provider initialization and negotiated bounds

First ordinary method: `file_search/initialize`.

```
request = {
  contract_version: 1, identity: ProviderIdentity,
  base_dir: NativePath,
  requested_limits: {
    max_leases: u32, max_pending_polls_per_lease: 1,
    max_query_utf8_bytes: u32, max_roots_options_bytes: u32,
    max_matches: u32, max_frame_bytes: u32, max_poll_wait_ms: u32,
    resources: WireBudget
  }
}
ok.result = {
  limits: same record with negotiated ceilings,
  native_path_platform: "unix_bytes" | "windows_wide",
  encoding: "file_search1"
}
```

Check manifest version before process launch and protocol version again here.
The worker intersects explicit host ceilings with its supported/policy ceilings,
returns them once, and the host validates every field before lease admission.
Lower negotiated ceilings are a visible capability result, not permission to
silently clamp a caller's options later. There are no invented index defaults:
resource defaults must come from the native bounded-index implementation and its
measurements. A backend unable to enforce requested resource semantics rejects
initialization/options explicitly.

Hard v1 maxima:

| Quantity | Ceiling |
| --- | ---: |
| Retained native leases, including Preparing/Ready/Closing/Unconfirmed | 16 |
| Accepted poll per lease through completion | 1 |
| Accepted update per lease at a time | 1 |
| Query UTF-8 bytes | 65,536 |
| Compact serialized roots-and-options record | 262,144 |
| Requested/returned matches | 1,024 |
| Compact serialized WireFrame, including query and snapshot | 1,048,576 |
| Snapshot wait | 1,000 ms |
| Ordinary transport request payload | 524,288 bytes |
| Ordinary transport reply payload | 1,114,112 bytes (1 MiB + 64 KiB envelope reserve) |
| Control request/reply payload | existing 65,536 bytes |

The ordinary request cap accommodates six-character JSON escapes for a maximum
query plus identity/envelope; roots/options have their own smaller cap. Never
assume query UTF-8 bytes equal serialized JSON bytes. The reply cap is above the
transport's required 64 KiB minimum and accommodates the bounded full frame plus
identity/envelope. Domain errors and two bounded diagnostics fit the control cap
including JSON escaping. Validate actual encoded lengths before accepting them.
If reduced limits are supported later, configure both sides consistently; do not
change a transport limit mid-session.

`base_dir` is an explicit absolute host context snapshot, never worker cwd. This
is necessary because component-host launches entrypoints in package_dir and
current App Server forwards relative roots. Preserve original root values and
returned roots; interpret relative filesystem access against base_dir. Never change the host
process cwd or perform per-lease worker chdir. Native adaptation for mixed/overlapping/relative roots must
preserve matching precedence and duplicate behavior; simply absolutizing roots
can change deepest-root selection/tie order. This is an unresolved integration
gate, not something DTO conversion proves. TUI's absolute roots avoid that case.
Do not reject all relative App Server/CLI roots and call behavior preserved.

Implementation alternatives, requiring differential proof before selection:

1. **Recommended minimal process path:** an explicit opt-in startup cwd on the
   dedicated search worker, applied by `Command::current_dir(base_dir)` before
   exec. Preserve original roots and native matching unchanged. Add a narrow
   connect/start option; existing components retain package_dir cwd. Entrypoint
   remains absolute, and search SDK packages locate their own assets via their
   package/module location, not implicit cwd. The worker compares its actual
   startup cwd to the initialization snapshot and rejects mismatch. This supports
   different base directories on different provider processes without host
   mutation. It is a proposal; current ComponentBinding::connect does not expose
   this option, and no transport-agent implementation is assumed.
2. Preserve each traversal's original lexical path view while accessing explicit
   resolved filesystem paths inside native code. This can support independent
   in-process providers but is substantially more delicate for mixed roots,
   overlaps, duplicate roots, scoring and deepest-root choice.
3. A built-in in-process provider can preserve current behavior only when its
   requested base is the actual host cwd snapshot; a different explicit base
   requires option2 or a separately owned native worker. Never chdir to emulate
   independent embedded contexts. Distinct Codex homes alone do not imply distinct
   historical filesystem cwd semantics.

Pinned ignore0.4.25 `WalkBuilder::current_dir` adjusts global-ignore matching;
it does **not** resolve relative traversal I/O. Do not use it as a substitute.
Required differential fixtures include roots `[".", "/cwd/sub"]`, reversed
order, overlapping relative roots, trailing lexical separators, duplicate
resolved paths, symlinks and non-UTF-8 roots, with score/root/path/indices compared.

Initialize via `start_with_cleanup(initialize, ..., shutdown, ProviderIdentity)`.
Retain its provider-shutdown guard for the entire backend. At most 16 lease guards
plus this guard consume 17 of 32 control reservations. Handshake cancellation or
failure retains the guard/process owner until cleanup is observed. Selected
backend failure must never fall back silently to native.

## Lease methods

All lease requests include `contract_version:1` and `identity:LeaseIdentity`.
Service/client adapters additionally bind replies to the exact operation.

### file_search/open (ordinary)

```
request += { roots: [NativePath], options: WireOptions, budget: WireBudget }
ok.result = { initial_cursor: "0", limits: LeaseLimits, budget: WireBudget }
error.error = StartError
LeaseLimits = {
  max_query_utf8_bytes, max_matches, max_frame_bytes, max_poll_wait_ms,
  max_pending_polls: 1, max_in_flight_updates: 1
}
```

Preserve order and exact root bytes. Validate options >0, negotiated limits,
roots/options compact byte cap, platform, checked budgets/worker arithmetic and
aggregate reservation before native startup. Empty roots retain explicitly tested
native behavior; never substitute base_dir when no roots were provided. This
reply's cursor is the starting observation cursor, not a claim that no frame was
produced during startup. Freeze negotiated lease settings; do not reopen on cwd
or configuration changes behind the handle.

Host preallocates UUID/epoch and calls `start_with_cleanup(open, ..., release,
LeaseIdentity)`. A retained owner holds both handles across all awaits. An
observed open failure awaits typed release_reply unless NotAdmitted is already
proven; still dispose the reserved guard safely. Unknown/lost release proof is
unconfirmed. Never infer cleanup from an error response or dropped receiver.

### file_search/update_query (ordinary)

```
request += { query_epoch: PositiveDecimalU64, query: string }
ok.result = { accepted_query_epoch: PositiveDecimalU64 }
```

Validate strict increase under the lease's admission lock and exact UTF-8 length.
Reject duplicate/out-of-order IDs as stale_epoch. The backend admission receipt
must equal the submitted ID. Acknowledgment is not matching completion. Coalesce
superseded pending query work into one bounded latest-query slot; no replay on
transport failure. A second concurrently accepted update is ResourceExhausted;
the process adapter serializes accepted updates before they reach this gate.

### file_search/next_snapshot (ordinary)

```
request += { after_revision: DecimalU64, wait_ms: u32 }
ok.result = { status: "changed", frame: WireFrame }
          | { status: "unchanged", revision: DecimalU64 }
WireFrame = {
  revision: DecimalU64, query_epoch: DecimalU64, query: string,
  phase: { state: "running" | "idle" | "cancelled" | "closed" }
       | { state: "failed", error: SearchError },
  snapshot: null | {
    matches: [WireMatch], total_match_count: DecimalU64,
    scanned_file_count: DecimalU64, walk_complete: bool
  }
}
WireMatch = {
  score: u32, root_index: u32, path: NativePath,
  match_type: "file" | "directory", indices: null | [u32]
}
```

Wire snapshot omits duplicate query text/identity; decode reconstructs the neutral
snapshot from its enclosing current frame. `root_index` references the original
ordered roots and prevents arbitrary result roots; match path must be relative
(no prefix, root or parent traversal; empty path for root entry is allowed).
This lexical validation is not a symlink sandbox. Validate root index before
constructing FileMatch, exact original root bytes, indices ordering/uniqueness,
matching query identity/text, counts, frame size, phase and current epochs.

Changed revision must advance the supplied cursor; gaps mean coalesced complete
replacements. Unchanged revision equals the supplied cursor. Reject a cursor
beyond current state and waits above the bound. Idle requires a current snapshot
with completed walk. Never mark a partial exhausted index idle/complete. Retain
at most one accepted poll, even after its waiter drops; close wakes it and its
owner releases the poll slot only when its backend observation actually ends.

Validate/measure bounded typed output before allocating a JSON Value or starting
spooling. Snapshot overflow is a typed ResourceExhausted operation failure with
lease cancellation/cleanup, not a silently truncated successful result. Retain
only latest frame/query state, not one completion object per historical query.

### file_search/release (reserved control only)

```
request = { contract_version: 1, identity: LeaseIdentity }
ok.result = CloseOutcome
```

Fence the lease synchronously at admission. Preparing is marked released; any
accepted native start remains owned and is joined before receipt. Ready becomes
Closing; poll/update admission stops, pending observation wakes, retained work
and backend session close drain. Joined releases aggregate resources even after
operation failure. Unconfirmed holds reservation through provider recovery.

The service's outer respond(Ok(...)) delivers either kind of domain outcome.
Process adapter must call release_reply and decode it; release() is insufficient.
Method/identity/epoch mismatch, malformed reply or lost transport receipt becomes
explicit uncertainty, never Joined. Public cloned-session close has a retained
local outcome and does not issue duplicate cleanup requests.

## Bounded admission, receipts and start/release races

Use a single runner admission step before spawning handler futures. It validates
provider/component/method/control lane and registers the lease state under a
short synchronous lock. Do not await native work while holding it. Read requests
in ComponentServer order; the transport paired-send fence makes a paired release
arrive only after the full open request. Therefore a release racing the *async
open handler* finds Preparing, even before native admission starts.

This design needs zero normal release-before-open tombstones. A release with an
unknown future epoch is an explicit UnknownLease protocol/domain failure and
must not later authorize/open that identity. If a future alternate dispatcher
cannot preserve admission order, it must add a separately bounded released-before-
open state and retain its fence until paired admission or provider shutdown;
never expire a tombstone into permission to start work. Do not copy migration's
asynchronous start-entry insertion into the new runner and assume ordering holds.

Maintain:

- <=16 active resource reservations including failed-but-unconfirmed work.
- Last admitted open epoch, checked monotonically. A duplicate/replayed epoch
  cannot reopen work after receipt eviction. Valid-identity rejected opens advance
  the watermark too and can store a bounded rejection receipt.
- <=64 terminal/rejected receipt cache entries, each with at most the existing
  bounded operation and cleanup errors; no native handles or query snapshots.
  Retire only after actual Joined, or keep Unconfirmed in active accounting.
- One first provider operation failure plus saturating omitted-failure count,
  not an unbounded history of error strings. Provider cleanup remains separate.
- <=32 ordinary plus <=32 control handler ownership inherited from transport,
  with per-lease operation slots underneath. JoinSet tasks are drained, not
  detached fire-and-forget jobs.

Repeated release while active/closing or cached returns its retained outcome.
After terminal cache eviction, return ClosedLease/UnknownLease without claiming
joined proof. Host close observers remain repeatable because they retain their
original outcome locally. This distinction must be documented in the SDK:
idempotent request side effects do not promise infinite remote receipt retention.
Never allow an evicted UUID+old epoch to create a new index. UUID reuse with a new
epoch is prohibited by the host SDK; full never-reused UUID checking independent
of monotonic identity would itself require unbounded worker history.

If no room remains for a valid-identity rejected-open receipt, return typed
ResourceExhausted/NotAdmitted and fence further open admission until cleanup
restores bounds. A lost rejected-open reply can then require provider recovery;
unknown release is not fabricated success. Close/release remains available at
lease/query capacity. Malformed identities cannot create retained metadata.

## Provider shutdown and failures

`file_search/shutdown` is reserved control with ProviderIdentity; successful
result is CloseOutcome. Fence all leases/admission first, wake polls, request
all backend closes, drain retained handlers/startups, and observe backend shutdown.
Do not await the runner JoinSet from inside its own shutdown handler: a dedicated
owner coordinates drainage and excludes the observer/control task awaiting the
receipt. Each request's response owner must still complete/drop before
ComponentServer::finish.

On transport EOF or explicit transport Shutdown, the runner executes the same
owner drain even when no domain shutdown observer remains. Preserve the first
operation failure and cleanup result. An explicit typed shutdown can report
operation failure + Joined; after it, transport finish can acknowledge orderly
process exit without hiding that earlier domain failure. An unconfirmed drain
must not emit a successful transport shutdown acknowledgement. Process adapter
combines the domain receipt with actual ComponentSession::close: lost/forced
transport termination cannot upgrade cleanup to Joined. No automatic reconnect,
replay or live implementation replacement behind existing leases.

All application failures use a domain envelope returned via respond(Ok(value)),
including ResourceExhausted. Reserve respond(Err(...)) for malformed protocol or
transport handling errors, with bounded static diagnostics. No arbitrary native
error strings are serialized. A locally downcastable PayloadLimitExceeded maps
to ResourceExhausted; request rejection proven before admission is NotAdmitted.
A reply/remote transport limit failure after admission still requires cleanup and
may invalidate every lease on that connection. Preserve this uncertainty even
when the operation category is ResourceExhausted.

## Factories and host composition

Proposed component crate entry points:

```
ProcessSearchBackend::connect(binding, initialization) -> Result<Arc<dyn SearchBackend>, SearchStartError>
run_stdio(factory: Arc<dyn SearchBackendFactory>, limits: ServiceCeilings) -> Result<(), ServiceRunError>
SearchBackendFactory::open(context: BackendContext, limits: NegotiatedLimits)
    -> Future<Result<Arc<dyn SearchBackend>, SearchStartError>>
BackendContext { base_dir: PathBuf }
```

These are proposal shapes, not implemented signatures. Factory startup is retained
through observer cancellation and receives typed context/policy, not Config or
Core pointers. Read plugin-specific configuration from SessionInitialization at
this boundary, with a validated native-plugin schema; never include configuration
or secrets in Debug/error output. No mutable global catalog/home cache.

The native executable composes its native backend factory with this generic
runner. The runtime crate loads one catalog snapshot and uses
catalog.selected("file_search", "default"); absent selection composes native,
invalid/failed selection fails explicitly. It owns provider/scopes and callbacks.
Embedded App Server and TUI receive the same provider after effective bootstrap;
remote TUI keeps its explicit local provider. See FILE_SEARCH_COMPOSITION_PLAN.

Manager generic one-shot Call must reject file_search alongside thread_store.
A later typed diagnostic can open/query/poll/release/shutdown through this facade.
SDK examples build separately and never depend on codex-core. Keep the private
storage search fallback named as pending until broker/dependency injection.

## Acceptance sequence and unresolved work

1. DTOs/path/integer/size/error conversion: actual serialization compatibility,
   rejected oversize/mismatched epochs/root escape/foreign paths. No mock quota
   owner tests presented as production proof.
2. Generic service with the actual native bounded backend: gated startup/release,
   poll/update saturation, abandoned observers, repeated close, failed-but-joined
   quota reuse vs unconfirmed quarantine, worker panic, clean/forced shutdown.
3. Actual stdio process/client: hold all16 polls and concurrent updates, prove the
   reserved release lane progresses; lost open/release replies, handler ordering,
   old epochs after receipt-cache eviction, malformed/oversize payloads, correct
   typed ResourceExhausted while storage large-history tests remain unchanged.
4. Relative/absolute/mixed/overlapping roots and duplicate roots against legacy
   behavior, same-OS non-Unicode transport, native UTF-8 skip policy, ignore rules,
   scoring/highlighting, empty query/roots, no silent option clamping.
5. Separately build native/custom workers outside source, install/select with
   unchanged host, actual App Server/TUI/GUI/headless functionality, scope
   isolation, restart selection, remove/upgrade/incompatible version/failure.
   Existing GUI tests must exercise the installed search provider, not native-only
   search behind a new wrapper. Preserve real Ctrl+C Launch shutdown proof.

Ready for implementation: DTO module boundaries, wire identities/envelopes,
limits, admission order, cleanup decoding and process factory seam.
Still required: transport cap/release_reply tests; actual bounded native backend
and measured budgets; relative-root native adaptation; retained service owner and
process adapter; runtime scopes and consumer injection; external build/install
and real-host/UI proof. No completed extraction claim is warranted at this point.

Existing plan text to reconcile during implementation: its earlier code snippets
still return Result<(), SearchError> for close/shutdown; later published neutral
API uses SearchCloseOutcome. It also describes already-fixed StageA connection
ownership and GUI absence in historical present tense. Correct those status
paragraphs without rewriting historical evidence.


# P02B admission ordering and per-provider cwd audit

Read-only source audit, 2026-10-01. No repository changes, builds, tests or Git
operations. Complements `/tmp/p02b-search-wire-plan.md`; these are implementation
requirements, not runtime proof.

## Recommendation: serialize transport admission, not startup completion

Use exactly one asynchronous admission mutex per ProcessSearchBackend connection,
shared by every clone and all runtime scopes. The guarded sequence is:

1. An owned startup record reserves provider/scope capacity before a task starts;
   <=16 Preparing/Ready/Closing/Unconfirmed leases exist in aggregate.
2. The retained startup task acquires the admission mutex. Public open observers
   do not own this task; their cancellation requests cancellation/cleanup.
3. Under the mutex, check the provider/lease cancellation fence, choose the UUID,
   increment the positive session epoch with checked arithmetic, build/validate
   bounded open and release DTOs, and await `start_with_cleanup`.
4. Keep the mutex across that await. It protects submission order, not just epoch
   allocation. As soon as admission returns, put both returned handles into the
   already-registered startup owner before the next await, then release the mutex.
5. Observe PendingComponentReply outside the gate. Observe or request release
   outside the gate. Never hold this gate through native startup, a reply wait,
   snapshot polling, query matching, release acknowledgement or provider shutdown.

A bounded single-opener actor can implement the same ordering, but a per-provider
mutex is the smaller change. Do not have separate gates per scope, nor atomically
allocate IDs before entering the gate. No bounded-hole protocol is needed while
the guarantees below hold. If future ordinary writes become concurrent, replace
the simple watermark with an explicitly negotiated bounded-hole/reorder protocol
or keep serialized send completion; do not silently retain the old watermark.

### Cancellation proof and code grounding

`component-host/src/session.rs::ComponentSession::start` does validation and all
awaits (cleanup-slot, cleanup-writer, regular-slot, regular-writer reservations)
before the pending-map insertion and Outgoing send. From that insertion through
return of the response/cleanup handles there is no await. Its public
`start_with_cleanup` likewise has no await between admitted result and returning
both handles.

Therefore, while the admission gate is held:

- Cancellation before transport admission drops its reservations and sends no
  request. The allocated epoch is a harmless gap. Do not wait for the absent
  epoch on the worker or create a tombstone for it.
- Once admitted, the retained owner already has/receives both handles in the
  same uninterrupted poll. Dropping an observing future must not abort this
  owner; it queues/awaits the pre-reserved release and retains the quota through
  its typed cleanup outcome. A panic/owner failure is unconfirmed and requires
  provider recovery, not a permit refund.
- Cancellation racing admission is decided by the retained task: if cancellation
  wins while start is pending, the transport contract says no request; if start
  returns handles, admission won and cleanup must be observed. Do not equate a
  public oneshot receiver disappearing with the transport admission outcome.
- Gate acquisition cancellation does not allocate an epoch or issue a request.
  Already-reserved Preparing metadata must still be finished under its owner.
- Close/poll/query and provider fencing must never need this mutex, otherwise
  saturated regular capacity can deadlock the release that would free capacity.

### Wire order was verified by source, not assumed

`session_wire.rs::write_messages` has exactly one `active: Option<Sending>` for
ordinary messages. It fully sends that message before receiving the next regular
message. Regular mpsc sends therefore retain FIFO order. Controls can interrupt
ordinary chunking, but DeferredControl's `after_sent` flag becomes true only after
its paired ordinary payload end and flush; a paired release cannot overtake its
own open. No ordinary round-robin/chunk interleaving exists in this implementation.

`session_server.rs::read_requests` receives complete messages sequentially, awaits
payload decoding, then sends each request to the bounded incoming queue in that
order. The search runner must synchronously register Preparing/high-water state
when it dequeues the request, before spawning the asynchronous backend handler.
Do not copy migration's entry insertion inside an asynchronously scheduled start
handler: that would reintroduce handler-reordering races.

Together these facts make a simple new-open epoch watermark sound. Epoch1 can
complete after epoch2; it was nevertheless admitted before2. Open replies can
arrive in either order and are matched to their retained startup identities.

**The watermark checks new opens only.** Updates, polls and release first look up
the exact live identity. An active epoch1 lease remains valid after epoch2 opens.
A late valid open reply for1 is not stale merely because2 is now the high-water
mark. Retired receipt eviction never allows an old epoch to reopen.

Release received after open dequeue but before async startup execution finds
Preparing and fences it. The owner either prevents native admission or joins
whatever the backend already accepted. Normal paired traffic needs no pre-open
tombstone. Unknown future release is rejected; never expire a pre-release fence
into permission to open that identity if a different dispatcher later needs it.

### Tests required with actual transport/service

- Saturate regular slots; start two opens concurrently. Observe that epoch2 is
  not submitted before epoch1 admission, while release controls still progress.
- Cancel1 while blocked before admission; admit2; worker accepts the epoch gap
  and sees no open/release for1.
- Cancel1 just after admission and before its reply; its release joins owned
  startup, and2 proceeds without waiting for1's native startup reply.
- Gate the first native startup, allow second startup to finish first; both
  results are accepted and the first remains usable after the higher watermark.
- Use a multi-chunk first open and small second open; assert admission FIFO and
  control pairing over the actual writer/server, not a fake ordering model.
- Release before asynchronous open handler execution finds Preparing; stale
  retries after bounded receipt-cache eviction never create new work.

## Recommendation: explicit immutable startup cwd on a dedicated worker

Current spawn contracts:

- Persistent `ComponentBinding::connect` delegates to `connect_with_limits` in
  `session_supervisor.rs`. Its retained startup task constructs plugin_command,
  then calls `.current_dir(&self.package_dir)` before spawn. ChildGuard/startup
  cancellation, process-group ownership and Linux parent-death behavior follow.
- `process.rs` one-shot/stream invocations independently use package_dir cwd.
- `broker_connection.rs` is another spawn implementation using package_dir; it is
  not currently exported by component-host lib.rs. Do not broaden this change
  into that unfinished route.
- Catalog bindings retain absolute canonical entrypoint/package paths. Do not
  mutate package_dir to fake a search cwd: package identity, asset location and
  filesystem authority are separate facts.

Smallest clean additive API:

```rust
#[derive(Clone, Debug, Default)]
pub enum SessionWorkingDirectory {
    #[default]
    PackageDirectory,
    ExplicitAbsolute(PathBuf),
}

#[derive(Clone, Debug, Default)]
pub struct ComponentSessionOptions {
    pub payload_limits: SessionPayloadLimits,
    pub working_directory: SessionWorkingDirectory,
}

impl ComponentBinding {
    pub async fn connect_with_options(
        &self,
        options: ComponentSessionOptions,
    ) -> Result<ComponentSession>;
}
```

Existing connect and connect_with_limits become backward-compatible wrappers
choosing PackageDirectory. No fields are added to ComponentBinding, so existing
literal construction/callers need no migration. No manifest/transport version
change is necessary. Keep one-shot/stream defaults unchanged. Search supplies its
chosen base_dir as ExplicitAbsolute before starting this dedicated provider.

Validate the explicit path is absolute before spawning. Clone the owned option
into the existing retained startup task and apply `Command::current_dir` exactly
once before exec. Do not call std::env::set_current_dir anywhere, do not alter
PWD/HOME, do not chdir per lease, and do not change an already running provider's
setting. Preserve existing direct-child/process-group/reaping behavior. Missing
or inaccessible startup cwd produces explicit startup failure; no package/native
fallback. This needs no additional crate dependency.

The wire initialization repeats the explicit base_dir and worker checks it against
its actual startup context before creating search work. The setting is immutable
for this provider; it is not a promise that another process cannot rename the
filesystem directory later. Search plugin assets use executable/package/module
locations, not implicit cwd. Configuration/state paths stay the existing absolute
host-provided values. Never print configuration in mismatch diagnostics.

Pass original lexical roots to the unchanged native walker/matcher. This preserves
relative traversal origin, deepest-root choice, score/highlight inputs and output
root labels. Simply absolutizing roots and mapping labels back cannot preserve
all of those facts. Pinned ignore0.4.25 WalkBuilder::current_dir configures global
ignore matching; it does not resolve relative traversal I/O.

Dedicated external providers can have different immutable cwd values concurrently
without sharing process state. A built-in in-process provider can preserve the
same semantics only for the actual host cwd snapshot; arbitrary different bases
need origin-path-view adaptation or a dedicated worker. Do not use temporary
host-global chdir for independent embeddings. Different homes need not imply
different cwd contexts. The base should reproduce the caller's actual previous
relative-I/O context, not silently substitute config.cwd for ambient cwd.

### Tests required before relative-root compatibility is accepted

- Existing persistent/default and one-shot plugins still observe package_dir.
- Two real provider processes use different explicit cwd values simultaneously;
  host cwd remains unchanged before/during/after, with independent shutdown.
- Compare actual legacy/native results against selected process results for
  `[".", "/cwd/sub"]`, reversed order, mixed absolute/relative overlap, duplicate
  roots, lexical `./`/trailing separators, symlinks and ignore files. Compare
  full result records, counts, scores, root/path choice and highlighting.
- Missing cwd, nonabsolute cwd, initialization cancellation and worker rejection
  produce explicit failures and joined/reaped or clearly unconfirmed cleanup.
- Do not infer compatibility from successful process startup or absolute-root-only
  tests. Original-path-view adaptation for differently based in-process providers
  remains separate unresolved implementation work.
