# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Partial platform; P03 remains incomplete.**
Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md), root AGENTS.md and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Detailed earlier runs, failures and completed resource actions are preserved in
[EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

## Resume immediately

1. Production build completed0 with unchanged current source. Source receipt:
   A/p03-featured-warmup-current-production-build-01.source.json SHA256
   b3ca445c160e51cda07fd39564342a86a59592a784235a2289bc3827e00fe5a4.
   Actual CLI bbed687d4c1a59f64c8b071ad0e5a328520d3db18d5e3dc37c8f064d196c9853;
   manager stays f054d84a…. Exact .binaries.json SHA256
   ee94d8dee01f3abd5c49fb094cc65cd1a53d62ace8a70017308c5bb31f230bc5.
2. Root verified actual8936/f05b source and stable binaries, and completed the
   runplan POSTBUILD_BINDING.json SHA256
   00e8a65ebea5c7f0ae6831d1c3c6cc9a3ecdb227b814951c7f2fd48a7bf72152.
   Installed-storage gate passed13 checks on the new CLI; package reused from its
   original independent build, no new Rust plugin build. Prefix
   A/p03-featured-warmup-newhost-storage-01; strict0/null, adopted -9:6/0:1.
   All four independent migration gates now passed0. Ordinary
   and selected-search GUI each passed two cold cycles with clean Launch Ctrl+C.
   Approval/Stop were exercised on cycle1; deterministic inference, real Chromium.
3. Planned current-production runtime gates are complete: storage, four migrations,
   ordinary/selected-search GUI, both slow Launch cases and all8 held cases. Normal
   Launch46.40997016s exited0 with cold recovery; forced2.055999819s exited1 with
   explicit durability uncertainty and expected absence of the unforwarded event.
   Both tracked process sets were absent; each subsequent cold normal shutdown
   exited0. Fixture admission is before native forwarding, not internal store admission. Use the
   runplan's exact fixture pins, fresh homes and before/after source wrapper.
   The additive featured-owner collector passed58 pure checks after a preserved
   layout-only correction and all8 live receipt gates. All89 original assertions,
   deadlines and the unchanged strict runner were retained. Do not splice partial
   failed matrices into a pass; scope/process receipt counts are not lifetime totals.
4. Complete affected CLI/exec/TUI scoped tests and lint as headroom allows. The
   read-only scope audit R/p03-executable-consumer-coverage-audit-01 lists exact
   just test commands. A TUI test-name filter still builds its full large test ELF.
   No current consumer test pass is claimed yet; production CLI does not compile
   standalone TUI main. Preserve proof binaries before compiler path reuse.
5. Publish the accurately bounded verified milestone on the existing WIP branch.
   Main promotion awaits the applicable acceptance gates. Continue P03 and later
   dependency-ordered extraction; do not stop at this ledger or a plan.

## Environment, ownership and preservation

- Original checkout: /workspace/codex-harness-everythings-a-plugin.
  Preserve sibling /workspace/codex-harness-next-components. No new checkout/VM/chat.
- Origin: https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git.
  Actual commands execute in the recovered original cloud VM; stale access blocker
  is cleared. Remote viewer forwarding remains separate and unavailable.
- R = /workspace/recovery-backups/20260930T165936Z; A = /workspace/acceptance.
  Root owns source/Git/compiler/cache mutations. Workers stage proposals in R only.
- Use /workspace/toolchains/component-verification-env.sh; one compiler job.
  Build/runtime TMPDIR=/tmp/p03-featured-warmup-current-production-build-01.
- Local HEAD/index remain upstream, and local main is stale. Do not reset or rewrite
  the real index. Index SHA256:
  0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59.
- Preserve the older live GUI in /workspace/verified-component-checkpoint-v2-20260930
  and /workspace/remote-viewer-setup. They are not current-source runtime proof.

Full earlier two-worktree/Git/SDK/evidence backup:
R/codex-recovered-workspace.tar.zst, SHA256
3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd.
Fresh primary pre-format source/Git snapshot:
R/p03-replacement-current-source-before-format-01.tar.gz, SHA256
e6ab98fe2bd9625c10fe67979eaf308b4f7e4ee7892548482ff0750d9847df5e.
These and all proof archives are **VM-local**, not proven external backups. GitHub
provides external durability only for published paths; preserve all isolated proposals.

The CLI65 proof archive095169ae was stream-verified and fully restored before
only its two inactive original output links were retired. Only the new restoration
scratch was then released under separate guards; older restorations,230 archives,
16 aliases, both worktrees and runtime homes remain intact. Action receipts:
R/p03-stage-c-cli65-proof-preservation-01/RETIREMENT.json SHA256
4eae26d6f99f824c6ef386beec4553e7c76449543abf6dee11b99a56b941ca61;
RELEASE_RESTORE.json SHA256
8ecba951bca6d561aad91352a3c5d23ede7b8f00cd991ba14776c6312631f34c.
Never rerun completed retirement actions after compiler output reuse.
B553/C432 proofs were then archived together with their distinct source/strict/log/
binary receipts, all10 members stream-verified, and both ELFs fully restored before
only the two inactive original targets were retired. Only the new shm scratch was
released afterward. All older restorations,230 archives,32 aliases/eight homes and
both worktrees were preserved. Archive R/p03-current-b553-c432-testproofs-preserved-02.tar.zst
SHA25655bf20f97a73fcacf623c5825b1cd1c0bf794e240c7e7c98e95929d2fa574634.
Receipt directory R/p03-current-b553-c432-proof-preservation-02; release SHA256
780fc79663e7226b3f1818cbe5ce2605d0fdcbb64ef69252f13c91bb695e3937.
The first resource audit rejected newly found aliases before mutation; it remains
preserved. These exact test proofs now require restoration from the verified archive.
After release:985923584B overlay/478621696B tmp free; remeasure before each large task.

## Published source and current proof

Upstream: openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478.
Exact-tree import ae720ae9a98bad29ca2cff998e7d5baaf05cec86; retain LICENSE/NOTICE.
Latest WIP: **2ce48a627876b3a9a53e388f356df83e5ccff254**, tree
8d201a7149663256c9ed8364bc85178a874f8aea, parent706bf9e, branch
wip/p03-process-final-and-mcp-preservation-20261002. All36 selected remote blobs
and both refs verified after nonforce update. Main is unchanged at
781080f7e3c8bfe1953378001d777dff33d74bc3.
R/p03-featured-warmup-bc-publication-01/PUBLICATION.json SHA256
6c7ad1c4de4ea3e03d2362052ca1594157c55cbd54a811d24d3abee9ab05771e.

Current C candidate R/p03-featured-warmup-stage-c-source-01.json SHA256
ffcc9711a3418d30c429e2b8bd14cc10a89dfc83a95119448ba936619322b99c;
8936-file Rust/SDK/build scope map
f05b8b9f19a7d8a9a78691970549c30fc0660b5c64769c03d94dc5292c523328.
Root docs/evidence edits do not alter this scope. Source wrappers record actual
maps; candidate labels alone are not proof. Test ELF252dfaf2… is not a production CLI.

| Gate | Actual evidence |
| --- | --- |
| Native featured owner/library | Stage A552 passed; published706bf9e. |
| Native startup/processor/process-final wiring | Stage B553 passed, clean scoped lint; original compile101 failure preserved separately. |
| Current App Server library |432 passed,2 parent-owned helpers skipped; separate new caller and curated pending/replay/race all passed against actual252dfaf2 ELF. |
| Caller lifecycle | Real A public Drop → held peer EOF before observer/global close; joined cancellation; B same-home public shutdown. A runtime-result join/cache/whole-host claims remain false. |
| Current App Server lint/format | Both0, source unchanged, no lint warnings. No tests rerun solely for lint/style. |
| Current production CLI/GUI | Build0/unchanged; storage13 checks, four migrations, both GUI modes, normal/forced slow Launch and all8 held cases plus task-receipt postcheck passed. |

[Current production evidence](verification/2026-10-02/p03-featured-warmup-current-production/README.md),
[Stage B evidence](verification/2026-10-02/p03-featured-warmup-stage-b/README.md),
[Stage C evidence](verification/2026-10-02/p03-featured-warmup-stage-c/README.md),
and their upstream lineage maps keep each source boundary separate. Strict adopted
statuses are not all zero in library suites; exact histograms and failures remain.

Older final65 production proof remains valid only for its own map1c54717d/CLI dc7f1e84:
installed storage, four migrations, ordinary/selected-search Chromium GUI, slow normal
Launch46.317s with recovery, forced Launch2.042s with explicit durability uncertainty,
and a fresh all8 held matrix passed. Original auxiliary-admission race and earlier
failed matrices remain open/preserved. [Prior evidence](verification/2026-10-02/P03_CURRENT_PRODUCTION_RUNTIME_EVIDENCE.md)
must not be relabeled as current B/C proof. Independent package-build identities
are reused, not claimed as fresh Rust builds.

## Coverage and remaining implementation

Native replacement coverage is still **three bounded families**: thread storage/manual
migration, inline attachment subset and native file search. The separately packaged
GUI is additive presentation. API/host/SDK, ownership helpers, fixtures and maintenance
planning do not count as further native extraction. Most engine families remain coupled.

After the current regression checkpoint, close P03's named native acquisition/lifetime
owners and broker gates. The R-only HTTP-construction audit identifies retained builder
custody/cohort lifetime and a closed-publication fence; preserve late cache publication
while a pool stays open. Closing one featured task must not close shared HTTP consumers.
Universal in-process reqwest join is not an exposed contract or a prerequisite to
invent; retained constructor custody and truthful transport uncertainty remain required.
MCP upper owner → retirement → lower transport → pinned SDK → late-admission proposals
remain isolated and unadopted; preserve every version. Curated archive/publication/host-
death fences, general Session acquisition, model-v2, activated broker capabilities and
real failure/installation matrices remain P03 exit obligations. Do not weaken deadlines,
security boundaries or positive cleanup assertions to complete them.

Then follow P04–P19 for configuration/auth, models/providers, remaining state/replay,
context/compaction, approvals/sandbox/execution/tools, MCP/skills, orchestration,
background work, events/telemetry, clients and release packaging. Kernel exceptions
remain explicit in the roadmap. P18U is required: the offline planner passed17+22
fixture checks but integrated no later upstream revision. Installed maintenance plus
external recovery, a real revision integration preserving custom plugins/UI, coordinated
versions/migration, incompatible-update rejection and rollback remain mandatory.

## Testing and execution limits

Use just test, never direct cargo test; scoped packages and just fix/just fmt.
A complete workspace suite still requires separate user approval; none is recorded.
Preserve original failed runs, exact source/binary pins and unchanged strict assertions.
In-app Browser and Context7 are unavailable here. Real Chromium/Playwright fallback
uses deterministic inference; do not claim in-app manual or live-provider validation.
No credential/config/environment dumps. No force push, scheduled upstream polling,
unattended live deployment or hot swapping. Remote viewer blocking must not stop
independent component work. Continue feasible work; no unlimited-run guarantee.

Collector pure run01 failed57/58 because its historical test expected a missing
sibling preimage directory. Original failure/flat proposal01 are preserved. Proposal02
restored that exact preimage without changing the eight Python files or58 methods;
pure02 passed58/58. Live held8 and its featured postcheck passed separately. The
original auxiliary-admission race remains unexercised, MCP/whole-host cleanup and
HTTP-internal custody remain unproven. See R/p03-featured-owner-postchecker-proposal-02
and A/p03-featured-warmup-held-all-mode-ack-01 / held-featured-postcheck-01.
Fresh attachment-content runtime acceptance has not yet run on bbed: the storage
and migration fixtures do not send image bytes. Earlier attachment evidence retains
its original source identity.
