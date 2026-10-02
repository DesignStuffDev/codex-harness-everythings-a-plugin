# Codex component inventory

This is the source inventory annex to `IMPLEMENTATION_ROADMAP.md`. Boundary IDs C00–C27 identify functional ownership; they are not phase numbers or a requirement to create one plugin per crate. A subsystem may require several component interfaces, and a helper crate can remain a shared library under an identified subsystem owner.

Inventory baseline date: 2026-09-30; C02/C18 updated through verified P01/P02 on 2026-10-01. Source baseline: official `openai/codex` at `d42056091aded7feb1d88ac7e83972108b2aa478`, with preserved project changes in `DesignStuffDev/codex-harness-everythings-a-plugin`. The recovered published baseline is commit `4e38e02993df698cefa99d7cc325890d837262de`; `wip/recovered-next-components-20260930` at `0b38d5974159ab2a8776200025dbb602e89b8f8f` preserves newer, unverified work. The primary checkout also contains unlinked staged sources: inclusion in the baseline snapshot is not evidence of activation.

The inventory covers all 169 explicit Rust workspace members (164 baseline, two P02 contract/codecs and three search component/runtime crates), five additional existing native path-dependency crates, six unregistered staged Rust crates in the primary checkout, one additional WIP Rust crate, nine non-Rust package manifests, and the executable scripts, SDKs, frontend assets, platform bridges and build infrastructure around them. Package coverage is exhaustive for that manifest set; behavioral completeness must still be established by the extraction and acceptance work below. No functional area is excluded merely because it is native, security-sensitive, platform-specific, a helper, or already a separate crate.

## Status and evidence rules

| Status | Meaning |
| --- | --- |
| VERIFIED_NATIVE | A bounded pre-existing native implementation runs through a selected external component, built and installed independently of the unchanged host, with recorded real-runtime acceptance evidence. This never implies its entire surrounding subsystem is extracted. |
| ADAPTER | A real host call path can delegate through a new interface, but the complete pre-existing native implementation has not been independently packaged and proven replaceable. Infrastructure alone is also labeled ADAPTER, not a native extraction. |
| ADDITIVE | A new capability, contributor, test implementation or presentation client. Useful and independently installable, but not extraction of prior native behavior. |
| STAGED | Source or integration patches exist, but are unregistered, unlinked, incomplete or unverified on the relevant host. |
| COUPLED | Pre-existing functional behavior remains selected or owned by the built-in runtime. Existing Rust crate or trait boundaries are useful starting points, not installation boundaries. |
| SUPPORT | Shared schemas, algorithms, runtime helpers or build/test assets. Their behavior belongs to the subsystem using them; this does not exempt a functional service from extraction. |

These labels apply to the precise scope named in each row. The recovered runtime proves native `LocalThreadStore` operations and the native `InlineAttachmentStore` upload/resolve behavior through external native workers. The inline implementation returns the original upload bytes and resolves every file ID as `NotFound`; it does not persist blobs. It does not prove every storage/metadata/migration operation, nor all model, authentication, context or tool implementations.

Evidence: `verification/2026-09-30/recovery-runtime-summary.json` records real CLI/model/tool/context/resume/native-storage recovery, the manager shutdown cases and a 5 MiB inline attachment upload, `NotFound` resolution and component-uninstall check, using fresh external homes and unchanged preserved host binaries. The original attachment reporter wording mismatch is retained and adjudicated in `recovery-attachment-revalidated.json`. That report exercises the native attachment package through the component manager; it is not an engine-adapter acceptance test. Its removal step uninstalls the component, not an attachment. `verification/2026-09-30/recovery-gui-acceptance.json` records externally packaged GUI installation, streamed responses, a native shell approval, cancellation, actual manager Ctrl+C, cold recovery and continuation. Inference used a deterministic external model fixture, not a live model provider. Browser evidence is Chromium through Playwright; the requested in-app Browser was unavailable. `recovery-core-lib-portable-20260930.json` records 2,694 passing core library tests, with one ignored helper outside that selection; it is not a complete workspace, integration, doctest or cross-platform result. See `recovery-core-failure-analysis.md` for the two test-only cloud portability fixes and unchanged strict process-exit checks under a subreaper.

Current HTTP continuation: native constructor/result-disposal custody and later-runtime cleanup recovery are implemented as SUPPORT within the coupled HTTP service. The full package passes139 tests, including ten custom-CA integrations; four configured-CA classification tests pass separately. The typed error traversal fix changes classification, not certificate trust or fallback policy. The [process-final composition](verification/2026-10-02/p03-http-native-constructor-stage-b/README.md) passes a real native request/aggregate-closure test,553 library regressions and scoped lint. Individual current-source consumer/replacement tests remain open. The later
[production build](verification/2026-10-02/p03-http-stage-b-production-build/REPORT.md)
and current-source storage/migration/GUI/shutdown gates are published at
`c463a4f46318ccbbc811c202639d53f733d5831f`; see
[EXECUTION_STATE.md](EXECUTION_STATE.md) for all12 passing runtime slots, the
preserved first GUI failure, reused package identities and remaining limits. These helpers are not an independently installed HTTP component and do not increase the three bounded native replacement families. A subsequent lexical test-scope correction passes formatting and scoped lint without warnings; no tests were repeated solely for style. See [HTTP evidence](verification/2026-10-02/p03-http-native-constructor-stage-a/README.md) and `EXECUTION_STATE.md` for exact source transitions.

## Functional boundaries, contracts and remaining work

The paths in this section are relative to the repository. Source modules inside `core` or `app-server` are intentionally listed as owners even when their crates appear only once in the package ledger. `codex-rs/core/src/state/service.rs` is a useful concrete dependency inventory: `SessionServices` still owns many native services that must be addressed.

| ID / subsystem | Current source and status | Required component contract and ownership; dependency/acceptance concerns |
| --- | --- | --- |
| C00 Component composition | `codex-rs/component-api`, `component-host`, `component-adapters`, `component-state-codec`; upstream `core-api`/`ext/extension-api` and `ExtensionRegistryBuilder`. ADAPTER/new infrastructure; contributors are ADDITIVE. | Manifest discovery, install/remove, selection, versions, dependencies, configuration, capability grants and process lifecycle. Keep explicit generation/owner identities, bounded frames and completion/cancellation outcomes. Reuse upstream contributor contracts without treating registration as native extraction. The accepted storage2 persistent transport is active and runtime-verified; newer model-v2 and reverse dependency-broker enhancements remain staged. P03 private wire support and service declaration parsing/fail-closed catalog checks are verified; checked broker limits, handles, grants and exact offer/acknowledgement data are also API-tested, but runtime negotiation, quota enforcement and granted service execution remain inactive. Plugin installation must not require rebuilding the host. |
| C01 Session and turn orchestration | `codex-rs/core/src/session`, `core/src/tasks`, `core/src/thread_manager.rs`, `core/src/state/service.rs`, `core-api`. COUPLED; WIP runtime lifecycle work is STAGED. | Start/resume/fork/turn/interrupt/shutdown requests produce ordered events and terminal outcomes. Define separate owners for a session, an active turn and spawned work, with one joined shutdown path and explicit persistence obligations. Agent loop, retries and orchestration policies remain extraction targets; the minimal supervising kernel is only a proposal. Depends on C02–C10, C12–C17 and event/client contracts. |
| C02 Thread persistence and maintenance | `codex-rs/thread-store`, `thread-store-component`, `thread-store-local-plugin`, `rollout`; production selection in `core/src/thread_manager.rs`. VERIFIED_NATIVE for selected native thread-store operations and P01 manual rollout migration. `cli/src/migrate_rollouts/runtime.rs` resolves the selected backend before native construction; optional migration1 is implemented over storage2. Other maintenance and auxiliary state remain incomplete. | Thread/list/load/create/fork/append/checkpoint/live-thread operations and close/flush must preserve recovery and ordering. The external worker owns native storage state; host retains session authority. Distinguish live persistence from rollout migration, backfill, reclamation and repair. P01 implements migration dispatch, retained leases and selected CLI composition: dry-run preserves storage, cancellation completes accepted publication, close joins, and forced termination reports durability uncertainty. See the 2026-10-01 evidence for the tested limits. |
| C03 Auxiliary state repositories | `codex-rs/state/src/runtime`, `state/migrations`; host `StateDbHandle`/`StateRuntime` remains COUPLED. | Typed repository operations with transaction, revision and lifecycle semantics, not arbitrary SQL RPC. Domains include threads/metadata, projects, section order, thread attachments, goals, queued items, memories/readiness/versions, logs/maintenance, remote enrollment, external-agent imports, rollout migration, backfill, reclamation and recovery. These are not all replaced by C02. Define cross-domain atomicity and migration ownership before choosing one process versus multiple selected repositories. |
| C04 Attachment storage and upload | `codex-rs/attachment-store-api`, `attachment-store-inline`, `attachment-store-component`, original `attachment-store` facade. VERIFIED_NATIVE for the inline implementation’s byte-preserving upload and `NotFound` resolution; remote uploads remain COUPLED. | The existing `AttachmentStore` trait has only upload and resolve. Inline upload returns the original bytes and owns no durable blobs; resolve always returns `NotFound`. Stored backends may instead return file references and resolve metadata/URLs with TTL semantics. Native upload routing and `RouteAwareClientPool` remain separate targets. Preserve bounded/chunked transport and prove production caller parity, partial-transfer/cancellation behavior and worker shutdown. Existing 5 MiB evidence covers the inline package through the manager, including component uninstall. Attachment deletion would be a new capability/API, not extraction of an existing method. |
| C05 Model access and provider catalog | `codex-rs/core/src/component_model.rs`, `core/src/client.rs`, `model-provider`, `models-manager`, `codex-api`, `codex-client`, backend clients and local model providers. ADAPTER for selected inference; full native transport remains COUPLED; catalog is STAGED. | Inference takes an authorized immutable model request and produces ordered response/tool/usage/error events; session transport owns retry, cancellation and connection reuse. Catalog takes owner/config/version snapshots and returns model capabilities/cache state. Preserve ETag/offline refresh, static provider seeds and owner-epoch fences. Current v1 model frames have a 4 MiB limit; WIP model2 is not accepted. Catalog's host endpoint grants/reverse broker must be activated and tested before credentials or network behavior are delegated. |
| C06 Authentication and secrets | `codex-rs/login/src/component_auth.rs`, native login/AuthManager, `aws-auth`, `workload-identity`, `websocket-auth`, `user-verification`, `secrets`, `keyring-store`. ADAPTER for selected resolve/refresh; full authentication COUPLED. | Split credential acquisition/refresh, browser/device OAuth, identity validation, account/workspace policy and secret persistence into explicit interfaces. Keep secret handles and scoped host grants where possible, not raw credential transport. Account/workspace changes revoke prior component authority. Existing `KeyringStore` and `SecretsBackend` traits are native abstractions, not installed implementations. Native login/logout and OS credential store behavior need their own acceptance matrix. The compiled native workspace-policy prerequisite passes 256 login tests and one targeted App Server test; [evidence](verification/2026-10-01/P03_NATIVE_POLICY_EVIDENCE.md) records scope and source lineage. It does not extract auth or yet fence asynchronous credential, request or cache publication. The later [reload/acquisition slice](verification/2026-10-01/P03_NATIVE_RELOAD_POLICY_EVIDENCE.md) passes 271 login tests and adds policy-checked in-memory reload publication plus current cached-auth/factory acquisition; final scoped lint passed unchanged and formatting was mechanically reviewed. Credential/source-owner changes, persistence and request/catalog-cache lifetime remain unfenced by this slice, and full authentication stays COUPLED. |
| C07 Configuration and requirements | `codex-rs/config`, `config-schema`, `cloud-config`, `features`, `core/src/config`, `core/src/session/config_refresh.rs`. COUPLED. | Source layers, parse/migrate/edit, provenance, effective snapshots, profiles/features and managed requirements need separate input/output contracts. Bootstrap only needs enough fixed configuration to locate the component registry; complete configuration business logic is not a permanent kernel exception. Changes publish versioned snapshots and invalidate dependent services; policy grants must not outlive the configuration version authorizing them. |
| C08 Context, history and compaction | `codex-rs/core/src/context_manager`, `core/src/compact.rs`, `core/src/context`, `history`, prompt/context fragment crates, native message-history path dependency. COUPLED; `context-engine`, replay/worker/adapter and parity bridges are STAGED. | Separate pure reconstruction/reduction from live incremental context transactions, instruction rendering, token budgeting, local/remote compaction and retained authorization evidence. Inputs include history, settings and checkpoint versions; outputs include bounded ordered context plus retained evidence/IDs. Preserve no-history-rewrite and cache stability invariants. Stateful session ownership and commit/rollback/revocation are required before activating reconstruction. No current context contributor proves native context management was extracted. |
| C09 Tools and command execution | `codex-rs/tools`, `exec`, `exec-server`, `shell-command`, `shell-escalation`, `core/src/tools`, `core/src/unified_exec`. ADDITIVE external tools and ADAPTER selected overrides; native dispatcher/process runtime COUPLED. | Tool registry/schema/call/result and execution spawn/stdin/PTY/output/exit/interrupt/close are distinct contracts. Execution owns child/process groups, output limits and joined cancellation; approvals/sandbox grants come from C10 and environment selection from C07. Preserve streaming and long-running command semantics, process descendants, terminal dimensions and sleep inhibition. Packaging a calculator does not extract native shell/tool execution. |
| C10 Approval and sandbox policy | `codex-rs/execpolicy`, `sandboxing`, `network-proxy`, Linux/Windows/MXC sandbox crates, `bwrap`, `process-hardening`, guardian extensions; native core approval/exec-policy/network decision flows. COUPLED. | Approval requests and decisions, policy evaluation and OS enforcement are distinct implementations behind typed grants. Preserve command canonicalization, per-turn decisions, escalation, network enforcement, inherited environment policy and cancellation. Platform helpers need target-specific packages and tests. Kernel may mediate/enforce an authority verdict; policy rules, reviewers and sandbox implementations remain component targets. |
| C11 Code mode runtime | `codex-rs/code-mode`, `code-mode-host`, `code-mode-protocol`, `code-mode-runtime`, `v8-poc`. COUPLED. | Execution/session handles, bounded values, tool/host call bridge, isolate ownership, timeout, interruption and teardown. Separate code-mode planning/service from embedded engine choice. Needs C09/C10 capability mediation and tool-call cancellation, plus runtime-specific cleanup proof; an isolated Rust crate does not establish replaceability. |
| C12 Agent execution and collaboration | `codex-rs/core/src/agent`, `agent-identity`, `agent-roles`, `agent-graph-store`, `agent-message-board-client`, `ext/agent`, `ext/agent-message-board`. COUPLED. | Spawn/configure/send/wait/interrupt/close with explicit parent/child authority and durable graph/message ownership. Agent role policy and scheduling can be selected implementations. Depend on session/model/context/storage contracts; preserve child approvals, budget propagation, cancellation trees and recovery without orphaned agents. |
| C13 MCP and connectors | `codex-rs/codex-mcp/src/runtime.rs`, `connection_manager.rs`, `rmcp-client`, `connectors`, `ext/mcp`, `ext/connectors` and core call/exposure modules. COUPLED. | Config/bindings/catalog, startup/reconnect, calls/resources, elicitation, origin/authorization and joined shutdown. `McpRuntime`, `McpBinding` and `McpCatalogBuilder` are useful existing interfaces, not external component implementations. Preserve skill/MCP dependency resolution across the selected services. Installing an MCP server is not replacement of Codex's MCP connection manager. Preserve auth/config epochs and cancel pending startup and calls on owner release. |
| C14 Skills, instructions and marketplace | `codex-rs/skills`, `ext/skills/src/host_service.rs`, `core/src/agents_md_manager.rs`, `core-plugins`, `core-plugin-common`, `plugin`, `utils/plugins`. COUPLED. | Skill roots/parse/catalog/cache/render, embedded system skills, AGENTS/instruction discovery and dynamic selectors require owned snapshots and invalidation. Existing ordinary marketplace/plugin install/update/remove/config/auth is a separate native subsystem from the new engine component manager. Define installation transactions and resource ownership; preserve symlink/path rules and rollback. Include embedded resources in independently built packages. |
| C15 Memory and history notes | `codex-rs/ext/memories/src/backend.rs`, `local.rs`, `extension.rs`; `memories/read`, `memories/write`, `ext/history-notes`, auxiliary state memory APIs. COUPLED. | Existing `MemoriesBackend` is a useful leaf boundary for note add/list/read/search, but its local implementation and prompt read path are still built in. Broader memory includes generation phase1/phase2, extraction/synthesis prompts, read readiness/versioning, jobs/leases/heartbeats and persistence. Define cancellation and claim recovery separately from memory I/O. Depends on context/model/storage/config; preserve scope and path protections. |
| C16 Goals and queued work | `codex-rs/ext/goal`, `ext/queue`, `state/src/runtime/goals.rs`, `queued_items.rs`; existing `GoalStore` and `SqliteQueueStore`. COUPLED. | Goal lifecycle/progress and queue enqueue/claim/ack/retry/cancel are separate domain contracts. State ownership, transactional claims and durable recovery belong to the selected service, while session task execution consumes authorized work. Shutdown releases or persists leases; restart must not lose or duplicate queued actions. |
| C17 Hooks | `codex-rs/hooks`, `core/src/hook_runtime.rs`, `core/src/hook_mcp_executor.rs`. COUPLED. | Ordered event subscription, hook configuration/input/result/decision, process or MCP invocation, timeout and cancellation. Preserve hook order, policy effects, failures and ownership; dependent tool/MCP work must terminate with its session. Upstream lifecycle contributors do not replace the native hook implementation by themselves. |
| C18 Files, search, Git and migration/import | `codex-rs/file-system`, `file-search`, `file-watcher`, `git-utils`, `worktree`, `utils/git-discovery`, `apply-patch`, `ext/git-attribution`, `external-agent-migration`; `app-server/src/fuzzy_file_search.rs`. VERIFIED_NATIVE bounded search subset; remaining C18 behavior COUPLED. | Filesystem and patch operations require roots/capabilities and atomicity; search/index/query and watch streams require revisions, cancellation and complete close; Git/worktree operations need explicit repository identity and ownership. Native bounded traversal/matching is VERIFIED_NATIVE for standalone CLI, App Server and TUI/GUI consumers. Main c28a1c33 adds verified host/process Preparing cancellation with retained cleanup. Pending-Open manager shutdown, native-constructor runtime proof, further GUI retirement triggers and private storage lookup remain distinct gaps. The rest of C18 remains COUPLED. Preserve ignore/filter/fuzzy matching, watch invalidation, Git diff/attribution, worktree create/remove and external import semantics. Search/index functionality inside storage or Git discovery is covered jointly with C02/C03, not omitted for lack of an index crate. |
| C19 Presentation and application service | `codex-rs/app-server`, `app-server-transport`, daemon/client/protocol, CLI/TUI; `component-sdk/examples/desktop`. Native frontends and App Server are COUPLED; the new independently packaged GUI is ADDITIVE. | Presentation contract covers session create/resume/list, input, ordered streaming, tool progress, approval responses, interruption and durable recovery. Preserve CLI/headless operation with GUI absent. App Server request processing/composition and native TUI/terminal presentation remain separate extraction targets; the GUI's use of actual App Server is not extraction of that server. Manager Launch must allow gateway → App Server → storage cleanup and report uncertainty after forced kill. |
| C20 Cloud tasks and remote products | `codex-rs/cloud-tasks`, `cloud-tasks-client`, backend clients, `responses-api-proxy`; remote control under `app-server-transport/src/transport/remote_control` and CLI/daemon/request-processor callers. COUPLED. | Cloud task create/list/read/stream/cancel and remote device enroll/pair/client authorize/revoke/enable/disable/reconnect require separate contracts. Remote control owns connection loops and enrollment state (C03), tied to identity epochs (C06), network grants (C21) and session events (C19). Preserve backoff, pairing cancellation and disable cleanup. This is Codex product functionality, distinct from the cloud environment used to develop this project. |
| C21 Transport and OS socket bridges | `codex-rs/http-client`, `websocket-client`, `stdio-to-uds`, `tcp-tunnel`, `uds`, TLS/stream-parser utilities. COUPLED services; SUPPORT helper libraries. | Connection establishment, proxy/TLS policy, frames/flow control, retries, cancellation and close. A consuming component can own a transport helper library, but any host-shared connection pool or routing policy needs an explicit service contract. Unix socket/TCP/stdio bridges and platform security assumptions must remain testable when independently packaged. |
| C22 Voice and realtime | `codex-rs/voice-host`, `realtime-webrtc`, `utils/audio`, `core/src/realtime_conversation.rs`, `realtime_context.rs`, `realtime_history.rs`. COUPLED. | Device/session/audio-frame/transcript/realtime-event contracts, bounded buffers and clock ownership, interruption and device/network teardown. Distinguish local capture/playback/voice host from realtime model signaling and context/history integration. Preserve permission/device errors and shutdown across supported OS targets; no runtime extraction proof exists yet. |
| C23 Telemetry and diagnostics | `codex-rs/analytics`, `diagnostics`, `feedback`, `otel`, `otel-trace-websocket`, `rollout-trace`, auxiliary state logs. COUPLED exporters/collectors; SUPPORT measurement helpers. | Structured events/spans/metrics, feedback bundles, log retention/query/export and explicit consent/redaction configuration. Exporters own queues and bounded flush budgets; forced exit reports dropped/uncertain export separately from durable session writes. Preserve privacy filters and tracing identities across component boundaries; telemetry must not become a hidden mandatory runtime dependency. |
| C24 Image and web capabilities | `codex-rs/ext/image-generation`, `ext/web-search`, `utils/image`, `core/src/web_search.rs`, `core/src/image_preparation.rs`. COUPLED. | Search/image requests and typed results with references/attachments, cancellation and provider capability limits. Native image preparation/detail budgeting and search configuration/response handling remain extraction targets. Depend on model, attachment, network and approval contracts where applicable; these native extension crates are not already independently installable engine components. |
| C25 Shared schemas and utilities | `codex-rs/protocol`, path/value/cache/async/readiness/string helpers. SUPPORT. | Stable schemas and deterministic value transforms can remain libraries shared by host and SDK. Versioned wire schemas need compatibility tests. Any mutable cache, background task or readiness owner used as a service must be assigned to C00–C24 with close/cancel semantics; SUPPORT is not a license for a hidden singleton. |
| C26 Build/test and distribution support | Workspace build metadata, proc macros, test clients/support and sample harness; root Bazel/Cargo/CI/scripts and SDK packaging. SUPPORT. | Maintain separate host and external component build/package recipes, reproducible manifests, platform assets, notice propagation and conformance fixtures. Build/test helpers are not runtime plugin targets, but packaging must prove components compile outside the host source and install after the host build without changing its hash. Preserve upstream license and third-party notices. |

### Auxiliary state ownership checklist

The current `codex-rs/state/src/runtime` modules with production domain behavior are: `threads.rs`, `thread_metadata.rs`, `projects.rs`, `thread_sections.rs`, `thread_section_order.rs`, `thread_attachments.rs`, `goals.rs`, `queued_items.rs`, `memories.rs`, `memory_readiness.rs`, `memory_versions.rs`, `logs.rs`, `logs_maintenance.rs`, `remote_control.rs`, `external_agent_config_imports.rs`, `rollout_migration.rs`, `backfill.rs`, `reclamation.rs` and `recovery.rs`. The SQLite migrations and ownership/transaction machinery are part of the extraction, not merely implementation detail to leave reachable through an untyped escape hatch. Thread payload persistence and this auxiliary database have different owners today and must not be conflated in completion reporting.

### C27 Upstream maintenance and update capability

**ADDITIVE maintenance support implemented; updater planned and unimplemented; no native-extraction claim.** The project must support bringing later official Codex changes into the compartmentalized distribution while preserving independently installed components and existing sessions. This is a required delivery track, not an optional future feature. Its design and recorded provenance are in `UPSTREAM_MAINTENANCE.md` and `upstream/lineage.json`.

The separately installable maintenance component should take an exact upstream revision, the current imported-source lineage, local extraction changes and installed component compatibility information. It should produce a reviewable change classification, an isolated candidate build/package, compatibility and regression evidence, and an explicit activation/rollback result. It must detect upstream changes to owned boundaries and shared schemas rather than assuming a clean textual merge proves behavior. Reuse the C00 manifest/version/dependency contracts and C26 build/conformance tools; require C02/C03 persistence compatibility and C19 client lifecycle checks. Preserve licenses, notices, plugin configuration, user data and the last known working distribution. No automatic publication, destructive replacement or silent migration is implied by this design.

A small external recovery/bootstrap path is a justified proposed SUPPORT/kernel exception: it must be able to locate and restore a previously accepted distribution when the candidate host or updater cannot start. It should have minimal checkpoint/integrity and activation responsibilities, with a versioned on-disk recovery record independent of the failing host's runtime ABI. Upstream fetch/diff/porting rules, compatibility policy, migration planning and test orchestration remain owned by the replaceable maintenance component or explicit build services; they are not a reason to retain a second full harness inside bootstrap. Activation must join active owners or occur after shutdown, stage changes without disturbing the active distribution, and distinguish completed rollback from uncertain data durability.

Acceptance must exercise a real later pinned upstream revision in an isolated candidate, explain changes to extracted boundaries, retain an already installed external component without rebuilding that component when its API stays compatible, and reject incompatible versions clearly. Prove both successful activation/recovery and failed-candidate rollback with preserved sessions/configuration. Until that exists, the updater execution and recovery gates remain unverified. Current source mapping and the read-only checker provide the bounded metadata evidence described below. C27 currently has no Rust workspace member, so adding this requirement does not itself add a member to the ledger below.

C27 has two separately reported workstreams. **Current P00M provenance:** close
and maintain the normalized original-to-current boundary/path/contract/evidence
index described in [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md#current-provenance-closure).
Exact historical path/blob inventories and selected anchors already exist; they
are not yet a complete current semantic ownership index. **Future P18U delivery:**
implement and independently install the updater, then demonstrate a real later
upstream candidate, incompatible-candidate rejection and external-bootstrap
recovery after failed activation. Documentation or native provider prerequisites
do not satisfy those execution gates or create another VERIFIED_NATIVE subsystem.
The [read-only index/checker](upstream/CHECKPOINT_LINEAGE_README.md) is now implemented
as project-authored maintenance support. Its expanded 22 fixtures pass; actual immutable-object
checks cover the pinned d04 source/all 28 map shapes with zero invalid metadata and 1,167 unresolved
source, semantic and acceptance findings. It is neither an installed updater nor native extraction.

## P03 checkpoint — 2026-10-02

Native extraction coverage is unchanged: thread storage/manual migration, the inline
attachment subset, and bounded native file search. The GUI remains an independently
packaged additive client. The published `706bf9e` checkpoint adds a task-ownership
library, with552 scoped tests and exact [provenance](upstream/p03-featured-warmup-stage-a-lineage.json);
it does not activate a production replacement or advance a family to VERIFIED_NATIVE.
The subsequent published WIP `2ce48a6` replaces one detached featured-cache warmup in
Codex's pre-existing plugin-marketplace service. Its Core Plugins suite passed553
cases; the current App Server suite passed432 tests plus separate public-Drop/
same-home shutdown and three curated replacement cases. The rebuilt production CLI
now passed storage, four migrations, both GUI modes, both slow Launch modes and
all eight held Git/HTTP cases plus the added task-receipt postcheck. Exact
[current evidence](verification/2026-10-02/p03-featured-warmup-current-production/README.md)
keeps reused independent packages, deterministic inference and browser fallback
limits explicit. [Fresh attachment caller proof](verification/2026-10-02/p03-current-attachment/README.md) now covers five groups/17 commands on that unchanged host, including installed native upload and custom file-reference cold resume without reupload. Native state was empty; production resolve, native result envelope and stored-blob durability remain unproven. CLI focused retry03 passed3 tests; exec/TUI scopes and remaining lint are pending.
This is lifecycle support within coupled native services, distinct from our custom
component installation mechanism. Shared HTTP internals, neighboring startup tasks,
MCP custody and the remaining P03 contracts retain their existing open status.
Use [EXECUTION_STATE.md](EXECUTION_STATE.md) for later exact runs; historical tests
and source maps must not be relabelled as proof for current working code.

The later C06 reload change extends the native in-memory publication check to the
committed auth-source identity, including equal-credential replacement and ABA.
Its five causal tests,297 full grouped login tests and108 grouped provider tests
passed on source map `0742356b`; scoped login lint03 subsequently passed with unchanged source and no new OOM. A rebuilt host remains a separate gate. See the
[source map](upstream/p03-auth-reload-source-lineage.json) and
[focused evidence](verification/2026-10-02/p03-auth-reload-focused/README.md).
This does not extract authentication. Permanent refresh-failure publication is a
staged follow-up; successful refresh/install persistence, conditional backend
writes and request admission remain coupled and require separate contracts.

## Complete explicit workspace package ledger

Every entry below is `package-name (path relative to codex-rs/)`, read from its actual Cargo manifest. All 169 explicit workspace members are accounted for in this ledger and its dated additions below. Shared implementation dependencies can cross rows; the ledger assigns inventory ownership, not an exclusive dependency graph. Status details above take precedence over a row-level label.

| ID / owner | Status | Explicit workspace packages |
| --- | --- | --- |
| C00 Component composition and extension interfaces | ADAPTER / ADDITIVE; SUPPORT contracts | `codex-component-api` (`component-api`); `codex-component-host` (`component-host`); `codex-component-adapters` (`component-adapters`); `codex-component-path-codec` (`component-path-codec`); `codex-component-state-codec` (`component-state-codec`); `codex-extension-api` (`ext/extension-api`); `codex-extension-items` (`ext/items`) |
| C01 Session, turn and task orchestration | COUPLED; STAGED lifecycle work | `codex-core` (`core`); `codex-core-api` (`core-api`) |
| C02 Thread storage, rollout persistence and maintenance | VERIFIED_NATIVE operations + manual migration; remaining maintenance COUPLED | `codex-thread-store` (`thread-store`); `codex-thread-store-component` (`thread-store-component`); `codex-thread-store-local-plugin` (`thread-store-local-plugin`); `codex-rollout` (`rollout`) |
| C03 Auxiliary state and durable domain repositories | COUPLED | `codex-state` (`state`) |
| C04 Attachments and upload routing | VERIFIED_NATIVE inline upload/resolve subset; COUPLED remote uploads | `codex-attachment-store` (`attachment-store`); `codex-attachment-store-api` (`attachment-store-api`); `codex-attachment-store-inline` (`attachment-store-inline`); `codex-attachment-store-component` (`attachment-store-component`) |
| C05 Inference transport, provider selection and model catalog | ADAPTER inference subset; STAGED catalog; COUPLED native implementations | `codex-model-provider` (`model-provider`); `codex-model-provider-info` (`model-provider-info`); `codex-models-manager` (`models-manager`); `codex-client` (`codex-client`); `codex-api` (`codex-api`); `codex-backend-client` (`backend-client`); `codex-backend-openapi-models` (`codex-backend-openapi-models`); `codex-lmstudio` (`lmstudio`); `codex-ollama` (`ollama`); `codex-utils-oss` (`utils/oss`) |
| C06 Authentication, identity, secrets and account policy | ADAPTER credential subset; COUPLED native authentication | `codex-login` (`login`); `codex-aws-auth` (`aws-auth`); `codex-secrets` (`secrets`); `codex-keyring-store` (`keyring-store`); `codex-workload-identity` (`workload-identity`); `codex-websocket-auth` (`websocket-auth`); `codex-user-verification` (`user-verification`); `codex-utils-redacted-string` (`utils/redacted-string`) |
| C07 Configuration, requirements, features and managed settings | COUPLED; SUPPORT schemas | `codex-config` (`config`); `codex-config-schema` (`config-schema`); `codex-features` (`features`); `codex-cloud-config` (`cloud-config`); `codex-home` (`codex-home`); `codex-utils-home-dir` (`utils/home-dir`); `codex-utils-json-to-toml` (`utils/json-to-toml`) |
| C08 History, context reconstruction, instructions and compaction | COUPLED; STAGED replay extraction | `codex-history` (`history`); `codex-context-fragments` (`context-fragments`); `codex-prompts` (`prompts`); `codex-collaboration-mode-templates` (`collaboration-mode-templates`); `codex-guardian-context` (`guardian-context`); `codex-response-debug-context` (`response-debug-context`); `codex-utils-output-truncation` (`utils/output-truncation`); `codex-utils-template` (`utils/template`) |
| C09 Tool dispatch and native command execution | ADDITIVE tools; ADAPTER selected tool override; COUPLED native execution | `codex-tools` (`tools`); `codex-exec` (`exec`); `codex-exec-server` (`exec-server`); `codex-exec-server-protocol` (`exec-server-protocol`); `codex-shell-command` (`shell-command`); `codex-shell-escalation` (`shell-escalation`); `codex-utils-process` (`utils/process`); `codex-utils-pty` (`utils/pty`); `codex-utils-sleep-inhibitor` (`utils/sleep-inhibitor`) |
| C10 Approvals, execution policy and OS/network sandboxing | COUPLED | `codex-sandboxing` (`sandboxing`); `codex-execpolicy` (`execpolicy`); `codex-network-proxy` (`network-proxy`); `codex-linux-sandbox` (`linux-sandbox`); `codex-mxc-sandbox` (`mxc-sandbox`); `codex-bwrap` (`bwrap`); `codex-process-hardening` (`process-hardening`); `codex-windows-sandbox-service` (`windows-sandbox-service`); `codex-guardian-reviewer` (`ext/guardian-reviewer`); `codex-guardian-v2` (`ext/guardian-v2`); `codex-utils-sandbox-summary` (`utils/sandbox-summary`); `codex-utils-approval-presets` (`utils/approval-presets`) |
| C11 Code mode and embedded execution runtime | COUPLED | `codex-code-mode` (`code-mode`); `codex-code-mode-host` (`code-mode-host`); `codex-code-mode-protocol` (`code-mode-protocol`); `codex-code-mode-runtime` (`code-mode-runtime`); `codex-v8-poc` (`v8-poc`) |
| C12 Agents, collaboration and message coordination | COUPLED | `codex-agent-identity` (`agent-identity`); `codex-agent-roles` (`agent-roles`); `codex-agent-graph-store` (`agent-graph-store`); `codex-agent-message-board-client` (`agent-message-board-client`); `codex-agent-extension` (`ext/agent`); `codex-agent-message-board-extension` (`ext/agent-message-board`) |
| C13 MCP and connector runtime | COUPLED | `codex-mcp` (`codex-mcp`); `codex-rmcp-client` (`rmcp-client`); `codex-connectors` (`connectors`); `codex-mcp-extension` (`ext/mcp`); `codex-connectors-extension` (`ext/connectors`) |
| C14 Skills, instruction discovery and ordinary plugin marketplace | COUPLED | `codex-skills` (`skills`); `codex-core-plugins` (`core-plugins`); `codex-core-plugin-common` (`core-plugin-common`); `codex-plugin` (`plugin`); `codex-utils-plugins` (`utils/plugins`); `codex-skills-extension` (`ext/skills`) |
| C15 Memory reading, writing, jobs and history notes | COUPLED | `codex-memories-read` (`memories/read`); `codex-memories-write` (`memories/write`); `codex-memories-extension` (`ext/memories`); `codex-history-notes-extension` (`ext/history-notes`) |
| C16 Goals and queued work | COUPLED | `codex-goal-extension` (`ext/goal`); `codex-queue-extension` (`ext/queue`) |
| C17 Hooks and event-triggered execution | COUPLED | `codex-hooks` (`hooks`) |
| C18 Filesystem, search, watch, Git, worktrees and import | VERIFIED_NATIVE bounded search subset; remaining filesystem/watch/Git/worktree/import COUPLED | `codex-file-system` (`file-system`); `codex-file-search` (`file-search`); `codex-file-search-api` (`file-search-api`) [SUPPORT contract]; `codex-file-watcher` (`file-watcher`); `codex-git-utils` (`git-utils`); `codex-worktree` (`worktree`); `codex-utils-git-discovery` (`utils/git-discovery`); `codex-git-attribution` (`ext/git-attribution`); `codex-apply-patch` (`apply-patch`); `codex-external-agent-migration` (`external-agent-migration`); `codex-utils-fuzzy-match` (`utils/fuzzy-match`) |
| C19 CLI, TUI, App Server, daemon and presentation clients | COUPLED native frontends; ADDITIVE GUI | `codex-app-server` (`app-server`); `codex-app-server-transport` (`app-server-transport`); `codex-app-server-daemon` (`app-server-daemon`); `codex-app-server-client` (`app-server-client`); `codex-app-server-protocol` (`app-server-protocol`); `codex-tui` (`tui`); `codex-cli` (`cli`); `codex-ansi-escape` (`ansi-escape`); `codex-terminal-detection` (`terminal-detection`); `codex-mermaid` (`mermaid`); `codex-arg0` (`arg0`); `codex-install-context` (`install-context`); `codex-utils-cli` (`utils/cli`) |
| C20 Cloud tasks and provider proxy products | COUPLED | `codex-cloud-tasks` (`cloud-tasks`); `codex-cloud-tasks-client` (`cloud-tasks-client`); `codex-responses-api-proxy` (`responses-api-proxy`) |
| C21 Network transports, socket bridges and streaming plumbing | COUPLED / SUPPORT | `codex-http-client` (`http-client`); `codex-websocket-client` (`websocket-client`); `codex-stdio-to-uds` (`stdio-to-uds`); `codex-tcp-tunnel` (`tcp-tunnel`); `codex-uds` (`uds`); `codex-utils-rustls-provider` (`utils/rustls-provider`); `codex-utils-stream-parser` (`utils/stream-parser`) |
| C22 Voice, realtime and audio platform integration | COUPLED | `codex-voice-host` (`voice-host`); `codex-realtime-webrtc` (`realtime-webrtc`); `codex-utils-audio` (`utils/audio`) |
| C23 Telemetry, diagnostics, feedback and trace storage/export | COUPLED / SUPPORT | `codex-analytics` (`analytics`); `codex-diagnostics` (`diagnostics`); `codex-feedback` (`feedback`); `codex-otel` (`otel`); `codex-otel-trace-websocket` (`otel-trace-websocket`); `codex-rollout-trace` (`rollout-trace`); `codex-utils-elapsed` (`utils/elapsed`) |
| C24 Image generation, image handling and web search | COUPLED | `codex-image-generation-extension` (`ext/image-generation`); `codex-web-search-extension` (`ext/web-search`); `codex-utils-image` (`utils/image`) |
| C25 Shared protocol, value types and runtime utilities | SUPPORT; no blanket functional exemption | `codex-async-utils` (`async-utils`); `codex-protocol` (`protocol`); `codex-utils-absolute-path` (`utils/absolute-path`); `codex-utils-path-uri` (`utils/path-uri`); `codex-utils-cache` (`utils/cache`); `codex-utils-readiness` (`utils/readiness`); `codex-utils-string` (`utils/string`); `codex-utils-path` (`utils/path-utils`) |
| C26 Build, test, schema-generation and sample helpers | SUPPORT | `codex-build-info` (`build-info`); `codex-app-server-protocol-noop-macros` (`app-server-protocol-noop-macros`); `codex-app-server-test-client` (`app-server-test-client`); `codex-cloud-tasks-mock-client` (`cloud-tasks-mock-client`); `codex-exec-server-test-support` (`exec-server/tests/support`); `codex-windows-sandbox-test-support` (`windows-sandbox-rs/tests/support`); `codex-utils-cargo-bin` (`utils/cargo-bin`); `codex-test-binary-support` (`test-binary-support`); `codex-thread-manager-sample` (`thread-manager-sample`); `codex-experimental-api-macros` (`codex-experimental-api-macros`) |

## Other Rust manifests and staged integration

The following five existing native path-dependency crates are present in workspace dependency declarations but are not explicitly listed in `workspace.members`. They are not new staged extraction work:

| Owner | Native package / path | Status and responsibility |
| --- | --- | --- |
| C05/C06 | `codex-chatgpt` (`codex-rs/chatgpt`) | COUPLED ChatGPT-specific client/account integration |
| C08 | `codex-message-history` (`codex-rs/message-history`) | COUPLED native message-history persistence/integration |
| C10 | `codex-windows-sandbox` (`codex-rs/windows-sandbox-rs`) | COUPLED native Windows sandbox implementation and platform assets |
| C26 | `core_test_support` (`codex-rs/core/tests/common`) | SUPPORT core integration-test harness |
| C26 | `app_test_support` (`codex-rs/app-server/tests/common`) | SUPPORT App Server integration-test harness |

Six unregistered primary package manifests are STAGED. A copied source tree or old evidence file does not turn these into an active or accepted component:

| Owner | Staged packages | Activation and evidence still required |
| --- | --- | --- |
| C08 | `codex-rs/context-engine`, `context-replay`, `context-replay-component`, `context-replay-native-plugin` | Register and build leaves, activate core replay bridge and component selection, run native differential/parity fixtures and an external-worker acceptance test. `context-engine/NEXT_INTEGRATION.md` documents gates. Pure reconstruction is narrower than all live context management/compaction. Session startup must be fallible and owner release must close the worker. |
| C05 | `codex-rs/model-catalog-api`, `model-catalog-native-plugin`; unlinked `models-manager/src/component*` and `model-provider/src/component_catalog*` modules | Follow `model-provider/COMPONENT_CATALOG_ACTIVATION.md`; activate dependency broker/session/spool integration, endpoint grants and authority fencing before replacing catalog selection. Preserve native provider seeds, ETag/cache/offline/auth-change behavior and test independent installation. |

P02 supplement, 2026-10-01: the original table accounts for 166 members;
main `a469cf4be85fda40c64e563ece3e1639e4abdbac` adds the three search packages
below, bringing accepted source to 169. The actual native traversal/matcher now
runs through an independently built/installed worker in standalone CLI, App Server,
TUI and the additive GUI file picker. See [standalone evidence](verification/2026-10-01/P02B_SELECTED_SEARCH_EVIDENCE.md),
[App Server evidence](verification/2026-10-01/P02B_APP_SERVER_CONSUMER_EVIDENCE.md)
and [TUI/full CLI evidence](verification/2026-10-01/P02B_TUI_CONSUMER_EVIDENCE.md).
The latter includes 5,620 passing TUI tests and real terminal, storage/migration and
GUI checks with immutable host/package hashes and manager-only Ctrl+C drainage.
App Server/TUI themselves remain compiled adapters. The bounded Preparing milestone is
now promoted to main `c28a1c33a856a987316f4b97b4c488d68055fffc`, with its actual
gates and limitations below. Private rollout lookup remains queued. C18 as a whole
is not extracted.

The subsequent worker04 package is independently built at version 0.2.0 with
unchanged wire contract 1. Its [installed old-host compatibility gate](verification/2026-10-01/P02B_WORKER04_OLD_HOST_COMPAT_EVIDENCE.md)
passed without rebuilding the accepted full CLI. New required Rust source API2,
native/process/runtime pending controls and App Server consumer changes have
separate focused evidence. Public Stop and TUI consumer libraries now pass6095 combined tests
(4skipped). The new fullCLI build and installed legacy/public sessionStop Preparing gates pass;
old0.1 and new0.2 worker packages remain unchanged. Normal storage/migration/GUI also passed
separately. TUI integration passed23/4skipped; instrumented GUI clear-query Preparing passed
both cold-launch cycles and subsequent manager shutdown. Exact limits remain in
[EXECUTION_STATE.md](EXECUTION_STATE.md). Package capability metadata
is a declaration, not proof of those new host behaviors. Preserve the accepted
0.1.0 package and its original runtime evidence independently.

| Owner | Newly accepted workspace member | Classification |
| --- | --- | --- |
| C18/C00 | `codex-file-search-component` (`file-search-component`) | SUPPORT/ADAPTER: bounded wire, process adapter, ownership and selection boundary. |
| C18 | `codex-file-search-local-plugin` (`file-search-local-plugin`) | VERIFIED_NATIVE: separately built native traversal/matcher packages 0.1.0 and 0.2.0, unchanged wire contract 1; old-package compatibility retained. Host/process Preparing is verified within the stated limits; remaining startup cases are explicit follow-up. |
| C18/C19 | `codex-file-search-runtime` (`file-search-runtime`) | ADAPTER: selected provider/scopes and CLI composition; not extraction of the whole client. |

| Owner | Additional source | Classification and scope |
| --- | --- | --- |
| C18/C26 | `codex-rs/third-party/nucleo/Cargo.toml`, `matcher/Cargo.toml`; retained matcher fuzz manifest | SUPPORT/native dependency, moved from the exact pinned Git dependency to local source with owned-pool/shutdown changes. MPL-2.0 licenses and provenance are retained. Vendoring is not component extraction. |
| C18/C00 | `codex-rs/file-search-api`, `codex-rs/component-path-codec` | Registered SUPPORT crates; 11 neutral API, 8 shared-path and 37 existing state-codec tests passed. Native path serialization now lives in the shared crate with existing state-codec compatibility exports. That initial support-only checkpoint did not prove installed selection; the later native milestone above does. Earlier unregistered drafts are preserved at WIP `17c366bb94f77b3d3895acd6f061a5cb308bd384`; they were excluded from StageA. This supporting-library refactor is not independently installed native search. |

The P02 component plan and its native-backend/composition annexes document all four
search consumers, explicit ownership and remaining activation gates. The private
storage search fallback remains pending dependency injection; client search
replacement alone cannot make that path replaceable.

The isolated WIP checkout adds `codex-rs/model-transport-api` (STAGED), model2 transport/worker paths, runtime lifecycle/cleanup work and thread-store migration protocol/client work. Its copied baseline reports apply to the older baseline only. In particular, migration wire variants and client calls do not supply the missing server dispatch/lease registry/CLI composition. See `component-sdk/MIGRATION_COMPONENT_PLAN.md`, `MODEL_TRANSPORT_V2_PROPOSAL.md`, `MODEL_TRANSPORT_V2_DOMAIN.md` and `SESSION_RUNTIME_LIFECYCLE_PLAN.md`; none of those proposals alone proves implementation. Primary broker activation patches under `component-sdk/` likewise remain STAGED until integrated and tested.

## Non-Rust distributions and executable surfaces

The manifest ledger below includes all nine non-Rust package manifests found outside generated caches and vendored code. These distributions participate in the same boundary contracts; Python/TypeScript/npm packaging is not a substitute for native extraction.

| Owner | Manifest / distribution | Responsibility and status |
| --- | --- | --- |
| C26 | `package.json` — `codex-monorepo` | Root monorepo tooling/distribution coordination; SUPPORT |
| C19/C26 | `codex-cli/package.json` — `@openai/codex` | npm CLI launcher and platform binary packaging; COUPLED launcher / SUPPORT distribution |
| C20/C26 | `codex-rs/responses-api-proxy/npm/package.json` — `@openai/codex-responses-api-proxy` | Proxy executable npm distribution; COUPLED proxy / SUPPORT distribution |
| C26 | `scripts/pyproject.toml` — `codex-scripts` | Build/release/package helper tools; SUPPORT |
| C26 | `scripts/codex_package/smoke_tests/pyproject.toml` — `codex-package-smoke-tests` | Distribution smoke tests; SUPPORT |
| C19/C26 | `sdk/python-runtime/pyproject.toml` — `openai-codex-cli-bin` | CLI binary Python distribution; SUPPORT packaging of native runtime |
| C19 | `sdk/python/pyproject.toml` — `openai-codex` | Existing Python SDK client surface; COUPLED to supported runtime API, not a native engine plugin |
| C19 | `sdk/typescript/package.json` — `@openai/codex-sdk` | Existing TypeScript SDK client surface; COUPLED to supported runtime API, not a native engine plugin |
| C00/C19/C26 | `component-sdk/pyproject.toml` — `codex-component-sdk` | New component developer SDK/build/package/templates; ADAPTER infrastructure / ADDITIVE examples / SUPPORT conformance |

Additional executable/assets surfaces without their own manifest:

- **C19 ADDITIVE GUI:** `component-sdk/examples/desktop/plugin.py`, `desktop_ui/gateway.py` and `desktop_ui/static/` implement the separately packaged client. Its gateway owns App Server subprocess/stream lifecycle; frontend owns presentation state and reconnect/recovery interactions. `component-sdk/tests/gui_harness_fixture.py`, `gui_browser_driver.cjs`, `test_desktop.py` and actual manager shutdown evidence cover this new client. Official desktop UI source is not assumed available or reused.
- **C00/C26 SDK and examples:** `component-sdk/codex_component_sdk`, `rust_component_package.py`, calculator/model fixture examples, build/package recipes and `component-sdk/tests/` cover authoring, validation and conformance. Example functionality is ADDITIVE; native storage packages are separately identified above.
- **C19 existing clients:** `codex-cli/`, `sdk/python/`, `sdk/python-runtime/`, `sdk/typescript/`, app-server-generated schemas and TUI assets must keep supported public APIs, CLI flags, streaming and resume behavior. Their language/package boundaries do not prove independent engine replacement.
- **C10/C21/C22 native platform bridges:** `codex-rs/windows-sandbox-rs`, Windows sandbox service, Linux sandbox/bwrap, socket bridges and audio/WebRTC native bindings/assets remain in scope. Each target needs its own dependency/license/packaging and runtime validation; Linux evidence does not establish Windows/macOS behavior.
- **C26 build/test/release:** `.github/`, `.cargo/`, `bazel/`, `scripts/`, `tools/`, `patches/`, `third_party/`, root build and lock files, Rust build scripts/Bazel targets, fixtures/snapshots/benchmarks and generated bindings are SUPPORT. They must carry source/resources/NOTICE correctly into independently built component packages and remain reproducible. Vendored implementations follow the functionality of their consuming boundary, not an exemption from scope.

## Dependency order and extraction acceptance

Use these as dependency constraints under the roadmap phases, not a promise to activate every staged source at once:

1. Establish the manifest/compatibility/capability and single-owner shutdown contracts (C00/C01); retain and regress the accepted storage/GUI baseline (C02/C04/C19). Complete the existing storage migration/maintenance route before claiming whole storage is replaceable.
2. Define typed auxiliary state repositories and configuration/identity snapshots (C03/C07/C06). In parallel, pure context reconstruction and bounded native leaf services can be integrated after their shared contracts stabilize (C08, memory I/O in C15, search in C18). Each still needs a real selected caller and shutdown proof.
3. Activate scoped host-dependency grants and authority fencing before catalog/transport/authentication replacement (C05/C06). Connect live context, tool/process execution and approvals/sandbox services using those stable contracts (C08–C11).
4. Extract higher-order session/agent, skills/marketplace, MCP, memory jobs, goals/queue and hooks with their dependencies already explicit (C01/C12–C17). Regress the native CLI, App Server and GUI every cycle that affects their events or lifecycle.
5. Complete the remaining file/Git/worktree, cloud/remote, transport, voice/realtime, telemetry, image/web and native presentation targets (C18–C24), then run platform/distribution/conformance work across the full ledger (C25/C26). Develop the required upstream maintenance/update component and independent recovery bootstrap (C27) alongside compatibility and distribution work, and exercise a later pinned upstream update before completion. Dependency ordering does not remove any of these requirements.

Every extraction needs explicit requests/results/events, state and transaction owner, dependency grants, startup readiness, errors, cancellation and joined shutdown. Its acceptance record must show the production call path selecting the external implementation; a package built outside the harness source, installed after the host build with unchanged host hash; native-versus-selected behavioral comparison; interruption and forced-stop behavior; cold recovery/removal/reselection; and relevant existing runtime regressions. A crate split, trait definition, contributor, mocked unit test or build alone does not satisfy that claim.

The proposed permanent kernel is limited to minimal accepted-distribution recovery/bootstrap, registry discovery, compatibility/dependency resolution, capability/authority mediation, request/owner/epoch identity, lifecycle supervision and event routing. Even that boundary must be reviewed after implementation. Native SQL repositories, model/cache policy, auth/account rules, configuration loaders, context reducers, scanners, MCP connections, memory jobs, approval decision implementations and remote-control loops remain extraction targets. Shared schemas and pure helpers may stay libraries with stable interfaces and owners; there is no requirement to make one plugin for each helper crate.

## Inventory audit

The accompanying local audit is `roadmap-inventory-audit.json` in the verified recovery backup directory. It records the actual manifest names/paths, exact-once coverage, additional native/staged packages, non-Rust manifests and existence checks. It is a source inventory audit, not a build or runtime test result. No package manager or compiler was invoked to produce this document.

## Native auth checkpoint: full-host regression

The source checkpoint `e289b6b5` (published with scoped evidence at `f9e922c3`)
also passed a fresh full CLI build and unchanged installed-package acceptance:
13 storage commands, 10 migration commands and two cold GUI cycles. Both GUI runs
terminated normally after first SIGINT to the actual component manager during an
active turn, with tracked processes absent and no forced cleanup.
[Runtime evidence and reviewed screenshots](verification/2026-10-01/P03_NATIVE_AUTH_FULL_HOST_EVIDENCE.md)
bind the exact source, executable and independently built packages. This uses
Chromium/Playwright and deterministic inference; in-app Browser and live-provider
proof remain unavailable. Existing native extraction coverage is unchanged, and
credential/source lifetime, broker/catalog activation and P18U upstream integration/
rollback remain required work. The subsequent cache-revision checkpoint has its
own scoped evidence below; it is excluded from this earlier runtime checkpoint.

C06's subsequent [credential-cache checkpoint](verification/2026-10-01/P03_CACHE_REVISION_EVIDENCE.md)
adds native stale-reload protection with 385 scoped passes, including retained
provider HTTP behavior. Extraction status and installed-component counts do not
change. Source ownership, installer ordering, persistent authority, selected
catalog activation remain separate gates. Fresh full-host/UI acceptance is recorded below.

## Current cache-revision host and GUI regression

[The runtime supplement](verification/2026-10-01/P03_CACHE_REVISION_FULL_HOST_EVIDENCE.md)
binds source `18140083910d19c86e9cadaef0990db9aa652f6e` and exact CLI
`876c826f76d574d9fee62011166b1ae3f4b15f69103f361aedb4e44cef08de71`.
The full build, installed storage, migration and two cold GUI cycles passed with
unchanged independently built packages. Both actual manager first-SIGINT shutdowns
exited 0 in about 0.27 seconds with tracked processes absent and no forced cleanup.
The ENOSPC attempt and unchanged fixture-isolation rejection remain failed evidence;
a fresh workspace migration fixture supported the successful GUI run. Assertions
were not weakened. Some adopted storage/migration descendants exited nonzero;
strict drains passed, but their executable/cause attribution is unknown.

This proves current same-upstream runtime regression for the recorded behaviors,
using Chromium/Playwright and deterministic inference. It does not increase native
extraction coverage, exercise a new attachment roundtrip, establish installed auth
or catalog selection, or satisfy P18U's real later-upstream integration and rollback.
In-app Browser and live-provider acceptance remain pending. Upstream stays pinned.

## Native source and credential ownership checkpoint

Source `99e6802fd478e55686559aefdd6ae79177a45397` pairs the active external auth
provider with credentials and failure metadata under one native owner. Failed
credential preparation preserves the previous in-memory pair; clear and provider
retirement observe the published state, with provider destruction outside locks.
The [scoped evidence](verification/2026-10-01/P03_SOURCE_OWNER_EVIDENCE.md) retains
four baseline assertion failures and **391 passing login/provider tests**, exact
post-test type-alias/formatting transitions and an unattributed SIGKILL child reap.
The [lineage](upstream/p03-native-source-owner-lineage.json) maps actual pinned
upstream methods separately from private owned-update and persistence helpers.

This is a compiled native ownership prerequisite, not installed authentication or
catalog extraction. Installer ordering, source ABA, load/refresh ownership and
shared persistence remain open. Its fresh full-host build, migration and GUI checks
now pass, but two storage lifecycle checks fail on surviving Git descendants; see
[the runtime supplement](verification/2026-10-01/P03_SOURCE_OWNER_FULL_HOST_EVIDENCE.md).
This source has no overall full-host acceptance. Extraction counts and the upstream
pin do not change. P18U's integration/recovery gates remain
required, with no active polling or live update.

## C05/C06 installation-ordering prerequisite

[Source `65511842`](verification/2026-10-01/P03_INSTALL_ORDER_EVIDENCE.md) adds ordered native auth installation
and clear invalidation; 400 scoped tests pass. This changes compiled ownership,
not the count of independently installed native subsystems. Source-aware reload,
refresh, persistent writes, catalog/broker activation and real installed-auth
acceptance remain open. Current full-host acceptance is also blocked by inherited
curated-sync shutdown; successful GUI checks on source `99e6802f` do not validate
this newer source. [Exact mapping](upstream/p03-native-install-order-lineage.json).

## Curated Git transport candidate and ownership gate

Source `7ab5dd77b87c2e6bf7040824e67bf6f22af6a073` lives on the separate
`work/p03-curated-sync-lifecycle` development branch; it is **not promoted to main**.
Main source remains the native installer checkpoint655 while documentation records this
candidate's progress. [Evidence](verification/2026-10-01/P03_CURATED_GIT_TRANSPORT_EVIDENCE.md)
preserves a failed link, three real baseline lifecycle failures and two541-pass scoped
runs, including the final explicit child-ownership repair, clean lint and separate
mechanical formatting. [Lineage](upstream/p03-curated-git-transport-lineage.json) binds
upstream primitives, native adaptations and new support without claiming an installed component.

Linux private groups/pipe bounds/direct-child reap are scoped native improvements. HEAD
lookup also enters the existing timeout path on non-Linux, which is untested. Private
groups escape the parent's process-group guard on abrupt host death. The current String
pipeline can still fall back/release protected state on uncertain cleanup, and manager
admission can reopen. Therefore B1a must couple typed outcomes with actual retained
cleanup resources, worker/admission ownership and manager integration **before** B1b
cancellable locks/Git. [Required next contract](verification/2026-10-01/P03_CURATED_B1_OWNERSHIP_REVIEW.md)
supersedes any queue that separates this ownership transition into unsafe intermediate
production changes. All full-host/UI gates and remaining extraction remain open.
P18U remains required and unimplemented; no polling/live update has started.
