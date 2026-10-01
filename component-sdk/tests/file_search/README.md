# External native file-search worker and unchanged CLI acceptance

This is a verification harness; its presence is not completed runtime evidence.
Use a tested, frozen standalone CLI and record each actual run separately.
These scripts do not edit the checkout, build a host, or publish anything.
It uses the actual `codex-file-search` CLI moved into `codex-file-search-runtime`,
the actual component manager, the SDK exporter, and the actual native worker.
No substitute protocol probe supplies the success evidence.

## Resource decision before running

The last observed shared resources during preparation were roughly 1.5 GiB free
on overlay, 1.7 GiB free on tmpfs, and a 16 GiB cgroup memory ceiling. These are
historical observations, not a reservation. Do not overlap this build with root's
Rust checks. Root is auditing old generated caches separately; these scripts do
not delete caches, source, or evidence.

Use a **new target directory outside the checkout**, preferably overlay after
reviewed cache reclamation. Use the existing `dev-small` profile (no debug
information, stripped symbols), one Cargo job and no incremental compilation.
Registry/git source and pinned toolchain caches may be reused; no host target
or local-source compiled artifact is reused. Source export was previously
planned at 183 files / approximately 1.8 MiB; the new export inventory is the
authoritative tested version.

The runner checks at least 2 GiB free on the target filesystem, 256 MiB free for
artifacts, and 768 MiB current cgroup headroom. These are conservative admission
thresholds, **not a measured peak guarantee**. A fresh target might exceed them.
Do not silently fall back to the shared host target if resources are insufficient.
The report retains pre/post resources. On build failure preserve the entire run
and choose a new run directory for a corrected attempt.

## Concrete invocation sequence

Fill in the two exact, already verified binary paths. Freeze the CLI and manager
through build and runtime acceptance; do not relink or replace their backing
files. Use the same pinned Rust environment as the successful native tests.
There is no need to rebuild `codex`/App Server/TUI for this standalone milestone.

```sh
task_repo=/workspace/codex-harness-everythings-a-plugin
task_stage="$task_repo/component-sdk/tests/file_search"
task_cli=/ABSOLUTE/PATH/TO/VERIFIED/codex-file-search
task_manager=/ABSOLUTE/PATH/TO/VERIFIED/codex-component
task_build=/workspace/acceptance/p02b-search-independent-01
task_target=/workspace/p02b-search-external-target-01
task_runtime=/workspace/acceptance/p02b-search-cli-independent-01
```

First inspect only the plan; this neither exports nor invokes Cargo/Rust:

```sh
python3 "$task_stage/build_worker.py" --plan \
  --repo "$task_repo" --cli "$task_cli" --manager "$task_manager" \
  --work-dir "$task_build" --target-dir "$task_target"
```

Then, only after root admits the resources and serial Rust slot, perform the
independent build under the **unchanged** repository subreaper runner:

```sh
python3 "$task_repo/component-sdk/tests/subreaper_runner.py" \
  --report /workspace/acceptance/p02b-search-independent-01-subreaper.json -- \
  python3 "$task_stage/build_worker.py" \
    --repo "$task_repo" --cli "$task_cli" --manager "$task_manager" \
    --work-dir "$task_build" --target-dir "$task_target"
```

The build script executes this SDK export (no secondary harness import):

```sh
python3 "$task_repo/component-sdk/rust_component_package.py" export \
  --repo "$task_repo" --package codex-file-search-local-plugin \
  --output "$task_build/source"
```

It runs platform-filtered offline Cargo metadata in `source/codex-rs` using the
same fresh target. The copied lock may prune unused dependencies; new/changed
package identities/checksums are rejected. All local dependency manifests must
be inside the export. To avoid Cargo's unstable ordering of duplicate unused
patch receipts, the script removes only pinned Git patches proven unused by the
full resolved lockfile. It preserves the original export, manifest, lock and
inventory, records every removal, reruns metadata, and requires identical full
package records and dependency graph. Path patches and ambiguous or potentially
used patches remain. This normalization applies only to the isolated copy.
The final locked build must leave its resolved lock bytes unchanged. It executes:

```text
cargo build --offline --locked --profile dev-small -j 1 \
  -p codex-file-search-local-plugin --bin codex-file-search-local-plugin \
  --message-format=json-render-diagnostics
```

`CARGO_TARGET_DIR` is explicitly the new target and `CARGO_INCREMENTAL=0`.
Core, runtime selection, App Server, TUI and CLI packages must not resolve as
worker dependencies. The actual compiler artifact must be newly compiled with
its source inside the export and binary inside the fresh target. The SDK then
assembles `native.file-search-local`, version `0.1.0`, contract `file_search1`:

```text
rust_component_package.py assemble --repo SOURCE --binary ACTUAL_ARTIFACT \
  --output PACKAGE --id native.file-search-local --kind file_search \
  --name default --contract-version 1 --version 0.1.0
```

Apache LICENSE/NOTICE and inventoried modified Nucleo/Matcher MPL sources/notices
remain in the package. The dev-small artifact is already stripped; the package
is not changed after inventory. The source export is renamed `source.parked`,
making the original build source paths unavailable while preserving the work.
This is **preserved local source**, not a claim that all source was erased or
that an external backup was created. The target is also retained.

After build and outer subreaper both succeed, run the real CLI gate:

```sh
python3 "$task_repo/component-sdk/tests/subreaper_runner.py" \
  --report /workspace/acceptance/p02b-search-cli-independent-01-subreaper.json -- \
  python3 "$task_stage/cli_acceptance.py" \
    --repo "$task_repo" --cli "$task_cli" --manager "$task_manager" \
    --build-report "$task_build/independent-build.json" \
    --work-dir "$task_runtime"
```

No implicit retries occur. Pass requires each inner report and both unchanged
outer subreaper reports to pass. A process-list observation alone is insufficient.
Root should combine these reports with the exact CLI source/build/test evidence;
the binary hash identifies the tested host but does not independently establish
its source-to-binary provenance.

## Runtime cases and exact assertions

1. Baseline the unchanged CLI with a fresh isolated `CODEX_HOME` and no selection.
   A real filesystem fixture contains `.gitignore`, excluded paths, Unicode,
   scoring candidates and bounded traversal work. `-C ./project//` must retain
   its original lexical root while the process base remains the caller cwd.
2. Verify JSON results include actual score, relative path, match type, root and
   character highlight offsets. Check ignore behavior, an explicit exclude,
   Unicode query, limit/truncation marker and omitted indices when disabled.
3. Install a byte/mode-identical package copy using the real manager; select
   `file_search default native.file-search-local`. Verify immutable object
   manifest and every copied file. Park the install-input copy. The original
   package remains preserved as build evidence, but the observed executable must
   be the installed object, not the build artifact or input package.
4. Run every baseline query again. Compare entire ordered JSON values, including
   scores and lexical roots. No sorting, score normalization or path cleanup is
   allowed. Observe the actual installed worker via `/proc/PID/exe` in at least
   one real selected query and require all tracked descendants absent afterward.
5. Request real CLI Ctrl+C only after observing the installed worker. Signal the
   CLI alone, require its clean-cancellation exit 130 and no partial result,
   then require all observed PIDs absent. A signal exit or forced cleanup fails.
6. Put an unknown config option in this temporary home's selected worker config.
   Require the actual native worker's initialization rejection, nonzero status
   and no fallback search output. Preserve no-pattern directory listing even
   with that invalid selected worker; it must not start the worker.
7. Restore the temporary configuration, remove activation via the manager, and
   verify future invocations match all native baselines. The old immutable
   object must still exist unchanged; no removed worker may start.
8. Compare exact CLI and manager SHA-256/size/mode/mtime before export, after
   worker build, and after install/use/removal. Every command records actual
   status, logs, process identities, descendants and strict PID absence.

The monitor does not read process arguments or environments. It only tracks
descendants of the command and executable identity. A timeout initiates failure
cleanup and can never produce a passing test. Zombies are counted as present;
the unchanged subreaper must reap them. Emergency signals do not count as normal
lifecycle proof and are explicitly recorded.

## Expected preserved artifacts

```text
/workspace/acceptance/p02b-search-independent-01/
  independent-build.json
  source-inventory.json
  source.parked/                 # actual exported source and resolved lock
  package/                       # binary, manifest, notices, covered source
  export.{stdout,stderr}
  metadata.{stdout,stderr}
  build.{stdout,stderr}           # actual Cargo compiler-artifact evidence
  assemble.{stdout,stderr}
/workspace/p02b-search-external-target-01/
/workspace/acceptance/p02b-search-independent-01-subreaper.json
/workspace/acceptance/p02b-search-cli-independent-01/
  cli-acceptance.json
  fixture/                       # real paths used by both implementations
  home/components/objects/...    # removed activation's preserved immutable code
  install-input.parked/
  native-*.{stdout,stderr}
  external-*.{stdout,stderr}
  selected-failure-*.{stdout,stderr}
  removed-native-*.{stdout,stderr}
/workspace/acceptance/p02b-search-cli-independent-01-subreaper.json
```

These paths are in-workspace checkpoints. Root must separately include coherent
source and selected evidence in the authorized repository publication; none of
these scripts claims a durable external backup or performs a Git write.

## Scope and remaining gates

This proves a separately built native search replacement through one real CLI
binary, provided all commands pass. It does not prove arbitrary custom backend
conformance, full-harness extraction, GUI search, TUI/App Server routing, mixed
roots (the CLI accepts only one), live replacement, cross-platform packages or
production-scale sizing. Native/API lifecycle and capacity regressions remain
separate prerequisites. GUI and Browser validation are owned by root; no browser
test is simulated here. An independent maintenance/upstream-update component
remains governed by the canonical roadmap.
