# Public session Stop admission — combined library verification

The corrected combined AS/client/TUI **library** gate executed **6,095 tests: 6,095 passed, 0 failed, 4 skipped**, exit 0. The scope is App Server 426, client 42 and TUI 5,627 executed tests. This is **not a complete TUI integration-suite pass** and is not new full-host, installed-worker or graphical UI/browser proof.

The first combined attempt failed E0624 before any tests ran. Root changed only `prepare_search_admission` visibility to `pub(super)` so its parent dispatcher can call it. Both the original failure and exact fix hashes are retained. The corrected gate used the unchanged strict subreaper, `env -u NO_COLOR`, two test threads and stable recovered temporary space. Its complete source-before/after maps match.

## Source and ownership boundary

This slice makes an already-received `fuzzyFileSearch/sessionStop` signal the exact pending Start before its ordinary FIFO handler can run. It changes the existing App Server adapter, not the native search implementation or wire protocol. The lineage maps all 10 paths to original Codex `d42056091aded7feb1d88ac7e83972108b2aa478` source bytes and symbol anchors, reread from preserved Git objects: four paths existed upstream; six are custom adapter/ownership/test files. It also maps the earlier eight Preparing-consumer paths and identifies the four touched here.

Upstream already serialized Start/Update/Stop exclusively by session ID. Connection-scoped queue identity and retained component ownership came from the earlier extraction. This slice preserves that connection/session FIFO while admitting cancellation intent through the existing current-auth/closed RPC gate before enqueueing. A queued Start owns its completion ticket before first poll. Stop holds passive receipts and exact cancellation capabilities, signals outside connection locks and leaves later Starts uncancelled. Accepted owner loss retains cleanup uncertainty. First-cause retention covers real startup failure before publisher cleanup and genuine quota refusal before restored no-work ownership.

The front-door intent limit of 16 live starts is intentional earlier backpressure, separate from existing engine quotas. Stop remains admissible at that limit; registry metadata refunds never assert backend cleanup.

## Test coverage and remaining failures

All eight new ingress tests are mapped to actual log entries in the results JSON. They exercise real serialization queues, controlled retained-backend scenarios and one actual-native normal-query/FIFO/restart regression. They do not prove complete public dispatcher transport, revoked-auth behavior, installed-worker Preparing cancellation, native constructor interruption, or GUI/browser behavior. The known nested Ready-close owner-loss cause window remains deferred.

The earlier full-TUI retry02 had 17 failures. This library gate reran all 12 library cases; their test source files match the earlier failed run exactly. It did not execute the five `codex-tui::all` integration cases (external editor, daemon compatibility, no-daemon startup, provider history fixture and worktree interaction). Each previous failure is mapped individually in the results JSON. All five integration cases remain pending a new full CLI build and complete integration/full-TUI gate; passing library tests do not clear them. Expected snapshots and assertions were not weakened.

## Next gates

Record scoped lint/format and any source changes separately. Build and hash the new full Codex CLI, explicitly bind `CARGO_BIN_EXE_codex`, and rerun the complete TUI integration gate. Then run the genuine installed-worker public Start/Stop/sibling/error/uncertainty and separate GUI/legacy cancellation gates. Raw report/source archives are cloud-local; publication of evidence does not export those archives.

Companion records: [results](p02b-public-stop-results.json) and [source lineage](../../upstream/p02b-public-stop-lineage.json).
