# Native installer ordering evidence — 2026-10-01

This checkpoint validates a **compiled native authentication prerequisite**: latest-admitted installation ordering and rejection of stale publication after clear, policy or credential changes. It does not install an auth component. Full-host acceptance remains blocked by the separately preserved curated-sync lifecycle failure.

The [results JSON](p03-install-order-results.json) binds exact commands, raw reports, logs, wait statuses and source maps. The [native lineage map](../../upstream/p03-native-install-order-lineage.json) extends the unchanged source-owner map, distinguishing native adaptations from project-private support and tests.

## Source and adoption

The exact native base is source `99e6802fd478e55686559aefdd6ae79177a45397`, tree `651cca62fb376fed10b50872165edda8d9b099e3`. Root adopted the final-base candidate manifest `63ed01b3beba1933dea700b05e24628720a68f4460a4c59f18c134cc97e93fb9`: tests first, then exactly three production paths. Candidate provenance SHA-256 is `4dfedc05c57372c4df7869e1638145da4de6638a52e53cf1aa8dd7501b68ae4a`.

Final native source is `65511842d7051b2a1f5cc52917f3ebb5c03be4f3`, tree `ed564f68f3297cd65b65321d1f41f79b4f9c4031`, parent `63dded3a250527fae51fa61d1e24b70a4029044b`. All `8889` scoped final entries match the completed format snapshot and were independently rehashed. The four changed source paths are `auth_install_tests.rs`, `auth_reload.rs`, `auth_source.rs` and `manager.rs` under `codex-rs/login/src/auth/`. Published `auth_source_tests.rs` remains byte-identical. Historical wrapper `baseline` fields are not the actual source identity.

The final-base stage archive is bound separately. Original red source maps, logs and strict report are retained; no retained red test executable or external backup is claimed. This evidence author ran no Rust commands and made no repository or Git changes.

## Terminal native checks

| Gate | Actual result | Source and runner evidence |
| --- | --- | --- |
| Tests-only red | **9 run: 3 passed, 6 assertion failures**, 283 filtered, 0 retries; compile **2m 45s**, tests **0.061s** | 8,889 entries unchanged; command/strict exit **100**, null runner error; sole recorded reap PID 828179, wait status 25600 |
| Full login + model-provider green | **400 passed**, 0 failures, 0 skipped, 0 retries; compile **2m 58s**, tests **166.651s** | 8889 entries unchanged; exact new test file unchanged from red; command/strict exit 0/0, null runner error |
| Scoped fix | Exit 0 | 8889 entries; 0 warning lines recorded |
| Formatting | Exit 0 | 2 files changed among 9788 Git-visible entries; mechanical review bound separately |

The unfiltered green scope contains `292` login tests and `108` model-provider tests. All nine new functions passed, completing their coded variants. Their red/green test file SHA-256 is `168da6c4fe1cded025ed624d935d2f197be74ebb53c9841c1a932e17df20692d`; final formatted test hash is `bc54d238cf3751237145bacda20f9496a6de792f2bb350d9cedd0ad9e07a77dd`. Formatting is a separate transition after the test run; tests were not repeated solely for formatting. The green strict report also recorded PID 830991, wait status 9, return code -9. Executable attribution and cause are unknown; this is not an all-child-exit-0 claim.

The six actual red failures were:

- Held older installation was not rejected after a newer equal-credential installation. The first equal-key variant failed at line 114; the different-key variant and later owner/watch assertions were not reached.
- Clear with no active external source failed to invalidate a pending initial installation at line 142. The first `native=false` variant failed; `native=true` and later preservation/reload assertions were not reached.
- A failed newer admission allowed older work to remain eligible at line 186. The `cancel=false` variant failed; cancellation and later exact owner/failure/watch assertions were not reached.
- Policy change-and-return did not cause the expected native transient rejection at line 206. Later owner and classifier-count assertions were not reached.
- Real current-provider refresh did not fence held installation at line 232. The first `aba=false` variant failed; cache-ABA and later failure/watch assertions were not reached.
- Poisoned owner still invoked provider resolution once, versus the required zero calls, at line 285. The caught panic at line 281 intentionally poisons the fixture; the actual failure is the later resolve-count assertion.

These line numbers refer to the exact unformatted red test file. The three red compatibility passes cover an unpolled future, real stale-provider error classification without poisoning the winner, and workload public no-ops preserving private initialization. Tests hold and poll real provider futures with bounded waits; these are not HTTP or installed-plugin tests.

## Contract and limits

The first poll of an installer reserves a retained latest intent and captures source identity, credential revision and policy. Failed or dropped newer work does not restore older eligibility. A successful equal-credential installation advances source identity independently of credential-change notification. Final paired provider/credential publication checks source, intent and cache under the owner guard and the captured policy under its exact policy guard.

Clear with no active external provider invalidates pending installations while preserving native credentials, permanent failure, revisions and notifications. Workload public replacement/clear restrictions remain. Real provider errors retain their classification; native conflicts become Transient without provider classification. Provider callbacks and trait-backed retirement remain outside owner/policy locks. No public ABI, component contract, dependency or persisted-schema change is introduced.

The lineage map retains original upstream `d42056091aded7feb1d88ac7e83972108b2aa478` manager symbol/blob anchors through the bound final-base provenance. This pass refreshes final destination hashes and line anchors; it does not newly re-derive upstream objects. Literal ancestry is not semantic equivalence, full call-graph closure or updater acceptance.

Source identity is retained and advanced here, but load/recovery and refresh success/outer failure do not yet consume it. These installer-only tests do not independently distinguish source revision from intent invalidation. Shared ephemeral persistence still occurs outside locks before final memory publication, so a denied attempt can leave storage side effects. Conditional persistence, durable logout, provider-retained work settlement, request/catalog authority and per-retry/response publication remain separate work.

The [source-owner runtime checkpoint](P03_SOURCE_OWNER_FULL_HOST_EVIDENCE.md) remains **mixed/failed**: both storage attempts exited strict 125 despite passing behavior checks, while migration and GUI passed. Its source99/CLI `c711…` evidence does not validate this installer change. The [curated-sync diagnosis](P03_CURATED_SYNC_LIFECYCLE_DIAGNOSIS.md) remains the full-host blocker. No fresh full CLI/GUI, installed native auth/catalog, broker activation, whole-harness completion or upstream update/rollback acceptance is claimed.
