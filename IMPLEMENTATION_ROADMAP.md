# Codex Harness Compartmentalized — canonical implementation roadmap

**Current order, 2026-10-05: infrastructure acceptance, then automatic continuation
through all remaining product phases.** OpenAI cloud environments remain primary;
Proxmox provides explicitly offloaded builds and durable recovery. The consolidated
[completion handoff](COMPLETION_HANDOFF.md) maps the infrastructure gate, every phase,
all inventory families and final acceptance. It does not narrow this roadmap.
Feature work remains paused while that gate is open; after it passes, continue
successive dependency-ready milestones without repeatedly asking to continue.

The user permits a separately provisioned expandable native worker, while the
original environment and all work remain intact. [Source/control handoff](NATIVE_BUILD_WORKER_HANDOFF.md)
and its [exact manifest](verification/2026-10-05/native-worker-handoff/MANIFEST.json)
replace the same-VM-only resumption order. No worker, source transfer, dependency
bootstrap or native execution has occurred in this source-readiness continuation.
Required next inputs are authenticated
worker access, admitted clean-build resources, pinned dependency access, source/control
custody and reproducible bootstrap. Do not copy the damaged warm target or use the
old conditional warm-cache budget for the clean worker. Do not resume product
extraction/updater work before infrastructure acceptance or repeat completed
reviews/blocked original-VM commands.

The prior [handoff/capacity design](NATIVE_OUTPUT_HANDOFF.md) stays preserved as
original-VM recovery guidance, not permission for another attempt. The source audit
identified eight adopted auth paths, nine separate unpublished document/helper
variants, fourteen private runtime controls and standalone package/proof portability
gaps. Only nonsecret coordination evidence is published by this checkpoint.

The coherent eight-path external-token-install change is adopted and formatted,
but remains **untested, unaccepted and unpublished**. Exact 8,980-path source map:
`b75efc9751977009a9b28f2893c120db45a8f6e79f4993125e3685ae90d6cf5b`.
The first focused attempt exited 102 on missing offline metadata inputs. An
additional 227 genuine current-lock archives were then restored and verified while
preserving the earlier 902; no source extraction or unrelated 166-file restoration.
Actual locked/offline metadata passed on unchanged source/index (1,315 packages,
7,058,322 output bytes). Focused retry `03` subsequently exited 101 at the first
dependency compile: restored proc-macro2 metadata was read-only. **Zero native tests
ran in either focused attempt.** All 1,047 preserved rlib backing payloads remain;
one canonical proc-macro2 rlib alias is now absent and 1,046 remain intact.

[Current checkpoint](verification/2026-10-05/p03-token-install-build-inputs/README.md)
keeps these separate results and their exact identities. Original-VM native admission is closed
until a recoverable writable-output handoff and expanded dirty-closure budget cover
the full required sequence. Do not chmod recovery backing, reset fingerprints or
blindly retry. A clean offloaded worker follows I1–I5 without repairing these
protected aliases. The eight-path store/caller operation must remain coherent.
Focused/full login/provider tests, scoped lint, production CLI/manager, all 12 prior
runtime recipes and the new auth consumer remain pending. Collector and broader
updater/planner/scope-validator work stay paused.

[Installed lifecycle evidence](verification/2026-10-05/p18u-installed-overlay-lifecycle/README.md):
five source/strict stages passed, with eight normal commands (two setup and six
recovery) and two direct installed-SDK primaries. Cooperative shutdown and abrupt
plugin-only death both left incomplete jobs, all six tracked identities disappeared
without rescue, and each subsequent normal-manager invocation prepared a verified
13-path overlay. Installed package0.4/legacy overlay0.3 reused pyz913e0842 from prior
8,975-path d84cf069 source; then-current8,977-path f27ab9e source stayed unchanged. The
held Git was observed at exec, not during computation beyond exec. Manager graceful
forwarding, complete updated-host integration, rollback, UI/native validation and
all seven updater release gates remain unproved. Do not replay the completed gate.

[Completed capacity evidence](verification/2026-10-05/p03-capacity-restored/README.md):
895 metadata and902 genuine registry archives restored;55 duplicate-only paths
reclaimed1,786,339,328 bytes;1,047 selected unchanged rlibs preserved in a565,190,455-
byte verified archive and fully restored before journaled alias substitution.
All original source/index/accepted binary identities remain preserved. Postcheck
observed4,126,289,920 persistent free bytes and8,842,485,760 hard-unused memory,
above the conditional3.85GB/8GiB guards. This is repaired headroom, **not a native
acceptance pass**. The historical auth-specific3,797,387,752-byte phase budget counted missing
core/TUI pairs once. The newly observed dirty-output/ownership issue requires a
renewed whole-sequence allowance; larger P03 gates also need separate admission.
See [capacity plan](CAPACITY_PLAN.md).

Latest published checkpoint before this documentation update is
`95359037b45621f54c4284c049873f0834f3f123` (tree
`b30e7e35b17411f1fc9710105cf6c78154052f9d`) on the existing WIP branch. Preserve completed repair receipts; do not replay relocation.
The original conservative8GiB-free recommendation is not mandatory. Recovery
`rehydrate`/`recover` paths also passed three disposable synthetic cases/18 phase
calls for normal, lost-backing and interrupted-exchange recovery; this is not
power-loss or production-disaster proof. The actual full restoration did execute. Historical instructions below to advance unrelated small
maintenance work are superseded by this exact resumption order.

The accepted CLI/manager now also have a140,182,904-byte archive, fully restored
and hash-verified before only the duplicate restore probe was released. Scoped
clean-cache advice preserved exact contents/metadata and yielded9,139,257,344
hard-unused bytes. The earlier8,486,326,272-byte admission refusal remains preserved;
no mutation/build ran in that refused attempt. Full details and archive identity
are in the native checkpoint above. Neither restored backing nor formatting proves
that the complete native validation sequence fits or passes.

Status: **incomplete platform; verified partial baseline**. Current checkpoint2026-10-05.

### Preserved October2 source42 build/runtime baseline
The current source42 production/runtime checkpoint now passes the12 planned
rebuilt-host gates. Source code is preserved by WIP047e91e; subsequent15bd9d1 records
409 package tests and scoped lint. Current build/runtime evidence is published at `ad2fc04768ff7abc8bbd623de462dee6d9b69c07`
on the same `wip/p03-process-final-and-mcp-preservation-20261002` branch. Main remains
`781080f7e3c8bfe1953378001d777dff33d74bc3` until the remaining promotion gates pass.

[New production build](verification/2026-10-02/p03-auth-refresh-production-build/README.md)
passed locked/offline on source map42e3899a (8,949 files), new CLI8e8a5dac and reused
managerf054d84a. All12 current runtime recipes passed on their first attempts:
installed storage, four isolated manual migrations, ordinary/selected-search GUI,
attachments, slow/forced Launch shutdown, eight held Git/HTTP cases and owner postcheck.
Normal slow shutdown waited46.452s and recovered the canonical held message. Forced
second-interrupt shutdown exited1 after2.041s with explicit durability uncertainty;
the held message was absent on recovery. Four real Chromium cycles exercised approval,
native commands, Stop, streaming, file search, cold history and actual manager Ctrl+C.
These used deterministic inference; in-app Browser and live-provider validation are
unavailable. Earlier source7a GUI01 timeout remains preserved with unresolved cause.

Preserved October2 source42 passes301 login +108 provider tests, focused4 overlapping301,
zero skips/retries, serialized full-package execution and scoped login lint02. The
preceding reload slice passed297+108 and scoped lint03. Native HTTP Stage A139 plus
four CA-classification checks and Stage B553 plus real HTTP composition retain their
original source identities, as do historical CLI3/Exec9. The TUI OOM ran zero tests;
TUI6+2 snapshots, App Server lifecycle/search/storage16 and same-process replacement
parents remain open. No full workspace result is claimed. Source42 success does not
retroactively rebind old package results or prove universally graceful cleanup.

[Preservation evidence](verification/2026-10-02/p03-auth-refresh-production-resources/README.md)
records fully restored proof archives before exact test-artifact/cache retirement.
At that historical checkpoint, roughly477MB overlay blocked another substantial
Rust build. The completed October5 repairs and conditional plan above supersede
that capacity observation and the earlier instruction to pursue small-feature detours. P03 credential writes, MCP/lower transport custody,
actual broker/catalog activation and the remaining subsystem extractions stay open.

Coverage is still three bounded native replacement families: thread storage/manual
migration, inline attachments and native file search. The installed GUI is additive.
HTTP/auth custody is lifecycle support in coupled native services, not another
extracted family. Most engine services remain coupled; broker/model-catalog
activation, further extraction and updater acceptance remain unfinished. Follow
[execution state](EXECUTION_STATE.md) for current evidence, limitations and order.

This is the execution plan across runs. Read [EXECUTION_STATE.md](EXECUTION_STATE.md)
first on resume, then the [source inventory](COMPONENT_INVENTORY.md). Historical
[COMPONENTS.md](COMPONENTS.md), [VALIDATION.md](VALIDATION.md), and the domain plans
remain supporting evidence/designs; they do not supersede this ordered plan.

P18U now has an independently installed [read-only review component](component-sdk/examples/upstream-maintenance/README.md).
Its 17 planner, 22 lineage and 11 boundary checks passed; 23 acceptance commands (19 through the real manager)
proved external build/install/invoke/version rejection/replacement/removal and a
host-independent packaged inspector without a host rebuild. The actual local-object
report retained no-upstream-advance, unavailable composition and historical ownership
blockers. This is additive review support, not a native extraction, update application,
later-revision integration or rollback result. The complete P18U release gates below
remain required; see [evidence](verification/2026-10-02/p18u-installed-impact/README.md).
Published/read-back checkpoint: `914cc59374c1149463e78bc33851d83e3f14d0a4` on the
existing WIP branch. Fresh migration and two Chromium GUI cycles also passed; the
next native auth-install slice remains source-only. Its
[historical capacity gate](verification/2026-10-02/p18u-installed-impact/NEXT_NATIVE_CAPACITY.md)
is superseded for resource planning by the completed repair and fresh phased guards;
current-source native acceptance still has to run.

The October4 maintenance0.1.1 checkpoint closes its real direct-Git lifecycle gap:
58 focused cases and fresh external install/replacement/removal, abrupt manager
SIGINT, cooperative protocol shutdown and subsequent invocation pass on unchanged
hostf054. [Evidence](verification/2026-10-04/p18u-owned-git/README.md) preserves the
original failed cancellation and the lost temporary-report limitation. This adds
no native extracted family. The subsequent0.2.0 independently installed source-input
capsule now passes49 real-package commands against actual later upstream2e5fea64
and custom914cc593:13paths/25blobs/1,228,513B with distinct custom content retained.
Eleven capsule fixtures,13 reviewer regressions, admitted native-Git cancellation/
SIGKILL and subsequent successful invocations pass. Fresh storage/manual migration
and two real Chromium GUI cycles also pass on unchanged production binaries.
[Evidence](verification/2026-10-04/p18u-source-capsule/README.md) separates real runtime,
fixtures and artifact inspection from unproved crash durability and live inference.
Next is sparse content reconciliation with exact custom provenance and conflict
rejection, followed by coordinated candidate build/version/migration and recovery
gates. Source preparation is not candidate integration or rollback.
The0.3.0 successor now passes47 focused cases and61 real installed commands:
it upgrades an isolated0.2 installation using remove/install, preserves restored
capsule state and both old contracts, and produces13 exact source outputs. The
one genuine three-way merge retains our custom change and the later upstream
changes. This is a sparse patch set over pinned custom914cc593, not a complete
candidate or updated-host acceptance. The later installed exec-boundary lifecycle
evidence above closes the bounded paused gate; cancellation after Git computation
has progressed beyond exec remains unproved.
Reattachment had cleared volatile backing and services. The October5 repairs now
restore selected backing and pass the conditional resource guards; that does not
rebind old test results or prove the new native cohort. Follow EXECUTION_STATE.md
and the phased plan before any further Rust compilation.

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

Required upstream updates additionally use immutable A/B generations: A remains
available during B preparation, healthy B receives new sessions and existing A
sessions remain pinned. Ordinary plugin activation may still use restart. See
[LIVE_UPDATE_GENERATIONS.md](LIVE_UPDATE_GENERATIONS.md); this is not hot replacement
inside running sessions and is not implemented by current package remove/install.

### Permitted kernel responsibilities

| Responsibility | Why it remains in the kernel | What must move out |
| --- | --- | --- |
| Bootstrap and administrative installation/selection commands | Something must locate and activate components before domain code runs. | Interactive CLI/TUI, application configuration resolution, marketplace workflows. |
| Manifest/contract compatibility and immutable composition | Prevent incompatible or missing required implementations from becoming an active session. | Provider/model selection and domain defaults. |
| Stable deployment routing and independent recovery | A failing release cannot be its own only route/recovery owner; retain journaled generation identities and session pins. | Update planning, porting, domain migration policy, building/testing and normal UI remain components. |
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

### P00M — Current provenance closure and ongoing maintenance intake

**Required now and at every extraction checkpoint; updater execution remains P18U.**
The historical maps and pinned index retain exact source identities and selected
semantic anchors; current ownership/impact semantics remain incomplete. Close that
provenance gap alongside implementation, using the
[current mapping requirements](UPSTREAM_MAINTENANCE.md#current-provenance-closure).
For each accepted boundary, link the original repository/revision/path/symbol to
its current implementation, contract, customization, owner and evidence; classify
new project code and foreign dependencies separately. Preserve historical records.
**Exit:** a pinned checkpoint index and read-only validation report account for
all changed paths and accepted boundaries, with unmapped or ambiguous entries
explicitly unresolved. Do not report complete current provenance while affected
entries remain unresolved; prior runtime results retain their original scope.
This gate does not claim a complete symbol graph, updater, later-upstream candidate
or rollback result.

**Implemented metadata checkpoint:** the [pinned index and checker](upstream/CHECKPOINT_LINEAGE_README.md)
cover all 1,092 changed paths, 28 maps and 27 explicit relationships at `d04d5a7`.
The initial thirteen-fixture result is preserved. The explicit provider-schema
adapter now passes 22 fixtures; the object check recognizes all 28 maps with zero
invalid metadata and 1,167 unresolved semantic, historical-evidence/anchor and release
findings. [Supplement](verification/2026-10-01/P00M_PROVIDER_SCHEMA_EVIDENCE.md).
P00M semantic closure
and P18U execution remain incomplete; this index does not silently advance with main.

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

**StageB support slice:** `98eeb3e1c93b3a856834c57e6c4d57bfa9f8c782` registers
neutral search/cleanup contracts and the shared lossless path codec.
[56 focused cases](verification/2026-10-01/P02B_CONTRACT_EVIDENCE.md) passed at
that support checkpoint; subsequent implementation and installation proof follows.

**Later StageB working checkpoint:** source WIP `651b0b87a281934e5cd7c639021049fad8d393ac`
implements the actual bounded native worker and standalone CLI selection. Its
[evidence](verification/2026-10-01/P02B_SELECTED_SEARCH_EVIDENCE.md) records 222
focused cases and an external source/target build followed by 24 real CLI/manager
commands, including parity, Ctrl+C, failure without fallback and removal, with
host hashes unchanged. This establishes native search replacement for that caller.
The subsequent App Server checkpoint `30c2674600cc84da161562c407515b61511b7c3a`
routes actual App Server search through that selected provider. Its
[evidence](verification/2026-10-01/P02B_APP_SERVER_CONSUMER_EVIDENCE.md) records
457 library/client, 351 protocol/runtime and 13 public-RPC passes in separate
scopes, followed by real installed-worker parity, streaming, failure, removal
and shutdown with unchanged binaries. [Lineage](upstream/p02b-app-server-lineage.json)
maps all 62 changed files and key original/current ownership symbols.

**Verified multi-client search checkpoint:** main
`a469cf4be85fda40c64e563ece3e1639e4abdbac` promotes that native component and
adds actual TUI selection. The [new evidence](verification/2026-10-01/P02B_TUI_CONSUMER_EVIDENCE.md)
records 5,620 passing TUI tests (four ignored), the independently installed worker
through a real terminal, and both desktop GUI cycles with selected search/storage,
streaming, approvals, Stop, reload, cold recovery and manager-only Ctrl+C shutdown.
The host and separately built package hashes stayed unchanged during installation
and runtime. Existing storage and manual migration gates passed against this full
CLI. Chromium/Playwright is fallback evidence; in-app Browser and live inference
remain unverified. App Server and TUI are compiled consumers, not extracted
presentation services. Private storage lookup remains a fourth consumer awaiting
the P03 authority broker; this milestone does not complete P02 or the platform.

The subsequent bounded lifecycle slice was [per-start Preparing cancellation](component-sdk/FILE_SEARCH_PREPARING_CANCELLATION_PLAN.md):
an immediately available single-start control, retained cleanup receipt, native
constructor propagation and unchanged sibling authority. The historical gates below
culminate in promoted main `c28a1c33a856a987316f4b97b4c488d68055fffc`; remaining
limits are retained explicitly. The additive SDK primitives were initially
implemented at `d22cea88aa23e35a619d996fc731122450e51313`; their
[19 passing API tests](verification/2026-10-01/P02B_STARTUP_SDK_EVIDENCE.md) cover
carrier ownership and handoff, not active native/process Preparing cancellation.
The [process control gate](verification/2026-10-01/P02B_PREPARING_PROCESS_EVIDENCE.md) passed42,
the [native owner gate](verification/2026-10-01/P02B_PREPARING_NATIVE_OWNER_EVIDENCE.md) passed88,
and [native backend gate](verification/2026-10-01/P02B_PREPARING_NATIVE_BACKEND_EVIDENCE.md) passed96
(overlapping scopes, not a sum). These historical sources were preserved at cancellation
WIP `9bd3bc30` (required startup base `272993ff`); final WIP `3b8a889` is a parent
of the later accepted main `c28a1c33`. Required SDK revision2 and worker
service now pass [169 focused tests](verification/2026-10-01/P02B_REQUIRED_START_SERVICE_EVIDENCE.md);
runtime pending control passes [55](verification/2026-10-01/P02B_RUNTIME_PREPARING_EVIDENCE.md),
including six real worker-process cases. New0.2 worker built separately against unchanged frozen
host fingerprints; metadata alone is not proof. AS consumer controls pass460 strict-run tests.
TUI production is adopted: first full regression failed at linking with zero tests; second executed
5650 with5633pass/17fail/8skip. Failures include full /tmp tmpfs, inherited NO_COLOR, missing full CLI
and unresolved I/O/PTY exits; preserve each diagnostic. No original snapshots were accepted/changed.
Worker04 passes normal installation against unchanged old fullCLI. Reviewed public pending Stop
reader-intent/retained-receipt source now passes the [combined library gate](verification/2026-10-01/P02B_PUBLIC_STOP_EVIDENCE.md):
6095passed/4skipped, unchanged source and strict0/null. Initial E0624 compilation failed before
tests; a parent-only visibility correction precedes the pass. All12 formerly failed library cases
now pass unchanged; all five prior integration failures now pass in a separate23-pass/4-skip
gate using frozen CLI1b72. Lint/format and build bindings are recorded separately. Unchanged GUI uses concurrent legacy token calls.
New fullCLI build and installed legacy/public Stop Preparing gates now pass with exact artifact
binding; old0.1 and new0.2 search packages both pass unchanged. Normal storage/migration/GUI
passed separately. TUI integration passed23/4skipped; instrumented GUI Preparing clear-query cancellation passed
both cold-launch cycles, followed by real manager-only SIGINT shutdown and recovery.
See [new-host evidence](verification/2026-10-01/P02B_PREPARING_NEWHOST_EVIDENCE.md).
See [GUI Preparing evidence](verification/2026-10-01/P02B_PREPARING_GUI_HELD_EVIDENCE.md).
Pending-Open shutdown and other Preparing GUI retirement triggers remain separate future gates.
These strengthen search, not an additional
extracted subsystem. The source audits and exact evidence are linked from EXECUTION_STATE.md.
The native/process audit annexes refine guard, receipt and handoff requirements. Preserve
old package compatibility and separately build the updated worker before claiming
its stronger cancellation behavior.

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

The reviewed [next-slice source audit](P03_SOURCE_AUDIT.md) grounds implementation in the active
transport and identifies stale unlinked broker/catalog drafts. Follow its ordered private codec,
negotiated leaf broker and native catalog prerequisites without silently removing the Session,
model-v2, authority/configuration or acyclic-dependency obligations below.


**Verified support so far:** private wire factoring passed 87 focused tests and 24 actual
installed-search commands on a freshly linked standalone CLI. Checked service declarations
passed 177 tests and eight real manager installation/catalog commands. The declaration API
fails closed while broker negotiation is inactive; these are support changes, not additional
native extraction. See [current evidence/queue](EXECUTION_STATE.md), the
[declaration API](component-sdk/SERVICE_REQUIREMENTS_V1.md), and the preserved
[agreed broker design](P03_LEAF_BROKER_DESIGN.md). Checked [limit configuration](component-sdk/BROKER_LIMITS_V1.md)
passes ten API tests, including six existing declaration cases; it reserves no resources.
Checked handles/service grants additionally pass fifteen API cases, including prior cases.
The [offer/acknowledgement contract](component-sdk/BROKER_NEGOTIATION_V1_DESIGN.md) now passes
twenty API tests, including prior cases. Runtime negotiation, accounting and lifecycle enforcement
remain planned. Source
presence does not satisfy activation or full GUI acceptance.

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

The [native catalog prerequisite audit](component-sdk/design/native-catalog/NATIVE_CATALOG_NEXT_CHECKPOINT.md)
identifies two independently implementable native seams: a provider-owned endpoint capability
and an atomic forced-workspace policy epoch. Both remain prerequisites for selected remote
catalog authority. The audit explicitly marks saved catalog code as unlinked drafts and
defines native/custom external-worker acceptance; it is not extraction or runtime evidence.
The provider endpoint ownership change now passes 160 native provider/models-manager tests,
including five new local-HTTP cases; lint passed unchanged and test formatting was reviewed.
[Evidence](verification/2026-10-01/P03_PROVIDER_ENDPOINT_EVIDENCE.md) retains the initial
disk-full compilation failure and exact source transitions. This is a compiled native
prerequisite, not an independently installed catalog/provider or a new full-host/UI gate.
The subsequent [fresh full-host gate](verification/2026-10-01/P03_PROVIDER_FULL_HOST_EVIDENCE.md)
now passes on exact published `d04d5a7` source: unchanged installed storage, migration,
search and two cold GUI/manager-shutdown cycles. It adds consumer regression evidence,
not a catalog/provider plugin, live-model test or in-app Browser verification.

The native workspace-policy prerequisite now passes 256 login tests and the focused
App Server policy test (425 unrelated tests filtered), followed by unchanged scoped
lint and reviewed formatting. [Evidence](verification/2026-10-01/P03_NATIVE_POLICY_EVIDENCE.md)
binds all six source paths and preserves both failed compilation attempts separately.
Atomic manager-local snapshots reject old A→B→A revisions and fail closed on revision
exhaustion or detected poison. This is compiled native policy support, not installed
auth extraction. Asynchronous credential publication, owner changes, persistence,
request dispatch and cache publication still need their actual production fences.

The later [reload/acquisition checkpoint](verification/2026-10-01/P03_NATIVE_RELOAD_POLICY_EVIDENCE.md)
passes 271 login tests, including 15 new test functions. Private `auth_reload.rs`
fences loaded credential publication against the captured workspace policy;
`auth_acquisition.rs` checks current cached credentials in actual `auth()` and
factory acquisition without clearing a newer cache. Initial 266-pass evidence and
three intentional pre-fix failures remain separate. Final scoped lint passed unchanged and formatting was mechanically reviewed. This is native prerequisite work: independent credential/source-owner
changes, persistence, gateway settlement, dispatch/retries and catalog-cache
publication still need connected fences; no new full-host, GUI or installed-auth
proof is claimed.

### P04 — Configuration, credentials and native authentication

**Prerequisites:** P03 capability/owner semantics.
**Deliver, separately:** configuration source/load/refresh component; native OAuth,
device-code and API-key provider; credential persistence/keyring backend; workload/
AWS identity adapters. Resolved config snapshots include provenance, revision and
managed constraints. Kernel bootstrap only selects config roots; managed permission
ceilings and credential/account validation remain authoritative enforcement.
Replace actual native implementations, not merely `auth.resolve/refresh` examples.
Advance policy/owner epochs inside native setters, including A→B→A workspace changes.
The [storage ownership audit](AUTH_STORAGE_OWNERSHIP.md) fixes the next internal
order: conditional Ephemeral caller ownership, then complete coordination of the
overlapping File/Keyring/Auto writer cohort and shared passphrase initialization.
Persist generations through absence, distinguish conflict from fallback-worthy
errors, retain accepted-write custody and recover uncertain outcomes. An isolated
File adapter cannot claim safety while legacy cleanup still mutates its file.
This is a required native prerequisite, not an additional extracted component.
**Acceptance:** offline/interactive auth, refresh concurrency, refresh cancellation,
account switch, stale-owner rejection, keyring unavailability and private error logs;
config precedence/refresh/denied widening and existing credential/config migration.
Include equal-value/absence ABA across managers/processes and crash recovery at
write/cleanup/revision boundaries; explicitly exclude uncooperative legacy writers
from transactional concurrency guarantees.
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
Add immutable release compositions, stable launcher/router, durable session pins,
reference-safe retention and journaled new-session route transactions for A/B updates.
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

**Required product behavior:** an interactive update walkthrough in the separately
installed GUI, with equivalent typed headless operations and independent recovery.
Selection, per-plugin modification limits, verified backups and later rollback
after new writes are release requirements, not optional UI polish. The concrete
contract and implementation order are in [UPDATE_WALKTHROUGH.md](UPDATE_WALKTHROUGH.md).

**Prerequisites:** maintain current lineage through P00M now; do not defer missing
provenance until updater implementation. Implement candidate integration after P03
and representative native packages stabilize; activation/rollback uses P18
installation transactions. See [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md)
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
7. Implement the walkthrough and enforce its selected plan throughout staging:
   explicitly initiated pinned revision; current/proposed versions, diffs, affected
   components and source/custom mapping; conflicts and unresolved mappings; migration,
   backup and storage estimates. Select/exclude components, features and coherent
   change groups, including finer review only where valid. Show required dependencies,
   reject invalid combinations and validate the exact selected combination. Persist
   deliberate exclusions for later updates rather than silently reintroducing them.
8. Enforce per-plugin allowed changes to implementation, configuration, public
   contracts, dependencies and state schemas against the final classified diff.
   Unknown classifications and scope expansion block preparation/activation; a user
   change creates a new plan identity and invalidates affected evidence. Never broaden
   permissions silently to make an upstream patch fit.
9. Keep A usable while isolated immutable B builds/tests. Before activation verify a consistent recovery set
   covering host, exact packages/versions/selections, configuration, custom changes,
   conversations, attachments, auxiliary state and migration metadata. Protect secrets;
   show destination, completeness and verification, and demonstrate restoration.
   Stage/build/test outside the active installation, bind every gate to exact source,
   packages, configuration and plan, and display failed or skipped gates. Journal
   activation progress and coordinate versions. Health B before recoverably/atomically
   routing new sessions to it; existing sessions stay pinned to A and drain normally.
   Require consistent provider snapshots/catch-up, additive old/new schema overlap
   and fenced single-session/job ownership. Never promote stale candidate data.
   Incompatible or resource-blocked updates stay pending with A usable. Preflight
   A+B+recovery+build/migration+continuing writes; cap background CPU/RAM/I/O and share
   only verified immutable objects, never writable hard-link aliases.
10. Offer user-triggered later rollback for bugs discovered after successful updates.
    First preserve and verify a rescue copy of newer writes. Distinguish software
    rollback from state-snapshot restoration, show whether newer conversations can
    remain compatible, be reverse-migrated or exported, and never silently discard
    them. The external recovery entrypoint must work with the candidate host/updater
    unavailable. Keep update/recovery presentation alive independently of the engine
    being replaced; the current GUI/App Server lifetime coupling must be resolved.
    Retain the previous usable release locally and independent recovery. Software
    rollback routes new sessions while preserving newer data and explicitly pinning/
    draining active B sessions. Defer destructive schema cleanup past the rollback window.

**Acceptance:** integrate at least one real, later pinned upstream Codex revision
into an isolated candidate. Prove compatible, already-installed custom replacement
and additive plugins still work with their package digests unchanged, without
rebuilding those plugins; preserve both UI and headless recovery. Exercise both
rejection of a breaking/incompatible candidate and rollback after a controlled
failed activation. Recover the prior state/composition through the external
bootstrap with the candidate host intentionally unavailable. Where the candidate
changes persistent schemas, exercise the actual migration on preserved state,
interrupt or fail it after mutation begins, and prove recovery through a supported
downgrade or a pre-upgrade snapshot with its compatible host/package/schema tuple.
Record the no-migration compatibility basis when schemas remain unchanged.
The interactive browser flow must additionally exercise custom conflicts, invalid
selections, prohibited-scope changes, verified backup restoration, interrupted
migration, failed activation, host-unavailable recovery and manual rollback after
new post-update conversations/attachments/state writes. Verify those newer writes
remain recoverable; a pre-activation failure or software-only downgrade is not a
substitute. Prove A remains responsive and writes/streams during B staging, isolated
candidate tests cause no production side effects, new sessions route to B while
existing A sessions continue, and no job/session has duplicate ownership. Exercise
lost replies/interrupted cutover, insufficient-capacity refusal with A usable and
rollback with active B sessions. Repeat affected regressions and preserve actual failed/skipped results.
**Regression:** upstream/fork contract corpus, independent builds, installation/
upgrade lifecycle, old-data migration, security boundaries and actual GUI/manager
shutdown. **Exit:** exact provenance, dependency impact and rollback are reproducible;
the updater is installable separately; ambiguous changes are flagged; neither a
polling schedule nor live application is implied by candidate creation.


The subsequent [immutable selection-plan primitive](verification/2026-10-05/p18u-selection-plan/README.md)
passes19 scoped fixtures on exact sourceee5e3ad3. It binds explicit choices/exclusions,
graph constraints and per-owner whole-file permission plans; historical real-impact
mapping gaps remain blocked. The subsequent0.4installed planning/revalidation adapter
passes79 focused/affected cases and36 direct installed-package commands on unchanged
managerf054; [exact evidence](verification/2026-10-05/p18u-selection-tools/README.md).
Its positive policy cases remain synthetic and the actual13-path mapping stays
blocked. Complete final-inventory audit, trusted filesystem collection, persisted
profiles, interactive UI, live A/B routing and recovery remain separate gates.
This supplied-evidence policy support adds no native extracted family.

The next pure [final supplied-inventory audit](verification/2026-10-05/p18u-final-scope/README.md)
passes50 scoped cases (18 new,19 plan,13 adapter) on8977-path sourcef27ab9e9.
It checks every supplied before/after difference against selected all-owner grants,
including executable modes and rename endpoints; unknown mapping stays blocked.
Trusted filesystem collection, original-object/semantic attestation and installed
audit exposure remain separate gates. All activation/authority flags remain false.


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
Complete the interactive P18U walkthrough with dependency-valid selections and
enforced per-plugin scope; verify full recovery-set restoration, interrupted
migration/activation and host-independent recovery. After a successful update,
create new conversation/attachment/state data, initiate later rollback through the
browser, and prove both the recovered old installation and the protected newer-data
rescue/export or supported reverse migration. Prove immutable A/B staging, healthy
new-session cutover, pinned old sessions, write preservation/duplicate prevention,
capacity rejection with A usable, failed/interrupted activation and rollback while B
has active sessions. No silent data loss or stale-snapshot promotion is acceptable.
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
   separate user approval is required for a complete workspace suite unless an
   explicit applicable approval is already recorded; general continuation is not
   that approval. A resource-blocked
   run is a recorded limitation, not a pass. Never weaken lifecycle/security tests.
5. Exercise the existing GUI at each real-host integration cycle: streaming, tool
   progress, approvals, Stop, persistence/recovery and manager Launch Ctrl+C. The
   requested in-app Browser is currently unavailable in cloud; record that and use
   actual Chromium/Playwright runtime checks. Keep the requested Browser/manual
   gate pending where unavailable; do not hold core development for the optional
   relay or falsely label fallback tests as in-app Browser tests.
6. Run scoped `just fix`, `just fmt`; update generated config/protocol schemas and
   `just bazel-lock-update` when applicable. Do not repeat runtime tests solely for
   formatting; follow AGENTS' no-test-rerun-after-fix/fmt rule. Source-changing lint
   requires an explicit new source/acceptance decision; never bind earlier passes
   to a semantically changed tree.
7. Record commands, statuses, actual tested tree/diff, binary/package hashes, OS,
   fixtures and failures. Preserve original failed runs and corrected-run provenance.
8. Update inventory, execution queue, validation and P00M provenance. Bind each
   changed native boundary and intentional customization to its exact origin,
   destination, contract owner and evidence; retain unresolved mapping explicitly.
   Commit only the coherent milestone; retain unfinished work separately. Recheck remote refs, push without
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

## P03 next checkpoint: native credential-cache ownership

Source `18140083910d19c86e9cadaef0990db9aa652f6e` rejects stale native reload and
failure publication after credential replacement, clearing or A→B→A changes.
[Evidence](verification/2026-10-01/P03_CACHE_REVISION_EVIDENCE.md) preserves the
1-pass/7-failure baseline and corrected **385 passes**, including two retained
provider-endpoint HTTP cases. Lint passed unchanged; three formatting-only
transitions were reviewed. This is a compiled native C06 prerequisite, not a
new installed component. The later full-host/UI supplement below is separate proof.

Next: unify active provider and cached credential ownership, then fence pending
installs, equal-byte source replacement, refresh outcomes and shared credential
writes. Keep catalog activation fail-closed until its connected authority and
unchanged-host external-package acceptance pass. P18U remains required and
unimplemented; the new [lineage](upstream/p03-native-cache-revision-lineage.json)
preserves the current source/test/customization mapping without advancing upstream.

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

## Immediate lifecycle gate before advancing P03

The source-owner CLI was rebuilt from `99e6802f` (8,888 unchanged scoped entries).
Migration and two normal GUI cycles passed. Both unchanged storage attempts
passed their 13 behavioral commands but failed the strict five-second descendant
drain (exit 125). This remains a failed full-host checkpoint, even though GUI
manager SIGINT shutdown passed. Preserve both attempts and the sampled process
trace; never turn later zombie observations into a passing lifecycle assertion.

Investigate/fix curated marketplace startup synchronization ownership before
accepting the next full-host checkpoint: retain cancellation and join ownership,
stop new work on shutdown, terminate and reap Git process trees, and bound pipe
and HTTP cleanup. Cover real CLI and both App Server shutdown routes. Keep trusted
Git selection, credential restrictions, proxy/TLS and normal plugin behavior.
Adjacent detached marketplace refresh paths need their own audit. The native
source/installer work remains staged independently; disabling the feature or
extending the test drain would not repair the default lifecycle. Record exact
upstream/customization lineage and test this failure with a controlled held child,
then rerun default-feature installed storage and GUI acceptance.

## Native authentication installation ordering checkpoint

Source `65511842d7051b2a1f5cc52917f3ebb5c03be4f3` adds latest-admitted install
intent and retained source identity under the native credential owner. Newer
failed/cancelled attempts and clear invalidate older pending installs; captured
policy/cache changes reject stale publication. Equal credential installs retain
watch compatibility while source identity advances. [Evidence](verification/2026-10-01/P03_INSTALL_ORDER_EVIDENCE.md)
preserves six baseline assertion failures and **400 passing login/provider tests**,
clean lint and two mechanical formatter changes. [Lineage](upstream/p03-native-install-order-lineage.json)
binds original upstream anchors and intentional private support separately.

This is a compiled native prerequisite. It does not establish source-aware
load/refresh publication, conditional persistent writes, installed auth/catalog
selection or broker activation. No newer full CLI/UI acceptance is claimed. The
curated startup lifecycle failure remains the next full-host gate; its bounded
transport proposal is archived and uncompiled. Complete transport, cancellable
pipeline and actual owned host shutdown, then rerun unchanged storage/migration/GUI
checks before claiming recovery. P18U remains required and unimplemented.

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


## P03 locked-attempt prerequisite checkpoint

Development source `9a2be0abfa51d5baf932301ab3158b4b4aef8f19` introduces an
immediately used private owner boundary without changing the original transport body.
[Evidence](verification/2026-10-01/P03_CURATED_LOCKED_ATTEMPT_EVIDENCE.md) records
476 passing core-plugin tests, clean lint and reviewed formatting. No extraction count
or full-host gate advances. Next is the atomic typed-outcome/resource-custody/manager
admission transition: unknown cleanup retains resources and blocks retry; later direct
reap cannot clear it. Recovery requires external termination/fencing evidence, not merely
restarting the host. Keep this candidate separate from main until combined host/UI gates pass.


### Retained native sync ownership checkpoint (source739)

B1a is now integrated at `739eb89c537a29f715e79ebf6e6405181fec9744` on the
isolated lifecycle development branch. [Exact evidence](verification/2026-10-01/P03_CURATED_RETAINED_OWNER_EVIDENCE.md)
and [lineage](upstream/p03-curated-retained-owner-lineage.json) bind typed transport
outcomes, primary attempt resource custody and exact worker generations to the real
native pipeline. Unknown cleanup, publication uncertainty and unwind retain locks,
staging and admission despite discarded errors; later direct-child evidence never
clears quarantine. Ordinary confirmed failure keeps the existing fallback policy.
The scoped gate passed557/557 with strict process cleanup; two compiler failures
remain preserved. Lint/format transitions are recorded separately.

This is a native lifecycle prerequisite, not another installable component or a
full-host shutdown result. Next is B1b shared stop/deadline control across worker,
lock and Git, followed by HTTP/body cancellation, recoverable publication, callback
ownership and actual host shutdown. Preserve quarantine through these changes;
restart alone cannot prove recovery. The full CLI/installed-storage/UI regression
must still pass before main promotion. P00M mapping continues and P18U remains a
required, unimplemented release gate; no polling or live update is activated.

### Shared cancellation checkpoint (sourcecb9)

B1b is integrated at `cb9f7409071e64293801e52b21a2fc1d1a87de6f` on the isolated
development branch. [Evidence](verification/2026-10-01/P03_CURATED_SHARED_CONTROL_EVIDENCE.md)
and [lineage](upstream/p03-curated-shared-control-lineage.json) bind the shared stop/deadline
control, worker Closing state, cancellable stable lock and Git cleanup, typed stage admission
and callback enqueue guard. The admitted activation/SHA pair remains continuous; this is not
transactional publication. First run572/572 passed, strict0/null, clean lint and a recorded
formatting transition. This adds no independently installed subsystem or full-host proof.

Next C2 isolates policy-aware HTTP collection and extraction. Chosen proposed defaults are
a30s absolute request budget, metadata1MiB/archive64MiB/diagnostics8KiB observed response caps,
and extraction512MiB total/64MiB per file/20,000entries/64KiB chunks/30s. These are deliberate
compatibility limits, not measured upstream maxima. They require normal and over-limit fixtures.
Keep response/client/futures inside the native worker and finish ordinary Runtime teardown
before downstream publication; hidden resolver/client blocking work may delay that teardown.
ZIP constructor allocation and individual decoder/syscall latency are separate unresolved gates;
cooperative checkpoints must not be labeled universal hard bounds. Review these limits in C2
before adoption. Then finish publication recovery, queued work and actual shutdown integration,
followed by unchanged real-host/UI gates. P00M lineage closure and P18U upstream update/rollback
remain required and incomplete; no polling or live update is configured.

### HTTP await and teardown checkpoint (source922)

C2a is integrated at `92212516ad4d12bcf60546ee5f879b983ed44682`; [evidence](verification/2026-10-01/P03_CURATED_HTTP_AWAIT_EVIDENCE.md)
and [lineage](upstream/p03-curated-http-await-lineage.json) bind the actual policy-aware request/body
path, original metadata charset/BOM behavior and retained ordinary Runtime teardown. First scoped
run580/580 passed with strict0/null; lint/style/format transitions retain exact source identities.
The byte/deadline defaults above are now implemented for HTTP, while extraction remains unchanged.
This establishes neither a hard blocking-work join deadline nor installed-component/full-host proof.

C2b will independently add cooperative extraction limits. The [shutdown audit](verification/2026-10-01/P03_CURATED_SHUTDOWN_INTEGRATION_AUDIT.md)
pins21 source files at sourcecb9 and identifies remaining native ownership gaps: queued callbacks
and config work, process-global stop conflicting with embedded TUI replacement, and a45s stdio
watchdog that can preempt longer storage/GUI cleanup. Implement exact completion/callback scopes,
an explicit process-final owner distinct from embedded leases, publication recovery and coordinated
drain deadlines before new full-host promotion. Slow-cleanup tests must go beyond the previously
fast Ctrl+C cases; forced/deadline exits retain uncertainty. These repairs remain prerequisites
for installed auth/catalog, not substitutes for further extraction. P00M/P18U/P19 gates remain.


### Source922 fresh runtime checkpoint — 2026-10-02

The user-prioritized [rebuilt-host regression](verification/2026-10-02/P03_SOURCE922_FULL_HOST_EVIDENCE.md)
now passes: freshCLI890b4da,13 storage commands,10migration commands,2cold GUI cycles,
all unchanged strict runners0/null. Existing independent storage/search packages and binaries
remained unchanged during runtime; actual Launch Ctrl+C exited0 in both cycles.
The earlier source99 lifecycle failures remain separate. Sampled Git SIGPIPE statuses and
fixture/browser limitations are recorded, not dismissed.

This is exact-source922 compatibility/regression proof, not another extracted component or
completed shutdown contract. Native-worker completion was the next slice and is now scoped-tested
at source27d below; source922's runtime passes are not tests of those newer bytes. Owned curated
callback scopes, explicit process-final integration and held Git/HTTPS/slow-cleanup checks with
coordinated watchdog deadlines remain required. Keep C2b extraction separate, preserve quarantine
and repository/SHA journal/fencing requirements, and do not promote merely because fast regression passed.
Auth/provider extraction, all remaining inventory rows and the P18U updater acceptance remain
required; the optional remote viewer does not block them.


### Native completion observer checkpoint (source27d) — 2026-10-02

Source `27d004ccd824b54a86400cee215af7ccd1148c76`, tree
`2fa75acdf837420041e35d72ec2c157be1d0a37d`, adds the explicit process-final begin API and
retained observation of the exact native worker handle. [Evidence](verification/2026-10-02/P03_WORKER_COMPLETION_EVIDENCE.md)
and [lineage](upstream/p03-worker-completion-lineage.json) preserve proposal, adopted/tested and
formatted identities. First scoped compilation/run passed **587/587** (516 core-plugins + 71 PTY),
zero skips/retries, unchanged strict runner exit 0/null error. Lint made no edits; reviewed
format-only changes affect five files. Tests were not rerun solely for formatting.

Seven native regressions exercise idle closure, result-before-attachment ordering, observer
expiry/cancellation, two concurrent native observers, spawn failure, post-outcome panic and
Linux TLS-controlled stop during retry join. Exact join status stays separate from immutable
sync outcomes and quarantine. Pending observation retains unfinished custody. Synchronous join
and arbitrary panic-payload destruction can exceed polling deadlines; no hard return bound,
universal descendant fence or durability receipt is established.

**Next:** rebase/review the separately staged callback scope, synchronously register accepted
work before spawn/enqueue, retain exact handles and await accepted work before dependencies
drop. Complete late-owner visibility and owned refresh/config-work review. Integrate one explicit
binary-owned process-final authority across actual success/error/final shutdown paths while
embedded replacement and sibling scopes remain usable. Run held production Git/HTTPS, cancelled
observers and shortened-deadline uncertainty tests before repeating rebuilt CLI/installed storage,
migration and GUI Launch cancellation/recovery/shutdown gates with coordinated watchdog budgets.
No production caller/callback scope was included in source27d, and source922's prior runtime
successes are not proof of that missing integration. This prerequisite adds no independently
installed component. C2b extraction, publication recovery, native auth/catalog extraction, all
remaining P04–P19 subsystems and P18U real upstream update/rollback acceptance remain required.


### Curated callback ownership and actual routing (source0c1) — 2026-10-02

Ordered commits `8d3beeff520177c3dee7562d168253ed7bdeafda` (scope/export/tests,3 paths) and
`0c1ed130daf3dc36768e35109c576336d9f66005` (actual routing/lifetime guards,13 paths) extend
publication6856/source27d. [Evidence](verification/2026-10-02/P03_CURATED_CALLBACK_SCOPE_EVIDENCE.md)
and [lineage](upstream/p03-curated-callback-scope-lineage.json) bind adoption, failed/green tests,
post-test lexical lint correction, final formatting and both source commit trees. Only their
combined integration ran: **953/953** library tests passed (429 App Server+524 core-plugins),
zero skips/retries and unchanged strict0/null. Initial link failure101 ran no tests and coincided
with disk exhaustion; it remains separately preserved. First lint warned, a reviewed two-brace
test scope correction followed, second lint was clean, and13 final formatting changes were
reviewed as nonsemantic. Tests were not rerun solely for lint/style.

Real curated startup/account callbacks now reserve per-processor custody before spawning and
retain exact handles/failures through cancellation. Local lifetime guards await admitted bodies
before later thread cleanup and remain reachable after processor failure. Generic callbacks and
immediate account refresh retain their previous paths. This advances the native prerequisite
and actual caller routing; it adds no independently installable component or whole-host result.

**Next gate:** exact ownership and joining of MCP refresh/prewarm descendants; callback return or
invalidation acknowledgement alone is insufficient. Complete registry admission fencing against
late scopes and native completion delivery to replacement/sibling generations, then actual binary
process-final authority for success/error/early returns with one absolute deadline and explicit
force/uncertainty policy. Test held production Git/HTTPS, registration/completion races, cancelled
observers, slow cleanup and same-process replacement, followed by a source-bound rebuilt CLI and
unchanged installed storage/migration/GUI Launch streaming/approval/cancel/recovery/shutdown gates.
Source922's passing full-host result stays separate; its CLI is now archive-restorable, with mutable
aliases intentionally dangling. No current callback full-host or in-app Browser proof exists.

Keep C2b extraction, repository/SHA recovery journal, host-death fencing, native auth/catalog,
remaining P04–P19 extraction and P18U real upstream integration/rollback on the ordered queue.
Main stays source655 until the combined host contract gates pass. Viewer connectivity is optional;
P00M provenance and verified source/evidence checkpoints continue without it.

### Process-final integration and resource recovery — 2026-10-02

The exact candidate and next acceptance actions are maintained in
[EXECUTION_STATE.md](EXECUTION_STATE.md). Six current native suites have completed:
703 native/plugin/transport/process tests,431 App Server,42 client,73 exec,299 CLI
and88 manager tests:1,636 executed passes, strict0/null and unchanged source maps.
CLI and manager each ignore one subprocess helper; neither is counted passed.
Manager's short injected-budget checks do not establish production210s cleanup behavior.
No fresh full-host proof or new extracted-component count follows from these results.

The TUI test-target compiler was killed before tests. A later actual production CLI/manager
build failed with ENOSPC in the normal TUI library. These are distinct retained failures.
The cloud execution transport then failed; status metadata returning connected did not
restore terminal process creation. See the
[exact recovery checkpoint](verification/2026-10-02/P03_PRODUCTION_BUILD01_AND_EXECUTOR_RECOVERY.md).
No same-source retry is warranted until execution, temporary backup restoration and measured
disk/memory headroom are established. Existing successful checks need not be repeated just
because an older historical queue above named them.

Next: regain execution on the original VM, verify saved source and archives, and review fresh
resource guards. Rebuild/source-bind actual CLI and manager; repeat independently installed
storage/migration/GUI streaming, approval, cancellation, recovery and actual Launch shutdown.
Run held production Git/HTTP and real slow-storage normal/forced cases, including an admitted
relay-held write exceeding45 seconds. The relay boundary precedes native storage admission.
Same-process A→B replacement must prove no stale A effects and one refresh in active B.
TUI tests, changed App Server lifecycle integration, scoped lint and global formatting remain.
Source922's prior green runtime checkpoint stays separate from the current native candidate.

MCP session/transport custody and pinned HTTP-worker completion remain staged and uncompiled;
their source/ownership/real-worker gates are required before any all-host cleanup claim.
C2b cooperative extraction, transactional repository/SHA publication and host-death fencing
remain prerequisites before native auth/catalog extraction. Additional observation/callback/
replacement-acceptance proposal source is externally preserved at1f808b9; it is not adopted.

This is still the P03 prerequisite gate. P04–P19 and the required P18U updater retain their
scope and acceptance criteria; optional viewer networking remains independent.

### Independent P00M progress during executor startup failure — 2026-10-02

The [process-final lineage supplement](upstream/p03-process-final-lineage.json) maps all56 current
P03 candidate paths,56 selected literal anchors and two unchanged manager references. Direct
reads of all31 upstream-existing paths match official pinned OpenAI source. The [audit](verification/2026-10-02/P03_PROCESS_FINAL_LINEAGE_AUDIT.md)
records eight semantic impact groups and five local dependency edges. This does not update the
frozen normalized index or close its unresolved findings, establish a new extraction, implement
the updater, or satisfy any current-source runtime gate. No live update/polling was enabled.

Original-executor startup now fails before terminal tools are exposed, despite connected status
metadata. Follow EXECUTION_STATE.md recovery first; current production build01 ENOSPC is the last
recorded result. Keep P03 runtime/MCP/publication-recovery gates and P18U integration/rollback open.

### Recovered current58 host checkpoint — 2026-10-02

This supersedes earlier executor/ENOSPC blockers and historical "latest source922" statements.
The original source and archives were verified; exact guarded cache retirement enabled the
production build. Current58 binds8,921 files and unchanged native candidate bytes. Three SDK
acceptance changes explicitly permit reuse of an originally independently built search package
on a newer immutable host, without relabeling reuse as a new independent build. Installed storage,
manual migration, ordinary/search GUI and slow installed-storage normal/forced Launch gates passed.
The real GUI used Chromium with deterministic inference; in-app Browser remains unavailable.

P03 next actions are the remaining held Git/HTTP native ownership matrix, same-process A→B
replacement, MCP custody, TUI/integration/lint/format checks and crash-safe publication. Preserve
the raw-input-without-UI-event interrupted-history gap as P07/P16/P14 work. The complete evidence,
retained failed attempts and next action live in EXECUTION_STATE.md and the recovered-host report.
P04–P19/P18U retain their scope; no upstream integration/rollback acceptance or whole-host clean
claim follows from these bounded passes.

### Native observation stage1 continuation

[Eight-path lineage](upstream/p03-curated-observation-lineage.json) records copy-only worker and
callback snapshots: finished handles remain distinct from joined ownership. Six new tests and
the existing core-plugins library suite passed542/542, with exact source/strict evidence in
[the stage1 report](verification/2026-10-02/P03_OBSERVATION_STAGE1_EVIDENCE.md). This adds no
installed subsystem. Lint/format, actual same-process replacement and a rebuilt-host held matrix
remain pending. Preserve prior current58 runtime proof separately. The remaining replacement
fixture must explicitly isolate its SQLite root before initialization. P18U impact review must
retain these ownership/observation distinctions; no update integration or rollback has run.
