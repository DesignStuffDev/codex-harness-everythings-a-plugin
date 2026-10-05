# Execution state — Codex Harness Compartmentalized

Updated 2026-10-05. **Partial platform; P03 and whole-harness extraction remain
incomplete.** Read this file, [roadmap](IMPLEMENTATION_ROADMAP.md),
[inventory](COMPONENT_INVENTORY.md), root AGENTS.md and
[maintenance contract](UPSTREAM_MAINTENANCE.md) before resuming.

This concise checkpoint replaces the long chronological ledger without discarding
it: [exact prior ledger at395aac0](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/blob/395aac0e305739023f4636c229c4c65972050c9c/EXECUTION_STATE.md),
[checkpoint history](EXECUTION_CHECKPOINT_HISTORY.md), and private preimage
R/p18u-final-scope-docs-20261005-01/EXECUTION_STATE.md retain earlier actions/results.
Never replay a completed mutation from a historical next-action paragraph.

## Current priority: finish the paused acceptance, then native P03 — 2026-10-05

Selected build backing and measured headroom are now repaired in this same checkout.
**The critical native build/validation gate has not run.** Resume only the exact
paused installed-overlay cancellation acceptance to a bounded verified checkpoint,
then the eight-path native auth-install sequence. The collector and broader updater,
planner and scope-validator work remain paused; do not start a maintenance detour.
A Python acceptance pass will not prove native capacity or complete P03.

Latest published checkpoint before this documentation update:
**fd28a6c0187a7548c60812a8541aad30099e1344** on the existing WIP branch. The completed
source/final-scope publication2690688 and earlier395aac0 remain preserved. Do not
replay their mutations or the completed capacity operations.

Exact preserved resumption points:

- Installed-overlay cancellation: sealed R/p18u-overlay-installed-lifecycle-proposal-01;
  its cooperative SDK shutdown, abrupt plugin death, tracked Git cleanup and successful
  manager invocation after each primary still require fresh source/package/resource
  admission and actual execution. The synthetic trace calibration is not this proof.
- Inventory collector: R/p18u-inventory-collector-proposal-01/files contains two
  unsealed draft files plus PRE_FORMAT.json; no tests/adoption ran. Keep paused.
- Synthetic trace evidence: R/p18u-overlay-trace-calibration-evidence-20261005-01 remains
  unadopted. Resource-estimate correction at
  R/p18u-small-stage-resource-correction-20261005-01 remains preserved; earlier small
  estimates were exceeded, so do not claim their budget passed.
- Native auth: R/p03-ephemeral-token-install-cohort-proposal-01 remains unadopted,
  uncompiled and untested. Preserve the complete eight-path cohort and all alternatives.

## Completed capacity repairs and conditional native admission

[First repair](verification/2026-10-05/p03-capacity-repair/README.md), publishedfd28a6c:
all895 metadata and902 genuine registry archives restored/read-back exact,
827,656,451 logical bytes.55 duplicate-only paths reclaimed1,786,339,328 allocated
bytes after restoration/lease/alias checks. No recovery archive, source or unique
raw evidence was retired. Approximately2.065GB persistent space was free then.

[Second repair](verification/2026-10-05/p03-capacity-restored/README.md): root preserved
1,047 selected unchanged rlibs in a565,190,455-byte verified durable archive,
fully restored exact payloads to temporary backing, then journaled canonical alias
replacement. Final backing allocates2,634,592,256 bytes. The independent metadata
postcheck verified all1,047 aliases/portable identities and3,141 journal events;
it did not rehash payloads or run a build. Root separately verified8,977 source
files,16,906 fingerprint/depinfo inputs, the original index and accepted binaries.

Postcheck observed **4,126,289,920 persistent free bytes** and **8,842,485,760
hard-unused memory bytes**, exceeding the reviewed3,850,000,000-byte disk and8GiB
memory requirements. These are point measurements, not future reservations. The
phased3,797,387,752-byte planning envelope includes all required new test/production
outputs, growth, lint/link overlap, evidence, proof archives and a256MiB recovery
floor. Its442,593,280-byte missing ordinary core/TUI pair is counted once, within
the focused output comparator. It does not admit giant TUI/App Server/P03 debt.

Receipts: R/p03-cache-relocation-20261005-02/COMPLETE_MANIFEST.json SHA256
`8ce4184c7a95a18b3cb94bf465775e08a98add362970a8ae117eff2fbb58db6b`.
Archive SHA256 `0691ef18f30d705eacb611a0637e59dcecb241b04488952767cd72323254c353`.
Preserve archive, ownership/restore receipts and journals. Rehydrate/recover paths
are documented in the evidence. Three disposable synthetic cases/18 phase calls
passed, including lost backing, newer regular output preservation and interrupted
forward/reverse exchange recovery. They were not power-loss or production-disaster
rehearsals. Archives remain VM-local. Do not rerun preserve/restore/relocate.

Read [CAPACITY_PLAN.md](CAPACITY_PLAN.md), the exact phased card and fresh counters
before every remaining stage. Jobs1 already applied historically; it is not a new
saving. Old near-limit production included10.35GB shmem, much larger than current
selected backing. No blanket8GiB disk requirement or RAM-expansion claim remains.
No native build, targeted native test or new GUI run has occurred for these repairs.

## Identity and preservation

- P: `/workspace/codex-harness-everythings-a-plugin`; origin:
  `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
  Preserve sibling `/workspace/codex-harness-next-components`; no new checkout/VM.
- R: `/workspace/recovery-backups/20260930T165936Z`; A: `/workspace/acceptance`.
- Upstream imported pin and local HEAD:
  `d42056091aded7feb1d88ac7e83972108b2aa478`, official `openai/codex`.
  Apache LICENSE/NOTICE/provenance retained. No second harness substituted.
- Original index SHA256:
  `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
  Do not stage/reset it. Selected publications use Git object APIs and readback.
- WIP: `wip/p03-process-final-and-mcp-preservation-20261002`.
  Main remains `781080f7e3c8bfe1953378001d777dff33d74bc3`; no force publication.
- Latest published installed-maintenance checkpoint:
  **`395aac0e305739023f4636c229c4c65972050c9c`**, tree
  `c71438ed33eb3d34ae90d6c18908029e47e323f8`; all17 files/442,831B and both refs
  read back. Receipt R/p18u-selection-tools-publication-20261005-01/PUBLICATION.json,
  SHA `c02a4f8b7307750a8f7fc09ac6b7fc149211306e70bcd8972e3ba2ffa671a065`.
- Preserve all unadopted auth/MCP proposals, previous outputs, linked cache backing,
  both source worktrees and archives. Full initial recovered-workspace archive SHA
  `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd` remains in R.
  Workspace-only archives are recovery checkpoints, not proven offsite backups.
  GitHub provides durability only for included source/evidence, not private state.

The task-bound environment reattached with the original checkout/index. Old `/tmp`
and `/dev/shm` backing and services disappeared; that is not shutdown proof or
physical-VM continuity proof. VM commands now execute; stale executor-access
blockers are cleared. The old mirror's promisor repair completed/restored before
reattachment; its mirror is absent. Do not replay it. Official chosen objects live
in an isolated bare test cache A/p18u-upstream-objects-20261004.git, not a checkout.
One separately observed Git PID2006 was Z/PPID1 with no independent owner attribution;
it is not dismissed or counted as cleanup success. Acceptance proves tracked IDs only.

## What is actually implemented and tested

Three bounded native families have replacement proof: selected thread persistence
and manual rollout migration; inline attachment storage subset; native search.
Most engine functionality remains coupled. Plugin host/API/adapters/SDK and
presentation/maintenance additions do not count as additional native extraction.
[PROJECT_REPORT.md](PROJECT_REPORT.md) and the28-row C00–C27 inventory distinguish
EXTRACTED, ADAPTER, STAGED, COUPLED and kernel responsibilities.

Preserved production CLI SHA256:
`8e8a5dac859825a910403fc85dc7aed4a4ddaf3cc1c7d585f3a0fb27a7d64dc0`;
manager SHA256:
`f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
Native accepted source map42e3899a remains8949 paths; accepted implementation914cc593
and roadmap777ee7c remain ancestors, not proof for later unbuilt auth proposals.

Current native/runtime evidence is in
[all12 current-host gates](verification/2026-10-02/p03-auth-refresh-runtime/README.md):
storage, four migrations, GUI/search, attachments, slow normal and forced Launch,
held Git/HTTP ownership checks. Slow normal Launch took46.45s and recovered the
held canonical message; forced second interrupt reported durability unknown.
These results reuse exact independently built native packages. Remaining P03 gaps
include the TUI6+2 snapshots, App Server16 lifecycle/search/storage cases, same-process
replacement, raw/response-only durability, lower-transport/constructor and remaining
MCP custody. No full-workspace pass or whole-host graceful cleanup is claimed.

The0.2 maintenance checkpoint also refreshed installed storage/manual migration and
two real Chromium GUI cycles: approvals/native tools, Stop/streaming, native search,
cold history and manager Launch Ctrl+C. Model responses were deterministic fixtures.
In-app Browser and Context7 are unavailable; Chromium is fallback evidence, not
in-app manual or live-provider proof. No0.3/0.4/final-scope GUI rerun is claimed.
Remote viewer connectivity remains optional and unverified; do not retry unchanged
network blockers or let them block independent development.

## Current maintenance progression

1. **0.1.1 direct-Git custody:**58 focused cases and23 installed commands. Held real
   Git, cooperative cancellation and abrupt manager SIGINT cleanup, then subsequent
   successful invocation. SIGINT exit-2 is not graceful forwarding. Earlier failed
   attempts remain. [Evidence](verification/2026-10-04/p18u-owned-git/README.md).
2. **0.2 sealed source inputs:**11 capsule+13 review fixtures and49 installed
   commands; real later upstream2e5fea64 against custom914cc593,13 paths/25 blobs.
   Held Git cooperative/abrupt cancellation leaves incomplete jobs and tracked
   cleanup; host-independent inspection after removal. [Evidence](verification/2026-10-04/p18u-source-capsule/README.md).
3. **0.3 sparse transformation:**47 focused cases and61 installed commands. All13
   actual later-upstream output records/bytes verified independently, including
   custom scenarios.rs import removal. Source merge is not complete updated-host
   integration. All seven release gates remain pending; active-merge cancellation
   is still separate. [Evidence](verification/2026-10-04/p18u-candidate-overlay/README.md).
4. **Selection primitive:**19 pure fixtures; immutable selections/exclusions,
   dependencies/coherent groups and owner scopes. Publishedb66c9f2.
5. **Installed0.4 planning/revalidation:**79 focused/affected cases and36 direct
   commands (31 manager,17 tool responses) on exact8975-path map
   `d84cf069cea7be4da7aa68cbc43d56f72483d3ea05546f1b340eab28b9e30c99`.
   Pyz `913e0842bfae1ec72e80c118612b8aed73575625ea13b7bd71d4bf073dd87a29`.
   Fresh external build, explicit non-atomic0.3→0.4 remove/install, three old tools,
   invalid/stale-plan refusal, retained state and five removed-tool refusals pass.
   Positive policy graphs are synthetic; actual13-path report retains22 unresolved
   findings and zero mapped selected changes. Oversized complete plans are explicitly
   omitted; direct-manager reconstructed frames do not prove native model-budget
   nontruncation. No persistent profiles/offline selection restore/update authority.
   [Evidence](verification/2026-10-05/p18u-selection-tools/README.md).

0.4 roots87363/37038 completed0; strict sole children45435/46637 reaped0, no runner
error/new OOM, exact source unchanged. A root helper seal-check initially used the
wrong collection type before launch; complete seals verified afterward and correction
preserved. No product testcase replay or result relabeling. Full private runtime
export R/p18u-selection-tools-real-runtime-export-20261005-01/private-runtime.tar.gz,
SHA `889e743fa68e54fdb5e770d598c26525bb36fb529734f8e8939f7d09a465303a`,
956,383 compressed/4,618,748 member bytes, full readback/source rehash; VM-local only.

### New pure final-scope stage

Adopted two new paths: `component-sdk/examples/upstream-maintenance/final_scope.py`
and `component-sdk/tests/test_final_scope.py`;425 added lines, no installed surface.
Exact8977-path map
`f27ab9e949b997ff910baf0c891465e3847879a94b0b29b498b04c01246ed4b2`.
Root6167 passed50 cases:18 new whole-inventory scope fixtures,19 selection and13
adapter regressions. Source wrapper/strict0, no runner error/new OOM, exact source
unchanged. Formatter75375 completed0 before tests with no source change. Adoption:
R/p18u-final-scope-adoption-20261005-01; full gzip/readback receipts:
R/p18u-selection-final-scope-focused-20261005-01.

It derives every supplied before/after difference, requires all owners' operation
permissions, binds whole baseline/plan/output identity, rejects excluded/unmapped
changes and requires add+mode for new executable files. Rename needs both endpoints;
case/file-directory transitions and selected no-ops conservatively block. It still
trusts supplied completeness/hashes/classification; **no filesystem attestation**,
installed audit operation, browser walkthrough or update activation is claimed.
[Evidence](verification/2026-10-05/p18u-final-scope/README.md) is the publication target.
Old larger proposal R/p18u-selection-scope-proposal-01 stays preserved/unadopted.

A separate actual-Git **synthetic calibration** passed root21653 on earlier d84
source: a new worker starts Git, the tracer detaches all tracees before primary
SIGCONT, and the worker normally reaps Git with exact output. No rescue/signals,
strict0/source unchanged. R/p18u-overlay-owned-trace-admission-20261005-01 retains
all tiny runtime files and binary bindings; result SHA
`d886acfad59b4cbabf282ea8234da7c40c57f5c7c35861bb5cd1ce3442c23e1d`.
This proves an exec-boundary test mechanism, not installed-plugin cancellation or
that merging had begun. The R-only actual installed-overlay lifecycle proposal is
sealed and remains unexecuted at this checkpoint. The unchanged strict
runner has no command timeout; root supplied timeout15s TERM/+5s KILL inside it.
Every deadline/rescue/nonzero remains failure. No test command remains active.

## Capacity and required product scope

Historical pre-repair observations at01:58:06 UTC were287,522,816 persistent bytes
and a closed native gate. The earlier16-section pilot left879 unprobed then; the
later full probe authenticated all895, and both repairs above supersede those
missing-backing and free-space descriptions. They do not supersede old failed tests.
The8.17GB historical archive inventory remains protected; no recovery archive was
retired. No supported same-instance resize or authenticated cloud-to-home export
is established. The single home-service check failed DNS and proxy CONNECT403;
do not repeat unchanged access or expose Proxmox management publicly.

Drive/PC/home storage remain optional private cold-archive candidates. No upload,
provider purchase, account creation or archive retirement is implied. Remote
integrity, demonstrated restoration and explicit retention decisions precede any
protected artifact retirement. The measured same-VM repairs permit the specified
bounded resumption order; they do not reopen a broad small-feature queue.

Required P18U/P19 behavior is documented in [walkthrough](UPDATE_WALKTHROUGH.md)
and [live generations](LIVE_UPDATE_GENERATIONS.md): user-initiated pinned updates,
visible diffs/provenance/conflicts, dependency-valid selections and remembered
exclusions, per-plugin implementation/config/contracts/dependencies/schema scopes,
full verified consistent recovery and later rollback preserving newer writes.
Immutable A/B behind a stable router: A stays usable while isolated B builds/tests;
health B before atomic new-session routing; A sessions remain pinned/drain. Use
consistent snapshots/catch-up/fenced ownership, never stale candidate-state promotion.
Insufficient capacity or incompatible coexistence leaves B pending and A usable.
Retain previous usable release locally and independent recovery. These live/update
operations remain requirements, not implemented behavior. No polling/unattended
live activation/hot replacement within running sessions.

## Ordered next actions

1. Read both completed repair seals, source/index bindings and exact phased budget;
   reconcile current processes and fresh disk/tmpfs/memory counters. Preserve all
   archives/aliases and unfinished proposals. Never replay completed cache mutations.
2. Finish only the paused installed-overlay cancellation acceptance: separately built
   installed package with the unchanged manager, cooperative SDK shutdown, abrupt
   plugin death, tracked Git cleanup before rescue, incomplete-job outcomes and
   subsequent successful manager invocation after each primary. Use fresh no-clobber
   evidence and exact package/source identities; preserve failures and checkpoint.
3. Return immediately to the coherent eight-path native auth-install cohort. Recheck
   all source preimages and whole-sequence capacity before adoption. Run required
   formatting, grouped focused/full login and provider `just test`, scoped login
   `just fix`, then production CLI/manager rebuild with retained flags/toolchain.
   Bind actual new-source/binary results; older source42 passes are not new proof.
4. Reexercise installed storage/migration/GUI/attachments, normal/forced Launch,
   cancellation/recovery and the held ownership matrix against that new host. Include
   the new auth-install real consumer gate; do not substitute native unit fixtures.
   In-app Browser remains unavailable; identify actual Chromium/deterministic-model
   fallback explicitly. Native capacity success requires this source-bound sequence.
5. Separately budget remaining TUI6/two snapshots, App Server16, same-process
   replacement and lower-transport/MCP/durability debt. The auth-specific9-root
   capacity plan does not admit those larger or unresolved builds. Collector and
   broad P18U/P19 feature work stay paused until the critical native priority is met.

The user's home Proxmox (>40TB reported) is another cold-storage destination to
assess with the coordinator. No provisioning or checkout relocation is authorized;
prove a private authenticated cloud-to-home route and exact restore before uploads
or retirement. Never expose Proxmox management publicly.

Use `just test` for Rust and scoped `just fix`; never direct `cargo test` or kill Rust
commands. Run `just fmt` after code. Full workspace suite still needs separate user
approval. Current native blockage does not authorize deleting archives or weakening
tests. Distinguish fixture/build/runtime/browser evidence. Commit/push verified
milestones nonforce, preserve unfinished work separately, revalidate refs immediately
before publication and read back exact files afterward. No perpetual execution promise.
