# Source-owner full-host regression — mixed checkpoint, 2026-10-01

**Overall acceptance failed.** The fresh full CLI built successfully and the existing storage behavior checks, manual migration and normal GUI checks passed. Both storage attempts failed the unchanged strict descendant drain with exit **125**. Successful behavioral reports and later GUI shutdown do not turn those attempts into lifecycle passes. These are finalized facts for a failed checkpoint; the native curated-sync lifecycle repair is the next priority.

The [results JSON](p03-source-owner-full-host-results.json) binds original reports, logs, strict process statuses, source maps, binaries, scripts and recovery receipts. The [read-only diagnosis](P03_CURATED_SYNC_LIFECYCLE_DIAGNOSIS.md) and [audit](p03-curated-sync-lifecycle-audit.json) describe the likely startup-sync path and a proposed repair contract; no repair is implemented here.

## Actual source and executable

- Native source: `99e6802fd478e55686559aefdd6ae79177a45397`, tree `651cca62fb376fed10b50872165edda8d9b099e3`, parent `b3530790258a74c0c61bfd00e4d13141b763269b`.
- Published checkpoint at build/runtime: `7ad8b08c32530efcb7f57f9db6682e55a4808528`, tree `5eff6f1953ce75c7c7c3f40bbaf87bcca72123c2`. Publication receipt SHA-256: `580f8212c018689645b62ae02042804c11d2c1fcca488c222bd0bc8919a18555`.
- All five build/runtime before/after maps match the same **8,888** scoped entries. An independent final rehash matched every entry. The wrappers' older `baseline` label is not the actual source identity.
- Fresh `cargo build --locked -p codex-cli --bin codex` exited 0; compiler elapsed **4m 38s**. CLI SHA-256: `c7111d534c600c35812b714c4a1a536348254e7e1d3aea10dd77a411b48ff021`, **634,826,664 bytes**.
- The CLI ran from the mutable original Cargo target path under root's exclusive no-Rust/no-source-writer window. All four runtime reports' before/after binary fingerprints and the final independent hash match. This is not an uncompressed frozen host.
- The unchanged component manager SHA-256 is `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a`. Existing independent native storage/search packages were reused; no new native worker extraction or build occurred. Small Python harness fixture plugins were built as usual.

## Terminal attempts

Every prefix below is under `/workspace/acceptance/p03-source-owner-`; each runtime used its own new `*-runtime` workspace directory and `TMPDIR=/workspace/acceptance/p03-source-owner-runtime-temp`.

| Prefix | Behavioral result | Command / strict wrapper | Lifecycle conclusion |
| --- | --- | --- | --- |
| `newhost-storage` | 13 commands passed; package, binaries and provenance unchanged | 0 / **125** | Failed: descendants remain after command exit |
| `newhost-storage-02` | 13 commands passed; package, binaries and provenance unchanged | 0 / **125** | Failed again: descendants remain after command exit |
| `migration` | 10 commands passed; 6 observed storage PIDs | 0 / 0, null runner error | Passed with the nonzero adopted-child caveat below |
| `gui-normal` | Two cold GUI cycles passed, including selected search | 0 / 0, null runner error | Passed; both first-SIGINT manager shutdowns exited 0 |

Both storage attempts used the original five-second strict drain and unchanged assertions. Their recorded reaps all exited 0, but the runner still reported `descendants remain after the command exited`: recorded reaps do not account for every remaining descendant. The original attempt took 11.020512507 seconds in the strict runner; the second took 10.842914148 seconds. The original reports are preserved separately, including all PIDs and wait statuses.

The second attempt's read-only sampler observed **live native Git/HTTPS descendants** beyond the drain. Later zombie state or disappearance does not cure this failure. `/usr/local/bin/git` and `/usr/local/libexec/git-core/git` are the same native ELF hardlink set; calling one a wrapper is unsupported. Process shape, inherited group identity and unfinished `plugins-clone-*` staging directories strongly match native curated startup sync. The sampler did not capture argv, so no particular PID is claimed as an exactly traced fetch/index-pack command.

The audited startup-sync files are byte-identical to upstream `d42056091aded7feb1d88ac7e83972108b2aa478`. They start a detached native thread without an explicit host cancellation/join contract and run plain child processes that can survive host exit. This is evidence of an existing ownership gap, not proof that changing `/tmp` to `/workspace` caused the failure. Native plugins were not disabled; the strict timeout was not increased. The diagnosis proposes bounded cancellation, whole-process-tree cleanup and actual shutdown integration, followed by deterministic failing tests and fresh unchanged runtime gates.

Migration used the second storage attempt's **behavioral** report. That input does not establish a passing storage lifecycle. Migration verified legacy conversation creation, native/selected dry-run parity, selected migration and repeated apply, cold CLI/App Server history, cancellation and process reaping. Its strict runner recorded adopted PID **825974**, wait status **13**, return code **−13 (SIGPIPE)**, while the command and runner exited 0. Executable attribution and cause are unknown; this is not an all-child-exit-0 claim.

GUI ran actual Chromium through Playwright against the real manager, gateway, App Server and installed storage/search workers, using deterministic fixture inference. Both cycles recovered persisted conversation and exercised selected file search. First SIGINT produced manager exit 0 after **0.265443679** and **0.267221652** seconds; reported tracked identities and selected search workers were absent, packages/binaries unchanged, internal/final drains passed, and no forced cleanup was reported. These passing GUI checks do not repair the earlier CLI storage lifecycle failures.

## Reviewed screenshots and recovery

Root visually reviewed both images, confirmed persisted fixture conversation and file-search UI, and checked for secrets before copying them. The private GUI report is never published wholesale.

- [Recovered conversation](p03-source-owner-gui-recovered.png): SHA-256 `32f42b39cd73a43652075d4a709bbda4ebeb8c380e6981b9d5e81e76ab20ca1d`, 147,751 bytes.
- [Search results](p03-source-owner-gui-search.png): SHA-256 `56eea55ff446b9fbc4b2b98d402b48ca5350b5d20496137643784904451f4c2c`, 85,711 bytes.

Root executed the resource agent's reviewed archive script and verified the CLI archive's nine members. Archive SHA-256: `68102a992d8d7ebe3105933d5377c9ec215ed5d51c8f814891555f81fb098222`, **138,823,583 bytes**. Receipt SHA-256: `2e3fe79f270d4645be9c7d5545bf683be2fdd238aba0622445b23051a6565ff1`. This evidence author independently checked the compressed digest. The operation preserved both original executable hardlinks and all eight historical GUI aliases. It was archive-only; no external backup is established and historical alias names do not prove old binary identity.

## Scope limits

The separate [391-test native evidence](P03_SOURCE_OWNER_EVIDENCE.md), its red baseline, lint warning, mechanical alias/format transition and unknown-attribution SIGKILL reap remain unchanged. Those tests are not counted as runtime commands. The [earlier cache-source full-host evidence](P03_CACHE_REVISION_FULL_HOST_EVIDENCE.md) remains bound to source `181400…` and CLI `876c…`; it is not relabeled as proof of this source-owner CLI.

The in-app Browser was unavailable. These are deterministic fixture regressions, not live-provider or attachment-payload acceptance. Storage's executable-disappearance checks alone do not exclude zombie PIDs; sampled process scans can miss short-lived descendants. No source identity/ABA, install admission/clear ordering, refresh ownership, persistent or per-retry publication fencing, installed native auth/catalog extraction, broker activation, full-harness completion or updater/rollback acceptance is established. Private launch URLs, authentication/configuration values, browser logs, fixture objects and raw process snapshots are excluded from the published projection.
