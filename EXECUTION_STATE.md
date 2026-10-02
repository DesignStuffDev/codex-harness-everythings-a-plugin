# Codex Harness Compartmentalized — execution state

Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required update track: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
**Full v1, installed auth/catalog and the upstream updater remain incomplete.**

## Resume identity and publication

Original cloud checkout: `/workspace/codex-harness-everythings-a-plugin`;
preserve `/workspace/codex-harness-next-components` and both worktrees' changes.
Origin: `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
Upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`; exact-tree import
`ae720ae9a98bad29ca2cff998e7d5baaf05cec86`. Retain LICENSE/NOTICE.

Last verified main: `781080f7e3c8bfe1953378001d777dff33d74bc3`, tree
`22cf918cf77f16d5d947a59b968e342cde7f71d0`. Main source remains native installer
`65511842d7051b2a1f5cc52917f3ebb5c03be4f3`; later main changes are documentation.
The previous candidate publication is `a69fa42ffd83a134624842e7649502abd30455b1`;
receipt `p03-curated-http-await-publication.json` verifies both refs and thirteen changed blobs.

**Active development source**, present in the owning checkout:
`92212516ad4d12bcf60546ee5f879b983ed44682`, tree
`746d93dcdc89de6882dba313d4ad3626d33b0ad0`, parent candidate `8d1216a3`.
This propagates the exact stop control through policy-aware HTTP send/body reads, with one
request deadline, observed byte caps and existing charset/BOM decoding. Ordinary client/runtime
teardown finishes on the retained native worker before later stages. Its destination remains
`work/p03-curated-sync-lifecycle`, **not main**.
Before continuing, verify `p03-source922-runtime-publication.json` and the remote branch;
this file cannot contain its own eventual documentation commit SHA. Source922 now has scoped
and fresh full-host regression proof, while its shutdown contract remains incomplete. C2b
bounded extraction is staged separately; the original extractor is unchanged.

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

A frozen completion proposal is preserved at recovery/p03-curated-worker-completion-proposal;
manifest `f5d541d283275b90d384aaeba183e2a554c54a251cd48a2ea1eb4b8e644ef8db`,
patch `d4df562e2b6ccd447cfebf70282e3e653538a0e1dfb0e0245bf1e30976ed44aa`.
Six files/451 changed lines/seven new tests; exact replay and two static reviews passed.
It is not adopted, compiled or tested at this evidence checkpoint. Callback scope work is
being staged separately against that frozen proposal. Preserve both; do not assume adoption.
The source922 runtime runplan and held-startup shutdown test plan are in recovery, with
strict arguments/default startup retained and private report projection rules.

## Ordered implementation queue

1. Verify source922 publication/ref and exact-source evidence. Keep main source655 until
   combined full-host gates pass; preserve the superseded C2a proposal and pending C2b separately.
2. B1a retained ownership, B1b shared cancellation and C2a HTTP awaits are scoped-tested.
   Preserve sticky quarantine and original outcomes; direct-child reaping never clears them.
   External termination/fencing is required for recovery; restarting alone is insufficient.
3. The latest user request prioritized the fresh source922 runtime gate; it now passes as
   recorded above. Next preserve the new CLI/test evidence, then adopt/test the reviewed exact
   native-worker completion slice. Add owned curated async callbacks and real process-final
   callers as one host-repair acceptance, preserving per-embedded replacement/sibling scopes.
   A completion primitive alone is preparatory. Pending/cancelled observers retain custody;
   native finalization may outlast observation deadlines, so external watchdog/uncertainty
   policy must remain explicit. Never turn !pending or a joined uncertain outcome into success.
4. Run deliberately held production Git/HTTPS with exec success/error and App Server EOF/SIGTERM,
   then repeat unchanged installed storage, migration, GUI/session/streaming/approval/cancel/
   recovery/actual Launch Ctrl+C gates on the actual repaired source. Coordinate the45s stdio
   watchdog with longer storage/GUI budgets and exercise slow cleanup, forced uncertainty and
   embedded replacement. Only then consider nonforce main promotion. Preserve the separately
   frozen C2b archive-extraction proposal (`d76bf2389695619fd051ffd24758ba43346e51977dc7115c98d568e863d7ef09`),
   still unadopted/untested, and implement its cooperative limits as a separate slice. Complete
   recoverable repository/SHA journal and host-death fencing; no total ZIP-constructor memory
   bound or hard native-blocking deadline is implied by cooperative checks. Follow the
   [shutdown audit](verification/2026-10-01/P03_CURATED_SHUTDOWN_INTEGRATION_AUDIT.md).
5. Resume native auth load/refresh/persistence authority, independently installed auth/catalog,
   then every remaining P04–P19 subsystem and kernel audit. This lifecycle repair is prerequisite
   work, not a substitute for extracting actual components.
6. P00M provenance closure continues now. P18U must deliver separately installed maintenance,
   isolated real later-upstream integration preserving custom packages, semantic/security review,
   coordinated host/plugin/schema versions and external bootstrap recovery/rollback. Updater
   remains unimplemented; no periodic polling or live update follows from the roadmap addition.
   Remote viewer transport is optional and independent of extraction.

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

The inactive source99 CLI c711 and its2mutable hardlinks were retired **after** full archive
and reference verification to recover625,934,336 allocated bytes. Archive
`p03-source-owner-cli-archive-preserved.tar.zst` SHA256
`68102a992d8d7ebe3105933d5377c9ec215ed5d51c8f814891555f81fb098222`;
retirement receipt `p03-source-owner-cli-recoverable-retirement.json` SHA256
`4e08e878403129680208a6fac8c0e52c194f8bfad2353d247c03b1c0a3e9f0ba`.
Eight historical mutable-target symlinks remain unchanged and temporarily dangle; restore exact
archived bytes/metadata or rebuild before using them. Original GUI PID371365 still uses the
separate frozen `/workspace/verified-component-checkpoint-v2-20260930/codex`; leave it intact.
Three inactive installer proof ELFs are verified in `p03-install-order-green-test-executables.tar.zst`
SHA256 `f289ec637ffccf8d400b335daa6c811b282162689fe58ca36acd47b2b8084fb8`; receipt
`p03-install-order-green-elf-preservation.json` SHA256
`d41c1028af19baa890f0fce69f137cdf43dd8f90457b7a2b68e03a984ed64a8a`.
This binds preformat400-test source separately from formatted source655. Their3build aliases
were retired after verification; Cargo metadata/source and current transport tests remain.

No Rust/runtime command is active at this checkpoint. Before commands verify current disk/cgroup
headroom, not a historical estimate. `/tmp` is nearly full. Source
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
