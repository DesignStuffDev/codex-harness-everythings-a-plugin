# Codex Harness Compartmentalized — execution state

Canonical plan: [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).
Scope/status: [COMPONENT_INVENTORY.md](COMPONENT_INVENTORY.md).
Required update track: [UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md).
Full v1 is incomplete. Follow this queue across runs; do not infer completion from crate counts.

## Last verified source and publication

- Repository: `DesignStuffDev/codex-harness-everythings-a-plugin`; original cloud checkout
  `/workspace/codex-harness-everythings-a-plugin`. Preserve isolated saved work at
  `/workspace/codex-harness-next-components`.
- Verified cancellation source: `3b8a889738d27eb1edc95e3f63891ab9cf0b6017`, tree
  `8f64a2b3b70cbf52bfb89a065f2bf791b214cc2a`, published and remotely verified on
  `wip/p02b-preparing-cancellation-20261001`. Six final review slices extend `9bd3bc30`;
  their intermediate commits were not individually compiled. This checkpoint promotes the
  verified final source while retaining current main documentation and original failed evidence.
- Last full-host/UI checkpoint: `c28a1c33a856a987316f4b97b4c488d68055fffc`, tree
  `b40fa4d6f635dea2de7ca1856aaf96d5af58f37b`, remotely verified after nonforce publication.
  Parents are `a0be45bd` and `3b8a889`. It promotes73 source paths plus22 documentation/evidence/
  fixture/image paths. Among8978 published scoped entries, implementation hashes match the build;
  one historical-plan status header was updated afterward. Bubblewrap LICENSE remains the same
  `COPYING` symlink, outside the regular-file hash map. Receipt:
  `/workspace/recovery-backups/20260930T165936Z/p02b-preparing-promoted-publication.json`.
  Previous accepted runtime `a469cf4` and additive SDK `d22cea88` remain historical.
- Latest previously verified remote support checkpoint: `ca38f8c6d956f514dd09b03a6540ba388e5ec038`,
  tree `c8073ab505d94ce06ed2e345f74e364c83c44adc`. Seven private wire paths plus evidence/docs
  were nonforce-published and verified. Receipt `p03-wire-slice1-publication.json` is in recovery.
  The next declaration checkpoint below has separate source and runtime bindings.
- Declaration support is now published and remotely verified at `8ee6b667521e49ed9879a5ccd35cc22aa6595458`,
  tree `2822cb23417e67a22434c275b7af7155aee8bae8`, parent `ca38f8c6`. It contains twelve
  reviewed source paths plus Cargo.lock and separate evidence/design documentation. Receipt:
  `p03-service-declarations-publication.json`. Broker remains inactive.
- Latest checked-limit checkpoint: `5fbe8a7901e1f935c40e969b4bb3cd7811e0d6be`, tree
  `2ef11a23fd848b4e236186b8518dd5d7c811d2df`, parent `8ee6b667`, nonforce-published
  and remotely verified. Three API source files plus nine documentation/evidence files;
  receipt `p03-broker-limits-publication.json`. No live broker enforcement is activated.
- Checked grants are published and remotely verified at `6c5b232518ca951ebc80d5199cae01c901f68c85`,
  tree `7b2c4e0d1332b6cbab75b95e969e061d1307fde7`, parent `5fbe8a7`. Five API source
  paths plus nine docs/evidence/audit files; receipt `p03-broker-grants-publication.json`.
- Latest offer/ack API checkpoint: `9cf9002d9c70068f2eb9f3bcd054ee64120feb6c`, tree
  `4de47ce8a643d605b6097c16498f6b292e1f92df`, parent `6c5b232`; eleven paths were
  nonforce-published and remotely verified. Receipt `p03-broker-offer-publication.json`
  records the exact three formatted source paths and separate documentation/evidence.
- Official upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`;
  exact-tree import `ae720ae9a98bad29ca2cff998e7d5baaf05cec86`, tree
  `147ac2447134294359c4071b0aeb495922760db7`. Retain LICENSE/NOTICE and lineage.
- The local HEAD/index intentionally remain at the original upstream pin. Local `main` is stale.
  Do not reset or fake local commits to match connector publication. Original index SHA256:
  `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.
  Use temporary indexes/exact trees; root owns checkout edits, Rust builds and Git writes.

## Verified search-lifecycle milestone

- Combined AS/client/TUI libraries: **6095 passed, 4 skipped**, zero retries. Initial E0624
  compile failure preserved; one parent-visibility correction preceded the pass. Lint passed
  unchanged. Formatting changed ten files mechanically, reviewed separately. See
  [library evidence](verification/2026-10-01/P02B_PUBLIC_STOP_EVIDENCE.md) and
  [format transition](verification/2026-10-01/P02B_PUBLIC_STOP_LINT_FORMAT_EVIDENCE.md).
- FullCLI build passed with all10148 scoped fingerprints unchanged. Frozen artifact:
  `/workspace/component-checkpoint-candidate-p02b-preparing-full-cli-20261001/codex`,
  SHA256 `1b72a190ba6ebcca68c4f0d4145f4ec129b975e0e18fcddfab322f8e7f9165e7`,
  634578704 bytes, mode0555/nlink1; original inode retained, no byte transformation.
- Real installed search: unchanged worker03(0.1.0) and worker04(0.2.0) normal compatibility;
  held-reply legacy cancellation and public sessionStart/Stop FIFO/replacement all passed.
  Release was Joined before unhold; exact late reply, sibling survival, ordinary shutdown,
  removal/native restoration and tracked PID absence passed. The relay holds a reply after
  native Ready: this specifically proves host/process Preparing, not native construction.
  [Build/installed evidence](verification/2026-10-01/P02B_PREPARING_NEWHOST_EVIDENCE.md).
- TUI integration: **23 passed, 4 skipped**, zero retries, unchanged source. All five former
  integration failures now pass; the separate library gate cleared the other twelve.
  Preserve the earlier5633pass/17fail/8skip report. No test assertion or snapshot was weakened.
  CLI env binding is a retrospective root-launch transcription, not wrapper-captured metadata;
  some cases use the standalone TUI. [Evidence](verification/2026-10-01/P02B_PREPARING_TUI_INTEGRATION_EVIDENCE.md).
- Unchanged native storage0.2/contract2:13 runtime commands passed, including real CLI/tool/context/
  streaming/cold-resume behavior; fresh manual migrations01/02 each passed10 commands.
  Normal GUI passed two cold launches, search/reference flows, streaming, approval, Stop, reload,
  continuation and recovery. First SIGINT only to manager; exits0/.266s/.265s, tracked identities
  absent. [Storage/normal GUI evidence](verification/2026-10-01/P02B_PREPARING_STORAGE_GUI_EVIDENCE.md).
- Instrumented GUI clear-query Preparing passed two cold-launch cycles. Actual joined Release
  and empty original/cancel replies preceded unhold; sibling/replacement searches survived.
  Existing GUI regressions and first-manager-SIGINT shutdown passed0/.719s/.215s, all tracked
  identities absent. SIGINT occurs after unhold; pending-Open shutdown remains a distinct gap.
  [Held GUI evidence](verification/2026-10-01/P02B_PREPARING_GUI_HELD_EVIDENCE.md).
- Earlier separate native-owner88/native-backend96, API/native/worker169, process42 and runtime55
  scopes remain in their evidence files; do not add overlapping counts. Raw nonzero adopted
  child exits and original failures remain preserved. GUI uses Chromium/Playwright fallback and
  deterministic inference, not in-app Browser or live-provider proof. Storage's historical
  per-child exe-disappearance assertion is weaker than GUI/migration PID-absence assertions.

## Verified P03 support checkpoints

- Private wire factoring: **87 passed, one skipped** (81 host library, six manager; API compiled
  with no executed cases). Lint unchanged; three mechanical format changes independently reviewed.
  Source/format lineage and raw child statuses are in
  [focused evidence](verification/2026-10-01/P03_WIRE_SLICE1_EVIDENCE.md) and its format companion.
- New frozen search CLI `a52a960f424101e34d0027555d8ee7f223c7a685721d57477c2a4dcf1cb7dfa1`
  passed **24 actual commands** against unchanged worker04 and original manager: exact parity,
  CLI-only Ctrl+C exit130, selected failure exit1 without fallback, removal and PID absence.
  [Runtime evidence](verification/2026-10-01/P03_WIRE_SLICE1_RUNTIME_EVIDENCE.md) binds the
  8,861 historical source entries. FullCLI1b72/GUI evidence predates P03; it is not new-codec proof.
- Service declarations: **177 passed, one skipped**, zero retries across five crates; includes
  all seven new tests. The first compile/link failed SIGBUS while disk was full, with zero tests;
  unchanged source passed after generated-cache preservation. The offline all-platform metadata
  failure is retained; Linux-filtered metadata passed. Cargo.lock adds only the API test dependency;
  required Bazel lock update passed unchanged. Lint unchanged; three mechanical format changes.
  [Test evidence](verification/2026-10-01/P03_SERVICE_DECLARATIONS_EVIDENCE.md) retains failures
  and raw four -9 child reaps without inventing causes; formatting is recorded separately.
- Fresh manager `e34edd90256bafb3b3aec94360a4de2ee885e0082085df7b91d33449aea748fb`
  passed **eight actual install/catalog commands**. Plain install/select/remove works; required
  and optional service declarations both reject before changing existing state. All observed PIDs
  disappeared; package/manager/source unchanged. [Manager evidence](verification/2026-10-01/P03_SERVICE_DECLARATIONS_MANAGER_EVIDENCE.md)
  covers installation only: it did not execute worker04 or activate broker/services/full GUI.
- [Published declaration API](component-sdk/SERVICE_REQUIREMENTS_V1.md) is custom support,
  not native extraction. Adding the Rust struct field requires external Rust literal updates;
  ordinary JSON packages omit it and remain compatible. Never count these overlapping suites
  as additive independent coverage. Their exact snapshots and source-only runner archives remain.

- Checked broker limits: **10 API tests passed, zero skipped/retries** (four new budget cases,
  six existing declaration cases). Lint unchanged; two mechanical format changes reviewed.
  [Evidence](verification/2026-10-01/P03_BROKER_LIMITS_EVIDENCE.md) binds actual tested and formatted
  bytes. No dependency, manifest, runtime enforcement, native extraction or UI change.

- Checked handles and service grants: **15 API tests passed, zero skipped/retries**
  (five new grant cases and ten earlier cases). Lint unchanged; three mechanical format changes.
  [Evidence](verification/2026-10-01/P03_BROKER_GRANTS_EVIDENCE.md) records exact source and scope.
  This validates descriptions only; it issues no authority and activates no service.

- Canonical offers/acknowledgements: **20 API tests passed, zero skipped/retries**
  (five new offer cases and fifteen previous cases). Lint unchanged; two mechanical format changes.
  [Evidence](verification/2026-10-01/P03_BROKER_OFFER_EVIDENCE.md) distinguishes the original stage
  from root integration onto verified grant source. Reordering is accepted; incomplete or changed
  grants/limits/handles are rejected. No live handshake, service execution or GUI proof is added.

## Ordered next actions

Current verified native prerequisite: the two-file provider-owned model endpoint
change is adopted from `/tmp/p03-provider-endpoint-stage/FINAL_MANIFEST.json`
(`22c63961cea9539a2b6ed8031740cd8b74e5ed20d1606ac08f9de8f584907119`).
The first provider/models-manager check exited101 during compilation because the
overlay filled; **zero tests executed**, and all scoped source hashes were unchanged.
Retain `/workspace/acceptance/p03-provider-endpoint-tests.{log,source.json,subreaper.json}`.
The unchanged-source retry passed **160/160, zero skipped or in-run retries**: 106 provider
and 54 models-manager tests, including all five new real local-HTTP cases. Lint passed
unchanged; formatting changed only the new test module mechanically. See
[evidence](verification/2026-10-01/P03_PROVIDER_ENDPOINT_EVIDENCE.md) and its source lineage.
The raw successful-run subreaper record includes one unattributed -9 child exit and command
exit0; preserve both. This is compiled native capability ownership only; no catalog plugin
or broker is activated, and old full-host/UI results do not validate this new code.
Six generated apt metadata files were checksum-archived before retirement, preserving
sysroot libraries and setup/package records. Recovery archive
`p03-apt-metadata-preservation.tar.gz` has SHA256
`1531a9707373af18a4102ce83ad0b4bb651cc346dd2309e4cf780c3c13f49026`.
It is a cloud-local cache recovery record, not an external source backup.
Recovery also retired 314 audited obsolete generated library objects (640,327,680 allocated
bytes) while retaining source/toolchain, metadata, current dependency closure and binaries.
Receipt `p03-provider-obsolete-library-retirement.json` records exact old identities.
Eight inactive completed storage worker copies now share the retained original package
inode; bytes, modes and paths remain unchanged, but historical mtimes/inodes/link counts
deliberately differ. Preserve `p03-completed-storage-copy-dedup.json`; copy before any future
write/chmod/utime. This changed storage representation, not historical runtime outcomes.

1. Recheck original environment, source state, remote refs, active work and resource headroom.
   Both wire and declaration checkpoints have durable receipts above; revalidate remote identity
   before new publication. Never infer it from local files or stale local main.
2. Publish the native provider prerequisite with its actual evidence; maintain the P00M
   current provenance index/checker alongside extraction. Stage native policy-epoch work
   separately. A fresh full-host build and UI regression must bind the new source before
   claiming runtime integration. Follow the agreed [leaf-broker design](P03_LEAF_BROKER_DESIGN.md) and
   [source bindings](upstream/p03-broker-design-bindings.json). The card is a preserved design
   snapshot, not execution proof. Declaration DTOs are accepted; retain the catalog rejection
   until coordinated negotiated activation. Checked limits, handles/grants and the
   [canonical offer/acknowledgement API](component-sdk/BROKER_NEGOTIATION_V1_DESIGN.md) pass their
   focused gates. Implement the runtime handshake against these contracts next. Missing optional-only
   acknowledgement disables the broker; required absence or present-invalid acknowledgement fails.
   Keep strict parsing on original bounded handshake bytes, including envelope duplicate fields.
3. Implement bounded connection/call/decoded-work ownership, exact grant equality, retained
   decode/serialize/flush/receiver receipts and all centralized leaf-entry guards in reviewed
   slices. Reuse active startup supervision; do not revive stale waiter-owned broker drafts.
   Only after shared signatures are frozen should independent module implementation proceed
   in parallel. Plain history semantics and native security ceilings must remain intact.
4. Prove a separately built installed diagnostic consumer on the real host, then recheck old
   search/storage packages and manager/GUI lifecycle. This is infrastructure acceptance, not
   native extraction. Browser-tool absence and deterministic-model limitations stay explicit.
5. Extract native model-catalog/cache after required provider capability, atomic native policy
   epochs, isolated-session behavior, finite domain limits and retained cleanup seams. Real
   model listing/selection must change through separately built replacements with unchanged host.
   Follow the [native prerequisite audit](component-sdk/design/native-catalog/NATIVE_CATALOG_NEXT_CHECKPOINT.md)
   and its exact source bindings; provider endpoint and native policy epoch can be implemented
   independently before broker activation, but neither alone is extraction.
   General Session cleanup, model-v2, authentication/configuration, inference and the private
   storage search fallback remain obligations, not silently removed prerequisites.
6. Continue P04–P19 and every inventory row. P18U must implement the installable updater plus
   external bootstrap, prove a real later-upstream isolated integration and breaking-candidate/
   failed-activation recovery. Preserve lineage now. No polling or live update is enabled.

## Resume and preservation rules

- Read AGENTS and the cloud-runtime skill; verify this original task-bound environment, repository,
  both worktrees, active processes, current/enforced policy, mount space and cgroup memory.
  Never substitute a blank checkout/local VM. Shell Git auth is unavailable; supported GitHub
  connector publication works. Recheck refs immediately before nonforce updates; no unrelated settings.
- Recovery root: `/workspace/recovery-backups/20260930T165936Z`. Full both-worktree archive
  `codex-recovered-workspace.tar.zst`, SHA
  `3ed6902a915654787bcc6166fcd27da8c71feb0a186f180b5d98f823854e9cbd`.
  Later source incrementals, original Git state, staged patches and publication receipts remain.
  These are cloud-local checkpoints. GitHub protects included published source only; it does not
  back up private state, credentials or executable artifacts. Preserve isolated unfinished work.
- Keep original manager `/workspace/component-checkpoint-candidate-p02b-search-cli-20261001-02/codex-component`
  (SHA eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a,mode0700), both frozen fullCLIs,
  worker03/04 source/package receipts, storage package, source archives and all failed evidence.
  Package helper guards bind exact modes/fingerprints; never alter artifacts to bypass checks.
- Rust1.95 environment: `/workspace/toolchains/component-verification-env.sh`; override jobs1,
  debug0/incremental0 already set. Use `just test`, zero retries and the unchanged
  `component-sdk/tests/subreaper_runner.py` (SHA fe01097ae1741cbcb76e15fb07c0c936108a4dc35c08b4264c1caa9eb1e08875).
  Do not kill Rust. Source wrapper baseline f2cc is stale annotation; actual hash maps are authoritative.
- Memory/tmpfs is a material constraint. Some generated rmeta/rlib paths link to `/tmp` or `/dev/shm`.
  New twelve-library relocation:1478025216B, original paths retained,60 metadata files unchanged.
  Recovery reports `p02b-additional-completed-library-shm-relocation.json` and
  `p02b-additional-shm-post-verification.json` record exact aliases/hashes. Prior four-library report
  `p02b-completed-library-shm-relocation.json` remains. One old TUI alias was rebuilt regular;
  preserve it and its different prior backing. SHM is volatile cache, not source backup. If missing
  after restart, while Rust is idle remove only verified recorded dangling generated-cache links;
  allow Cargo regeneration. Never remove a rebuilt regular file based on an old symlink record.
- Disk remains tight. Three completed TUI executables/four generated aliases were checksum-verified
  in `p03-completed-tui-executables.tar.gz` before retirement; 184 metadata files remained unchanged.
  Archive SHA `125e90d2ec711bf4fa2a2c62bdf84abb2fbd6ed0facd3ec202d24dde4e9ed19d`; receipt
  `p03-completed-tui-executable-preservation.json`. Do not treat absent generated executables as
  missing source or delete preserved hosts; regenerate only when needed with planned headroom.
- One generated temporary linker output (`codex_thread_store_component-fd3e6e92c2bd856a.tmp83d0ef5`)
  was preserved then retired after idle/process-reference checks. Its basename/timestamp align
  with the earlier SIGBUS failure; the failed log does not bind its exact temporary pathname.
  `p03-failed-link-temporary-preservation.json` records original bytes, modes and restore recipe;
  archive SHA `628a47758ffc72b4e207435498e27deca92f34708795bbbb44ac66900f2928d2`.
  The successful final executable and metadata remain. No source/frozen host was removed.
- Two completed generated helpers (`logs_client`, `exec-server`, including their two deps
  hardlink aliases) were archived and verified before retirement for native-test headroom.
  `p03-completed-helper-executable-preservation.json` retains aliases, modes, hashes, native
  build associations and restore instructions; all fingerprint/dep metadata stayed unchanged.
  Archive SHA `df6a8be63dba8005329a817759bb149ce2db9a585c9a95cb618fac272968fdf7`.
  The exec-server bytes matched the earlier retained replacement. These are generated targets;
  future tests selecting either helper must restore or rebuild them. Frozen hosts are untouched.
- GUI reports/logs/readiness URLs are private. Publish only reviewed whitelisted summaries/images.
  Tested staged-runner source archive and member manifest are under `verification/2026-10-01/fixtures/`
  and `p02b-preparing-source-fixtures.json`; exact path-bound provenance checks require reviewed
  restaging on another machine. Do not disable them. Viewer networking remains optional and separate:
  current enforced policy exposes no incoming preview bridge or approved relay hostname.
