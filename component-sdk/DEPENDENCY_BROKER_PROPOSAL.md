# Dependency broker proposal

Status: **design direction accepted; implementation in progress, not activated**. The implemented persistent
protocol is documented in [PERSISTENT_PROTOCOL.md](PERSISTENT_PROTOCOL.md). It has no
component-to-host calls. This proposal supplies the dependency boundary required by
[MODEL_COMPONENT_PLAN.md](../MODEL_COMPONENT_PLAN.md), initially for a native catalog
worker using the host's existing model endpoint.

## Scope and ownership

The host registers typed, allowlisted services with explicit integer contract versions.
A component declares required or optional dependencies; declaration is a request, not a
grant. Application composition supplies the actual service implementations and allowed
operations for that binding. Loading a manifest must not create an ambient generic HTTP,
credential, filesystem or other capability.

An opaque authority handle identifies one grant in a host-owned ledger. It binds the
plugin process/session, selected component binding, service/version/operations, actual
provider instance and its current credential/network authority. It cannot be reused in
another session and is never persisted as a reusable credential. The handle does not
authorize arbitrary destinations or headers. The host keeps live `AuthManager`, gateway
auth, `HttpClientFactory`, cookies and network-policy watches; none is serialized into
plugin configuration.

These contracts enforce dependency ownership and avoid accidental authority expansion.
Installed component executables remain trusted code with the runtime's existing OS
access; the broker is not an OS sandbox for malicious plugins.

## Backward-compatible negotiation

Keep `api_version:1` and the exact existing `session:{mode:"multiplexed",version:1}`.
Propose an optional top-level handshake extension:

```json
{
  "extensions": {
    "dependency_broker": {
      "version": 1,
      "max_in_flight": 16,
      "services": [
        {
          "name": "host.model_endpoint",
          "version": 1,
          "operations": ["fetch", "validate_owner"],
          "authority": "opaque-session-bound-handle"
        }
      ]
    }
  }
}
```

This object is added to `initialize`; it is not inserted inside `session`. `ready` must
explicitly acknowledge broker version 1 and the supported service/version/operation
subset. The host verifies acknowledgement against its offer and the component's required
dependencies before admitting application calls. A required unavailable service fails
activation with `service_unavailable`; an incompatible version fails with
`unsupported_version`. A peer that omits the extension has no broker, even if it accepted
the ordinary persistent session. Do not send extension frames to such a peer.

Existing `connect()` and `ComponentServer::stdio()` remain unchanged for broker-free
sessions. Proposed new entrypoints are `connect_with_broker(...)` and
`stdio_with_broker(...)`. Optional dependencies can be absent only when the component
declares and implements that mode. Missing required dependencies never trigger a native
backend fallback or an automatic restart into another protocol.

The extension version governs its added frame fields/types. An updated codec must reject
them when the extension was not negotiated. Existing version-one peers continue to use
their current strict frame schema unchanged.

## Request scope and authority checks

A service grant is necessary but insufficient to make a call. For each authorized host
request, the host creates a short-lived operation context and includes an optional
`broker_context` in that request's start frame, only after broker negotiation:

```json
{
  "handle": "opaque-operation-context",
  "request_generation": 7,
  "owner_generation": 12
}
```

The ledger binds this context to the parent host request ID, its unique request
generation, the selected provider, service grants, deadline, cancellation authority and
opaque provider/auth identity. The component can echo this context but cannot choose a
new owner generation, extend its lifetime or substitute a different parent. Generation
numbers are compared against host-owned state, never trusted as self-issued authority.
The accepted operation also retains a host-only typed request context. In particular,
catalog calls must capture their actual per-call `HttpClientFactory` and policy snapshot;
a startup factory or mutable connection-wide “current factory” would lose routing
semantics or race work whose waiter was cancelled. This host-only context is never
serialized or included in diagnostic output.

Owner generation follows the native identity rules: routing, account/user/plan and opaque
API-credential changes invalidate ownership. Stable ChatGPT token rotation alone does
not change catalog ownership. Transport/auth revisions can change without changing that
identity; keep those revisions distinct so connection reuse and credential freshness do
not accidentally control catalog cache validity. Discovery/picker policy booleans, such
as API-key opt-in, command authentication and Codex-backend eligibility, also belong to
the validated owner/configuration generation rather than an unversioned worker snapshot.

The host validates session, grant, service version, operation, parent request/context,
generation, deadline and live owner at admission. The service revalidates the live
authority when resolving credentials/acquiring its network permit, and after the async
operation before returning a result. Live network revocation must interrupt the real
request through the existing network-policy mechanism. A copied policy snapshot is not
an acceptable implementation.

Initially, background broker calls without an admitted parent request are unsupported.
A future background capability would need an explicit host-owned lifetime and budget;
an arbitrary parent ID or an old operation context cannot stand in for that grant.

## Wire directions and independent IDs

Broker requests originate in the component; ordinary requests continue to originate in
the host. Both sides may allocate numeric ID 1 at the same time. The lookup key is the
request-origin namespace plus ID, never the number alone. Keep separate allocation,
pending, assembly and duplicate-detection state for ordinary and broker messages.

Proposed frames:

| Frame | Direction | Fields after `type` |
| --- | --- | --- |
| `broker_request_start` | Component to host | `id`, `service`, `version`, `method`, `authority`, `context`, `parent_id`, `request_generation`, `expected_owner_generation`, `bytes` |
| `broker_result_start` | Host to component | `id`, `bytes` |
| `broker_chunk` | Direction of its broker start | `id`, `index`, `data` |
| `broker_end` | Direction of its broker start | `id`, `chunks` |
| `broker_error` | Host to component | `id`, `code`, `message` |
| `broker_cancel` | Component to host | `id` |

The broker-specific frame types prevent collision with ordinary request/result chunks
sharing an ID on the same pipe. They reuse the existing 4 MiB physical frame and 192 KiB
decoded chunk limits, standard base64 and exact byte/chunk counts. Each successful body
is one JSON value. There is one terminal result or error per accepted broker ID.

`broker_cancel` refers only to the sender's own broker call in this connection. It does
not cancel the parent host call, change authentication state, terminate another plugin
or release unrelated resources. It receives no independent request ID: cancellation
eventually produces the target call's terminal error, unless completion already won the
race. Late cancellation of an already completed valid ID is harmless; cancellation of
an ID never admitted to this connection is rejected. Keep bounded completion/range
tracking rather than an unbounded cancellation history.

Framing violations are connection failures. Ordinary service errors are correlated
typed broker failures and do not tear down a healthy session. Error codes initially
include `service_unavailable`, `unsupported_version`, `authority_revoked`,
`owner_changed`, `cancelled`, `deadline_exceeded`, `busy`, `invalid_request` and
`service_failure`. Messages are static or reviewed for secret-safe content. Provider
diagnostics must not serialize raw credentials, headers, credential-bearing URLs or
response bodies. This does not reinterpret existing `model_transport` version-one errors,
which retain their current terminal `InvalidRequest` mapping.

## Capacity, progress and reentrancy

Start with 16 outstanding broker calls per session, independent of the existing 32
ordinary and 32 cleanup reservations. Reserve each call's completion ownership and one
coalesced cancellation slot before admission; cancellation must not wait for ordinary
request capacity. Bound writer queues, active broker assemblies and decoded request
dispatch to the same negotiated broker capacity. Cancellation flags are idempotent and
bounded by outstanding calls, not an unbounded notification queue. Resource saturation
waits within the caller's deadline or returns `busy`; it does not silently allocate more
capacity.

Use one owned reader task per pipe to demultiplex both namespaces. Do not block that
reader on executing a service or on ordinary-call capacity: an ordinary plugin handler
may be waiting for the reverse response. Reader-owned bounded dispatch and pre-reserved
completion capacity must keep cancellation and responses flowing. A writer schedules
small cancellation/control frames ahead of ordinary chunks; responses need a separately
reserved lane so all 32 forward calls can wait on broker work without deadlock. None of
these operations runs while holding the pending-map mutex.

Initial broker services are non-reentrant leaves. `host.model_endpoint.fetch` can call
the native endpoint and its lower-level host dependencies, including existing
`AuthManager` resolution through a separate per-call auth component. It cannot call back
into the requesting persistent plugin or invoke a dependency that can re-enter the same
operation. Track operation ancestry and reject such cycles explicitly. Supporting other
nested component dependencies
later requires a validated acyclic dependency graph, bounded depth and per-edge capacity;
do not rely on spare queue capacity as a reentrancy policy.

The count and frame limits bound protocol buffering, not all application allocations or
temporary-file bytes. Logical payloads keep the existing spooling semantics. Service
limits, including the native explicit-catalog download cap, remain in force.

## Cancellation and close lifecycle

A persistent call waiter disappearing must not revoke accepted work implicitly. Its
operation context belongs to the session's supervised pending request, not to the
caller's temporary future. Each broker operation declares cancellation behavior. The
initial catalog fetch and owner validation are read-only and cancellable; future accepted
durable-write services require a separate outcome/cleanup contract before registration.

The plugin may cancel its own read-only broker request. The host can revoke its context
on owner changes, explicit authorized operation cancellation, deadline expiry or plugin
disconnect. These actions cancel only operations authorized by that context. A lost
response can still leave a provider request's completion uncertain; there is no automatic
replay or native fallback.

Graceful close stops new forward-root admission but keeps broker contexts available for
already accepted parent requests while they drain. Reject new background work. Drain
outstanding broker work and its responses before the corresponding parent can complete,
then revoke those contexts, perform ordinary shutdown/acknowledgement and exit. Closing
the broker before draining its parents would deadlock them. A parent returning while it
still owns broker work must have that work cancelled and joined before releasing its
context. Forced close/disconnect revokes all session grants, cancels broker handlers and
discards late completions; existing process-group termination/reaping remains responsible
for the child. No service survives through a hidden reconnect.

## First service: native model endpoint

Register `host.model_endpoint`, version 1, scoped to a specific existing provider
endpoint. The first two operations should be:

| Operation | Input | Output |
| --- | --- | --- |
| `fetch` | Client version; expected owner generation comes from the validated broker context | Native models, optional ETag, actual opaque identity, captured owner generation and current owner generation |
| `validate_owner` | Expected opaque identity and owner generation | Confirmed identity/generation or `owner_changed` / `authority_revoked` |

`fetch` accepts no URL, destination override, headers, bearer token or serialized
`AuthManager`. The grant fixes the endpoint and its policy. It retains native command-auth
precedence, ephemeral external credentials, gateway composition, request-time routing,
redirect policy, cookie state, telemetry redaction and `NetworkPermit` revocation. An
external custom provider does not acquire first-party credentials merely by advertising
an OpenAI-shaped API.

There are three separate owner checkpoints:

1. Validate before/after the real endpoint fetch, returning the identity of the credentials
   actually used.
2. Validate again after the worker's asynchronous cache write and before its catalog
   publication. The existing synchronous `ModelsEndpointClient::identity()` snapshot cannot
   implement a live cross-process check. Add a small async validation seam to the native
   manager, backed by a local check in-process and `validate_owner` in the worker. A stale
   cached auth snapshot or fetch-time generation alone does not preserve this behavior.
3. Before accepting the returned snapshot, the host atomically checks the generation and
   identity against current ownership while updating its visible catalog. A validation
   response is not a lease that freezes the owner. Cache entries stay identity-scoped and
   may never be reused for a newer owner just because they were written successfully.

Static/offline catalog operations need no network broker call. Current-account revocation
and authoritative-catalog invalidation still apply to cached data. Owner changes during
cache load, fetch, cache write or publication must not expose the old owner's catalog.
Apply the same owner checks to cache-load publication, matching-ETag TTL renewal and
explicit-provider fetch-failure invalidation. Invalidation for an old owner must not
clear a newer owner's successful refresh. Reads of the host snapshot must remain
owner-scoped, including after publication, because account changes do not wait for a
worker's acknowledgement. The native regression anchor is
`account_switch_during_cache_store_preserves_new_catalog_for_next_turn` in
[`injected_models_cache.rs`](../codex-rs/core/tests/suite/injected_models_cache.rs).

The native catalog package can then reuse `OpenAiModelsManager` with an RPC endpoint
adapter. This extracts catalog policy/cache behavior while endpoint/auth/network remain
declared dependencies. It does not complete native provider or inference extraction.

Provider extraction needs a later versioned service family for owned capability metadata,
routing decisions and credential/network-backed endpoint operations. Keep capability
upper bounds enforced by the host and bind resolved routes to authority revisions. Use
typed operations per provider/transport; a generic arbitrary-URL request proxy would lose
the intended boundary. Native inference HTTP/WebSocket reuse, authentication recovery,
Bedrock signing, realtime and richer retry/error mapping need their own agreed contracts.

## Proposed Rust surface and acceptance gate

The following names describe ownership, not implemented API signatures:

- `HostDependencyRegistry`: explicitly registered typed service/version implementations.
- `HostAuthorityGrant`: private host state and a redacted opaque wire handle.
- `HostOperationContext`: admitted parent ID/generation, current owner authority, deadline,
  revocation and cancellation behavior; created by host composition.
- `ComponentBinding::connect_with_broker`: negotiate only the binding's allowed grants.
- `ComponentSession::call_scoped`: admit a normal request while associating its host-owned
  operation context; preserve ordinary accepted-work ownership on waiter cancellation.
- `ComponentServer::stdio_with_broker` and a cloneable `DependencyClient`: services invoke
  `call(context, authority, method, params)` while the independent reader remains active.
- `BrokerFailure`: typed, secret-safe service failures distinct from framing failures.

Agree concrete DTOs and lifetimes before implementing these alongside the model-service
factory. Required tests include old-peer negotiation failure, equal numeric IDs in both
directions, wrong-session/grant/parent/generation rejection, token rotation versus owner
change, owner switches during fetch and cache write, live network revocation, custom-URL
and credential-authority isolation, no-fetch offline mode, no fallback, progress with all
forward slots occupied, cancellation at saturation, reentrancy-cycle rejection and
graceful/disconnected shutdown while both directions have pending work. Independently
build/install the native catalog package and prove its real catalog behavior with an
unchanged host binary before claiming that extraction complete.

## Staged request parsing limit and cancellation ownership

The initial broker contract caps reverse request parameter JSON at 8 MiB and
returns the typed `request_too_large` error before decoding or invoking a host
service. The current staged receiver checks after the complete private spool
arrives. This limits parser work; it does not impose a cap on ordinary histories
or broker responses. Parsing checks cancellation around 8 KiB reads, and a
cancelled parser remains owned until its blocking worker joins. Parent terminal
response delivery must retain its admission slot and owner context until every
reverse handler for that parent has joined and its response's final physical
frame is flushed. Unflushed response failure fails the parent. Response queue
admission alone cannot release ownership. Uncapped request/response serialization
in the broker writer uses cancellable 8 KiB writes and a joined blocking worker;
normal writer shutdown signals cancellation and joins, rather than aborting that
owner and detaching serialization. Failed forced cleanup remains explicitly
unconfirmed.

Implementation remains unlinked and untested; see [the activation review](BROKER_REVIEW.md)
for staged fixes, unapplied patches, and remaining response-decoder/server work.
