# P03 wire slice 1: lint and formatting follow-up

`just fix -p codex-component-host` exited 0 and left all source fingerprints unchanged. `just fmt` then exited 0 and changed exactly three files: `session_wire/codec.rs`, `session_wire/payload.rs`, and `session_wire/sending.rs`. No file was added or removed.

Every diff hunk was reviewed against the checksum-verified adoption archive. Formatting collapses one existing `ensure!` call onto a single line and removes a trailing empty line from each of the three files. The changes are whitespace only: literals, assertions, control flow, policies and evaluation order remain unchanged. Both existing wire tests and the new raw-byte test file are byte-identical.

The complete 10,153-entry maps match across tests → scoped lint → formatter input. The separate JSON records those map hashes, exact tested/formatted file hashes and all diff hunks. The original three tested-source lineage/evidence files remain unchanged.

The earlier 87 passes and one skip apply directly to preformat bytes: 81 host-library tests plus six manager tests; API compiled without executed pass entries. No postformat test rerun is claimed or needed solely for these mechanical changes. A fresh codec-linked executable and genuine installed-worker runtime gate remain separate acceptance work. This follow-up does not activate the broker or count shared custom transport support as native subsystem extraction.
