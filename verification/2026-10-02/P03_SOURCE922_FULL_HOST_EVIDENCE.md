# Source922 full-host regression — 2026-10-02

**The recorded storage, migration and GUI regression gates passed. The native-worker shutdown contract remains incomplete.** This fresh result supersedes the earlier source99 failure only as the latest runtime sample; the original failed runs remain unchanged. It does not establish that the missing production stop/join/callback ownership has been implemented.

[Results and exact artifact bindings](p03-source922-full-host-results.json) record the fresh executable, all source maps, package reuse, strict runner outcomes, safe GUI projection and sampled process observations. Main is not promoted by this documentation checkpoint.

## Actual source and packages

Native source: `92212516ad4d12bcf60546ee5f879b983ed44682`, tree `746d93dcdc89de6882dba313d4ad3626d33b0ad0`. Published checkpoint at build/runtime: `a69fa42ffd83a134624842e7649502abd30455b1`, tree `6726f26f52f46a68b193cf8b1de51fb8a68cf28d`, on `work/p03-curated-sync-lifecycle`.

`cargo build --locked -p codex-cli --bin codex` passed in **6m 25s**. All four build/runtime before/after maps agree on **8,900 scoped entries**, independently rehashed afterward. The wrapper's historical `baseline` field is not the tested-tree identity.

The fresh CLI is **635,068,200 bytes**, SHA-256 `890b4da9756644e56276e4f6dddb84439417dc250fa2f74ae201e1efb4948b11`. It ran from the original mutable Cargo path under an exclusive no-Rust/no-source-writer window; every runtime's before/after binary fingerprints match. Component manager SHA-256 remains `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a`.

Existing independently built native storage and search packages were reused, installed and exercised with the newly built host. Original exported source remains absent and package inventories match their original proofs. This checks compatibility without rebuilding those native packages; it is not another native plugin build or newly extracted subsystem. Small Python fixture plugins were built normally.

## Terminal gates

Prefixes below are under `/workspace/acceptance/p03-source922-`, with fresh `*-runtime` directories and a private workspace TMPDIR. No previous runtime fixture was relocated or replaced.

| Gate | Behavior | Command / strict runner | Result |
| --- | --- | --- | --- |
| `newhost-storage` | 13 commands passed; unchanged package and host fingerprints | 0 / 0; null runner error; 5.1689s | Passed |
| `migration` | 10 commands passed; six observed storage PIDs | 0 / 0; null runner error; 5.8104s | Passed |
| `gui-normal` | Two cold GUI cycles, selected search and actual manager SIGINT | 0 / 0; null runner error; 21.3324s | Passed |

The strict runner and its default **five-second** descendant drain were unchanged. Native plugins were not disabled. The first storage-launch preflight had an incorrect list-offset assertion; it stopped before launching any wrapper/workload. Corrected preflight located the exact strict script path and verified its complete original argument suffix. This was not a failed test run; its note is preserved separately.

Storage's strict report recorded three adopted Git children exiting with **SIGPIPE (−13)**. A read-only observer sampled each matching PID/start-tick generation as `/usr/local/bin/git`, executable SHA-256 `37d928337f8e6c6b4c875d79fd33ee2f440594a589f584c89146bbf0aeafd63e`. The observer did not signal processes, reap descendants, change the strict deadline, or capture argv/environment. Its 250ms post-exit tail is diagnostic only. The specific Git commands and SIGPIPE causes are unknown; sampling may miss short-lived descendants. Migration also recorded one adopted SIGPIPE with no executable attribution. These are not all-child-exit-zero results.

Unlike the earlier source99 attempts, neither strict storage nor migration report found descendants remaining at its deadline. This does not turn historical failures into passes or prove the unimplemented shutdown-owner contract under deliberately held native work.

Migration verified native/selected dry-run parity, manual apply and repeated apply, cold CLI/App Server history, cancellation and reaping. GUI used actual Chromium through Playwright, the real manager/gateway/App Server, selected storage/search workers and deterministic model fixtures. It exercised persisted conversation recovery, streaming, native command approval, cancellation and continuation, file-search insertion and response fences.

First SIGINT during an active turn exited the component manager with status **0** after **0.264997117** and **0.264944718** seconds. All tracked manager/gateway/server/storage/model identities and selected search workers were absent; internal/final drains passed, packages/binaries were unchanged and no forced cleanup was reported. These fast exits do not demonstrate that the longer storage/GUI cleanup budgets survive the existing 45s App Server watchdog.

## Reviewed screenshots

Root visually reviewed both fresh screenshots for functional UI evidence and secrets before copying them. This is review of Playwright screenshots, not manual in-app Browser interaction.

- [Recovered conversation](p03-source922-gui-recovered.png), SHA-256 `a5ba7e22df683664a97450a7e4f577ba9ef1a11a109aa297cd2a68b6d1a9e1c6`.
- [Selected file search](p03-source922-gui-search.png), SHA-256 `1054991727f3dacd0e450286acf2fb8e906035f5ffd5be744d033cc2dd9966d7`.

The private GUI report is bound by digest only. Launch URLs, tokens, authentication/configuration values, browser logs, arbitrary exceptions and raw process snapshots are excluded from the published projection.

## Remaining acceptance

The [shutdown audit](../2026-10-01/P03_CURATED_SHUTDOWN_INTEGRATION_AUDIT.md) still applies: source922 has no production stop caller, native join receipt or owned curated callback drain. A frozen completion proposal adds exact handle observation and race tests but has not been adopted or compiled at this runtime checkpoint. Owned callback scopes and explicit process-final integration must follow, preserving embedded replacement and sibling-instance behavior. Held real Git/HTTPS success/error/EOF/SIGTERM tests and slow-cleanup/forced-uncertainty tests remain required.

The frozen C2b extraction proposal remains separate and untested. Recoverable repository/SHA publication, host-death fencing and coordinated watchdog deadlines remain open. No in-app Browser, live-provider, new attachment-payload roundtrip, installed native auth/catalog, whole-harness completion or upstream-update/rollback acceptance is implied. Current installed replacement coverage and the required updater track are unchanged.

Source, old failed evidence, both worktrees, staged proposals and the original GUI process remain preserved. Exact guarded cache-retirement receipts record removal of only obsolete ordinary build libraries; metadata, source and test executables remain preserved. Filesystem archives are local recovery checkpoints; external source durability applies only to the files actually published on GitHub.

The newly built CLI and17source/build/strict/metadata/restore members were fully verified in `p03-source922-cli-preserved.tar.zst` (138,845,342 bytes), SHA-256 `32a3ec3233a54369e8bfe95f313d4cc3dec8a87be09f447836b2a21d061f7ef5`. Both original hardlinks, all eight historical aliases and the original frozen GUI remain unchanged. The archive preserves one executable payload plus exact topology/restore metadata; it does not recreate the original inode/ctime or establish an external backup.
