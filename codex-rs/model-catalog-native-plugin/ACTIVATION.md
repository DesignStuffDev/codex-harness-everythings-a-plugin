# Staged catalog implementation activation

The files below are saved but unlinked, uncompiled and unverified. Existing runtime behavior
has not changed. Do not describe this stage as installed catalog support.

## Saved implementation

- `model-catalog-api`: owned source/cache/auth policy and snapshot/domain-error contracts.
- `models-manager/src/manager/component.rs`: process manager, owner-fenced mirror, accepted
  operation ownership, existing metadata/default-model policy, health and explicit shutdown.
- `component_endpoint.rs`: credential-free native manager adapter using broker calls.
- `component_service.rs`: native policy/cache service; no copied manager implementation.
- `component_mirror_tests.rs`: five owner/publication/replay/source-policy cases.
- `component_cache_tests.rs`: three cache load/write/ETag owner-validation cases.
- `component_process_tests.rs`: three actual installed native-worker cases for discovery,
  metadata parity, host-versioned cache/restart/offline behavior and invalidation/recovery.
- This package: standalone native service executable and install manifest, independent of core.
- `model-provider/src/component_catalog*.rs`: endpoint authority/service, seven staged tests
  and `model-provider/COMPONENT_CATALOG_ACTIVATION.md` with exact provider hook instructions.

## Source activation order

1. Complete the prior storage/attachment acceptance wave. Link and verify the generic
   dependency broker first, including host-only per-request HTTP context retention.
2. Add `model-catalog-api` and `model-catalog-native-plugin` to workspace members and register
   `codex-model-catalog-api` as a workspace dependency. Allow `model_catalog:default`, contract
   version 1, in component host/SDK validation and persistent-session admission.
3. Add production `anyhow`, `codex-component-host` and `codex-model-catalog-api` dependencies
   to `models-manager`; add dev `codex-utils-cargo-bin`. Existing serde_json, Tokio and
   protocol dependencies are reused. Ensure Tokio production `rt` is enabled for retained
   accepted operations. Add host/catalog-api dependencies to `model-provider` per its notes.
4. Apply `verification/activation-patches/model-manager-component.patch` after checking it
   against the current formatted manager source. This adds optional read-only endpoint auth
   policy, asynchronous live-owner checks at cache/publication boundaries, host client-version
   cache identity, the health hook, and module declarations. Existing native defaults retain
   their behavior. Apply the provider endpoint hook/export from its activation notes.
5. Build the standalone native worker before `models-manager` tests that locate its binary.
   Run scoped new manager/provider/broker tests, then all native manager/provider tests.
   Regenerate Cargo/Bazel metadata and native package exporter information as required.
6. Add async core model-services composition. Selection must precede native remote-manager
   construction; preserve the legacy explicit-manager constructor. The provider supplies
   its actual endpoint and provider-scoped auth manager. Configured static catalogs are
   pinned `Static`; native static provider seeds use replaceable `ProviderStatic`; explicit
   provider URLs use `Authoritative`; ordinary OpenAI-compatible defaults use `BundledMerge`.
   Construct the registry through `create_catalog_dependencies` and call
   `ProcessModelsManager::connect`. Selected native cache interoperability must explicitly
   use `NativeDirectory`; custom component state must not silently overwrite a native cache.
7. Carry default-vs-explicit model-manager provenance through ThreadManager, SessionSpawnArgs,
   SessionServices and inline delegates. Isolated sessions must use a native/default snapshot
   without invoking user-selected catalog components; caller-supplied explicit managers stay
   explicit. Resolve this before early session model listing/default-model selection, which
   currently happens before the later inference component catalog is loaded. A global eager
   factory alone is insufficient for an isolated-only caller with an invalid installed plugin.
8. Check selected manager health before inference and after model refresh/metadata lookup;
   propagate component startup/IPC/contract/ownership failures instead of using an infallible
   getter's empty fallback silently. Preserve domain endpoint failure semantics: native
   authoritative catalogs clear models/ETag, while ordinary same-owner endpoint failure is
   distinct from component-health failure. Retire selected managers explicitly on owner-scope
   shutdown/config replacement; `close` revokes admission before waiting for accepted work.
9. Add real engine acceptance for actual default model, instructions/context/tool metadata,
   isolation, cancellation, refresh errors and recovery. Build native and custom catalog
   packages outside the checkout, install without host rebuild, compare the host hash,
   exercise live discovery via the broker, and verify removal/native recovery.

No model/provider/core composition changes are included in this staged source yet. Inference
transport, realtime, memory summaries, provider routing replacement and richer inference
errors remain separate boundaries; catalog replacement does not complete them.
