# GUI Preparing cancellation audit — current legacy one-shot path

Read-only source audit of `/workspace/codex-harness-everythings-a-plugin`, 2026-10-01. No source edits, builds, test runs, browser interaction, or new behavior proof. Frozen TUI/fixture/packaging stages unchanged. Paths below are repository-relative.

## Result

The installed desktop example's file picker does **not** use `fuzzyFileSearch/sessionStart`, `sessionUpdate`, or `sessionStop`. It already emits a separate cancellation RPC while the original one-shot search is pending. Preserve that protocol; no frontend migration or redesign is recommended. The new public session-Stop admission work is a separate headless/API acceptance path and must not be reported as exercised by this GUI.

`component-sdk/examples/desktop/desktop_ui/static/file-search.js::SearchController` uses:

- Search: `fuzzyFileSearch {query, roots:[root], cancellationToken:pageUUID+":"+revision}` (`28-37`). The active request is retained before awaiting the RPC. Only one request and one latest queued successor are retained (`31-46`).
- Cancellation: `cancelActive` marks the exact active request cancelled once, then immediately invokes `rpc("fuzzyFileSearch", {query:"", roots:[old.root], cancellationToken:old.token})` without awaiting the original request (`12-16`). Repeated retirement does not issue repeated cancellations for that active query.
- Identity: a random page ID plus monotonically increasing safe-integer revision prevents old cancellation targeting a later query. `current` compares revision and the context tuple `[root,threadId,connection,pathStyle]`; accepted results also validate root/path (`11,18-29,37-42,59-63`). Successor admission waits for the original promise's `finally`, not for the cancellation HTTP reply. Empty-query success is not a cleanup receipt.

## Real UI event coverage in source

| Trigger | Source path and behavior |
| --- | --- |
| Clear query / B or C | `file-picker.js:47` invokes `search`; `file-search.js:22` cancels old active immediately before scheduling any new nonempty query. Empty input clears results with no new open. |
| Root edit | `app.js:167` -> picker `rootChanged` (`file-picker.js:54`) -> search if picker open; old cancellation retains old root/token, new query gets new context/token. |
| Escape / close / choose result | `file-picker.js:30,41,44-50` -> controller `close` (`file-search.js:49-51`), clearing queued intent and fencing old output before sending cancellation. |
| Select/new task / submit | `app.js:81-104` calls `filePicker.close` before thread resume/start or turn submission. Thread-switch UI is blocked while submitting, consistent with existing behavior. |
| SSE reconnect/disconnect | `app.js:155-165` closes picker and advances connection generation. This generation is a presentation fence, not an App Server transport reconnect: the gateway still owns one App Server child. |
| Page hide | `app.js:168` calls close. Current fetch has no keepalive (`app.js:13-19`), so document unload cannot establish delivered cancellation or joined cleanup. |
| Main Stop button | `app.js:171` sends `turn/interrupt` for the current turn. It is not a file-search session Stop control. |

## Gateway and App Server path

- Gateway `METHODS` (`component-sdk/examples/desktop/desktop_ui/gateway.py:23-35`) permits only legacy `fuzzyFileSearch` among search APIs. Session Start/Update/Stop would be rejected as unsupported. Browser RPC is independent authenticated `fetch` (`app.js:13-19`). `ThreadingHTTPServer` (`gateway.py:292`), `Bridge.rpc` (`94-119`) and per-frame `send` lock (`84-92`) permit the cancellation HTTP handler to write another JSON-RPC request while the original waits. Numeric gateway request IDs correlate replies; they are distinct from the query's cancellation token.
- Legacy search is deliberately concurrent: `codex-rs/app-server-protocol/src/protocol/common.rs:1472-1477` declares `serialization:None`. `MessageProcessor::dispatch_initialized_client_request` (`message_processor.rs:1037-1083`) spawns such requests independently. The session APIs separately serialize by session ID (`common.rs:1479-1495`), which explains why their Stop fix is distinct. No legacy serialization defect is inferred.
- `SearchRequestProcessor::fuzzy_file_search` (`request_processors/search.rs:175-272`) validates token, looks up the current connection's token->internal request->observer under lock, admits no new search for empty query, then requests exact prior cancellation outside the lock (`204-266`). Cancellation is available at request capacity. The immediate empty result acknowledges intent handling only; the original retained owner still awaits startup/cleanup before releasing its slot.
- `fuzzy_file_search/startup.rs::open` publishes an exact `SearchStartControl`, latches cancellation raced with publication, retains `pending.finish`, then observes the cancellation receipt (`17-85`). `SearchObserver::request_close/bind_start` (`publisher.rs:151-176`) forwards cancellation outside its mutex and retains a pre-publication latch. `OneShotGuard` (`search/connection.rs:193-213`) removes a token only if it still names that guard's request, preserving successors; `OneShotWaiter` (`253-260`) requests cancellation without refunding the retained slot.

## Existing tests versus missing acceptance

- `component-sdk/tests/test_desktop_search.cjs` contains seven controller/path tests. A/B/A checks a cancellation is issued before resolving the original mock response; root/task context fencing, repeated close/reconnect, empty-query cancellation, explicit errors, safe path insertion and platform roots are present. These are mock-RPC tests, not proof of real backend startup cancellation. No tests were run in this audit.
- `component-sdk/tests/test_desktop_search.py` checks authenticated legacy forwarding, error preservation, forbidden methods, and static assets. It does not gate pending backend startup or join cleanup.
- `component-sdk/tests/desktop_search_acceptance.py::hold_real_reply` (`26-41`) first completes `route.fetch()` and reads the real response body, then withholds browser delivery. Its A/B/A/root/Escape/task barriers (`103-160`) prove stale presentation fencing after server completion. They do **not** hold a backend Open or prove a cancellation request reached a Preparing owner. Preserve this useful regression gate but label it accurately.
- Current source `codex-rs/app-server/src/request_processors/search/preparing_tests.rs::token_retirement_reaches_pending_owner_and_waits_for_cleanup_without_cancelling_sibling` (`27-98`) directly exercises held-start token cancellation, retained quota and sibling survival. Its direct processor calls bypass gateway/browser/public scheduling. Runtime/test results are root-owned; source presence alone is not a passing claim.
- Cancellation HTTP errors are intentionally swallowed; a stale original error does not reopen/poison the current UI. That means the UI cannot claim joined cancellation or display a late retired operation's durability result. Actual retained failure/uncertainty must remain in backend/runtime evidence.
- Pre-admission ordering remains a distinct unverified edge: separate HTTP handlers can in principle deliver an empty-token cancellation before the original request has registered that token; current AS token lookup has no pre-admission tombstone. The proposed relay gate starts only after real Open, so it will not cover that earlier ordering. Do not assert this race occurred or change the protocol merely from this audit; record it as a separate lifecycle acceptance gap. Likewise pagehide is best effort.

## Coordinated real-GUI relay acceptance (proposal, not executed)

`/root/native_backend` is staging an isolated relay that forwards real worker bytes, arms the next Open, and withholds a genuine successful worker Open response before any response frame reaches the host. Its private marker directory records only exact request/lease/native-PID identities, never initialization/config/payload. The worker has reached Ready; the host/process is still Preparing. This must not be described as native constructor Preparing; native constructor gates remain unit evidence.

1. Fresh GUI/provider, open picker with empty query (no search RPC), arm next Open, type target query. Wait for `held-open-ready` and observe the original query/token is still pending in the actual GUI/gateway.
2. Clear query or press Escape; then repeat focused cycles for root/task replacement. Assert the browser emits the empty legacy RPC with that exact old root/token. Do not release the held Open bytes yet.
3. Require relay `target-release-joined` from a real worker Release receipt while Open remains held; verify exact target lease and sibling/control traffic remain functional. An HTTP request, marker saying cancel was sent, timeout, or vanished process alone is insufficient.
4. Release the original unchanged Open bytes, let retained host startup/cleanup drain, and require no stale results/reopened picker. For root/task replacement, verify only the latest queued query runs under a new token/current root and real file reference insertion still works.
5. Retain existing ordinary browser search, draft/path safety, task recovery and launcher shutdown regressions; record exact worker/host/UI source hashes and receipt evidence. Run separate actual public sessionStart/sessionStop acceptance for the AS FIFO fix. In-app Browser availability remains a distinct tooling limitation; no Browser check was performed here.

No frontend code fix is needed for the proven source-level pending-send behavior. Keep the current GUI protocol and tests intact; add the narrowly scoped real relay acceptance instead of silently migrating to experimental session APIs.
