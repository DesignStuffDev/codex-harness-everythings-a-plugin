# Codex Harness Compartmentalized: component inventory and acceptance plan

This fork is based on OpenAI Codex revision
`d42056091aded7feb1d88ac7e83972108b2aa478`; see
[UPSTREAM_PROVENANCE.md](UPSTREAM_PROVENANCE.md). The goal is to make the actual
Codex engine's functional subsystems replaceable through explicit component
contracts, including custom packages installed after the host has been built.
The host does not require Cordis. Existing skills, MCP packages, and an unchanged
Codex executable behind a wrapper do not establish engine compartmentalization.

This inventory records source inspection and implementation on 2026-09-30.
[VALIDATION.md](VALIDATION.md) records the focused tests, unchanged executable
hashes, actual CLI tool/context/model/recovery milestone, and GUI evidence.
Several narrow boundaries now have runtime evidence. Complete replacement of
every functional subsystem remains unfinished; neither the new infrastructure
nor an additive contributor is counted as whole-subsystem replacement.

[Storage contract version two](component-sdk/STORAGE_WIRE_V2.md) now preserves
trusted native state across the process boundary; the originally failing raw
persistence comparison passes in both history modes. The updated host rejects
storage contract 1 before activation. Native migration/compression ownership and
joining are also implemented: all 653 distinct storage/state/rollout cases passed
across the main run and a corrected test-fixture rerun, with clean final scoped
Clippy. The latest host gate passed all 56 tests, including persistent process
reaping and the presentation service lifetime; three actual manager signal cases
also passed with direct-child PIDs absent.
Exact run splits and limits are recorded in [VALIDATION.md](VALIDATION.md).

The storage-v2 checkpoint **passed**: an independently exported and built native
package was installed after its source was removed, then exercised through the
actual CLI and GUI without changing either host binary. Two actual manager Ctrl+C
shutdowns exited successfully, all tracked PIDs disappeared, and cold recovery
retained history and completed a new turn. See the [independent-build report](verification/2026-09-30/thread-store-native-v2-independent.json)
and [GUI report](verification/2026-09-30/gui-manager-native-storage-v2.json). Earlier
contract-1 evidence remains historical; neither GUI run deliberately held native
maintenance active. The separate SQLite-lock test covers that lifetime.
The new shared state codec is a domain serialization library; its tests do not
establish activation of the planned context/replay subsystem.

Model transport v1 also remains limited to 4 MiB per frame. An
[actual native-versus-component CLI comparison](verification/2026-09-30/model-wire-audit.md)
confirmed that larger history and inline-image requests work natively and fail
through that component. The proposed chunked model contract v2 addresses this
gap; it is not yet implemented or verified.

## What the current boundaries establish

The upstream workspace already contains substantial separation. Its
[core API](codex-rs/core-api/src/lib.rs) re-exports
`ExtensionRegistryBuilder`, `ThreadManager`, storage, authentication, and agent
types. The [extension registry](codex-rs/ext/extension-api/src/registry.rs)
collects typed in-process contributors. These are useful existing contracts;
they are neither a stable Rust dynamic-library ABI nor an external package
loader.

The authoritative inventory of live engine services is
[SessionServices](codex-rs/core/src/state/service.rs). It still owns concrete
model, execution, approval, MCP, plugin, telemetry, context-related, and network
services, alongside injected `AgentControl`, `ThreadStore`, `AttachmentStore`,
and other traits. Consequently, a crate boundary, a trait, and an independently
installed implementation are three distinct levels of separation.

| Status | Meaning in this document |
| --- | --- |
| Existing native contract | Upstream supplies a trait, protocol, or injection point. External installation and complete replacement remain to be implemented or demonstrated. |
| In progress / source present | New packages or adapters are being implemented in this branch. Integration, build, and runtime evidence must be recorded separately. |
| Planned | A required functional boundary has no external adapter in this phase. |
| Verified boundary | The specific contract has reproducible runtime evidence in [VALIDATION.md](VALIDATION.md). This does not imply every behavior of its larger subsystem has been extracted or tested. |

## Implemented component boundaries and remaining acceptance

| Package / boundary | Current source and responsibility | Evidence and remaining acceptance gap |
| --- | --- | --- |
| `codex-component-api` | [Manifest and settings types](codex-rs/component-api/src/lib.rs): API version, plugin identity/version, entrypoint, dependencies, component kind/name/contract version, configuration, enablement, and explicit selections. This is new infrastructure. | Host compatibility/error tests and SDK/package acceptance passed. An arbitrary kind string still does not constitute a working adapter. |
| `codex-component-host` | [Catalog](codex-rs/component-host/src/catalog.rs), [process transport](codex-rs/component-host/src/process.rs), [persistent sessions](codex-rs/component-host/src/session.rs), and [management API](codex-rs/component-host/src/lib.rs): discovery, dependency checks, explicit selection, process invocation, immutable install/remove. This is new infrastructure. | Latest focused gate: 56 passed together, one helper-only skip; scoped host/API Clippy passed. Startup, driver and close errors acknowledge direct-child reaping; cancelled startup keeps an independent cleanup owner. Existing large-body, accepted-write, abandoned-waiter, cleanup-capacity and immutable-binding regressions remain covered. [Latest scope](verification/2026-09-30/component-host-service-lifetime.json). Three actual manager signal cases and fresh v2 independent-build/CLI/GUI acceptance passed. Presentation startup is bounded, its ready service has no execution-lifetime cap, and exit cleanup has a full bounded grace. Cleanup deadlines report reaping/joins unconfirmed. Reaping does not establish durability, arbitrary descendants, legacy codec worker ownership or Windows behavior. |
| `codex-component-adapters` | [Registration](codex-rs/component-adapters/src/lib.rs) connects installed tools, contextual input, and lifecycle observers to native extension contracts. Tools can add capabilities or explicitly replace supported names; context/lifecycle are additive. | Five focused tests and actual CLI calculator/context execution passed. Native startup registers these adapters. The full tool executor, context manager, lifecycle state machine, and extension engine remain native. |
| Selected model transport | Explicit `model_transport:default` selection at [ModelClientSession](codex-rs/core/src/client.rs) replaces the existing inference stream implementation, carrying native request data and ordered response events. | Actual CLI tool execution, streamed response events, persisted resume, zero native inference while selected, and native restoration passed. All four engine tests passed together; see VALIDATION. Full event/provider/WebSocket parity remains. Catalog, capabilities, realtime, and unary memory summarization remain separate boundaries. Remote V2 compaction uses this seam; its native policy/validation and external behavior need further checks. |
| Selected credential provider | [External-auth adapter](codex-rs/login/src/component_auth.rs) selects `auth:default` during native `AuthManager` startup; acquisition and refresh feed existing login-policy validation and credential commit. | Six real-process tests passed for resolve/refresh, native restrictions, account ownership, error/debug redaction, workload-identity rejection, and deselection. All 249 login crate tests passed. OAuth/login UI, keyring, and the whole auth subsystem are not extracted. |
| Developer SDK | [SDK](component-sdk/codex_component_sdk/__init__.py), [template/build CLI](component-sdk/codex_component_sdk/__main__.py), [versioned wheel metadata](component-sdk/pyproject.toml), and [examples](component-sdk/examples) produce a separately deployable Python zipapp with notices. | Fourteen SDK process regressions passed. The [0.1.0 wheel distribution gate](verification/2026-09-30/sdk-wheel-distribution.json) built outside the checkout, installed in a fresh venv without PYTHONPATH, then built a custom template/static-asset plugin. SDK/source removal preceded actual manager install/invoke/remove; its frozen executable hash stayed unchanged. Earlier real-engine package evidence remains separate. Python 3.12/Linux is verified; other-language SDKs and broader platform conformance remain. |
| Separate GUI package | [Presentation example](component-sdk/examples/desktop) is independently installed and uses the actual app-server protocol. It is a new client, not replacement of the compiled TUI/app-server internals. | Twelve fixture gateway tests passed. The [storage-v2 manual gate](verification/2026-09-30/gui-manager-native-storage-v2.json) passed seven actual-engine Chromium/Playwright evaluations with no JavaScript errors: streaming, native approval/execution, Stop, reload, manager Ctrl+C, cold recovery and a further completed turn. The independently built storage package and both host hashes stayed unchanged; all tracked shutdown PIDs disappeared. The earlier ten-check contract-1 gate is retained separately. Active maintenance ownership uses separate tests; see VALIDATION for tool availability. |
| Native thread storage | [Typed service](codex-rs/thread-store-component) and [native package](codex-rs/thread-store-local-plugin) expose 47 RPC operations and whole-store shutdown. Contract 2 transports faithful native state; LocalThreadStore retains persistence/read policy and owns writers, fork reservations and scheduled maintenance; session/app-server integration uses explicit persistence services. | All 27 initial v2 fidelity cases passed across two runs. The later maintenance gate passed all 653 distinct rollout/state/store/component/plugin cases across the main run and a corrected fixture rerun, including 28 component/plugin cases; final scoped Clippy was clean. [Results](verification/2026-09-30/storage-maintenance-regressions.json) include a real native migration blocked by SQLite while close waits. Fresh [v2 independent-build/install/CLI](verification/2026-09-30/thread-store-native-v2-independent.json) and GUI checks passed with source removed before installation and unchanged host binaries. Earlier contract-1 and filtered core/app-server gates remain separately dated evidence, not rerun claims. Auxiliary host SQLite and manual migrate-rollouts remain separate boundaries. |
| Shared state serialization | [codex-component-state-codec](codex-rs/component-state-codec) and narrow trusted protocol helpers preserve native in-memory history, metadata, precise numbers and paths before backend persistence policy. | 37 codec and 3 helper tests passed; scoped codec/protocol Clippy was clean, and storage v2 uses the codec in its passing process gate. This is a domain library, not an independently selected subsystem or activated context/replay service. Ordinary provider/disk distrust rules remain native. |
| Native attachment handling | [API](codex-rs/attachment-store-api), [unchanged inline implementation](codex-rs/attachment-store-inline), [adapter/native package](codex-rs/attachment-store-component), and the original facade separate the pre-existing implementation while preserving public imports. | Twelve focused tests, four engine integration tests and the isolated-delegate regression passed; scoped Clippy passed. Native source was exported and built outside the checkout, removed before installation, then invoked with unchanged host binaries; see [report](verification/2026-09-30/attachment-inline-independent.json). Only upload has a production caller at this revision; resolve is adapter-tested. |

### Shared contracts, ownership, and lifecycle

The following describes the implemented contracts and their limits; the
validation record identifies which guarantees have direct runtime evidence.

- **Discovery and activation:** manifests reside under
  immutable directories under `CODEX_HOME/components/objects/`; settings map each
  installed plugin identity to its package object. User settings choose enabled packages,
  per-plugin configuration, and explicit `kind:name` replacement selections.
  Tools, context, lifecycle, and model bindings use an immutable catalog snapshot
  captured for each new engine session. Credential selection is captured when
  `AuthManager` starts; that manager may outlive several sessions. Installation
  alone does not execute the package. Existing snapshots retain their bindings
  and immutable package bytes across removal/reinstallation. Host restart is
  sufficient for activation, but not universally required: new sessions capture
  ordinary adapters, whereas auth selection needs a new manager. Live replacement
  during an active session is not implemented.
- **Dependencies and compatibility:** the host validates manifest API and
  component contract versions, semantic plugin versions, enabled dependency
  requirements, dependency cycles, identity/entrypoint constraints, and duplicate
  component selections. Dependency ordering does not yet provide a plugin-to-
  plugin service-call interface. Persistent component processes have an explicit
  connection lifecycle; the proposed reverse dependency broker is separate work.
- **State ownership:** the engine owns sessions, turns and approvals. A selected
  thread-store service owns its live writers, fork reservations and durable
  history; host auxiliary SQLite services keep their own explicit handle.
  Plugins receive owned serialized values, explicit configuration,
  and a plugin-specific state-directory path. They do not receive implicit Rust
  pointers or ownership of host state. Per-invocation processes must persist
  any state they need across calls themselves. State retention and migration
  during removal/upgrades is tested separately from activation. Old object bytes
  and durable plugin state are retained; automatic object garbage collection and
  state migrations are not implemented.
- **Transport and startup:** the one-invocation protocol uses versioned NDJSON
  initialize/ready, request, incremental event, terminal response and shutdown.
  Stateful storage uses the separately negotiated [persistent protocol](component-sdk/PERSISTENT_PROTOCOL.md),
  with concurrent calls, chunked bodies and private temporary-file reassembly.
  Frame size, channel capacity and shutdown grace are bounded. The physical
  frame cap is 4 MiB. Persistent logical histories span bounded frames; legacy
  one-invocation messages, including model v1, must fit the whole envelope in that
  cap. Neither permits unbounded model-context injection. Attachment blobs use
  private staged files.
- **Cancellation and shutdown:** the one-invocation drop guard terminates its child
  when an invocation or pending stream read is cancelled. Unix process groups
  are included in termination. A successful response is followed by bounded
  shutdown and exit-status checking. Linux startup cancellation and idle-consumer
  backpressure/deadline behavior have focused tests. Linux child setup also requests parent-death termination and checks the
  parent-exit race; tests cover blocked handlers whose stdin is not being read.
  The creating OS thread must outlive the component. Windows descendant cleanup
  and other platform behavior still require conformance tests.
  Persistent accepted operations survive an abandoned waiter and are never
  implicitly replayed; reserved cleanup calls release fork leases, and explicit
  close stops admission before draining. Persistent returned startup, driver and
  shutdown errors follow direct-child reaping; cancelled startup retains a cleanup
  owner while the runtime remains alive. Native store shutdown also fences
  maintenance admission, joins accepted migration/compression work and closes its
  owned pools. Ordinary maintenance operation errors retain native warning policy;
  panic/join failures prevent successful shutdown acknowledgement. A crash or
  cleanup deadline can leave completion unknown.
  Create/resume acquisition uses a stricter rule: a transport timeout or malformed
  reply fences the entire selected store, including retained handles. This
  preserves the configured caller deadline and prevents an unacknowledged writer
  from surviving in a store that accepts retries. Accepted work drains under the
  existing shutdown supervisor; it is never replayed. Confirmed native
  application errors keep the store available. This conservative failure scope
  was chosen over an unbounded acquisition wait or an additional lease protocol.
  Lifecycle observers have a one-second deadline. Observer callbacks and long-lived GUI sessions have different lifetime needs;
  presentation startup remains bounded, but the ready service has no execution-lifetime
  cap. Normal result/exit and first Ctrl+C receive a full bounded cleanup grace
  (210 seconds for the manager). Forced termination reports unknown durability.
- **Execution authority:** engine plugins are explicitly installed executable
  code running with the host account's authority. A process boundary alone is
  not an OS sandbox. Generic plugin calls must not be presented as preserving
  shell approval/sandbox semantics that the adapter does not implement.
- **Failure behavior:** selected replacement failures should surface as such;
  silently falling back can change semantics or duplicate side effects. An
  additive observer may log failure and preserve the host transition. Each
  contract needs its own error, retry, idempotency, and cancellation semantics.

### Current adapters are intentionally narrower than whole subsystems

[Tool adapters](codex-rs/component-adapters/src/tools.rs) use native
`ToolContributor`/`ToolExecutor` contracts. Inputs are bounded, owned call data;
outputs carry text and success status and remain subject to host output budgets.
Explicit replacement selection preserves the native registration policy. Generic
replacement of `exec_command` and `shell_command` is rejected because their
approval, environment, PTY, and sandbox contracts are not captured by a generic
JSON tool call.

[Context adapters](codex-rs/component-adapters/src/context.rs) contribute bounded
typed user-context fragments through `TurnInputContributor`. They do not replace
history ownership, context-window construction, token accounting, compaction,
retained review evidence, or authorization. Contribution bounds include markers
and attribution; the current fragment limit is 1,000 UTF-8 bytes.

[Credential adapters](codex-rs/login/src/component_auth.rs) use `ExternalAuth`:
`auth.resolve` takes `{}`, and `auth.refresh` takes an unauthorized reason and
previous account ID. Results are tagged API-key or external ChatGPT credentials,
or a safe transient/permanent error classification. Native login restrictions
and credential commit remain authoritative; a refresh cannot change the previous
account, and workload identity selection rejects replacement. Transport/parser
errors are replaced by static diagnostics. Authentication subprocess stderr and
generic CLI credential-result display are suppressed. Native auth-value Debug
formatting is redacted. Six component checks and the full 249-test login crate
regression run passed. This does not replace OAuth, keyring storage, or all
authentication behavior.

[Lifecycle adapters](codex-rs/component-adapters/src/lifecycle.rs) deliver bounded
thread/turn events and identities through native lifecycle contributors. They
are observers, not replacements for the scheduler or session state machine.
Extension-owned `ExtensionData` is a scoped in-memory type map, explicitly not a
durable state service; see [its source contract](codex-rs/ext/extension-api/src/state.rs).

## Functional subsystem inventory

Paths below identify the existing implementation or contract. The state and
lifecycle columns describe ownership that must be preserved when extracting each
domain. Related crates are grouped by behavior rather than counted as completed
plugins. Unless explicitly stated, independently installed replacement is planned.

| Functional domain and source | Inputs, outputs, and dependencies | State, lifecycle, and cancellation | Independence and remaining work |
| --- | --- | --- | --- |
| Host bootstrap and distribution: [cli](codex-rs/cli), [arg0](codex-rs/arg0), [install-context](codex-rs/install-context), `build-info`, `codex-cli`, packaging scripts | CLI arguments, installation layout, configuration, and selected component packages produce the chosen runtime. Depends on engine/service composition. | Process-owned startup, signal handling, child ownership, and final exit status. No session plugin should own unrelated process shutdown. | Component-management and native startup wiring have actual CLI evidence. A smaller host composition root and factories for remaining concrete services are still required; native packaged dispatch remains authoritative. |
| Presentation and client SDKs: [tui](codex-rs/tui), [exec](codex-rs/exec), [sdk](sdk), [app-server-client](codex-rs/app-server-client) | User input and protocol notifications produce terminal, JSON, SDK, or GUI interaction. Depends on stable event/request contracts. | Frontend owns drafts, rendering, subscriptions, and transport attachment; engine owns turns. Disconnect, reconnect, interrupt, and shutdown are distinct. | Independent GUI package has actual-engine turn/reload evidence and fixture coverage. A new client demonstrates presentation independence only; TUI/exec internals remain compiled with native dependencies. |
| Application service and protocol: [app-server](codex-rs/app-server), [app-server-protocol](codex-rs/app-server-protocol), [app-server-transport](codex-rs/app-server-transport), [app-server-daemon](codex-rs/app-server-daemon), `protocol` | JSON-RPC methods, approvals, and notifications adapt clients to thread/turn services. Depends on config, identity, engine, storage, transport. | Connection registries, request correlation, subscriptions, daemon state; pending requests must resolve on cancellation/disconnect. | Existing process protocol, but service construction and request processors remain compiled. All 382 app-server library tests passed, including five controlled-store lifecycle probes; separate integration binaries are not included in that count. Preserve v2 wire compatibility and schema fixtures before replacing handlers/transports. |
| Session and turn engine: [session](codex-rs/core/src/session), [tasks](codex-rs/core/src/tasks), [ThreadManager](codex-rs/core/src/thread_manager.rs) | Operations and admitted turn input produce ordered events, model requests, tool calls, and persistence effects. Depends on most service ports. | Session owns queues and services; tasks own cancellation tokens and execution lifetime. Stop hooks precede final events; shutdown must cancel/join work before closing history. | `SessionTask` still accepts concrete session/turn context; `run_turn` remains in core. Extract orchestration only after shared service contracts are agreed. Lifecycle observers are insufficient. |
| Agent execution and coordination: [AgentControl](codex-rs/core/src/agent/api.rs), [agent-graph-store](codex-rs/agent-graph-store), [agent roles](codex-rs/agent-roles), [ext/agent](codex-rs/ext/agent), `agent-identity`, `agent-message-board-client`, `ext/agent-message-board` | Spawn/send/wait/configure requests produce agent identities, delivery receipts, results, and graph/message-board updates. Depends on sessions, budgets, storage, environments. | Parent/child ownership, shared graph, per-agent turns and execution guards; cascading cancellation and join behavior must remain explicit. | `AgentControl` and `ThreadManager.with_agent_control_factory` are actual native injection points. Core configuration/snapshot types still cross them. External runner and graph adapters remain planned. |
| Model transport and stream decoding: [client](codex-rs/core/src/client.rs), [codex-api](codex-rs/codex-api), [codex-client](codex-rs/codex-client), `responses-api-proxy`, `response-debug-context` | Full native model request produces ordered `ResponseEvent` values, usage, tools, errors, and completion. Depends on provider/auth, request interception, routing, metrics. | Session client differs from per-turn transport/routing state; stream drop must stop the active request. Retries must not repeat completed tools or committed output. | Selected external transport has real CLI tool/context/resume/native-restoration evidence and focused engine coverage. Complete event-form, native HTTP/WebSocket retry, and incremental-session parity remain to be tested. |
| Provider selection, model catalog, and local/remote backends: [ModelProvider](codex-rs/model-provider/src/provider.rs), [model-provider-info](codex-rs/model-provider-info), [models-manager](codex-rs/models-manager), [lmstudio](codex-rs/lmstudio), [ollama](codex-rs/ollama) | Provider config/account/auth and catalog queries produce capabilities, routes, model lists, and authentication material for requests. Includes Bedrock implementation. | Shared provider/account state, catalog/cache identity, refresh lifetimes, unauthorized recovery. Cancel pending discovery/refresh independently of unrelated sessions. | Existing provider trait is broader than streaming and is not itself an external loader. Provider/catalog contracts are specified in MODEL_COMPONENT_PLAN.md and the dependency-broker design; new native service/API sources are staged but not activated or tested. Credential acquisition/refresh already has six component tests and 249 login regressions. |
| Authentication, credentials, and attestation: [login](codex-rs/login), [keyring-store](codex-rs/keyring-store), [secrets](codex-rs/secrets), [attestation](codex-rs/core/src/attestation.rs), `aws-auth`, `workload-identity`, `user-verification`, `websocket-auth` | Explicit credential scopes, login/refresh/challenge requests produce headers, identity, attestations, or errors. Depends on configured stores and transport. | Host account state and refresh coordination; callbacks must not leak credentials into generic plugin metadata. Shutdown cancels login listeners and in-flight work. | `ExternalAuth` acquisition/refresh is externally selectable through `auth:default`, with native validation/ownership and passing real-process tests. Workload identity cannot be replaced. Attestation, credential stores, login/OAuth, revocation, and broader capability scoping remain planned. |
| Configuration, feature policy, and instructions: [config](codex-rs/config), [core config](codex-rs/core/src/config), [codex-home](codex-rs/codex-home), [features](codex-rs/features), `config-schema`, `cloud-config`, `prompts`, `collaboration-mode-templates` | Layered config, managed requirements, environment settings, and instruction sources produce validated immutable snapshots and diffs. Depends on filesystem, auth/bootstrap, schema. | User/process/thread/turn scopes and live refresh must be explicit; reload failure must retain a valid snapshot. | Config and instructions contributors exist; source-loading hooks do not replace merge, validation, managed constraints, or refresh application. External source/policy factories are planned. |
| Model-visible context and history: [ContextManager](codex-rs/core/src/context_manager/history.rs), [context updates](codex-rs/core/src/context_manager/updates.rs), [context](codex-rs/core/src/context), [history](codex-rs/history), `context-fragments`, `guardian-context` | Accepted response items, settings/world-state changes, and tool results produce bounded incremental model history and review snapshots. Depends on protocol/domain types, token policy, retained evidence. | Context owns ordered items, metadata, generations, user/review revisions, and reference/world-state baselines. Host facts are distinct from model-visible text. | Concrete core-owned manager has no replacement factory. Additive fragment adapter is exercised in focused tests and actual CLI model input; stateful context engine and snapshot/commit contracts remain planned. |
| Compaction, truncation, and context recovery: [compact](codex-rs/core/src/compact.rs), [context manager](codex-rs/core/src/context_manager), [history checkpoints](codex-rs/history), [model-context loading](codex-rs/thread-store/src/local/model_context.rs) | History, model/capability/token budgets produce compacted checkpoints, retained suffixes, and restored model context. Depends on model calls and durable history. | Revisions, compatibility metadata, fork cutoffs, and compaction checkpoints must commit consistently. Abort must not leave a half-replaced history. | Remote V2 compaction calls `ModelClientSession::stream`, so its inference uses the selected transport; compaction policy, validation, checkpoints, and recovery remain native. Other compaction paths require separate audit/tests. Pure history policy extraction is a planned stage. |
| Tool registry, dispatch, schemas, and output policy: [tools](codex-rs/tools), [core tools](codex-rs/core/src/tools), [extension registry](codex-rs/ext/extension-api/src/registry.rs), `ext/items` | Discovered tool definitions and calls produce exposure decisions, execution results, item events, and model-visible outputs. Depends on turn permissions, history, environment, metadata, truncation. | Registry snapshot, call identities, parallelism policy, executed-call tracking, and scoped handlers. Aborts must stop live calls and preserve terminal ordering. | Native `ToolContributor`/`ToolExecutor` external adapter is exercised through focused tests and the real CLI for supported tools. Registry/router/output policy and all builtin implementations are not yet independently replaceable. |
| Command execution and environment selection: [exec-server](codex-rs/exec-server), [exec-server-protocol](codex-rs/exec-server-protocol), [unified exec](codex-rs/core/src/unified_exec), `shell-command`, `shell-escalation`, `utils/process`, `utils/pty`, `worktree` | Environment IDs, permission profiles, command/PTY input produce process handles, sequenced output, exit results, and worktree state. Depends on approvals, executor filesystem, sandbox/network policy. | `EnvironmentManager`, `ExecBackend`, `ExecProcess`, PTY/process retention, and native execution manager own different resources. Interrupt, stdin-close, detach, kill, and final reaping differ. | Existing native traits and remote exec protocol are reusable. Constructors and manager assumptions remain fixed; process-capability RPC and environment-provider adapters are planned. Generic shell-tool replacement is currently rejected. |
| Filesystem, patching, discovery, and watchers: [file-system](codex-rs/file-system/src/lib.rs), [apply-patch](codex-rs/apply-patch), [file-search](codex-rs/file-search), [file-watcher](codex-rs/file-watcher), `git-utils`, `utils/git-discovery` | Typed path/permission/read/write/walk operations produce files, metadata, patch results, search hits, and change notifications. Depends on execution environment and sandbox context. | `ExecutorFileSystem` owns streams/handles; watchers own subscriptions. Cancellation must close handles and stop scans; partial writes require stated transaction behavior. | Native filesystem trait exists; tool-level swaps do not replace all filesystem access. External filesystem and discovery providers remain planned. |
| Approval, sandbox, and risk policy: [sandboxing](codex-rs/sandboxing), [execpolicy](codex-rs/execpolicy), [guardian review](codex-rs/core/src/guardian_review.rs), [extension review hooks](codex-rs/ext/extension-api/src/contributors/approval_review.rs), `ext/guardian-reviewer`, `ext/guardian-v2`, `linux-sandbox`, `bwrap`, `windows-sandbox-service`, `mxc-sandbox`, `process-hardening` | Requested actions plus explicit authority/context produce permit/deny/escalation and sandbox configuration. Depends on session policy, risk reviewer, environment, client approvals. | Pending approvals, cached grants, reviewer context and policy generations are host-owned. Cancellation must resolve/remove pending decisions without granting access. | Review contributors exist; approval-store/enforcement ownership remains native. Define decision and enforcement ports separately and preserve managed constraints before any external policy replacement. |
| MCP, connectors, skill packages, and existing plugins: [codex-mcp](codex-rs/codex-mcp), [rmcp-client](codex-rs/rmcp-client), [connectors](codex-rs/connectors), [skills](codex-rs/skills), [core-plugins](codex-rs/core-plugins), `core-plugin-common`, `plugin`, `utils/plugins`, `ext/mcp`, `ext/connectors`, `ext/skills` | Configuration, discovery, resources, tool calls, and skill invocation produce capability definitions, call results, prompt input, and elicitation requests. Depends on auth, executors, caches, host policy. | Session owns live `McpRuntime`; connections, caches, server startup/shutdown, elicitation, and selected package snapshots have distinct scopes. `RmcpClient::shutdown` is an existing lifecycle operation. | Existing packaged capabilities and native extension hooks are reusable but are not replacements for the MCP/skills/plugin managers. External manager/backend contracts remain planned. |
| Hooks, queue, goals, and extension composition: [hooks](codex-rs/hooks), [extension-api](codex-rs/ext/extension-api), [ext/queue](codex-rs/ext/queue), [ext/goal](codex-rs/ext/goal), [queue store](codex-rs/thread-store/src/queue_store.rs), `ext/git-attribution`, `ext/history-notes` | Thread/turn/tool/config events and queued work produce ordered contributions, admission decisions, goal updates, and hook results. Depends on session state, tools, instructions, persistence. | Typed data scopes, admission permits, hook processes, pending items, and durable queue/goal records must not be conflated. Stop callbacks must finish/cancel before scope disposal. | Lifecycle observer adapter has focused bounded-event tests; it remains additive. Queue and goal persistence, turn admission, every contributor type, and hook execution are not externally replaceable yet. |
| Memory and retained knowledge: [memories/read](codex-rs/memories/read), [memories/write](codex-rs/memories/write), [ext/memories](codex-rs/ext/memories), [state memory runtime](codex-rs/state/src/runtime/memories.rs) | Thread evidence, extraction requests, versions, and retrieval queries produce bounded memory input and durable records. Depends on models, storage, readiness/lease policy, environment. | Background extraction/consolidation, leases, version/readiness state, and context admission. Shutdown must preserve completed work and cancel remaining jobs explicitly. | Crates exist; backend/work-queue/model-access coupling remains. Memory service/provider and storage ports are planned. |
| Thread persistence, session recovery, and projections: [ThreadStore](codex-rs/thread-store/src/store.rs), [LiveThread](codex-rs/thread-store/src/live_thread.rs), [rollout](codex-rs/rollout), [state](codex-rs/state) | Create/resume/append/flush/load/fork/revert/list/archive operations produce durable history, model context, metadata, projects, sections, attachment references, and query pages. Depends on history/protocol types, SQLite/rollout implementation, migrations. | Store owns writers and durable records; host owns acquisition, commit, and discard. Flush/shutdown fence queued writes. Recovery, compaction checkpoints, fork lineage, and projections require coherent ordering. | The selected process adapter implements native operations and contract-2 faithful state transport; contract 1 is rejected before invocation. All 653 distinct storage/state/rollout cases passed across the main gate and corrected fixture rerun, including owned maintenance and installed-process fidelity/lifecycle cases; final scoped Clippy was clean. Codec tests cover domain serialization, not replay-engine activation. Fresh v2 independent package/CLI/GUI acceptance passed with unchanged host binaries. Earlier contract-1 and filtered engine gates remain separately recorded evidence. Auxiliary SQLite queue/goals/memory and migrate-rollouts remain separate boundaries. |
| Attachment/blob storage: [AttachmentStore](codex-rs/attachment-store/src/lib.rs), [thread attachment records](codex-rs/thread-store/src/thread_attachments.rs), `utils/image`, `utils/audio` | Upload/resolve and typed metadata produce inline/stored references or bytes. Depends on environment/file access, codecs, and possibly authenticated upload transport. | Blob ownership/retention and thread references have different lifetimes. Cancellation and partial upload cleanup require a stated policy. | Native inline implementation is extracted into a separately built/installable package, with real-process and four engine tests. Upload keeps original inline bytes or provider-usable file IDs; large blobs use bounded private staging. Durable remote-blob retention, deletion and migration remain provider-specific work. |
| Realtime, voice, images, and web tools: [realtime-webrtc](codex-rs/realtime-webrtc), [voice-host](codex-rs/voice-host), [ext/image-generation](codex-rs/ext/image-generation), [ext/web-search](codex-rs/ext/web-search), `mermaid` | Media streams, search/image requests, and structured outputs produce audio/video/text/image events and artifacts. Depends on model/provider capabilities, transport, tools, storage. | Realtime channels, media buffers, host processes, and tool jobs have separate startup/stop/drain rules. | Existing domain crates/extensions remain native. Text model transport does not establish realtime or multimodal replacement; dedicated contracts and fixtures are planned. |
| Code mode and embedded execution: [code-mode](codex-rs/code-mode), [code-mode-host](codex-rs/code-mode-host), [code-mode-runtime](codex-rs/code-mode-runtime), [code-mode-protocol](codex-rs/code-mode-protocol), [v8-poc](codex-rs/v8-poc) | Program/tool definitions and execution requests produce tool dispatches, runtime events, and results. Depends on tool capabilities, runtime/process isolation, environment. | Session provider, interpreter/isolate/process and pending calls; timeout/abort must terminate computation and settle calls. | Separate host/protocol already exists; provider/session construction and tool integration still require external selection/adaptation. Experimental code is inventoried, not promoted to a stable plugin contract. |
| Networking, routing, and remote access: [http-client](codex-rs/http-client), [network-proxy](codex-rs/network-proxy), [websocket-client](codex-rs/websocket-client), `tcp-tunnel`, `stdio-to-uds`, `uds` | Routes, connection requests, domain policy, and bytes produce approved connections and ordered streams. Depends on credentials, TLS, environment/network approvals. | Connection pools, listeners, tunnels, and pending policy requests must drain/close independently of turn history. | Crates/protocols exist; reusable transport/policy factories and independently selected adapters are planned. Preserve configured proxy/trust and routing identity. |
| Cloud tasks and service integration: [cloud-tasks](codex-rs/cloud-tasks), [cloud-tasks-client](codex-rs/cloud-tasks-client), [backend-client](codex-rs/backend-client), `codex-backend-openapi-models`, `external-agent-migration` | Task/service/migration requests produce remote statuses, artifacts, or imported local configuration/history. Depends on auth, transport, protocol, storage. | Remote task identity, retries, polling/subscriptions, and migration checkpoints. Local cancellation does not automatically mean remote task cancellation. | Client crates exist; service-provider and migration adapters remain planned. Mock clients alone do not prove live-service compatibility. |
| Observability, diagnostics, and host utilities: [otel](codex-rs/otel), [analytics](codex-rs/analytics), [diagnostics](codex-rs/diagnostics), [feedback](codex-rs/feedback), `rollout-trace`, `otel-trace-websocket`, `async-utils`, `terminal-detection`, `ansi-escape`, remaining `utils/*` | Structured events, spans, metrics, diagnostics, and pure utility inputs produce bounded exports or transformed values. Depends on explicit sinks, privacy/redaction, clocks, routing. | Export queues, telemetry subscribers, timers, tracing context, caches, and sleep-inhibitor guards need owned lifetime and shutdown flushing. | Event/metrics/time traits offer starting points. External sinks/clock/cache policies remain planned. Pure utility crates should stay narrow libraries unless a real runtime replacement need exists. |
| Developer tooling and compatibility tests: [justfile](justfile), [test support](codex-rs/test-binary-support), [core test support](codex-rs/core/tests), [app-server tests](codex-rs/app-server/tests), `app-server-test-client`, `thread-manager-sample`, `cloud-tasks-mock-client`, platform test-support crates | Build inputs, schemas, fixtures, and test commands produce reproducible binaries and behavioral evidence. Depends on both Cargo and Bazel graphs and packaged resource lookup. | Test isolation and cleanup, fixture-server/process lifetime, bounded retries, snapshot review. | SDK/templates and focused component tests have recorded passes. A complete plugin architecture needs contract tests and examples for every supported replacement, plus distribution/upgrade checks. |

## Dependency and state rules for further extraction

1. Keep protocol/domain DTO crates independent of concrete engine implementations.
   Extract an interface before assigning independent implementation work; a new
   crate that imports private session internals is not a completed boundary.
2. Make service construction explicit at the host composition root. Keep native
   adapters as default implementations, and select a replacement before creating
   threads. Missing or incompatible selected providers must fail predictably.
3. Publish, for each contract, its inputs/outputs, errors, state owner, state
   lifetime, dependencies/capabilities, startup readiness, shutdown ordering,
   cancellation, concurrency, retry rules, and version compatibility. Never
   infer these from the common NDJSON envelope alone.
4. Keep operation/session/thread/host state scopes separate. Any stateful external
   service needs durable identity and either resumable operation IDs or an
   explicit long-lived service lifecycle. Per-call process memory cannot own an
   ongoing PTY, live thread writer, model session, or agent runtime.
5. Preserve model-context ordering, bounded additions, checkpoint metadata,
   approval provenance, and existing recovery formats. Generic serialized
   snapshots must not lose host-only metadata or turn untrusted text into policy.
6. Retain Apache-2.0 LICENSE/NOTICE and package notices. The current component work
   does not incorporate DeepSeek/Cordis. Any future selective reuse requires its
   actual licenses and provenance to be recorded at the imported files.

## Delivery stages and acceptance gates

Stages are integration checkpoints, not permission to reduce the final scope.
Parallel work begins after affected contracts, ownership, and dependencies agree.
Each cycle reruns the earlier behavior it can affect.

1. **Pinned baseline and executable build.** Preserve target repository identity,
   record upstream provenance, build the actual native CLI/app-server, and record
   focused baseline behavior. Keep schema and Cargo/Bazel dependencies coherent.
2. **Installable foundation and early GUI.** Finish manifests/catalog/lifecycle,
   SDK/template/package management, native startup registration, initial tool and
   context contributions, lifecycle observers, and selected model transport.
   Build the GUI separately as a presentation package speaking app-server JSON-
   RPC. Retain CLI/TUI/headless execution as supported first-class entrypoints.
3. **First end-to-end independent-package gate.** Build and hash the host. Create
   a plugin project outside the harness checkout with only the SDK/contracts,
   build its package separately, install/enable it, create the required new session or auth manager in the unchanged host,
   and exercise its contribution inside a real engine turn. Repeat with an
   explicitly selected replacement, then disable/remove and verify native
   behavior. Hash the host again and require equality. Record paths, commands,
   events, outputs, and errors. Merely invoking a plugin-process echo method does
   not satisfy this gate. Apply the same independent build/install criterion to
   the GUI package and confirm it connects to that already-built app-server.
4. **Durable service boundaries.** Extract persistence/attachment/agent-graph
   adapters and remove inappropriate local-store assumptions. Agree transaction,
   snapshot, idempotency, fork-reservation, and cancellation semantics before
   externalizing mutation. Exercise restart and interrupted-write recovery.
5. **History and model domain.** Extract context/history policy and its native
   adapter, compaction/replay behavior, provider/catalog/auth factories, and
   realtime/media contracts. Keep domain types, engine orchestration, and
   transport independently testable. Confirm native and replacement parity.
6. **Execution and policy services.** Adapt environment providers, process/PTY,
   filesystem, discovery, approval/review/enforcement, network routing, and
   credential capabilities. Preserve request authority and OS-specific behavior;
   do not replace a privileged builtin through an under-specified generic tool.
7. **Remaining extensions and orchestration.** Finish agent runtime, MCP/skills/
   plugin managers, hooks/queue/goals/memory, code-mode, cloud integrations,
   configuration sources, observability, and then the engine scheduler/composition
   boundary. Update every inventory row with implemented native and external
   adapters, independent packaging, and concrete behavioral evidence.
8. **Whole-harness completion gate.** Every functional domain has an explicit
   contract and documented state/lifecycle ownership. Every replaceable service
   has a real selected implementation path and an independently built test
   package; additive extension-only domains are identified as such. Remaining
   libraries are justified as pure/shared infrastructure. Run platform, upgrade,
   packaging, native-default, and selected-provider conformance checks. A small
   successful plugin demonstration must not be reported as this completion gate.

## Required runtime and regression evidence

| Behavior | Evidence required before marking its affected boundary verified |
| --- | --- |
| Independent installation | Outside-checkout source/build, package contents and notices, installed manifest/settings, unchanged host hash, real native-engine invocation, session/manager activation, disable/remove, missing dependency/version/conflict failure. |
| Streaming and model access | Ordered incremental deltas, final output, usage and errors, tool-call events, request interception, native HTTP/WebSocket behavior, interruption before first token and midstream, stream consumer drop, no duplicate terminal event. A deterministic model fixture proves protocol integration only; live-provider access is separate evidence. |
| Approval and execution | Allowed/denied/pending decisions through real client APIs; cancellation while approval is pending; unchanged native shell/process behavior and sandbox policy; no generic adapter bypass of tool authority. |
| Cancellation and lifecycle | Interrupt during startup, model streaming, tools, child agents, and shutdown; observe terminal ordering and process/task cleanup. Observer failure must not strand host transitions. Include backpressure and malformed/oversized/timeout responses. |
| Context and compaction | Incremental context preservation and caps; attribution and host-only metadata survive; local/remote compaction, fork, rollback, resume, and interrupted compaction agree with native behavior. |
| Persistence and recovery | Flush makes records readable/durable; shutdown fences writes; failed acquisition discards only uncommitted preparation; reopen/resume/fork/revert/archive/delete and paginated/legacy rollouts preserve history and projections; interrupted writes and migrations do not fabricate committed data. |
| Agents and execution environments | Real spawn/send/wait/interrupt plus parent/child cleanup; remote/local executor selection, PTY output/exit ordering, file operations, and OS-specific permission behavior. |
| GUI and existing clients | Actual app-server initialize/thread/start/turn/start stream, approval response, turn interruption, history/reconnect, component selection, and error rendering; manually inspect web interactions throughout development and recheck older CLI/TUI flows each cycle. |

Existing suites worth preserving include `model_request`, `client_websockets`,
`abort_lifecycle`, `startup_cancellation`, `compact_resume_fork`, and
`agent_execution` under the native engine tests; storage suites live next to
`ThreadStore`/state implementations, and app-server tests exercise public JSON-
RPC. These names identify targets, not results. Follow [AGENTS.md](AGENTS.md):
use `just test` for Rust tests, scoped checks while iterating, required formatting
and linting, and update schemas/lockfiles when the corresponding contracts change.

The selected cloud environment currently has no callable Context7 or in-app
Browser tool in its available tool catalog. Current contracts are therefore
grounded in the pinned source. For library/framework questions, the requested
Context7 resolve/query workflow remains unavailable and must not be claimed as
used. Actual GUI checks used Chromium driven by Playwright, including the real
engine turn/reload scenario. This is an alternative to the requested in-app
Browser, not a claim that the requested tool was available. CLI/runtime-only
checks do not need a browser. Results and remaining gaps are recorded in
[VALIDATION.md](VALIDATION.md); no live-model inference is claimed.

## Current extraction checkpoint and next dependency

The initial persistence recommendation is now implemented as a typed process
adapter and a separately packaged native LocalThreadStore implementation. The
native trait preserves history, metadata, capability checks and fork semantics;
a persistent process owns its writers and reservations. Accepted operations keep
their completion ownership after a waiter disappears and are never automatically
replayed. A disconnected call can have an unknown outcome: the current contract
does not claim durable idempotency keys or transparent retry.

The historical contract-1 persistence acceptance gate passed with a separately
built installed native store, source removed before installation, real engine
turns and GUI/manager shutdown/cold recovery. Subsequent audit exposed state
fidelity and maintenance-ownership gaps those cases did not cover. Contract 2 and
owned maintenance now pass their focused native/process gates; run splits are
recorded above. Fresh storage-v2 outside-source native build and actual CLI/GUI
acceptance also passed: 81 exported local crates exclude the engine/CLI/TUI/app-server,
and the same separately built package ran after source removal without rebuilding
either host binary. Auxiliary queue,
goal, memory and agent records still need their own boundaries.

The explicit local [`codex migrate-rollouts` command](codex-rs/cli/src/migrate_rollouts.rs)
still constructs `LocalThreadStore` directly, outside selected engine storage.
This is a remaining native maintenance boundary, not a fallback during engine
turns. A coherent next storage extraction is its typed dry-run/apply operation,
thread filters, throughput options, progress, per-thread report and cancellation
contract, followed by CLI routing through the selected implementation. The
existing background maintenance scheduling RPC is not equivalent to that
interactive/reporting operation. Native session construction, resume metadata,
rollout-path queries and background maintenance already use the explicit
persistence bundle or native trait ports.

The [second complete Linux workspace attempt](verification/2026-09-30/full-workspace-regressions-attempt2.json)
stopped during test linking because the filesystem ran out of space. It targeted
164 current workspace members after the corrected storage checkpoint; no tests
ran, and no complete-suite runtime result is claimed. Its V8 release dependency
has been recovered and verified against upstream's pinned checksums; this is
environment preparation, not test evidence. See the [setup record](verification/2026-09-30/full-workspace-v8-setup.json)
and [validation scope](VALIDATION.md#full-workspace-preparation-not-a-test-result).

The next model-domain design is in [MODEL_COMPONENT_PLAN.md](MODEL_COMPONENT_PLAN.md)
and the [dependency-broker contract](component-sdk/DEPENDENCY_BROKER_PROPOSAL.md).
It keeps live credential/network authority in an explicit host dependency while
extracting native catalog policy/cache behavior. Catalog publication and cached
reads must remain scoped to the current account/provider generation. No model
catalog extraction is counted as complete until it is installed separately and
exercised against an unchanged host.

Context/history extraction follows with a native implementation behind an
explicit snapshot/operation/commit contract. Its current manager owns ordered
`ResponseItemEnvelope` values, retained context, review history, multiple
revision counters, token information, settings reference context and world-state
baselines. Moving that struct to another crate without separating those inputs,
policies and commit semantics would not establish replacement.
