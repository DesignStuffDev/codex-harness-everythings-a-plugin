# P03 wire slice 1: real CLI runtime evidence

Passed on October 1, 2026, 13:24:43–13:24:47 UTC. The newly built shared-codec-linked `codex-file-search` used the original component manager to install, select, execute and remove the preserved independently built worker04 package. This run did not rebuild the worker or rewrite its original proof.

All 24 actual commands met their assertions: 22 exited 0, CLI-only Ctrl+C exited the expected 130, and deliberately invalid selected-worker configuration exited the expected 1 without fallback. Five native/external/restored cases compared exact ordered JSON, including lexical roots, exclusion, Unicode highlighting, truncation and indices disabled. No-pattern behavior bypassed the deliberately invalid selection. Removal restored native selection and retained the immutable package object.

Every observed command/worker PID set was absent after completion; no timeout or emergency cleanup occurred. The unchanged strict subreaper returned 0 with no runner error in 2.889674341 seconds. Its raw record contains one reap, PID 738880, wait status 0 / return code 0. This is the outer command record, not a count of every descendant. Polling can miss short-lived processes.

The CLI digest is `a52a960f424101e34d0027555d8ee7f223c7a685721d57477c2a4dcf1cb7dfa1` (21,185,808 bytes). The original manager is `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a`; installed worker04 is `a3f73fed15dd7b19ab0f1daed55a74212d4b2f5d2ba5cdebee2a616d2f2419a1`. The gate preserved binaries, package, independent-build proof, source inventory/parking, tooling and receipts.

The completed runtime's 8,861 source entries equal its successful unchanged build map. They bind the formatted codec slice; the earlier 87-pass unit/library gate used pre-format bytes. Full `codex-rs/` and `component-sdk/` subsets matched the format report, with only the recorded extra `MODULE.bazel` and 1,293 omitted SDK paths. The companion JSON records all seven codec path hashes, report/tool hashes and exact scope differences. Later API declarations were adopted after this completed run and are not covered. No equality to the current checkout is asserted.

This is standalone one-root CLI compatibility for custom shared transport. It does not demonstrate a new upstream subsystem extraction, broker activation, a new independent worker build, execution of the newly built manager, App Server/full Codex CLI/TUI/GUI behavior, or native-constructor cancellation.

Report fingerprints:

- Acceptance: `097580712e38a2c3fb362cb13f27d203739937840d0dc5209de49e54e17f29e1`.
- Source wrapper: `182048101b20756b5d3200c8f6cdb9641bccda43081324daf9b2b8ce0e13998f`.
- Strict subreaper: `623c54553b765474273940a00098dc6022cd67fcfeda613b804fe429275a5bc9`.

Detailed evidence: [p03-wire-slice1-runtime-results.json](p03-wire-slice1-runtime-results.json). Earlier checks: [P03_WIRE_SLICE1_EVIDENCE.md](P03_WIRE_SLICE1_EVIDENCE.md) and [P03_WIRE_SLICE1_LINT_FORMAT_EVIDENCE.md](P03_WIRE_SLICE1_LINT_FORMAT_EVIDENCE.md).
