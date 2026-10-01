# P03 broker grant validation evidence

The checked grant API passed **15/15 tests**, with no skips or retries: five new grant cases and ten existing declaration/budget cases. This is custom validation infrastructure, not native subsystem extraction or an active broker.

The five-path, 367-addition candidate was adopted exactly from manifest `ba38a6cf6505f8552ad8aca2fd5015edcbe61437a70f255ee9742c722d2a4e2f`. Its cloud-local adoption archive is SHA256 `81bcc6743cda6ec96e71ededcbdf5ae1ba3928b9bd0984bcbfcb8486453b57d4`. Publication parent is `5fbe8a7901e1f935c40e969b4bb3cd7811e0d6be`; grant publication binding remains pending. The recorder's older `baseline` field does not identify the tested dirty tree.

Validation ran `just test --locked --retries 0 --test-threads 2 -p codex-component-api` inside the unchanged strict subreaper, with `CARGO_BUILD_JOBS=1` and `NO_COLOR` unset. All 8,789 recorded source entries were unchanged. The outer runner returned zero, had no runner error and reaped one command with status zero. `just fix -p codex-component-api` returned zero without source changes. No failing grant attempt was observed in these reports.

The tests cover checked UTF-8 handle/name limits, redacted Debug with intentional wire serialization, positive operation caps, descriptor/count limits, exact duplicate rejection, canonical operation ordering without collapsing case/Unicode distinctions, handle identity, and raw duplicate/missing/unknown JSON fields. Positive standalone caps are descriptions; dispatch must later compare them against connection limits. Validation does not bound allocations before serde materializes strings/vectors or issue runtime authority.

`just fmt` returned zero and changed only `broker_grants.rs`, `broker_grants_tests.rs` and `broker_negotiation_error.rs`. Full before/after diffs were reviewed: wrapping, optional trailing commas and braces around the same string-valued match expression. Predicates, strings and assertions are unchanged. Tests were not rerun solely for formatting. The results file retains every transition and both tested/formatted hashes. The unchanged formatted `component-api/src/lib.rs` SHA256 is `7cd447933f44428ceaaac200385803010376246e83814a44ede20b3f382ee17c`, the preimage for later offer registration.

Source reports and logs are `/workspace/acceptance/p03-broker-grants-{tests,fix,fmt}.source.json` and corresponding `.log` files; the strict runner report is `p03-broker-grants-tests.subreaper.json`. The [results](p03-broker-grants-results.json) and [lineage](../../upstream/p03-broker-grants-lineage.json) files contain exact report identities, commands, source-map digests and all five file bindings.

The existing native catalog audit is preserved byte-for-byte in `component-sdk/design/native-catalog/{NATIVE_CATALOG_NEXT_CHECKPOINT.md,SOURCE_BINDINGS.json}`. Those documents propose provider-owned endpoint and policy-epoch prerequisites and later native catalog extraction; they are not implemented source or runtime proof.

Offer/ack remains a separate pending DTO slice. Resource accounting, retained settlement, original-byte handshake validation and installed native catalog integration remain future work. The fail-closed inactive catalog guard stays in place. This gate provides no new installed-worker, service execution or GUI evidence.
