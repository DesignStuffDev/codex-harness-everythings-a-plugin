# Model services: proposed component contracts

Status: implementation plan, not implemented capability. This document records the next
model-service boundaries after the current component milestone. Source references refer to
the imported OpenAI Codex revision recorded in [UPSTREAM_PROVENANCE.md](UPSTREAM_PROVENANCE.md), with local changes
identified below. Agree the shared broker and service DTOs before parallel implementation.

Implementation staging now exists in new, unlinked sources:
[catalog DTOs](codex-rs/model-catalog-api/src/lib.rs),
[host manager adapter](codex-rs/models-manager/src/manager/component.rs),
[owner-fenced mirror](codex-rs/models-manager/src/manager/component_mirror.rs),
[native service](codex-rs/models-manager/src/manager/component_service.rs), and
[standalone native worker](codex-rs/model-catalog-native-plugin/src/lib.rs).
The [minimal manager activation patch](verification/activation-patches/model-manager-component.patch)
is prepared but not applied. These sources have not been compiled or behaviorally verified;
they are not an active installed capability until the next source/acceptance gate.

## Current behavior and existing seams

- [The current model adapter](codex-rs/core/src/component_model.rs) replaces one inference
  stream through `model_transport:default`, contract version 1, method `model.stream`.
  [ModelClientSession::stream](codex-rs/core/src/client.rs) constructs the native normalized
  request and preserves model-request contributors, response interceptors, tracing and
  native event processing. A selected component suppresses native inference/prewarming;
  errors do not trigger a native-backend fallback. Cancellation drops the invocation.
- **Current error behavior:** every component/protocol failure becomes a terminal native
  `ApiError::InvalidRequest`. The current contract does not preserve the full native retry,
  authentication recovery, rate-limit or context-window error taxonomy. The richer error
  contract below is planned. It must not silently change existing version-one plugins.
- [ModelsManager, ModelsEndpointClient and RefreshStrategy](codex-rs/models-manager/src/manager.rs)
  already separate catalog policy from provider-specific fetching. `ModelsManager` owns
  refresh policy, catalog merging, synchronous cached reads, picker filtering and model
  metadata lookup; `ModelsEndpointClient` owns discovery authentication and transport.
- [ModelsCache](codex-rs/models-manager/src/cache.rs) already exposes load, store and
  conditional TTL renewal. Native entries carry client version, provider/auth identity,
  ETag and freshness. The default file cache is `models_cache.json`, with a 300-second TTL.
- [ModelProvider](codex-rs/model-provider/src/provider.rs) already owns capability bounds,
  preferred background models, account state, routing, authentication recovery and model
  manager construction. [Configured providers](codex-rs/model-provider/src/provider.rs)
  and [Bedrock](codex-rs/model-provider/src/amazon_bedrock/mod.rs) implement different
  capabilities, catalogs, signing, region resolution and recovery behavior.
- [Core catalog construction](codex-rs/core/src/thread_manager.rs) and
  [ModelClient construction](codex-rs/core/src/client.rs) currently construct provider
  instances separately. A provider replacement must reach both paths consistently.

These Rust traits and crates are useful boundaries. Their current existence does not make
the native implementations independently installable process plugins.

## Composition and ownership

Add an asynchronous `ModelServices` composition factory with explicit provider, catalog,
inference and realtime dependencies. Select components before constructing native
implementations. Retain existing constructors as native compatibility adapters, following
[the persistence composition pattern](codex-rs/core/src/component_persistence.rs).

The host owns session/turn orchestration, prompt construction, approvals, model-config
overrides, credential authority, managed network policy and user-visible event processing.
Each service owns its provider-specific state and lifecycle. Native implementations should
be separately buildable packages using the existing lower-level crates; they must not
launch an unchanged Codex harness as a substitute for extracting the subsystem.

Synchronous methods need a validated host snapshot. Initialize it before exposing a
selected component; refresh it atomically after successful asynchronous calls. Infallible
legacy manager methods must not hide selected-component startup errors. A fallible service
factory and explicit runtime health/error path are needed before wiring the new adapter.

## Catalog contract, proposed version 1

Proposed capability: `model_catalog:default`. A persistent session owns the catalog manager,
cache and refresh coordination. Inputs are owned DTOs; no `Config`, `AuthManager`,
`HttpClientFactory`, Rust lock or database pointer crosses the boundary.

| Operation | Input | Output / effect |
| --- | --- | --- |
| `model_catalog/open` | Contract version, source policy (`bundled_merge`, `authoritative`, `static`), optional configured catalog, client version, cache policy, discovery policy, auth context | Initial catalog snapshot and negotiated capabilities |
| `model_catalog/refresh` | `online`, `offline` or `online_if_uncached`; current auth context | Catalog snapshot or typed failure |
| `model_catalog/refresh_etag` | Observed ETag and current auth context | Renew matching cache TTL or refresh; return snapshot |
| `model_catalog/snapshot` | Expected identity/revision | Current snapshot without network access |
| `model_catalog/close` | No new work admitted | Finish accepted work and close owned resources |

`CatalogSnapshot` contains the native `Vec<ModelInfo>`, optional ETag, opaque provider/auth
identity, revision and status. `CatalogAuthContext` contains owner generation, auth mode,
the opaque identity and the booleans needed by native picker/discovery policy. It contains
no access token, API key or arbitrary serialized auth object. Refactor the existing small
catalog dependency on `AuthManager` into a read-only auth-context interface that supports
both the native in-process manager and a worker snapshot.

Preserve these source-backed rules:

- A configured static catalog is authoritative and bypasses cache/remote fetching.
- An explicit provider catalog is authoritative, clears bundled seeds, uses exact model
  matching, serializes refresh and invalidates stale data on fetch failure.
- The default catalog retains its existing bundled/remote merge behavior.
- API-key discovery must remain enabled and supported before remote cache reuse. Command
  credentials retain their existing precedence. See
  [API-key discovery tests](codex-rs/models-manager/src/api_key_discovery_tests.rs).
- Cache entries must match current identity and client version; invalidated entries stay
  invalid even if durable invalidation failed. A response must still match the CURRENT
  identity after asynchronous fetch and cache writes before the host publishes it.
- Identity changes include provider routing, account/user/plan and opaque API credentials.
  Native identity generation deliberately does not invalidate stable ChatGPT ownership on
  token rotation alone. Reuse [the native identity logic](codex-rs/model-provider/src/models_identity.rs).
- External cache storage is scoped to its component configuration and provider/auth
identity. It must not overwrite an unrelated native cache. Native file-cache compatibility
and any migration must be explicit.
- Offline means no fetch. A failing selected component must never trigger discovery or
  inference against another backend. Same-owner last-good data is allowed only under the
  negotiated source policy; authoritative failure must not resurrect bundled models.

The host applies existing [model-info/config adjustments](codex-rs/models-manager/src/model_info.rs)
to the accepted snapshot, keeping native context-window limits, reasoning, instructions,
truncation and runtime selectors intact.

Configured static catalogs remain pinned user configuration. Native provider static seeds
(such as Bedrock's bundled catalog) use a separate `provider_static` source: the native
worker preserves its static behavior, while custom components can replace those seeds.
Domain endpoint failures retain native source policy: authoritative failure clears its
catalog/ETag, and ordinary same-owner endpoint failure is distinct from broken component
health. IPC failure, malformed snapshots and ownership failures fail explicitly.

## Native dependency broker prerequisite

The [dependency broker proposal](component-sdk/DEPENDENCY_BROKER_PROPOSAL.md) specifies
the proposed bidirectional framing, host-issued authority handles, cancellation and
publication guards. It is a contract proposal, not a currently implemented host feature.

A faithful native catalog worker cannot simply reconstruct HTTP/auth from disk.
[The native endpoint](codex-rs/model-provider/src/models_endpoint.rs) resolves credentials,
gateway auth and request-time routing policy. Ephemeral external credentials may never
exist in `auth.json`. [HttpClientFactory](codex-rs/http-client/src/outbound_proxy.rs) carries
cookie/proxy state and [NetworkPolicy](codex-rs/http-client/src/network_policy.rs) has live
revocation watches. Copying a configuration struct loses those behaviors.

Add typed, allowlisted dependency calls from component to host with separate request-ID
direction/namespace, cancellation semantics, bounded queues and secret-safe diagnostics.
The first dependency can be deliberately narrow:

```text
host.model_endpoint.fetch {
  provider_handle, client_version, expected_owner_generation
} -> {
  models, etag, actual_identity, current_owner_generation
}

host.model_endpoint.validate_owner {
  provider_handle, expected_owner_generation, expected_identity
} -> {
  matches, current_owner_generation
}
```

The generation returned by `fetch` cannot validate an asynchronous cache write that
finishes later. Owner validation is also required for cache-load publication, matching-ETag
TTL renewal and explicit-provider failure invalidation. A remote validation reply still
has a post-reply race: the host must atomically guard snapshot publication and cached reads
against its current owner generation. Discovery capability booleans belong to that same
generation. Preserve these checks when replacing the native synchronous identity read;
do not treat a successful earlier dependency call as a permanent ownership guarantee.

The opaque handle binds an existing provider endpoint and current host credential/network
authority; it is not permission to attach first-party credentials to an arbitrary URL.
The native catalog package can then reuse `OpenAiModelsManager` with an RPC endpoint
adapter. An external catalog implementation may provide its own catalog without using
this dependency. Endpoint/network implementations remain explicit dependencies with their
own future replacement contracts, rather than undocumented access to the harness.

```mermaid
flowchart LR
  H[Engine model-services adapter] --> C[Catalog component]
  C --> B[Typed dependency broker]
  B --> E[Provider endpoint]
  E --> A[Credential and network authority]
```

The persistent component transport added for storage is a useful foundation, but these
server-to-host dependency calls are a new contract. Do not assume unary session support
already supplies them. Dedicated readers must remain cancellation-safe while calls in
both directions and cleanup controls are active.

## Provider and inference contracts

Proposed provider operations are `describe` and `resolve`. `describe` returns owned
provider metadata, capability upper bounds and preferred model strings; do not allocate
permanent strings to satisfy native `&'static str` helpers. `resolve` receives service kind,
session identity, owner generation and previous-route state, and returns a route capability
bound to an authorization authority. Define route/auth ownership before implementation.

Preserve [workspace routing validation](codex-rs/model-provider/src/workspace_routing.rs),
managed residency, redirect rejection, gateway authentication, Bedrock signing/recovery
and [custom-provider auth separation](codex-rs/model-provider/src/auth.rs). Socket reuse
must depend on effective destination, routing header and auth revision; account switches
must invalidate prior-owner connections. Custom provider components use component-owned
credentials unless an explicitly scoped native authority dependency is negotiated.

The separately packaged native inference implementation should own HTTP/WebSocket
transport, warmup, connection reuse and session-scoped HTTP fallback. Turn-local sticky
routing and incremental-request state must reset at the same native boundaries. Keep the
existing request/response-event schema and native request contributors/interceptors.

Planned richer errors for the new service contracts distinguish authentication failures,
retryable transport failures, rate limits/retry-after, request timeout, context-window
overflow, model-not-found, safety routing and invalid requests. Their adapter must preserve
native recovery decisions and exhaustion limits. Broker transport errors remain redacted
and generic; endpoint service payloads need typed domain outcomes so an authentication or
rate-limit failure is not reduced to a generic `ServiceFailure`. If this extends the existing
`model_transport` contract rather than introducing a new service contract, negotiate a new
contract version; existing version-one failures remain terminal `InvalidRequest`.

## Unary and realtime work still required

- [ModelClient::summarize_memories](codex-rs/core/src/client.rs) is a unary
  `/v1/memories/trace_summarize` operation. A `memory.summarize` contract can reuse native
  `ApiMemorySummarizeInput` and output DTOs, with explicit auth/routing/error ownership.
- [Remote compaction V2](codex-rs/core/src/compact_remote_v2.rs) already calls the selected
  streaming seam. Compaction policy, history replacement and validation remain separate
  native owners; transport replacement is not compaction extraction.
- [Realtime conversation](codex-rs/core/src/realtime_conversation.rs) constructs its own
  `RealtimeWebsocketClient` and API provider. WebRTC call creation and sideband auth also
  have separate methods in `ModelClient`. A duplex realtime contract needs start, send,
  ordered events, configuration update, cancellation and close, with one transport owner
  for SDP, call ID, sideband authentication, reconnect and audio buffering. A unary text
  adapter does not cover this behavior.
- Authentication/account display and [provider OAuth notifications](codex-rs/app-server/src/gateway_oauth_notifications.rs)
  also construct providers outside the main inference path. Inventory and route these
  through shared composition before claiming complete provider replacement.

## Acceptance and regression gates

Build the native implementation and a custom component outside the harness checkout.
Install/activate each without rebuilding the host, verify real functionality, remove or
deselect it, and verify native recovery. Verify the host executable is unchanged.

Required behavioral cases:

1. Custom catalog changes actual default-model choice, model instructions, context limits,
   tool/runtime capabilities and inference routing; it does not merely change a picker.
2. Offline performs zero fetches; ETag renewal, expiry and client-version checks hold.
3. Account/provider switches during fetch and during cache writes never publish prior-owner
   metadata. Static and explicit-provider catalogs remain authoritative.
4. API-key discovery opt-in, command-auth precedence, longest-prefix versus exact matching,
   configured limits and empty-catalog handling preserve native behavior.
5. Custom destinations never inherit unrelated first-party credentials. Managed routing,
   redirect rejection and owner changes affect both HTTP and WebSocket connections.
6. Native streaming, tool calls, approvals, cancellation, resume, remote compaction,
   typed retry/auth recovery, memory summaries and realtime are tested as each seam moves.
7. Failed startup, incompatible versions, interrupted dependency calls, process death and
   cleanup under full queues fail explicitly without backend substitution or stale-owner reuse.

Existing regression sources include
[catalog/cache tests](codex-rs/models-manager/src/manager_tests.rs),
[identity races](codex-rs/models-manager/src/cache_identity_tests.rs),
[provider endpoint tests](codex-rs/model-provider/src/models_endpoint.rs),
[provider identity tests](codex-rs/model-provider/src/models_identity_tests.rs),
[workspace routing tests](codex-rs/model-provider/src/workspace_routing_tests.rs),
[remote model behavior](codex-rs/core/tests/suite/remote_models.rs),
[auth rotation](codex-rs/core/tests/suite/models_cache_auth.rs),
[TTL](codex-rs/core/tests/suite/models_cache_ttl.rs),
[ETag refresh](codex-rs/core/tests/suite/models_etag_responses.rs),
[injected-cache races](codex-rs/core/tests/suite/injected_models_cache.rs),
[runtime selectors](codex-rs/core/tests/suite/model_runtime_selectors.rs), and
[the current installed-model integration tests](codex-rs/core/tests/suite/component_model.rs).
