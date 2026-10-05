# Native build-worker handoff and source custody

Infrastructure-only checkpoint, 2026-10-05. **All product work is paused.** A
separately provisioned, expandable worker is now an allowed capacity solution;
preserve the original environment, both workspaces and every recovery artifact.
This document prepares that handoff. It provisions nothing, transfers no private
payload and establishes no build or runtime acceptance. The accompanying
[manifest](verification/2026-10-05/native-worker-handoff/MANIFEST.json) binds exact
source/control identities and private evidence references without their contents.

Continuation: cloud environments remain primary; Proxmox provides explicitly
offloaded builds and durable recovery. Product work resumes automatically after
the infrastructure acceptance gate in [COMPLETION_HANDOFF.md](COMPLETION_HANDOFF.md).
The immutable references below remain the source assembly baseline. Publication
`47b225fc3b5ee4bd3cf1b37e056f5818f58a54ac` added only five coordination files; it did
not publish the eight-file overlay or private runtime controls. New private packet
preparation preserves exact originals and does not make those controls portable or
approved for public export. The [continuation readiness receipt](verification/2026-10-05/completion-handoff/MANIFEST.json)
records 32 privately staged payloads/1,550,203 bytes with exact readback, including
the full canonical source map. The resumed bounded static review covers 31 selected
source/control files; [bootstrap requirements](verification/2026-10-05/completion-handoff/BOOTSTRAP_CONTROLS.md)
retain concrete source-envelope, observer, companion-input, notices and private-log
requirements. It grants no payload export. No worker access, transfer or native pass
is implied.

The later [static companion selection](verification/2026-10-05/worker-companion-selection/README.md)
closes the selected local-file/import inventory and exact pinned-source routes.
It distinguishes historical inputs from worker-generated proof and retains
portable-control, acquisition and admission gaps. VM105 memory/storage commitments
are still under independent infrastructure review, not granted resources.

## Immutable source references

| Reference | Identity |
|---|---|
| Repository | `DesignStuffDev/codex-harness-everythings-a-plugin` |
| Official upstream import / original local HEAD | `d42056091aded7feb1d88ac7e83972108b2aa478` |
| Main, observed unchanged | `781080f7e3c8bfe1953378001d777dff33d74bc3` |
| Existing WIP branch | `wip/p03-process-final-and-mcp-preservation-20261002` |
| Published assembly base before this documentation | `c0412a00d90cca162e7a54b5c6aa06cb67979f55` |
| Its tree / parent | `72afcf70ca516071911c4732ec22ec5c90eff3b3` / `95359037b45621f54c4284c049873f0834f3f123` |
| Accepted native implementation reference | `914cc59374c1149463e78bc33851d83e3f14d0a4` |
| Accepted native source map, 8,949 paths | `42e3899a688183ae740926d188204ea1222d24f79db1e4b9cb37d51aa046dc59` |
| Current unaccepted candidate map, 8,980 paths | `b75efc9751977009a9b28f2893c120db45a8f6e79f4993125e3685ae90d6cf5b` |
| Original index, retained on original VM | `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59` |

The complete published tree contains 12,059 entries / 10,693 blobs and is not
truncated. A source-only comparison read 10,371 nonignored files (124,599,106 bytes),
verified every candidate hash and compared Git blob identities/modes. No warm cache
was scanned. The original checkout's local HEAD is upstream; its local object store
does not contain c041. Never use `git diff HEAD` alone as the custom-platform delta.

## Exact working source and separate preserved work

The candidate differs from c041 in exactly eight paths, all under
`codex-rs/login/src/auth/`:

| Path | Operation |
|---|---|
| `auth_install_storage_tests.rs` | Add |
| `auth_install_tests.rs` | Modify |
| `auth_reload.rs` | Modify |
| `auth_source.rs` | Modify |
| `ephemeral_install_store.rs` | Add |
| `ephemeral_install_store_tests.rs` | Add |
| `storage.rs` | Modify |
| `storage_telemetry.rs` | Modify |

All five preimages equal the published blobs; all three new paths are absent there.
Current SHA256, Git blob ID, mode, size and base identity are in the manifest. Keep
the complete store/caller cohort together. The formatted change has 596 changed
lines; the older 432-line proposal is a different snapshot. Both native attempts
ran zero tests. Formatting and source custody do not establish safe behavior.

There are 674 required source paths absent from upstream HEAD: 668 already match
c041, three already-published additions are modified by this cohort, and three are
new. Starting from upstream plus eight files would omit the custom platform.

Nine further unpublished differences are separate custody: one status document
and eight verification Python helpers. Their base/current identities are listed
in the manifest. Bounded static recipe/import inspection found none selected by
the planned native/runtime commands; this is not a dynamic isolation proof. Preserve
their exact bytes in a non-operative holding artifact, without silently adopting
them or assigning published test results to them.

Retain all 325 published paths absent from the local nonignored inventory: one
WORK_IN_PROGRESS file, 254 `_preserved_wip` paths and 70 verification records.
Ignored records may still exist locally. Assemble from the published base; never
use deletion mirroring to erase these inherited files. Keep LICENSE, NOTICE,
upstream/custom provenance, AGENTS, SDK, desktop package, component sources and locks.

Older auth/MCP/collector alternatives remain isolated. The sibling workspace has
543 dirty-source paths; 541 match the initial recovery manifest, while README and
WIP_STATUS have separately recorded current hashes. Its older publication
`0b38d5974159ab2a8776200025dbb602e89b8f8f` is custody history, not this worker's base.
No fresh complete sibling-branch readback or private-archive payload audit is claimed.

## Minimal source transport and assembly

1. Obtain authenticated command/file access to the future worker and independently
   admit its clean-build storage, RAM, inodes and expansion/retention arrangement.
   No worker endpoint or usable access mechanism has yet been supplied to this task.
2. Use the existing GitHub source path. Prepare a reviewed, explicitly **UNVERIFIED
   source-handoff** commit/branch containing the coherent eight-file overlay and a
   portable source-map manifest, based on the exact c041 tree or a documented
   documentation-only descendant. This checkpoint publishes no candidate source.
3. Read back that immutable source commit/tree. On the future worker, verify base
   identities, five preimages, three additions, licenses and every resulting source
   hash. The 8,980-path projection must match the intended candidate; additional
   inherited preservation files stay present. A moving branch name is insufficient.
4. Carry the nine other variants as separate identified custody when transferring
   complete unpublished work. Keep older proposals inert. Do not copy the original
   Git index/config or damaged target: the new checkout owns a new index and cache.
5. Review the small private test-control sources for content/license/secrets,
   adapt their machine bindings in separate derived files, then use the authorized
   source path for reviewed controls. Preserve originals and bind each derived hash.
   Raw logs, databases, readiness URLs and live configuration are excluded.

GitHub connector read/write works for source coordination. It is not proof that
the future worker can fetch Git or download dependencies. The original executor's
separate REST attempt was denied at proxy CONNECT before TLS; it was not retried.
No cloud-to-home archive path is needed for this source-first bootstrap.

## Toolchain and dependency contract

- Target: native Linux `x86_64-unknown-linux-gnu`; Rust **1.95.0**, with clippy,
  rustfmt and rust-src. Recorded tools: just **1.58.0**, nextest **0.9.146**,
  cargo-insta **1.48.0**, dotslash **0.5.7**. These are metadata pins, not executions
  of the tools during this handoff.
- `codex-rs/Cargo.lock` SHA256
  `0baf51d1b779c63b0ea6257f5292a73913b92c8e86c34a111f21cbeb95ab6f55`
  binds 1,487 packages: 1,295 registry, 16 Git packages from seven pinned sources,
  and 176 local packages. Retain workspace patches, vendored paths, MODULE files,
  native build scripts and all relevant locks. The manifest lists Git revisions.
- V8 **150.4.0**, Linux `ptrcomp_sandbox_release`, uses the official pinned release
  manifest and two exact artifact hashes. GStreamer **1.28.7** package/sysroot
  receipts and safe compiler/pkg-config/linker settings are preserved separately.
  Fetch these from their pinned official origins; never inherit broken cache aliases.
- The observed base is Debian **13.6**, x86_64, with GCC/G++ **14.2.0** package
  metadata and a custom native sysroot. This is not a complete immutable image.
  The infrastructure implementation must pin its image and full native-library,
  compiler/linker and loader closure and attest the resulting binaries.
- GUI fixture prerequisites: Node, Playwright core **1.57.0**, Chromium and runtime
  libraries. Observed Chromium package is **151.0.7922.173-1~deb13u1**. Inherited
  Python uses a 3.12 runtime path; installed system Python 3.13.5 is distinct. Choose
  and record actual worker executables; do not assume paths imply versions.

Future acquisition uses a new CARGO_HOME, authorized Git/crate/tool/native-asset
downloads and `cargo +1.95.0 fetch --locked`, preserving the root lock. Resolve the
full required metadata closure before offline validation; metadata is not a build.
Preserve jobs=1, incremental=0, dev/test debug=0, package/feature groups and the
original-relative host target. Compiled target/fingerprint/tmpfs data never moves.
No historical warm-cache allowance admits a clean worker's build peak.

## Future native validation sequence — not dispatched here

Run from the worker's `codex-rs`, with fresh source/resource receipts and the strict
subreaper owner. The private exact argv templates are hash-bound in the manifest.
The focused filter is
`package(codex-login) & (test(auth::manager::auth_install_tests::) | test(auth::manager::auth_source_tests::) | test(auth::storage::ephemeral_install_store::tests::))`.

```sh
just test --locked --retries 0 -p codex-cli -p codex-login --lib -E 'package(codex-login) & (test(auth::manager::auth_install_tests::) | test(auth::manager::auth_source_tests::) | test(auth::storage::ephemeral_install_store::tests::))' --test-threads=1
just test --locked --retries 0 -p codex-cli -p codex-login --lib --test all -E 'package(codex-login)' --test-threads=1
just test --locked --retries 0 -p codex-cli -p codex-login -p codex-model-provider --lib -E 'package(codex-model-provider)' --test-threads=1
just fix -p codex-login --locked
cargo build --locked --offline -p codex-cli --bin codex -p codex-component-host --bin codex-component
```

The templates add source/strict receipts and unchanged cleanup wrappers; they do
not attach disk/cgroup sampling. The recorded resource observer is an additional
bootstrap input whose worker selection/adaptation and new identity remain pending.
The inner commands alone are not complete acceptance. No direct `cargo test`, unapproved full
workspace suite, new feature/profile combinations or weakened lifecycle assertions.
Follow repository fix/format rules; source-changing lint requires an explicit new
source/evidence decision, never silently rebinding earlier passes. Do not kill Rust
commands to enforce a monitor timeout. Bind actual final CLI/manager producers and
hashes; the older accepted 8e8a5dac/f054d84a binaries remain original-VM recovery.

## Independent component packages and real runtime gates

Recommended route: rebuild packages separately on the worker, using disjoint export
trees/targets and fresh proofs; shared verified dependency downloads are acceptable.
Freeze the newly built host and prove its bytes stay unchanged during installation.

| Package | Source/entrypoint | Required additional custody |
|---|---|---|
| Thread storage, contract2/package0.2.0 | `component-sdk/tests/native_storage_acceptance.py` | Preserve exported/resolved lock and source before the helper removes its own exported source; control adaptation is pending. |
| Search, contract1/package0.1.0 | `component-sdk/tests/file_search/build_worker.py` | Build `codex-file-search-runtime` / `codex-file-search` separately first; preserve existing dev-small profile, original/resolved export locks and independent report. |
| Attachment, contract1 | `codex-rs/attachment-store-component/package/build.py` | Export, record isolated lock normalization, reject new external identities, locked standalone build and package; create a fresh producer/package proof. |
| Desktop GUI, contract1 | External copy of `component-sdk/examples/desktop`, built by SDK | Install the separate package and launch through the manager against the new host; fresh private GUI home. |

Run the twelve retained runtime gates in order: storage, migration-normal,
gui-normal, attachment, migration-search, gui-search, migration-slow-normal,
slow-normal, migration-slow-forced, slow-forced, held8, featured-postcheck; then the
new auth-consumer gate. Preserve dependent migration homes and source identities.
Check real install/replacement, normal manager Launch Ctrl+C, forced durability
uncertainty, recovery, streaming, Stop, approvals, search and tracked child cleanup.
Use fresh deterministic loopback model/auth/storage fixtures, not personal sessions
or provider credentials. Chromium/Playwright fallback is real browser evidence if
executed, but not in-app Browser/manual or live-provider evidence. No viewer is a
prerequisite. Worker process/loopback/tracing capabilities must support unchanged
assertions; a skipped gate is not a pass.

The runtime recipe binds 44 helper inputs, including 14 privately retained sources.
The later exact-blob comparison corrects the earlier "unpublished" classification:
12 of those 14 already match published source; two have no match in the pinned tree.
This is not the complete runnable bootstrap closure. Four environment scripts and
the resource observer are separately identified in the manifest. Known
portability gaps are absolute root/Python paths, local main-reference assumptions,
trusted Git/strace hashes, source-absence and package timestamp assertions, and old
production/proof links. Preserve semantic ownership/deadline assertions while
rebinding machine facts. The nine differing verification files are not silently
substituted for these controls. New builds produce new hashes and independent
proofs; old reports cannot be edited into new acceptance.

## Unique custody versus reproducible inputs

Preserve the original two workspaces, untracked proposals, original Git state,
adoption/preimage journals, failed outputs/fingerprints, accepted binaries and all
archives. Raw conversations, attachments, databases, configuration, logs and viewer
credentials stay private. Existing archive hashes/restoration receipts establish
their historical recovery scope, not an offsite copy of every latest change.

Upstream objects, pinned public dependencies and toolchains can be reacquired.
Targets, registry caches and fingerprints are unnecessary to bootstrap this worker;
their current originals remain protected until a separate retention decision.
The 28-archive proposal remains unchanged, with zero certified reclaim. Private
artifact movement still requires an exact approved destination, scoped access,
encryption/key custody, manifest/checksums, independent download/full restoration
and explicit retention approval before any original is retired.

## Next infrastructure implementation decision

The focused infrastructure owner must supply a retained worker proposal with an
authenticated command/file-access method, durable expandable storage and independently
admitted clean-build resources, plus demonstrated GitHub and pinned dependency
access. Then prepare the reviewed source/control handoff and reproducible bootstrap
definition, with the original VM retained. The missing worker access and portable
bootstrap/control implementation are concrete prerequisites; this report is not a
ready-to-run image, completed source transfer or successful native capacity test.
All product extraction, collector/updater work and original-VM Rust attempts remain
paused. Only nonsecret coordination evidence is published by this checkpoint.
