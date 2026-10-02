# Historical checkpoint ledger

Frozen on 2026-10-02 while the process-final regression gate was in progress.
Use [EXECUTION_STATE.md](EXECUTION_STATE.md) for the current queue and active status.
Earlier open-item statements apply only to their named source revision.

# Codex Harness Compartmentalized — execution state

Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required update track: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
**Full v1, installed auth/catalog and the upstream updater remain incomplete.**

## Current work in progress — 2026-10-02

The last published candidate is `ba87799c2e9ab821a326f10715c11cf6ec256896`, tree
`c75dfdae88843dcbcbdbebe8d0e2ed4dfae6e9c7`, on `work/p03-curated-sync-lifecycle`.
Main remains `781080f7e3c8bfe1953378001d777dff33d74bc3`. The historical sections below
record earlier checkpoints; their open-item statements describe those sources.

New process-final and replacement-delivery changes are adopted in the owning checkout,
**unpublished and not yet accepted as a full-host checkpoint**. The exact 56-path candidate
manifest is recovery `p03-process-final-native-retry02-source.json`, SHA256
`0a46a9a3f50670a4cc10180012dd3adb09ee2226c75efb8db8c86fbb093d4065`, virtual tree
`1da290682fc6ddd00994c2af7eac9018c8143dea`. All 8,920 scoped entries have tested map
`31f82854bdfeffe7ec99a05ef46a9a2eaba2cacd1b2587a568e8f2be81836a73`.
The source includes executable-owned process-final authority, a shared 200-second grace /
205-second forced-exit-initiation deadline, actual CLI/exec/TUI/App Server callers, irreversible
curated admission fencing, retained callback/native observations and same-home replacement
delivery. GUI budgets are coordinated at 207/209 seconds with the existing 210-second manager
budget. Forced termination remains an uncertain durability outcome. These are native lifecycle
prerequisites, **not additional independently installed components**.

Actual checks against this candidate so far:

- Native tests: **703/703** (536 core-plugins, 157 App Server transport, 7 arg0, 3 process
  utilities), no skips/retries; unchanged strict runner exit0/null error.
- App Server library: **431/431**, no skips/retries; unchanged strict runner exit0/null error.
  Its two slow-store tests use paused virtual time, not elapsed native-storage proof.
- Python desktop transport fixtures: **14/14** against the pre-compiler-repair source;
  strict0/null. This is not real engine/browser integration.
- Client attempt01: **no tests executed**. Linking failed with signal7/Bus error at zero free
  disk, exit101; strict101/null. Its before/after source maps match the two green native runs.
  The earlier native compiler syntax failure is also retained separately.
- Client unchanged-source retry02: **42/42**, no skips/retries; strict0/null in17.642870592
  seconds. Seven adopted SIGPIPE statuses lack executable attribution; this is not a claim
  that every descendant exited successfully.

Evidence is under `/workspace/acceptance/p03-process-final-*`; source/proposal archives,
exact evidence bindings and phased proof-ELF retirement receipts are in recovery. Completed
703/431 test executables were hash-verified in archives before retiring their inactive build
paths. The failed client linker output is also fully archived, receipt
`p03-client01-failed-link-retirement.json`. These are VM-local recovery checkpoints, not
external backups. No broad cargo clean, source deletion, index rewrite or running GUI change
occurred. Re-read current resource state before the next build; old cache audits are stale.

**Immediate next action:** preserve the client42 proof, prepare enough build headroom using
a fresh idle cache/reference audit, and verify changed exec/CLI/TUI/component-host and
App Server integration paths. The guarded172-file obsolete-library retirement completed and
the unchanged-source client retry passed. Then
run scoped lint/global formatting, rebuild the actual CLI and manager, and execute held native
Git/HTTP plus installed storage/migration/GUI gates. Reuse source922 proof only as history.

MCP upper session/task-custody and lower transport/process-custody proposals are frozen in
recovery but **unadopted and uncompiled**. Pinned RMCP HTTP completion is being staged with
upstream license/file provenance. No claim of complete descendant cleanup follows from the
current curated-only receipt. The installed slow-store fixture is also staged, not executed:
it holds a request at the installed relay before native admission; its 46-second real Launch
cleanup and forced-uncertainty checks remain future acceptance. Preserve all proposal versions.
Optional remote-viewer transport is unchanged and does not block component work. In-app Browser
and Context7 tools remain unavailable; real UI regression will use the recorded Chromium /
Playwright fallback with deterministic inference, labeled as such.

## Resume identity and publication

Original cloud checkout: `/workspace/codex-harness-everythings-a-plugin`;
preserve `/workspace/codex-harness-next-components` and both worktrees' changes.
Origin: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
Upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`; exact-tree import
`ae720ae9a98bad29ca2cff998e7d5baaf05cec86`. Retain LICENSE/NOTICE.

Last verified main: `781080f7e3c8bfe1953378001d777dff33d74bc3`, tree
`22cf918cf77f16d5d947a59b968e342cde7f71d0`. Main source remains native installer
`65511842d7051b2a1f5cc52917f3ebb5c03be4f3`; later main changes are documentation.
The preceding verified candidate publication is `6856eb25b2570ecebe036d07a3a515b708893b9d`,
tree `7ab60abde432f41b9c602413a6d1534da33910b7`; receipt
`p03-worker-completion-publication.json` verifies its 11 changed blobs and both refs.

**Active development source**, present in the owning checkout:
`0c1ed130daf3dc36768e35109c576336d9f66005`, tree
`f2b6c900d8225325affc0252d6f6680f500e0e5a`. It adds actual curated callback routing and local
lifetime guards (13 paths) on scope/export/tests commit `8d3beeff520177c3dee7562d168253ed7bdeafda`,
tree `75dbe7618f88f6a36ef9784ab5592c496c28331d` (3 paths), parented on `6856eb25`.
Only the combined source was scoped-tested: 953 library tests passed; failed-link, lint and
format transitions are recorded below. No binary-wide final authority, global registry fence,
MCP descendant-join or replacement-delivery acceptance is claimed. These native prerequisites
add no independently installed component. Destination remains `work/p03-curated-sync-lifecycle`,
**not main**. Before continuing, verify `p03-curated-callback-publication.json` and remote branch;
this document cannot contain its own eventual evidence commit SHA. The source readback receipt
verifies 16 blobs but did not move the branch; until the publication receipt is verified,
`6856eb25` is the last verified branch ref.

The latest full CLI/runtime proof remains **source922**, commit
`92212516ad4d12bcf60546ee5f879b983ed44682`, tree
`746d93dcdc89de6882dba313d4ad3626d33b0ad0`; publication `2ac44529d0dfa261c6d2a09005a34b3ee22754ed`,
tree `a25f9c7a740124967f2c7dbf93627d2baf6d370a`. Do not relabel that proof for source27d or
source0c1. Its inactive CLI is now recoverably retired; restore and reverify its archive in an
isolated location or rebuild the current source. C2b remains staged separately and unadopted;
the original extractor is unchanged.

Local HEAD/index remain upstream; local main is stale. Use exact reviewed temporary
indices and connector publication; do not reset/rebase/rewrite the real index.
Original index SHA256: `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
Root is the sole checkout/Rust/cache/Git mutation owner; workers stage proposals in recovery.

## Implemented and tested scope

Installed native replacement evidence covers recorded thread storage/manual migration and
bounded search across CLI/App Server/TUI/GUI. Inline attachment storage has manager-level
subset proof. The separately installed GUI is an additive client. These counts do not grow
because the following compiled prerequisites exist; see the inventory for coupled subsystems.

[Installer source655 evidence](verification/2026-10-01/P03_INSTALL_ORDER_EVIDENCE.md):
latest admitted installs fence older work across clear, credential and policy changes.
Baseline9:3pass/6assertionfail; full scoped400pass (292login+108provider), clean lint,
two reviewed format-only changes. No installed auth/catalog or conditional durable-write proof.
Source-aware load/refresh publication, retained provider work and broker activation remain open.

[Curated transport candidate evidence](verification/2026-10-01/P03_CURATED_GIT_TRANSPORT_EVIDENCE.md)
binds all8,892 scoped final entries to source7ab5. Initial link attempt101 ran no tests and
coincided with zero free disk; preserve it separately. Identical-source retry ran6:3existing
policy passes/3new lifecycle assertion failures. Original green541 passed. Lint then found
panic accessors; the explicit ownership repair passed a fresh **541/541** (476core-plugins,
65PTY), no skips/retries, strict0/null. Final lint is clean; global formatting changed only
layout/import splitting in3files. No tests repeated solely for final formatting. Raw52
adopted wait records include nonzero statuses with no executable/cause attribution.
Bazel lock refresh passed; its actual lockfile was unchanged.

Candidate behavior: Linux private process group, fair bounded pipe draining (1MiB/pipe),
retained direct-child ownership before group signal/reap, bounded cleanup, explicit uncertain
cleanup errors. Reuses inherited upstream PTY reaper/group primitives. HEAD lookup now uses
the timeout helper on all platforms; non-Linux retains the older transport helper and is untested.
No component ABI/schema changes and no new independently installed subsystem are proved here.

**Do not promote this candidate yet.** Private groups can outlive abrupt host death and lie
outside the parent's process-group guard. At source7ab5 the native pipeline still used String failures,
could fall back/drop locks or staging on uncertain cleanup, swallowed HEAD errors and reset
process-global admission on errors. No host cancellation/owned-worker/full-CLI/GUI result for
source7ab5 exists. The [required ownership review](verification/2026-10-01/P03_CURATED_B1_OWNERSHIP_REVIEW.md)
changes the next stage ordering; enqueueing a child to the shared reaper is not a completion receipt.

[Locked-attempt source9a2 evidence](verification/2026-10-01/P03_CURATED_LOCKED_ATTEMPT_EVIDENCE.md):
private `LockedSyncAttempt` binds the actual home/File and preserves original transport
body, tracing, fallback and drop ordering. The existing real HTTP-error regression now
also checks stable-lock release. **476/476** passed, no skips/retries; strict0/null;
scoped lint clean and source unchanged. Global formatting only wrapped two calls.
This is mechanical preparation, not retained cleanup or another installed component.
That historical checkpoint still permitted unsafe release/fallback on unknown cleanup; source739 supersedes this boundary.

[Retained-owner source739 evidence](verification/2026-10-01/P03_CURATED_RETAINED_OWNER_EVIDENCE.md):
typed Git/HEAD failures now reach the real pipeline and manager. Primary custody is registered
before spawn; uncertainty keeps the canonical-home lock, staging/backup directories, command
slots and worker generation reachable even after error drop, swallowed result or unwind.
A bounded stop attempt does not clear quarantine. Ordinary confirmed failures preserve fallback;
publication uncertainty stops fallback. Exact worker handles survive fast completion, and ordinary
retry requires a finished joined generation. No production quarantine reset or restart recovery exists.
Two compile101 attempts ran no tests (extra pipe references, then an old test's removed flag).
After repair **557/557** passed (488core-plugins+69PTY), no skips/retries, strict0/null;
8,896 scoped files unchanged. Scoped lint made four equivalent if collapses in three files;
formatting changed layout/imports in ten files. No tests repeated solely for lint/format.
Raw54 adopted statuses have no executable/cause attribution. This adds no installed subsystem.
Non-Linux retains legacy unverified transport. Blocking HTTP, cancellation, transactional repo+SHA
publication, queued callbacks, durable fencing across host death and full-host gates remain open.

[Shared-control sourcecb9 evidence](verification/2026-10-01/P03_CURATED_SHARED_CONTROL_EVIDENCE.md):
one exact control is reserved before worker spawn; stop closes admission even before first
start and deadlines only shorten. The control reaches real File-lock waiting and every
Linux Git operation. Typed stop blocks fallback/new stages and callback enqueue admission;
an already admitted activation/SHA pair completes before stop is reported. Existing cleanup
uncertainty remains sticky. **572/572** passed on the first compile/run (501core-plugins+71PTY),
no skips/retries, strict0/null; 8,898 scoped files unchanged. Scoped lint exited0 without edits
or warnings; global formatting changed ten paths, with exact before/after maps preserved.
Raw54 adopted statuses have no executable/cause attribution. No test rerun solely for formatting.
This is no new installed subsystem. Current HTTP operations, queued callbacks, transactional
publication and actual host-shutdown integration remain pending. A stop request is not a join
receipt, and process-local ownership cannot prove safe recovery after host death.

[HTTP-await source922 evidence](verification/2026-10-01/P03_CURATED_HTTP_AWAIT_EVIDENCE.md):
all real metadata/archive helpers consume bounded policy-aware chunks, with a30s absolute
request budget, metadata1MiB/archive64MiB caps and8KiB diagnostic prefix. Limits are deliberate
compatibility constraints, not measured upstream maxima. Metadata retains original charset/BOM
decoding after bounded collection. First run **580/580** passed (509core-plugins+71PTY),
no skips/retries, strict0/null; 8,900 scoped files unchanged. Lint made one equivalent test-fixture
if collapse. Exact argument-comment-only edits and global formatting are recorded separately;
no tests repeated solely for lint/style. Raw52 adopted statuses have no executable/cause attribution.
The original extractor remains unchanged. Hidden client/DNS blocking work can keep ordinary
Runtime teardown pending beyond the stop deadline; the registered native worker still owns it.
This is no new installed subsystem and no whole-host/UI acceptance.

[Prior full-host checkpoint](verification/2026-10-01/P03_SOURCE_OWNER_FULL_HOST_EVIDENCE.md),
source99/CLI c711, remains **mixed/failed**: both storage attempts passed13 behavior checks
but strict125 found live descendants; migration10 and GUI2cold cycles passed. Actual manager
active-turn Ctrl+C exited0 in0.265/0.267s without forced cleanup. Chromium/Playwright used
deterministic inference, not in-app Browser/live-provider or new attachment proof. Last older
fully passing runtime is `b3530790` (source181400/CLI876c); never relabel it for newer source.

## Fresh source922 runtime checkpoint — 2026-10-02

[Exact-source full-host evidence](verification/2026-10-02/P03_SOURCE922_FULL_HOST_EVIDENCE.md):
fresh CLI build passed in6m25s; SHA256
`890b4da9756644e56276e4f6dddb84439417dc250fa2f74ae201e1efb4948b11`,635,068,200 bytes.
All4build/runtime before/after maps equal8,900 entries and were independently rehashed.
Existing independent native storage/search packages were reused unchanged with the new host.
Storage13commands, migration10commands and GUI2cold cycles passed; all3 unchanged strict
runners exited0 with null errors. First active-turn Launch SIGINT exited0 in0.264997/0.264945s,
tracked identities absent, no forced cleanup. Fresh Chromium/Playwright screenshots were
visually reviewed; this is deterministic fixture evidence, not in-app Browser/live-provider proof.

Storage recorded3 adopted SIGPIPE statuses whose sampled PID generations match the native
Git executable; exact operations/causes are unknown. Migration recorded1 unattributed SIGPIPE.
This is not an all-child-exit-zero claim. Old source99 strict125 failures remain preserved.
The current fast regression is green; the shutdown contract is still open: no production stop
caller, native completion receipt or owned curated callback scope is present at source922.
The45s stdio watchdog/longer service budgets still need coordinated deadline/slow-cleanup proof.
Main remains source655; keep this work on the candidate branch until the contract gate passes.

## Native completion primitive checkpoint — 2026-10-02

[Source27d completion evidence](verification/2026-10-02/P03_WORKER_COMPLETION_EVIDENCE.md)
and [lineage](upstream/p03-worker-completion-lineage.json) bind six adopted files to source27d.
`begin_curated_repo_sync_process_stop` permanently closes process-wide admission and returns
an observer of the exact native handle. Awaiting attachment, Running, Joining, Joined,
Panicked and SpawnFailed remain distinct from the original typed sync outcome. Observer
expiry/cancellation preserves unfinished handle custody; a stopped retry join preserves its
original record. Competing native observers share one join, outside the gate mutex.

First compile/run **587/587** passed (516 core-plugins + 71 PTY), zero skips/retries,
strict exit 0/null error; 8,902 scoped files unchanged. Seven new tests cover closure,
attachment ordering, timeout/cancelled observers, competing observers, stopped spawn failure,
post-outcome native panic and Linux TLS-controlled stop during retry join. The 53 raw adopted
wait statuses include 17 nonzero results without executable attribution; no all-child-success
claim is made. Scoped lint exited 0 with no warnings or edits. Global format changed five
files only (imports/layout/closures/trailing commas); no tests repeated solely for formatting.
Tested map `1ccba62059edc9e98e83d3970eadf07b983ee57d0a6de17ccedef377a3e40c49`;
formatted map `83d21c866a170f6ffba815b9311615d3108aff3a88dae7b49cb10dd7ab34cfd0`.

This is preparatory native lifecycle work, **not another installed component or full-host repair**.
No production caller or callback ownership was added. Native TLS teardown, synchronous join
and panic-payload destruction can exceed an observer deadline; !pending does not imply success,
clear quarantine, establish descendant fencing or prove durability. Source922's passing full-host
checks remain separate. The callback checkpoint below adds local scope/routing; process-final authority/integration remains open.
The preserved completion proposal was adopted with a pretest cancellation-assertion lint fix and
exact parameter comments; its original bytes remain in recovery. The callback proposal was subsequently adopted as recorded below; C2b remains unadopted. The held-startup shutdown runplan retains native default startup,
strict arguments and private-report projection rules.

## Curated callback ownership and routing checkpoint — 2026-10-02

[Source0c1 callback evidence](verification/2026-10-02/P03_CURATED_CALLBACK_SCOPE_EVIDENCE.md)
and [lineage](upstream/p03-curated-callback-scope-lineage.json) bind the two ordered source commits.
Per-processor scopes reserve strong custody before spawning; close/admission linearize under one
mutex. Exact handles and original failures remain owned through pending/cancelled observers.
Real curated startup/account actions now use this scope. Embedded/external guards close local
admission and await accepted callback bodies before later teardown; outer guards survive processor
failure. Generic marketplace/remote callbacks and immediate account refresh are unchanged.

Attempt01 failed during libtest linking (signal 7 / Bus error, exit101), executing zero tests;
zero free disk was observed. The unchanged-source retry passed **953/953** (429 App Server +
524 core-plugins), zero skips/retries, strict0/null. All 11 new cases passed. Raw49 reaps include
12 nonzero statuses without executable/cause attribution. Both 8,906-entry maps equal
`dbcac4b64bb506cc7058d14ceccfad05477147644b9bc501379c345cd287fbb3`.
First lint preserved source but warned about a test's lexical guard scope. A reviewed two-brace
correction ended the existing synchronous block before its await; second scoped lint was clean
and unchanged, map `795a1cdc60c34becb591139f921a01393e66b178efc1c98bda61d5cf35935a54`.
Reviewed global formatting changed 13 paths, final map
`c4c476492f356181b4552fdaaa7bab1b4465837a5dc10dab8068b19c83f2bdd4`.
No tests repeated solely for lint/style. Only combined library tests ran; the first source commit
was not independently tested and no fresh full CLI/UI gate ran.

This is native lifecycle prerequisite work, not new component extraction or whole-host repair.
MCP invalidation/reload awaits do not establish custody/termination of delegated prewarm work.
Replacement B still cannot receive the native worker's completion captured for A. A process-wide
registry closing fence and actual binary final authority/callers are still absent. Deadlines do
not guarantee bounded synchronous teardown or clear uncertainty/quarantine.

## Ordered implementation queue

1. Verify source0c1's evidence publication/ref against its 16-file source manifest and all
   failed/tested/lint/formatted maps. Keep main source655 unchanged and source922 runtime proof
   separate. Preserve C2b and process-final/replacement proposals separately until reviewed.
2. Complete exact MCP refresh/prewarm ownership, registry late-owner admission fencing and
   home/generation-aware replacement completion delivery. Retain exact task/native custody
   across close, cancellation and observer expiry; test registration-versus-completion and
   replacement/sibling races. Merely awaiting an invalidation request is insufficient.
3. Add explicit binary-owned process-final authority on actual success/error/early-return paths,
   distinct from embedded leases. Use one absolute shared deadline, coordinated watchdog/force
   policy and explicit uncertain outcomes. Preserve sticky quarantine and original outcomes;
   !pending, direct-child reap and native join do not independently prove success or durability.
4. Rebuild the exact repaired CLI; run held production Git/HTTPS with exec success/error and
   App Server EOF/SIGTERM, then unchanged installed storage/migration/GUI session, streaming,
   approval, cancellation, recovery and actual Launch Ctrl+C gates. Exercise slow cleanup,
   forced uncertainty and same-process replacement. Coordinate the45s stdio watchdog with
   longer storage/GUI budgets. Only then consider nonforce main promotion. Never reinterpret
   older source99 failures using later zombie observations.
5. Adopt/test C2b cooperative extraction as a separate slice after rebasing against the frozen
   source; preserved proposal `d76bf2389695619fd051ffd24758ba43346e51977dc7115c98d568e863d7ef09`
   remains unadopted/untested. Complete recoverable repository/SHA journal and host-death fencing;
   restarting alone is insufficient. Cooperative checks do not prove bounded ZIP-constructor
   memory or hard native-blocking deadlines. Follow the
   [shutdown audit](verification/2026-10-01/P03_CURATED_SHUTDOWN_INTEGRATION_AUDIT.md).
6. Resume native auth load/refresh/persistence authority and independently installed auth/catalog,
   then all remaining P04–P19 subsystems and kernel audit. These lifecycle repairs do not replace
   actual extraction. P00M mapping continues; P18U must deliver independently installed maintenance,
   isolated real later-upstream integration preserving custom packages, semantic/security gates,
   version/schema coordination and external bootstrap rollback. Updater remains unimplemented;
   no periodic polling/live update follows from this roadmap. Optional viewer transport stays
   independent. Continue feasible checkpoints; no perpetual unattended runtime is guaranteed.

## Preservation and operational limits

Recovery root: `/workspace/recovery-backups/20260930T165936Z`.
Full original two-worktree/Git/SDK/evidence archive `codex-recovered-workspace.tar.zst` SHA256
`3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Frozen C2b proposal archive `p03-curated-c2b-extraction-proposal-preserved.tar.gz` SHA256
`3ef1f64c611440cadae1624c6b8512d3011d177ad4827732c120427fba6f7c3e`; local only, unadopted/untested.
Source922 archive `p03-curated-http-await-source.tar.gz` SHA256
`7f32137a371421dd3e0066e97a1128f9e74d8e7ea787853f21bb78b12b291067`.
Corrected C2a proposal archive `p03-curated-http-await-proposal-preserved.tar.gz` SHA256
`82fafc4a652e4a9b6cc0872ba9fee4b8bf658aa5bcdee59823ab7e0e19a04422`.
Sourcecb9 archive `p03-curated-shared-control-source.tar.gz` SHA256
`a40270b0c2c14d395e544c91f30bc80bb56b524bc1719ecd32caa94197569df7`.
Frozen B1b proposal archive `p03-curated-shared-control-proposal-preserved.tar.gz` SHA256
`58a1291c697f9954b17f3a95289165b64c15045b75ae2b2cec46ea801c9a0325`.
Source739 archive `p03-curated-retained-owner-source.tar.gz` SHA256
`a56c0bdbfc084cb49b0d7ea8cc1bc04803fd051743645ac651f439e904ba2225`.
Frozen original B1 proposal archive `p03-curated-retained-owner-proposal-preserved.tar.gz`
SHA256 `bf18fe1df4cee764d10d631c000b58e9292c396d0c96794f3a12feb037568f25`.
Earlier source9a2 archive remains preserved with its publication receipt.
Earlier candidate source archive `p03-curated-git-verified-native-source.tar.gz` SHA256
`e7212fe787c0ab1a83d37b069278a83e9926991e9a990fa376abefee99cee4f4`.
Original failed reports, tests-only candidates, both working trees and WIP branches remain.
Filesystem archives are local recovery, not proven external backups; GitHub provides external
source durability only for included published paths.

Source27d six-file checkpoint plus manifest: `p03-worker-completion-source.tar.gz`, SHA256
`14e2e5204433650639d2b544cbcd648ee6557fa97239d03e413900da88dcaa31` (42,897 bytes, seven members).
Original completion proposal: `p03-curated-worker-completion-proposal-preserved.tar.gz`, SHA256
`9ad6b58d322cb0554d78675f5accc3910bf9520aa937d44024b993a74cee2c41`.
Source922 CLI, all four source wrappers and strict runtime receipts are preserved in
`p03-source922-cli-preserved.tar.zst`, SHA256
`32a3ec3233a54369e8bfe95f313d4cc3dec8a87be09f447836b2a21d061f7ef5` (138,845,342 bytes).
Its receipt binds the exact CLI and two hardlinks/eight historical aliases. After all 17 archive
members and fresh process/reference guards were checked, both inactive hardlinks were retired to
recover626,188,288 allocated bytes; retirement receipt `p03-source922-cli-recoverable-retirement.json`
SHA256 `ca99e3555a1080cbfd320ae5fa18e42b4e090227196a7bf72b53aecc34aae41f`. The old HTTP-await core proof ELF was archived before completion tests reused
its mutable target: `p03-http-await-core-proof-preserved.tar.zst`, SHA256
`8650fb77202d9028d4df79d5529775a8d50bbd7ca217727d5dddf73b946396f4` (45,943,911 bytes).
It preserves the 509-case core executable from the 580-test cohort and separately binds tested
versus later formatted source. Its binary hash was observed during preservation, not captured
at test time; do not claim contemporaneous ELF identity or that all four cohort ELFs are archived.

The inactive source99 CLI c711 and its2mutable hardlinks were retired **after** full archive
and reference verification to recover625,934,336 allocated bytes. Archive
`p03-source-owner-cli-archive-preserved.tar.zst` SHA256
`68102a992d8d7ebe3105933d5377c9ec215ed5d51c8f814891555f81fb098222`;
retirement receipt `p03-source-owner-cli-recoverable-retirement.json` SHA256
`4e08e878403129680208a6fac8c0e52c194f8bfad2353d247c03b1c0a3e9f0ba`.
Eight historical mutable-target symlink paths remain unchanged. Source922 previously repopulated
their shared target; after its recoverable retirement they intentionally dangle. Alias names never
establish source identity. Restore an isolated archived target or rebuild and hash/source-bind
a new binary before reusing historical commands.
Original GUI PID371365 still uses the
separate frozen `/workspace/verified-component-checkpoint-v2-20260930/codex`; leave it intact.
Three inactive installer proof ELFs are verified in `p03-install-order-green-test-executables.tar.zst`
SHA256 `f289ec637ffccf8d400b335daa6c811b282162689fe58ca36acd47b2b8084fb8`; receipt
`p03-install-order-green-elf-preservation.json` SHA256
`d41c1028af19baa890f0fce69f137cdf43dd8f90457b7a2b68e03a984ed64a8a`.
This binds preformat400-test source separately from formatted source655. Their3build aliases
were retired after verification; Cargo metadata/source and current transport tests remain.

Callback source0c1: `p03-curated-callback-source.tar.gz`, SHA256
`2e29ff6603bd1711a920315d92a4b6cf842562f17b5ad7a04962723d58abf4f8` (140,474 bytes,17 members).
Frozen original callback proposal: `p03-curated-callback-scope-proposal-preserved.tar.gz`, SHA256
`66af7d8f0cebcca306d28fccb97571a1fd6a7e3b61096a7b6fd77cfa93be3f5a` (63 members).
The prior587 core proof is separately preserved in `p03-worker-completion-core-proof-preserved.tar.zst`,
SHA256 `053932884dae358433597d7776c0066fd2e1c97e99df0f087f1bfb4bc0469232` (46,063,416 bytes,35 members).
It binds the516-core subset of the587 cohort; current archival hash is not a test-time ELF hash.
Current953 core71/AppServer bafa proof executables still need guarded preservation before their
mutable targets are reused. Do not delete them or the old failed2cf proof without explicit
archive/ref verification. Keep failed01 and green02 maps/logs/strict receipts separate.

No Rust/runtime command is active at this checkpoint. Additional exact unused Cargo artifacts
were retired under guarded manifests to build source922 and the completion tests; retain receipts
`p03-source922-build-cache-retirement.json`, `p03-source922-postbuild-cache-retirement.json` and
`p03-worker-completion-build-cache-retirement.json`, `p03-callback-cache-retirement.json` and
`p03-callback-failed-link-temp-retirement.json`. Source, depfiles/fingerprints, static V8,
proof archives and the frozen GUI were preserved. No broad cargo clean is authorized.
Before commands verify current disk/cgroup headroom, not a historical estimate. `/tmp` is nearly full. Source
`/workspace/toolchains/component-verification-env.sh`; set `CARGO_BUILD_JOBS=1` and
`TMPDIR=/workspace/acceptance/p03-source-owner-runtime-temp` (0700). Read AGENTS.md; use unique
`/tmp/run-p02-check.py` report prefixes/actual maps, `just test --locked --retries 0`, scoped
`just fix` and global `just fmt`. Never kill Rust or weaken assertions. No test rerun solely
for formatting. Preserve failures and privacy: no raw private GUI/auth reports or credential output.

The selected expanded LLVM Bazel cache was retired after download checksum and fresh
reference checks, recovering 619,810,816 allocated bytes. Compressed LLVM archive and
all Rust/source/runtime artifacts remain. Receipt `p03-bazel-expanded-cache-retirement.json`
records normal expunge removing the generated output base but exiting 36 while waiting for
server PID 834487 (observed Z); do not report expunge success. About 1.07 GB was free afterward.
Future Bazel re-expansion/offline operation is unproven; inspect headroom before builds.

The exact unused expanded Bazel V8 source cache was also retired after fresh entry identity,
process/alias and retained download checksum verification, recovering220,135,424 allocated bytes.
Receipt `p03-bazel-v8-expanded-cache-retirement.json` SHA256
`cdab7f82848ebcc7407e192a60cf20b00cca3358f29b83715ee884368912507d`.
Compressed V8 archive, Cargo static V8 and all Rust/proof/source/runtime files remain.
Offline Bazel re-expansion remains unproven; about1.25GB was free before C2a compilation.
Recheck actual headroom before the next command.

Original task environment/config identity is unchanged, revision 1462 observed running/connected;
restricted package-managers policy still has additional allowed_hosts[]. In-app Browser and
Context7 are unavailable, and no external preview is proven. Keep the original VM; no reset or
policy bypass. Upstream libraries/framework docs are required when applicable, not for ordinary
native refactoring. Viewer networking does not block this queue.
