# P03 wire slice 1: custom shared transport support

The adopted seven-file slice passed **87 tests with one skipped**, zero failures and zero retries. The log attributes 81 passes to the component-host library and six to component-manager launch tests. Component API compiled, but no API tests executed in the logged pass entries. The three-binary count must not be reported as three independently tested subsystems.

This slice reorganizes the project's existing custom persistent-session transport; it does **not** extract a native OpenAI Codex subsystem or activate a broker. At pinned upstream `d42056091aded7feb1d88ac7e83972108b2aa478`, neither `codex-rs/component-host` nor `codex-rs/component-api` exists. The actual source baseline is published P02 `c28a1c33a856a987316f4b97b4c488d68055fffc`, tree `b40fa4d6f635dea2de7ca1856aaf96d5af58f37b`.

| Location | Existing responsibility now isolated |
|---|---|
| `session_wire/payload.rs` | Temporary spool ownership and blocking serialization/parsing. |
| `session_wire/codec.rs` | Existing physical frame schema, bounded raw-byte reading and frame writing. |
| `session_wire/assembly.rs` | Existing ID tracking, admission, logical payload assembly and EOF checks. |
| `session_wire/sending.rs` | Existing outgoing validation, spool progress and flush-before-send fence. |
| `session_wire.rs` | Retained public/session types, component validation, reader composition and control/regular arbitration. |
| `session_wire_tests.rs` | Six existing test bodies retained; three explicit imports added. |
| `session_wire/raw_tests.rs` | One new production-reader regression with four raw duplicate-field cases. |

The lineage JSON maps original symbols and lines to the tested destinations, records all old/staged/adopted hashes, and verifies unchanged adjacent boundaries. Frame fields, component limits, 4 MiB physical frame bound, 192 KiB chunks, 64 pending assemblies/ID gaps, 64 KiB control allowance, optional ordinary payload caps, shutdown ordering and send fences remain. Raw frames are still decoded directly into the same derived schema; no intermediate `Value` can erase duplicate fields. No new task, lock, process policy, dependency, API version, or broker registration is part of this slice.

The new raw duplicate-field test passed for duplicate ID, byte count, variant tag and nested component kind. Existing tests passed for a 17 MiB history string plus JSON envelope, malformed streams, bounded out-of-order IDs, partial-result errors, lazy JSON parsing, urgent-control interleaving and cleanup fences. The broader focused gate also covers payload budgets, retained paired-start/release ownership, process reaping, launch interruption and explicit shutdown uncertainty.

The exact 10,153 source fingerprints were unchanged during the gate. The adopted `session_wire.rs` SHA-256 is `d86453e22238de4ac0c8aa756e76420eedfabfc0d81c71d0bcc90676be8aaeeb`; its only change from the frozen stage is the repository-required explicit raw-test module path. The preserved source archive and manifest retain that provenance. Tests apply to the full adopted aggregate, not separately to each historical review patch.

The test/source wrapper and unchanged strict subreaper exited 0 with no runner error. All four raw reap records remain in the results JSON: three return −9 and one returns 0. This evidence does not infer their individual causes or claim all descendants exited zero. The terminal log reports one skipped test without naming it.

Lint and formatting follow as separately recorded checks. Real installed-worker interoperability requires newly built executables that link this codec. The accepted P02 full CLI and its search/storage/GUI results predate this change and cannot prove that new runtime behavior. The separately staged broker extension remains unlinked. Prior partial-frame cancellation and blocking-task ownership semantics are unchanged, not newly guaranteed by these file boundaries.
