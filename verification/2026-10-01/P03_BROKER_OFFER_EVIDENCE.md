# P03 checked broker offer and acknowledgement

The API gate passed **20/20 tests, zero skipped, zero retries**, on October 1, 2026 (14:25:28–14:25:33 UTC). Five new offer/ack tests passed alongside five grant tests, four budget tests and six declaration tests in one API test binary.

The new checked offer canonicalizes service identities, validates operation caps against connection limits, and matches complete required/optional declaration sets. Acknowledgements use the same checked wire shape and compare every canonical value: connection handle, all nine connection limits, service identities/versions, authority handles, operation sets and both directional caps. Tests accept equivalent reordering and reject altered, missing, extra, narrowed or widened grants. They also exercise empty optional-only offers, empty offers with required declarations, case/Unicode distinctions, service-count boundaries and malformed original JSON bytes.

This is custom API validation, not a native subsystem extraction or active broker. Parsing must receive original bounded bytes to preserve duplicate keys; decoding an already materialized Value cannot recover duplicates. Serde allocation limits, live authority/revocation, connection permits, runtime quotas, actual handshake activation, missing-ack startup behavior and cleanup remain host obligations. No installed-worker or host/App Server/CLI/TUI/GUI runtime proof arises from this gate.

The original frozen module manifest is `6336a7f54c713b6e8d3b420a30af463bfe6fbec89fbcdd6ca90629675cfd59f9`. Root's integrated manifest is `218c2d1183b67282c58ee166dd2dbf985268f4af52110cd7996adcb2a487ee61`: the two module files are byte-identical to that stage, while library registration was merged onto the accepted grant `lib.rs` preimage `7cd447933f44428ceaaac200385803010376246e83814a44ede20b3f382ee17c`. The tested merged library hash is `4ceaee48795dadb36645d6ce99d1f81916ac41df0616cabe0ac374c1009508d7`. The older budget library snapshot was not restored.

The publication parent is the verified grant checkpoint `6c5b232518ca951ebc80d5199cae01c901f68c85`. The root adoption archive is `b49975c969ff0c1513afd5d1f55682cc09a15c16a0c1776fea30bc0da31063ca`; that archive alone is cloud-local preservation. Historical stage labels saying uncompiled are distinguished from the actual terminal test evidence here.

All test/lint/format source maps contain **8,791 entries**. Tests and `just fix -p codex-component-api` both passed with unchanged source. `just fmt` passed, changing only the two new offer files. Independent diff review confirmed mechanical wrapping, trailing commas and whitespace; predicates, literals and assertions were unchanged. Tests ran on adopted pre-format bytes, and the results/lineage record the separate formatted hashes. No post-format retest is claimed.

The strict subreaper exited 0 with no runner error in 3.779436039 seconds. Its sole raw reap is command PID 745937, wait status 0 / return code 0. This is not proof that a broker worker or host session was exercised.

Details: [results](p03-broker-offer-results.json) and [source lineage](../../upstream/p03-broker-offer-lineage.json). Earlier standalone CLI runtime evidence remains tied to its earlier codec source checkpoint.
