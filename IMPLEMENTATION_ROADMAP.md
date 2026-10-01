# Codex Harness Compartmentalized — canonical implementation roadmap

Status: **incomplete platform; verified partial baseline**. Updated 2026-09-30.
This is the execution plan across runs. Read [EXECUTION_STATE.md](EXECUTION_STATE.md)
first on resume, then the [source inventory](COMPONENT_INVENTORY.md). Historical
[COMPONENTS.md](COMPONENTS.md), [VALIDATION.md](VALIDATION.md), and the domain plans
remain supporting evidence/designs; they do not supersede this ordered plan.

## 1. Endpoint and scope

V1 is our own platform derived from actual OpenAI Codex, with a small documented
kernel and separately built, installed, discovered, versioned component packages.
The native implementations preserve Codex behavior; compatible custom packages
can add capabilities or replace a selected implementation **after the host build,
without recompiling it**. Activation at restart is sufficient. Hot replacement
during a session and modification of the installed official Codex desktop app are
outside v1. Upstream revision integration is required through the maintenance
track below; periodic polling and unattended live deployment are not enabled by
this roadmap. Cordis and a second harness are not
required. An unchanged Codex executable wrapped in a plugin is not this endpoint.

The desktop-style GUI is an independently packaged presentation component. A
headless client remains usable with it absent. The final default distribution
contains native component packages and a composition manifest; bundling them in
one download must not turn them into inseparable compiled host implementations.
The current built-in implementations are migration fallbacks, not permanent
exceptions to extraction. Each fallback must be labeled and removed from the
minimal-host composition when its external equivalent reaches acceptance.

V1 requires all functional rows in the inventory to have a verified replacement
boundary or an explicit, reviewed kernel/support classification. A crate, trait,
file move, additive contributor, staged worker, or successful build alone does
not satisfy extraction. Every grouped family below must retain separate contracts
where implementations have independent ownership; a phase is not one giant plugin.

### Permitted kernel responsibilities

| Responsibility | Why it remains in the kernel | What must move out |
| --- | --- | --- |
| Bootstrap and administrative installation/selection commands | Something must locate and activate components before domain code runs. | Interactive CLI/TUI, application configuration resolution, marketplace workflows. |
| Manifest/contract compatibility and immutable composition | Prevent incompatible or missing required implementations from becoming an active session. | Provider/model selection and domain defaults. |
| Process supervision, bounded transport, cancellation ownership | A failed component cannot be its own only cleanup owner. | Domain session state machines, maintenance policy, agent scheduling. |
| Capability issuance/revocation and owner/incarnation identity | Cross-component calls need an authority independent of their requester. | OAuth implementation, credential storage backend, policy scoring and approval UI. |
| Permission-ceiling validation and OS enforcement boundary | A reviewer or executor cannot authorize its own privilege expansion. | Rule interpretation/scoring, sandbox backend mechanisms, approval presentation. |
| Minimal request correlation and ordered delivery checks | Detect stale replies, duplicate terminal events and invalid ownership. | Domain events, history algorithms, event projections, telemetry and presentation. |

These are explicit exceptions, not permission to leave `Session`, `run_turn`,
`ContextManager`, `ModelsManager`, or `UnifiedExecProcessManager` in the kernel.
Final dependency checks must show that the minimal host does not link `codex-core`
or native model/tool/history implementations. Pure shared wire types, codecs and
small utilities may be linked by multiple packages without being runtime plugins.
Document such libraries in the inventory and keep their transitive dependencies
free of hidden domain services.

### Trust model

Installed native/custom components are currently **trusted executable code**.
Process separation and capability DTOs are not OS isolation against a malicious
same-user executable. Preserve native tool sandboxing, managed requirements,
credential redaction and permission ceilings. Do not claim untrusted-plugin
isolation unless a separate enforced isolation profile has been implemented and
tested. Never weaken native restrictions to make a component acceptance test pass.

## 2. Starting point: evidence, not assumed completion

Upstream: `openai/codex` at `d42056091aded7feb1d88ac7e83972108b2aa478`.
Verified published baseline: `4e38e02993df698cefa99d7cc325890d837262de` on `main`.
Preserved unfinished sibling: `0b38d5974159ab2a8776200025dbb602e89b8f8f` on
`wip/recovered-next-components-20260930`. Preserve Apache LICENSE/NOTICE and
[provenance](UPSTREAM_PROVENANCE.md). No DeepSeek/Cordis code is currently used.

| Classification | Actual starting point |
| --- | --- |
| Verified native extraction | Local thread store API/adapter/native package, storage contract 2; inline attachment API/native implementation/package. Storage is not all auxiliary persistence. Production attachment consumption is upload; resolve has adapter tests. |
| Verified replacement adapters | Model inference stream v1 and auth resolve/refresh. Native HTTP/WebSocket transport, OAuth/keyring and provider routing have not been extracted. |
| Verified new infrastructure/additions | Component API/host, immutable installation/catalog, SDK wheel/templates, tool/context/lifecycle adapters, independently packaged GUI. Additive context is not the native context manager. |
| Staged source, not accepted extraction | Dependency broker, native model catalog, context engine/replay worker and adapters. Main does not link/activate them. |
| Isolated unverified WIP | Chunked model v2, typed event/error bridge, session startup/cleanup owner, manual migration/run ownership. Some referenced test files and workspace/module links are missing. Port coherent pieces; do not merge the whole WIP tree as a verified upgrade. |
| Still coupled | Session/turn/agent loops; live history and compaction; native execution/security; auxiliary state; MCP/skills/marketplace; background services; native CLI/TUI and application protocol services. See inventory for every family. |

Existing proof: corrected complete core **library** run 2,694/2,694 passed;
native storage and attachment installation, CLI streaming/tools/resume, GUI
streaming/approval/Stop/cold recovery, and actual manager Ctrl+C cleanup passed
with recorded unchanged host hashes. These used deterministic inference fixtures.
The original failing run is retained separately. Full workspace testing was
disk-blocked; cross-platform and live-provider coverage are not established.
Exact run IDs, sources, commands and limits are in VALIDATION, not implied by this
roadmap. New work must produce new evidence against its actual source.

## 3. Contracts agreed before parallel implementation

For every boundary, write a contract card before implementing adapters:

1. Name, semantic purpose, contract/schema version, named operations and event types.
2. Owned inputs/outputs, units, ordering, precision, null/absence semantics and limits.
3. Sole authoritative state owner, immutable projections, durable source and recovery format.
4. Dependency/capability grants, provider selection and explicit/default provenance.
5. Startup readiness, admission, cancellation before/after acceptance, drain, shutdown and removal.
6. Typed domain errors versus transport/protocol/timeout/owner-stale/outcome-unknown errors.
7. Retry/idempotency/reconciliation rules; never infer exactly-once from request IDs.
8. Trust/permission/credential boundary, logging policy and failure behavior.
9. Upgrade compatibility, migration/rollback limits and old-session pinning.
10. Native differential corpus and separately installed real-host acceptance scenario.

Prefer owned domain DTOs and coarse operations. No serialized `Session` pointers,
`TurnContext`, mutexes, closures, arbitrary host callbacks, or raw SQL escape hatch.
Do not await plugin RPC while holding session/configuration locks. Break cycles
with narrowly scoped capabilities and frozen snapshots, not a globally accessible
service locator. Accepted mutations retain a supervisor after caller cancellation;
cancel-before-admission performs no operation. A lost reply may mean outcome
unknown and must not trigger a blind replay or native fallback.

The v1 registry must support namespaced contracts and declared provides/requires
capabilities, version ranges, optional dependencies and deterministic resolution.
The baseline hardcoded eight kinds and dependency presence checks are insufficient.
New capability providers and clients can be installed through generic registration;
adding a tool/client/provider implementation must not require rebuilding the host.
Consumers still validate the typed contracts they understand. Unknown contracts,
dependency cycles, incompatible schemas and denied grants fail before activation.

Use bounded queues and physical frames, explicit aggregate/logical-body limits,
streaming or pagination for large state, backpressure and reserved cleanup
capacity. Spooling a value while later materializing an unbounded JSON object is
not memory-bounded streaming. Model v1's reproduced 4 MiB limit is a known defect
to close, not a new accepted application limit.

## 4. Ordered phases and dependency gates

Execute in order unless the execution-state file records an independently ready
substep. Parallel work may implement separate leaf components only after shared
contracts are agreed. One integration owner serializes changes to workspace
metadata, host protocols and `core`; one Rust build runs at a time on this VM.
Update SDK authoring and GUI/headless checks with every applicable phase, not only
at release. The deliverables and checks below are **future work** unless explicitly
listed as existing proof above.

### P00 — Baseline, inventory and durable execution ledger

**Prerequisites:** original checkout/remote identity and full source backup.
**Deliver:** this roadmap, complete inventory, execution state, baseline/WIP
distinction, source/command/result recording, resource budget and reviewable commits.
Revalidate remote refs before publishing; preserve both saved worktrees and WIP.
**Checks:** every workspace package and staged functional family mapped; referenced
paths exist; evidence citations resolve; no claimed new test results; no secrets,
binaries or caches staged. **Exit:** documentation checkpoint pushed and verified.

### P01 — Finish native storage maintenance/migration selection

**Checkpoint complete:** source `5d2f2a026ae6ce0c42cf56bb2f3b01971e4bf0bb`.
[Exact validation and retained failures](verification/2026-10-01/P01_TEST_EVIDENCE.md)
cover native/selected parity, independently built package 0.2.0, unchanged host,
old storage2 compatibility, real migration/cold recovery, GUI approval/Stop and
actual manager Launch SIGINT. Focused native state/rollout integration passed;
this does not claim complete standalone state/rollout or workspace suites.

**Prerequisites:** accepted storage2 and persistent transport.
**Deliver:** route the pre-existing manual rollout-migration CLI through selected
storage capabilities instead of constructing `LocalThreadStore` directly. Reuse
the saved migration-run owner after review; complete worker dispatch, capability
negotiation, CLI selection/progress/cancel/close. State the migration operation's
own version/support contract; an older selected store must fail explicitly, never
silently migrate a different native database. Keep storage ownership and maintenance
join semantics; this extends native storage extraction, not a new whole subsystem.
**Acceptance:** independent native package install, unchanged host, actual Legacy/
Paginated data migration and resume, custom implementation dispatch, cancellation
under SQLite contention, restart after partial migration and absence of surviving
workers. **Regression:** storage/state/rollout, migration CLI, recovery and GUI
session history. **Exit:** selected/default parity and truthful forced-shutdown
durability result, package compatibility tests, SDK/native package docs and evidence.

### P02 — Native search and workspace services

Detailed P02a audit, staged contract proposal and pending consumers:
[file-search component plan](component-sdk/FILE_SEARCH_COMPONENT_PLAN.md).

**Verified prerequisite (2026-10-01):** StageA joins native search workers and
client publication across cancellation/reconnect/shutdown; the separately packaged
GUI adds native file references. Source `25b2c3150879761026086a62e480045fc38d32ff`
and [evidence](verification/2026-10-01/P02_SEARCH_PREREQUISITE_EVIDENCE.md)
include new-host storage, migration and GUI regression. Search is still **COUPLED**:
no installed search backend is active. StageB must register the neutral contracts,
add bounded native/process implementations and replace real composition paths.
[Lineage](upstream/p02-search-lineage.json) records this distinction.

**Prerequisites:** P02a search uses contract cards, the accepted process/session
protocol and existing native authorized root snapshots. P02b filesystem/watch/Git
and worktree mutation services additionally require P03/P04/P10 authority/config
contracts; leave those substeps queued until their prerequisites pass.
**Deliver, separately:** file-search query service; watcher subscription service;
Git/worktree operations and repository-discovery services. Start with native
`FileSearchSession`, replacing actual app-server/TUI call paths. Give its current
detached matcher/walker threads explicit joined close before process extraction.
Use query/subscription epochs, native path DTOs, root/ignore rules and bounded
result streams. Git mutations retain repository locks and scoped authority.
**Acceptance:** independently built search plugin changes real GUI/CLI search;
rapid query replacement, ignored/symlink/Unicode paths, watcher overflow/rescan,
repository disappearance, worktree recovery and cancellation. **Regression:**
native search/watch/git/worktree tests plus UI task/workspace selection. **Exit:**
each service passes its own native/custom/default gate; no hidden native worker
left behind when selected service closes.

### P03 — Shared lifecycle, transport and dependency broker

**Prerequisites:** baseline preserved; no domain activation mixed into this change.
**Deliver in reviewable stages:** integrate saved session constructor cleanup owner;
complete chunked/typed model-v2 transport and missing test/linkage files; activate
the staged bidirectional broker with explicit grants and owner revocation. Keep
compatible v1 adapters during migration and reject unsupported versions explicitly.
**Acceptance:** startup failure after resource acquisition, abandoned startup/close
waiters, full request queue, reserved cleanup, parent/child death, stalled writes,
oversized/fragmented bodies, terminal ordering, reentrancy/cycles and revoked calls.
Accepted work is supervised, not replayed. **Regression:** all host transport and
storage lifecycle gates, unchanged strict PID assertions, core startup/cancellation,
actual manager Ctrl+C and GUI Stop/recovery. **Exit:** broker truly linked and used
by an installed dependency consumer; logical limits and error/deadline fidelity
documented; session startup failure cannot leak an acquired native service.

### P04 — Configuration, credentials and native authentication

**Prerequisites:** P03 capability/owner semantics.
**Deliver, separately:** configuration source/load/refresh component; native OAuth,
device-code and API-key provider; credential persistence/keyring backend; workload/
AWS identity adapters. Resolved config snapshots include provenance, revision and
managed constraints. Kernel bootstrap only selects config roots; managed permission
ceilings and credential/account validation remain authoritative enforcement.
Replace actual native implementations, not merely `auth.resolve/refresh` examples.
Advance policy/owner epochs inside native setters, including A→B→A workspace changes.
**Acceptance:** offline/interactive auth, refresh concurrency, refresh cancellation,
account switch, stale-owner rejection, keyring unavailability and private error logs;
config precedence/refresh/denied widening and existing credential/config migration.
**Regression:** complete login/config suites and real host default/custom auth paths.
**Exit:** independently built native/custom auth and config packages work without
host rebuild; no credentials in transport logs or unauthorized consumers.

### P05 — Model catalog, providers and native inference

**Prerequisites:** P03 broker/model v2 and P04 authority fences.
**Deliver in order:** (a) activate native catalog/cache worker and validated mirror;
(b) package configured-provider and Bedrock selection/capability behavior;
(c) extract real HTTP/WebSocket inference session implementation, warmup/reuse,
retry/fallback/auth recovery; (d) unary summary and duplex realtime model contracts.
Reuse staged catalog code only after workspace/SDK/kind/core composition links and
tests exist. Grant `host.model_endpoint` per owner/request; do not serialize raw
credentials or HTTP factories. Static user catalogs remain pinned configuration.
**Acceptance:** independently install native and custom catalog/provider/transport;
prove actual model/instructions/capabilities and outbound requests change. Exercise
offline zero-fetch, ETag/TTL, owner races, stream fragments/tool events/usage, >4 MiB
history/media, HTTP↔WebSocket fallback, aging retry deadlines, interruption, terminal
failure and next turn. Realtime has ordered duplex/cancel/reconnect/close tests.
**Regression:** native model/provider/login, actual CLI tools/resume/compaction and
GUI streaming/Stop. **Exit:** the native inference implementation runs outside the
minimal host; it is not a worker that starts an unchanged Codex engine.

### P06 — Remaining durable metadata and attachment implementations

**Prerequisites:** P03 ownership and P04 configuration/auth.
**Deliver:** domain contracts for goals/queued items, projects/sections, attachment
metadata, memory jobs/readiness/versions, logs, remote-control enrollment and import
history currently owned by concrete `StateRuntime`; package native SQLite domains.
Separate remote-upload/resolve/cache attachment implementations from inline storage.
Define cross-domain transaction boundaries explicitly; keep one authoritative writer
per durable domain and fenced leases where operations span services.
**Acceptance:** independent install/custom backend, cross-process concurrency,
schema upgrade, interrupted migration, cold recovery, signed-URL expiry and original
byte/path/provenance fidelity. **Regression:** state/storage/memory/rollout corpus,
large attachments and session resume. **Exit:** no product persistence silently
bypasses selected providers; domain migrations and backward compatibility recorded.

### P07 — Native replay/reconstruction

**Prerequisites:** P03 constructor cleanup; storage2 and state codec.
**Deliver:** activate staged context-engine/replay crates and process adapter; wire
ordinary resume, in-memory fork and constructor commit/revoke paths. Worker owns
temporary reconstruction only; host uses an owner/epoch ticket to accept results.
Preserve complete annotated originals, retained review evidence and world-state
baseline distinctions. This is not yet live context or compaction extraction.
**Acceptance:** exported native worker built separately, source removed, unchanged
host; native/external differential Legacy/Paginated replay, malformed checkpoints,
legacy compaction, media, forks, cancellation, stale reply, worker crash and owner
revocation. **Regression:** reconstruction/Guardian checkpoint suites, real resume/
fork/compaction and GUI cold recovery. **Exit:** all real replay entrypoints select
the contract; no missing constructor cleanup or fallback hidden by copied tests.

### P08 — Live context/history and prompt construction

**Prerequisites:** P07 plus bounded snapshot transport.
**Deliver:** sole mutable context owner with atomic typed transactions, frozen
snapshots, independent working branches and explicit persistence effects. Extract
token estimation, normalization, world-state/fragment assembly and projection into
their appropriate native component/services. Host coordination owns admission and
commit ordering, not a duplicate mutable `ContextManager` implementation.
**Acceptance:** differential native/custom history projections; append/usage/
reference/reset/rollback/fork/retained-context operations; abandoned accepted write,
lost reply, persistence error, stale revision and incarnation reuse; old snapshots
stay frozen. **Regression:** history/normalization/token accounting, pending input,
review provenance, replay, large media and actual next-model request bodies.
**Exit:** live context implementation is independently replaceable; full originals
remain durable even when model-visible history is truncated.

### P09 — Compaction and summarization policy

**Prerequisites:** P05 inference, P08 transactions and immutable evidence snapshots.
**Deliver:** native local/remote compaction strategies, budget/trigger policy and
summary request service. Proposals carry captured revisions and preserved metadata;
the commit coordinator rejects stale results. Separate model work from scheduling.
**Acceptance:** install a custom policy that changes actual compaction decisions;
manual/pre-turn/inline compaction, local/remote errors, model switch, cancellation,
queued input, fork/resume and retained authorization evidence. **Regression:**
compact/compact_resume_fork/token_usage_rollout, Guardian and GUI ongoing turns.
**Exit:** native algorithms and scheduling policy are packages; no semantic rewrite
or history loss disguised as extraction.

### P10 — Permission, approval and review-policy components

**Prerequisites:** P03 scoped capabilities, P04 constraints and P08 evidence.
**Deliver:** native exec-policy decision service, approval correlation/persistence,
network approval policy, and review interfaces. Separate review decisions from
kernel enforcement of the configured ceiling. Use immutable action identity,
environment snapshot and policy epoch; a reviewer cannot grant itself authority.
**Acceptance:** allow/deny/late/stale decisions, policy narrowing during a request,
cached review identity, denied escalation and network attribution; cancellation
unblocks all waiters without inventing approval. **Regression:** exec_policy,
request_permissions, network_approval and Guardian authorization tests, plus actual
GUI approval and denial. **Exit:** custom decision services alter allowed policy
behavior within the ceiling; no path bypasses native safety boundaries.

### P11 — Native process execution and sandbox mechanisms

**Prerequisites:** P10 action/authority protocol and P03 supervision.
**Deliver:** independently packaged native executor with process/PTY/stdin/output/
yield/background-terminal ownership; separate OS sandbox backend adapters and
remote exec-server transport. File operations, shell escalation and cancellation
must use scoped environment grants. Kernel validates grants; platform mechanisms
implement them. Preserve connected app-server/exec-server cross-OS contracts.
**Acceptance:** real shell/PTY interaction, stdin after yield, output truncation,
background processes, cancellation/deadlines, child reaping, denied sandbox/network
access and approved retry; remote executor disconnect and cleanup uncertainty.
**Regression:** native unified_exec/exec-server/sandbox suites and real GUI tool
progress/Stop. **Exit:** selected native/custom executor handles actual built-in
tools without a concrete host process manager or permission widening.

### P12 — Native tools, routing and code-mode runtime

**Prerequisites:** P08 context effects, P10 approvals and P11 executor.
**Deliver:** replace `ToolInvocation`'s Session/TurnContext pointers with owned
invocations and scoped capabilities; package routing/dispatch policy and native
shell/apply-patch/file/image/search tools. Separate code-mode host/protocol/V8
execution component and tool discovery/cache. Additive tools remain supported.
**Acceptance:** replace a pre-existing native tool and dispatcher independently;
parallel tools, streaming output, hooks, cancellation after acceptance, patch/diff
fidelity, malformed args and constrained filesystem access. **Regression:** actual
agent tool loop, tool_parallelism, apply-patch, code-mode and approvals/UI cases.
**Exit:** native tools and execution are distinct replaceable packages; a custom
tool demo is not counted as extracting the whole tool subsystem.

### P13 — MCP, connectors, skills and ordinary plugin services

**Prerequisites:** P04 credentials/config, P10 authority and P12 tool ports.
**Deliver, separately:** native MCP connection/catalog/runtime; connector service;
skills discovery/load/cache/rendering; existing Codex marketplace/plugin manager.
The ordinary skills/MCP/plugin ecosystem remains compatible and distinct from
engine component packages. Preserve prepared MCP bindings, resource origin,
elicitation review, auth/config epochs and extension data ownership.
**Acceptance:** independent native/custom service packages, MCP startup cancellation
and reconnect, tool/resource changes, skill root precedence/cache invalidation,
marketplace install/remove/disabled-state recovery and no credential disclosure.
**Regression:** existing MCP/skills/core-plugin/connectors tests and real GUI tools.
**Exit:** these native managers are selected services; installing an MCP server or
skill alone is not reported as extracting them.

### P14 — Session, turn and multi-agent orchestration

**Prerequisites:** stable ports from P04–P13 and explicit event envelopes.
**Deliver:** session factory/lifecycle service, native turn runner/input queue,
agent hierarchy/scheduling/budget/messaging service and recovery ownership.
Break Session→tool→Session and AgentControl→ThreadManager→AgentControl cycles
with scoped factories and capabilities. Package actual native orchestration;
do not rename the existing monolithic loop to “kernel.”
**Acceptance:** independently replace scheduling/turn policy; concurrent input,
steering, suspend/resume, parent interruption, child fork/reload/eviction, queue
drain, budgets and shutdown during construction/active turn. **Regression:**
agent_execution, agent_control, pending_input_persistence, startup/abort lifecycle,
real CLI/GUI multi-turn recovery and actual manager Ctrl+C.
**Exit:** native orchestration runs as selected services against explicit dependency
ports; state ownership and accepted-operation cleanup are explicit across every
exit path. Transitional packages may still link remaining native extensions or
client services until P15–P17. The complete default composition's minimal-host
no-`codex-core` dependency audit is a P19 gate, not a premature P14 claim.

### P15 — Background memory, goals, queues, hooks and remaining extensions

**Prerequisites:** P06 state, P09 summaries, P12/P14 service ports.
**Deliver:** native memory read/search/note component and distinct memory writer/
consolidation/job scheduler; goals, queue, history notes, agent message board,
Git attribution, image generation, web search, hooks and Guardian reviewer packages.
Inventory every compiled contributor kind; borrowed callbacks do not automatically
become remote-safe. Guardian gets bounded evidence and a restricted reviewer-session
capability; host authorization remains independent of the reviewer.
**Acceptance:** actual native feature replacement, persistence/restart, job leases,
retry/cancel/disable, reviewer recursion and stale evidence, hook failure semantics,
timely background cleanup and ordering after visible final output.
**Regression:** memory/extension/Guardian/hook suites and composed agent/UI sessions.
**Exit:** each discovered functional contributor has an installed native package
or a documented non-runtime support classification, never an unexplained omission.

### P16 — Domain events, streaming projections and observability

**Prerequisites:** P03 bounded transport and stable domain producers.
**Deliver:** versioned domain event bus/projections, stream parsing where not part
of provider transport, telemetry/OTel/analytics/feedback/diagnostics/trace consumers.
Kernel retains only correlation/order enforcement. Consumers cannot block accepted
domain cleanup indefinitely; loss/backpressure policy is explicit by event class.
**Acceptance:** slow/disconnected consumers, ordered partial/final/error events,
reconnect cursors, privacy filtering, disabled telemetry, exporter failure and
shutdown flushing without leaking secrets or corrupting sessions.
**Regression:** native stream/error/accounting and CLI/GUI event rendering.
**Exit:** replaceable event/presentation/telemetry behavior with preserved native
protocol semantics; dropping diagnostics is not mistaken for dropping durable work.

### P17 — Presentation, clients, cloud and remote features

**Prerequisites:** P14 session service, P16 events and P04/P06 auth/state.
**Deliver:** maintain the early separately installed GUI throughout; package actual
app-server protocol/transport service, interactive CLI/TUI, cloud-task client,
remote-control enrollment/reconnect/revocation, realtime voice/media frontends and
diagnostic clients. Minimal administrative/headless dispatch stays in bootstrap;
the headless application client is independently replaceable. Preserve TypeScript
SDK and non-Rust launcher compatibility explicitly.
**Acceptance:** GUI absent still supports headless use; independently built GUI
installed into an unchanged host; streamed replies, tools, approval/denial, Stop,
disconnect, persisted history, fork/resume, multiple sessions and manager Ctrl+C.
Remote control tests pairing/revocation/auth epochs/reconnect/disable; cloud/media
tests bounded cancellation and permission-specific failures.
**Regression:** app-server public JSON-RPC, protocol schemas, TUI snapshots, native
clients/SDKs and connected-client/executor OS combinations. **Exit:** clients are
packages with real behavior, not static imitations or mandatory host dependencies.
Our optional cloud noVNC relay is a development aid, not this native remote-control
component and not a prerequisite for core extraction.

### P18 — SDK, installation/upgrade and release hardening

**Prerequisites:** every prior contract has authoring support as it is introduced.
**Deliver:** consolidated Rust/Python developer SDKs, templates for every lifecycle
class, schema/conformance tools, dependency/capability examples, build/export/package
commands and documented language-neutral wire protocol. Complete transactional
install/enable/disable/configure/select/upgrade/rollback/remove/GC operations,
integrity/path validation, version solver, lock/composition file and migrations.
Baseline remove+install and Python contract1 templates are not the final SDK.
**Acceptance:** build outside source tree with only published SDK; install after
host hash recorded; remove build sources; run, upgrade and roll back compatible
packages. Test incompatible major/schema, missing/cyclic dependencies, crash during
install/upgrade, live-session pinning, retained state, removed dependencies, corrupt
packages and explicit irreversible-migration refusal. Avoid deleting user state on
uninstall; GC only unreferenced objects with defined retention/recovery.
**Regression:** all contracts/native defaults, full Rust/Python/TypeScript suites,
Cargo/Bazel metadata, Linux/macOS/Windows packaging and termination behavior.
**Exit:** reproducible release artifacts, licenses/notices/SBOM/provenance, developer
guide, migration/compatibility matrix, supported-platform results and honest limits.

### P18U — Upstream maintenance and update component

**Prerequisites:** begin lineage/mapping in P00; implement candidate integration
after P03 and representative native packages stabilize; activation/rollback uses
P18 installation transactions. See [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md)
and [upstream/lineage.json](upstream/lineage.json) for the concrete workflow/ledger.

**Deliver in sequence:**

1. Preserve upstream commit/tree/license provenance and map original paths/symbols
   to destination components, contracts, copied algorithms and intentional custom
   changes. Record still-coupled and unmapped semantics honestly. Update this ledger
   in every extraction commit; a path rename is not sufficient semantic mapping.
2. Build a separately installable maintenance component accepting an explicit
   upstream repository/revision and current composition. Resolve the exact commit,
   fetch into a separate source mirror, and inventory changes against the recorded
   upstream base. Pin all inputs; never edit a live installation or its state.
3. Produce a dependency/contract/security/state impact report and isolated candidate
   worktree. Reapply/adapt each customization with source mapping. Flag upstream
   deletion, changed assumptions, unrecognized call paths and incomplete mappings.
   A conflict-free merge is not proof of behavioral compatibility.
4. Build a coordinated candidate host/native-package/SDK set with explicit versions
   and migration requirements. Run native differential, compatibility, permission/
   credential boundary, full affected regression, independent-install real-host and
   GUI/headless gates. Retain original failures, commands, source/binary hashes and
   the exact upstream commit range. Classify semantic ambiguity as review-required.
5. Emit reviewable changes and a signed-off compatibility result; routine compatible
   updates may automate candidate preparation/promotion only under an explicit
   policy whose required gates all passed. Contract-major, irreversible migration,
   security-boundary, unclear semantic and unsupported-platform changes must stop
   for review rather than being silently copied or declared safe.
6. Prepare immutable rollback artifacts: prior host/plugins/composition, schemas,
   state backup/restore requirements and transaction journal. A minimal external
   bootstrap/recovery command must inspect/restore a known-good deployment even if
   the updated host or maintenance component cannot start. Never claim binary
   rollback reverses an incompatible data migration.

**Acceptance:** integrate at least one real, later pinned upstream Codex revision
into an isolated candidate. Prove compatible, already-installed custom replacement
and additive plugins still work with their package digests unchanged, without
rebuilding those plugins; preserve both UI and headless recovery. Exercise both
rejection of a breaking/incompatible candidate and rollback after a controlled
failed activation. Recover the prior state/composition through the external
bootstrap with the candidate host intentionally unavailable.
**Regression:** upstream/fork contract corpus, independent builds, installation/
upgrade lifecycle, old-data migration, security boundaries and actual GUI/manager
shutdown. **Exit:** exact provenance, dependency impact and rollback are reproducible;
the updater is installable separately; ambiguous changes are flagged; neither a
polling schedule nor live application is implied by candidate creation.

### P19 — Final v1 acceptance and kernel audit

**Prerequisites:** every functional inventory row accepted; no unexplained coupled
row or hidden built-in fallback. All advertised platform gates and P18U's real
upstream integration plus failed-update recovery have evidence.
**Deliver:** frozen host/kernel ABI, native package set/composition, release guide,
clean-room acceptance record and final dependency/ownership/security audit.
**Acceptance scenario:** build the host once; record its hash; on a fresh home build
native packages and custom model/catalog, context, tool/executor and scheduling
replacements outside the checkout using released SDKs; install them plus GUI;
remove all package source access; run a real streamed tool/approval conversation;
interrupt, compact, fork, shut down via manager Ctrl+C, restart and recover; upgrade
one compatible plugin, reject an incompatible one, remove GUI and continue headless;
remove a selected required dependency and receive an explicit activation error;
restore a valid composition and continue from preserved data. Host hashes never
change. Repeat the custom-behavior and UI/headless checks against the accepted
isolated upstream-update candidate, then exercise its breaking-update rollback.
Verify at least one configured real provider or local model in addition to
deterministic protocol fixtures; report unavailable credentials as a pending gate.
**Regression:** complete upstream/fork suites and differential contract corpus;
user-facing GUI manual Browser verification when available, cross-platform and
failure/recovery matrix, no surviving tracked processes or false durability claim.
**Exit:** no missing gate labeled passed; kernel exception review and all inventory
rows signed off with source/contract/package/test references. Otherwise ship only
an explicitly partial development checkpoint/release candidate.

## 5. Mandatory checkpoint procedure for every phase

1. Record the exact baseline, selected contract card, affected callers and source
   diff/manifest. Agree shared interfaces before workers edit consumers.
2. Extract the native implementation and move its semantic tests/docs with it.
   Keep existing behavior and compare native/external outputs over the same corpus.
3. Implement real host selection and native/custom independently built packages.
   Confirm the actual path is exercised (component-side trace and a meaningful
   custom behavior change); reject failure without accidental native fallback.
4. Run focused meaningful tests through `just test`, then affected regression gates.
   For core/common/protocol, the complete suite is required before full acceptance;
   existing user authorization covers necessary regression work. A resource-blocked
   run is a recorded limitation, not a pass. Never weaken lifecycle/security tests.
5. Exercise the existing GUI at each real-host integration cycle: streaming, tool
   progress, approvals, Stop, persistence/recovery and manager Launch Ctrl+C. The
   requested in-app Browser is currently unavailable in cloud; record that and use
   actual Chromium/Playwright runtime checks. Keep the requested Browser/manual
   gate pending where unavailable; do not hold core development for the optional
   relay or falsely label fallback tests as in-app Browser tests.
6. Run scoped `just fix`, `just fmt`; update generated config/protocol schemas and
   `just bazel-lock-update` when applicable. Do not repeat runtime tests solely for
   formatting; substantive fixes require their own appropriate checks.
7. Record commands, statuses, actual tested tree/diff, binary/package hashes, OS,
   fixtures and failures. Preserve original failed runs and corrected-run provenance.
8. Update inventory, execution queue and validation. Commit only the coherent
   milestone; retain unfinished work separately. Recheck remote refs, push without
   force, and verify remote commit and critical files. Stop a conflicting push and
   reconcile concurrent publication; never overwrite it.

## 6. Repeatable startup, recovery and release operation

Current native bootstrap is documented in [README.md](README.md): build from
`codex-rs` with its pinned toolchain/config, then install packages into an explicit
isolated home using `codex-component`. Record selected package IDs/config and start
the real CLI or `launch desktop --codex-bin ...`. Never silently use the official
desktop executable. Keep accepted immutable binaries separate from ongoing builds.

By P18 provide one documented clean-machine command sequence: verify release
integrity and prerequisites; install kernel; create explicit home/workspace;
install native composition; select model/auth using secure setup; start headless;
optionally install/launch GUI; shut down gracefully; resume; upgrade/rollback;
remove GUI/components while retaining data. Test the exact documented sequence on
each supported OS. Signing/release hosting/account purchases require the actual
available authorization; do not invent an externally deployed service.

Durable state changes require explicit schema versions, legacy readers/migrators,
backup/restore tests, interrupted-migration reconciliation and rollback limits.
Preserve upstream config/rollout semantics, native paths, original tool/reasoning
metadata, permission provenance and account isolation. Do not expose credentials
in releases, logs, screenshots or evidence. Do not call process disappearance
proof of durable flush; report forced/unknown shutdown honestly.

## 7. How this plan continues across runs

`EXECUTION_STATE.md` is the compact resumable ledger: exact last accepted commit,
phase/substep, ordered next actions, dirty/unverified paths, active commands/PIDs,
validation and blockers. Update it at start, before a checkpoint and on a material
blocker. Keep detailed evidence under `verification/<date>/`; runtime credentials,
caches and private backups stay outside published source. A cloud-local archive
is not an externally durable backup; GitHub establishes durability only for its
included source. Maintain upstream path/symbol/contract/customization mapping with
each extraction and record accepted upstream revision changes separately from our
own commits. Preserve the WIP branch until each piece is accepted or explicitly
retired with an explanation.

Continue successive feasible components after each checkpoint. Stop only for
actual completion, an unavoidable resource/runtime limit or material missing
input/authorization; record the next exact action before ending. Cloud execution
does not depend on the user's computer staying on, but no uninterrupted/perpetual
agent lifetime or always-on service is promised. The optional viewer never blocks
the extraction queue. Upstream-update capability is required v1 work; candidate
automation is sequenced by P18U and periodic/live deployment remains inactive.
