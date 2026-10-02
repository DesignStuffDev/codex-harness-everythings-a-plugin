# P03 curated replacement acceptance

This checkpoint records scoped runtime acceptance and the remaining platform gates. The JSON files include hashes and allowlisted typed results. Private raw logs, fixture configuration, ports, authentication data, process IDs and session/turn UUIDs remain outside this draft. Artifact paths identify VM-local originals; hashes do not make those originals externally durable.

## What this establishes

The native stage passed 431 ordinary App Server library tests (one ignored helper was invoked separately). Its bound ELF then passed three fresh-process parent-driven cases: pending native work while A closes/B starts, completed-work replay, and concurrent release beside B start. Each case used a fresh home shared by its A and B runtimes, with real native Git/App Server code and deterministic Git/model/MCP fixtures. A retired with zero callbacks, B with one completed callback, and the final native worker was joined with no quarantine or unexpected handles. Runtime02 and all ten adopted subprocesses exited zero; no rescue cleanup was needed.

The initial parent run remains failed: pending aborted with a Tokio worker stack overflow before B. The parent was corrected to pass the repository recipe's 8 MiB RUST_MIN_STACK before launching the same test ELF. Assertions and deadlines did not change. The ordinary 431-test suite had nonzero adopted subprocess statuses despite successful suite/strict exit; its exact distribution is retained and is distinct from runtime02.

## Source lineage

Published stage1 base `3ccba58b1077e9372458fe2994a72f30ee643e41` / actual source map `6f11c751…` → native child stage2 (63 selected paths; 8926-path map `1d8ccc03…`) → SDK parent (65 selected paths) → stack-setting correction / successful runtime02 map `6e7b4081…` → separately reviewed `just fmt` transition (38 files: 34 Rust and four Python; map `4bfe3b73…`). No post-format native build/runtime pass is claimed, and tests were not repeated solely for formatting. All four Python AST checks passed; the 34 Rust changes were independently reviewed as mechanical. All 60 frozen historical artifacts retained their exact hashes.

The authoritative formatting manifest is v2 `7d9b57fc…`; it corrects inherited metadata in superseded `16f3d289…` without changing any of the 65 source fingerprints. The later final manifest `bfad182c…` records the test-only lint transition and full map `1c54717d…`. [Source lineage](../../upstream/p03-curated-replacement-lineage.json) retains these separate identities and the historical FINAL_SOURCE record.

Scoped Core Plugins lint passed unchanged. App Server lint first failed after its automatic fix removed a test-used import. Moving that same import into the test module allowed the retry to pass; Clippy also made two equivalent test-fixture edits. Final formatting passed unchanged. [Lint evidence](P03_REPLACEMENT_LINT_EVIDENCE.json) and [the exact patch](P03_REPLACEMENT_LINT.patch) preserve the failure, correction and successful checks. No assertions or deadlines changed, and no tests were repeated solely for lint or formatting.

## What remains open

This is same-home curated callback/worker lifecycle acceptance, not a newly extracted component, installed component-plugin replacement, or hot swapping active components. MCP transport shutdown custody remains unproven and `whole_host_clean` deliberately remains false. The new production build/eight-case held Git/HTTP matrix and GUI regressions remain pending for this newer native stage. Current58 storage/migration/slow-storage/GUI evidence stays attached to its older source. Broader subsystem extraction, updater acceptance and final clean-install custom-plugin/UI gates remain open in the canonical roadmap.

[P03_REPLACEMENT_EVIDENCE.json](P03_REPLACEMENT_EVIDENCE.json) contains execution results; [source lineage](../../upstream/p03-curated-replacement-lineage.json) contains compact transitions and source identities. Full source maps and raw failed/passed artifacts remain preserved at their referenced original paths. Consult EXECUTION_STATE.md for subsequent lint/build/publication progress; these runtime identities remain immutable.
