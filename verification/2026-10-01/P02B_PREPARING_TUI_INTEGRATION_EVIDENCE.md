# TUI integration verification on the new host

The selected TUI integration gate passed **23 tests, with 0 failures and 4 skipped**, using zero retries and two test threads. It ran `codex-tui::all` (22 passes) and `codex-tui::manager_dependency_regression` (1 pass). This is a separate execution from the earlier 6,095-test App Server/client/TUI library run; their counts are not combined into one result.

All five integration cases that failed in the earlier full-TUI retry now pass: external-editor draft restoration, daemon compatibility, no-daemon startup, provider-history lookup and worktree interaction. The JSON maps each original failure and new pass to its exact log entry. The earlier retry remains recorded as 5,633 passed, 17 failed and 8 skipped; its original reports and generated failed snapshots are preserved. Twelve earlier library failures passed in the separate library gate.

The 23 files under `codex-rs/tui/tests/`, all 1,366 expected TUI `.snap` files, and the TUI Cargo manifest plus nextest configuration are byte-identical across the earlier failed run's before/after maps and this run's before/after maps. No expected snapshot was accepted or assertion weakened to obtain these results. Production source and executable inputs changed through the separately documented implementation, formatting and full-CLI build.

All 10,148 scoped source fingerprints remained unchanged during this run and match the successful full-CLI build's before/after maps. The frozen CLI is `1b72a190ba6ebcca68c4f0d4145f4ec129b975e0e18fcddfab322f8e7f9165e7` (634,578,704 bytes). Its actual digest matches the artifact receipt and integration preflight. A separately hashed retrospective transcription of the root shell invocation records `CARGO_BIN_EXE_codex` set to that frozen executable. The original wrapper records the command, while the separate preflight records the CLI digest; neither captures the inherited variable; the JSON preserves that distinction. Some helpers prefer a dedicated `codex-tui` executable, so this is not a claim that every test child was the frozen CLI.

The source wrapper, test command and strict subreaper exited 0 with no runner error. The JSON preserves all 39 reap records: 10 exit 0, 15 return −13, 9 exit 128 and 5 exit 141. Those raw records do not imply every descendant exited successfully. `NO_COLOR` was explicitly unset. Exact runner, preflight, source, log, build and artifact-receipt hashes are retained.

Four skipped tests remain unexecuted; their names are not listed in the terminal log. This run selects two integration targets rather than repeating the entire TUI package. Its terminal/PTY scenarios do not independently establish installed-worker Preparing cancellation, native-constructor interruption, graphical GUI/browser behavior or whole-harness completion. Raw reports and binaries remain cloud-local evidence.

See [exact results and prior-failure mapping](p02b-preparing-tui-integration-results.json).
