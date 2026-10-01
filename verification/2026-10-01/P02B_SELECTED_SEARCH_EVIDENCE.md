# P02B: native worker, failure retention and selected standalone CLI

The actual native file-search subsystem was **built separately, installed, selected and exercised by the unchanged frozen standalone CLI and component manager**. Final independent build attempt 03 and runtime attempt 02 passed, including real Ctrl+C exit 130, invalid selected configuration failing without fallback, and removal restoring native selection. The combined startup regression passed **222/222 without retries**; earlier checks passed **178/178**, **200/200 repeated race executions**, and **32/32** selected CLI cases. Failed attempts remain preserved below.

This advances replacement of pre-existing traversal/matching behavior; it is not whole-harness compartmentalization. App Server and TUI selection are not integrated in this milestone. The previously accepted Stage A GUI remains unchanged and has no new Browser verification here. The required upstream updater remains planned.

## What changed

- `codex-rs/file-search-local-plugin` composes the real bounded `NativeSearchBackend` with the generic versioned process service. Its strict configuration accepts object/null and rejects unsupported fields/types without echoing configuration values. Native snapshot allocation charge and exact serialized frame limits remain distinct.
- `codex-rs/file-search/src/native_backend_session.rs` fixes a real error-publication race. Native shutdown could reject query admission while its terminal callback was still waiting for the adapter mutex, causing a generic `ClosedLease` to hide `ResourceExhausted`. Rejected admission now leaves that mutex and observes the retained close receipt. The original operation error wins; clean joined closure maps to `ClosedLease`, and uncertain cleanup remains an error. Abandoning this observer cannot abandon cleanup.
- `codex-rs/file-search-runtime` owns selection, shared provider/scope lifetimes, retained update and poll operations, callback delivery, and one-shot CLI completion. Selection uses `file_search/default` in the explicit component home. Missing selection uses native; invalid or failed explicit selection fails without silently falling back. Process cwd and lexical roots are preserved.
- The runtime crate now owns the `codex-file-search` binary composition. Existing matching output, exclusions, highlighting, truncation and no-pattern listing are preserved through the new route. Listing bypasses provider/catalog startup. Ctrl+C requires joined cleanup before exit 130. The first independent attempt exposed a cancellation-cause issue; the corrected candidate passed that real runtime check in the final attempt.

## Preserved failures and passing gates

All original reports remain separate. Results are unique cases unless explicitly called repeated executions.

| Gate | Observed result | Interpretation |
| --- | --- | --- |
| First native worker/component suite | 32/33 passed; one case failed twice | Unsupported non-object configuration was accepted. Strict shape validation fixed the product; the assertion was retained. Cargo.lock changed during compilation. |
| Corrected configuration suite | 33/33 eventually passed, **one flaky** | Real installed-worker exhaustion first returned `ClosedLease`; retry passed. This was not accepted as a clean gate. |
| First deterministic race regression | 174/177 passed, 3 failed; no retries | New fixtures exhausted their initial snapshot before reaching the intended callback gate. All six native-process cases passed. |
| Corrected native/worker regression | **178/178**, 0 skipped; no retries | Includes three deterministic races, real allocation-profile check, native API/process worker and matcher regressions. |
| Targeted repeated race checks | **200/200 executions** | Four tests × 50 iterations: installed-worker exhaustion plus three native races. No retries; 86 other cases filtered per iteration. |
| First facade cause/pacing build | Exit 101; no tests executed | 17 test compilation diagnostics: two missing `FileSearchProvider` imports and 15 ambiguous `assert_eq` references. |
| Corrected facade cause/pacing suite | **18/18**, 0 skipped; no retries | Corrected test imports; preserved behavioral assertions. |
| Selected CLI/runtime suite | **32/32**, 0 skipped; no retries | Includes real native CLI parity, catalog failure, no-pattern behavior, interruption, missing highlights, callback ownership and cause retention. Only Cargo.lock changed within this run. |

The native race fixtures now account for the initial directory entry: real traversal indexes the supplied root at relative path `""`. They compute the snapshot cap from the actual `FileMatch` layout plus root OS bytes, assert the healthy initial directory result, then admit a query one byte longer than the cap. A private test-only per-session gate delays the actual terminal callback. The update must remain pending until the gate releases, then preserve the exact resource error, joined cleanup, unchanged rejected identity and successful reuse of the sole reservation. Variants cover an earlier close request and a dropped update observer. No production failure is fabricated, and installed-worker assertions were unchanged.

The corrected 178-case run and stress run have identical, unchanged **174-file** source fingerprints. Their unchanged subreaper runner returned exit 0 with no runner error. The initial fixture run returned exit 100. These focused results do not imply full-workspace, new GUI or Windows runtime verification.

## Allocation policy evidence

The profile test uses actual `IndexedEntry`, Nucleo `IndexAllocationPlan` and optional highlight scratch on the 64-bit target. It verifies rejection one byte below the fixed floor, exact fixed ledger admission, real traversal with additional root payload, actual matching/highlights and joined cleanup.

| Capacity | Threads | Highlights | Fixed charged bytes | Headroom within 128 MiB |
| --- | ---: | --- | ---: | ---: |
| 100,000 entries | 2 | Off | 9,605,104 | 124,612,624 |
| 100,000 entries | 2 | On | 9,738,224 | 124,479,504 |
| 250,000 entries | 2 | Off | 19,945,136 | 114,272,592 |
| 250,000 entries | 2 | On | 20,078,256 | 114,139,472 |

The initial standalone policy chooses **100,000 entries / 128 MiB charged index storage**, with actual per-session workers checked as `2*T+2` under a 64-worker ceiling; default T=2 reserves six. The test used small real fixtures, not populated 100k/250k indexes. These are operational ceilings, not a benchmark optimum or RSS guarantee. Arbitrary path payload can exhaust the byte cap before the entry cap. Traversal metadata, stacks, allocator/runtime overhead, parsed patterns and client copies retain their documented exclusions. Native snapshot storage and wire-frame bytes use separate limits.

## Source transitions and frozen builds

[Machine-readable evidence](p02b-selected-search-results.json) records report/log hashes, source snapshot identities, changed paths, selected file bindings, commands, statuses and binary identities. Baseline labels in execution reports are comparison references; they do not identify the dirty tested tree.

Scoped native/worker Clippy and selected-runtime/native Clippy passed without source changes. Global formatting changed 15 native/runtime files after the earlier gates, then nine runtime CLI/selection files after the selected CLI gate. Formatting is not represented as a new test run. The selected CLI run changed only Cargo.lock from `9d1b163e…` to `118378e1…`; its Rust/test source stayed stable. The required Bazel lock update exited 0 without lock changes; annotation warnings remain, and this was not a Bazel build/test.

Locked manager and CLI builds passed with unchanged 60-file and 190-file scoped fingerprints. The frozen candidate is **only** these two executables, not a rebuilt Codex/App Server/TUI/GUI:

| Frozen path | SHA-256 |
| --- | --- |
| `/workspace/component-checkpoint-candidate-p02b-search-cli-20261001/codex-file-search` | `6122a5648f8d4cc23e0809146bdfd9a2d5f01d185152e93ee4517868ddd28eec` |
| `/workspace/component-checkpoint-candidate-p02b-search-cli-20261001/codex-component` | `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a` |

The milestone source is published at WIP [`651b0b87a281934e5cd7c639021049fad8d393ac`](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/commit/651b0b87a281934e5cd7c639021049fad8d393ac), tree `bb7e7996a14d14eaf11e00239eabe628916a77f2`, parent `cb977e7d664bc752c28854d468619039bd3ad167`, on `wip/p02b-bounded-search-20261001`. The remote ref and `runtime/main.rs` blob `b94364e86ccd57970e43c7babe1e7cb0aad6a2fe` were verified. Main remains `562554acf45a39cb82cd62df65cf3b6627dc9011`. All **57** published source SHA-256 and Git blob identities match the preserved archive. Exact pre-format test hashes remain separate from published bytes. Upstream remains OpenAI Codex `d42056091aded7feb1d88ac7e83972108b2aa478`; Apache LICENSE/NOTICE and applicable vendored notices are retained.

## Startup cancellation correction and second candidate

After the first external Ctrl+C failure, facade and service startup ownership now distinguish an explicit caller close from the automatic close caused by a startup error. A known clean `ClosedLease` after an earlier caller fence does not poison provider shutdown; genuine startup/release errors and cleanup uncertainty remain failures. Process startup observations preserve a genuine release cause when it follows a synthetic cancellation result.

The exploratory `p02b-facade-startup-cancellation` run passed **35/35**, but its 37 recorded paths did not capture all dependency files being changed concurrently. It is preserved as exploratory evidence, not a complete immutable-source acceptance gate.

The subsequent `p02b-selected-startup-regression` run passed **222/222**, 0 skipped, with `--retries 0`. All **203 scoped source fingerprints** stayed unchanged; the unchanged subreaper runner exited 0 without a runner error. It includes native, API, runtime, component service/process, actual worker, Nucleo and matcher checks. New cases cover explicit cancellation before startup rejection, unsolicited closure, genuine startup/release errors and uncertain cleanup. This gate is separate from the historical 178-case result and does not erase the first external CLI failure.

Scoped runtime/component Clippy then exited 0 with 52 recorded paths unchanged. Formatting exited 0 and changed eight recorded paths: five Rust implementation/test files and three SDK acceptance scripts. Their before/after hashes are retained; no post-format test rerun is invented. The locked second CLI build exited 0 with **193 scoped paths unchanged**.

The new frozen candidate directory is `/workspace/component-checkpoint-candidate-p02b-search-cli-20261001-02`. Its CLI SHA-256 is `206a4d86a661953e9a5f6fe38576bae6ea7d724fd9f84d250d6e14bb3391bacd` (20,992,384 bytes). Its manager is reused unchanged, SHA-256 `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a`. The first candidate and its failed acceptance remain preserved.

Independent attempt 02 did **not** build a corrected worker: export/compiler/metadata completed, then `cargo build --locked` exited 101 because the exported Cargo.lock needed an update. The original stderr and report are retained in `/workspace/acceptance/p02b-search-independent-02/`; frozen CLI/manager hashes stayed unchanged, all observed command processes exited, and no forced cleanup was needed. This attempt provides **no corrected external runtime proof**. At that checkpoint the corrected independent build and runtime rerun were pending. Attempt 03 below resolved this isolated export issue while retaining the final locked build; the failed attempt is preserved.

## First independent build and incomplete runtime acceptance

The separate worker build passed from exported source with an initially empty external target, `dev-small`, one build job, offline/locked dependency resolution and incremental compilation disabled. All ten local crate paths resolved inside the export; core, CLI host, App Server, TUI and the selected runtime crate were excluded. Registry/toolchain cache reuse was allowed. The worker artifact SHA-256 is `262e983e66bb815f16b68ef504cf8738b4ff400049cf4ff02672a5f3c0f3c56f`. Exported source was parked before runtime, and the install input was parked after installing the package with retained license notices.

Using the unchanged frozen manager and CLI, installation/selection succeeded and five native-versus-installed comparisons passed: lexical root spelling, exclusion, Unicode, truncation and indices off. Comparisons used exact ordered JSON, including scores, paths, roots and highlights. Both frozen binary hashes remained identical throughout build and runtime.

The next check, `external-cli-ctrl-c`, failed: expected exit 130, observed 1, with `file search interrupted; file-search provider failed: file-search owner is closing`. The full acceptance report is therefore **failed**, even though all observed processes exited and no emergency forced cleanup was needed. Subsequent acceptance steps are not claimed. The original failure is retained under `/workspace/acceptance/p02b-search-cli-independent-01/cli-acceptance.json`; independent build evidence is `/workspace/acceptance/p02b-search-independent-01/independent-build.json`.

The final attempt below passed the corrected interruption, selected-failure and removal checks. App Server/TUI and consumer/UI integration gates remain separate work. A separate read-only concern about forced runtime destruction masking a known session failure in `ShutdownGuard` is queued for P03 and remains unverified; it is not a failure reproduced by these passing race checks.

## Final standalone acceptance passed

Independent build attempt **03** passed all seven commands: export, compiler identity, metadata, normalized metadata, locked build, package assembly and shared-library inspection. It used an initially empty external target, offline dependency access, one job, `dev-small` and no incremental compilation. All ten local crate paths resolved inside the export; core, host CLI, App Server, TUI and the selected runtime crate were excluded. The final worker at `/workspace/p02b-search-external-target-03/dev-small/codex-file-search-local-plugin` has SHA-256 `7d56e0dc370641d6fdb8bd2fb1cf81fb4db123a09cbb69315c85a396e4b17ddf` (10,276,816 bytes). Source was parked before runtime, and installed-package input was parked before execution. Package LICENSE/NOTICE and third-party notices were retained.

The lock correction applies only to the isolated source export. It removes four pinned remote patch declarations proven unused in the full resolved export lock: crossterm, tokio-tungstenite and tungstenite under crates-io, plus the duplicate tungstenite declaration under its SSH source table. Each package name was absent from the full package-record array and matched an exact unused Git revision. Path patches, used patches and ambiguous declarations are retained. Original manifest, lock and inventory were preserved; the export inventory records the transformation. Re-running metadata verified **exact full package records unchanged**, including dependencies/checksums, and revalidated local paths/closure. No new dependency identity was introduced. The final `--offline --locked` build preserved the resolved lock bytes (SHA-256 `dbe166dc08aa0c038fd4e7d9c5625e3f0a66af08b0c9e61c014b1f1192c6c3ac`). This did not modify the original repository manifest or SDK exporter.

`p02b-external-lock-format` exited 0. Its six-path snapshot records formatting changes in `build_worker.py`, `lock_patch_normalization.py` and its test file; exact transitions are in the JSON. These tooling changes are distinguished from the earlier native/CLI test bytes.

Runtime attempt **02** passed all **24 command checks** with their expected statuses using frozen candidate 02. Five installed-worker cases matched native output exactly—lexical roots, exclusion, Unicode, truncation and indices off—including ordered scores/paths/roots/highlights. Real Ctrl+C was sent to the CLI alone after observing the installed worker: **exit 130**, no partial output and all observed command processes gone. Unsupported selected configuration produced **exit 1 without native fallback**. No-pattern listing still worked. Removing the selected package affected future selection, retained its immutable object, and all five post-removal comparisons again matched native behavior.

The CLI remained `206a4d86a661953e9a5f6fe38576bae6ea7d724fd9f84d250d6e14bb3391bacd`; the manager remained `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a` before/after build and runtime. Both outer subreaper reports exited 0 with no runner error. Every recorded build/runtime command reported all observed PIDs absent, no timeout and no emergency forced cleanup. The 7/24 numbers count commands, not additional Rust test cases.

Authoritative reports: `/workspace/acceptance/p02b-search-independent-03/independent-build.json` and `/workspace/acceptance/p02b-search-cli-independent-02/cli-acceptance.json`, with sibling outer `...-subreaper.json` reports. Their hashes, commands, package/worker identities, normalization record and limits are retained in [the JSON evidence](p02b-selected-search-results.json).

This proves the independently installed native replacement for the **standalone one-root CLI**. It does not add App Server/TUI selection, new GUI/Browser verification, arbitrary custom replacement coverage or whole-harness completion. The prior Stage A GUI remains unchanged; updater work remains planned. The verified source WIP above preserves this standalone checkpoint; later consumer work is not included in these acceptance claims.

The source archive `/workspace/recovery-backups/20260930T165936Z/p02b-selected-cli-tested-source.tar.gz` has SHA-256 `b077447af6c7d80e09c371a11f07f95e55e70803e3819d62d0ab09b7ac1bdb36`. Its checksum and all 57 file identities were verified. This archive, `/workspace` execution reports and candidate binaries remain cloud-local, not user-downloadable or externally durable merely because they have checksums. GitHub provides external durability for the included source at the verified WIP ref. No new Browser/GUI run, new live installation update or updater implementation is claimed here.
