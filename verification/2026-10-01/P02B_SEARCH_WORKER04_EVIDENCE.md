# P02B independent search worker04 build evidence

Worker04 built successfully from an exported ten-crate source closure in a
separate, initially empty target directory. All seven commands completed with
exit 0, no retries, timeout or emergency cleanup; all observed PIDs were absent.
The unchanged outer subreaper exited 0 with no runner error. This is **build
proof only**: worker04 has not yet been proven installed or compatible at runtime
by this record.

The package worker is 10,362,672 bytes, SHA256
`a3f73fed15dd7b19ab0f1daed55a74212d4b2f5d2ba5cdebee2a616d2f2419a1`.
Its package declares version 0.2.0, API 1, search contract 1 and
`preparing_cancel_receipt: 1`. Those declarations are not behavioral proof; the
Rust crate still uses workspace version 0.0.0. All 43 package-file fingerprints
were independently rechecked, including Apache LICENSE/NOTICE, third-party notices,
and the modified Nucleo/MPL source and provenance.

The frozen full Codex CLI remains SHA256
`7a63e9406d1605bac0a84e1b703735caeb211ceccf148337acf07614c7c0032d`;
the original manager remains
`eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a`.
Before/after and fresh audit comparisons preserve bytes, size, mtime and mode;
both files have one hardlink. Neither host was rebuilt for this worker build.

The build uses offline Cargo, `--locked --profile dev-small -j1`, incremental
compilation disabled, and existing registry/toolchain caches. Cargo's
`fresh:false` record proves this worker artifact was compiled rather than reused.
All local dependencies resolve inside the export. The host-platform graph has
102 packages; the resolved lock retains 141 package records and introduces no new
identities relative to the original lock. Full core, CLI, App Server, TUI and
runtime-facade crates are excluded from the exported local build closure.

The preserved export contains 196 inventoried source files. 195 match inventory
export hashes directly. The sole exception is Cargo.lock, whose documented
resolution changed from the original full-workspace lock to the subset lock
`dbe166dc08aa0c038fd4e7d9c5625e3f0a66af08b0c9e61c014b1f1192c6c3ac`.
The audit checked that exact preserved lock. Four proven-unused pinned patch
declarations were removed only from the isolated export: crossterm,
tokio-tungstenite and two origins for tungstenite. Full package records before
and after normalization and metadata package/resolve/workspace-member structures
were independently compared equal. The final locked build preserved lock bytes.
All fourteen command stream hashes were rechecked.

The original build-source path is absent; source is preserved at
`/workspace/acceptance/p02b-search-independent-04/source.parked`.
The exporter inventory's `independent_build_verified:false` is its original
pre-build state, retained unchanged. The later successful build report supplies
the build proof. Source/package/report artifacts remain cloud-filesystem
checkpoints, not externally durable artifacts established by this audit.

For backward runtime interoperability, the existing
`component-sdk/tests/file_search/app_server_acceptance.py` supports this exact
frozen full CLI with `--server-mode codex`, the original manager, worker04 build
report, and the existing full-CLI build report. It needs a fresh external work
directory and the unchanged subreaper. Its existing gates cover exact ordered
native/installed parity, streams and sibling isolation, real resource exhaustion,
selected failure without fallback, removal/restoration and strict PID absence.
No script or fixture change is required to select that mode.

`cli_acceptance.py` instead expects a standalone search executable and passes
search flags directly. It cannot use full Codex 7a63 as-is; swapping in the old
standalone CLI02 would correctly fail the worker04 build report's strict frozen
CLI fingerprint check. A compatibility test using the old full host also cannot
prove new host-side Preparing cancellation: that requires a newly built host and
separate causal gates. No installed proof, GUI/Browser result, release/update
completion or new subsystem extraction is claimed here.

Later qualification: [old-host installed runtime](P02B_WORKER04_OLD_HOST_COMPAT_EVIDENCE.md) is separate evidence. Root formatting later changed three Python helper files; [exact before/after hashes](p02b-worker04-format-followup.json) preserve that distinction. No worker rebuild or formatting-only retest is inferred.
