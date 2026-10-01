# P02B TUI selected-search consumer — real installed-runtime evidence

Status: **verified partial native-search consumer milestone, published on main at `a469cf4be85fda40c64e563ece3e1639e4abdbac`**. Full CLI/TUI, reused storage, manual migration and selected-search GUI gates passed. Source tree is `3216dfb131f72939c986902053665cbad85ce610`, parent docs `b68db514687f6d4466b8d960b0e88010ddc179c9`. No whole-harness completion is claimed.

Main documentation checkpoint is `b68db514687f6d4466b8d960b0e88010ddc179c9`, tree `91597ccbdd327c7e60d80a3aa003a4a178073d7e`. The last verified source WIP is the separate App Server checkpoint `30c2674600cc84da161562c407515b61511b7c3a`, tree `86c0bce7a5b399c2337c7c3878ed12ee76da4918`. The later TUI implementation is now included in the published main checkpoint above; preserve the historical checkpoint identities. Connector publication reports and fresh remote checks are authoritative; intentionally stale local refs are not repaired by inventing another commit or resetting original work.

## Completed commands

These are separate scopes; their counts are not one combined test suite. The later full CLI build and installed-runtime gate are detailed below. Rust tests used `just test --retries 0`. All commands are finished, their logs match their recorded hashes, and the machine report retains exact commands, times and source transitions.

| Gate | Result | Scope and limits |
| --- | --- | --- |
| `p02b-tui-selected-tests` | Link failed, exit 101; **zero tests executed** | `ld` terminated with signal 7/SIGBUS. Cargo.lock changed across 8,746 fingerprints; all other 8,745 matched. |
| `p02b-tui-selected-tests-02` | **23/23 passed**, 5,601 not run by this selection | 10,112 fingerprints unchanged; subreaper 0, no runner error. This is a separate corrected attempt, not an automatic retry. |
| `p02b-tui-regression` | **5,620 passed**, four skipped | 10,112 fingerprints unchanged; subreaper 0, no runner error. Preserve adopted-child exit receipts; runner success does not mean every child exited 0. |
| `p02b-tui-fix` | Scoped lint/fix exit 0 | Exactly coordinator.rs changed among 10,117 scoped entries. This command did not execute tests. |
| `p02b-tui-format` | Formatter exit 0 | 17 changes among 10,118 entries: 13 TUI Rust paths and four Python acceptance files. No tests repeated solely for formatting. |
| `p02b-tui-bazel-lock` | Lock regeneration exit 0 | Three scoped paths unchanged. Not a Bazel build or test claim. |
| `p02b-desktop-unit` | **14/14 passed** | 77 fingerprints unchanged; subreaper 0. Gateway/package tests use an explicit fixture App Server, not real inference. |
| `p02b-desktop-controller` | **7/7 passed** | 77 fingerprints unchanged. Node controller tests inject RPC/scheduling; no Browser, DOM or native-engine interaction. No subreaper report for this command. |
| `p02b-terminal-observer` | **5/5 passed** | 15 fingerprints unchanged. Tests validate terminal-parser observations; they do not execute a real terminal application session. No subreaper report for this command. |

The first linker failure is preserved with its original report/log. Root observed zero free disk around that failure; this supports a resource hypothesis, **not a proven ENOSPC cause**. The subsequent successful attempt has its own expanded source scope. Fingerprint entry counts are not test counts or necessarily counts of product files; the machine report records scope additions separately from changes to comparable paths.

## Source transitions after the regression

The only Clippy source change collapsed a nested conditional in `file_search/coordinator.rs` into a short-circuit `&& let` condition. An independent read-only review reconstructed that single edit from its preserved preimage and reproduced the recorded post-fix hash. Root then added `#[cfg(test)]` above the test-only `FileSearchManager::accepts`; reconstructing only that annotation reproduced the recorded pre-format hash. The cancellation-plan document was also newly captured between these gates and is attributed separately from formatting.

The same review bound all 17 formatter preimages and postimages; the four Python acceptance files have identical ASTs before and after formatting. This establishes the recorded mechanical transitions, not post-format behavior. The scaffold retains 26 TUI consumer path bindings against the earlier frozen App Server scope, the actual passing TUI regression bytes and the post-format bytes. The completed full CLI build binds its own complete 10,118-path scope and binary tuple. Every post-format fingerprint matches that build's preimage; all 26 TUI path bindings also match the frozen build. No live checkout is silently substituted for that proof.

## Exact generated test preservation

Root preserved and revalidated three completed generated test executables using streaming gzip before retiring only their mutable build-cache paths. The report is `/workspace/recovery-backups/20260930T165936Z/p02b-completed-test-cache-preservation.json`. These are cloud-local recovery artifacts, not external backups or shipped runtime binaries.

| Generated test | Uncompressed SHA256 | Gzip SHA256 |
| --- | --- | --- |
| `codex-tui` | `12cb9292e500dc5baa31ac3cc4e5fbc4cdc9b7068636ed6a3e7283e510e8ccb2` | `33a444d124f64d443fda9c66c8b254c49ef8158c4dbb729e20bb824d7e63e557` |
| `codex-app-server-client` | `d11f9f3fcad20da53074ee473692be2c7387607e42f918ebb796186802c930fa` | `b9dd211823c4a8a731cc7766de9b9a76ff6a76593803f0072bddb6026d6b1322` |
| `codex-app-server-protocol` | `07679b71635e8a05b3cb321c03a670c6f28d375526386830a05c0b1ebe9debce` | `b7f69280551d368b9cc79e63cc673d913fc5211d8b3ae9e9bfc66307686c42c6` |

The preservation report records original inode/size/hash, live-reference audit, exact compressed and decompressed identities, and final cache-path revalidation. Source, evidence, libraries, depfiles, fingerprints and frozen runtime hosts were protected. This evidence preparer checked retained report consistency and archive existence/size without another decompression while the build was active. Earlier test reports identify crate tests and unchanged source but do **not** independently attest those exact executable hashes; preservation must not retroactively invent such attestation.

## Frozen full CLI build

`p02b-full-cli-build` completed `cargo build --locked -p codex-cli` with exit 0 and all **10,118** source fingerprints unchanged. The report SHA256 is `210f5249d759b5064938b65960a18cf0cbd03dd213acd58afc3b0f3c9e3e5d64`; its scope includes Codex Rust and component SDK source. This is separate build evidence, not a test count.

The frozen CLI is `/workspace/component-checkpoint-candidate-p02b-full-cli-20261001/codex`, **634,206,672 bytes**, SHA256 `7a63e9406d1605bac0a84e1b703735caeb211ceccf148337acf07614c7c0032d`. It was preserved by **exact relocation**, not copying or stripping: both original mutable Cargo hardlink names were accounted and removed, leaving the original inode/device with one frozen link. This preparer independently rehashed the binary, verified its size/nlink/original inode and checked both original aliases were absent. The unchanged manager, SHA256 `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a`, was copied separately.

`candidate.json`, `host-build-binding.json` and `relocation-preparation.json` bind these identities. The candidate's creation-time “runtime pending” status remains historical; the subsequent completed runtime report supplies the acceptance result. These binaries remain cloud-local artifacts, distinct from published source.

## Real installed TUI acceptance

`p02b-tui-installed-01` passed on its first recorded invocation. The outer source report has 78 unchanged fingerprints; its subreaper exited 0 without a runner error. Five manager commands—install, select, list, remove, list—each exited 0. Four actual full-CLI PTY cases ran once: native 0, installed 0, invalid selected configuration 1, restored-native 0. The audited tooling contains no retry branch; this statement does not infer unrecorded history.

The gate reused independently built package03, worker SHA256 `7d56e0dc370641d6fdb8bd2fb1cf81fb4db123a09cbb69315c85a396e4b17ddf`, **10,276,816 bytes**. No new worker compilation is claimed. Its earlier export/build provenance and parked source were checked. The full CLI and manager stayed unchanged across installation/removal; the original package and installed immutable object were also required unchanged.

Actual behavior observed:

- Type `@` filename queries in the real TUI, observe expected matches, reject ignored/wrong-root filenames, and clear the draft between queries. No user model turn was submitted.
- Stop the selected worker PID 578566/start ticks 8235363 with `SIGSTOP`, then submit a fresh beta query. The expected filename stays absent for 0.6 seconds while the worker is stopped. Resume with `SIGCONT`; the filename appears through the same worker identity.
- Execute real `/cd`, wait for its completion message, and observe a root-B match with root-A match absent. The same installed worker remains across that directory change.
- Reject unsupported selected configuration before interactive startup with exit 1 and the actual rejection/confirmed-cleanup cause. No native fallback is accepted. This is a startup failure gate, not mid-session failure proof.
- Exit each successful interactive case through real `/quit`, require exit 0, remove the selected component, and observe restored native behavior in the unchanged host.

All **25** tracked PIDs were absent at the cleanup checks. No emergency termination, timeout cleanup or unknown terminal mutation was accepted. The preparer verified 44 saved artifact hashes: 10 manager stdout/stderr artifacts plus 34 PTY transcript/screen/session artifacts. Raw environment values and transcript contents are not republished in this evidence.

The outer subreaper additionally records **six adopted Git helpers exiting 128**: PIDs 578450, 578445, 578581, 578576, 578731, 578725. Their causes remain unestablished. The command and subreaper returned 0 and drainage passed; **not every child exited 0**. These receipts are retained rather than dismissed as zombies.

The pause/resume and same-worker check supports dependence on the selected external worker; it does not inspect in-memory provider identity. This small fixture does not prove scale, stress, budget exhaustion or delayed stale-root callback handling. Shutdown here is `/quit`, not Ctrl+C. Remote reconnect/daemon/shared-socket, model turns, auth/network streaming, desktop GUI and in-app Browser are outside this run.

## Storage and manual migration on the new host

The existing independently built native thread-storage package 0.2.0/contract 2 was reused unchanged. No fresh Rust storage build is claimed. The nested storage gate passed 13 commands: two small Python fixture builds, seven manager commands and four actual CLI turns. It exercised tool execution/context contribution, deterministic model-event streaming, cold resume, three selected storage processes, and native restoration with retained history. Inner process checks establish termination; final descendant drainage is proven separately by the outer subreaper.

Manual migration01 and fresh02 each passed 10 commands plus two actual App Server lifetimes with six observed storage processes. Native/selected dry runs matched without changing the scoped storage snapshot; apply returned `migrated` and repeated apply `already_paginated`. Cold CLI/App Server recovery retained history; a blocked deterministic turn was interrupted and processes disappeared. Background migration/compression was disabled for these one-conversation fixtures. These are not attachment-specific, crash-durability or live-provider tests.

## Real selected-search GUI and launcher shutdown

GUI04 passed **49 and 38 Chromium/Playwright commands**, all successful, with zero page errors and both browsers closed. This is actual browser automation against the real new CLI/App Server, selected native search and selected storage; it is **not the requested in-app Browser**. Inference remains a deterministic fixture.

Both cycles exercised real filesystem search, click/keyboard reference insertion without premature submission, draft preservation and query/root/task/close fences. Response-order barriers delayed actual `route.fetch` responses without changing their bodies. Streaming, first-cycle approval/cancellation, persisted history and cold recovery were exercised. Original component objects/configuration and the independent installed search package remained unchanged.

During an active turn in each cycle, the **first SIGINT was sent only to the component manager**. It exited 0 in **0.266133s** and **0.268672s**; all 24 and 12 tracked manager descendants were absent, selected search workers were absent, browser children were drained, and final inner/outer subreapers passed. No forced cleanup counts as success. Observed-PID scanning can miss short-lived processes; the separate drain receipts remain explicit.

Screenshots `search-results-1.png` and `recovered-2.png` were reviewed by parent as safe; their hashes and cloud paths are recorded. Private readiness URLs, credentials, raw browser commands/results and environment contents are excluded from the machine evidence.

All three earlier GUI failures remain preserved:

- GUI01: zero GUI runs; strict independent-manager fingerprint differed only in file mode between original 0700 and unchanged copied 0555. Retry used the original manager; no chmod/assertion change.
- GUI02: zero GUI runs; fixture records included `path` alongside three fingerprint keys while manual records contained only the fingerprints. The reviewed correction preserves exact roles/fieldsets, isolated home/claim checks and fresh actual file hashes/size/mtime.
- GUI03: zero GUI runs after three successful manager setup commands. The helper incorrectly assumed install did not enable and List returned configuration. The correction checks the complete saved settings exactly, plus the exact three-field public List projection. This fixture was already exclusively claimed and selected, so it was retained; fresh manual02 created the fixture for GUI04.

These were acceptance-tool defects; no harness code, expected runtime behavior, fixture isolation or strict package/manager validation was weakened. Exact helper revisions 67a→ffff→022ba and both preserved preimages are recorded.

## Published source and tested bytes

Source publication promoted 215 paths over prior main, with 34 new blobs versus the preceding App Server tree. The publication report verifies 8,946 retained build-source entries exactly. Two explicit exceptions remain: acceptance helper 67a→022ba was separately captured and exercised by GUI04; `startup.rs` ed1cb→270bc only reformatted two calls after runtime. The raw 10,118-entry build/final scopes have 10,116 equal entries plus those two exceptions; this is a different denominator from the filtered published-source intersection. Do not claim full byte equality with the built executable.

The final formatter passed without tests repeated solely for formatting. Post-format archive `p01-source-20261001T084942Z.tar.gz`, SHA256 `acf2258611dec4e7f35b4bd1a27bec4dbaea1d8d2031c97fbf26eb215eed94d5`, independently verifies all 223 listed members. Published source archive SHA256 is `d570aed0ae603bba00c5f3a95a4b2109954577d2d44adb26e1051c461e6067ad`. Archives/binaries remain cloud-local; the GitHub source checkpoint is externally durable.

[Lineage](../../upstream/p02b-tui-consumer-lineage.json) maps 26 TUI paths, selected original/current symbols and helper/format transitions. It distinguishes existing native search replacement from compiled TUI ownership and additive GUI presentation. The [machine report](p02b-tui-consumer-results.json) preserves exact separate gates, hashes, failures, relocation and whitelisted process actions.

Preparing-request cancellation, further package/replacement and reconnect/daemon gates, private rollout lookup, remaining subsystem extraction and P18U upstream integration/rollback remain unfinished. User-accessible cloud viewer and actual in-app Browser verification are still separate limitations. This milestone does not complete the platform.

Published screenshot copies: [selected file search](p02b-desktop-search-results.png) and
[recovered session](p02b-desktop-recovered-session.png). These are still images, not a reachable live preview.
