# TUI Preparing cancellation: passing focused cases, failed full regression

On October 1, the initial TUI consumer candidate reached five passing new lifecycle test functions (nine authored scenarios), plus two passing owned-fixture regressions. The full regression **did not pass**: retry02 executed **5,650 tests: 5,633 passed, 17 failed, 8 skipped**, exiting100. The preceding attempt failed during linking with SIGBUS, exiting101 before any tests ran. Both used the unchanged strict subreaper runner.

## Component behavior exercised

The production TUI coordinator retains exact pending-start control, keeps nonempty B/C input on one pending lease, resamples the newest query at handoff, and drains retired startup before admitting a replacement. Empty input, root changes, restart and manager drop cancel outside state locks. Preparing/in-flight observation also wakes when the UI receiver closes. Real operation failures remain distinct from cleanup uncertainty.

The five passing new tests cover retained cleanup with abandoned observers; B/C coalescing; old-root cancellation before new-root handoff; genuine ClosedLease/SearchFailed receipts after retirement; and receiver drop while startup is blocked. Two owned-fixture tests pass for unpolled/runtime-loss uncertainty and sibling-independent abandoned startup. These use a controlled backend through the production facade. They do not prove native constructor interruption, an installed worker in a new full CLI, or graphical UI/browser behavior. The idle-ready receiver-drop wake gap remains outside this slice.

## Full regression counterevidence

Nine failures show explicit SQLite disk-full/disk-I/O or OS ENOSPC errors, including the worktree-fork terminal screen. Four cursor/render checks fail under an observed NO_COLOR=1, TERM=dumb environment. Three CLI children exit1 while the tests await text, without a specific cause established by those logs. One integration cannot locate the full `codex` CLI fixture. These are observations, not dismissals; all17 remain failed until corrected gates run. The exact test names and log locations are in the results JSON.

The strict retry runner records631 reaped entries: the command plus630 adopted descendants. Return codes include283 SIGPIPE(-13),46 code141,162 code128,139 code0 and the command's code100. This proves the recorded runner completion, not all-zero descendant exits or a passing lifecycle suite.

## Exact source lineage

All seven initial consumer-stage files match their frozen candidate hashes in both runs' before/after source maps. The only change between actual runs removes an unused `use std::future::Future;` from `test_start_tests.rs`; reconstructing the earlier bytes reproduces its original hash. The separate fixture stage was formatted before the first run, so its original authored bytes are not described as a one-line-only difference.

Retry02 correctly reports `scoped_source_unchanged=false`: two failed `.snap.new` outputs were added. No existing source or expected snapshots changed during the run. Both generated snapshots were independently verified in their byte-preserving archive and then removed by root; expected snapshots were never accepted or changed. Their archive and original failure evidence remain cloud-local.

Source reports label baseline `f2cc6c2ed7dd84f606d43947a3c71f85bfc0daad`, but the tested tree was dirty; this document binds the actual source fingerprints instead of claiming that commit alone was tested. The historical frozen-stage manifest still accurately says uncompiled/zero tests at its creation. Later PublicStop adoption is outside these tested snapshots.

## Next acceptance gates

Restore stable `/tmp` and overlay capacity without losing source or failed evidence. Run the combined AS/client/TUI library gate with NO_COLOR unset. Build the new full CLI, bind CARGO_BIN_EXE_codex to that exact artifact, and run the complete TUI integration suite. Keep installed-worker and graphical UI/runtime acceptance separate. No failure expectations or snapshots are weakened by this evidence checkpoint.

Companion records: [results](p02b-preparing-tui-consumer-results.json) and [source lineage](../../upstream/p02b-preparing-tui-consumer-lineage.json). Their raw report/archive references distinguish cloud-local files from published evidence.
