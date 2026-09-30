# Staged broker review and activation sequence

Updated 2026-09-30. All broker modules remain **unlinked and uncompiled**. The
previously verified 46-test host, storage and GUI checkpoint remains recorded;
the subsequent active persistent reap prerequisite passed its 52-test host gate. The changes below
are staged implementation and test source, not verified runtime evidence.

## Lifecycle and admission changes staged

- **Parent completion waits for reverse joins and physical flush.** `broker_completion.rs` retains
  the ordinary pending entry, its response and admission permit while the
  dispatcher revokes authority and joins that parent's handlers. The physical
  reader queues this bounded work and continues reading. Duplicate parent
  terminal frames are rejected; authority/context ownership is released only
  after joins and the reverse response's final physical flush acknowledgement.
  Queuing a response is not completion. Failed/dropped writer acknowledgements
  fail the parent instead of publishing its previously received success. Parent
  revocation and new reverse admission share one lock.
- **Dispatcher ownership survives cancellation.** Each monitor owns and awaits
  its actual handler JoinHandle. Active entries disappear only after that join,
  including destruction of the service future and the following writer ack.
  `cancel_all` signals cancellation
  without clearing the map or aborting decoder-owning handlers. Normal teardown
  joins reader tasks, cooperatively stops and joins the writer, reaps the child,
  drains handlers and parent
  completions, then releases contexts. A two-second forced teardown failure
  explicitly leaves joins/reaping and accepted outcomes unconfirmed.
- **Reverse request decoding is owned and cancellable.** `broker_decode.rs`
  retains the temporary spool and a blocking parser until its JoinHandle resolves.
  Cancellation is checked around 8 KiB reads; queued work is aborted and running
  work is signalled and joined. Parent completion, owner change and peer
  cancellation all follow that ownership path. An abnormal whole-runtime abort
  remains a forced fallback and cannot claim a successful join.
- **Uncapped serialization remains owned through cancellation.** `broker_spool.rs`
  checks cancellation at most every 8 KiB written and joins its blocking worker
  before returning. The broker writer uses it for ordinary and broker bodies,
  then reuses shared `Sending::from_payload` framing without another serializer.
  Pending spool reads finish before cancellation is observed; physical writes may
  be interrupted as a failed transport. Normal connection teardown signals and
  joins this writer instead of aborting it. A real IO/worker failure remains an
  error, while confirmed controlled cancellation ends the writer cleanly and
  fails every unflushed response acknowledgement.
- **Reverse request parameters have an explicit 8 MiB limit.** Completed spools
  above it return typed `request_too_large` before parsing or invoking a service.
  This bounds parser work; the current wire assembler still receives the complete
  spool before this check. Ordinary history and broker response bodies remain
  uncapped and continue using the shared fragmented spool transport.
- **Client admission is immediate and transactional.** Slot, cancellation and
  writer reservations use `try_*`; saturation returns `busy`, closed capacity
  returns `service_failure`, and no ID or pending state is consumed on rejection.
  Accepted responses and cancellation reservations remain supervised even when a
  waiter is abandoned or is not polled after its response arrives.
- **Authority negotiation preserves error distinctions.** An offered name with
  the wrong service version yields `unsupported_version`; a missing name yields
  `service_unavailable`. Revoking a connection rejects subsequent scope admission
  while retaining existing owner records until joined work is released.
- **Writer EOF remains observable at capacity.** Closed input lanes are checked
  independently of the cancellation wait branch and deferred-queue receive
  guards. Full deferred queues with impossible send fences return an error
  instead of waiting forever. Timeout-bounded tests cover both control and
  cancellation lanes. A racing cancellation cannot hide a joined serializer
  worker panic.

The new deterministic tests hold a parser read, serializer write, physical flush,
or service-future destructor open.
They assert that cancellation, parent response delivery and idle status cannot
complete until the gate is released and the worker/handler joins. The files now
contain 46 staged broker tests; the session activation patch adds another seven.
**None of these tests have been compiled or executed.** Only Rust formatting and
sequential application of all three activation artifacts to disposable copies
have run. This checks patch compatibility, not compilation or behavior.

## Unapplied activation artifacts

- [`broker-session-activation.patch`](broker-session-activation.patch): optional
  broker ownership in `SessionInner`, common persistent reentry guards, and
  `call_scoped`. Admission computes a candidate ID, performs fallible scope
  creation, then commits the ID, pending ownership and enqueue without an await.
  Its seven tests cover rejected scopes with unchanged IDs/capacity, held-session
  reentry, detached cleanup and accepted scope ownership after waiter cancellation.
  Detached `DeferredControl` remains unscoped and keeps its pre-authorized lane
  and request send fence. One-shot native auth remains permitted. Task-local
  guards cannot infer ancestry of arbitrary spawned helper tasks; leaf services
  must obey the persistent-callback restriction in those tasks too.
- [`broker-runtime-activation.patch`](broker-runtime-activation.patch): staged
  modules/exports and UUID v4 dependency, and `Payload::into_parts` for owned decoding.
  The explicit child-reap helper is already crate-visible in the active prerequisite. It does
  not supply the shared-codec extraction or missing server-side broker wiring.
- [`broker-spool-activation.patch`](broker-spool-activation.patch): private
  `Payload::from_parts` and `Sending::from_payload` constructors for already
  joined serialized bodies. Legacy header validation still runs before its old
  serializer. Apply runtime, session, then spool patches before extracting the
  shared codec; preserve these constructors in that extraction.

No patch is a standalone buildable activation. All three remain unapplied.

## Remaining activation work

0. Apply the verified active persistent reap prerequisite's startup ownership to
   the staged broker. The active implementation now retains
   its child outside cancellable protocol work and publishes failure after bounded
   kill/wait cleanup, or reports reaping unconfirmed. Dropping public `connect`
   signals an independent startup owner; it receives no cleanup acknowledgement.
   The complete host gate passed 52 tests, including actual-process startup,
   cancellation, worker-failure and shutdown PID-absence checks; scoped host/API
   Clippy passed. Evidence is in
   [`component-host-persistent-reap.json`](../verification/2026-09-30/component-host-persistent-reap.json).
   The staged broker's handshake-error path still requires this adaptation.
   Do not equate async reader/writer joins with
   ownership of the legacy codec's detached blocking serialization/decoding tasks.
1. Extract shared `session_wire` physical framing, `MessageAssembler` and
   `Sending` primitives without changing ordinary protocol behavior. Preserve
   direct raw-byte deserialization and duplicate-field rejection. Broker files
   currently reference these not-yet-exposed primitives.
2. Apply the session/runtime artifacts at the coordinated gate. Update Cargo
   lock and repository-required Bazel dependency/build metadata. Resolve compile
   issues and run all linked host and staged tests before claiming correctness.
3. Implement `ComponentServer::stdio_with_broker`, dependency client access and
   per-request scope access. Its dedicated reader/writer ownership must fail
   pending dependency waiters on EOF, parser error, writer failure and task
   cancellation. Keep plain `stdio` and `connect` compatible with old peers.
4. Review component-side reverse **response** decode ownership: the current
   `DependencyClient::call` still uses `Payload::into_value` after wire completion.
   Cancelling that caller can detach the blocking parser after wire pending
   capacity is released. Own/cancel/join that parse under component-side service
   supervision without imposing the request-size cap on responses.
5. Exercise actual bidirectional processes: old-peer rejection, simultaneous
   numeric IDs, all forward slots occupied, cancellation at saturation, wrong
   authority/session/parent/generation, owner changes during fetch/result delivery,
   parent response before reverse completion, EOF, forced close and graceful
   drain. Include failed teardown and child-reaping acknowledgement boundaries.
6. Activate native catalog adapters only after the transport gate. Preserve
   current-owner cache-load/fetch/write/TTL/invalidation/publication/read guards
   and typed domain failures, then independently build/install a native catalog
   plugin against a host whose binary hash remains unchanged.

The transport alone does not establish catalog/provider replacement. See
[the accepted design](DEPENDENCY_BROKER_PROPOSAL.md),
[the model-services plan](../MODEL_COMPONENT_PLAN.md), and staged package
activation notes for remaining native seams.
