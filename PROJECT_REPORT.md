# Codex Harness Compartmentalized — development report

Report date: 2026-10-05. **This is a working partial platform, not a completed
“everything is a plugin” harness.** The canonical plan is
[IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md); the precise resume queue,
source identities and blockers are in [EXECUTION_STATE.md](EXECUTION_STATE.md).

## What has been built

Official OpenAI Codex was imported at
`d42056091aded7feb1d88ac7e83972108b2aa478`, retaining Apache LICENSE/NOTICE and
provenance. The project uses Codex's actual Rust engine. It does not substitute
another harness or count a wrapper around an unchanged engine as extraction.

The component API, host, adapters, state/path codecs, package manager and Python
developer SDK provide an initial versioned external-process plugin system. External
packages can be built separately, installed and selected without rebuilding the
host. Discovery, manifest/API compatibility, configuration, process lifecycle and
removal have working paths. Shared service declarations, limits, grants and
negotiation contracts have additional tested support, but the complete dependency
broker is not yet active. The current remove/install upgrade path is non-atomic;
it is not the required future live release switch.

Three **bounded families of pre-existing native functionality** have real external
implementation/replacement evidence:

| Native family | Implemented scope | Remaining boundary |
|---|---|---|
| Thread persistence | Selected native thread-store operations; separately packaged local worker; manual rollout migration selection | Auxiliary database domains, remaining maintenance and full lifecycle matrix |
| Inline attachment storage | Original inline byte-preserving upload and `NotFound` resolution | Durable remote attachments/uploads and all production caller paths |
| File search | Native traversal/matching worker; selected CLI, App Server and TUI/GUI consumers | Other filesystem/Git/watch/import operations and remaining startup/retirement cases |

Exact paths, symbols and evidence appear in
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md). New infrastructure, adapters,
example tools, compiled auth/HTTP ownership work and use of existing contributor
interfaces are **not additional native extractions**.

A separately packaged desktop-style web GUI works through real Codex App Server
interfaces: sessions/history, messages and streaming, tool progress, approvals,
Stop/cancellation, search and cold recovery. The CLI/headless path remains usable
without it. This is our new UI, not the official desktop application's source.
Native App Server/TUI extraction and an engine-independent maintenance screen remain
unfinished. The current GUI closes with its App Server.

## Work on reliability and update maintenance

Significant prerequisite work addresses native authentication/policy epochs,
credential publication and refresh ownership, HTTP and Git cancellation, retained
workers/callbacks, cleanup deadlines and shutdown uncertainty. These changes reduce
risks in later replacement boundaries but do not make native auth/providers
independently installable yet. Saved auth/MCP/catalog/context proposals remain
preserved and are not silently counted as accepted implementations.

The additive maintenance component now has three installed operations:

1. Review a pinned upstream change against component/custom-source provenance.
2. Preserve bounded exact base/upstream/custom source inputs in an integrity-checked
   capsule, retaining incomplete jobs after interruption.
3. Reconcile a sparse source overlay, retaining custom edits and refusing unresolved
   conflicts or unsupported changes. Inspect retained artifacts without the host.

Published implementation checkpoint: [`bec32f7`](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/commit/bec32f7295411c2b048e4d2fa8286547994471ae)
on the existing WIP branch; all 16 selected files were read back exactly.

The latest 0.3 milestone passed **47 focused tests and 61 real installed-package
commands** using the unchanged production manager. It processed 13 actual changed
paths from later official revision `2e5fea64eefcaa19f48458b2386011b619f69c70`, based
on committed custom composition `914cc593`. Independent byte comparisons confirmed
all outputs, including our import removal alongside the upstream changes. External
build, non-atomic 0.2→0.3 replacement, old-tool compatibility, refusal cases, removal
and host-independent inspection passed. See
[the exact evidence](verification/2026-10-04/p18u-candidate-overlay/README.md).

This has **not** built or activated a complete later-upstream harness. All release
gates remain pending. Source merging and artifact inspection do not establish
behavioral compatibility, complete backups or update rollback. Active-merge
cancellation is also a distinct pending gate.

The next pure selection-plan implementation also passes **19 focused fixtures**.
It binds selections/exclusions, graph constraints and per-owner scope permissions to
immutable evidence identities. Installed exposure, actual final-diff enforcement and
persistent UI profiles remain separate gates; it adds no native extraction.

## What has actually been tested

Evidence is bound to each tested source map, package and binary, rather than one
ever-growing test total. Earlier failures and unverified stages are retained.

- The 0.1.1/0.2 maintenance checks exercised real blocked Git cancellation/abrupt
  termination, tracked cleanup and a subsequent successful manager invocation.
- The fresh 0.2 regression checkpoint ran installed storage, manual migration and
  two real Chromium GUI cycles on preserved production binaries. It covered
  streaming, tools/approvals, Stop, search, cold history and actual manager Launch
  Ctrl+C with tracked process cleanup.
- Those browser tests used real Chromium/Playwright with deterministic model
  responses. They were neither the requested in-app Browser nor live-model proof.
  No new 0.3 GUI/storage/migration rerun is claimed.
- Native tests and runtime receipts from prior milestones remain separately
  documented. Current Rust build capacity is insufficient, so newer unbuilt auth
  proposals are not credited with older results. No complete workspace suite is
  claimed; that suite still requires separate approval.

## What remains and the roadmap

The inventory covers 28 functional/support boundaries, C00–C27. Most native engine
responsibilities remain coupled. The dependency order is:

1. Close P03 lifecycle, transport, dependency-broker and replacement gates; finish
   the pending native token-install ownership prerequisite once capacity permits.
2. Extract configuration/auth, model catalog/providers/inference, remaining durable
   state and attachment implementations (P04–P06).
3. Extract reconstruction, live context/history/prompt building and compaction
   (P07–P09).
4. Extract approval/policy, execution/sandbox, native tools and MCP/connectors/skills
   services (P10–P13), preserving authoritative security enforcement.
5. Extract session/turn/multi-agent orchestration, background services, events,
   telemetry and remaining presentation/remote functionality (P14–P17).
6. Complete SDK/install/release compatibility and the maintenance platform (P18/P18U).
   The required [update walkthrough](UPDATE_WALKTHROUGH.md) includes selections,
   per-plugin scope enforcement, full verified recovery, live immutable A/B release
   generations, old-session continuity and later rollback preserving newer data.
7. Complete P19 clean-install acceptance with independently built native/custom
   plugins, real UI/headless operation, a real upstream update and failure/rollback
   scenarios. Audit every remaining kernel responsibility explicitly.

Each canonical phase defines prerequisites, deliverables, acceptance, regressions
and exit criteria. A small kernel may supervise processes, enforce permission
ceilings and capabilities, route to immutable releases and support independent
recovery; domain functionality cannot quietly remain built in under that label.
Hot replacement within running sessions, periodic polling and unattended live
deployment remain outside this scope.

## Capacity, preservation and immediate work

The latest bounded capacity audit found about **306 MB persistent free space**,
9.19 GB temporary RAM-backed space and a 16 GiB memory limit. It inventoried **8.17 GB
of recovery archives**, including a conditional historical shortlist of 700 MB.
**Zero bytes are certified safe to delete.** Native builds remain blocked by both
disk headroom and missing volatile backing for cached inputs; free tmpfs alone does
not establish a complete build/lint/link/runtime budget.

The quantified proposal is [CAPACITY_PLAN.md](CAPACITY_PLAN.md).
Small bounded Python maintenance work can continue under explicit resource floors.
Offsite storage, including Google Drive, is being evaluated for cold artifacts only.
Quota/access, confidentiality, full checksums, independent remote readback, tested
restoration and explicit retention decisions are prerequisites. No upload or local
archive deletion has been authorized by that evaluation. No supported expansion
operation for the running environment has been verified.

Recovery archives and unfinished work are preserved. Workspace-only archives are
recovery checkpoints, not proven offsite backups. Verified source milestones are
published to the existing WIP branch; `main` and the original local index are
preserved. Remote-desktop networking is separate and does not block feasible code
work. There is no verified locally reachable live viewer URL to report.

Next feasible implementation: selection/dependency and final-diff scope enforcement,
then durable maintenance ownership and UI lifetime separation. Full candidate
assembly, A/B routing, consistent recovery and later rollback remain explicit
subsequent gates, alongside the blocked native extraction queue.
