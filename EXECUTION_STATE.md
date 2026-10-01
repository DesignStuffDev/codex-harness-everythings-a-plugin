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

Last verified main before this documentation supplement: `43a758138c457569a33ccd14f01446d715eb54e0`,
tree `894c09d7398209cfd3846ec87d6a3188c062568f`, including native installer source
`65511842d7051b2a1f5cc52917f3ebb5c03be4f3`. Twelve changed blobs and main were verified;
receipt `p03-install-order-publication.json`, SHA256
`315aee4b63943e621f35a23b9359365e1f3199381088fbd2ce608d993b9634b3`.

**Active development candidate**, present in the owning checkout: source
`7ab5dd77b87c2e6bf7040824e67bf6f22af6a073`, tree
`1f650c46fdca056ec7cfd333a65c4defc4f9d066`, parent `43a75813`.
Its source is assigned to `work/p03-curated-sync-lifecycle`, **not promoted to main**.
Main receives roadmap/evidence updates only. Before continuing, verify the final
`p03-curated-git-publication.json` recovery receipt and both remote refs; this document
cannot contain its own eventual publication SHA. A candidate's scoped proof does not
validate the source of a different branch or establish release readiness.

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
outside the parent's process-group guard. The native pipeline still uses String failures,
can fall back/drop locks or staging on uncertain cleanup, swallows HEAD errors and resets
process-global admission on errors. No host cancellation/owned-worker/full-CLI/GUI result for
source7ab5 exists. The [required ownership review](verification/2026-10-01/P03_CURATED_B1_OWNERSHIP_REVIEW.md)
changes the next stage ordering; enqueueing a child to the shared reaper is not a completion receipt.

[Prior full-host checkpoint](verification/2026-10-01/P03_SOURCE_OWNER_FULL_HOST_EVIDENCE.md),
source99/CLI c711, remains **mixed/failed**: both storage attempts passed13 behavior checks
but strict125 found live descendants; migration10 and GUI2cold cycles passed. Actual manager
active-turn Ctrl+C exited0 in0.265/0.267s without forced cleanup. Chromium/Playwright used
deterministic inference, not in-app Browser/live-provider or new attachment proof. Last older
fully passing runtime is `b3530790` (source181400/CLI876c); never relabel it for newer source.

## Ordered implementation queue

1. Verify this candidate branch and main documentation publication receipts/refs. Continue
   source7ab5 on the candidate; keep main source655 until the combined lifecycle gates pass.
2. Implement B1a as a coherent ownership transition: typed command cleanup outcome **plus**
   retained attempt/admission generation integrated into the real pipeline and manager. Unknown
   cleanup retains lock/staging/Git resources, forbids fallback/retry, and survives String
   formatting or observer drop. Pull necessary retained worker/C1 ownership ahead of cancellation.
   The existing shared reaper lacks completion acknowledgment. Measure a reviewable patch; split
   tests/mechanical moves first if necessary, not an unsafe halfway production transition.
3. Then add B1b cancellable stable file-lock waiting and same-control Git propagation; preserve
   ordinary safe fallback. Continue policy-aware HTTP send/body cancellation, chunked extraction,
   recoverable repo+SHA publication/journal, tracked callbacks/queued config work, and actual
   in-process/external App Server/CLI shutdown with honest forced-termination uncertainty.
4. Build a fresh full CLI and rerun unchanged default-feature installed storage, migration and
   GUI/session/streaming/approvals/cancellation/recovery/actual Launch Ctrl+C gates. Record exact
   source and package hashes, process identities and strict runner result. Only then consider
   nonforce promotion of the combined source; do not disable plugins or extend drain assertions.
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
Candidate source archive `p03-curated-git-verified-native-source.tar.gz` SHA256
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

Original task environment/config identity is unchanged, revision1458 observed running/connected;
restricted package-managers policy still has additional allowed_hosts[]. In-app Browser and
Context7 are unavailable, and no external preview is proven. Keep the original VM; no reset or
policy bypass. Upstream libraries/framework docs are required when applicable, not for ordinary
native refactoring. Viewer networking does not block this queue.
