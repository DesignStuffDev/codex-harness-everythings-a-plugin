# Current final65 production/runtime checkpoint

This report preserves the checks preceding the all-mode readiness correction. The
subsequent complete eight-case result is recorded separately in
[the matrix supplement](P03_CURRENT_HELD_MATRIX_EVIDENCE.md); both failed runs below remain failed.

This root-reviewed checkpoint records current production/runtime evidence. It makes no new native code adoption or completed extraction claim. All referenced current checks used the unchanged 8,928-file source map `1c54717dd95ff85361d33076863129435f9516d3f815ff8690263fdda083549a`, selected65 manifest `bfad182c…`, published code checkpoint `7adb464` / tree `4e84d272…`. Exact receipt, log and artifact hashes are in P03_CURRENT_PRODUCTION_RUNTIME_EVIDENCE.json and P03_CURRENT_PRODUCTION_RUNTIME_ARTIFACTS.json. Private reports, configuration, auth, process identities and session identifiers are not copied.

## Build and installed components

The production build of `codex-cli` and `codex-component-host` finished successfully in 6m22s with source unchanged. CLI SHA-256 is `dc7f1e84d5bcb311fdc7751b1538319f6205a892203edfc74c2993103f18f158` (635,421,088 bytes). The manager retains cached identical bytes, SHA-256 `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954` (7,274,296 bytes). The build receipt's original runtime-gates=false field is a point-in-time build-only statement, preserved rather than rewritten.

| Current runtime gate | Confirmed result |
|---|---|
| Installed native storage reuse | Passed real CLI turns, tool execution/streaming, cold resume, selected storage termination and native restoration with retained history. Original independently built package 0.2.0 / contract 2 was reused; no new Rust plugin build occurred. |
| Four fresh migrations | Normal GUI, selected-search GUI, slow-normal and slow-forced fixture preparations each passed dry-run parity/unchanged bytes, selected apply/idempotence, cold CLI/App Server history and cancellation/reaping. |
| Ordinary GUI, native search | Two real Chromium/Playwright cycles passed recovery, streaming, file search and manager Launch/first Ctrl+C shutdown. Approval and Stop/cancellation were exercised in cycle 1 only. Manager exited 0 both cycles. |
| GUI with independently installed search | Two cycles passed recovery/streaming/search/shutdown against selected `native.file-search-local`; approval and Stop/cancellation were cycle 1 only. Reused original independent package/build proof `32ff6aba…`; current host/CLI stayed unchanged, original components were preserved, and the install input was parked. Worker hash `a3f73fed…` matches installed bytes. |
| Installed slow storage, normal Launch shutdown | Passed the existing 46-second hold: manager exited 0 after 46.317012759s; native append/shutdown and directional observations completed; tracked processes disappeared and cold native GUI recovered the held canonical user-history event. |
| Installed slow storage, forced Launch shutdown | Passed the expected uncertainty case: manager exited 1 after 2.041877452s with explicit unknown durability; tracked processes disappeared, held event was not cold-recovered, and subsequent continuation/normal shutdown passed. |

Every gate above has source return0/unchanged and an outer strict subreaper receipt with command0/exit0/error null. Exact adopted return-code histograms are retained, not flattened into an all-zero claim. Slow normal and forced each recorded six adopted statuses 0. Each ordinary/selected GUI cycle separately reaped two browser orphans; tracked-process scans can miss short-lived descendants, so this is not complete whole-host custody proof. Selected-search workers were also checked absent.

The storage outer receipt accurately classifies this checkpoint as `reused_package_new_host`, `new_independent_build=false`, retaining original independent-build proof `ab41bc4a…` and unchanged package hashes. Its inner legacy runtime report has old “independent build” wording; that does not mean a new build occurred here. The same separation applies to the selected-search package. No historical current58 runtime result is relabelled as current-final65 proof.

Both GUIs and slow-storage checks explicitly use **real Chromium/Playwright with deterministic inference fixtures**, not the in-app Browser or live model-provider testing. GUI native/selected search, approvals, cancellation, persistence and manager shutdown are real fixture-driven runtime paths. These receipts alone do not verify live-provider auth, remote viewer connectivity, every descendant or full native-component ownership.

## Held-host matrices remain failed

| Run | Result and exact remaining boundary |
|---|---|
| Original current-final65 eight-case fixture | First two CLI cases accepted. Third Git/App Server EOF case had inner `fixture_protocol_failed` with proxy error `auxiliary_request_after_stop_trigger`, then outer `strict_descendant_gate_failed`; five cases unrun. Host0 after0.032503985s and scoped curated joins do not erase the fixture failure. Rejecting the auxiliary CONNECT closed its socket; this is not native auxiliary peer-EOF proof. |
| Full matrix with diagnostics-only fixture `281173ec…` | First two CLI cases accepted. Third EOF inner case passed, but outer validation failed `frozen_observer_real_git_missing`; five cases unrun. The independent observer was otherwise complete and strict was0/null. Neither inner success nor strict success substitutes for the missing exact independent Git executable proof. |

The first two CLI strict receipts in each matrix recorded adopted return codes `{-9:2,0:1}`; the original failed EOF recorded `{-9:2,1:1}`. The diagnostic EOF recorded `{-9:2,0:1}`. These are reported without newly attributing the -9 statuses. Neither of these two matrices passed. The original outer `strict_descendant_gate_failed` code follows the failed inner fixture returning1; runner_error stayed null. It is not evidence by itself of a runner error or pending descendants.

The diagnostic EOF outer observer saw the same process generation the inner Git sampler reported, but its executable observation was unavailable and the process subsequently disappeared. It retained Python/CLI executable hashes, not the required exact Git hash. The recorded state was S; no zombie or different-binary explanation is established. The observer collapses multiple /proc open/recheck failures to unavailable, so the exact cause is unknown. Git mode currently uses the inner sampler gate; independent acknowledgment is configured only for fallback. A separate all-mode acknowledgment proposal is being prepared; it is not adopted or counted as runtime evidence here.

## Successful single diagnostic and its limits

The separate single-EOF diagnostic completed with source0/unchanged, strict0/null, original case pass, host0 in0.031684908s, no fixture rescue or cleanup errors, EOF on both output pipes and bounded diagnostic records. Independent review applied the matrix's outer checks despite using the internal single-case entrypoint: exact stable trusted-Git executable, matching generation, complete observer/no failures, current binary/config/fixture/source bindings and all original case assertions were verified. Its adopted return codes were `{-9:2,0:1}`.

In that successful single run the auxiliary connection was already accepted and parsed about 11.5 ms **before** the external trigger, then reached peer EOF16.830870ms afterward. It therefore does not reproduce or close the original late-auxiliary startup race. Its valid exact-Git proof stands independently of the later full matrix's missing-Git observation; no cases are stitched together. The diagnostics-only pure suite passed 23 checks with source unchanged and strict0/null. Those are fixture checks, not extra native runtime acceptance.

## Scope retained

No new native subsystem was extracted by this verification work. Coverage remains three bounded native component families; P03 and the whole-harness transformation remain open. Curated worker/callback receipts do not prove detached featured-plugin warmup ownership, MCP transport/handle custody or whole-host cleanup. The slow-storage selector observes an accepted canonical UI-history event before forwarding to native storage; it does not prove native admission before release or fix the separate response-only interrupted-history projection gap. Remaining roadmap extraction and real upstream-update/rollback acceptance remain required.
