# P03 native reload and current-acquisition evidence

The corrected native slice passed **271 login tests, zero skipped, no in-run retries**, including 15 new test functions. Final scoped lint passed without source changes. Global formatting passed; all six changed-file diffs were independently reviewed as mechanical. No post-format test run is claimed.

The immutable source commit is `e289b6b5a2f6608779059963aef73c8cc66148eb`, tree `04ca864b074159df73d6914c851da59ba3444424`, with source base/actual parent `f507f8099554504a316dc2fe16c4636c3d97be98`. Root's created-commit receipt records all seven matching remote blobs. **Advancement of main remains pending at that receipt**; the later publication receipt owns the final branch and evidence/docs commit relationship.

## Behavior and limits

Private `LoadedAuth` carries the workspace-policy stamp through actual `reload` and guarded reload into `commit_auth_load`/`replace_auth_cache`. Credentials are locked before the exact policy guard. Owned assignment is the policy publication linearization point; policy is released before native network/watch notifications, credentials remain locked through them, and retired auth/failure values are dropped after locks.

Actual source/parsing and true external resolver transient failures preserve cached credentials. Successful absence and native restriction filtering remain authoritative None. The private `NativePolicyRejection` preserves the exact native PermissionDenied origin; unrelated permission errors remain source errors. External Restriction Transient still clears; Permanent/Policy retain classifier-specific Preserve behavior. Failure metadata requires matching current credentials under current policy, and Preserve cannot restore old cached auth.

Every actual `auth()` return, including proactive-refresh error fallback, and the final `auth_with_http_client_factory()` capture now checks the **current cache** directly against native policy. Credentials stay read-locked during validation and clone/factory preparation; an exact policy guard supplies the acquisition linearization point. Only the two fixed native preparation closures run before that guard. Denial, stale policy or unavailable policy returns None without clearing a newer cache. Raw `auth_cached()` remains a legacy unchecked getter, and public reload bool projections remain compatible.

These are policy-stamped in-memory publication and point-in-time acquisition checks. Independent credential/source-owner transitions and ABA, persistence, resolver side effects, gateway retained work, HTTP dispatch/retries/body/decode and catalog-cache publication still need connected fences. Returned credentials are not future dispatch permits. There is no new independently installed auth/catalog component, broker activation, dependency, persisted migration or full-host/App Server/GUI/plugin acceptance claim.

## Actual gates

| Gate | Result | Source evidence |
| --- | --- | --- |
| Initial reload login | 266 passed / 0 skipped; exit 0 | 8,882 before/after entries equal; subsequently exposed acquisition gap was untested |
| Initial scoped lint | Exit 0 | 8,917 entries; one reviewed closure-to-method-reference change |
| Intentional pre-fix acquisition regression | 2 passed / 3 failed / 266 filtered; exit 100 | 8,883 entries equal; test-only adoption, production helper absent |
| Corrected full login | 271 passed / 0 skipped; exit 0 | 8,884 entries equal; unchanged regression tests |
| Final scoped lint | Exit 0 | 8,919 entries equal |
| Global formatting | Exit 0 | 10,331-entry wrapper and 9,760-entry Git-visible maps agree on exactly six changed files; no Git-visible additions/removals |

All test gates used the existing strict subreaper with `--locked --retries 0 --test-threads 2 -p codex-login`. The red gate selected only the five new acquisition tests. Its three failures stop at their first failing assertion: the held resolver failure observed `with_factory=false`, `revoke_while_held=false`, `permanent=false`; this does not prove every loop combination failed. The fixed run used the identical test sources; only `manager.rs` and the new `auth_acquisition.rs` changed from red to green. All 15 new test functions have individual PASS records.

Strict runner results remain separate: initial PID 781786/wait0/exit0/null error; red PID 784482/wait25600/exit100/null error; corrected PID 785292/wait0/exit0/null error. Corrected test time was 150.064 seconds; runner elapsed 320.9448007530009 seconds. No tests, builds or resource mutations were performed by the evidence author.

The 15 tests cover actual held external resolution, policy ABA through both reload paths, successful notifications, source-error preservation versus authoritative absence/restriction clearing, permanent failure metadata, protection of newer cache, actual native AgentIdentity rejection, and current auth/factory denial/preservation including policy poison/exhaustion. The retained native candidate test proves post-load commit validation, not a blocked filesystem or public reload scheduling barrier. The earlier 256-login/one-targeted-App-Server policy gate and d04 full-host proof remain historical and are not added to these counts.

## Exact final source

| Path | Final SHA-256 |
| --- | --- |
| `codex-rs/login/src/auth/manager.rs` | `ea5f2cff15667f77eeaffb91e09b754fd6262f59454374aa507daa7499ab792f` |
| `codex-rs/login/src/auth/workspace_policy.rs` | `b0fd23019ae782164c83d69e9a1040c6f1625ca74d56e8da7d0e24daf2ae4d89` |
| `codex-rs/login/src/auth/workspace_policy_tests.rs` | `dc462f3d3b6afa8281562d21df2ebc6a1b298f964e00380f18b01ff5a45c36d4` |
| `codex-rs/login/src/auth/auth_reload.rs` | `95ed91b13b6d95ee8bcbe2e5adfc54f4df1c8deb0a64fdc53aec76e66851a2d9` |
| `codex-rs/login/src/auth/auth_reload_tests.rs` | `11ffcb5bd5f76515ef047f11a0ed79a60f950e12235ac814e7a6649273445555` |
| `codex-rs/login/src/auth/auth_acquisition.rs` | `7d1dc4a3a629ed60bfaade82e945627d67e5f290f98950a5a754fdbee3bdb337` |
| `codex-rs/login/src/auth/auth_acquisition_tests.rs` | `92081c7f4b491e34c4c5330cda5a18775c690a2bd6aecbef1895e1dd2993b8e1` |

Every corrected login source equals final lint input; final lint is unchanged and equals formatter input. The formatter changes only imports/layout, trailing commas and equivalent expression blocks; predicates, literals, assertions, lock/drop/notification order are unchanged. `auth_acquisition.rs` is byte-identical to tested source. Exact pre-format and final hashes, full report bindings and the reviewed patch are in [results](p03-native-reload-policy-results.json).

## Provenance, change size and preservation

[Lineage](../../upstream/p03-native-reload-policy-lineage.json) binds the original OpenAI Codex pin `d42056091aded7feb1d88ac7e83972108b2aa478`, `codex-rs/login/src/auth/manager.rs` blob `cdc9c8d3560359aca41ebf28b1b313e1a1793304`, SHA-256 `d2081e3e4f2f339a7a1aa52ea62d8485e7785c2b0a12dde1a7f470019d56dc2a`, and ten literal declarations read from the immutable local object. Adapted native methods, unchanged contract/reference anchors and new project-private helpers/tests are classified separately. This is bounded source provenance, not complete semantic equivalence or a full symbol graph; existing P00M checker recognition is not claimed.

The combined source diff is 762 changed lines before formatting and 1,129 after formatting. Root kept production and all 15 new test functions atomic because publishing cache-preserving reload without corrected acquisition would expose the reproduced regression; mechanical expansion was independently reviewed. Evidence/docs are separate. Historical frozen stages and their original gates remain preserved.

The exact rebased reload archive (`698b8b47…`), acquisition stage (`f250af70…`) and final source archive (`d7aafff4…`) are bound in results. Root also preserved the two completed login test ELFs in an archive with SHA-256 `915cf2c654d23c142fd7db479046c8a706b3598303a65175c10568710c6f28f4`; every decompressed member was checked against its receipt. Those binary digests were observed after the run, not captured by the historical runner. Archives remain cloud-local, without external restore proof.
