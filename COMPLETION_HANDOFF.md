# Completion handoff — cloud primary, offloaded native builds

Updated 2026-10-05. This is the coordinator's ordered execution view of the
[canonical roadmap](IMPLEMENTATION_ROADMAP.md), not a second or reduced definition
of completion. [EXECUTION_STATE.md](EXECUTION_STATE.md) owns the current resume point;
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md) owns all C00–C27 coverage;
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md) owns update provenance and policy.
Historical evidence retains its original source and scope.

The user directs continued execution through the full platform without repeated
requests to continue. **Infrastructure acceptance still precedes feature work.**
OpenAI cloud environments remain the primary development/coordinating environments.
Proxmox provides explicitly offloaded builds and durable recovery; it is not an
implicit relocation of the project or permission to expose home management services.
The original cloud VM, both workspaces, index, accepted binaries, failed build state,
all proposals and recovery archives remain intact. Do not retry unchanged native
builds, alter protected output aliases, or repeat completed capacity reviews.

## Source and acceptance baseline

| Identity | Meaning |
|---|---|
| Official import `d42056091aded7feb1d88ac7e83972108b2aa478` | Original local HEAD; Apache LICENSE/NOTICE/provenance retained; no second harness substituted |
| Main `781080f7e3c8bfe1953378001d777dff33d74bc3` | Last observed main, distinct from WIP |
| WIP `47b225fc3b5ee4bd3cf1b37e056f5818f58a54ac` | Latest verified publication entering this continuation; documentation/source-custody only |
| Assembly base `c0412a00d90cca162e7a54b5c6aa06cb67979f55` | Pinned complete published custom source; later documentation does not change the native projection |
| Accepted implementation `914cc59374c1149463e78bc33851d83e3f14d0a4` | Accepted native source projection `42e3899a…`, 8,949 paths; CLI `8e8a5dac…`, manager `f054d84a…` |
| Candidate projection `b75efc9751977009a9b28f2893c120db45a8f6e79f4993125e3685ae90d6cf5b` | 8,980 paths including maintenance changes and coherent eight-path auth overlay; adopted/formatted, uncompiled and untested |

Full identities, eight exact native paths/preimages, nine separate unactivated
document/helper variants, 325 published preservation paths to inherit, locks and
controls are in [NATIVE_BUILD_WORKER_HANDOFF.md](NATIVE_BUILD_WORKER_HANDOFF.md) and
its [sealed public manifest](verification/2026-10-05/native-worker-handoff/MANIFEST.json).
Do not start from upstream plus eight files, silently activate the nine variants,
or mistake the original checkout's `git diff HEAD` for the published-platform diff.

Existing proof is deliberately bounded:

- Native selected thread persistence/manual migration, inline attachment subset
  and bounded search have independently built/installed replacement evidence.
  These are three bounded native families, not complete storage/search domains.
- Model/auth overrides are adapters; tool/context/lifecycle contributors and the
  separately packaged desktop GUI are additive. Most engine services remain coupled.
- [Source42 production evidence](verification/2026-10-02/p03-auth-refresh-runtime/README.md)
  records all twelve storage/migration/attachment/GUI/slow-Launch/held-Git-HTTP gates.
  Slow normal Launch recovered the held event after 46.45 seconds; forced Launch
  reported unknown durability. Reused package bytes, deterministic inference and
  real Chromium fallback are explicit. No in-app Browser or live-provider proof.
- [Installed overlay cancellation](verification/2026-10-05/p18u-installed-overlay-lifecycle/README.md)
  is DONE at the Git exec boundary: cooperative and abrupt installed-SDK primaries,
  tracked cleanup and later successful manager invocations. Do not replay it.
  Git computation beyond exec and manager graceful forwarding were not proved.
- Maintenance 0.1.1–0.4 has real installed review/capsule/sparse-overlay/selection
  evidence. [Final supplied-inventory audit](verification/2026-10-05/p18u-final-scope/README.md)
  passed 50 fixtures on `f27ab9e9…`; it does not attest filesystem completeness.
  Collector drafts remain unadopted/unsealed. No complete updated-host, activation,
  recovery or A/B release gate has passed.
- Current auth attempts ran **zero native tests**: first missing registry metadata,
  then a read-only compiler output. Repaired metadata and source custody are not
  native acceptance. Older tests cannot be assigned to the candidate.

## Infrastructure gate and automatic continuation

The focused infrastructure owner supplies worker readiness, not this original
checkout. [CAPACITY_PLAN.md](CAPACITY_PLAN.md) retains measurements and protected
recovery constraints. [NATIVE_OUTPUT_HANDOFF.md](NATIVE_OUTPUT_HANDOFF.md) remains an
unexecuted original-VM recovery design; a clean worker need not mutate those aliases.
Historical warm-cache allowances do not admit a cold build.

New bounded preparation completed: a private input packet contains **32 payloads /
1,550,203 logical bytes**: eight exact auth files, fourteen unpublished runtime
helper sources, five bootstrap/observer controls, four exact command-reference
files and the full 8,980-path canonical source map. That map's bytes hash directly
to `b75efc97…`; it was derived from preserved records without rescanning the cache
or whole source tree. Selected originals and the index remain unchanged. The packet
is not a checkout, archive, transfer or executable bootstrap. Original absolute
paths and old producer/tool assertions are deliberately retained for review.

The [readiness receipt](verification/2026-10-05/completion-handoff/MANIFEST.json)
publishes metadata only. Exact packet bytes remain under private recovery custody.
The resumed bounded static review covered 31 code/control files (364,822 bytes);
its [concrete findings](verification/2026-10-05/completion-handoff/BOOTSTRAP_CONTROLS.md)
do not establish universal secret safety, runtime portability or export authorization.
A permitted exact delivery selection/route remains pending.
The nine separate variants, private state and caches are excluded, not discarded.
Known gaps include an immutable OS/native-library closure (nine GStreamer/ORC
package pins alone are insufficient), complete tool/browser artifacts, portable
control/import closure, wired resource observation and new independent-package
lock/proof preservation. No worker-native command was dispatched.

**Concrete bootstrap next action:** close companion-input and exact delivery
selection, preserve originals and derive a separate portable control/image recipe
with new identities. In parallel the infrastructure owner supplies the worker's
authenticated access and admitted resources. Only then deliver/read back the exact
unverified source generation and dispatch the admitted validation sequence.

| Gate | Required implementation/evidence | Exit / owner |
|---|---|---|
| I1 Worker access and isolation | Authenticated supported command/file access; identified retained worker and storage; observed quota/inodes/cgroup/process/FD limits; verified dependency access; isolated test homes/sinks; no public management exposure | Infrastructure coordinator records actual worker readiness, not a display name or ping alone |
| I2 Source/control bootstrap | Complete pinned published source plus coherent eight-file unverified overlay; exact readback; retained excluded variants; reviewed portable strict/resource/runtime controls; Rust 1.95.0, locked Git/registry/V8 and native sysroot/tool provenance; fresh writable targets/CARGO_HOME | Derived controls get new hashes and preserve assertions; no damaged cache, old binary or personal data dependency; unresolved input fails closed |
| I3 Whole-sequence capacity | Admit cold dependencies, test and production outputs, lint/link overlap, independent packages, evidence, recovery/restore workspace and continuing writes across persistent storage/tmpfs/shared RAM; measure clean build peaks and keep growth/rollback reserve | Scalable resources and bounded ownership monitoring cover every step, not merely one successful small command; do not kill Rust commands to enforce a monitor timeout |
| I4 Native proof | Source-bound focused login, full grouped login, provider regression, scoped lint, CLI/manager and search binary builds; separately built packages and all thirteen runtime gates listed below | Exact tested source and producer/package hashes; unchanged-host install/replace/remove; current GUI and lifecycle evidence; failures retained separately |
| I5 Durable recovery | Retained accepted worker source, controls, binaries/packages and evidence with manifests/checksums; authenticated approved backup destination and isolated restore verification; explicit custody/retention for original unique work | Distinguish source durability from private-state backup; source-first bootstrap needs no private archive upload; no original retired without its own authorization and full restore proof |

Infrastructure acceptance is a source/control/environment-bound coordinator receipt
closing I1–I5 or explicitly identifying an equivalent reviewed implementation.
Do not mark I5 complete from a VM-local archive or GitHub source alone. Additional
private artifact movement needs an approved exact destination, scoped access and
retention decision; worker/bootstrap preparation can proceed independently.

On acceptance, continue automatically with the remaining native auth/P03 checkpoint,
then successive dependency-ready phases below. The previously paused installed-overlay
item is already complete, so it is not a prerequisite to rerun. The collector stays
preserved for its P18U prerequisite slot; it is not a storage-workaround priority.
No fresh request to continue is needed. A material missing access/approval or runtime
limit requires an exact ledger handoff, not an unlimited-execution promise.

## Native worker's first complete validation sequence

Use the exact grouped commands in [worker handoff](NATIVE_BUILD_WORKER_HANDOFF.md).
Acquire pinned dependencies into new owned inputs; do not change profiles/features
to dodge the suite. `CARGO_BUILD_JOBS=1` was already in use; serial nextest execution
reduces test-process overlap but is not a linker-memory bound. The resource observer
must be attached and its new worker identity sealed before admitting a build.

1. Verify published source, coherent overlay, lockfiles and adapted controls; preserve
   the candidate map before/after every command and any subsequent source transition.
2. Run focused auth-install/source/store cases, full grouped login and grouped
   provider cases through `just test --locked --retries 0`, retaining the existing
   package grouping and `--test-threads=1`. No direct `cargo test`.
3. Run scoped `just fix -p codex-login --locked`; follow AGENTS formatting and
   no-test-rerun-after-fix/fmt rules. If lint changes semantics, preserve the old
   receipts and resolve a new source/acceptance sequence; never rebind old passes.
4. Build actual production CLI and manager, plus separately required search CLI.
   Export/build native thread, search and attachment packages outside host source,
   preserve original/resolved export locks and proofs, and build desktop via the SDK.
5. Freeze new host hashes. Execute storage → migration-normal → gui-normal →
   attachment → migration-search → gui-search → migration-slow-normal → slow-normal
   → migration-slow-forced → slow-forced → held8 → featured-postcheck → auth-consumer.
   Keep migration dependencies, exact cleanup/deadline assertions and fresh fixtures.
6. Prove manager Launch Ctrl+C, forced durability uncertainty, cold recovery,
   installed replacement/removal and all tracked child cleanup. Separately retain
   the still-open TUI 6+2 snapshots, App Server 16 cases, same-process replacement,
   raw/response-only durability, lower transport/constructor and MCP ownership gates.
7. Record native acceptance and resource/recovery receipts; publish only coherent
   verified source with readback. Preserved alternatives and failed states remain.

The inner commands are not complete acceptance without their control/resource
wrappers. New worker OS paths, local main-reference assumptions, Git/strace identity,
package source-absence and timestamp checks need reviewed adaptation. A skipped
process, tracing, sandbox or browser gate is unavailable, never passed.

## Remaining dependency-ordered product phases

Each row requires a versioned contract card, explicit inputs/outputs and sole state
owner, grants/dependencies, readiness/cancel/drain/removal, typed errors and uncertain
outcomes, migrations/compatibility, and a separately built native/custom replacement
exercise on the real host. File moves, traits or compilation alone never close a row.
Detailed deliverables/acceptance/regression/exit criteria remain in the linked
roadmap phases. P00M provenance and SDK/UI support accompany every phase.

| Order / phase | Prerequisite and remaining deliverable | Completion evidence |
|---|---|---|
| P00 / P00M — baseline and provenance | Maintain complete inventory and original revision/path/symbol → implementation/contract/customization/owner/evidence mapping; close unresolved current semantics | Exact source/index/accepted-vs-WIP ledger; current mapping checker and explicit unresolved findings, not just historical path coverage |
| P01 — thread maintenance | Accepted bounded migration implementation; retain compatibility and add regressions as dependent services change | Existing selected/default migration parity, cancel/close/cold recovery remains source-bound; remaining maintenance goes to P06 |
| P02 — search/workspace | Bounded search accepted; finish pending-open/constructor/consumer retirement gaps and assign file/watch/Git/import work through P11–P15/P17 | Installed selection across actual CLI/App Server/TUI/GUI callers, no accidental fallback, native corpus, cancel and replacement |
| P03 — shared lifecycle/broker | Finish current auth ownership prerequisite and lifecycle debt, Session construction cleanup, bounded model-v2 transport, actual broker grants/negotiation/quota/revocation; staged catalog is not active | Installed dependency consumer, large/fragmented bodies, startup/owner death/full queue/reentrancy, retained cleanup, same-process replacement and GUI gates |
| P04 — config/auth | P03 authority; finish Ephemeral then coherent File/Keyring/Auto writer/passphrase custody; extract config/requirements, OAuth/device/API-key, credential/keyring and workload/AWS identity separately | Actual native/custom packages, refresh/account/workspace/ABA races, cross-process writes/crash recovery, denied widening, private logs and migrations |
| P05 — model/catalog/providers | P03 bounded broker/model-v2 + P04 authority; catalog/cache, provider routing, HTTP/WebSocket inference, summaries and realtime ports | Real changed outbound requests/capabilities, ETag/offline/owner fences, >4 MiB payloads, stream/tool/usage/error fidelity, fallback/cancel/next turn |
| P06 — durable state/attachments | P03/P04; all typed auxiliary repositories and transactions, remote attachment upload/resolve/cache, provider-owned migrations | Separate native/custom stores; concurrency, interrupted schema migration, nonempty data/cold recovery, signed references and production resolve callers |
| P07 — replay | P03 constructor ownership + storage/state codec; activate all real resume/fork replay paths | Independent native worker, Legacy/Paginated differential corpus, malformed/media/compacted history, stale result, cancel/crash/revoke and GUI recovery |
| P08 — live history/context | P07 + bounded snapshots; mutable context transactions, prompt fragments/budgeting/world state and immutable projections | Actual model request corpus, old snapshots frozen, accepted-write/lost-reply/stale-owner recovery, complete originals preserved |
| P09 — compaction | P05/P08; native local/remote algorithms and trigger/budget policies with versioned proposals | Custom policy changes real decisions; manual/pre-turn/inline, queued input, model switch/cancel, fork/resume and evidence retention |
| P10 — permissions/approvals | P03/P04/P08; decision/review/network policy with independent enforcement ceiling | Allow/deny/late/stale/cached decisions, policy narrowing and cancellation; actual GUI approval/denial; no privilege expansion |
| P11 — exec/sandbox/files | P10 + P03; process/PTY/stdin/background output, OS sandbox backends, remote exec and constrained files | Real native/custom executor, process descendants, truncation/interrupt/retry/disconnect, permission denial and target-platform behavior |
| P12 — tools/code mode | P08/P10/P11; owned tool ports, native dispatcher/routing and built-ins, code-mode host/protocol/V8 | Replace actual existing tool/dispatcher, parallel streaming/patch fidelity, isolate lifecycle, hooks and cancellation after admission |
| P13 — MCP/connectors/skills/plugins | P04/P10/P12; separate native connection/runtime/catalog, connectors, instructions/skills and ordinary marketplace managers | Startup/reconnect/cancel/resource origins, auth epochs, elicitation, discovery/cache, install/remove/recovery; not an MCP-server-install demo |
| P14 — session/turn/agents | Stable P04–P13 ports and events; native factory, turn runner/input queue, scheduling/agent graph/budget/messaging | Replace actual loop/scheduler, concurrent steering, forks/children, pending input, orphan-free shutdown and recovery; Session is not renamed kernel |
| P15 — background/extensions | P06/P09/P12/P14; memory/read/write/consolidation/jobs, goals/queue/hooks/history notes/Guardian/agent-board/Git attribution/image/web | Every native contributor accounted for, leases/retries/disable/cancel/restart, bounded reviewer evidence and actual replacement |
| P16 — events/observability | P03 + stable producers; domain event/projection service and telemetry/analytics/feedback/diagnostics/trace consumers | Slow consumers, order/terminal errors/reconnect, privacy, optional telemetry, bounded flush and preserved durable session work |
| P17 — clients/remote/media | P14/P16/P04/P06; actual App Server, CLI/TUI, cloud tasks/remote control, voice/realtime and non-Rust clients; keep GUI working throughout | Independent GUI and headless packages, public RPC/schema/snapshots, pairing/revocation/disable and OS/client/executor coverage |
| P18 — SDK/distribution | Incremental authoring for all earlier ports; final dependency/version solver, transactional package lifecycle, composition lock, A/B router/pins/recovery and releases | Outside-source build, source removed, unchanged host, upgrade/rollback/crash/corrupt/missing/cyclic dependencies, retained data, licenses/SBOM and platform recipes |
| P18U — selective updater | Current provenance throughout; collector next in this track; stable P03/native ports and P18 transaction primitives before activation | Seven update gate groups below; complete later revision integration, interactive selections/scope, no-side-effect staging, A/B and lossless recovery |
| P19 — full-platform acceptance | All inventory rows and advertised platforms have accepted contracts/packages or reviewed kernel/support classification | Clean-install scenario, current regressions/security/kernel dependency audit, real model, interactive update and recovery proof; no unexplained coupled fallback |

Coverage cross-check: C00→P03/P18; C01/C12→P14; C02→P01/P06;
C03/C04→P06; C05→P05; C06/C07→P04; C08→P07–P09;
C09→P11/P12; C10→P10/P11; C11→P12; C13/C14→P13;
C15/C16/C17/C24→P15; C18→P02/P11/P12/P15/P17;
C19/C20/C22→P17 (realtime model ports also P05);
C21→P03/P05/P11/P17; C23→P16; C25/C26→P00/P18/P19 support;
C27→P00M/P18U. Pure libraries are not plugins; hidden mutable services are not pure
libraries. The inventory's auxiliary-state checklist and every contributor remain
mandatory; phase grouping must not become one monolithic replacement plugin.

## Minimal kernel: explicit exceptions, not unextracted domain code

Retain only bootstrap/admin install-selection, contract/composition validation,
stable generation routing and independent recovery, process supervision/bounded
transport/cancellation ownership, capability/owner identity and revocation,
permission-ceiling validation/enforcement, and minimal correlation/order checks.
These responsibilities need authority outside a failing/requesting component.

Native config/auth/provider/history/tool/session/agent algorithms, policy scoring,
OS sandbox mechanisms, domain events, telemetry, normal UI and maintenance planning
remain components. Shared wire schemas/codecs/deterministic utilities and build/test
support are documented C25/C26 exceptions. P19 proves the minimal host does not link
`codex-core` or native model/tool/history implementations. No permanent fallback or
unchanged Codex process wrapper may silently satisfy extraction. Trusted same-user
plugins are executable code; process boundaries alone do not isolate malicious plugins.

## Collector/updater completion: seven required gate groups

The authoritative contracts are [walkthrough](UPDATE_WALKTHROUGH.md) and
[A/B generations](LIVE_UPDATE_GENERATIONS.md). These groups consolidate existing
requirements; none is claimed complete by the installed reviewer or sparse overlay.

1. **Trustworthy inputs and selective plan.** Review/adopt the preserved collector
   only after native infrastructure/P03 priorities allow; bind race-resistant actual
   filesystem/object inventories to original/current component mappings, generated
   files, modes/renames/dependencies and custom conflicts. Persist valid exclusions,
   dependencies/coherent groups and fine selection only where sound. Reject unknown
   mappings and invalid combinations. Validate the selected combination itself.
2. **Interactive policy and final scope.** User initiates and pins the revision;
   show before/after versions/diffs, ownership, conflicts, migrations and storage.
   Per-plugin implementation/config/public-contract/dependency/schema permissions
   bind the actual final diff. No silent expansion. Failed/skipped/stale gates stay
   visible. The independent maintenance/recovery presentation survives engine exit.
3. **Complete isolated later-upstream candidate.** Extend the real pinned later
   revision beyond thirteen sparse paths into a complete adapted host/package/SDK
   composition with exact custom provenance, coordinated versions and schema plan.
   Real-host/security/regression/UI gates bind exact source, package, config and plan
   digests; unchanged compatible custom packages must still operate. Candidate
   tools/model/connectors target isolated sinks; no production external side effects.
4. **Consistent verified recovery.** Cover host, exact package bytes/selections,
   config/custom changes, conversations/attachments/auxiliary state, schema/migration
   metadata and independent bootstrap. Protect secrets, expose completeness and
   destination status, and restore in isolation. A hash or source-only backup does
   not establish state consistency or restorability.
5. **Usable A during B preparation and cutover.** Immutable generations, bounded
   background work and A+B+recovery+build/migration/continuing-write reserve. Consistent
   provider snapshots plus catch-up, additive schema overlap and fenced single-job/
   session ownership preserve writes. Health B before journaled atomic new-session
   routing; existing A sessions stay pinned and drain. No stale candidate snapshot
   replaces current data, no writable hardlink sharing. Unsupported overlap leaves
   B pending and A usable rather than forcing downtime.
6. **Failure and independent recovery.** Reject an incompatible/breaking candidate;
   interrupt migration after mutation begins and activation around intent/commit/lost
   reply. Reconcile durable progress, restore through bootstrap with host/updater
   unavailable and keep A usable on failed/insufficient-capacity staging. Retain the
   previous usable release locally and independently retrievable recovery material.
7. **Later rollback after writes.** Through the browser create new conversations,
   attachments and state under B, then request rollback. Verify a newer-data rescue
   first; distinguish software route rollback from state restoration. Pin or safely
   drain active B sessions; retain compatible data or prove reverse migration/readable
   export. Never silently discard newer writes or reopen an incompatible schema.
   Defer destructive cleanup until all session and rollback obligations end.

Routine candidate automation may follow explicit policy only after all required
gates; ambiguous semantics/contracts/migrations require review. No polling schedule,
unattended live deployment or running-session hot replacement is authorized.

## Final acceptance and retained permissions

Build the minimal host once and record its bytes. On a fresh installation build native
and custom model/catalog/context/tool/executor/scheduler packages with released SDKs
outside the checkout, install the GUI, then remove source access. Exercise actual
streaming, tools and approval/denial; Stop, compact, fork, shutdown through manager
Launch Ctrl+C, restart/resume, upgrade/rollback a compatible plugin, reject an
incompatible/missing dependency, remove GUI and continue headless, restore composition
without losing data. Host hashes remain unchanged. Repeat with the accepted later
upstream composition and all seven update groups. Include at least one configured
real provider or local model; deterministic fixtures alone do not close this gate.

At every relevant development cycle exercise earlier GUI behavior, not only new
screens. Use the requested in-app Browser when available. Existing cloud capability
does not provide it: record unavailable manual coverage and use real Chromium
fallback where supported, clearly distinct from mocks and live inference. Optional
remote viewer networking does not block independent extraction, but it also does
not erase the final interactive/browser acceptance requirement.

Run meaningful scoped `just test` and affected regressions; separate user approval
is still required for a complete workspace suite unless explicitly already recorded.
The general instruction to continue is not that specific approval. Preserve AGENTS
fix/fmt rules, original lifecycle/security assertions and all failed evidence. No
unapproved purchases/accounts, private uploads, archive retirement, destructive
replacement, public unauthenticated services or unrelated settings changes.
Authorized verified milestones may be committed/pushed without routine reapproval:
check remote refs, nonforce publish to the existing WIP branch and read back exact
files. Use a clearly labeled unverified source handoff if delivering the eight-file
candidate; do not present it as a completed feature.

At every checkpoint update the ledger with exact accepted/candidate identities,
active owned work, current gate result, unresolved rows and one concrete next action.
Continue automatically when dependencies pass. Stop only for a real access/resource/
authorization limit or completion, with a durable resumption point. This plan does
not promise an uninterrupted agent lifetime or an always-on deployed service.
