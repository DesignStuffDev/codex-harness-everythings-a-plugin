# Recovered current58 host: bounded runtime checkpoint

This records new actual production/runtime evidence on the original VM. It is a WIP
checkpoint, not a main promotion, release, complete lifecycle pass or new subsystem extraction.
The original checkout, sibling source, uncommitted work, real index and recovery archives were
preserved. Original executor access and disk-full build blockers are resolved.

## Identity and reproducibility

Upstream remains `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`; LICENSE/NOTICE remain.
The native process-final candidate is unchanged. Three SDK acceptance files add explicit
hash-pinned original-package reuse on a newly built host, preserving original-manager validation
by default. [Lineage](../../upstream/p03-recovered-search-reuse-lineage.json) records preimages,
new hashes, acceptance invariants and update hazards; this is not another native extraction.

The [complete tested source map](P03_RECOVERED_HOST_SOURCE_MAP.json) has8,921 entries in four
parts. Merging the parts and hashing compact sorted JSON gives
`15e9a44f54f414972eff02de9cd4afdfa5f900263d46b69a96119499779c136b`.
Scope: codex-rs, component-sdk, MODULE.bazel, MODULE.bazel.lock; excludes target, .git,
__pycache__ and symlinks. This inventories inputs, not proof that every file was compiled/tested.
The58-path manifest is `5e476a930d92284e0b827c7cbec247b97ba21976519a7ad31bba060747ac3094`.
Root documents/evidence are outside that frozen runtime source scope.

Production build02 completed0 in about75 seconds using locked CLI/manager targets. Before/after
source maps were identical; source receipt SHA256
`a1d4c107b8a83ffb0ce584d61bfafecc28afa8f6fc8dc9cf3777e307c068127f` and binary receipt
`3c1910323305f5851c828d7e01df15add26b7925d5d5350c6b6ce87ffc3acb7e` bind:

| Executable | SHA256 |
| --- | --- |
| codex | d716c6ee9c738c2db9197fbb2ab91d3918839d0b6d32f4556e8a7e1fa2717a92 |
| codex-component | f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954 |

[Sanitized receipts](P03_RECOVERED_HOST_RECEIPTS.json) include source/log/strict/report digests,
counts, raw adopted-status summaries and limits for17 runs. Private logs, auth/configuration,
readiness URLs, runtime homes and binaries are not published. Source hashes and before/after
binary checks bind current proof; legacy source-label strings are not authoritative revisions.

## What ran

All following successful runtime checks use actual rebuilt CLI/App Server/manager/components.
The inference service is deterministic. GUI checks use real Chromium through Playwright;
in-app Browser and live-provider inference were not exercised. No user-accessible viewer is
claimed. The earlier independent storage/search packages are reused with their original build,
source-removal and complete package inventory proof. This is not a fresh independent build.

| Gate | Result / scope |
| --- | --- |
| Search-package reuse helper | 11 unit checks passed. |
| Installed thread storage | 13 named behavior assertions: real turns, tool/context contribution, streaming, selected storage, cold resume and native restoration. |
| Manual migration | 10 CLI commands per ordinary/search/slow-normal/slow-forced setup, including dry-run parity, apply/reapply, cold CLI/App Server history and cancellation cleanup. |
| Ordinary GUI | Two cold cycles: stream, shell approval, Stop, continuation, search and actual manager SIGINT; manager0, about0.265–0.268s, tracked descendants absent. |
| Installed-search GUI | Two cold cycles through selected native search, same lifecycle checks, unchanged packages/host, about0.267s shutdown. |
| Slow selector04 | 8 unit checks passed; no runtime claim from these checks. |
| Synthetic wire04 | 12 wire-only cases passed; no installation/native-storage/browser claim. |
| Slow normal04 | Actual Launch SIGINT waits46.469s; manager0, native append/shutdown acknowledged, all directional results observed, tracked processes absent, canonical event recovered in cold GUI and continuation/shutdown pass. |
| Slow forced04 | Second SIGINT produces manager1 after2.041s and explicit durability-unknown; tracked processes absent, cold restart/continuation/native shutdown pass. |

The unchanged strict subreaper has SHA256
`fe01097ae1741cbcb76e15fb07c0c936108a4dc35c08b4264c1caa9eb1e08875`, original5s drain.
Successful commands have strict exit0/error-null. Storage/migration adopted SIGKILL statuses
remain unattributed in the receipts: strict success is not a claim every descendant exited0.
The GUI and both successful slow runs' recorded adopted return codes were zero.

## Slow boundary and retained failures

The installed test relay holds the lossless wire `EventMsg/ItemCompleted/UserMessage` append
before forwarding to native storage. Its46s clock begins at the manager's first interrupt
acknowledgment. This proves cleanup of an already in-flight component call beyond the previous
45-second launcher limit. It does not establish native-internal durability admission before
shutdown or exact onset of the App Server watchdog. Forced canonical UI absence does not imply
absence of all earlier raw model-history records; forced durability remains unknown.

Slow01 actually completed native append/cleanup after46s but failed cold GUI recovery: raw
ResponseItem user input persisted while cancellation preceded ItemCompleted(UserMessage).
That interruption/projection gap remains open under P07/P16 with P14 atomicity dependencies.
Slow03 used the saved-rollout JSON shape instead of the lossless component-wire shape; its gate
never matched and the run timed out before SIGINT. Its passing synthetic checks had shared that
wrong assumption. Neither failure is erased or counted as a pass after corrected04 success.

Held Git/HTTP attempts01/02 failed before complete readiness and required fixture cleanup.
01 watched /usr/bin/git while native trusted resolution selected /usr/local/bin/git;02 corrected
that identity and recorded an additional ChatGPT CONNECT. Source supports an independent featured
warmup, but CONNECT does not reveal the encrypted request path. Attempt03 preserves exact Git/API
readiness, single bounded auxiliary accounting, explicit peer EOF and original strict checks.
Its outcome is recorded separately; this paragraph is not a matrix pass.

Original compiler/SIGKILL/ENOSPC failures, historical source922 runtime and1,636 native suite
passes remain attached to their exact source cohorts. They are not current58 full regressions.

## Remaining gates and preservation

Held Git/HTTP native custody, same-process A→B replacement, exact MCP refresh/prewarm/transport
custody, TUI/changed integrations, scoped lint/format and recoverable repository/SHA publication
remain P03 work. Main stays unchanged. Native auth/catalog extraction follows these dependencies;
P04–P19 and required P18U real later-upstream integration/custom-plugin preservation/rollback
remain open. No periodic updater, live deployment or whole-host-clean claim is enabled here.

VM-local archives are recovery checkpoints, not externally durable backups. Included published
source/proposals/evidence gain external durability only after GitHub ref/blob readback. Runtime
binaries, private evidence and full archives remain inside the VM. The optional viewer does not
block component development. Follow EXECUTION_STATE.md for the exact next action.

## Held transport supplement


**Held-host03 terminal result:** four real Git-held cases passed all native ownership, independent
executable-observation and strict descendant checks (exec success/error; App Server EOF/SIGTERM).
The fifth Git-failure→HTTP-held case passed its local native/strict checks, but the independent
sampler missed the short-lived Git executable. Its parent gate failed; it is not counted passed.
Three remaining cases did not run. No forced fixture cleanup occurred in these five cases.
The matrix remains failed/incomplete, and MCP/session ownership remains separately unverified.
See [sanitized exact receipts](P03_RECOVERED_HELD_HOST03.json). A bounded
positive-observation handshake is being reviewed; do not bypass the independent observer check.
