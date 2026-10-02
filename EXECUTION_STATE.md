# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Incomplete platform; P03 lifecycle prerequisites remain open.**
Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Coverage: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required updater: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Earlier detailed ledgers are preserved, with process IDs redacted from the public copy, in
[EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

Immediate queue: obtain the current-source
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

Main remains 781080f7e3c8bfe1953378001d777dff33d74bc3 (native installer source
65511842d7051b2a1f5cc52917f3ebb5c03be4f3). Do not promote unfinished P03 work.
Latest WIP ref verified after this checkpoint:
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
