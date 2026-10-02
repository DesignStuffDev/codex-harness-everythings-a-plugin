# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Partial platform; P03 remains incomplete.**
Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md), root AGENTS.md and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md). Superseded ledgers and exact
preservation/resource chains remain in [EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

## Current checkpoint

Latest publication verified: `e996f0db617318f06ab3a5f8d2db8498298777e2`,
tree `38acb622744bcf0f6809175e887ca97f4859a945`, parent75a42cc below.
All22 selected files (406,889B) and both refs were read back. This publishes the
focused5/full-login297 results, blocked lint admission, current provenance/roadmap
and the unadopted refresh rebase. Provider108 completed after this frozen selection
and is not included in that publication. Main and the original index are unchanged.
Receipt R/p03-auth-login-regression-publication-01/PUBLICATION.json SHA256
`d1e2ff16c92e221b498bd0402b699a6e63730a1c0685d435f1c364e6777151c3`.

Latest continuation publication verified: `75a42cc526e2065bc471b993c5c8151be3da7c9c`,
tree `787eef0e2eb5e3a68cc401696bbf5d5f943a1490`, parent bb48fe3 below.
All25 selected files (550,514B) and both refs were read back and checked. This
preserves Exec9 acceptance, failed TUI compilation, five-path untested auth reload
source/provenance and updated docs; it is WIP, not a main promotion or auth pass.
Receipt R/p03-exec-auth-wip-publication-01/PUBLICATION.json SHA256
`8a8c5383028d885c3998074c0ba12b8dd04e07d4de5388078709a970f01a26e8`.
The original local Git index is unchanged. Main remains781080f.

Preceding publication verified: `bb48fe352dbc5e86edbdbb0f992f63fdcbee80c9`,
tree `f18132d22033ec0ea2cadf451bfa20dfaa5c9546`; all14 selected Git blobs
(259,293B) and both remote refs checked. Main remains unchanged. This publishes
focused CLI and capacity evidence plus synchronized project/roadmap/status docs.
Receipt: R/p03-cli-capacity-status-publication-01/PUBLICATION.json SHA256
`afc6c79cbdef3598ff86d5697d91692f5ec73ce4fbbee8e85a3772a9f7e711ba`.
Parent `c463a4f46318ccbbc811c202639d53f733d5831f` preserves55 production/runtime
and unadopted auth-source artifact files, separately verified. No active Rust/SDK
implementation changes in either evidence-only checkpoint; the original index
remains unchanged. VM-wide external backup is not claimed.

- Original VM commands execute; stale access blocker is cleared. Checkout
  `/workspace/codex-harness-everythings-a-plugin`, origin
  `https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin.git`.
  Preserve sibling `/workspace/codex-harness-next-components`; no new VM/checkout/chat.
- Last production-tested source WIP `d229296b29947ef1328cbe931c810276f220a23c`, tree
  `44570e71ff6978933a6d01ef34b2a2e1de4ea125`, branch
  `wip/p03-process-final-and-mcp-preservation-20261002`.
  Main remains separately verified at `781080f7e3c8bfe1953378001d777dff33d74bc3`.
  This evidence checkpoint changes no active Rust/SDK implementation.
- Production build02 passed offline in7m47s. Source8,947 files, map SHA256
  `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`.
  CLI `78d9194cd4329e9b83b75fe07389d524d8b1f425353d71df9a68607be2945e83`;
  manager `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
  [Build/resource evidence](verification/2026-10-02/p03-http-stage-b-production-build/REPORT.md).
- All12 planned production-runtime slots have passing source7a attempts:
  13 successful commands because search retry needed a fresh migration. One failed
  GUI attempt remains preserved. This does not close all P03 acceptance.
  [First five gates](verification/2026-10-02/p03-http-stage-b-current-runtime-first-five/README.md)
  cover storage, migration-normal, ordinary GUI, attachments and migration-search.
  [Six later gates](verification/2026-10-02/p03-http-stage-b-current-runtime-shutdown/README.md)
  cover two migrations, slow normal/forced shutdown, held8 and the featured-owner
  postcheck; [search02 evidence](verification/2026-10-02/p03-http-stage-b-current-search-gui/README.md)
  covers the diagnostic retry.
  Every successful source wrapper matched the unchanged map above.
- Storage reuses an independently built native package for real CLI streaming,
  tools, cold resume and default restoration. Migration covers dry-run parity,
  apply/idempotence, recovery/cancellation. Attachments pass5 groups/17 commands:
  native upload, cold reference reuse, typed fallback, removal/default restoration.
  Reuse is not a new Rust build. Production attachment resolve, native response
  envelope inspection and nonempty attachment-state retention remain unproved.
- Ordinary and selected-search GUI each pass two real Chromium cycles. Approval,
  native command and Stop are cycle1 assertions; cycle2 covers cold history,
  interrupted state, continued streaming/search. Launch first-SIGINT returned0
  with tracked descendants absent. Inference is deterministic. In-app Browser
  remains unavailable; root visually reviewed selected current screenshots.
- Slow normal Launch waited46.4658s and recovered the canonical message; the46s
  gate holds before native forwarding, not native-internal admission. Forced
  second-SIGINT returned1 at2.0393s with unknown durability; cold continuation
  passed. The raw-input/response-only history gap remains open.
- Held Git/HTTP matrix: all8 case gates and featured-owner postcheck passed.
  Strict histograms include the direct command and reaped descendants; retain
  nonzero child statuses without causal attribution. No universal whole-host-clean
  or all-descendant-graceful claim. The89 require call sites are a static count,
  not executed assertions. Active-held constructor/lower-transport/205s hard-exit
  proof remains separate.
- Search GUI01 failed on its cold cycle's60s file-result wait. Native startup had
  ENOSPC warnings, but causality is unproved. GUI02 used fresh executable SHM homes
  and an observation-only external driver, preserving28 assertions/deadlines.
  Both cycles passed;12 search responses each, no diagnostic overflow/errors or
  pending bodies at close. Preserve01; a retry pass does not explain its cause.

## Current continuation — read before running commands

- TUI grouped command1433 finished with compiler exit101: rustc lib-test received
  SIGKILL and cgroup OOM counters rose9→10 / kills4→5. **Zero tests executed.**
  Source7a stayed unchanged. A/p03-http-stage-b-tui-grouped-01 preserves log,
  source/strict/preflight/terminal receipts. Do not repeat the unchanged TUI attempt.
  The six-test/two-snapshot gate is open; CLI3 and Exec9 results do not satisfy it.
- Production CLI78d has been restored from the full34-member verified archive.
  Both original hardlinks and16 unchanged home aliases resolve again. Receipt
  R/p03-current-production78d-proof-preservation-01/RESTORATION_RECEIPT.json:
  `f6e67f8a6a5eaf5948f1c8999f7f89de913c91fe5047f5f0cef0a65c4323a8d2`.
  Actual `codex --version` returned0; READINESS.json records this bounded check.
  This restores previously tested bytes, not a new regression pass.
- **Current source is newer than production.** Five-path native auth reload change
  adopted after exact preimage checks; original files and patch preserved in
  R/p03-auth-reload-adoption-01. Required `just fmt` returned0, changing only the
  new test module and manager formatting within those five paths. No dependency,
  public API, wire or state-format change. Source8,948 files, map
  `0742356b9ff442a4d38f6704928930602f92df0fdfe89bfb250897fda470729f`.
  Candidate SHA256 `a3e21f97c6d5e83cab69821a4507e223deaecf5604fd2d7aaffbc7b19c1e6f4f`.
  Five focused,297 full login and108 provider tests now pass as recorded below.
  Lint and rebuilt-host validation are pending. Prior source7a runtime results do not validate this change.
  Reload publication now checks committed source identity as well as cache/policy
  identity; five causal tests cover replacement, ABA, stale failures and cancellation.
  This is a native ownership prerequisite, not an additional extracted family.
- Exec grouped9/9 current7a tests passed, zero retries,64 filtered, strict0/null.
  [Exec evidence](verification/2026-10-02/p03-http-stage-b-exec-grouped/README.md)
  and [CLI proof preservation](verification/2026-10-02/p03-current-cli-proof-capacity/README.md)
  were published at75a42cc above. Exec and CLI proof ELFs were archived,
  fully restored/hash checked, then only their original files/new verification
  restores were retired. All old proofs/restores remain. See historical ledger.
- Current production archive SHA256
  `b4c79bcc83cb356d9f2707fb9a17a3deae10c86860e9ac601af25d8b8d8154bf`;
  Exec archive `90c667161c10b669ea4392c1398f228df707622168990e23b829d4d3372f62e1`;
  CLI-test archive `f5197d265f3742c497d44eff0252936438b5cd88fb9e406ae41d3da21ca7101f`.
  These are VM-local, not external binary backups. Completed resource actions
  must never be replayed after path reuse. Original Git index remains unchanged.

## Ordered next actions

1. Compiled-only CLI library ELF ba3b51 and47 provenance members were archived,
   fully restored and hash checked. Only the original cached ELF and new verification
   restore were retired. Archive SHA256
   `e32848acf97b6f6dbd4df26bf9f7244c37f0b1891399bc53beb9244504ca90d3`;
   release receipt `9bca0ff99227a104698f6bfa9c61f62730dc0955e623040938cbd9c1c71be5c8`.
   [Adoption and preservation evidence](verification/2026-10-02/p03-auth-reload-adoption/README.md).
   A future compiler may now reuse that selector. Keep restored
   production78d, all recovery archives, old restores and the separate older GUI.
   Review fresh disk/RAM/process headroom; TUI OOM proves prior admission was not
   sufficient. No compiler currently runs; the completed gate is recorded below.
   Four exact source-invalidated production-context ordinary cache files were then
   retired after independent review and two complete guard passes, preserving2,554
   dependency records. No warm-test dependency intersects this scope. Receipt
   R/p03-auth-reload-production-ordinary-four-action-01/RETIREMENT.json SHA256
   `5f699334068cc4d5a8432554c627e4f95eea0ac7a6e9296685c7479ee876b214`.
   Overlay afterward942,952,448B; source/index and production metadata unchanged.
   This adds339,161,088B of future production rebuild debt, not warm-test debt.
   Fresh test admission remains separate; never replay the completed action.
2. Focused reload, full login and provider gates are complete below; finish scoped
   lint using a feasible explicitly bound feature
   composition. Unconditional core_test_support cannot be removed to avoid cost.
   One compiler job; do not kill Rust commands. Full workspace approval is absent.
   Record actual source, exact executed binaries, original failures and retry counts.
   Focused root command70340 completed: **5/5 passed**,250 filtered, zero actual
   retries, source0742356/8,948 unchanged. Build4m49s; guarded command296.587s.
   Terminal A/p03-auth-reload-focused-01.terminal.json SHA256
   `498eb8be6f4f650b676021298c9399f0b403fbd541d449615a368afb486c1020`.
   Actual login ELF codex_login-36e9bc4560b410c1 (156,026,712B), SHA256
   `73b95bfe4a82af5ec52383e202e8e0730145c144496b077e63a41db2f4cca723`.
   Strict0/null, reaped{0:1}, OOM10/kill5 unchanged. Production7a remains
   unchanged and differs from tested auth source. Two discovery calls are separate
   from five actual test executions. Compiled-only CLI library SHA256
   `21f25bfd24f62da5466ee94153f7823aab6aaade57702d774570b615190a671b`
   is separately bound to this source; it was not executed as CLI tests. Provider
   regression, lint and new production/GUI checks remain open.
   The full login command uses explicit `--test-threads=1` to bound nested fixture
   compilers, preserving all test bodies, security/network guards, local profile,
   retries and deadlines. Serialized coverage is not default-concurrency coverage.
   Full login root command19303 completed accepted:297/297, zero skipped/retries,
   strict0/null with reaped{0:1}; source0742356 unchanged and OOM10/kill5 unchanged.
   Four discovery invocations remain separate from297 actual test invocations.
   No observed test had the sandbox network guard present; conditional occupied-port
   early return is not independently body-attested. Serialized scheduling remains
   distinct from default-concurrency coverage. Terminal SHA256
   `db87017ab3a882105de845abc24f190d58cd8491b975b4e8099c91e8c84b0fa3`.
   The focused login library was reused. New integration ELF all-ca6d95d908331531,
   139,872,288B, SHA256
   `fb25da3c5079f339f51c979af68d09e142e67975ad015d84c6402e3af4cf5543`.
   Plan manifest
   `efa4df7d5cb5f6c1a74a17428ed81404c29ddbf22e3636d6908199527b6df2bb`;
   output prefix A/p03-auth-login-full-01. Admission passed after bounded
   clean-page advice:587,833,344B overlay and1,462,861,824B hard-unused RAM,
   OOM10/kill5. Both library outputs, integration ELF and production78d remain
   protected. Prioritize required login lint next: conservative disk floor391,987,200B,
   hard2GiB/effective3GiB RAM; current overlay444,121,088B. Whole model-provider
   library follows if fresh311,066,624B disk/1GiB hard/2GiB effective floors fit.
   Lint root11489 stopped before compiler startup: hard1,849,307,136B below2GiB;
   effective3,392,063,488B/disk428,666,880B passed, OOM10/kill5 unchanged.
   [Preserved admission failure](verification/2026-10-02/p03-auth-login-lint-admission/README.md).
   Six exact cache hints changed no artifact bytes. No Clippy/source edit occurred;
   do not replay this attempt or lower the floor. Proceed with the lower-cost
   provider gate and inspect a bounded ordinary-cache opportunity separately.
   Provider root53017 then passed108/108, zero skips/retries, unchanged0742356,
   no new OOM. Strict0/null retains reaped{-9:1,0:1} without causal/graceful-cleanup
   attribution. Terminal SHA256
   `cfe06da9e55e6f3c9989a1063e92cc03d1e733844787f5697125880b8defee2b`;
   actual111,256,736B provider ELF SHA256
   `6e8d48e93e721e322a204ff45a730b0b8fe76c7c6203eda9a13420f31e909f8b`.
   [Provider evidence](verification/2026-10-02/p03-auth-model-provider-full/README.md)
   is copied locally, not yet published. Overlay afterward313,868,288B, below
   lint's391,987,200B floor. Next resource proposal preserves and retires only the
   current compiled-only CLI library21f25, retaining all executed test ELFs.
   This resource chain has now completed in
   R/p03-auth-current-clilib21-artifact-preservation-01: all482 members were fully
   restored/hash checked before only CLI21 was retired and only the new private SHM
   verification restore released. Archive43,619,206B SHA256
   `d255e568044c3ffc93fe6706cded1e9df0adec6401b75985de34707d16ef0fc4`;
   preservation `6f14e66a59c71d22f13d3991442f9aa7813b0c95c9f246c3a9c86bbaa8970d3d`,
   retirement `04c43319a6db0283670dd575951f6604cf2f0f0808f60a59ea7b8d452a343abd`,
   release `ae47f513bc3cf40015b2977c47cc60c6223915e1ec0a597f31b1200f955c85c7`.
   Archive is VM-local. All old restores, source, index and executed test binaries
   remain. Never replay either CLI21 or prior ba3 actions after selector reuse.
   Seven exact proof-cache hints and five ordinary-library hints changed no bytes;
   the latter passed RESULTf28dcedc in R/p03-pre-login-lint-five-library-advice-01.
   Its first finite residency estimate was insufficient; the eight-file follow-up
   supplied a five-file448MB resident subset. Advice is not guaranteed reclamation.
   Lint02 root19497 then stopped before compiler startup: hard1,812,885,504B
   remained below2GiB; effective3,307,030,528B and overlay463,826,944B passed.
   OOM10/kill5 remained unchanged. Preserve A/p03-auth-login-fix-02.preflight-failed.json;
   no Clippy invocation, source edit or lint verdict occurred. Do not replay or lower
   the floors. Read-only audit identified one redundant old current58 verification
   restore (two hardlinks to one635,424,768B tmpfs inode). Its complete7-member
   archive remains in R. Root must freshly verify archive/member/restore bytes and
   live references before releasing only that generated copy. Source, runtime homes,
   current executed binaries and all recovery archives remain protected. A new lint
   attempt needs a measured resource change and fresh admission, not elapsed time.
   Record any eventual Clippy source transition; do not rerun tests solely for
   formatting/lint. No compiler currently runs.
3. Only after the reload stage is verified, adopt the separately preserved permanent
   refresh-failure publication fix. Formatting changed its manager preimage, so
   explicitly rebind/review that transition before applying it. Run its four causal
   tests, scoped regression, scoped lint and required format. Successful refresh,
   durable writes and request dispatch remain separate ownership gaps.
4. TUI six tests, App Server lifecycle/search/storage16 and actual same-process
   featured/curated replacement parent runners remain open. AS additionally
   enables rmcp elicitation and invalidates a47-record parent cohort (~1.158GB
   historical output scale). Do not repeatedly run unchanged resource-blocked builds.
   Future rebuilt-host/UI checks must bind the actual new source; old78d is retained
   baseline proof only. Browser unavailable; label real Chromium and deterministic
   inference fallback explicitly. Preserve the search GUI01 failure.
5. Preserve source-to-component provenance and publish reviewed coherent milestones
   on the existing WIP branch with nonforce ref checks, main untouched until accepted.
   Successful-refresh/request dispatch needs conditional durable ownership across
   participating writers, including equal-value/absence ABA, cross-process/keyring
   and Auto/legacy-writer limits. [The grounded writer audit](AUTH_STORAGE_OWNERSHIP.md)
   identifies the shared secrets passphrase dependency and ordered Ephemeral then
   complete File/Keyring/Auto activation; no implementation is claimed by that audit.
   Then integrate MCP custody proposals in order:
   Session prewarm/refresh, connection retirement, lower transport, pinned SDK,
   late admission. Resolve marketplace queue transfer before closing HTTP pools.
6. Activate negotiated broker/model-service composition and extract the actual
   native OpenAiModelsManager discovery/merge/cache. Require separate native/custom
   builds, installation/selection/cancellation/removal/default and GUI proof against
   an unchanged host. Continue remaining dependency-ordered P04–P19.
7. Required P18U needs installed maintenance plus external recovery, a real later
   upstream integration preserving custom plugins/UI, coordinated versions/state
   migrations, incompatible-update rejection and rollback. Offline17+22 support
   tests do not meet those gates. No scheduled polling or live unattended update.

## Preservation and execution rules

- R = `/workspace/recovery-backups/20260930T165936Z`; A = `/workspace/acceptance`.
  Full two-worktree/Git/SDK/evidence archive `R/codex-recovered-workspace.tar.zst`,
  SHA256 `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
  Primary pre-format archive SHA256
  `e6ab98fe2bd9625c10fe67979eaf308b4f7e4ee7892548482ff0750d9847df5e`.
  Keep later snapshots, failures, unadopted MCP/auth proposals and complete recovery
  proofs. A redundant generated verification restore may be consolidated only after
  exact archive/member/restore verification and reference guards; never discard unique
  recovery material. VM-local archives are not externally durable backups.
- Original auth reload/refresh proposal archives are preserved as unadopted snapshots under
  `verification/2026-10-02/unadopted-proposals/auth-source-ownership/`; both were externally preserved at c463a4f. Reload is now locally adopted as recorded
  above; that historical label does not describe current source.
- Root owns source/Git/compiler/cache mutations. Local HEAD/index remain upstream,
  local main is stale. Do not reset/stage the real index. Index SHA256
  `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
  Keep the separate older live GUI/remote-viewer directories. They are not current
  runtime proof. Viewer networking is optional; do not repeat its unchanged blocker.
- Build02/plan03 bindings stay frozen. Continuation04 changed only fresh directories,
  ran6 remaining commands, no new OOM. GUI02 dispatcher/strict/source reports are
  separate. Never replay cache retirement/restoration after compiler path reuse.
  Genuine Cargo download archives now have verified but ephemeral tmpfs backing.
- Source `/workspace/toolchains/component-verification-env.sh`, use one compiler
  job and fresh disk/RAM/process guards. Use `just test`, never direct `cargo test`;
  scoped `just fix` and required `just fmt`. Full workspace approval is not recorded.
  No force push, secrets dumps, scheduled polling, unattended live deployment or
  hot replacement. Context7 and in-app Browser remain unavailable here.

Coverage remains three bounded native replacement families: thread storage/manual
migration, inline attachments and file search. GUI is an additive presentation
package. HTTP/auth custody and SDK/host infrastructure are not additional extracted
families. Most engine subsystems and final clean-install/plugin/UI/updater gates
remain unfinished.
