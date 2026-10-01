# P03 authority/broker: source-grounded next-slice audit

Read-only audit of `/workspace/codex-harness-everythings-a-plugin`, 2026-10-01 UTC. No checkout edits, patch application, Rust commands, Git commands, installation or runtime checks were performed. The P02B TUI stage remains frozen at `/tmp/p02b-preparing-tui-consumers-stage/FINAL_MANIFEST.json` SHA256 `9fc7b27eafb85000e3e60fe30140f317b3903a7f9d6bfa05eb34eff16fd14fc9`. The parent owns its adoption, regression results and current Git checkpoint. Source hashes below identify this audit's input bytes; they do not establish a new consistent Git tree or test result.

## Recommendation and scope

Finish the search Preparing gates first. The smallest coherent P03 change is a **private extraction of the active persistent transport's physical codec/assembly and reuse of its retained process startup owner**, with broker activation still disabled. Then activate the optional, explicitly negotiated leaf dependency broker. The first concrete native service successor is **native model catalog merge/discovery/cache ownership**, using the existing native `OpenAiModelsManager` in a separately built worker and a host-owned, narrowly granted endpoint adapter.

These are three ordered work packages, not a claim that three commits complete P03 or P05. The roadmap's other P03 obligations—general Session construction/teardown ownership and model-v2 transport—remain required; P04 configuration/authentication and P05 native inference are separate work. The catalog package cannot silently skip the canonical P03/P04 prerequisites. If its narrow P05a milestone is intentionally decoupled from inference-specific model-v2 work, amend the roadmap explicitly and retain model-v2 as a prerequisite to P05c. No such dependency change is made by this audit.

The first two packages are **new infrastructure/shared support**, not native extraction. Only the third can establish a new extracted native subsystem, and only after external-build/unchanged-host acceptance. It does not extract all provider routing, OAuth/keyring, inference, `ModelsManager` helpers, or the agent loop.

## What the actual source says

Rust source paths in the discussion below are relative to `codex-rs/`; root, `component-sdk/`, `upstream/` and fingerprint-table paths are repository-relative unless absolute.

| Boundary | Active source and staged source | Consequence |
| --- | --- | --- |
| Persistent process ownership | `component-host/src/session_supervisor.rs::ComponentBinding::{connect,connect_with_limits,connect_with_options}` and `connect_owned` retain startup independently of the public waiter; `StartupCancellation` and explicit reap handling exist. Staged `broker_connection.rs::connect_with_broker` spawns/handshakes in the public future. | Do not activate the older constructor or use `ChildGuard::Drop` as observed cleanup. Reuse the active owner, immutable cwd, payload limits and typed failures. |
| Physical transport | Active `session_wire.rs::MessageReader` combines read/frame/assembly/duplicate detection/limits; `Sending` owns send state. Staged `broker_wire.rs` imports nonexistent `MessageAssembler`/`read_frame_bytes` and private `Frame`; `broker_writer.rs` expects nonexistent helpers. | A private shared codec seam is necessary. A second independent framing stack would duplicate validation and lifecycle behavior. |
| Bidirectional service lifecycle | `component-host/src/lib.rs` links no broker modules; `ComponentServer` has only plain `stdio`/`stdio_with_limits`. Staged worker calls `stdio_with_broker`, `.dependencies()` and request `.dependency_scope()`. | There is currently no working component-side broker connection, server demultiplexer or reverse-call failure owner. Staged tests are not active proof. |
| Current reply ownership | Active `SessionFailure`, `PendingComponentReply`, `start_with_cleanup`, `DeferredControl::release_reply` postdate broker patches. | Preserve exact typed local-limit/admission/error distinctions and paired cleanup receipts; old patches cannot be applied verbatim. |
| Reverse decode | Staged `DependencyClient::complete` removes pending ownership before `call` invokes `Payload::into_value`, which starts a blocking parser. | Wire completion is not joined decode completion. Retain ownership through parsing and publication, including abandoned waiter/runtime loss. |
| Composition/configuration | `ComponentBinding` is an immutable package/config/state/argv snapshot. Manifest `dependencies: BTreeMap<String,String>` is package-semver resolution. Staged `HostDependencyRegistry` is service authority. | Do not conflate installed-package dependency, component metadata, service requirement and actual issued grant. User JSON cannot create host authority. |
| Native catalog | Unlinked `models-manager/src/manager/component_service.rs::NativeCatalogService` uses actual `OpenAiModelsManager`, native cache and `BrokerModelsEndpoint`; `ProcessModelsManager` is the host mirror/adapter. | This is a concrete extraction candidate, but not active/compiled or independently replaceable evidence today. |
| Provider authority | Active `model-provider/src/provider.rs::ConfiguredModelProvider::create_models_manager` constructs the native `OpenAiModelsEndpoint` with the selected auth and gateway managers. Staged `create_catalog_dependencies` wants that actual endpoint. | Add a narrow provider-owned endpoint capability; do not rebuild provider/auth/gateway selection from config in core. Static providers expose no fabricated remote endpoint. |
| Native policy publication | `login/src/auth/change_state.rs::AuthChangeState` separates credential and owner generations. `AuthManager::set_forced_chatgpt_workspace_id` only updates its value lock. | An unobserved A→B→A restriction change is invisible to value polling. Publish a monotonic policy epoch atomically with the native setter before claiming strict authority fencing. |
| Early model selection | `core/src/thread_manager.rs::build_models_manager` constructs a native manager before `Session::new`; `core/src/session/session.rs:787` later loads components only for non-isolated sessions. | Selected catalog composition must occur before model listing/default selection, while preserving explicit source provenance and isolated-session bypass. Eagerly loading a broken global plugin for isolated sessions is a regression. |
| Session scope | `core/src/state/service.rs::SessionServices` retains shared models/auth, MCP, execution, proxy, storage and other coupled services. Managed `SessionStartup` exists, but ordinary construction commonly has no such owner. | Do not revoke a shared provider when one session cancels. Do not classify this large service holder or native domain implementations as unavoidable kernel responsibilities. |
| Fourth search consumer | `rollout/src/list.rs::find_thread_path_by_id_str_in_subdir`, fallback near1532, directly calls `file_search::run` with the storage root and native flags. | Remains an explicit selection gap. A thread-store worker calling persistent search through the first leaf broker is forbidden; needs a later bounded acyclic dependency contract. |

Independent transport sub-audit: `/tmp/p03-broker-transport-sub-audit.md`, SHA256 `a058ebd2bc611426e7414000f2df216dd779d036edb5386bcf25552bdc5a3ea6`. It documents exact stale symbols, guard entrypoints, admission/flush ownership and package-kind validation.

## Ordered slice 1 — preserve the active transport; extract shared private mechanics

**Deliverable:** isolate current frame parsing, assembly/chunk accounting, physical writing and payload ownership behind private reusable primitives. Preserve plain persistent operation as the only enabled mode. Keep common process construction in `session_supervisor.rs`/its private helpers rather than introducing another spawn policy. Reuse `ComponentSessionOptions`, `SessionPayloadLimits`, typed `SessionFailure` and retained startup completion. Separate mechanical moves from behavioral changes; keep each reviewable change within AGENTS size guidance.

**Exact starting paths:** `component-host/src/session_wire.rs`, `session_supervisor.rs`, `session_failure.rs`, `session_limits.rs`, `session_options.rs`, `session_reply.rs`, `session_server.rs`; tests already beside these modules. Do not add broad public API merely to satisfy stale broker imports. Ordinary and reverse IDs/maps and reserved lanes remain distinct namespaces/capacities; those are intentional, not duplication to eliminate.

**Contract/ownership requirements:** no changed current handshake/frame semantics; duplicate-field rejection must continue through raw deserialization, not a `Value` intermediate that discards duplicates. Preserve configured incoming declaration limits before spool admission, existing unlimited default, immutable explicit cwd, never-admitted local errors, physical send fences, startup abandonment, direct-child reaping and forced-unconfirmed outcomes. Do not claim async worker joins also join every `spawn_blocking` serialization/parser task; state and implement the exact ownership boundary.

**Acceptance:** existing `session_wire_tests`, payload/limit/server-limit/options/start/supervisor/reaping tests; malformed/duplicate/truncated fragments and child/stdin failure; actual old plain worker interoperability. Run scoped `just test -p codex-component-host` and appropriate API tests under the unchanged strict subreaper runner. Re-run affected real storage/search persistent-host lifecycle gates. Record actual tested source map and failed attempts separately.

**Exit:** no new domain selection or broker activation; current installed storage/search behavior and cleanup receipts preserved. This is shared support extraction only. Parent should avoid simultaneously changing the search startup contracts or fixtures while establishing this transport baseline.

## Ordered slice 2 — activate a negotiated, bounded leaf broker end to end

**Prerequisite:** slice1 accepted plus an agreed authority/lifecycle contract. Rebase the staged modules and SDK activation patches onto current APIs rather than applying old text. Link host and server in one coordinated checkpoint, with behavior-preserving intermediate private commits if needed.

**Deliverables:** optional broker handshake/version1 descriptors; host scoped call admission; component-side single reader/writer demultiplexing; `DependencyClient` completion/failure owner; owned reverse-request and reverse-response decode/spool lifecycle; separately retained accepted handler/destructor/final-flush ownership. Implement the current equivalent of `stdio_with_broker` without changing existing plain `stdio`. Caller-waiter loss retains accepted work and its scoped authority. Parent completion or explicit scoped cancellation/deadline retires admission for only that parent context; it does not revoke reusable connection-level grants. Connection/process teardown revokes connection authority, and actual owner/policy revocation rejects affected grants. Retain accepted cleanup/context ownership until joined or explicitly unconfirmed. Activation must use the same retained startup/options/limits/error spine as ordinary sessions.

**Contract card to agree before edits:**

- Component/API version, persistent transport negotiation and domain-contract versions are separate. Broker extension is explicitly versioned and opt-in; absent/unsupported required service fails before domain activation. Plain existing peers remain valid.
- Use bounded typed required/optional service requirements distinct from package dependencies; grants remain host-issued per immutable connection. Define service name, version, finite operation set, connection/incarnation, owner generation and parent invocation/request context. Match the currently staged32-grant ceiling or justify a different explicit bound.
- Staged `BrokerHost::acknowledge` requires an exact operation-set match, while the proposal describes subsets. Choose exact advertised-grant equality for initial v1, with negotiation selecting the grant first; test rejection without silently widening authority. Additive operation negotiation can be versioned later. Do not publish conflicting SDK text.
- Preserve typed never-admitted, operation error, timeout, stale authority, protocol loss and cleanup uncertainty. Accepted operations are supervised and not automatically replayed. Cancellation intent is synchronous/idempotent; observer abandonment must not abandon cleanup. Known operation failure precedes later cleanup failure. Avoid promoting search-specific `NotAdmitted`/`Confirmed` names into a universal API without domain review; preserve equivalent facts.
- Retain parent/context until handlers, decoders, serializers and physical terminal flush have actually settled. A receipt after forced termination must say what is unconfirmed. Ordinary and reverse request IDs may be numerically equal without aliasing; cleanup and response reserve remain independently available under saturation.
- Specify aggregate byte/spool/work limits, not just count bounds. The staged8MiB reverse-request parse ceiling applies after a full spool is already received; it does not currently bound spool disk use or response history. Do not reuse it as a hidden reverse-response/native-history cap. Review full JSON materialization limits explicitly.
- First version remains a host-owned **leaf** service model: no persistent constructor/start/release/close callback from a leaf handler, even to another component. Guard all centralized current entrypoints before spawned tasks lose task-local context. Arbitrary helper tasks do not inherit the task-local guard; this implementation restriction is not a malicious-plugin sandbox or a general dependency graph.

**Acceptance:** install an independently built minimal diagnostic consumer outside the harness source; unchanged host hash; prove a real reverse call, not only an in-process mock. Real stdio peers exercise colliding forward/reverse IDs, full forward capacity plus cleanup reserve, saturated cancellation/responses, stale/wrong-session grants, owner revocation during fetch, parent return while handler/destructor/flush is held, waiter drop during parse/write, EOF and handshake failure, sibling survival, repeated close observers, process/runtime death and forced uncertainty. Assert ledger counts and exact authority, not merely eventual zero processes.

**Regression:** existing plain peers/storage/search; cwd and configured size caps; managed launcher Ctrl+C and GUI Stop/recovery for affected shared lifecycle paths. Keep unknown capability/version/malformed reply failures explicit; no native fallback. Tests under `broker_*_tests.rs` are inputs to rebase, not accepted until linked and run. Cargo/Bazel metadata must follow actual dependency changes.

**Exit:** optional bidirectional broker linked, separately installed consumer verified, old peers preserved, actual cleanup and authority evidence recorded. This still adds infrastructure; the minimal diagnostic consumer does not count as native subsystem extraction.

## Ordered slice 3 — native catalog/cache ownership with minimal authority prerequisites

**Deliverable:** activate `model-catalog-api`, `model-catalog-native-plugin`, `NativeCatalogService`, `BrokerModelsEndpoint`, `ProcessModelsManager`/validated mirror and the provider-owned `host.model_endpoint` grant. Add narrow `model_catalog` contract1 admission to `ComponentCatalog::read_manifest`; it currently rejects this kind and hardcodes other kind versions. Add workspace/module/SDK/package/Bazel links. Preserve no-plugin native behavior and fail explicitly for an invalid selected package. A general extensible kind registry remains a later platform gap; do not replace validation with unchecked strings.

**Prerequisites/first patches inside this package:**

1. Fix native forced-workspace policy epoch publication and test unobserved A→B→A, same-owner token refresh versus owner replacement, revocation and network-policy changes. This is a small necessary P04 authority prerequisite, not complete config/auth extraction. Reuse `AuthChangeState` semantics instead of duplicating account identity in wire config.
2. Add provider capability `model_catalog_endpoint()->Option<Arc<dyn ModelsEndpointClient>>` using the actual configured endpoint/auth/gateway selection. `create_catalog_dependencies` retains HTTP factory/policy per accepted request in host-only `HostOperationScope::request_context`; the plugin receives opaque owner identity/capabilities, never credentials, raw auth headers or a generic URL/HTTP grant.
3. Define an owning model-services scope and early async selected-manager composition at the `build_models_manager` callers. Preserve default-versus-explicit catalog source and isolation before listing/default selection. A per-session cancellation cannot revoke a shared provider. Return acquired service ownership through initialization, failure and ready-but-unconsumed handoff; coordinate with the canonical general Session ownership work rather than relying on a dropped Arc.
4. Rework staged `ProcessModelsManager::refresh`: it currently spawns once per caller then waits on `operations:Mutex<()>`, permitting unbounded queued tasks. Admit bounded work synchronously, register completion guards before spawn, retain accepted operation/close ownership independently of waiters, preserve first cause and do not let close race a queued operation into reviving the mirror. Cover failed `connect` after session acquisition/OPEN/decode/mirror publication.
5. Keep `CatalogSource::Static` user configuration pinned; `ProviderStatic` remains a replaceable native provider seed; preserve BundledMerge/Authoritative/cache semantics. Bedrock has no fabricated OpenAI endpoint. Use the accepted lossless native path codec for `CatalogCache::NativeDirectory`; staged raw `PathBuf` serde is not sufficient for arbitrary native paths. Define finite snapshot/model-list limits and explicit exhaustion; replace `revision.saturating_add(1)` with nonreused/fail-closed revision behavior.

**Exact native implementation extracted:** native model catalog discovery/merge and file-cache state from `models-manager/src/manager.rs::OpenAiModelsManager`, instantiated in the external `NativeCatalogService`. Host adapter retains an owner-fenced mirror and existing helper interface; native network/auth policy remains the bounded `ModelsEndpointClient` leaf adapter. Be explicit about native helper code still linked in the host. This is not the native inference transport, all auth, all providers or all of `ModelsManager` made replaceable.

**Acceptance:** build native worker from an external source export/clean target, park that source, install and select without rebuilding the frozen host; separately build a custom catalog that visibly changes actual model choices/capabilities. Verify native/default/selected/custom parity where required: TTL/ETag, offline zero-fetch and cold cache, merge versus authoritative sources, cache client-version compatibility, token refresh, account/policy A→B→A, mid-fetch revocation, stale snapshot rejection, malformed/oversized snapshots, cancelled/abandoned startup and close, package upgrade/version rejection/removal restoring native behavior. Preserve old cache/state or document an explicit migration and rollback artifact.

**Real-host/UI gate:** actual CLI model selection and inference request model/instructions; App Server model listing and invalid-selected-package errors; existing desktop plugin model/task flow, streaming/tool progress/approval/Stop and cold recovery. Use deterministic endpoint/inference fixtures when credentials are not needed; label them. In-app Browser remains a separate availability limitation, so a Playwright/screenshot pass is not that requested proof. Remote viewer connectivity must not block the runtime extraction work.

**Exit:** actual native catalog/cache worker and independent custom replacement both work with the same host bytes; default/static/Bedrock behavior retained; exact ownership/cancellation/security gates accepted. Update inventory only for this native service boundary. Preserve canonical P03/P04/P05 prerequisites and outstanding obligations explicitly.

## Kernel/security ceiling and authority that must not be duplicated

The canonical kernel exceptions cover immutable composition/compatibility, identity, transport/process cleanup, host capability issuance/revocation, minimal correlation and final permission/OS boundary enforcement. They do not justify retaining native catalog policy, provider defaults, OAuth, rule scoring, broad `SessionServices` or the loop as kernel.

Retain the native endpoint's request-time auth resolution, gateway routing, managed destination enforcement, timeout/retry policy and response identity validation. Recheck authority before execution and publication; do not log provider diagnostics/URLs/response bodies in safe domain errors. Static/no-endpoint composition must grant no HTTP capability. Host-only contexts cannot be recreated from a child-supplied opaque identifier. Immutable installed package config is configuration, never a grant.

Installed components remain trusted same-user executables. Broker enforcement constrains its own service channel, not arbitrary filesystem/network syscalls by a malicious executable. Preserve existing sandbox/managed restrictions; do not change sandbox environment-variable semantics or advertise OS isolation that is absent.

## Provenance and evidence updates required for each adopted slice

Official source pin remains `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`, upstream tree `147ac2447134294359c4071b0aeb495922760db7`, exact-tree import `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`, as recorded by `upstream/lineage.json`. Retain Apache LICENSE/NOTICE; no DeepSeek/Cordis code is involved. That lineage file's `current_commit` is an immutable historical recovery checkpoint, not today's working tree.

| Proposed slice | Upstream/native mapping | Intentional custom boundary to record |
| --- | --- | --- |
| Transport and broker | `component-host`/SDK broker infrastructure is newly added custom source; map its actual prior custom checkpoint and renamed/split symbols, not invented upstream originals. | Shared physical codec, retained startup/decode/reply ownership, negotiated authority/version/security restrictions; preserve ordinary wire compatibility and current package dependencies. |
| Native authority prerequisite | `model-provider/src/provider.rs::ModelProvider` and configured native endpoint; `login/src/auth/manager.rs` native restriction publication plus custom `AuthChangeState` history. | Provider-owned endpoint injection; monotonic policy epoch and opaque per-request authority. Record these as semantic customization when updating upstream. |
| Native catalog | Existing lineage anchors `models-manager/src/manager.rs::{ModelsManager,OpenAiModelsManager}` upstream blob `af734443c07a27b9e3b22761edb97d05341a7556`; `model-provider/src/provider.rs::ModelProvider` blob `7d99c556dddcf48403936663154a57144f8124d4`. | Destination `NativeCatalogService`/worker, `BrokerModelsEndpoint`, `ProcessModelsManager`/mirror, CatalogSource/cache/auth/snapshot contracts. Record remaining host helpers and native endpoint ownership accurately. |

For each checkpoint retain original/current paths+symbols/blob hashes, actual commit/tree and working diff, contract/package versions, acquisition/ownership/cancellation/upgrade semantics, intentional changes, licenses and exact regression reports. Record external worker/source/host hashes and installed package metadata. Keep initial failed runs, pre-lint tested map and post-lint/format map separate. Suggested evidence files under `verification/<date>/`: `P03_SHARED_TRANSPORT_EVIDENCE.md`, `P03_LEAF_BROKER_EVIDENCE.md`, `P05A_NATIVE_CATALOG_EVIDENCE.md`; these names are proposals, not existing proof. Feed mappings to the required P18U isolated-upstream-update workflow; do not start polling or auto-apply revisions.

## Explicit remaining gaps after these recommendations

- General ordinary Session acquired-service cleanup is not solved by the process broker. Existing managed `SessionStartup` is optional; follow `component-sdk/SESSION_RUNTIME_LIFECYCLE_PLAN.md` for pre-Session contributors, retained handoff, loop loss, prewarm/follower/proxy ownership and idempotent joined teardown. Preserve shared-service authority and report native process-backend cleanup limits.
- Model-v2 chunking/typed terminal transport and its missing links/tests remain P03 obligations, and are required before native inference replacement. Ordinary Payload blocking jobs and aggregate spool budgets need evidence, not a claim inferred from frame limits.
- Storage's internal search fallback requires injected root-bound dependency and a real bounded acyclic graph/ancestry contract. Initial leaf rules prohibit it even if another process has free request capacity. Never let workers read ambient user config/global cwd to choose dependencies.
- Full config/auth components, provider selection/native inference, other workspace/watch/git services, general installable kind registry, security policy extraction, full SDK release and upstream updater remain outstanding. Do not treat the proposed catalog worker as those implementations.

Independent transport reviewer corrected the initial draft to distinguish caller-waiter loss from scoped cancellation: accepted work and its authority remain retained after observer loss. The initial draft SHA was `bd7f94ea29a0759db4102ab336094d287132dcccc24c480263db40507021a463`; it must not be used as the final recommendation. Review also clarified the source-path prefix convention.

## Audit input fingerprints

Generated from current file bytes after source inspection, without Git. Mutable parent-owned files may change after this snapshot; no tests bind to these audit hashes.

Snapshot UTC: 2026-10-01T10:28:05.045174+00:00

| Source | SHA256 |
| --- | --- |
| `AGENTS.md` | `b3dc6716209116a311dce67ddd490d7d0b998031734720524ad407fe67c7cf48` |
| `IMPLEMENTATION_ROADMAP.md` | `0c8e9eecd3fdda55231ca4c52b348ece4e1d34d6d996f51ef0e8b36a5ed66299` |
| `EXECUTION_STATE.md` | `f77f5278a412ea9dc921818ee7b6dea4e2bdd2f5e0762c0d1f0919fd2b3462f5` |
| `COMPONENT_INVENTORY.md` | `4bff267768e9855b35d752eb266a916e602dc9ddaf26f5403dd39234db6d7be2` |
| `UPSTREAM_PROVENANCE.md` | `c88d08438fa734d31240f14b0d7be7247e966d9194cf6fa93930a67acbf230e3` |
| `UPSTREAM_MAINTENANCE.md` | `7e55bc10d089d780d9893fd489d0896cc593459896c7238443887098d74d525a` |
| `upstream/lineage.json` | `6fbb06a193b5bcc766d2fa7cd0ce06cae2ace7b5594fcc2685e98503e9fc6328` |
| `component-sdk/BROKER_REVIEW.md` | `ddf6ec60193c60fa8623e90da7d43b2891b47b80e4ba0d3abe518b3cca2b6541` |
| `component-sdk/DEPENDENCY_BROKER_PROPOSAL.md` | `f8a0d0778c70f46b7ce399fc5498e57c8ff80b754de506ebd0c6b81ef79d1130` |
| `component-sdk/SESSION_RUNTIME_LIFECYCLE_PLAN.md` | `4c24d4cdcbdefb62868c8f01e9e0022dc622dab352a2dde84e94c30387c240fd` |
| `codex-rs/component-host/src/lib.rs` | `270e2f3d122c2baad90ad428c2662c9c81f65d4a1ed81cab2d36a505d4287d71` |
| `codex-rs/component-host/src/session_supervisor.rs` | `78c4f6b041dbf8bb91b089ed253adf7c57f7a86f1b525783fe7bd01260c1ada2` |
| `codex-rs/component-host/src/session_wire.rs` | `90e5f8943dd355179d6fcc699bdeea6f566d0bad2b368a8d6614dba5de3ca6e0` |
| `codex-rs/component-host/src/session.rs` | `510d2049f8717d38ea2bbab0b815c6a745f1f8902835310f88e3efb064840178` |
| `codex-rs/component-host/src/session_server.rs` | `6b7678ffcef8b5dc25a67d51e085d755672f00bd04423185dfa8cae07888706f` |
| `codex-rs/component-host/src/catalog.rs` | `3fef43219940b1d6d80e60582bae230a4257ca31e36e695906f0a72c9592c370` |
| `codex-rs/component-host/src/broker_connection.rs` | `35c8e8ed2b39d25debea08191b9d02d87047c54f8060ada0c5c25ad54f5052ff` |
| `codex-rs/component-host/src/broker_client.rs` | `d90c64ddd748a9d09dce89b015f203bc0c5cbcb925b8e5c20562dc83e2c1d08f` |
| `codex-rs/component-host/src/broker_host.rs` | `db5b164a2afa8fa0f8549f28ae63417f573868110a00f213af338590556295e9` |
| `codex-rs/model-catalog-api/src/lib.rs` | `d6f603cf85cb8ed21a6ccd35c46328106d09e2c55f3b39a2d6548a5a391bf916` |
| `codex-rs/model-catalog-native-plugin/src/lib.rs` | `5c2ada7903597561318194b0b0c8f6733d88dc5824e6e18c99e30a06e58c97b4` |
| `codex-rs/models-manager/src/manager/component.rs` | `d4fb0b43e6dc71755a0cc94f118995d20af31c180fddd3af9d0bbee355b7bf06` |
| `codex-rs/models-manager/src/manager/component_service.rs` | `82b8a9d9e225966f98a3cfb1498f2a192cda137b8f64b0a096e23a50101be90d` |
| `codex-rs/model-provider/src/component_catalog.rs` | `f183bb68f1e8b525166f92bd5e679572a9d9362d33150361efe808ab17cbaef6` |
| `codex-rs/model-provider/src/provider.rs` | `adabad97f59dbc593431a9c47911e74d1fd9d48648b874026a23db86b32ec5a1` |
| `codex-rs/login/src/auth/change_state.rs` | `f87d4d87d8b6737a78931cfff9f0c6b62f32cc87c10e73262c0bc7fc04445288` |
| `codex-rs/login/src/auth/manager.rs` | `2d1fae596b899a39b8db8af3a32ab944838b31a8972349f631fcb94e4f4f062c` |
| `codex-rs/core/src/thread_manager.rs` | `371cabacd7190f9b19c6bd483e9e7127a831f0f5c072b947708575ed21501c3c` |
| `codex-rs/core/src/session/session.rs` | `e3496ef88b2233cc7c90718e3f58e11a0897bae6871834d95e9759eff0e58650` |
| `codex-rs/core/src/state/service.rs` | `4bdcb237942356b7c2bbf9a1b3c31264bbdccdaddfef045cfb7f6d69cdcacbfd` |
| `codex-rs/rollout/src/list.rs` | `f86bfdf55686b2bced1f03ef9535918b4963bfb539b9bc06996737917ed5e79c` |
