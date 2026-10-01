# Real App Server native-search acceptance

This runner passed against a frozen App Server and the previously independent package03 on 2026-10-01. See [consumer evidence](../../../verification/2026-10-01/P02B_APP_SERVER_CONSUMER_EVIDENCE.md) for exact source, binary/package hashes, separate regression gates and remaining GUI/TUI limits. The worker and host were unchanged throughout installation, runtime checks and removal. This does not establish whole-harness extraction.

The scripts were prepared and reviewed before execution. The historical staging records below describe that preparation; they are not the final runtime status.

## Invocation

Use a new output directory on every attempt; failed attempts must remain intact. Supply the actual frozen App Server SHA256 and its successful root `*.source.json` build report (`returncode: 0`, `scoped_source_unchanged: true`). The runtime rechecks that binary before and after execution. This report link binds the operator-supplied candidate to its build evidence; it is not an independent reproduction of the App Server build.

```sh
python3 /workspace/codex-harness-everythings-a-plugin/component-sdk/tests/subreaper_runner.py \
  --report /workspace/acceptance/p02b-as-search-independent-01-subreaper.json -- \
  python3 /workspace/codex-harness-everythings-a-plugin/component-sdk/tests/file_search/app_server_acceptance.py \
  --repo /workspace/codex-harness-everythings-a-plugin \
  --server /PATH/TO/FROZEN/codex-app-server \
  --server-sha256 ACTUAL_FROZEN_SERVER_SHA256 \
  --server-build-report /PATH/TO/APP_SERVER_BUILD.source.json \
  --manager /workspace/component-checkpoint-candidate-p02b-search-cli-20261001-02/codex-component \
  --build-report /workspace/acceptance/p02b-search-independent-03/independent-build.json \
  --work-dir /workspace/acceptance/p02b-as-search-independent-01
```

The optional `--server-mode codex` inserts `app-server` after a frozen combined `codex` binary. Default mode directly invokes `codex-app-server --listen stdio://`. No shell commands or binaries are compiled by this runner. The CLI's earlier source snapshot does not claim to cover the newly changed App Server.

## Gates

1. Verify independently built worker provenance: accepted build03, separate initially empty target, newly compiled artifact, local dependencies wholly exported, no added lock identities, parked build source, unchanged package inventory, and unchanged component-manager binary. Verify the separate frozen App Server and bind its source-unchanged build report.
2. Start a real default-native App Server in a new isolated `CODEX_HOME`; initialize the stdio protocol with `experimentalApi: true`. Perform seven one-shot native search cases: original lexical `./project//` roots; ignored file omission; Unicode path/highlights; beta; no match; empty query; 50-result cap; and multiple roots (the first case covers both root spelling and ignore behavior). Save exact ordered JSON baselines including every score, path, root, filename, match type, and highlight. Close stdin and require exit0 plus absence of every observed process.
3. Copy/install/select package03 using the real component manager. Verify installed bytes/manifest and park the install-input directory before executing the installed copy. Start the same unchanged App Server and observe the actual installed worker `/proc/.../exe`. Require exact native/installed one-shot parity.
4. Start two search sessions and send updates concurrently. Await each start response, then explicitly submit and consume an empty-query update/snapshot/completion before later queries. Runtime query-id0 frames are internal state and deliberately produce no callbacks; a submitted empty query has a positive identity and is presented as `files: []`. `sessionCompleted` has only a session ID, not a query ID. Every cycle requires a current-query snapshot preceding completion and a successful request acknowledgement. Stop one session; require its subsequent update to fail, its sibling to complete alpha/beta/alpha, explicit clear-to-empty, and no-match cycles, and no stopped-session notification after the stop acknowledgement (no post-stop grace period). Leave the sibling open: normal App Server stdin EOF must close it and the selected worker, return exit0, and leave every observed PID absent.
5. Restart with the real worker's supported `native_snapshot_bytes: 65536` setting. Healthy startup and initial alpha query must succeed. Send a valid 65536-byte whitespace query: parsed matching has no positive atoms, while its raw query plus actual FileMatch/path/root allocations necessarily exceed the snapshot cap. Require real `sessionFailed` with `resourceExhausted`, exact native snapshot allocation diagnostic, bounded diagnostic length, and current raw query identity. Update/notification/stop/shutdown diagnostics must not add unconfirmed cleanup, forced shutdown, ClosedLease, or TransportLost causes. The update response may acknowledge before the worker preflight or return the same retained operation error; neither permits a successful completion. Require sibling search still works and stop retains the resource failure. EOF is expected to exit with a positive error status for the retained operation failure; all observed PIDs must nevertheless disappear without emergency termination. This distinguishes failure from uncertain cleanup rather than labeling any nonzero exit a leak.
6. Restart with an unknown native plugin config field. Require explicit startup error before any protocol output, preserving the actual native config rejection; no successful native fallback.
7. Restore good isolated settings and remove the plugin using the manager. Verify selection/activation are removed while immutable object bytes remain intact. Restart unchanged App Server, require exact default-native parity and streaming/sibling behavior, observe no removed worker execution, then require exit0 and strict process absence.
8. Recheck frozen host/manager, package03, installed object, both supplied build reports, Python scripts and the unchanged subreaper byte-for-byte. Produce runtime report, transcripts and tracked process identities; parent separately verifies the unchanged outer subreaper's exit0 and no `runner_error`.

## Limits and deliberate boundaries

- Linux `/proc` only, with the repository's **unchanged** `subreaper_runner.py`. The copied `acceptance_support.py` must match the current SDK helper hash. No zombie exemption, silent retry, forced-cleanup pass, runtime dependency installation, or model/search stubs.
- One stdio connection with concurrent sessions is tested. Multiple-client connection privacy has separate source tests and is not claimed by this runtime scenario.
- App Server query cap is 64KiB, match limit50, per-session index allocation100k entries/128MiB; threads remain actual `min(available_parallelism,12)`, so two T12 sessions reserve52 worker slots under64. Native worker negotiated aggregate capacity can be below the host's broader policy. This is not 100k-entry fill/load or total-RSS proof.
- Normal EOF receives150 seconds to cover the source's120-second search shutdown budget. RPC observations have bounded timeouts; stream cycles have a90-second total budget. Output is capped at16MiB/frame,2048 buffered messages,32MiB transcript per server. Emergency cleanup always records failure and never establishes a successful shutdown.
- No model turn, authentication, approval, tool execution, storage migration, TUI, GUI/plugin Browser interaction, launcher Ctrl+C, or full harness extraction claim is made by this test. No remote preview claim.
- A fresh home uses ordinary App Server config with shell snapshots disabled. The standard existing debug managed-config override points to a nonexistent path inside that home; no managed host configuration or credential values are read or printed. The runner does not invoke hidden plugin-startup bypasses.
- Deliberate config edits occur only in the new acceptance `CODEX_HOME` between App Server processes; installed code/package contents remain immutable. No live upgrade/hot swap is attempted.

The stage manifest records inspected source hashes and script hashes. Those are staging identities; root must bind the actual frozen tested source and binary before claiming runtime acceptance.

## Pre-run review correction

The original stage is preserved at `/tmp/p02b-as-external-acceptance.before-review`; it has never run. Its `start_pair` incorrectly expected unsolicited query-id0 callback notifications. Source review found `file-search-runtime/src/observation.rs` admits callbacks only for positive accepted query identities. The corrected fixture explicitly submits an empty query after both start responses; exact empty results, snapshot-before-completion, response identity and later query barriers remain required. An explicit empty-query update is also proposed for the separate WebSocket fixture in the adjacent staged patch; no checkout source was edited.

The review also strengthened failure-cause/uncertainty checks and final tooling/object fingerprints. Normal EOF logs/process status are now recorded before exit-status assertions, so an already-exited but unexpected status is not mislabeled as emergency process termination.

The 60 truncation candidates now use distinct suffix lengths: Nucleo orders equal-score candidates by text length and then injection index, so equal-length paths at a parallel-traversal cutoff would make exact parity nondeterministic. Equality assertions remain exact and unchanged. RPC observer construction also retains emergency ownership if setup fails after spawning the child.
