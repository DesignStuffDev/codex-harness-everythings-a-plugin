# P01 manual rollout migration: focused test evidence

Status at this checkpoint: **625 distinct Rust test cases have passed across focused runs**, including **44/44** in the final component/worker lifecycle rerun. This is the union of cases identified by nextest binary ID and test name, not a single 625-case run. Earlier failed attempts remain recorded below. **Independent external-package, exact-host runtime, old-package compatibility and corrected GUI regression also passed.** GUI checks used Chromium through Playwright plus visual screenshot inspection; the requested in-app Browser was unavailable and was not used.

The companion [p01-test-results.json](p01-test-results.json) records every passing case, its last successful log, run results, runner commands and exit status where available, source manifests, and SHA-256 digests. It contains no raw private logs, bearer URLs, tokens, or runtime session content.

## Source and evidence attribution

- Official upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`.
- Accepted P01 implementation: `5d2f2a026ae6ce0c42cf56bb2f3b01971e4bf0bb`; exact blobs and upstream anchors are in [P01 lineage](../../upstream/p01-migration-lineage.json).
- Comparison base: published roadmap checkpoint `83364aa63526d75840d4149f4598ba1aeed02a06`.
- Actual post-format source checkpoint: `p01-source-20261001T005826Z.json`, **47 changed paths**. Manifest SHA-256: `6f088fd73a19d266c32a6b943d12bfdee8b11a55539cfd83dc1976923c9bd14e`.
- Matching source archive: `p01-source-20261001T005826Z.tar.gz`, SHA-256: `1b6b460943826303bc5238cebf88ed972c6b882e4cdf4d228fb82ed322158f68`.
- Original logs, runner reports, manifests and incremental source archives are in `/workspace/recovery-backups/20260930T165936Z/`. These are same-cloud recovery artifacts, not independently hosted downloads.

The manifest describes its original capture as `unverified_in_progress_source_snapshot`; this report does not rewrite that history. Earlier manifests beginning `p01-source-20261001T003230Z` through `004343Z` preserve intermediate states and are enumerated in the JSON. Tests ran during incremental development, so their scope and ordering matter. Attribution uses these actual manifests, not stale source labels from an older regression wrapper script. At the initial evidence audit, only the evolving `EXECUTION_STATE.md` differed from the `005826` manifest; the production/test source still matched.

## Distinct passing cases

| Actual nextest binary ID | Distinct passing cases | Evidence |
| --- | ---: | --- |
| `codex-thread-store` | 279 | `p01-storage-regression.log` |
| `codex-thread-store-component` | 23 | storage regression and final lifecycle rerun |
| `codex-thread-store-local-plugin::process_storage` | 21 | worker regression and final lifecycle rerun |
| `codex-cli::bin/codex` | 295 | `p01-cli-corrected.log`; includes cumulative progress snapshot |
| `codex-cli::migrate_rollouts` | 5 | corrected/fixture CLI runs plus the final cold-readback pass |
| `codex-cli::migrate_rollouts_metrics` | 2 | corrected and fixture CLI runs |
| **Total** | **625** | Repeated runs/retries are deduplicated. |

The intentionally ignored CLI subprocess fixture `doctor::filesystem_paths::tests::blocked_probe_fixture` is not counted. Its parent bounded filesystem probe test passed. Filtered tests in focused runs are also not counted as passes. The two metrics tests execute ten native/selected CLI scenarios internally; these remain **two** test cases.

## Behavior exercised

The native/manual path tests exercise dry run without storage mutation, filtered/per-path outcomes, empty work, invalid limits, busy admission and recovery, cancellation before admission, cumulative progress, and cancellation/abandoned close while real SQLite publication is blocked. The corrected blocking fixture waits for both the paginated rollout and its pending journal while holding the metadata write lock, establishing that accepted publication is actually in flight. Cancellation joins that work without advancing the startup migration cursor.

Component tests exercise optional/versioned migration capability compatibility, start/release races, released-before-start requests, bounded lease/tombstone capacity, idempotent cleanup, shutdown signaling and joining, retained cleanup failures, and observations finishing before consuming close. The process tests additionally exercise the native worker through the installed storage boundary, process restart, shutdown with an unreleased migration, real cancellation, non-UTF-8 paths, large-history process transfer, existing timeline/context fidelity, pending metadata, fork/delete conflicts, collections and attachments.

Migration-report size coverage is a codec case; a dedicated >4 MiB migration-report IPC case was not run.

The CLI tests run the real `codex` binary against native storage and a selected installed native storage worker:

- Dry-run and failure reports match, with storage bytes unchanged.
- Apply honors the thread filter and preserves the other rollout; after CLI/worker exit, a newly opened native store reads both the paginated UI projection and supported model resume context, preserving the question and answer. Reports and normalized histories match between backends; repeated Apply reports `already_paginated`.
- An explicitly selected worker lacking the optional migration capability fails without native fallback or native metadata initialization.
- First interrupt requests cancellation and waits for an accepted SQLite publication before returning the non-success cancellation result and its completed report prefix.
- Repeated interrupt reports durability uncertainty, terminates/reaps the selected worker, and a subsequent migration can recover. Strict process-disappearance checks are retained.
- Inline snapshots cover the cancellation notice/report shape; a unit snapshot checks cumulative progress without double counting.

The metrics tests collect real CLI OTLP HTTP JSON exports and compare exact run/thread counters and tags for successful dry run, filtered Apply, partial failure, busy failure, and cancellation. They require exactly one run counter, detecting host/native double counting; durations are excluded from parity. Collector timing is set only on the child process. Injected proxy/security settings are unchanged.

These Rust installed-worker tests package the built worker for a fixture. The separate external-source acceptance below provides the additional no-host-rebuild proof.

## Attempts, failures and corrections

| Log | Actual result | Resolution or significance |
| --- | --- | --- |
| `p01-native-manual-first.log` | Compile failed; no tests ran | Missing test enum import and a non-`Send` watch borrow across an await were corrected. |
| `p01-native-manual-rerun.log` | 5 passed, 2 failed; retries also failed | Blocking fixtures used invalid session timestamps and insufficient admission evidence. Fixtures now use valid native records/backfilled state and wait for paginated rollout plus pending journal under the state-DB lock. |
| `p01-native-manual-corrected.log` | 7/7 passed; 272 filtered | Corrected native manual cases. |
| `p01-component-first.log` | Compile failed; no tests ran | Three `State`/`MutexGuard` type mismatches in migration failure recording were corrected. |
| `p01-storage-regression.log` | 302/302 passed | Full native thread-store and component test binaries at that stage. |
| `p01-worker-regression.log` | 21/21 passed; runner exit 0 | Real worker process regression. |
| `p01-cli-first.log` | 4 passed, 2 failed; runner exit 100 | Cold readback incorrectly called the legacy full-history API on a paginated thread; cancellation snapshot required `insta::allow_duplicates!` for the native/selected loop. |
| `p01-cli-corrected.log` | 301 passed, 1 failed, 1 ignored helper; runner exit 100 | Remaining cold-readback fixture contained display events without a complete model turn. The other CLI cases, all 295 CLI unit cases and both metric tests passed. |
| `p01-cli-final.log` | Compile failed; runner exit 101 | Complete turn fixture initially supplied `String` message IDs; changed to the real `ResponseItemId` constructor. |
| `p01-cli-fixture-final.log` | 7 passed, 1 failed; 295 filtered; runner exit 100 | Cold-readback assertion inspected serialized byte arrays instead of decoding each persisted `item_json`. The test now decodes JSON before checking UI content; model-context and UI-content assertions remain. |
| `p01-cli-cold-final.log` | 1/1 passed; 4 filtered; runner exit 0 | Final native/selected Apply, cold readback, filter preservation and repeat-Apply case. |
| `p01-final-lifecycle.log` | 44/44 passed; runner exit 0; no retries | Revalidated all 23 component and 21 process cases after the final migration-service observation/lock-scoping changes. |

No failing case remains without a subsequent pass in this recorded set. The final lifecycle run covers the production lifecycle changes made after earlier broader focused runs. It does not retroactively turn failed earlier runs into successful runs.

Process suites used [the Linux subreaper runner](../../component-sdk/tests/subreaper_runner.py) to adopt and reap intentionally killed worker descendants in this cloud environment. Its JSON records the actual command's exit status separately from adopted children, so a reaped child cannot turn a failed command into a pass. This does not relax strict `ESRCH` process-cleanup assertions. Original runner JSON is retained beside each applicable log.

## External source/package and exact-host runtime acceptance

The independently exported **native** `codex-thread-store-local-plugin` package `0.2.0` passed build, installation and runtime acceptance against the frozen P01 CLI and existing component manager. [The sanitized independent report](p01-independent-build-report.json) retains commands, package fingerprints and before/after host fingerprints. [The reviewed source inventory](p01-independent-source-inventory.json) is byte-identical to the original: SHA-256 `e90f4a0df9b7b3020f507ab53c15db3f9ec369f33dad09b67b5e098af9ed7c17`.

- External export: **81 local crates**, **3,227 source files**, **917 resolved packages** for `x86_64-unknown-linux-gnu`; every local dependency stayed inside the export. Core, CLI, App Server, TUI and core test support were excluded. Lock pruning introduced **zero new dependency identities**.
- Cargo compiled the native worker from the external export (`fresh: false`) with `--offline --locked`. The existing target directory and third-party cache were reused for disk capacity; this was not a cache-isolated or fully vendored build.
- The exported source directory was removed before package installation and runtime acceptance. The installed package retained Apache-2.0 `LICENSE` and `NOTICE`.
- Frozen CLI SHA-256: `467ee8360d45701bed728dead65568d6178b5b2d8c2c5b6d149ebcfb11a5ad01`; component manager SHA-256: `acffaefb470b796b47f36fd9a9d5d4c23d2b5d3622316256b6c8c770ccc8a8f3`. Their hashes, sizes and modification times remained unchanged through independent build/install/runtime tests.
- Installed native worker SHA-256: `0e691de19cf3239f702e384b7c483ecfcf1b72ba5711e196103160427cba0f99`. This is the newly compiled migration-capable package, distinct from the older compatibility package below.

[Existing headless behavior acceptance](p01-harness-runtime-report.json) passed real CLI turns, external tool execution, context contribution, streamed model events, persisted resume, selected native storage process restart, process termination, and restoration of native model/storage behavior with retained history. Inference used deterministic external-process/loopback fixtures; it did not contact a live model provider.

[Real migration runtime acceptance](p01-runtime-report.json) seeded a legacy conversation through the actual App Server, compared native/selected dry-run reports and unchanged storage bytes, applied migration through the selected independently built package, checked repeated-Apply idempotence, resumed the migrated session in a cold CLI process, read paginated history through a cold App Server, and cancelled a subsequent turn with strict storage/model process cleanup. `p01-independent-subreaper.json` and `p01-manual-runtime-subreaper.json` both record command/runner exit **0**. These are additional end-to-end gates, not extra Rust cases added to the 625 count.

[Old-worker compatibility acceptance](p01-old-worker-compatibility-report.json) also passed using the previously accepted **0.1.0** native package with storage contract **2**, unchanged. Ordinary turns/tools/context/streaming, writes, cold resume and native restoration still worked. Selecting that package for migration produced the explicit `manual_rollout_migration` unsupported error, exit **1**, empty stdout and no fallback. All **14** storage fingerprints remained unchanged; all **4** observed storage workers were absent afterward, and candidate/package fingerprints matched before and after. Its subreaper exited **0**. The helper rebuilt additive fixtures only; this gate does not claim another native-package build.

Source-inventory fields describe their original export-time observations, including that export alone is not independent-build verification. The separate successful compiler/package/runtime reports establish the later verification. Public reports normalize local paths and omit ready-file locations, raw logs, transient process/thread IDs and synthetic prompt text; full originals remain in the retained cloud acceptance directories.

## GUI failure retained; corrected regression passed

The first P01 GUI attempt completed seven browser commands, then timed out waiting for an approval. The preceding real CLI resume had persisted `Never`; the fixture assumed `on-request`. The model request showed the engine rejecting approval under that policy. The fixture now explicitly supplies `approvalPolicy: on-request` through the public cold `thread/resume` request and asserts the returned policy. This is a test setup correction, with no production policy change or automatic approval. The failed scenario remains recorded: manager SIGINT returned exit 0 with all tracked processes absent, but the private report also recorded a cleanup error and the full scenario failed. Raw errors/URL-bearing reports were not copied.

[Corrected manual runtime acceptance](p01-runtime-corrected-report.json) passed again with that policy setup; its subreaper exited **0**. [Corrected GUI acceptance](p01-gui-report.json) then passed **21/21** browser commands in the first cycle and **10/10** in the second, with zero page errors and both browsers closed. This exercised the real separately installed GUI, migrated history, streaming, approval, cancellation and cold continuation/recovery with deterministic model fixtures.

Both shutdown cycles sent the first SIGINT to the **component manager's Launch process during an active turn**. Each manager exited **0**, in approximately **0.316 s** and **0.265 s**. All tracked descendants were absent and both subreaper drains passed. The candidate binaries and installed packages remained unchanged. Process sampling covers observed descendant identities and the subreaper drain; it is not a claim that every short-lived descendant was individually sampled.

The requested in-app Browser tool was unavailable. These checks used **Chromium through Playwright**, followed by direct visual inspection of viewport screenshots. Reviewed public images show the [approval interaction](p01-gui-approval.png) and [recovered conversation](p01-gui-recovered.png); neither includes an address bar, bearer URL or token. No in-app Browser execution is claimed.

The runtime fixture's policy correction and GUI diagnostic changes occurred after the initial source checkpoint and before the successful corrected run. Final formatting also touched Python helpers. The JSON records current helper hashes separately from the **actually executed** GUI script/driver fingerprints in the GUI report; their later differences are formatting only, as recorded by the root agent. Rust sources and frozen candidate binaries had no semantic changes through these gates, and tests were not rerun solely for formatting under the repository instructions.

## Quality gates and remaining scope

- Final scoped `just fix`: exit 0, no warnings/errors/automatic fixes in `p01-clippy-final.log`. The earlier `p01-clippy.log` retained warnings that motivated the lifecycle lock-scoping/test cleanup; it is not used as the final clean result.
- `just fmt`: exit 0; `p01-format.log` is empty. Formatting followed the substantive verification/fix cycle; tests were not rerun solely because of formatting.
- `just bazel-lock-update`: exit 0 in `p01-bazel-final.log`; existing third-party annotation warnings remain. `MODULE.bazel.lock` did not change because the added local dependencies already existed in the graph. This is not a Bazel compile/test result.
- Standalone broad `codex-state`, `codex-rollout`, core, app-server and whole-workspace suites were not rerun for this P01 slice. State/rollout production crates were unchanged; integration with them is exercised by the 279 native tests, real worker tests, cold readback, held-SQLite cancellation and forced recovery. This is focused integration coverage.
- Exact-host runtime, external native package build/install without host rebuilding, old-worker compatibility and corrected Playwright GUI regression passed as detailed above. The unavailable in-app Browser check remains an explicit testing limitation.
- This slice addresses native manual rollout migration through the storage component boundary. It does not complete all subsystem extraction or the full roadmap.
