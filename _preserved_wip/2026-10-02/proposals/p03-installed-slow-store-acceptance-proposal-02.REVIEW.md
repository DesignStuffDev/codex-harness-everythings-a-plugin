# Proposal02 bounded independent re-review

Manifest reviewed: `313fbe4f70d7b550df3e50b9e7b5d330d5e0062a2008ada0e5843f6e35308a63`.

**Static ACK for the defined installed-fixture acceptance scope. Both proposal01 blockers are resolved; no new concrete blocker was found in this bounded delta review.** This does not report a real Launch/GUI acceptance run.

## Resolved findings

- `slow_store_relay.py:218` gathers both exact directional tasks and checks their settled outcomes. Non-cancellation failures remain primary or recorded secondary failures, and are raised before `finished.json` at line 255. Owner-requested input cancellation following terminal native EOF is distinguished from an operation error. A host task that fails while the native child is being awaited can no longer silently produce graceful success.
- `installed_slow_store_acceptance.py:163` requires the fresh manager's first-interrupt acknowledgment before recording the hold origin. Both the 46-second deadline and the >45-second release assertion use that acknowledgment observation. It continues to require a live manager and an unforwarded request throughout the hold. Signal delivery remains manager-only through the saved PIDFD identity.

## Exact proof limit

The installed provider accepts and retains the complete append **before forwarding it to native storage**. The planned real run can show that the component manager allows this fixture-accepted write to remain outstanding for more than 45 seconds after the manager has acknowledged graceful shutdown, then requires native acknowledgment, cleanup, process exit, and cold recovery.

That acknowledgment anchors the **manager's** shutdown request/grace. It does not establish when the AppServer's independent shutdown/watchdog clock starts; propagation and intermediate cleanup may delay that onset. This fixture must not be cited alone as proof of the AppServer's old 45-second regression. The separate actual-stdio FIFO tests provide that boundary-specific evidence. References to an "old 45-second deadline" in this fixture's assertion must be interpreted within this narrower manager/installed-provider scope. No new product tracing or test hook is required for the narrower claim.

## Evidence checked without execution

- All five proposal02 files match the manifest's hashes and byte counts, and all 15 audited checkout sources still match their recorded fingerprints.
- Proposal01's manifest and original independent review remain unchanged and match proposal02's recorded fingerprints.
- Stored synthetic runtime report hash: `52ad3b188f7c0b6237ed3643371ec5ace5dac653825fd555152497e3c25bdeee`.
- Stored strict-runner report hash: `5a3734ccb66c62211da296bd3799f3ee11da12d519ff778f6a3b418acabfe097`.
- The recorded four cases are exact gated forwarding, clean native EOF with explicit input cancellation, late input failure producing relay exit 1, and the unchanged original relay reproducing its false exit 0. The strict runner records command/runner success, no runner error, and no emergency cleanup. These existing records were inspected; the tests were not rerun.

The synthetic gate lasts approximately 0.2 seconds and uses a Python protocol peer. It proves neither native storage behavior nor the real manager timing property. Real normal/forced Launch variants, source/binary semantic binding, cold GUI recovery, and the separately checked strict runner remain execution requirements. The original review's disclosed sampled-descendant, synchronous-stdout, inherited browser-queue, and external source-fingerprint limits remain applicable.

No source, manifest, checkout, dependency, or cache file was changed. No Rust, fixture, install, or GUI execution was performed. This review note is the only new artifact.
