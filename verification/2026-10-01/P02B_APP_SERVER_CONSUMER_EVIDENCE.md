# P02B App Server selected-search consumer: scoped gates

Status: **verified partial App Server consumer milestone; source published at `30c2674600cc84da161562c407515b61511b7c3a`**. The frozen App Server passed real independently installed native-search acceptance as well as scoped library/contract and public RPC gates. These are separate runs, not one aggregate suite. The source tree is `86c0bce7a5b399c2337c7c3878ed12ee76da4918`, parent WIP `651b0b87a281934e5cd7c639021049fad8d393ac`. Main `f2cc6c2ed7dd84f606d43947a3c71f85bfc0daad` remains unchanged. This evidence update is staged separately from that source publication. No whole-harness, TUI, GUI or live-model completion is claimed.

## Actual consumer changes

- App Server starts one `file_search/default` selection and owns its shutdown outside abortable request processors. Independent connection scopes share that selected provider; original lexical roots and native search options remain explicit. A selected implementation error does not select the native fallback.
- Session startup, updates, observation and close have retained owners and query fences. A bounded publisher emits current snapshots before completion and separates explicit stop from a real terminal failure. New `fuzzyFileSearch/sessionFailed` carries a bounded diagnostic and stable error category; it does not claim cleanup has completed.
- The client exposes the selected search scope to an in-process picker without creating another provider owner. Search and storage receive deliberate shutdown intent and their normal cleanup is awaited. TUI adoption was applied later, after the App Server freeze; those newer bytes are excluded from this milestone and its acceptance.
- JSON/TypeScript protocol artifacts and Python SDK types were regenerated. Public RPC behavior passed the separate 13-case executable gate below. Actual installed selection and worker drainage also passed the separate unchanged-host acceptance below.

Core paths are `codex-rs/app-server/src/{file_search_services.rs,fuzzy_file_search.rs,fuzzy_file_search/,request_processors/search.rs,request_processors/search/}`, `codex-rs/app-server-client/src/file_search.rs`, and `codex-rs/file-search-runtime/src/interactive_policy.rs`. The machine report preserves the historical 54-path library binding and adds all 185 final archived paths, including 170 exact overlaps with the frozen App Server build scope. It never reads later live TUI bytes to form those bindings.

## Completed evidence

Every test command below used the unchanged SDK subreaper; its exit status agrees with the child and `runner_error` is null. Rust tests used `just test --retries 0`; there were no automatic retries. A failed compiler command is not counted as tests run.

| Run | Actual result | Source evidence |
| --- | --- | --- |
| `p02b-as-client-tests` | Compile exit 101; three production imports targeted nonexistent `codex_app_server_protocol::v2` | 8,739 scoped fingerprints unchanged; no tests executed. |
| `p02b-as-client-tests-02` | Compile exit 101; one test import had the same wrong module path | 8,739 unchanged; no tests executed. |
| `p02b-as-client-tests-03` | 457 executed: **456 passed, 1 failed**, 0 skipped; exit 100 | 8,739 unchanged. Stopped-scope error test expected `closed`, but the real retained cause says `file-search runtime ownership is closing`. |
| `p02b-as-client-tests-04` | **457/457 passed**, 0 skipped; exit 0 | 8,739 unchanged. App Server 415 + client 42. The sole preceding edit tightened the test to the full actual error phrase; INTERNAL_ERROR, no successful result, empty admission and cleanup assertions remain. |
| `p02b-as-protocol-runtime-tests` | **351/351 passed**, one ignored generator helper; exit 0 | 8,739 unchanged. Protocol316 + runtime35. |
| `p02b-as-python-sdk-contract` | **27 passed**; exit 0 | 1,102 unchanged fingerprints across the explicit SDK/schema scope. Contract generation, async-client behavior and public API signatures. |
| `p02b-as-protocol-schema` | Stable generator exit 0; explicitly invoked generator helper passed | 12 scoped paths changed, including Cargo.lock; new notification schemas/types and stable precomputed exports recorded. |
| `p02b-as-protocol-schema-experimental` | Experimental generator exit 0; helper passed | One scoped path changed: experimental precomputed export bundle. |
| `p02b-as-client-bazel-lock` | `just bazel-lock-update` exit 0 | Cargo.lock and MODULE.bazel.lock unchanged during this command. This is not a Bazel build/test claim. |
| `p02b-as-public-search` | **13/13 passed**, 1,448 filtered; exit 0 | 8,739 unchanged; no retries and clean subreaper. Twelve one-shot/session public RPC cases and one WebSocket connection-isolation case in this single run. |
| `p02b-as-client-fix` | Scoped `just fix` exit 0 | 8,739 unchanged fingerprints; App Server/client/runtime packages. |
| `p02b-as-client-format` | `just fmt` exit 0 | 26 changed Rust/Python paths out of 8,833 scoped entries. Exact transitions recorded; tests were not rerun merely for formatting. |
| `p02b-as-build` | Locked real `codex-app-server` build exit 0 | 8,739 source fingerprints unchanged; frozen binary copied without transformation or Cargo hardlinks. |

The protocol's single skipped helper is `schema_fixtures_tests::write_schema_fixtures_from_env`, intentionally invoked by the two generator commands. Do not add those two generator executions to the 351 regression count.

[Machine results](p02b-app-server-consumer-results.json) contain exact commands/times/statuses, report and log SHA256 identities, whole-scope map digests, each changed path's before/after hash, and strict subreaper reports. Original failed logs/reports remain under `/workspace/acceptance/`; no failed result was rewritten as passing. Import corrections used actual crate-root reexports. The correction between attempts 03/04 changed only `request_processors/search/search_tests.rs`, from SHA256 `56ba36e7fdc77a2fe1bd4ea28adffc63ddd02193850b9345910de61b8af35615` to `9c3a0ad2727c1e433a60db0952f6c8faec8bd3113b1293929d3763e250b850bf`.

## Schema scope qualification

`write_schema_fixtures.py` also invokes Python SDK generation during the stable command. The first schema report did **not** capture those SDK files' before/after hashes; it cannot prove that SDK output was unchanged. The subsequent expanded experimental capture and separate SDK test bind 45 overlapping SDK source/test files with no intervening change. Generated `v2_all.py` is SHA256 `e1d5dcdd2d46a49109c0d15fa9fb0b8ecb262de57a0a994123f06f9f1faf1805`; notification registry is `eb0bee91a52dae6b2fec44afd439906b4cad1fccb018c0c5ece034eeb04d5cd7`. Missing pre-generation SDK hashes are not invented. The expanded schema capture includes SDK tool-cache entries; fingerprint counts are not counts of source files or additional tests.

## Preservation and later gates

Cloud-local archive `/workspace/recovery-backups/20260930T165936Z/p01-source-20261001T072520Z.tar.gz` has SHA256 `6045d26e96e688e04132607fd56ed5e6ca2d88084af27c60c2e5aa1f4e0f3aa1`. All 185 listed source members were independently rechecked against its manifest during evidence preparation. It includes the corrected public RPC fixture and applied external acceptance tooling. It is a recovery checkpoint, **not externally durable publication** or a claim that the entire source delta is accepted.

The corrected WebSocket fixture explicitly submits an empty query after the start acknowledgement: the runtime suppresses initial query-id0 callbacks. Its archive SHA256 `c9f989f89c72f16208fee0cf7c275e55e0f34c6a4db6d9a2e5193478f06fec5f` differs from the library gate's captured old fixture `7bf2e5f6654870ae2ff2c4137e7b6ad5f5eb609ee578eb09f378f16600e0a774`. Neither fixture executes in `--lib`; the corrected pre-format fixture subsequently passed in the separate 13-case public RPC gate. Its later formatted hash remains a distinct source identity. Ordering, query identity, connection privacy, no post-stop grace period and process cleanup assertions remain; explicit terminal-error/duplicate-frame checks are stronger.

A later cloud-local checkpoint `p01-source-20261001T073424Z.tar.gz`, SHA256 `aa6d2f9e70aae809b4619d8a71c5a44f505ef80f01a5bda71e9ce083527e5159`, preserves all 185 source paths after formatting; each member was rechecked against its manifest. This is not a post-format test rerun. The final binding uses this 073424 archive over main `f2cc6c2ed7dd84f606d43947a3c71f85bfc0daad` and the completed build report's `source_after`. All 170 overlapping archived/build paths match exactly; the other 15 delta paths are outside that build scope, with runtime tooling bound separately where applicable. The older 54-path binding remains historical. The newer 074033 TUI archive must not be substituted.

[The upstream mapping](../../upstream/p02b-app-server-lineage.json) records four component boundaries, 44 exact original/current symbol anchors and all 62 changed paths. These are selected ownership/contract mappings, not a complete semantic graph or implemented updater.

Parent published 62 changed source/tooling paths over WIP `651` to `wip/p02b-bounded-search-20261001` through the authenticated GitHub connector after shell Git authentication was unavailable. The connector verified each blob and the resulting exact tree, then used a non-force update and verified the remote ref plus the search processor blob. The evidence author independently streamed that immutable local tree and matched **all 8,739** build fingerprints, not only the 170 archived overlaps. The tree receipt and complete 62-path delta are embedded in the machine report. One published acceptance README contains a documentation-only runtime status update after the archive. Original worktree/index and main were preserved; the local WIP ref still names `651` until a later fetch, so do not confuse local ref state with verified remote publication. External durability covers included source; archives, frozen binaries and later TUI work remain cloud-local.

## Actual installed-native runtime acceptance

`p02b-as-search-independent-01/app-server-acceptance.json` passed under its unchanged outer subreaper, which exited 0 with no runner error. This reused independently built package03; **no new worker build is claimed for this App Server gate**. The package was installed/selected after the host was frozen, and host/manager/package/tooling hashes stayed unchanged throughout. The original package03 source and copied install input remained parked.

| Frozen artifact | SHA256 | Size |
| --- | --- | --- |
| `codex-app-server` | `a4ca81bb835f8b1883f5fb702982055ebf7161fffd49b80e81265a9fb2500a0a` | 501,502,024 bytes; nlink 1 |
| Existing component manager | `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a` | 6,575,272 bytes |
| Independently built native worker | `7d56e0dc370641d6fdb8bd2fb1cf81fb4db123a09cbb69315c85a396e4b17ddf` | 10,276,816 bytes |

Frozen host: `/workspace/component-checkpoint-candidate-p02b-app-server-20261001/codex-app-server`. Its original `candidate.json` says runtime was pending **at creation**; that historical manifest remains unchanged. The later completed runtime report supplies the acceptance result. Both hashes are included in the machine evidence.

Six management/configuration commands had expected statuses: install 0, select 0, list-selected 0, invalid selected configuration 1, remove 0, list-removed 0. Four full RPC server runs returned native baseline 0, installed parity/streams 0, installed resource failure 1, and removed/default-native restoration 0. The invalid-config command is an additional App Server startup rejection, not one of those four initialized RPC processes.

Real behavior exercised:

- Initialize with experimental API and exactly compare seven ordered native/installed/default-restored JSON cases: lexical-root/ignore handling, Unicode, beta, no match, empty query, 50-result cap and multiple roots. Scores, paths, roots, filenames, match kinds and highlight indices were not normalized away. The truncation fixture uses distinct path lengths to avoid parallel-insertion tie ambiguity.
- Observe the installed worker executable; run two sessions and concurrent updates. Require snapshot-before-completion, accepted empty-query cycles, alpha/beta/alpha identity fencing, explicit clearing and no-match completion. Stop one session, reject its later update, keep its sibling working and reject any notification after stop acknowledgement without a grace period.
- Leave a sibling active during normal stdin EOF; require the real App Server to close its selected worker and return 0.
- Configure the real worker's 64KiB snapshot bound and submit an allowed 64KiB whitespace query. Require actual `resourceExhausted`/`sessionFailed`, the current query and native snapshot-allocation cause, sibling survival, retained stop error and expected EOF exit 1. No uncertainty/consequence error was accepted in place of that cause.
- Reject unsupported selected configuration before protocol initialization without a native fallback. Remove the selection, retain immutable package bytes, and recover identical default-native behavior in the same unchanged executable.

Every recorded command/server run required all observed PIDs absent, with no timeout or emergency cleanup; 40 unique tracked PIDs were absent at their checks. The unchanged outer subreaper reaped adopted descendants. One explicitly recorded native-baseline Git helper, PID 524152 (`/usr/local/bin/git`), exited from **SIGPIPE**, return code -13/wait status 13. Its exit cause was not investigated here. Native baseline returned 0 and the final drain passed, but **not every child exited 0**; that orphan receipt is retained rather than dismissed.

This stdio acceptance has one connection with concurrent sessions. Separate public RPC tests cover WebSocket connection privacy. There were no model turns, new storage/GUI/Browser/TUI checks, launcher Ctrl+C, live provider or full-harness acceptance in this run.

## Remaining scope

App Server source is now bound to commit `30c267` above; publication of these evidence/lineage documents remains separate. Newer TUI work was applied afterward and preserved separately in `p01-source-20261001T074033Z.tar.gz` (SHA256 `fd9061224c5a79c3ecac3b5eb317c29f770f642f946bf983f21dee94261c948e`, 210 source paths); the first selected TUI gate ended exit 101 during linker SIGBUS before any test ran. Its 8,746-path report changed only Cargo.lock (ceed3359… to `660c481f`…); the other 8,745 matched. Parent observed zero free disk around the failure, which is supporting context rather than proof of an ENOSPC cause. That historical failed report/log remains preserved; subsequent separate TUI test results belong to the execution-state chronology and do not extend this App Server checkpoint. No TUI executable/runtime acceptance is claimed here.

Continue TUI integration, the private rollout-search consumer/broker, supported package upgrade/replacement gates, and existing storage/GUI regression against the new engine, including manager Launch Ctrl+C. The [StageA GUI/runtime proof](P02_SEARCH_PREREQUISITE_EVIDENCE.md) remains tied to its older frozen engine. In-app Browser and user-accessible cloud preview are unverified here.

Preserve native Apache LICENSE/NOTICE, Nucleo's MPL provenance, original worktrees and all earlier [selected standalone evidence](P02B_SELECTED_SEARCH_EVIDENCE.md). Upstream integration/update/rollback remains a required, planned maintenance component; no live updater or polling schedule was added.
