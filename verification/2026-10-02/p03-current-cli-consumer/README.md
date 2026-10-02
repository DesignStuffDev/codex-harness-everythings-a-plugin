# Current-source focused CLI consumer check

The third attempt passed **3 selected tests**, with **297 filtered out**, against the unchanged 8,936-path Stage C source map `f05b8b9f…`. The source wrapper and strict subreaper both exited 0; the strict runner recorded one adopted exit 0 and no runner error. The actual test ELF is `93ffbf74…`, distinct from the production CLI `bbed687d…`.

| Attempt | Result | Meaning |
| --- | --- | --- |
| 01 | Preflight failed before dispatch | A scheduling guard treated a stably terminated scheduler record as active. No source/strict test receipts were created. The later capacity-only correction did not change lifecycle assertions. |
| 02 | Compilation exited 101 | ENOSPC while compiling `codex-history-notes-extension`; no tests ran. A passing preflight was insufficient. |
| 03 | 3 passed, 297 skipped | Selected `process_final::tests` verify awaiting controlled cleanup futures and retaining/reporting operation and cleanup errors. They do not directly instantiate the real plugin shutdown owner. |

The three completed cache retirements removed **129 ordinary files**: 84 `.rmeta` and 45 `.rlib`, totaling **1,182,949,376 recorded allocated bytes**. Each has a separately pinned receipt and journal. The exact recorded source, executables, proof documents, archives and protected test dependencies were retained. Reference scans had two allowlisted infrastructure-process blind spots and do not prove alias absence outside the scanned roots or through other bind mounts. They are resource audits, not lifecycle proof.

The retry used a conditional 1,118,416,896-byte overlay estimate and fresh ownership/source/disk/memory checks. This was not a measured peak or a future build guarantee. Minimum sampled overlay free space was 451,301,376 bytes; no new OOM/kill events were recorded, while the cgroup `max` counter increased by 23,003. The terminal ELF allocation and later audit allocation differ by one 4-KiB block; its size and SHA-256 match.

The 90 retired normal-production intermediates account for **977,203,200 bytes of regeneration debt**. Retained working production binaries do not imply complete production caches or guarantee that a new normal build fits. Completed retirement actions must never be replayed; subsequent compilation changes their assumptions.

The sealed [attachment evidence](../p03-current-attachment/README.md) is separate: its runtime and parser results are not included in these three tests. Full CLI/exec/TUI/workspace coverage, general ownership closure and remaining component extraction still require their own gates. No extraction family or P03 completion is added by this checkpoint.

`EVIDENCE.json` binds exact source, ELF, test/log/strict receipts and resource observations. `CACHE.json` and `CACHE_INPUTS.json` preserve the three completed resource actions and their limitations. `INPUTS.json` contains hashes and paths to private audit inputs without copying their contents.
