# Execution state — read first on resume

Updated 2026-10-01 UTC. Follow [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md) and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md). Full v1 is **not complete**.

## Last verified checkpoints

- Latest externally preserved selected-search source: `651b0b87a281934e5cd7c639021049fad8d393ac`,
  tree `bb7e7996a14d14eaf11e00239eabe628916a77f2`, parent `cb977e7d`, on
  `wip/p02b-bounded-search-20261001`. Its 57-path delta preserves the real native
  worker, runtime facade, selected standalone CLI and external acceptance tools.
  GitHub's runtime `main.rs` blob is `b94364e86ccd57970e43c7babe1e7cb0aad6a2fe`.
  Combined regression passed 222 cases; separately built/installed search passed
  24 real CLI/manager commands with expected statuses and unchanged host hashes.
  App Server/TUI/GUI adoption remains pending; main's source is unchanged.

- Latest externally preserved integration WIP: `cb977e7d664bc752c28854d468619039bd3ad167`,
  tree `40207f354efb3bee85265d8f3d83312489b56709`, parent `bb03f3a8`, on
  `wip/p02b-bounded-search-20261001`. Its 14-path delta preserves the real native
  backend/output limits and corrected/formatted client fixture. GitHub's
  `native_backend.rs` blob is `98a82dd1196735ade842c8cb09345978c68be5cd`.
  Native 141/141 and the separate earlier client 40/40 gates are described below;
  mechanical post-test transitions remain explicit. This is not a new host/GUI
  acceptance or independently installed search proof. Later worker integration
  is outside this WIP. The later local correction and its evidence are described
  below; preserve their separate source identities.

- Prior integration WIP: `bb03f3a81a828871e4e35d3e46b9c8528f3b7d00`,
  tree `6ad17d58936b6c9f519c06f2932efd1c8ecd4724`, branch
  `wip/p02b-bounded-search-20261001`, based on accepted main
  `59e5d576681d6cbb4977e9ccfaf5fe22bbeb2f15`. The remote ref was verified;
  GitHub's `native_budget.rs` blob is `fcc7eda119e5599e289697b5d3f590520dc21a0b`.
  Native budget/owner, search process/service and host cwd/catalog gates passed
  in separate runs described below. Later client evidence and its fixture correction
  are preserved separately; neither this whole WIP nor a new CLI/GUI is accepted.

- Earlier externally preserved WIP: `98c540863b9868eb216dba18f963c220c4be5187`,
  tree `d193d69db07c0ca378a9f95dae27d6dcee9d6019`, branch
  `wip/p02b-bounded-search-20261001` (now advanced to the WIP above).
  Bounded index primitives and opt-in process payload limits passed 166 focused
  cases with one ignored helper;
  all scoped test-source hashes were unchanged and the subreaper completed.
  Lint/format transitions are separately recorded. This is not an accepted new
  CLI/GUI build, native budget enforcement or installed search replacement.

- Pinned matcher correction source: `ac5fdae1b7db8134318891eeb991b9c330e6ea8b`,
  tree `d59a58799acde091d508b99e27bdf7fa3ae74564`. Safe pre-fix raw-metadata
  regression failed as expected; vendor37 cases passed after the correction.
  [Evidence](verification/2026-10-01/P02B_MATCHER_FIX_EVIDENCE.md) separates the
  newer WIP native41 cases and makes no new CLI/GUI runtime claim.

- New support-library source: `98eeb3e1c93b3a856834c57e6c4d57bfa9f8c782`,
  tree `01e5f25c3a40cd90ea4cae430e481a3e40cc9e65`: neutral search API and shared
  path codec with old storage exports; [56/56 focused tests](verification/2026-10-01/P02B_CONTRACT_EVIDENCE.md),
  scoped lint/format and Bazel graph gates passed. No new host/runtime claim.
  StageA below remains the latest complete new-engine/storage/GUI runtime gate.

- P02 StageA source: `25b2c3150879761026086a62e480045fc38d32ff`, tree
  `5ba385b6e7730119b6d5cdca67fba2dc4f5c395d`; its companion documentation commit
  publishes this state and evidence. Revalidate the current remote ref.
- P01 source: `5d2f2a026ae6ce0c42cf56bb2f3b01971e4bf0bb`.
  Native thread persistence and optional manual migration are independently selected;
  inline attachment storage is a separately replaceable native subset.
- StageA fixes native search worker/publication lifetime in App Server/TUI and adds
  file references to the independently packaged desktop GUI. **Search is still
  coupled.** Infrastructure, lifecycle repairs and additive UI do not count as its
  extraction. Model/auth remain adapters; loop, context, compaction, native tools,
  policy and auxiliary databases remain coupled.
- Historical unregistered StageB API/path-codec drafts: external WIP
  `17c366bb94f77b3d3895acd6f061a5cb308bd384`, branch
  `wip/p02-search-lifecycle-20261001`. They were excluded from StageA and are now
  integrated in the support-library checkpoint above.
- Older isolated WIP: `0b38d5974159ab2a8776200025dbb602e89b8f8f`, branch
  `wip/recovered-next-components-20260930`. Never merge WIP wholesale.
- Official upstream pin: `d42056091aded7feb1d88ac7e83972108b2aa478`; Apache LICENSE
  and NOTICE retained. Nucleo's separate pinned MPL-2.0 source/notices are retained.
  See [exact StageA lineage](upstream/p02-search-lineage.json).

## Verified behavior and limits

Latest P02B source gates, **separate runs and scopes**:

- [Actual native backend](verification/2026-10-01/P02B_NATIVE_BACKEND_EVIDENCE.md):
  141/141 passed, 0 skipped: native 75, neutral API 11, Nucleo 23, Matcher 32.
  All 82 scoped fingerprints were unchanged, test/subreaper exit 0. The native
  cases include 14 backend and five output tests plus the prior 56; actual
  traversal/matching, retained poll/lease ownership, aggregate resources and typed
  cleanup are exercised. Initial token-import compilation failure is preserved.
  Subsequent reviewed clone/condition/import edits and formatting are recorded;
  all final bindings match WIP `cb977e7d`. This does not prove installed selection
  or eliminate the later worker error race described below.
- [Native budget and owner](verification/2026-10-01/P02B_NATIVE_BUDGET_EVIDENCE.md):
  122/122 passed, 0 skipped: native 56, neutral API 11, Nucleo 23, Matcher 32.
  All 74 scoped source fingerprints were unchanged; test/subreaper exit 0.
  Real per-session entry/index-byte/worker enforcement and typed retained cleanup
  now exist. The original 32-diagnostic test compilation failure is preserved.
  Later documented lint expectations and scoped/vendor/global formatting have
  their own hashes; no post-format rerun is invented.
- [Search service/process adapter](verification/2026-10-01/P02B_SEARCH_PROCESS_EVIDENCE.md):
  22/22 passed, 0 skipped: six controlled-backend service, six codec and ten real
  protocol-peer process cases. All 95 scoped source fingerprints were unchanged;
  test/subreaper exit 0. These processes are fixtures, not installed native search
  workers. The original five-diagnostic compile failure is preserved. Subsequent
  malformed-identity hardening compiled but its new branch was not exercised by
  that passing run; mechanical/lint/format transitions are recorded separately.
- The separate host cwd/catalog run passed 80/80, with one ignored parent-death
  helper explicitly invoked by its passing parent test. Runner exit 0; code was
  unchanged while `Cargo.lock` changed during dependency resolution. The manifest
  honestly records this drift. These are not an aggregate 102-case search run.
- Initial high-level client regression `p02b-client-shutdown` failed compiling
  `codex-core` with ENOSPC before any tests executed; command/subreaper exit 101,
  scoped source unchanged. Its original log/source/subreaper reports remain in
  `/workspace/acceptance/`. The separately named `p02b-client-shutdown-retry`
  finished at 05:31:22 UTC: **40 executed, 39 passed, 1 failed, 0 skipped**,
  exit 100, scoped source unchanged. All 12 new shutdown cases passed. Existing
  `next_event_surfaces_lagged_markers` failed on both attempts because its fixture
  dropped the command receiver but expected successful shutdown; the new client
  reported `BrokenPipe` and unconfirmed runtime/storage cleanup. The fixture-only
  correction retained the receiver and acknowledged the real shutdown command,
  preserving existing assertions. The separately recorded corrected run passed
  **40/40, 0 skipped**, exit 0, with all 8,680 source fingerprints unchanged;
  its only pre-run change was eight added/two removed fixture lines in `lib.rs`.
  [Client shutdown evidence](verification/2026-10-01/P02B_CLIENT_SHUTDOWN_EVIDENCE.md)
  binds those actual tested bytes, which are newer than WIP `bb03f3a8`. Scoped fix
  passed with unchanged scoped source; final format passed and only wrapped the
  fixture acknowledgement call. Its hashes remain separate from the tested bytes.
  The corrected/formatted fixture is now published in WIP `cb977e7d`. Preserve both
  failed runs; do not relabel the earlier retry as all green. The client gate ran
  before native integration; it is no new native, GUI or Launch validation.

The later worker, runtime facade and selected standalone CLI are applied outside
WIP `cb977e7d`. [Selected search evidence](verification/2026-10-01/P02B_SELECTED_SEARCH_EVIDENCE.md)
preserves original compiler, configuration and lifecycle failures. A deterministic
native callback gate reproduced the first-error/close race; retaining its close
receipt preserves the real cause. The corrected native/worker gate passed 178/178;
four relevant cases then passed 50 stress iterations each, with no retries.

The CLI now selects `file_search/default` through the real component manager,
preserves native results and lexical roots, rejects selected-provider failure
without fallback, and awaits cleanup before output. The combined later gate
`p02b-selected-startup-regression` passed **222/222, zero skipped, no retries**,
with all 203 scoped source fingerprints unchanged and subreaper exit zero.
This includes explicit-startup-cancellation classification: only a prior shutdown
fence plus known clean `ClosedLease` becomes a clean cancellation; real errors
and cleanup uncertainty remain failures. Scoped lint, formatting and the locked
CLI build have separate source manifests and do not imply tests were rerun after
formatting.

The first independently built external worker passed five real CLI/native parity
cases, installation and selection. Its Ctrl+C gate correctly failed on status 1
instead of 130; the startup cancellation correction above addresses that cause.
Frozen corrected CLI is at
`/workspace/component-checkpoint-candidate-p02b-search-cli-20261001-02/`, SHA256
`206a4d86a661953e9a5f6fe38576bae6ea7d724fd9f84d250d6e14bb3391bacd`.
External rebuild02 failed before compilation because Cargo reordered duplicate
unused patch receipts and wanted to rewrite its copied lockfile. Package records,
versions and checksums were unchanged. Corrected rebuild03 removes only proven
unused pinned Git patches in its isolated export, checks identical full package
records/metadata, and builds with an unchanged locked dependency graph. All seven
build/export/package commands passed with a fresh external target. The real
CLI-independent02 acceptance passed all 24 commands: five exact ordered native/
installed result comparisons, clean CLI-only Ctrl+C exit 130, configuration failure
without fallback, no-pattern bypass and removal restoring native behavior. Both
host hashes stayed unchanged; all tracked PIDs were absent and both outer
subreapers exited zero. Preserve the original failed runs. This is standalone
native-search replacement proof, not new App Server/TUI/GUI proof.
The working workspace has **169 explicit Rust members**, adding
`file-search-component`, `file-search-local-plugin` and `file-search-runtime`;
the accepted inventory ledger remains **166**. Update that ledger with an
appropriate accepted component checkpoint, preserving the distinction.

[P02 evidence](verification/2026-10-01/P02_SEARCH_PREREQUISITE_EVIDENCE.md) and its
machine-readable reports retain the last accepted full new-engine runtime gate,
with actual per-run source/log hashes and failures:

- Native search 35/35, vendored matcher/Nucleo 35/35; App Server 395 library cases
  plus 13 public RPC cases across separate 12+1 runs; focused TUI reconnect 11/11.
- Full TUI: 5,604 passed, four cursor-color failures, four ignored. The same
  executable's six unchanged cursor tests passed with NO_COLOR unset. Together
  these establish 5,608 distinct passing executed cases across environments,
  **not one all-green run**. Original failures/snapshots remain preserved.
- SDK packaging 19/19, desktop controller/gateway 7 Node + 14 Python cases.
  Scoped fix, format, final Clippy and CLI build passed; eight vendor style
  warnings remain. Build scope recorded 2,979 unchanged source fingerprints.
- The new engine ran the existing independently built storage 0.2.0 package
  unchanged: real turns, tools, streaming, persistence, cold resume, restoration,
  manual migration dry-run/apply/idempotence and strict child cleanup passed.
  This proves compatibility, **not a new independent storage build**.
- New-engine GUI: 49+38 Chromium/Playwright commands, zero page errors, real file
  search/insertion, approval/native tool completion, streaming, Stop, reload and
  cold recovery. Manager Launch SIGINT during active turns exited zero in about
  0.315/0.316s; all observed processes absent and subreaper drains passed.
- In-app Browser is unavailable. This is real Chromium fallback evidence with a
  deterministic provider, not live-provider, Windows, remote-viewer or full-suite
  proof. The older P01-engine GUI gate remains separately identified.

Frozen StageA runtime: `/workspace/component-checkpoint-candidate-p02a-20261001/`.
CLI SHA256 `181463925d599f0eed023106820cad8fbb228d81e972eefb256a2076b88813c0`;
manager SHA256 `acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3`.
Original candidate.json records creation-time pending status; later evidence records
passed acceptance. Do not overwrite it or older frozen P01/v2 artifacts. Stripping
preserved all 30 allocated ELF sections; runtime gates used this immutable copy.

## Ordered next actions

1. Re-observe original task environment, repository, actual working files, refs,
   source/index hashes, processes and disk/memory. Read AGENTS and runtime skill.
2. Preserve the successful standalone source WIP `651b0b87`, frozen CLI02 and
   external build03/runtime02 artifacts. Complete the evidence/main-doc checkpoint
   and exact upstream lineage; do not overwrite original failed attempts.
3. Preserve the separate passing native T12 allocation-floor/traversal profile
   check (`p02b-interactive-native-profile`): one test with eight parameter rows,
   78 unrelated cases filtered. At 100k/T12 with highlights, fixed charged storage
   is 11,069,744 bytes and 26 workers; 128 MiB remains the allocation ceiling.
   This is a small real fixture, not full-capacity/RSS proof. Its test-only source
   is newer than WIP `651b0b87`; keep lint/format transitions separate.
4. Review/apply staged App Server, client and TUI integration. Measure the actual
   T12 native allocation floor before adopting the interactive policy. Preserve
   independent connection/picker scope ownership and provider guards outside
   abortable tasks; drain search and storage on normal launcher shutdown. Add
   synchronous shutdown intent before provider fencing so normal shutdown cannot
   emit a false search failure; deliver genuine terminal failures reliably. Add
   observable per-request pending-start cancellation before claiming that gate.
   Regenerate/test the additive bounded `fuzzyFileSearch/sessionFailed` API.
   See
   [native plan](component-sdk/FILE_SEARCH_NATIVE_BACKEND_PLAN.md) and
   [composition plan](component-sdk/FILE_SEARCH_COMPOSITION_PLAN.md).
5. Build one new host and independently build/install native/custom workers
   outside its source. Exercise actual package discovery/selection, upgrade and
   incompatibility rejection, removal/replacement, GUI/TUI/one-shot queries,
   cancellation, resource exhaustion and recovery without rebuilding that host.
   Regress existing storage and GUI behavior, including Launch Ctrl+C and full
   child drainage. Use available Chromium fallback honestly while in-app Browser
   remains unavailable. The private rollout lookup remains a fourth consumer
   pending P03 broker; do not call search fully extracted before closing it.
6. Continue P03 lifecycle/transport/broker and the roadmap's successive phases:
   model/provider/auth; session/turn orchestration; context/history/compaction;
   native tools/execution and policy/approvals/sandbox; auxiliary state/storage
   maintenance; configuration/events and remaining UI/platform/services. P02b
   filesystem/watch/Git mutations wait for authority/config/policy contracts.
   Keep per-component implemented/tested/planned status and compatibility gates.
7. P18U upstream update integration is **required**, sequenced after stable
   contracts. Record lineage now; demonstrate a real later upstream revision,
   unchanged compatible custom packages, breaking rejection and independent
   recovery-bootstrap rollback before release. No updater, polling schedule or
   live update is active. Follow the canonical roadmap through final acceptance.

Earlier queue checkpoints remain recovery references, not new next actions:

- Native queue and paired process-startup work are preserved at external WIP
  `812b2d3e7df40dde146f4a33b3ad6fbacd53542c` on
  `wip/p02b-runtime-20261001`. Combined 99/99 passed with one ignored helper,
  unchanged source and subreaper exit 0; after the separate matcher correction,
  vendor 37 + native 41 passed. Scoped fix/format completed.
- WIP `98c540`'s 35 source paths are member-verified in
  `p02b-bounded-search-source.tar.gz`, SHA256
  `692c3de970a5f0a7d2f4d51373ff38b7eedb66a21b7fd0f0ac5e6be29c40256e`.
  This older archive remains preserved; the later native ledger, explicit cwd,
  service/process adapter and client shutdown source are in WIP `bb03f3a8`.

## Preserve and resume safely

Original primary `/workspace/codex-harness-everythings-a-plugin`; isolated recovered
WIP `/workspace/codex-harness-next-components`. Origin remains
`https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
Original `work` HEAD/index deliberately stay at upstream. Baseline files are often
untracked relative to that index: ordinary diff against main can falsely show them
as deleted. Compare actual file blobs. Never reset, clean or bulk-stage either tree.
Use exact path whitelists and a temporary index based on the accepted ref; verify
remote refs immediately before non-force push and verify GitHub afterward.
Original index SHA256: `2e0abf9135fedee93d85ffedb3045397fb85d35017f4191dc5e7a133af0b6759`.

Full both-worktree/SDK/docs/evidence/Git backup is in
`/workspace/recovery-backups/20260930T165936Z/codex-recovered-workspace.tar.zst`,
SHA256 `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
Later member-verified incremental `p01-source-20261001T040040Z.tar.gz` (historical
prefix) covers 124 changed paths versus main4288; SHA256
`9e581f18c9abc5e3719b6f45c11c2fe7b23e0b9284faf408049462edd6691a05`.
It precedes later source/state/lineage edits. A later member-verified incremental
`p01-source-20261001T042418Z.tar.gz` covers17 runtime paths versus main612bf2a,
SHA256 `18871beeccf1df50794e2d8705589d503766de92fc1d0acac86cae37234a0d39`.
Those paths are also on external WIP812b2d3e. Keep original failures and snapshots.
The member-verified integration incremental `p01-source-20261001T051016Z.tar.gz` covers
81 changed paths versus accepted main `59e5d576`; SHA256
`53c274520e38ce7f573239172af967680d19b00e1a27687cbde9f1d0acac93b3`.
Source publication report `p02b-integration-wip-publication.json` binds that
checkpoint to external WIP `bb03f3a8`. It predates later `/tmp` staging and evidence
edits; preserve those separately before interruption.
The subsequent `p02b-native-backend-staged-unverified.tar.gz` preserves 29 verified
staged members, SHA256
`7448d0518a6e7b5454e95a7656b9c41d26c69a4c002e8f4ab1d442e1f48d41d0`.
This is an uncompiled, cloud-local staging archive; it is not an accepted source
checkpoint or a GitHub backup of the new native adapter.
The later `p02b-native-backend-tested-source.tar.gz` has SHA256
`842dc08868198a68752e7c7fde61143783115540a2b8c8438647b88f40ea11eb`;
`p02b-native-backend-wip-publication.json` binds its 14 source paths to externally
preserved WIP `cb977e7d`. It preserves reviewed post-test formatting, not a claim
that the passing test ran again on those formatted bytes.
Newer worker staging has a separate 17-member cloud-local archive,
`p02b-native-worker-staged-unverified.tar.gz`, SHA256
`5e3daf2002e8ae3b1b9387888d1d9fb54b9f3c20fc077d37854e257e83cdd8f1`.
It predates the worker configuration correction and unresolved lifecycle-race
investigation; preserve later edits independently. Worker/facade completion is not
implied by either archive or native WIP publication. The facade's original staging
is independently preserved in `p02b-runtime-facade-staged-unverified.tar.gz`, SHA256
`abc295c97229e282780a681cd606a7a884be07abe90ad32e8bc46c6b0b9605a6`;
it predates the test import correction and later formatting.
Cloud-local archives are recovery checkpoints, not proven outside backups. GitHub
protects included published source only. Exclude binaries, caches, runtime homes,
credentials and private bearer URLs from publication.

Latest standalone source archive `p02b-selected-cli-tested-source.tar.gz`, SHA256
`b077447af6c7d80e09c371a11f07f95e55e70803e3819d62d0ab09b7ac1bdb36`,
is member-verified and bound to external WIP `651b0b87` by
`p02b-selected-cli-wip-publication.json`. Later App Server/client/TUI drafts remain
isolated beneath `/tmp/codex-search-*-stage`; AS and client/supplement archives
are preserved in this recovery directory. They are uncompiled and are not covered
by standalone runtime proof. Preserve newer shutdown corrections and TUI drafts
before interruption; never copy an old frozen whole file over a changed base.

Environment last observed connected/running, revision45, identity
`ccarenv_b64_Y2NhcmVudl8wYzAyOTNkMzVhZTg4MTkxYjc4YjQyZmVhNWYwMDllYQ`.
Source `/workspace/toolchains/component-verification-env.sh`, jobs1, existing
Rust1.95/debug0/incremental0. One owner serializes Rust; never kill Rust. Use
`just test`, scoped `just fix`, `just fmt`, and required Bazel/schema regeneration.
Use `/tmp/run-p02-check.py` for actual source manifests. Never reuse unchanged
`/tmp/run-component-regression.py`: its source label is obsolete.

Resource limit: 32 GiB overlay, about **2.6 GiB free at the client retry start**
after narrowly reviewed generated-cache reclamation. This is a timestamped
observation, not current free space. Recheck overlay, tmpfs and memory before
linking; do not use tmpfs without memory accounting. Audited
cache maps/restoration live in recovery reports `p02-generated-cache-tmpfs-relocation.json`
and `p02-pre-cli-cache-tmpfs-relocation.json`. Original-path cache symlinks need
these backing files. `p02b-reviewed-generated-cache-eviction.json` records
777,682,944 bytes reclaimed from three completed Cargo executables, preserving
all aliases/identities and observing no live task-process references. Cargo's dangerous hardlink to frozen-v2 was safely detached;
**do not restore it**. Completed App Server executable is preserved in the
checksum-verified `p02-completed-public-app-server.tar.zst`; generated new CLI
cache was removed after independent immutable candidate verification. Do not remove
source, evidence, runtime state, frozen binaries or active mappings for space.
Later root-reviewed cache removals are recorded in
`p02b-final-reviewed-cache-eviction.json` and
`p02b-old-library-cache-eviction.json`. The latter covers 23 exact old regular
`.rlib` variants (2,250,502,144 allocated bytes), retaining newer protocol/config
variants, metadata, aliases and immutable runtime checkpoints. Root revalidated
live references/builds before removal; no live Rust task was killed. Source,
original backups, test evidence and runtime state were preserved. The audit's
exclusion of pre-existing zombies from live-file-reference scans does not waive
any unchanged process-lifecycle assertion.

Optional viewer lives at `/workspace/remote-viewer-setup`, outside harness.
Current enforced restricted policy still has no additional hostname grant; local
services do not establish user-accessible reachability. Never bypass policy or
reset/replace this VM. This workstream does not block component development.
Routine implementation/testing and non-force checkpoint pushes are authorized.
Continue feasible phases; finite resources or unavailable Browser are limitations,
not grounds to claim the platform finished.
