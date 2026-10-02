# Independent static review: installed slow-storage acceptance

Reviewed proposal: `p03-installed-slow-store-acceptance-proposal`.

Reviewed manifest SHA-256: `05eacbd2f21720dd0776d566c306a7e4771f83c17e74a18dda629c1222fa2b4a`.

Disposition: two acceptance-fixture blockers before treating a run as the intended shutdown regression proof. These are fixture findings, not claims of a demonstrated native storage failure.

## Blockers

1. **A late input-task error can be reported as successful relay cleanup.** In `slow_store_relay.py:189`, only tasks in the initial `FIRST_COMPLETED` result are checked. If `native_to_host` finishes first, `host_to_native` can fail while line 196 awaits the native child. Line 200 then writes `finished.json`, and line 207 discards that task's exception through `gather(..., return_exceptions=True)`. The relay can exit zero despite a framing, JSON, or pipe failure from its owned input task. Before emitting `finished`, settle the input task and inspect every joined outcome, allowing only cancellation deliberately requested because native EOF was terminal. Preserve any earlier operation error while finishing cleanup. Merely checking `task.done()` before child wait leaves the same race.

2. **The >45-second hold is not bound to observed shutdown onset.** `installed_slow_store_acceptance.py:152` records time before delivering SIGINT, and lines 156–167 measure from that send. No condition observes that the manager handled the first interrupt and started graceful shutdown. If signal handling starts more than a second later, the 46-second release can occur within the old 45-second shutdown grace; the rest of the test can pass while missing that regression boundary. After sending SIGINT, require the fresh manager's first-interrupt acknowledgment (`Stopping presentation; allowing up to...`, emitted in `component-host/src/launch.rs` immediately after requesting shutdown). Record that observation and hold for the full >45 seconds after it, keeping the manager alive and the operation unforwarded throughout. Continue signaling only the manager.

## Confirmed by source review

- The manifest hash matches the assigned value. All three staged files match their recorded hashes and lengths; all 15 audited checkout files match their manifest hashes and lengths. Both Python files parse with `ast.parse`.
- The admission boundary is accurately labeled: the installed relay has the complete target request, but the native storage service has not admitted it until release. This is not native-internal admission proof.
- Target detection checks a regular `thread_store/call` `append_items` request containing the unique prompt; the driver verifies its thread ID. Original physical frames are retained and forwarded in their original order after release.
- Normal success requires the selected native append reply, native `shutdown_complete`, direct native child exit zero, manager exit zero, tracked descendants absent including zombies, and prompt recovery through a fresh runtime using the original native storage selection. The append reply is not treated as the sole durability fence.
- Forced mode sends a second manager-only SIGINT, requires nonzero manager status and explicit durability uncertainty, and never creates the release file. Its outcome remains explicitly unknown rather than a durability success.
- PIDFD signaling targets the saved manager identity, not its process group. The relay retains its direct child and both I/O task handles. Linux parent-death SIGKILL and the parent-identity race check cover abrupt relay death for that direct child.
- The relay bounds physical frames, declared logical bodies, concurrent assemblies, retained wire batches, and aggregate decoded partial bodies. Once gated, it stops consuming input and relies on pipe backpressure. Cancellation/cleanup still waits on the direct child.
- The two native-package copies are checked against the independently built package; host binaries are compared before/after; existing installed objects and original evidence are checked for changes. A new claim file and new work directory prevent ordinary accidental reuse between specialized runs.

## Limitations and execution conditions

- The driver verifies a supplied receipt hash and supplied binary hashes; it deliberately does not validate the semantic relationship between source receipt and binaries. Root must review that relationship and the real terminal build result before execution. The driver records SDK helper/script fingerprints but does not itself compare those helpers to this manifest or recheck all receipt/script fingerprints at the end. Use the documented source-fingerprint wrapper and compare the run's actual fingerprints to the frozen review.
- The driver checks the subreaper script's bytes, not whether it is actually its ancestor or whether `--drain-seconds` was overridden. The inner `report.json` alone is insufficient. Acceptance must also require the separately retained unchanged runner invocation using the five-second default, its zero exit status, and no remaining descendants.
- Descendant observation is sampled and can miss short-lived processes. The outer subreaper remains necessary. Native worker exit has separate exact-child evidence; sampled absence is not a universal process-tree proof.
- These are relay payload limits, not a strict whole-test RSS bound. The inherited `Browser` helper uses an unbounded Python output queue, and browser error/report collections are not fixed-capacity. Synchronous relay stdout flushing can block the event loop; production force termination is the disclosed fallback. No unconditional wall-time guarantee follows from the relay's async timeout.
- No real host, GUI, install, build, Rust command, or fixture runtime was executed for this review. No checkout, proposal source, dependency, or cache file was changed. This review file is the only written artifact.
