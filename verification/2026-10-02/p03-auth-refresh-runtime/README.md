# Auth refresh: current production runtime checkpoint

All 12 planned runtime gates passed on their first attempts with the rebuilt CLI `8e8a5dac…` and unchanged component manager `f054d84a…`. Every source wrapper bound the actual 8,949-file source map `42e3899a…` before and after execution. This is a bounded regression checkpoint, not another extracted component family or completion of P03/the whole platform.

| Gate | Actual result |
| --- | --- |
| Installed native thread storage | 13 commands passed; three tracked storage children absent |
| Four migration preparations | 10 commands passed each; real CLI/App Server migration and cold recovery |
| Ordinary and selected-search GUI | Two real Chromium cycles each; streamed output, persisted history, search and normal closure; approval response and Stop exercised in cycle 1 |
| Attachments | 17 commands, five cases: built-in baseline, separately installed native upload, counted upload/cold reuse, typed fallback and restored built-in behavior |
| Slow normal Launch shutdown | Manager exited 0 after 46.452335498 seconds; held canonical history event recovered after cold restart |
| Forced Launch shutdown | Manager exited 1 after 2.040751248 seconds and explicitly reported unknown durability; the unforwarded held event was absent; subsequent cold continuation/normal closure passed |
| Held production matrix | All eight CLI/App Server success/error/EOF/SIGTERM cases passed original gates with exact independent Git acknowledgments and unchanged deadlines |
| Featured-task receipts | All eight separate receipt gates passed: one joined scope and two zero-joined process snapshots per case |

The native packages were built independently earlier and reused byte-for-byte. Current host and installed-package before/after fingerprints establish installation/exercise without rebuilding the host. The attachment gate reached the actual production CLI upload caller, observed the native executable and staged-file read, matched the built-in model payload, and retained custom upload count 1→1 during cold resume. It does not inspect the native result envelope or prove a production attachment resolve caller. Removing the unselected native package retained an **empty** state directory, not nonempty stored content.

The slow tests invoke the component manager's actual Launch command and SIGINT. The normal hold begins after shutdown acknowledgment and exceeds the earlier 45-second cutoff. It selects accepted `ItemCompleted(UserMessage)` history; it does not repair the preserved response-only interrupted-history projection gap. Forced termination deliberately leaves durability uncertain.

The independent postcheck verifies all eight case/report/strict/observer/acknowledgment hashes, exact Git hash and retained observed generation/ancestry, acknowledgment before held readiness/stop (also before fallback 502), unchanged host bytes, no fixture rescue, bounded connection diagnostics, proxy peer EOF and curated callback/worker outcomes. It does not equate the independently observed Git generation with an inner sampler or prove which Git operation owned a transport. The held matrix has eight nested strict reports and no fabricated outer strict report. Adopted return codes including `-9` and `128` remain explicit in EVIDENCE.json without attributing their cause or calling all termination graceful.

`http_cleanup_proven`, `mcp_cleanup_proven` and `whole_host_clean` remain false. Zero-joined process snapshots are not lifetime totals or evidence that work started. Lower transport/transitive ownership, session/MCP exact custody, current-source same-process replacement/consumer gates and the failed TUI test-build OOM remain separate work. This new source42 pass does not resolve the cause of the preserved older source7a selected-search timeout or earlier auxiliary startup race. Upstream revision integration/rollback and final clean-install platform acceptance remain open.

The UI ran in real Chromium/Playwright with deterministic inference fixtures. In-app Browser manual interaction and live-provider behavior were not tested. Root visually inspected only these selected screenshots, copied unchanged into this publication:

- [Recovered session](screenshots/recovered-session.png), SHA-256 `6cf357ebf6812b3c76094f557a18fded2c91e542504d3c7bee379b9136ce9cca`.
- [Installed search after restart](screenshots/installed-search-after-restart.png), SHA-256 `e656a61c01b4e7733667fae182ecfe3855811bfbb75b60cc9856138ca148cdd0`.

EVIDENCE.json contains sanitized typed outcomes and exact source/binary bindings. INPUTS.json pins completed private control/report artifacts by location, size and hash without copying raw configuration, process identities, trace bodies or credentials. These VM-local references are not externally durable copies of the raw evidence. The published commit/tree in held reports is a reference label; the actual tested source map and build receipts provide the source binding. Older failed runs remain preserved separately.
