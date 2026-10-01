# Curated Git transport Stage A: verified development candidate

Source `7ab5dd77b87c2e6bf7040824e67bf6f22af6a073` adapts the actual native curated-Git command path on Linux. Its repaired source passed **541/541 tests**, clean scoped lint and reviewed formatting. It is a candidate for `work/p03-curated-sync-lifecycle`; **main promotion, safe activation and full-host lifecycle closure are not claimed**. The source commit and seven file blobs were read back; branch publication has not yet been observed in this record. Main's comparison remains native authentication source `65511842` within checkpoint `43a75813`.

## Change and provenance

Pinned upstream is OpenAI Codex `d42056091aded7feb1d88ac7e83972108b2aa478`. The existing `core-plugins/src/startup_sync.rs` waited for exit before reading pipes and killed only its direct child on timeout. Stage A routes its actual Linux command caller to `utils/pty/src/bounded_command.rs`: concurrent nonblocking reads retain at most 1 MiB per pipe, cancellation/timeout/overflow fail, and forced direct-child/pipe cleanup has a two-second budget. An unreaped Linux leader reserves its identity until its private group is signalled; direct-child wait then reaps it. Drop retains owned cleanup on errors and transfers the direct PID to the pre-reserved upstream reaper. Known ECHILD paths and successful wait disarm numeric-ID cleanup. Missing pipes report cleanup uncertainty.

HEAD lookup now goes through `run_git_command_with_timeout` on every platform. Linux uses the new helper; other platforms retain the legacy helper and gain its existing timeout path. NonLinux behavior is not verified. Supplied executable, arguments, working directory, environment and trusted system-Git/network policy remain the caller's responsibility and retain the existing policy path.

All audited PTY preimages—`lib.rs`, `child.rs`, `child_reaper.rs`, `process_group.rs` and `win/job.rs`—were byte-identical upstream. The native production reaper is inherited, not previously project-authored. The separate project-authored `component-sdk/tests/subreaper_runner.py` uses Linux subreaper adoption and demands a bounded complete drain. Windows job code is only a reviewed future reference, not part of this Linux helper. This checkpoint is a compiled native prerequisite, **not a separately installable subsystem extraction**.

## Exact validation sequence

| Gate | Observed result |
| --- | --- |
| First red attempt | Link failure 101; zero tests executed; retained separately. |
| Unchanged-source red retry | Six selected: three existing policy passes, all three new lifecycle tests failed; exit 100. |
| Original candidate regression | 541/541 passed; strict runner exit 0, no drain error. |
| First scoped lint | Exit0 with six distinct new diagnostics and one automatic test-condition rewrite; not clean lint. |
| Manual repair regression | 541/541 passed: 476 core-plugins, 65 PTY; zero skips/retries; 13.699s tests. Strict runner exit 0, no drain error. |
| Second scoped lint | Exit0, zero warnings, source unchanged and equal to repaired regression input. |
| Formatting | Exit0; exactly three files changed only in layout, imports and optional trailing commas, independently reviewed against exact tested bytes. |
| Dependency lock refresh | Exit0; actual `MODULE.bazel.lock` unchanged. This is not a Bazel build or platform verification. |

Red reproduced full-pipe deadlock and delayed helper actions after parent exit/timeout. The latter failures reached marker assertions after helper-stopped checks: they demonstrate escaped work during the operation, not that the helper was still live after the API finally returned. Original red-to-green applicable test bytes were identical. After warning-bearing lint, production ownership/error handling was manually repaired and the test helper became fallible; those files differ from original red, while fixture commands, waits, thresholds and behavioral assertions remain. The separate repaired regression is required proof for that change; original green is not relabelled as its proof. No test was repeated solely for automatic formatting.

Both green runs recorded 52 strict reaps: 36 exit 0, two exit 128, twelve SIGKILL, one SIGTERM and one SIGPIPE. Exact records are preserved without executable attribution. Unit-observed zombies mean terminated, not reaped; the strict runner's complete drain remains a separate requirement. No larger drain, disabled feature or weakened runner was used.

## Source binding and preservation

Final tree is `1f650c46fdca056ec7cfd333a65c4defc4f9d066`, parent `43a758138c457569a33ccd14f01446d715eb54e0`. This evidence worker independently matched all 8892 scoped source paths to the final format map and local immutable tree, verified all seven changed file hashes/Git blobs, and verified all seven members of archive `e7212fe787c0ab1a83d37b069278a83e9926991e9a990fa376abefee99cee4f4`. That archive contains **seven changed files**, not an entire worktree. The source commit object is not available in the local object database: remote commit/file readback is bound to the parent's receipt; the API did not return the tree. Exact tree binding comes from create-tree output and create-commit input plus independent local tree verification.

The earlier failed build, behavioral red, both green runs, both lint passes, exact preformat source, frozen proposal/strengthening, resource receipts and B1 review remain separately referenced in [machine-readable results](p03-curated-git-transport-results.json). The wrapper's historical `baseline=f2cc6c2...` label is not source identity; actual source maps define each run. [Lineage](../../upstream/p03-curated-git-transport-lineage.json) maps upstream paths/symbols, project additions, repairs and final source. In-workspace archives are recovery checkpoints, not proven outside backups. Historical executable archive checks are inherited from the parent's resource audit and are not new runtime results.

## Why this remains isolated

Private Git groups can outlive abrupt host/worker death and lie outside an outer guard that signals only the parent process group. This is an unresolved source-level ownership risk; no new full-host test was run for this candidate. Direct-child wait plus pipe EOF after signalling does not prove every group member gone/reaped; exclusive waiter ownership is required. The real caller still supplies a fresh false cancellation flag.

Cleanup uncertainty remains a String error. The pipeline can fall back or release stable lock/staging before cleanup is confirmed; shared PID reaping has no acknowledgment or resource bundle. Git can mutate an existing repository, HEAD swallows errors into optional SHA fallback, activation disposes of backup before SHA publication, and the manager resets the process-global gate on every error. Prior default-feature full-host storage failures remain failures despite 13 behavioral operations passing. Stage A does not close lock, HTTP/body, publication, detached worker, callback, App Server/CLI or cross-platform ownership.

The [next ownership review](P03_CURATED_B1_OWNERSHIP_REVIEW.md) therefore requires real retained cleanup/resource/admission ownership and typed failure together in the pipeline and manager before cancellable lock/Git propagation. It is static design, not implementation. Move the necessary worker/admission ownership ahead of the old B1 order; do not assume an unmeasured 500-line fit. Retain locks/resources through unknown cleanup and dropped observers; release only on sufficient acknowledgment, prevent overlapping admission/fallback and preserve lost-identity uncertainty. Actual owned launcher/worker closure and unchanged storage, migration and GUI full-host gates are required before promotion.
