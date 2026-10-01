# P03 broker limits: checked configuration API

The API-only gate passed **10/10 tests, zero skipped, zero retries** on October 1, 2026 (14:03:11–14:03:15 UTC). Four new tests cover checked serialization/deserialization, zero/invalid concurrency limits, exact request-plus-response spool fit and overflow, process insufficiency, and raw duplicate/missing/unknown fields. Six existing declaration tests also passed. One API test binary ran; this is not a broker runtime acceptance.

The change adds explicit unchecked specs, immutable validated connection/process limits, structural ceilings, checked deserialization and typed validation errors to the custom `component-api` crate. Two files are new; `lib.rs` adds exports/test registration. No native Codex subsystem was extracted. Construction validates settings only: it neither reserves resources nor enforces live quotas, schedules codec work, grants authority or activates a broker. The frame-staging arithmetic is not measured memory usage.

`just fix -p codex-component-api` passed without source changes. `just fmt` passed and changed only `broker_limits.rs` and `broker_limits_tests.rs`. Independent diff review found mechanical wrapping, trailing commas and braces around an unchanged string expression; predicates, literals and assertions remained unchanged. Tests used the pre-format bytes; no redundant post-format rerun is claimed.

All three source reports contain **8,785 entries**. The complete test before/after maps equal the lint before/after maps and the formatter before map. The JSON records the separate formatted hashes for all three source paths. The strict subreaper exited 0 with no runner error in 2.798177027 seconds; its raw record is one command reap, PID 744189, wait status 0 / return code 0. No broader child-process behavior is inferred.

The source was adopted exactly from frozen manifest `90a0cf5a0d81d38b42d3a11a2e833ccf797041a16e13388069cc23ec6afaac11`, based on publication `8ee6b667521e49ed9879a5ccd35cc22aa6595458`. The adoption archive digest is `55acebe5ea9645b53a4ec1ca68bda6566b971f38c9108479982b009cb172e0ce`; that archive is cloud-local preservation. The stage's uncompiled status is historical and is superseded only for these tested API paths by the completed reports.

This checkpoint proves configuration validation. It provides no new independently installed plugin, host/App Server/CLI/TUI/GUI runtime, broker enforcement or cancellation proof. Previously completed CLI evidence remains tied to its earlier source checkpoint.

Details: [results](p03-broker-limits-results.json) and [source lineage](../../upstream/p03-broker-limits-lineage.json).
