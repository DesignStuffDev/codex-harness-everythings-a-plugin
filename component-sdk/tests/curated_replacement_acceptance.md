# Curated same-process replacement acceptance

Status: proposed, uncompiled and unrun. This gate supplements the WorkerGate unit tests and external held-host shutdown acceptance; it does not replace them.

The Python parent runs the exact app-server library test ELF three times. Each fresh child starts real embedded AppServer A, closes A locally, and starts B in the same process with the same absolute home. Startup remains enabled. The native worker uses the normal trusted Git resolver and fixed production URL. An ordinary child-scoped Git global configuration rewrites only that URL to an owned loopback smart-HTTP fixture served by real `git http-backend`. The test records the trusted executable hash and an observed descendant identity while the advertisement response is held. This exercises real Git fetch/materialization, not HTTPS TLS/body cancellation.

| Case | Release and observation | Required result |
| --- | --- | --- |
| `pending` | A closes while Git is held; B starts an existing thread/turn with old plugin state before release. | That same B thread observes new metadata, skill, MCP endpoint/tool behavior, and the new Interrupt hook. |
| `replay` | After A closes, release Git and observe exact-generation success plus the attached native handle being finished before B starts. | B receives one replay and exposes the new behavior. The read-only observation must still report Running; only final owner harvest reports Joined. |
| `race` | A closes, then a control barrier releases Git concurrently with B startup. | Either legal registration/completion ordering delivers once. This does not assert a particular internal lock interleaving. |

A continuously drained typed event stream belongs to each embedded client. New MCP Ready must precede the post-release direct tool call, which could otherwise independently refresh a dirty manager. The pending case keeps a real model response open during replacement; after typed behavior and the Interrupt hook are verified, a separate checkpoint releases that fixture before final shutdown. A final success checkpoint follows local client/event-task shutdown, A/B callback joins and the exact native join.

The original A scope must record zero successful actions; B must record exactly one. A finished but unjoined callback is not counted as a success until its original task result is observed. Successfully drained scopes retire from the global registry, so their retained per-scope receipts carry A/B counts; the final global registry observation must have no remaining callbacks. Native success, exact generation, Joined, absence of quarantine/unexpected handles, and clean curated ownership are all required.

## Build and invocation

First review/adopt the frozen native lifecycle observation unit, then callback activity unit, then the Rust child/wiring and this Python parent. Build and run the normal scoped app-server tests through the repository `just test` workflow. The new child is intentionally ignored during ordinary test discovery: only this parent supplies its fresh-process environment and owned fixtures. Select the exact app-server **library** test ELF from that successful build and bind its SHA256 to the source manifest; do not select the integration test executable or a stale ELF.

After root has selected the exact test ELF and trusted Git hash, run from the repository root (example variable values are explicit required inputs):

```sh
APP_SERVER_TEST_ELF=/absolute/path/to/the/verified/app_server_library_test_elf
APP_SERVER_TEST_SHA256=the_verified_64_hex_sha256
TRUSTED_GIT_SHA256=the_verified_64_hex_sha256
ACCEPTANCE_OUT=/absolute/fresh/evidence/directory
python3 component-sdk/tests/subreaper_runner.py \
  --report "${ACCEPTANCE_OUT}.strict.json" -- \
  python3 component-sdk/tests/curated_replacement_acceptance.py \
  --test-elf "$APP_SERVER_TEST_ELF" \
  --test-elf-sha256 "$APP_SERVER_TEST_SHA256" \
  --git-sha256 "$TRUSTED_GIT_SHA256" \
  --work-dir "$ACCEPTANCE_OUT" --timeout 120
```

Linux, Python 3.9+, trusted system Git with `http-backend`, and the compiled Rust child are required. No additional Cargo/Python dependency is introduced. The per-case parent deadline includes fixture preparation and protocol execution. Fixture cleanup uses finite failure waits; any forced child/backend cleanup or unconfirmed pipe drain fails acceptance. The unchanged strict runner independently waits/reports adopted descendants using its existing five-second drain; do not relax it. Review its command result, runner error and each raw adopted-child outcome rather than assuming all descendants exited zero.

The child receives `RUST_MIN_STACK=8388608`, matching the repository's `just test` recipe. Directly invoking the library test executable otherwise omits that recipe setting; the first real replacement attempt aborted with a Tokio worker stack overflow before B started. That failed attempt remains separate evidence. This setting does not change fixture deadlines or lifecycle assertions.

The parent supplies all environment before child startup, keeps sandbox/network policy variables unchanged, and never substitutes a PATH Git wrapper. Fresh fixture homes contain no production credentials. The parent binds `CODEX_SQLITE_HOME` to the case home, and the child checks the resolved SQLite home before state initialization so managed overrides fail before opening another database. Auth environment keys for API access are removed from this test child; the local mock provider requires none. Hook trust uses the existing supported test harness override. No production DNS, proxy, CA, allowlist or transport policy is weakened. A rejected normal Git rewrite is a setup failure, not permission to bypass production policy.

## Evidence and limits

Keep the source manifest, exact test/Git hashes, strict report, every case result, raw child stdout/stderr, original typed child receipt, generated old/new source inventories and commit IDs, narrow Git configuration, actual Git request records, callback observations and final join outcome. Failure cleanup does not replace original operation errors. The fixture retains timeout output and marks forced paths. No full environment, argv, auth data or unrelated home is collected.

A passing gate establishes the scoped same-home curated replacement behavior and its curated join contract for these cases. It does **not** establish MCP prewarm/transport shutdown custody, all-host cleanup, store durability, crash-safe publication, HTTPS body cancellation or arbitrary process-group completeness. HTTP MCP endpoints and Ready/tool calls verify behavior, not the separate unfinished lower transport ownership repair. `whole_host_clean` remains false in reports even on a passing scoped gate.

The current process-global once policy still does not support independent different-home sync. This fixture uses one exact same home and neither changes nor claims coverage of UnsupportedHome. Failed-generation retry subscriptions and wider concurrent embedded-owner behavior retain their documented limits. C2b extraction and the unadopted MCP repairs are outside this proposal.
