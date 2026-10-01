# P02 Stage B: one search composition for an embedded App Server and TUI

Audit date: 2026-10-01. Source checkpoint: `17c366bb94f77b3d3895acd6f061a5cb308bd384` (unverified P02 WIP), derived from upstream `d42056091aded7feb1d88ac7e83972108b2aa478`. These are proposed Stage B changes, not installed-component proof. Read with [FILE_SEARCH_COMPONENT_PLAN.md](FILE_SEARCH_COMPONENT_PLAN.md).

Read-only design against the current primary source. No API below is implemented
by this document. Stage A remains native lifecycle repair; its evidence does not
prove the shared selected provider proposed here.

## Recommended boundary

Create one `FileSearchProvider` per **embedded App Server runtime instance**,
after effective embedded policy/bootstrap is resolved. Pass it into the request
processor and expose only a scope-creation capability to local clients. Obtain
the TUI picker's scope from the final embedded client immediately before
`App::run`. App Server connection scopes and that picker scope then share one
selected backend, one package/selection snapshot, and one aggregate budget.

Do not use a static singleton, a process-global Codex home, or a cache keyed only
by home. Two embeds, including two different homes in the same process, own
independent compositions. Even equal homes do not imply common lifecycle
ownership. The aggregate quota is global to one explicit composition, across its
connections and local picker; it is not a claim of an OS-wide quota across
arbitrarily many independent embedding instances. A future embedder requiring
that wider budget can inject a shared parent budget explicitly without sharing
backend selection or using ambient globals.

## Actual startup and ownership locations

1. `codex-rs/app-server-client/src/lib.rs`:
   `InProcessAppServerClient::start` translates `InProcessClientStartArgs` using
   `into_runtime_start_args`, awaits `codex_app_server::in_process::start`, and
   moves the returned low-level handle into its facade worker.
2. `codex-rs/app-server/src/in_process.rs::start` calls `start_uninitialized`,
   performs initialize/initialized, and explicitly shuts the client down if
   initialization fails. `start_uninitialized` builds `ConfigManager`, then
   awaits `bootstrap::configure` before persistence and task startup.
3. `codex-rs/app-server/src/in_process_bootstrap.rs::configure` loads effective
   requirements and updates the passed `Arc<Config>` with effective auth/network
   policy. Select search from this runtime's `args.config.codex_home` and resolved
   catalog/policy **after this step**. Do not independently resolve home from an
   environment variable or start the worker using pre-bootstrap permissions.
4. Add the selected provider to the internal `MessageProcessorArgs` in
   `message_processor.rs`; `MessageProcessor::new` is synchronous and currently
   calls `SearchRequestProcessor::new(outgoing.clone())`. It must consume the
   already-composed provider rather than perform asynchronous selection or
   independently build another backend.
5. `SearchRequestProcessor` keeps its existing connection identity/fence maps,
   but uses `provider.new_scope` for each registered connection. Remove native
   construction from `SearchConnectionState::default`. Scope creation occurs
   under the existing admission fence and does no process startup, RPC or await.
   A bounded `OnceLock`/optional scope in connection state can avoid changing
   every transport's `ConnectionSessionState::new` call; registration assigns it
   once and rejects a mismatched provider/instance. Alternatively inject the
   scope at construction, but do not let `Default` silently allocate a backend.
6. The low-level runtime and its returned handle need search shutdown guards,
   analogous in purpose to their separate `StoreShutdownGuard` instances:
   destroying a public handle must fence the provider even when detached RPCs
   retain Arcs. Accepted startup/cleanup stays with the provider owner until it
   joins; factory/scope references cannot prevent the public-owner drop action.

The exact fallible provider creation location is between successful embedded
bootstrap and `MessageProcessor` task spawn. Pair it with an armed startup owner
before any later fallible operation or await. If persistence or installation-ID
setup follows provider startup and fails, explicitly await provider close and
combine cleanup errors with the original error. If selection instead follows
persistence, do the analogous joined storage cleanup on selection failure.
Merely dropping a guard is a fence, not a successful cleanup receipt.

## Minimal additive local-client API

Retain existing public `InProcessStartArgs`, `InProcessClientStartArgs` and their
`start` signatures; adding a required field to these public struct literals
would force unrelated embedders to change. No JSON-RPC DTO or initialize
capability needs changing to share an in-process service.

Add a small restricted factory in the planned `codex-file-search-runtime` crate:

```rust
// Proposed signatures only; names may be adjusted with the shared runtime API.
#[derive(Clone)]
pub struct FileSearchScopeFactory { /* provider incarnation, weak owner */ }

impl FileSearchScopeFactory {
    pub fn new_scope(&self, limits: ScopeLimits)
        -> Result<FileSearchScope, SearchError>;
}

impl FileSearchProvider {
    pub fn scope_factory(&self) -> FileSearchScopeFactory;
}

impl InProcessClientHandle {
    pub fn file_search_scope_factory(&self) -> FileSearchScopeFactory;
}

impl InProcessAppServerClient {
    pub fn new_file_search_scope(&self, limits: ScopeLimits)
        -> Result<FileSearchScope, SearchError>;
}
```

The factory has no provider-shutdown, catalog-reload or backend-replacement
operation. It does not keep public provider ownership alive: stale factories
fail as closed, and shutdown atomically fences factories before cleanup. Its
creation is local and bounded; scopes still enforce the aggregate provider
limits. It cannot allocate a fresh provider if the old one has stopped.

Implementation flow: store a factory privately on `InProcessClientHandle`;
`InProcessAppServerClient::start` clones it immediately after low-level start,
before moving the low-level handle to the worker, and stores it privately on
the high-level client. Its new scope method delegates to that factory. Update
the high-level client's explicit shutdown destructuring and test-only fake
handle literals for the new private field. This avoids a new queued facade
command, extra RPC surface, changes to external start arguments, and retention
of an entire server client by the TUI search manager.

For test dependency injection, an additive `start_with_services` overload can
be considered separately if needed, but is not necessary for production sharing.
Do not make callers construct a backend before the existing effective-policy
bootstrap merely to pass a provider through another argument.

## TUI integration and startup restarts

The correct TUI adoption point is
`codex-rs/tui/src/lib.rs`, immediately before the existing
`let file_search_runtime = file_search::FileSearchRuntime::new()` and
`Box::pin(App::run(...))` near line2034. Replace that constructor with a fallible
scope-based constructor and keep the existing post-App drain.

The TUI starts its initial server much earlier in `run_ratatui_app` through
`start_app_server`. Onboarding/config changes can shut that server down and
start another at the provider-change branches near lines1757 and1855. Therefore
do not retain a picker scope from the first startup server. Obtain it from the
final `AppServerSession` that is actually passed to `App::run`.

Add a narrow `AppServerSession` method which delegates to the in-process client
when `uses_embedded_app_server()` is true; it returns `Some(scope)` only for that
variant and propagates scope-creation failures. A closed/incompatible embedded
provider is an error, never a reason to create a fallback native owner.

`FileSearchRuntime` changes from owning `FileSearchOwner` to owning a
`FileSearchScope` plus its existing TUI task tracker. Its shutdown joins only its
scope and callbacks; it must not stop the shared App Server provider. Preserve
manager/session/query delivery generations, weak reporters, clear/cwd/drop
handling and the outer runtime's cleanup on every post-construction error path.
Handle failure to acquire the scope with `shutdown_startup_session` so the
already started embedded App Server is not abandoned by an early `?`.

`AppServerBootstrap` in `tui/src/app_server_session.rs` is RPC-derived UI/model
metadata (account, models, timing and collaboration modes). Do not put a live
provider or lifecycle owner there: it is not the composition bootstrap and it
also exists for remote clients.

`tui/src/app/reconnect.rs::reconnect` explicitly rejects `Embedded`; current
reconnection is out-of-process only. Consequently the final embedded provider
does not rotate during the active App loop. The existing file-search restart on
remote reconnect rotates presentation generations while retaining its local
provider snapshot. If embedded reconnect is ever added, replacing an entire
runtime requires an explicit new scope from that runtime and joined old-scope
cleanup; a copied connection ID is insufficient.

## Daemon/remote and other embedders

For `AppServerClient::Remote` / local-daemon mode, there is no same-process
provider to share. Preserve the current TUI local filepicker filesystem scope;
compose its one local provider from the local home/policy and own its shutdown
outside `App::run`. Do not route local picker roots into a remote search server
or claim this shares a backend across machines/processes. Existing remote-workspace
picker policy must remain explicit when that broader behavior is addressed.

Standalone App Server `lib.rs` composes its provider after effective config and
before its `MessageProcessorArgs` construction, then uses the same connection
scope/aggregate quota model. Its provider is owned by that server invocation.
Other in-process embedders (exec, worktree helpers, tests) keep calling `start`
unchanged and get one private composition per runtime. A separate picker runtime
created for an independent embedded server is independent by design, even when
the home is equal.

## Shutdown and failure contracts

- A connection close fences/drains only its scope. Closing the TUI picker or
  dropping its manager likewise closes only the picker scope. Neither kills
  unrelated sessions using the shared provider.
- Server runtime shutdown fences **the provider and all scopes**, including
  local TUI scopes, then joins accepted startup/update/poll/callback/close work
  and the single backend child. This must not wait for public scope handles to
  be dropped. Repeated scope shutdown after provider shutdown observes the
  retained close result and is safe.
- Keep graceful shutdown error propagation through MessageProcessor drains,
  in-process processor/store cleanup, the high-level client and final TUI
  result. Combine cleanup errors without skipping storage/thread cleanup.
  Source audit found a pre-existing follow-through gap in
  `app-server-client/src/lib.rs::InProcessAppServerClient::shutdown`: explicit
  acknowledged errors propagate, but an acknowledgement timeout or worker
  timeout/abort can fall through to `Ok(())`. Before claiming shared-provider
  shutdown success, return an explicit timeout/uncertain outcome for those paths
  and keep the lower-level provider guard's cancellation fence. This is not a
  Stage A cross-owner regression and was not changed during this audit.
- Forced timeout/drop fences all scopes but cannot claim a joined provider or
  reaped process until the owner receipt confirms it. Shutdown/cancellation of
  a waiting caller never releases quota for work still preparing or closing.
- Factory admission and the global budget are scoped by a unique provider
  incarnation, not by `ConnectionId(0)`, Codex-home text or user session IDs.
  Different embedded runtimes both use connection0 today and must remain
  separate. Selection is frozen for that runtime; cwd/config reload does not
  silently select a new provider.

## Required focused acceptance

1. One embedded server plus a local TUI scope: independently installed worker
   reports exactly one provider instance/process; both clients exercise real
   queries and share an aggregate limit. Gated close retains its slot until join.
2. Saturate App Server scopes and verify picker admission observes the same
   capacity; then close one scope and prove the other resumes. Reverse the roles.
   Scope cancellation/close must stay available at capacity.
3. Close a connection and picker separately; the other's existing session remains
   usable. Full server shutdown fences both and reaps the selected worker.
4. Two simultaneous embedded runtimes with different homes and distinct selected
   implementations: results, package identities, quotas, errors and shutdown are
   independent. Include same-home separate runtime instances to prevent hidden
   home-keyed global caching.
5. Startup policy failure, explicit selected-provider failure, provider creation
   success followed by persistence/initialize failure, and abandoned start wait:
   no native fallback; accepted work retains a cleanup owner; all observed child
   PIDs exit under the unchanged lifecycle runner.
6. TUI onboarding/provider restart before `App::run` adopts only the final
   runtime's scope. Earlier provider shuts down; no stale factory can reopen it.
7. Embedded full exit and early App startup error exercise the real CLI/TUI
   cleanup path. Remote reconnect retains local provider selection while fencing
   old query events; no new raw local paths are sent to remote RPCs.

No checkout edits, builds or executions were performed for this proposal.
