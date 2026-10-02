# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Incomplete platform; P03 lifecycle prerequisite in progress.**
Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required updater: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Detailed earlier results: [checkpoint history](EXECUTION_CHECKPOINT_HISTORY.md).

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
