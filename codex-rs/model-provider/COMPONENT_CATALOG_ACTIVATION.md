# Staged native catalog dependency adapter

The `src/component_catalog*.rs` files are **not linked or compiled yet**. They are
the next-wave source prepared while the existing acceptance build is frozen.
They do not change the current executable. Their eight tests are source only.

The public composition function is:

```rust
create_catalog_dependencies(
    endpoint: Option<Arc<dyn ModelsEndpointClient>>,
    auth_manager: Option<Arc<AuthManager>>,
    request_timeout: Duration,
) -> Result<(Arc<dyn CatalogAuthority>, HostDependencyRegistry), DependencyError>
```

It registers an optional `host.model_endpoint` version 1 grant with `fetch` and
`validate_owner`. The native worker explicitly requests that grant. A custom
catalog can omit it and perform its own discovery. `None` exposes no HTTP
endpoint: `fetch` returns `ServiceUnavailable`. Static native providers must not
be converted into an OpenAI endpoint to satisfy this dependency.

## Minimal existing-source activation edits

These edits are intentionally not applied during the source freeze.

1. Add workspace dependencies `codex-component-host` and `codex-model-catalog-api`
   to `model-provider/Cargo.toml`. Confirm production Tokio enables `macros` for
   the cancellation selects; this crate already requires `sync` and `time`.
2. In `model-provider/src/lib.rs`, declare private `mod component_catalog;` and
   re-export only `component_catalog::create_catalog_dependencies`.
3. In `models-manager/src/manager.rs`, expose the staged `component` module (or
   re-export its `CatalogAuthority` and `CatalogRequestAuthority` types and update
   these imports). The authority trait includes `revoke`; selected manager close
   must call it before closing the broker session.
4. Add this provider capability to `ModelProvider` in `src/provider.rs`, using
   the existing `ModelsEndpointClient` contract:

   ```rust
   /// Native model discovery available to an explicitly selected catalog component.
   /// Static providers return None and preserve their own catalog implementation.
   fn model_catalog_endpoint(&self) -> Option<Arc<dyn ModelsEndpointClient>> {
       None
   }
   ```

   Override it **only** in `impl ModelProvider for ConfiguredModelProvider`:

   ```rust
   fn model_catalog_endpoint(&self) -> Option<Arc<dyn ModelsEndpointClient>> {
       Some(Arc::new(OpenAiModelsEndpoint::new(
           self.info.clone(),
           self.auth_manager.clone(),
           self.gateway_auth_manager.clone(),
       )))
   }
   ```

   This repeats the existing `ConfiguredModelProvider::create_models_manager`
   construction and preserves its already-selected command-auth manager and
   gateway manager/error. It does not reconstruct auth from config in core.
   Existing `models_manager*` methods can remain unchanged for unselected runs.
5. At the async selected-manager composition point, explicit
   `config_model_catalog` chooses `CatalogSource::Static` and passes no endpoint;
   the host pins that user-configured catalog. A provider whose native catalog is
   static, including Bedrock, supplies `CatalogSource::ProviderStatic` and no
   endpoint. That native seed is replaceable by a custom catalog implementation.
   Configured OpenAI-compatible providers pass `provider.model_catalog_endpoint()`
   and `provider.auth_manager()`, choosing the same bundled/provider catalog source
   as native `create_models_manager`. Do not assume every `ModelProvider` has a
   remote `/models` endpoint.
6. Link the broker runtime, catalog contracts, worker and selected manager in the
   coordinated wave. Refresh Cargo/Bazel metadata and run the scoped native
   provider tests. `BUILD.bazel` uses the crate macro's Rust source glob; these new
   files have no `include_str!`/fixture compile-data dependency.
7. `AuthManager::set_forced_chatgpt_workspace_id` currently changes its lock value
   without advancing a native policy/owner revision. The authority observes the
   effective methods/workspaces before publication and during reads, but strict
   fencing of an **unobserved** restriction A -> B -> A transition requires that
   setter to publish a policy/owner epoch under its write lock. Coordinate that
   small login-crate change rather than treating value polling as an epoch.

## Ownership and behavior

The authority combines the native opaque endpoint identity, native auth owner
revision, effective auth mode and catalog capability flags. Native auth revisions
detect account A -> B -> A changes even if notifications coalesce. Access-token
refresh that preserves the native account/user identity and policy preserves the
catalog generation. API-key discovery changes and application network-policy
publications invalidate old generations; close explicitly revokes the authority.
Provider configuration belongs to the immutable provider/manager instance. Its
replacement must close/revoke the old selected manager; there is no global
provider pointer in this adapter.

The accepted operation retains its own HTTP factory and auth policy snapshot in
host-only `HostOperationScope::request_context`. Nothing puts a factory, auth
token, account ID, gateway config or request headers in the wire DTO. Native
`OpenAiModelsEndpoint::list_models` still owns request-time auth resolution,
gateway composition, routing, destination enforcement, retries/timeouts and
response validation. Acyclic one-shot external auth is allowed in this leaf;
callbacks into the requesting persistent component remain prohibited.

The host rechecks ownership before and after the native fetch and rejects a
response whose actual native identity differs from the accepted identity. It
also enforces the native API-key discovery gates, so a custom catalog cannot
bypass them by directly calling the grant. Broker cancellation/revocation races
the native future and drops it on cancellation; no retry is introduced. Native
errors become a small `CatalogError` enum and never include transport/auth error
text, response bodies or URLs.

## Pending validation

The eight staged tests cover actual native identity/auth rotation, coalesced
account changes, separate request factories, discovery-policy changes, mid-fetch
owner changes, mismatched actual response identity, static no-endpoint behavior,
explicit shutdown revocation, safe errors, network-policy invalidation, and
effective workspace-restriction changes.
They require activation and `just test -p codex-model-provider component_catalog`.

Full acceptance still requires a separately built/installed native catalog worker,
an unchanged host hash across install, real broker fetch/cancellation and native
cache/ETag/offline/auth-change regressions, plus default/static/Bedrock behavior.
These unit tests do not establish process interoperability or whole-catalog
replaceability by themselves.
