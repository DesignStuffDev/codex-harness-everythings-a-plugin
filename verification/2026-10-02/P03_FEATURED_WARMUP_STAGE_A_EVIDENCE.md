# Featured warmup ownership — Stage A checkpoint

Stage A adds ownership support in three native paths: `core-plugins/src/lib.rs`, `featured_warmup.rs` and `featured_warmup_tests.rs`. It does **not** activate that owner in production, fix the auxiliary startup race, or extract another installable component. The original detached featured-warmup block in `PluginsManager::maybe_start_plugin_startup_tasks_for_config` remains byte-identical to upstream.

The owner retains the exact task handle, fences admission and scoped publication, and preserves custody when an observer is canceled. The revision replaces the async polling mutex with a private single-permit semaphore, adds the queued-observer cancellation regression, and exports nameable observation/outcome/ownership types. Ten targeted ownership tests now sit within the existing library suite. Joining this future does not prove completion of transitive HTTP internals; synchronous publication locks remain outside the async deadline guarantee.

| Check | Observed result |
|---|---|
| Original Stage A | 551/551 passed, zero skipped; original lint exited0 with four warnings. Preserved separately. |
| Revised Stage A | 552/552 passed, zero skipped; source unchanged. |
| Revised scoped Clippy | Exit0, source unchanged. Poller warnings resolved; one dead-code warning remains for the intentionally unactivated process-close helper. |
| Final formatting | Exit0, source unchanged. |

Both test runs used strict subreaping and reported command0/exit0/error null, with adopted statuses `{0:36,128:2,-9:4}`; no all-zero claim or new attribution is made. Revised lint recorded one adopted status0. The revised test ELF is `4bbb4969…` (219,815,664 bytes), bound to the completed source receipt; this is a test artifact, not a production-host/runtime acceptance result.

The actual tested and finally formatted source has8,930 files, map `0895e8d7…`; corrected candidate is `c5bcf5bc…`. The dispatched `4181a929…` candidate is preserved exactly, but inherited stale full-map/format labels. Actual before/after wrapper fingerprints and the corrected metadata establish the source identity. The lineage records original551 map `07e852fd…`, semaphore/test revision `10b44ff4…`, and the final public-type-export transition to `0895e8d7…`.

Parent is the published checkpoint `7a811ea49f6ee7709b9556fbd9d3d3979f283a94`, tree `493ce644…`. [Source lineage](../../upstream/p03-featured-warmup-stage-a-lineage.json) records exact SHA-256 and computed Git blob IDs for all three paths, upstream `d4205609…` origins, unchanged detached production behavior and the parent publication/source binding. No successful local lookup of that parent Git object is invented.

The earlier551 proof archive checksum `d610b4dc…` was rechecked; its preserved receipt records stream verification. It remains a VM-local recovery artifact, not an externally durable backup. [Evidence](P03_FEATURED_WARMUP_STAGE_A_EVIDENCE.json) contains exact test/lint/format, candidate, binary and preservation hashes.

Next: review/adopt Stage B wiring into manager, processor and process-final ownership, then run real production lifecycle/regression gates on that new source. Current production GUI/storage/matrix proof remains tied to the preceding baseline; it is not relabelled as Stage A runtime proof. Component coverage, P03 completion, MCP ownership and whole-host cleanup claims do not advance from this support-only checkpoint.
