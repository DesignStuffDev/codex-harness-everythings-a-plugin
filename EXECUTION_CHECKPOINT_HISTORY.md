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
server PID [redacted from public history] (observed Z); do not report expunge success. About 1.07 GB was free afterward.
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


## Preserved execution ledger before the 2026-10-02 replacement checkpoint

This is historical, superseded state. The canonical current queue is in EXECUTION_STATE.md.

# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Incomplete platform; P03 lifecycle prerequisite in progress.**
Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required updater: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Detailed earlier results: [checkpoint history](EXECUTION_CHECKPOINT_HISTORY.md).

## Active continuation after published recovery checkpoint

Latest published WIP is `3ccba58b1077e9372458fe2994a72f30ee643e41`, tree
`df32f16bb0e64073d820b6ba24ea701a1a4b65e5`: all49 changed blobs and branch refs read back.
This preserves the542-pass stage1 source and pending exact-Git acknowledgment/SQLite-isolation
proposals. Main remains unchanged. Receipt R/`p03-observation-stage1-publication/PUBLICATION.json`.
A narrow formatter-only exclusion for byte-frozen recovered-gate-proposals is now adopted in
ruff.toml (aa96c73e…); active source/lint selection unchanged. Actual justfmt is still pending.


Recovery/current58 checkpoint `f8a5905bb666846a7885171280d9454a9b8a269b` (tree
`43763a456fe80361d24f8912c0328ce9ffdd8eba`) was pushed nonforce to the existing WIP branch;
all43 changed blobs and both branch refs were read back. Main remains781080f7. Publication
receipt: R/`p03-current58-publication-preparation/PUBLICATION.json`.

Root then adopted the reviewed stage1 read-only worker/callback observation APIs and explicit
replay-test semaphore-timeout failure correction: eight paths, no new stop/join authority.
The new62-path manifest is R/`p03-observation-stage1-source.json`, SHA256
`f0535c6c86c35e46b34fc361e5c60e495c6048688cf49bc616add66e8734a019`.
Original current58 source and proposal preimages remain preserved. Current checkout native
source is now newer than the build02 CLI; do not call that old binary current-source proof.

Scoped `just test -p codex-core-plugins --lib` started with source wrapper/unchanged strict
runner, one build job and test fixtures under `/tmp/p03-observation-stage1-tests-01`. Prefix
`/workspace/acceptance/p03-observation-stage1-tests-01`. The command finished0:542/542 tests passed,0 skipped, unchanged source; strict exit0/error-null.
Preflight free overlay1.286GB; only about245MB remains after dependency recompilation. Do not
start the larger App Server build before fresh headroom review. Never kill Rust commands.
Fresh resource review completed after stage1. Root independently recomputed122 protected
roots/1,892 dependency nodes with zero missing edges or selected intersections. Exact138 old
ordinary internal library caches were retired after fresh source/graph/hash/alias/process guards;
receipt R/`p03-observation-stage2-cache-retirement-01/RETIREMENT_RECEIPT.json`, SHA256
`2dd6249492d6214ebb12ba6d188b755767d01537517ef10397d0f6e5d01ba0f8`. All proof executables,
source, index and27 archive metadata records remained unchanged. Free overlay1,648,308,224B.
Do not rerun this completed action. A second completed preservation action archived ten inactive
acceptance snapshots (77,900 members), verified their exact contents, then retired only the
listed caches. Parent homes/configuration/auth/state, source and all proof executables remain.
Archive R/`p03-stage2-finished-snapshots-preserved-01.tar.zst`,54,969,828B, SHA256
`fb2c65d4a6e57b2952a1f4da7c4c6eb87b0fd14244e52c8a206c22767bb6558e`.
Promotion receipt SHA256 `563956a6d4fb4d39bbab981ee5305af4ab23b782b44d1fba10d052c812d7fe7d`;
retirement receipt SHA256 `3ca92e2f0579d46b92c12154481b4eae7e3c97a29d0cab530edc4e0f29a537a5`.
This is a VM-local archive, not an external backup. Do not rerun either completed action.

Remaining replacement corrections are staged at R/`p03-replacement-stage23-isolation-proposal-01`
(manifest861c8719…) and `p03-replacement-stage23-isolation-ordered-plan-01`
(APPLICATIONc2ac86c0…). They bind test SQLite storage before opening state; no production
configuration policy changes. Root adopted native stage2 units1+2 after preservation and
independent re-review; the SDK parent remains unadopted. Original proposals/preimages remain.
Current63-path manifest R/`p03-replacement-stage2-source.json`, SHA256
`2b435c5b0f74b2eaa65483a8ea9ddab613c46313e5a2304a7481d25019032626`.
Scoped `just test -p codex-app-server --lib` started2026-10-02T09:46:58Z with one build job,
the unchanged source/strict wrappers and fresh TMPDIR `/tmp/p03-replacement-stage2-tests-01`.
Prefix `/workspace/acceptance/p03-replacement-stage2-tests-01`; preflight overlay2,577,461,248B,
OOM9/kill4 unchanged. Inspect its terminal receipt before proceeding; never kill Rust commands.
The new native child is intentionally ignored by ordinary discovery; a build/suite pass will
not establish replacement acceptance. That requires the three parent-owned real-runtime cases.

After stage1, review/adopt native A→B child, run scoped App Server library checks, bind the new
actual child ELF and execute parent pending/replay/race. Reaudit resources before that larger
build. The held Git→HTTP fixture needs positive independent Git observation before releasing
502; its original4/8 result and all strict assertions remain.
The new positive-observation fixture is staged under R/`p03-held-host-observer-ack-proposal-01`,
manifest14c27cff51d2e369cdfbe7caa5e2f94ce4e20e535f06dee1f240fcef6d878568. All15 pure
fixture checks passed strict0/null, proposal files unchanged; prefix
`/workspace/acceptance/p03-held-host-observer-ack-policy-01`. No native runtime rerun is claimed. Stage this separately, then bind
any rerun to the actual tested native source/binary rather than relabeling build02.

## Current verified recovery — 2026-10-02

The original executor is usable again: the read-only marker executed successfully, followed
by repository/filesystem/process inspection. Origin and upstream local HEAD are unchanged;
the real index hash still matches the value below. All8,920 scoped source files match the
terminal production-build01 source map, including all56 candidate hashes. The original
sibling worktree and frozen GUI process family remain present. No live Rust build was found.
This clears the executor-access blocker; no new native/runtime test pass is implied.

The full-source archive and temporary test-fixture archive both match their saved SHA256.
The latter has been copied/fsynced/hash-verified back to its original recovery path with its
mode/mtime, and only the temporary duplicate removed. Its new inode is recorded explicitly.
Receipt: R/`p03-fixture-archive-restoration-20261002.json`. Published f122 documents were
reconciled after preserving local preimages under R/`p03-recovered-f122-sync/preimage`.

Production-build01 is confirmed finished101 at03:36:30Z with unchanged source and ENOSPC;
it is not still running. At initial recovery disk headroom was insufficient. First24 inactive clone caches
were archived with840 members verified, then exact paths retired and archive promoted to R;
all homes/configuration/state and recovery material were retained. Proposal02's leading-slash
ROOTS mismatch was preserved and repaired in03. One retirement invocation's malformed digest
was rejected before action; the corrected pinned invocation completed. Resource receipts live
under R/`p03-production-curated-clones-preservation-03`. A second reviewed wave05 archived
34 additional inactive clone roots with1,050 members before retiring only their exact cache
paths; proposal04's ancestor-alias rejection remains preserved. Both promoted archives are
VM-local recovery material, not external backups. About0.79GB remained before the additional
355-file cache retirement described below. Recheck resources before further compilation.

Root adopted three SDK acceptance files to allow an explicitly pinned, previously independently
built search package to be tested with the rebuilt host. Original-manager validation remains
the default; reuse checks the original build, package and source inventory and reports reuse
separately. Eleven focused checks passed with strict0/null and unchanged source, prefix
`/workspace/acceptance/p03-search-reuse-proof-tests-01`. This is test-helper evidence, not native
or GUI runtime proof. Native bytes are unchanged. Current58-path manifest:
R/`p03-process-final-search-reuse-source.json`, SHA256
`5e476a930d92284e0b827c7cbec247b97ba21976519a7ad31bba060747ac3094`;
current8,921-file map `15e9a44f54f414972eff02de9cd4afdfa5f900263d46b69a96119499779c136b`.
Original preimages, the previous56-path manifest and original failed receipts remain intact.

Fresh graph review identified355 obsolete internal ordinary library-cache files totaling
2,726,678,528 allocated bytes. Root reviewed and executed the exact journaled action after
fresh source/graph/hash/reference guards. RETIREMENT01 under
R/`p03-recovered-production-cache-review-01` records all355 removals, unchanged protected
inputs and3,515,535,360 free overlay bytes. No current compiler input, executable proof,
source, state, metadata or archive was selected. This completed action must not be rerun.

### Current58 production and runtime evidence

`/workspace/acceptance/p03-process-final-full-cli-build-02` completed0 in about75 seconds,
with identical8,921-file source maps. Actual CLI SHA256
`d716c6ee9c738c2db9197fbb2ab91d3918839d0b6d32f4556e8a7e1fa2717a92`;
manager `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
The `.binaries.json` receipt binds both to the successful source/log receipts. No additional
OOM event was observed. Native code is unchanged from the56-path process-final candidate.

The current-source storage reuse gate, ordinary migration, ordinary GUI, separate search-reuse
migration and independently installed-search GUI all passed unchanged source/binary guards and
the original strict runner (exit0, error null). Prefixes are `p03-process-final-` plus
`newhost-storage-01`, `migration-01`, `gui-normal-01`, `migration-search-reuse-01`, and
`gui-search-reuse-01`. Both GUI modes completed two cold cycles, streaming, approvals/Stop,
file search and actual manager SIGINT with tracked descendants absent. Manager shutdown was
about0.265–0.268s. These are real Chromium/Playwright runs with deterministic inference,
not in-app Browser or live-provider tests. Reused packages retain their original independent
build proof; no new independent plugin build is claimed.

**Slow installed-storage gate04 passed:** the corrected selector matches the actual lossless
component-wire `EventMsg/ItemCompleted/UserMessage` event. Eight selector and12 synthetic wire
cases passed, followed by two fresh10-command migration runs and real normal/forced GUI runs.
Normal actual Launch SIGINT waited46.469 seconds, exited0, completed native append/shutdown and
recovered the canonical event in a cold GUI. Forced second SIGINT exited1 after2.041 seconds,
explicitly reported durability unknown, and cold native restart succeeded. All tracked processes
were absent. Strict0/null and source/binary guards passed. The relay holds before native storage
admission; this proves an in-flight installed-component call across the old45s limit, not a
native-internal durability-admission or a timed App Server watchdog test. Forced canonical UI
absence is not proof that no earlier raw model-history input was stored.

**Failures retained:** slow01 confirmed raw prompt append/native cleanup but failed cold GUI
projection because cancellation preceded ItemCompleted(UserMessage). This gap remains open under
P07/P16 with P14 atomicity dependencies. Slow03's selector used persisted JSON rather than wire
JSON; its gate never matched and it timed out before SIGINT. Neither is relabeled as a pass.
Held Git/HTTP attempts01/02 both failed before full readiness on an unexpected auxiliary ChatGPT
CONNECT;01 also watched the wrong installed Git path.02 identified the actual native-selected
/usr/local/bin/git and separately observed the auxiliary connection. Forced fixture cleanup is
failed evidence. Source shows a parallel featured-plugin warmup consistent with this connection,
but the encrypted request path was not observed. A bounded separately accounted auxiliary
connection fixture is being checked; no held-host lifecycle pass exists yet.

Native A→B replacement, exact MCP custody, TUI and changed App Server integration tests, scoped
lint/global formatting and crash-safe repository/SHA publication remain open. This checkpoint
adds no independently extracted subsystem. Full evidence and immutable source identities are in
[the recovered-host report](verification/2026-10-02/P03_RECOVERED_HOST_EVIDENCE.md).


**Held-host03 terminal result:** four real Git-held cases passed all native ownership, independent
executable-observation and strict descendant checks (exec success/error; App Server EOF/SIGTERM).
The fifth Git-failure→HTTP-held case passed its local native/strict checks, but the independent
sampler missed the short-lived Git executable. Its parent gate failed; it is not counted passed.
Three remaining cases did not run. No forced fixture cleanup occurred in these five cases.
The matrix remains failed/incomplete, and MCP/session ownership remains separately unverified.
See [sanitized exact receipts](verification/2026-10-02/P03_RECOVERED_HELD_HOST03.json). A bounded
positive-observation handshake is being reviewed; do not bypass the independent observer check.

## Historical startup failure and independent progress — 2026-10-02

Supported startup of the selected original environment failed with
`exec-server protocol error: failed to query executor configuration capabilities`.
The environment context reports failed and terminal/edit tools are unavailable. The status
service separately reports running/connected at observed revision1463; this does not establish
execution readiness. No current process/build/filesystem inspection could run. The last recorded
production build is terminal101 (ENOSPC), not a newly observed live build.

Independent feasible work completed through immutable GitHub source: the
[P03 process-final lineage map](upstream/p03-process-final-lineage.json) now binds all56 candidate
paths to parent/import blobs,56 selected literal anchors and two unchanged manager references.
All31 upstream-existing paths match direct official pinned-revision reads. Eight impact groups
record ownership/deadline/replacement/publication hazards. See the [audit and validation limits](verification/2026-10-02/P03_PROCESS_FINAL_LINEAGE_AUDIT.md).
This is P00M provenance progress only: no new extracted component, native/runtime test, normalized
index closure or updater implementation. Main and all native implementation bytes are unchanged.

These historical blockers were resolved by the current verified recovery above. Retain this
failure history; it does not describe current executor availability.

## Historical immediate recovery checkpoint — 2026-10-02

Resolved by the current recovery/build/runtime evidence above. The following preserves the
original failure state, not the current next action.

The latest production CLI/manager build finished101 before runtime testing because rustc
could not create a temporary directory: `No space left on device (os error28)` while
compiling the normal `codex-tui` library. This is separate from the earlier TUI test-target
SIGKILL. The wrapper recorded unchanged scoped source. No newer storage/GUI runtime pass existed at that failure checkpoint.

After this failure, terminal transport disconnected. The task-bound environment subsequently
changed from offline/starting to running/connected in status metadata, but two read-only
`pwd` attempts still failed with `Noise harness handshake failed before connection became ready`.
Metadata connectivity does not establish usable execution or filesystem recovery. No reset,
replacement checkout/VM, source edit, or new extraction was performed. This update is published
through the GitHub connector only; the local working files have not been synchronized to it.

**First reconnect action:** verify the original checkout, real index hash, both source trees,
active processes and artifacts before any mutation. One inactive generated-test-cache archive
was temporarily moved from R to RAM-backed `/tmp`; its survival and restoration are outstanding.
See [exact failure, preservation and recovery instructions](verification/2026-10-02/P03_PRODUCTION_BUILD01_AND_EXECUTOR_RECOVERY.md). Do not blindly
retry the build with the same disk margin or reuse a historical cache-deletion list.

## Resume identity and rules

- Owning checkout: `/workspace/codex-harness-everythings-a-plugin`, original cloud VM.
  Preserve sibling `/workspace/codex-harness-next-components`; no replacement checkout/VM.
- Origin: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
  Upstream `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`, imported exactly at
  `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`. Retain Apache LICENSE/NOTICE.
- Local HEAD/index remain upstream and local main is stale. Use reviewed temporary indices
  and nonforce connector publication; never reset/rebase/rewrite the real index.
  Index SHA256 `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
- Root alone mutates the checkout, Git state and build cache. One Rust/build/cache writer.
  Workers stage proposals only in `/workspace/recovery-backups/20260930T165936Z` (R below).
  Read AGENTS.md; use `just test`, scoped `just fix`, global `just fmt`. Never kill Rust.
- Nonforce verified checkpoint commits/pushes are authorized. Check both remote refs before
  publication and read back changed blobs afterward. Main promotion still requires the gates below.

## Published and active source

Last verified main: `781080f7e3c8bfe1953378001d777dff33d74bc3`, tree
`22cf918cf77f16d5d947a59b968e342cde7f71d0`; implementation remains native installer
source `65511842d7051b2a1f5cc52917f3ebb5c03be4f3`.
Last published development candidate: `ba87799c2e9ab821a326f10715c11cf6ec256896`, tree
`c75dfdae88843dcbcbdbebe8d0e2ed4dfae6e9c7`, branch `work/p03-curated-sync-lifecycle`.
Receipt: R/`p03-curated-callback-publication.json`. Combined callback source0c1 passed953
library tests, lint and reviewed formatting; it did not have fresh full-host proof.

Latest published preservation/provenance checkpoint on the WIP branch is
`f122aed5fdf93e0afe0669d2022046c2749bd4b8`, tree
`bc436e345f83b3cdb080b4e0cf571042b18f53d5`. The following older entries are history.
This recovery checkpoint contains the current58 SDK helper changes and exact runtime evidence.
The parent ref above is the last verified publication before this commit; require remote readback
and the root publication receipt before treating this checkpoint as externally preserved.

**Current adopted candidate is preserved as unfinished WIP** at
`d989351c0f016a3f8c8bc2ea3248c1c19e915dd5`, branch
`wip/p03-process-final-and-mcp-preservation-20261002`; all270 selected blobs read back.
The WIP report/readback supplement is `248420e4bf8e6dc4c98c8ec04ee2f71b79b4f1f3`.
Terminal TUI failure is published at `bff986f90cacfa210b54edf5435f3e7220f52436`. The next verified preservation checkpoint is `1f808b9fcac8fc2f548b73bc0e486459bc9c59ce`, tree `d7bf394cf60aa688ca975c7952f4f1482e47f59f`:48 changed blobs read back, including45 inert observation/callback/replacement-acceptance source and metadata files. Main is unchanged. This later documentation-only recovery update does not adopt those proposals.
This is not a main promotion or release. Receipt:
R/`p03-wip-preservation-publication/publication-receipt.json`. It adds process-final authority/callers,
curated admission fencing/native and callback observations, coordinated shutdown budgets and
same-home replacement delivery. Exact56-path manifest:
R/`p03-process-final-native-retry02-source.json`, SHA256
`0a46a9a3f50670a4cc10180012dd3adb09ee2226c75efb8db8c86fbb093d4065`, virtual tree
`1da290682fc6ddd00994c2af7eac9018c8143dea` before subsequent root documentation edits.
Its8,920-entry tested source map is
`31f82854bdfeffe7ec99a05ef46a9a2eaba2cacd1b2587a568e8f2be81836a73`.
Shared deadline:200s graceful /205s forced-exit initiation through actual runtime teardown;
GUI207/209s inside manager210s. Forced outcomes explicitly retain durability uncertainty.
This adds **no independently installed subsystem**; MCP/global descendant closure remains open.

## Actual verification — do not transfer proof between sources

Current evidence prefixes are `/workspace/acceptance/p03-process-final-`:

| Check | Actual outcome | Limits |
| --- | --- | --- |
| `native-tests-02` |703/703; strict0/null |536 core-plugins +157 transport +7 arg0 +3 process utilities. |
| `app-server-tests-01` |431/431; strict0/null |Two slow-store cases use paused virtual time. |
| `client-tests-02` |42/42; strict0/null |Unchanged-source retry;7 adopted SIGPIPE statuses unattributed. |
| `exec-tests-01` |73/73; strict0/null |69 library +4 executable tests; actual production-host run still pending. |
| `cli-tests-01` |299/299 executed; one ignored helper; strict0/null |Four new lifecycle/fatal cases passed; skipped `blocked_probe_fixture` is not counted passed. |
| `manager-tests-01` |88/88 executed, one ignored subprocess helper; strict0/null |Six launcher cases; short injected budgets, not actual210s cleanup proof. Three raw adopted SIGKILL statuses not attributed here. |
| `desktop-tests` |14/14; strict0/null |Python transport fixtures on pre-compiler-repair source; no real browser/engine. |
| `native-tests-01` |compiler101; zero tests |One extra `>` repaired, original failure retained. |
| `client-tests-01` |linker101; zero tests |Signal7/Bus error at zero free disk; source unchanged, separate evidence. |
| `tui-tests-01` |compiler101; zero tests |rustc SIGKILL after2,156.43s; unchanged source. Memory pressure observed; no pre-run OOM counter baseline, so cause not conclusively attributed. |
| `full-cli-build-01` |production build101; no runtime checks |Normal TUI library compilation hit ENOSPC after about6m05s. Source wrapper recorded unchanged scope; distinct from TUI test SIGKILL. |
| `full-cli-build-02` |production build0; current58 source unchanged |Exact binaries and successful storage/migration/GUI gates are bound above; lifecycle/replacement scope remains incomplete. |

Green native runs have zero retries and identical before/after source maps. CLI and manager each
report one ignored helper; neither helper is counted passed. The other completed native suites
have zero skips. The six native suites total1,636 executed passes; this is not a unique project-wide coverage count.
No claim that all child processes exited0; strict descendant assertions were not weakened.
Manager88 checks now pass. TUI, changed App Server integration, lint and the remaining lifecycle/replacement gates remain open.

Historical rebuilt-host/runtime proof on **source922**, commit
`92212516ad4d12bcf60546ee5f879b983ed44682`, publication
`2ac44529d0dfa261c6d2a09005a34b3ee22754ed`. Storage13 commands, migration10 and GUI2 cold
cycles passed with strict0/null. Actual active-turn manager SIGINT exited0 in about0.265s,
tracked processes absent and no forced cleanup. Chromium/Playwright and deterministic inference
were used, not in-app Browser or live-provider testing. See
[exact evidence](verification/2026-10-02/P03_SOURCE922_FULL_HOST_EVIDENCE.md).
The old source99 live-descendant failures remain failed; later observations do not replace them.

## Ordered next actions

1. Recovery, source reconciliation, both original archive checks, cache waves03/05 and355-file
   retirement are complete. Do not repeat destructive cache actions or reset the checkout/index.
   The current58 production rebuild, installed storage/migration, ordinary/search GUI and
   slow-storage normal/forced gates passed. Preserve all original failures and exact source maps.
2. Finish the bounded current58 held Git/HTTP matrix with actual selected Git and separately
   counted auxiliary CONNECT. Require native ownership outcomes and unchanged strict descendant
   assertions; stop and diagnose any failure. Do not weaken proxy/DNS/feature/readiness controls.
3. Publish the SDK helper/evidence checkpoint on the existing WIP branch, leaving main unchanged.
   Then adopt the reviewed read-only native observation/callback units, including the explicit
   semaphore-timeout failure correction. Use a new manifest and scoped core-plugins just test.
   Adopt the native A→B child/parent in separate small units; scoped App Server library tests must
   produce the exact child ELF before actual pending/replay/race acceptance. Current58 binaries
   cannot prove these new files. TUI/changed integrations/scoped lint/global formatting remain.
4. Review/adopt MCP custody in dependency order: upper02 → retirement03 → lower workspace →
   narrowly pinned SDK completion/adapters → process-final aggregate/late-admission fence.
   Retain all proposals/failure outcomes and run actual transport/process/full-host gates.
5. Complete P03 cooperative archive extraction C2b, transactional repository/SHA publication and
   host-death fencing, then native auth persistence/load/refresh and installed auth/catalog.
   Lifecycle observation and tests are prerequisites, not substitute subsystem extraction.
6. Follow P04–P19 for inference, durable state/replay/context/compaction, security/execution/tools,
   MCP/services, session/agents/events and clients/release/minimal-kernel audit. P18U still requires
   installable maintenance plus external recovery, real later-upstream isolated integration
   preserving custom plugins, and breaking/failure rollback. No polling/live deployment scheduled.

## Staged work, not accepted implementation

All paths below are under R. Do not silently apply an entire staged tree.

- `p03-mcp-session-custody-proposal/COMBINED-MANIFEST-02.json`, SHA
  `d99b4b1ce641d51952eecb0adf9ba761777c1f5bea45ee2c9d7e5d961714dcef`;
  follow-on `propagation-lifetime-review-03/MANIFEST-03.json`, SHA
  `9431799f6a8331e5bd2e57db2122c2c25738685eb22d14028e5a869d8a1f319d`.
- `p03-mcp-transport-custody-proposal/LOWER-WORKSPACE-MANIFEST-01.json`, SHA
  `2b587ab16cf31e8798b0f5ceead85f6d1324ecf3ba78694292df0323b4bba2b4`.
- `p03-pinned-rmcp-http-completion-proposal/MANIFEST-01.json`, SHA
  `3882d8b918b2ba3f411488210b4a7cac24dc1f78ea01bfa3c4fb22420e2b8095`;
  use its ADOPTION-ORDER.md and upstream license/provenance before dependency changes.
- `p03-installed-slow-store-acceptance-proposal-02/MANIFEST.json`, SHA
  `313fbe4f70d7b550df3e50b9e7b5d330d5e0062a2008ada0e5843f6e35308a63`.
  Synthetic4/4 passed with strict0/null and negative-control bug reproduction. Its original actual normal01 run failed; corrected wire-selector04 real normal/forced runs
  passed as documented above. Original proposal02 remains unchanged. Original proposal and both independent reviews preserved.
- Held production host SDK acceptance and same-process replacement plans are staged separately;
  native Git/HTTP activation must stay enabled, without proxy/DNS/assertion weakening.
- New read-only native lifecycle observation proposal: `p03-curated-lifecycle-observation-proposal-01/MANIFEST.json`,
  SHA `09f621f1a67105a6794894f441322f92bc2af53348e89ce55be6c32269c42f4d`;
  callback activity overlay: `p03-curated-callback-activity-proposal-01/MANIFEST.json`,
  SHA `340033c388567d294cb471ecb741e62da0cb1f14e3f508ba4134f5afec53fa10`.
  Static reviewed only; six authored tests unrun. These and the actual same-process acceptance
  source are included in the additional45 inert files published at1f808b9, outside the earlier204-file
  set. The ordered plan manifest is2916793d6706c28fe1741799c7ee05c87bd36141f396f978ecc6f9849a98f0db.
  External preservation does not adopt, compile or execute them.
- C2b manifest `d76bf2389695619fd051ffd24758ba43346e51977dc7115c98d568e863d7ef09`
  remains unadopted/untested. Its cooperative limits are not a hard memory/syscall bound.

## Preservation, resources and test setup

Original full two-worktree/Git/SDK/evidence archive:
R/`codex-recovered-workspace.tar.zst`, SHA256
`3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Source/proposal/proof archives and retirement receipts were retained in R at the last working
terminal observation, except the explicitly documented temporary test-fixture archive relocation.
Filesystem access and both original archives are now verified as recorded above. These are
VM-local recovery, not externally durable backups. Published GitHub refs preserve only their included source.
The new WIP commit additionally preserves204 inert MCP/slow-store proposal/preimage/license files,
the current56 native paths and root planning/status/history documents. New observer/callback/actual
replacement acceptance proposals are additionally preserved externally at1f808b9 (ordered plan under
R/`p03-curated-replacement-acceptance-ordered-plan-01`, manifest2916793d…); not adopted/tested. No binary/runtime archives
were exported; unadopted proposal status remains unchanged.
Current703/431 test ELFs and failed client linker output are archived and verified; their inactive
mutable build paths were retired. Source922 CLI is also archived/retired; historical symlinks dangle.
Never use an alias name as binary identity. No broad cargo clean or source deletion.
Original GUI PID371365 uses frozen `/workspace/verified-component-checkpoint-v2-20260930/codex`;
leave that runtime intact. Detailed archive hashes are in the checkpoint history and receipts.

Source `/workspace/toolchains/component-verification-env.sh`; set `CARGO_BUILD_JOBS=1`,
`TMPDIR=/workspace/acceptance/p03-source922-runtime-temp`, `PYTHONDONTWRITEBYTECODE=1`.
Use R/`run-process-final-check.py` with the exact candidate manifest and unique report prefixes,
scoping codex-rs, component-sdk, MODULE.bazel and MODULE.bazel.lock; never relabel stale main.
Strict runner `component-sdk/tests/subreaper_runner.py` SHA256
`fe01097ae1741cbcb76e15fb07c0c936108a4dc35c08b4264c1caa9eb1e08875`, unchanged5s drain.
CLI executable tests completed green. Client42 and exec73/CLI299 proof archives were
verified before retiring their four inactive ELF paths; about2.09GB remained.
`p03-process-final-tui-tests-01` finished101 at03:08:23Z after2,156.43s; rustc was killed
with signal9 before tests. Scoped source map stayed31f82854… . Cargo908575 and compiler912290
are absent. About592MB overlay space remained; no linker invocation was observed. Cgroup
memory limit is16GiB; post-run shmem alone was11.14GB. OOM counters are cumulative and
have no recorded pre-run baseline, so SIGKILL is not conclusively attributed. Do not retry
with unchanged resource conditions. Preserve the failed run and exact compiler inputs first.
Inactive repository Bazel service886854 required forced termination after graceful attempts;
this is build housekeeping, not a harness cleanup pass. Rust was never signalled.
Only the byte-verified68MB SHM duplicate of the preserved failed-client archive was removed;
the nonidentical large TMP/SHM candidates remain intact. The inactive historical P01 CLI
was archived with all23 members verified (archive63f59e07…), then its exact426,586,112-byte
allocated path retired with fresh reference guards (receipt a5e0a16c…). This is VM-local
recovery, not external binary backup. The initial post-TUI policy752f6689… was rejected for runtime-first work because it
included ordinary TUI/CLI inputs. Policy02 SHA25a76a88… retains their exact dependency closure
and selects350 old internal cache files (448.6MB disk /1.775GB tmpfs). Exact audit03
SHA4f3e163a… passed with pinned guard bytes. Root action03 completed:350 obsolete regular
cache files and180 exact aliases retired,448,643,072 disk bytes and1,774,649,344 tmpfs bytes
reclaimed, source/current closure/proof metadata unchanged. Receipt892fdd3d… . Audit02 is
retained but rejected because guard source changed during that read-only run.
97 inactive generated test-plugin cache subdirectories (parents retained) were
archived privately (archiveabb99ada…,239,382,700B,88,546 members verified), then retired
with fresh guards:97 exact subdirectories/61,507 file aliases removed, all parents retained,
2,220,015,616B unique tmpfs allocation recovered. Receipt under
R/`p03-test-cache-preservation-proposal-01/RETIREMENT_RECEIPT.json`; VM-local recovery only.
After the production disk-full failure, that archive was hash-verified and temporarily relocated
to `/tmp/p03-test-curated-caches-preserved-01.tar.zst`; restoration is now complete and verified
as described above. The recovery note remains historical.
Resource receipts are in R.
Further cache or proof-artifact actions require their new exact guards; never reuse an old list. Read actual current processes before starting another Rust command.
Do not print raw private GUI/auth reports, environment or credentials.

In-app Browser and Context7 tools are unavailable at the latest capability check. Use the labeled
Chromium/Playwright fallback for real UI checks. Optional remote-viewer network access remains
blocked; no unchanged retry, public unauthenticated service, or replacement VM. It is not a core
extraction prerequisite. Work can continue across checkpoints; unlimited unattended execution is
not guaranteed.


# Historical ledger at the featured production rebuild — 2026-10-02

This is an immutable historical snapshot, including then-active build state. Follow
EXECUTION_STATE.md for current actions; do not repeat completed cache or proof actions.

# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Incomplete platform; P03 lifecycle prerequisites remain open.**
Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required updater: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Earlier detailed ledgers are preserved, with process IDs redacted from the public copy, in
[EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

Immediate queue: observe the running current-source
production build and installed storage/migration/GUI/slow Launch regressions.
Stage B Core Plugins passed553 tests; Stage C App Server passed432 with two
parent-owned cases skipped by the library suite. Both were then exercised:
the new featured public-Drop/same-home shutdown case passed, and the unchanged
curated pending/replay/race cases all passed against the new App Server ELF.
The current source is not yet verified as a production CLI or GUI. Do not reuse
older final65 runtime results as current proof. Close remaining P03 task/transport
ownership before P04 extraction; MCP proposals remain isolated under R.
Preserve failed matrices and do not promote unfinished P03 WIP to main.

## Identity and preservation

- Original checkout: /workspace/codex-harness-everythings-a-plugin.
  Preserve sibling /workspace/codex-harness-next-components; no new VM/checkout/chat.
- Origin: https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git.
- Upstream: openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478.
  Exact-tree import ae720ae9a98bad29ca2cff998e7d5baaf05cec86, tree
  147ac2447134294359c4071b0aeb495922760db7. Retain licenses/NOTICE.
- Local HEAD/index remain upstream; local main is stale. Never reset or rewrite
  the real index. Index SHA256:
  0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59.
- Root owns checkout/Git/compiler/cache mutations. Workers stage only under
  R = /workspace/recovery-backups/20260930T165936Z. A = /workspace/acceptance.
  Recheck resources before large builds; never kill Rust commands.
- Actual commands/tests executed in the recovered original VM. Executor access
  is restored; this is not full-v1 acceptance.
- Preserve older live GUI under /workspace/verified-component-checkpoint-v2-20260930
  and /workspace/remote-viewer-setup. It is not current-source proof.

Full earlier two-worktree/Git/SDK/evidence archive: R/codex-recovered-workspace.tar.zst,
SHA256 3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd.
Fresh primary pre-format archive includes 9,946 source/Git-metadata paths:
R/p03-replacement-current-source-before-format-01.tar.gz, SHA256
e6ab98fe2bd9625c10fe67979eaf308b4f7e4ee7892548482ff0750d9847df5e.
These are verified **VM-local** checkpoints. GitHub durability covers only published paths.

## Publication and current source

Latest bounded WIP checkpoint:2ce48a627876b3a9a53e388f356df83e5ccff254,
tree8d201a7149663256c9ed8364bc85178a874f8aea, parent706bf9e.
All36 selected remote blobs and both refs were verified after a nonforce update.
Main remains unchanged. Receipt R/p03-featured-warmup-bc-publication-01/PUBLICATION.json,
SHA2566c7ad1c4de4ea3e03d2362052ca1594157c55cbd54a811d24d3abee9ab05771e.
It publishes23 B/C source paths, exact historical B/C test and provenance records,
and updated roadmap/coverage/maintenance ledgers. Current production CLI, affected
entrypoint suites and installed-storage/GUI gates remain pending; no P03 promotion.

Main remains 781080f7e3c8bfe1953378001d777dff33d74bc3 (native installer source
65511842d7051b2a1f5cc52917f3ebb5c03be4f3). Do not promote unfinished P03 work.
Prior Stage A WIP checkpoint:
706bf9eaf61cd05ccd9568bf88ba1112d635c547, tree
957d20a4896e46e224a144888ebdc5c371f38149, on
wip/p03-process-final-and-mcp-preservation-20261002.
This publishes the three-path Stage A library owner and exact552-test evidence;
no production activation or additional native extraction is claimed. All8 selected
remote blobs and both refs were verified. Receipt:
R/p03-featured-warmup-stage-a-publication-01/PUBLICATION.json SHA256
b75cb6f60d46eea678e1e0e4ef5f4df2346cf202116e33fc9de7a4492547a602.
Prior verification-only checkpoint7a811ea49f6ee7709b9556fbd9d3d3979f283a94,
tree493ce644eca801c5ede9055bd023160e2521a1ac, retains final65 native source
at7adb464cd19c2ddfeff71ccdf13839d26979197b, tree4e84d27238beb9efd839ea2336ede740aefe9a67.
Its23 remote blobs and both refs were verified; receipt:
R/p03-current-production-runtime-publication-02/PUBLICATION.json SHA256
d784b5d98498a3dfc31e34de11a1cb810645bf1ff81342977bb8413ca99ca9b3.
Prior current58 runtime checkpoint: f8a5905bb666846a7885171280d9454a9b8a269b.
Prior final65 source checkpoint: all58 selected remote blobs and both refs were verified. Receipt:
R/p03-replacement-checkpoint-publication/PUBLICATION.json, SHA256
b042a69343b43d641ddac0876013370e5d900b8ab844a1a51bdd78cb63d86b9b.

Last verified production P03 source: 65-path final manifest
R/p03-replacement-final-source.json, SHA256
bfad182c433bf88ec31b65d1e4d4234385b15be2f1e160645ab1bfff2ebb9040.
Full 8,928-file scoped map:
1c54717dd95ff85361d33076863129435f9516d3f815ff8690263fdda083549a.
Earlier formatted V2 (7d9b57fc…) corrects an inherited byte-equality flag in
preserved V1 (16f3d289…). Subsequent lint changes are recorded separately.
The post-format/lint production build and bounded installed-storage/GUI regressions
below passed on these exact bytes. The full P03 lifecycle gate remains open.
New offline support under upstream/ is outside this native/SDK scope.

**Current working source now differs from that verified production checkpoint.**
Featured warmup Stage A was adopted in three core-plugins paths (new
featured_warmup.rs, featured_warmup_tests.rs, and lib.rs export). It adds bounded
task custody and nine tests but has no production activation. Its initial formatted version passed551/551 scoped tests,0skipped; source unchanged.
That original run is preserved separately from the revised observer described below. Exact adoption receipt:
R/p03-featured-warmup-stage-a-adoption-01/ADOPTION.json SHA256
5cd3b12f12f5a630bc725409bae285cb99c8705a916aca3955be18418bb6f27c.
Pre-format68-path manifest:
R/p03-featured-warmup-stage-a-preformat-source.json SHA256
788fa91e219eeb8efc02ceb935295554a9c72748b675b6be647c42069faf1351.
Formatting completed0, changing only those three paths. Actual formatted68-path
manifest R/p03-featured-warmup-stage-a-source-01.json SHA256
fe4edc204f82ff263a3c4b90d8b033163da755efd2c8e78e764cd096b5179db4;
full8930-file map07e852fd1dfd829b6282e576465438a50aa7c0ffd12c6aca5c28822c8461422c.
Scoped just test -p codex-core-plugins --lib finished0 at
A/p03-featured-warmup-stage-a-tests-01:551passed/0skipped. Source unchanged;
strict0/null, adopted statuses0:36/128:2/-9:4 without causal attribution.
Corrected metadata-only manifest p03-featured-warmup-stage-a-source-02.json
SHA256 dff1828796a6bf1d708d3554254d0c9ec0f8c6f11a81a0e226a4aba2b16f8de2
retains identical file entries; the original started receipt's inherited scope
label is inaccurate, but its actual before/after file map is preserved.
A/p03-featured-warmup-stage-a-fix-01 exited0 unchanged, strict0/null; four warnings
included the private Tokio polling mutex held across await. Revision replaces
that data-free mutex with a private one-permit semaphore, preserving exact task
custody across canceled queued observers; its tenth test covers that boundary.
Small Copy observation now uses by-value completion check, terminal matches are
exhaustive, and public observation types can be named. New scope remains unactivated.
Actual revised68-path candidate: R/p03-featured-warmup-stage-a-poller-source-01.json
SHA256 4181a929a8a6742cc4b1f10349021e92ea9c1a9674aca66d5a2e1583cb34a884.
Revised just test -p codex-core-plugins --lib passed552/552,0skipped at
A/p03-featured-warmup-stage-a-poller-tests-01; full8930-file source unchanged,
map0895e8d7a3b08b0490031cca4fc293af4111c976451edf9ea306c43f10ecb46b.
Strict0/null; adopted0:36/128:2/-9:4 without attribution. Actual test ELF is
4bbb496936729974d07dd8f62b518e325227e8a447c6bd67eda6b94cb2826f1e,
219,815,664bytes, bound after testing; live rustc intent was not observed.
Revised candidate4181a929 inherited old map/format metadata. It is preserved;
corrected metadata-only R/p03-featured-warmup-stage-a-poller-source-02.json,
SHA256 c5bcf5bcb3d6b5acf5595e32bd404115b0b70b74e921dd991d7675af1f5cd83a,
keeps identical file entries and binds the actual test receipt. Source-wrapper
before/after maps, not inherited labels, identify each run.
Scoped revised Clippy passed0 unchanged; only the explicit unused process-close
warning remains pending StageB integration. No tests rerun solely for lint/fmt.
The original551 ELF and exact tests/source/strict receipts are stream-verified in
R/p03-featured-warmup-stage-a551-proof-preserved-01.tar.zst,46,370,724bytes,
SHA256 d610b4dce80dbdc00ccf99be1bfc1a1fc9a0a32791d4c0b78cdbbb747ae9250f.
Its original path remains until compiler reuse. This is VM-local preservation.
The old542 test ELF was preserved before compiler reuse in
R/p03-current542-coreplugins-proof-preserved-01.tar.zst (47,139,150 bytes), SHA256
671307822b180bc2ae18ad7035b48d41a4792e858e4babd8335343b833f88f77.
All six members and a full restoration were verified; the exact restored ELF in
/tmp/p03-current542-coreplugins-proof-restored-01 remains intact; the compiler
reused its former target path for the separately preserved551-test ELF. This archive is
VM-local, not external binary durability. Existing CLI/App Server proof binaries
remain the earlier source cohort. Stage B was subsequently adopted and passed its scoped tests, after its separate deadline correction; do not relabel final65 runtime evidence as proof of Stage A/B.

Stage A's verified three-path ownership support is checkpointed with its
[552-test evidence](verification/2026-10-02/P03_FEATURED_WARMUP_STAGE_A_EVIDENCE.md)
and [source-to-upstream map](upstream/p03-featured-warmup-stage-a-lineage.json).
The root independently rechecked all22 report inputs before adoption. New StageB
proposal p03-featured-warmup-stage-b-poller-rebase-01/MANIFEST.json SHA256
edf4ba044a485facbd89aa2ef7e57972c6a3cddba23b7981d3e819836281b4d9
is415 additions/56 removals across21 paths, with no owner/test replacement.
It was subsequently adopted in the working tree only, after Stage A publication.
Exact21-path/four-new-path receipt: R/p03-featured-warmup-stage-b-adoption-01/ADOPTION.json.
Pre-format73-path manifest R/p03-featured-warmup-stage-b-preformat-source-01.json
SHA256339b3241675e3fc0ed998d28f3d6cfc12cb6d0d8efd889c2ad202107b6c54933,
full8934-file map137545f222bc3a73945c2e596cbae42c92c9c23ebebd3084a71e56e5888b2aaa.
StageB replaces the actual detached manager warmup, owns processor/public-handle
close, and observes featured/curated owners separately under shared deadlines.
Formatting completed0 with nine intended StageB paths changed. Actual73-path
candidate R/p03-featured-warmup-stage-b-source-01.json SHA256
bd3cbf7566fbc0d14ab2bd39be89ee13aa0b4defb82c0307d10d1b556707a82a,
full8934map7cfc3ed5373bcd1529c404b2b844d1db34b21ccfd9f20da582146c473d9bc95a.
Initial scoped Core Plugins compile failed101 on an ambiguous assert_eq import
in the new nested test module; no tests executed. The failure/source remains at
A/p03-featured-warmup-stage-b-coreplugins-tests-01. A single explicit
pretty_assertions::assert_eq import repairs name resolution; assertions, deadlines
and native logic are unchanged. Retry02 passed553/553,0skipped under unchanged strict/source
wrappers; strict0/null, adopted0:36/128:2/-9:4 without attribution. Candidate R/p03-featured-warmup-stage-b-source-02.json
SHA25682739233f89ecf28af5013e9c2c99ad9f075954d506dfa328ab86f6d15cc77f1,
full8934mapf89b89ab6f42afe0dfbbe4b11f3fe862be9414783076696436b79da33ee18b25.
B scoped Clippy completed0 unchanged with no warnings. Exact test ELF04a4344e...
and its completed source receipt are bound in
R/p03-featured-warmup-stage-b553-test-binary-01.json.
StageB production/App Server/GUI gates remain pending; no old production
proof is relabelled. No dependencies, configuration schema, or wire format changed.

Pre-integration proof archives preserve the current552 test, earlier431 App Server
and final65 CLI bytes, all stream-verified with source/binary receipts. They retain
original paths/aliases and are VM-local; combined receipt:
R/p03-featured-warmup-pre-integration-proof-archives-01/ARCHIVE_RECEIPTS.json
SHA256 b725105b74c2a0ad18700597d8603d23c74e77456ec2b58a3c08d185f28029a4.
Only54 newly source-invalidated production intermediate files were retired after
independent root graph checks and exact idle/alias/source/proof guards, recovering
732,200,960allocatedbytes. Receipt:
R/p03-stage-b-invalidated-production54-cache-02/RETIREMENT.json.
This lends space to tests; production must regenerate these caches. All proof
executables, index, source and230 recovery archives remain retained. Do not repeat
this completed action or reuse its source guard after StageB adoption.

StageC test-only proposal was then adopted: three paths, native public-Drop caller
case plus separate SDK runner, preserving existing curated tests. Receipt:
R/p03-featured-warmup-stage-c-adoption-01/ADOPTION.json. Formatting completed0, changing only the new native test and Python runner; the
Python AST is unchanged. Exact75-path candidate:
R/p03-featured-warmup-stage-c-source-01.json SHA256
ffcc9711a3418d30c429e2b8bd14cc10a89dfc83a95119448ba936619322b99c,
full8936mapf05b8b9f19a7d8a9a78691970549c30fc0660b5c64769c03d94dc5292c523328.
Scoped just test -p codex-app-server --lib completed0,432passed/2skipped,
source unchanged, A/p03-featured-warmup-stage-c-appserver-tests-01. Strict0/null;
adopted statuses -13:4/-9:1/141:2/128:2/0:4, without causal attribution.
The actual compiler intent was observed. New512,205,032-byte ELF SHA256:
252dfaf2d3a0590d6d83dcdf7a8eeee327b56b0225a6bb23f6c816096429b264.
Root independently rechecked the full source map,75 selected paths, original
index, source/log/strict receipts and actual ELF before child execution.
Binding R/p03-featured-warmup-stage-c-root-runtime-binding-01.json SHA256
2440ace2fd42d77313a301be02f90ee0fa584261545264fd94fa29a61473611a.
The separate A/p03-featured-warmup-stage-c-caller-01 passed0 unchanged,
strict0/null, four real Git HTTP requests and no fixture errors/rescue.
A public Drop closed a held native featured HTTP peer within the five-second
start-to-EOF window, before any self-closing owner wait and before the unchanged
ten-second HTTP timeout. Its task was then joined with Cancelled outcome.
B restarted in the same home and used public shutdown successfully. A runtime
result was not joined; cache publication and whole-host cleanup remain unproved.
The unchanged curated pending/replay/race cases also all passed with this ELF:
A/p03-featured-warmup-stage-c-curated-replacement-01, source unchanged,
strict0/null, no fixture errors/rescue. These are embedded-client lifecycle tests,
not installed-component hot swapping or proof of MCP/HTTP-internal task custody.
Scoped App Server lint completed0 with unchanged source and zero warnings at
A/p03-featured-warmup-stage-c-appserver-fix-01. No test rerun solely for lint.

Before that App Server build, the exact inactive prior431 ELF was archive-verified
and fully restored at /tmp/p03-pre-featured-integration-as431-proof-restored-01.
Only its old compiler output path was retired under source/owner/alias guards,
recovering505,303,040allocatedbytes. All16 symlinks remained unchanged; the compiler has now reused the target path. Archive/proof bytes remain recoverable; this is not
a lifecycle pass. R/p03-stage-b-as431-proof-retirement-01/RETIREMENT.json SHA256
67b9ca9b2b950ced26d76088e625cb09fd3c1a29d09ded0681b7f73d33f985e7.
Never rerun that action after the compiler replaces its target pathname.

## Implemented versus proved

Installed native replacement remains bounded to thread storage/manual migration,
inline attachments (manager-level subset), and bounded file search. The separately
packaged GUI is additive presentation. Observations, callback fixtures and maintenance
scripts are **not additional extracted subsystems**. Most engine/auth/model/context/
tool/session families remain coupled; inspect every inventory row.

- Current58 production build and installed-storage/migration, ordinary/search GUI,
  slow normal/forced Launch shutdown passed on their recorded source. Real Chromium/
  Playwright used deterministic inference. Normal slow shutdown waited 46.47s; forced
  shutdown reported durability unknown. Earlier failures remain failures.
  [Recovery evidence](verification/2026-10-02/P03_RECOVERED_HOST_EVIDENCE.md).
- Stage1: just test -p codex-core-plugins --lib: **542 passed**;
  A/p03-observation-stage1-tests-01, map6f11c751….
- Stage2: just test -p codex-app-server --lib: **431 passed, one intentionally
  ignored parent-driven child**; A/p03-replacement-stage2-tests-01, map
  1d8ccc03b00f0e2a39573d96220f1695d2c1817d74ef846aa0e0a141a1946d4b.
  Strict 0/null; adopted statuses include SIGKILL/SIGPIPE/141 without attribution.
- New App Server library ELF: target/debug/deps/codex_app_server-4a5773d640636e8f,
  SHA256 26777e075411f0a4e7b6761d106aad0c365a2c342bc5ff74a8aa49adf2ca3291.
  Compiler output selection observed; output hashed after the suite. Old same-name
  ELF remains archived in R/p03-process-final-app-server431-proof-preserved.tar.zst.
- Replacement run01 failed with Tokio stack overflow before B. Preserve
  A/p03-replacement-stage3-runtime-01. Parent now sets RUST_MIN_STACK=8388608,
  matching just test; assertions/deadlines unchanged.
- Run02: **pending/replay/race all passed** against the new ELF.
  A/p03-replacement-stage3-runtime-02: A callback 0/B callback 1, real Git, updated typed
  metadata/skill/MCP behavior, exactly one v2 interrupt hook, native handle joined
  without quarantine. Strict 0/null; all 10 adopted statuses0; no forced fixture cleanup.
  MCP transport custody remains unproved and whole_host_clean remains false.
  This is same-home embedded-client replacement, not component hot swapping.
- Just fmt passed: 38 files, 34 Rust mechanically reviewed, four Python ASTs equal;
  60 frozen evidence files unchanged. No tests rerun solely for formatting.
  Scoped Core Plugins Clippy passed unchanged. App Server's first lint failed
  after removing a test-used import; the import moved into cfg(test), and the
  retry passed with two equivalent test-fixture fixes. Final formatting passed
  unchanged. Assertions/deadlines remain unchanged; no tests rerun solely for lint.
  [Lint transitions](verification/2026-10-02/P03_REPLACEMENT_LINT_EVIDENCE.json).
- Offline impact planner:17 in-memory fixtures plus22 existing lineage checks passed.
  Real local Git invocation returned expected unresolved exit2 for the same upstream
  revision and stale composition. No update integrated/approved/activated.
  [Tool scope](upstream/UPSTREAM_IMPACT_README.md).
- Final65 production build and installed storage/four migrations/ordinary and selected
  GUI/normal and forced slow Launch shutdown are now independently recorded in
  [current production runtime evidence](verification/2026-10-02/P03_CURRENT_PRODUCTION_RUNTIME_EVIDENCE.md).
  All89 private artifact hashes were reverified before adopting this sanitized report.
  Both earlier full held-host matrices remain failed, with exact inner/outer codes.

## Current production build — active

A/p03-featured-warmup-current-production-build-01 is running in session8032,
using source candidateffcc9711 /8936-file mapf05b and published WIP2ce48a.
Command: cargo build --locked -p codex-cli --bin codex -p codex-component-host
--bin codex-component. One compiler writer; TMPDIR is /tmp with the same prefix.
Never kill Rust or start another compiler/cache action while this is active.
The old final65 CLI was stream-verified and fully restored from existing archive
095169ae before only its two inactive original hardlinks were retired. After a
separate source/owner/archive guard, only the newly generated restoration scratch
was released; all older restorations,230 archives and16 symlinks were retained.
This recovers626,528,256 allocated overlay bytes. Exact action receipts are under
R/p03-stage-c-cli65-proof-preservation-01: RETIREMENT SHA256
4eae26d6f99f824c6ef386beec4553e7c76449543abf6dee11b99a56b941ca61;
RELEASE_RESTORE SHA2568ecba951bca6d561aad91352a3c5d23ede7b8f00cd991ba14776c6312631f34c.
Do not rerun either action after compiler path reuse. The final65 ELF now requires
restoration from its verified VM-local archive; it is not a new external backup.
Preflight free space:1,817,157,632 overlay bytes and1,359,732,736 temporary bytes.

The exact existing runtime fixtures and fresh names are planned under
R/p03-featured-warmup-current-production-runplan-01, manifest9ae4aa70….
Its print-only resolver requires an actual new successful production source/binary
binding. A separately reviewed additive held-matrix collector is still required:
the old parser discards featured-owner events, so its earlier success cannot prove
this new owner's receipts. Preserve all89 original assertions and deadlines.

## Ordered next actions

1. Current C App Server scoped lint and required formatting both completed0
   unchanged. Do not repeat tests solely for lint/style.
2. With one compiler writer, obtain changed CLI/exec/TUI scoped verification and
   a current-source production build. Review fresh headroom; preserve all proof
   executables before path reuse. Never rerun historical cache-retirement actions.
3. Run installed storage, all four migration variants, ordinary/selected-search GUI,
   normal/forced slow Launch shutdown and the held Git/HTTP matrix against the new
   production ELF, retaining independent package build/reuse fingerprints.
4. Publish this bounded milestone to the existing WIP branch with source lineage,
   exact tests and explicit limitations. Preserve separate MCP/HTTP proposals.
5. Resolve remaining HTTP construction/transport and MCP/callback ownership before
   P04; then continue all dependency-ordered extraction and P18U updater gates.

The following numbered record is historical final65 progress, not an instruction
 to repeat its completed builds/cache actions or treat its source as current.

## Prior final65 action history

1. Recheck production build headroom. Completed 54 invalidated-intermediate
   retirement recovered 732,139,520 allocated bytes; receipt
   R/p03-post-stage2-invalidated-production-cache-02/RETIREMENT.json,
   SHA256 87bf9bdbbca83b7265531b4b2efb440a3c167e18a8daf1ff9d3046fcc8b88ea0.
   Lint subsequently used some space. Preserve exact current58 CLI proof before
   compiler path reuse. Retire only exact proven-obsolete intermediates or
   separately archive/verify recoverable inactive artifacts. Never rerun completed
   355/138/54-cache or ten-snapshot actions. The additional 44 obsolete failed-TUI
   intermediates are now retired (465,612,800 allocated bytes); receipt
   R/p03-post-lint-invalidated-tui-cache-02/RETIREMENT.json, SHA256
   d8cd971b1e518f6f29664cbd3ade77abe5e9e1fe64a1ea250d050fe79ef8b8dc.
   The first CLI archive guard found 16 existing arg0 aliases and stopped before
   mutation. Revised archive-only guards pin those identities unchanged. The
   current58 CLI is now fully archived and restoration-verified at
   R/p03-current58-cli-proof-preserved-01.tar.zst (139,748,196 bytes), SHA256
   d5368ee56cd9b4a78e3ad60ca52463c44c6d509aadb47cc367aa8d84eb9b2251.
   Separate retirement03 rechecked inactive ownership and retained all 16 symlinks;
   only the two original hardlinks were removed. Receipt SHA256
   1c9a90a85f73eb3ad6cedfa8717f0368317c8dcfc3e15195be759f315cb82405,
   R/p03-current58-cli-proof-preservation-03/RETIREMENT.json. Exact restored proof
   remains in /tmp/p03-current58-cli-proof-restored-01; do not delete it casually.
   All GUI/session homes remain intact. Resource summary before retirement:
   R/p03-replacement-resource-checkpoint-01.json. This archive is VM-local only.
   Production build started with 1,958,793,216 free overlay bytes; preflight
   R/p03-replacement-production-preflight-01.json. Do not rerun completed actions.
2. **Production build passed**: completed session 41614, prefix
   A/p03-replacement-production-build-01, final65 bfad182c / map1c54717d.
   Command: cargo build --locked -p codex-cli --bin codex -p codex-component-host
   --bin codex-component. Source wrapper records actual before/after; one compiler
   writer and TMPDIR=/tmp/p03-replacement-production-build-01. Never kill Rust.
   Build finished 10:40:52 UTC, exit0, unchanged full scope, 6m22s; no additional
   OOM kills. Source receipt SHA256
   14e91233defe092c937970956a6bdb11a0670566001d388ba8428a4b66bd6658.
   Binary receipt SHA256 cd588d2b27200e0553f8cb8e236c5c2e25cc8e94ded2dff4ecf32bd883133041.
   New CLI dc7f1e84d5bcb311fdc7751b1538319f6205a892203edfc74c2993103f18f158;
   manager remains cached unchanged f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954.
   CLI compiler intent was observed and output hashed after success; top-level CLI
   and exact selected deps output are the same inode. Runtime acceptance is separate.
   **Fresh eight-case matrix failed**: completed session57902, prefix
   A/p03-replacement-held-host-01, wrapper1 with unchanged source. First two CLI
   cases passed; third git-app-server-eof failed fixture_protocol_failed with
   auxiliary_request_after_stop_trigger. Host exited0 in about0.033s; no forced
   fixture cleanup. Five cases were not run. Investigate auxiliary request ordering
   without weakening fixture assertions; this is not a full matrix pass. The gate needs a **complete**
   eight-case held Git/HTTP matrix with positive independent Git acknowledgment.
   Current58 attempt03 stays **4/8 accepted, fifth failed, three unrun**.
   Corrected fixture: R/p03-held-host-observer-ack-proposal-01, manifest14c27cff…;
   Its 15 pure checks passed before the fresh failed native matrix. Do not combine partial runs.
   Prepared runplan: R/p03-replacement-held-host-runplan-01, manifest456032cc…;
   requires the new successful production receipt and actual CLI fingerprint.
3. Run current-production storage/migration and GUI/session/cancellation/recovery/
   actual Launch Ctrl+C regressions with exact independent package-build/reuse proof.
   Older current58 results and build success cannot substitute for these gates.
   Independent storage gate passed while the failed matrix is diagnosed:
   A/p03-replacement-newhost-storage-01, original package proofab41bc4a…,
   new CLI dc7f1e84… and unchanged managerf054d84a…. Thirteen behavior checks and
   all three storage child terminations passed; strict0/null, adopted -9:8/0:1.
   This is package reuse, not a new independent Rust build. Fresh normal migration
   passed ten commands, strict0/null, adopted -9:6/0:1. Ordinary GUI passed two
   Chromium/Playwright cold cycles, approvals/streaming/Stop/search/actual Launch
   Ctrl+C, no page errors and unchanged packages/host; strict0/null, adopted0:1.
   Prefixes: A/p03-replacement-newhost-{storage,migration-normal,gui-normal}-01.
   Do not report all adopted statuses zero for storage/migration.
   Root viewed gui-normal recovered-2.png; in-app Browser was unavailable.
   Selected-search migration and two selected-search GUI cold cycles passed with
   prefixes A/p03-replacement-newhost-{migration-search,gui-search}-01. The actual
   App Server used the independently built native.file-search-local package; the
   original external build proof is reused, not rerun. Manager shutdowns took
   about0.27s; tracked processes absent, binaries/packages unchanged. Root viewed
   search-results-2.png from that actual Chromium run.
   Fresh migrations for both slow-storage fixtures passed ten commands each;
   prefixes A/p03-replacement-newhost-migration-slow-{normal,forced}-01. Each strict
   runner returned0/null with adopted -9:6/0:1. Actual Launch normal shutdown
   waited46.317s, exited0, confirmed native cleanup and cold recovered the held
   canonical user-message event. Forced shutdown exited1 after2.042s, explicitly
   reported durability unknown, and cold recovery correctly lacked that unforwarded
   event; subsequent operation and normal shutdown passed. Prefixes
   A/p03-replacement-newhost-slow-{normal,forced}-01; strict0/null with adopted0:6
   each. All tracked processes absent. Admission was held in the installed relay
   before native forwarding; this does not prove the native internal admission
   budget or fix response-only interrupted-history projection.
   New test homes use
   /tmp/p03-replacement-newhost-*-01-runtime in this same VM to conserve overlay;
   wrapper/strict receipts remain in A. These are test homes, not another checkout.
   Connection-boundary diagnostics preserve all89 original fixture assertions.
   A/p03-held-connection-observations-policy-01 passed23 pure checks. A separately
   fingerprinted single EOF diagnostic completed0 on the same native bytes;
   independent observer, unchanged strict runner and case checks also passed.
   Its auxiliary connection was already held before shutdown, so this does not
   reproduce or close the original race. Full diagnostic matrix
   A/p03-replacement-held-diagnostic-matrix-01 completed1: first two accepted,
   third EOF rejected by the outer frozen_observer_real_git_missing check despite
   inner-case success; five unrun. The independent acknowledgment currently covers
   fallback mode only; an R-only proposal will require it in Git mode too, retaining
   all deadlines and rejection assertions. Review that correction before the next
   native matrix; do not repeat the unchanged failing setup. Instrumentation adds observations,
   not a native semantic fix; retain both failed matrices and do not infer
   startup-race closure from a later pass. The source audit separately found an
   unowned featured-cache warmup task; its scoped ownership proposal is R-only,
   unadopted and untested, and the encrypted auxiliary request is unattributed.
   **Fresh complete matrix passed after the separate readiness correction**:
   A/p03-held-all-mode-ack-policy-01 passed31 fixture checks; then
   A/p03-replacement-held-all-mode-ack-01 passed all8 cases in one unchanged-source
   run. Fixtureac8ae90e… / observer11e5c906… require independent exact-live-Git
   acknowledgment in both modes before held readiness, using the same60s budget.
   All original89 assertions and strict5s drain remain. Each case retained its
   expected host status, reported connection EOF, and used no fixture rescue;
   each strict runner returned0/null. Git adopted codes -9:2/0:1 per case;
   fallback adopted128:1/0:1. ACK generation is bound to the independent observer,
   not necessarily the inner sampler or one exact Git operation/transport owner.
   All auxiliary holds preceded stop in this run. The original early-auxiliary
   race remains unclosed; no native fix or whole-host/MCP custody is inferred.
   [Complete matrix evidence](verification/2026-10-02/P03_CURRENT_HELD_MATRIX_EVIDENCE.md)
   and frozen fixture sources retain exact31-check/eight-case fingerprints.
   Preserve the failed matrices and separate successful single diagnostic.
   Before the next scoped build, only82 proven source-invalidated failed-TUI
   intermediate files were retired, recovering665,604,096 allocated bytes.
   R/p03-post-new-production-invalidated-tui82-cache-01/RETIREMENT.json SHA256
   d2e7b28b122312078ad7f73e3dcef50443871c3e9bf40f227e4bc963b2578ea0.
   Root independently verified the conservative1783-node protected graph and all
   invalidation edges. Current proof executables, index,225 archives, both source
   worktrees and old restored CLI are retained. Do not rerun this completed action.
4. Close MCP custody: staged upper manager ownership → successful-owner retirement →
   lower transport ownership → pinned SDK → aggregate late-admission fence.
   These proposals remain unadopted/uncompiled. C2b archive cancellation remains
   staged; transactional repo/SHA publication and host-death fencing are still needed.
5. Resume installed native auth/catalog and all dependency-ordered P04–P19 work.
   Keep source/component provenance and kernel exceptions explicit.
6. P18U: independently installed maintenance worker and external recovery bootstrap,
   real later upstream integration preserving custom plugins/UI, coordinated versions/
   migrations, incompatible-update rejection and failed-update rollback. No polling
   or unattended live deployment. Remote viewer networking remains separate/optional.

## Resume constraints

Read root AGENTS.md, verify source/process ownership/disk/cgroup headroom/remote refs.
Use /workspace/toolchains/component-verification-env.sh, one compiler/cache writer,
and source/strict wrappers. Use just test and scoped packages; the complete workspace
suite still requires separate user approval. Never kill Rust. Do not dump auth/config,
credentials, raw runtime reports or environment values. Verified milestone publication
is authorized; force push and unrelated settings/account changes are not.
In-app Browser and Context7 are unavailable here; do not claim their checks ran.
Label real Chromium fallback and deterministic inference. Existing authenticated
remote viewer connectivity remains separately blocked.
