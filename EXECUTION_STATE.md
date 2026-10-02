# Codex Harness Compartmentalized — execution state

Updated 2026-10-02. **Partial platform; P03 remains incomplete.**
Read [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md),
[COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md), root AGENTS.md and
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md). Superseded ledgers and exact
preservation/resource chains remain in [EXECUTION_CHECKPOINT_HISTORY.md](EXECUTION_CHECKPOINT_HISTORY.md).

## Current checkpoint

Latest publication verified: `bb48fe352dbc5e86edbdbb0f992f63fdcbee80c9`,
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
- Last verified source WIP `d229296b29947ef1328cbe931c810276f220a23c`, tree
  `44570e71ff6978933a6d01ef34b2a2e1de4ea125`, branch
  `wip/p03-process-final-and-mcp-preservation-20261002`.
  Main remains separately verified at `781080f7e3c8bfe1953378001d777dff33d74bc3`.
  This evidence checkpoint changes no active Rust/SDK implementation.
- Production build02 passed offline in7m47s. Source8,947 files, map SHA256
  `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`.
  CLI `78d9194cd4329e9b83b75fe07389d524d8b1f425353d71df9a68607be2945e83`;
  manager `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
  [Build/resource evidence](verification/2026-10-02/p03-http-stage-b-production-build/REPORT.md).
- All12 planned production-runtime slots have passing current-source attempts:
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
  **Uncompiled and untested.** Prior source7a runtime results do not validate it.
  Reload publication now checks committed source identity as well as cache/policy
  identity; five causal tests cover replacement, ABA, stale failures and cancellation.
  This is a native ownership prerequisite, not an additional extracted family.
- Exec grouped9/9 current7a tests passed, zero retries,64 filtered, strict0/null.
  [Exec evidence](verification/2026-10-02/p03-http-stage-b-exec-grouped/README.md)
  and [CLI proof preservation](verification/2026-10-02/p03-current-cli-proof-capacity/README.md)
  are copied locally but not yet published. Exec and CLI proof ELFs were archived,
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
   sufficient. No Rust command currently runs as of this ledger update.
2. Run the five focused reload causal tests through `just test`, then the login
   and model-provider scoped regressions using a feasible explicitly bound feature
   composition. Unconditional core_test_support cannot be removed to avoid cost.
   One compiler job; do not kill Rust commands. Full workspace approval is absent.
   Record actual source, exact executed binaries, original failures and retry counts.
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
   and Auto/legacy-writer limits. Then integrate MCP custody proposals in order:
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
  Keep later snapshots, failures, unadopted MCP/auth proposals and old restored
  proofs. VM-local archives are not externally durable backups.
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
