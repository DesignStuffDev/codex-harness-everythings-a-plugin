# Curated production replacement acceptance — frozen proposal 01

Status: recovery-only source proposal. Not adopted, compiled, formatted by Rust tools, or executed. No primary checkout, Git, Rust, cache or runtime mutation was performed. Python was parsed through `ast.parse` without import/execution. Static review is not a passing acceptance result.

## Dependencies and review units

Apply after the current process-final/replacement candidate `1da290682fc6ddd00994c2af7eac9018c8143dea` (base manifest `0a46a9a3f50670a4cc10180012dd3adb09ee2226c75efb8db8c86fbb093d4065`), then native observation proposal01 manifest `09f621f1a67105a6794894f441322f92bc2af53348e89ce55be6c32269c42f4d`, then callback activity proposal01 manifest `340033c388567d294cb471ecb741e62da0cb1f14e3f508ba4134f5afec53fa10`. Each observer remains a separate review/adoption unit. They add readonly production diagnostics, not test-only stop/harvest hooks. Their snapshots neither initialize, close, cancel, move/join handles nor dispatch callbacks.

1. `01-native-child.patch`: 554 changed lines, one ignored Linux library child plus four-line sibling wiring. It exercises real `in_process::start` twice in one fresh process; normal test runs do not launch this fixture accidentally.
2. `02-sdk-parent.patch`: 561 changed lines, Python parent and focused SDK usage/evidence documentation. It owns three fresh child processes, the real trusted Git smart-HTTP service, model/MCP fixtures, protocol barriers, finite failure cleanup and retained evidence.

The two acceptance units form one runtime gate and may be reviewed/published separately, but neither alone earns a host replacement claim. No Cargo, Bazel, dependency, lock, schema, sandbox-policy, source-build or existing test timeout change is included. Current preimage bytes match the checkout at freeze; future postfmt changes require an explicit mechanical rebase, not silent fuzzy patching. Pure byte-derived Git blob IDs and read-only API/source bindings are recorded.

## Assertions and limitations

Pending proves an already-active B thread/turn moves from old to new typed plugin, skill and MCP behavior, and runs the refreshed Interrupt hook once. Replay observes exact generation + immutable success + attached handle finished before B starts; it deliberately remains Running until actual final harvest. Race releases native completion concurrently with B construction and accepts either legal ordering, demanding one delivered action. Final local observations demand A=0 and B=1 completed callbacks; global remaining registered scopes are zero after clean scopes retire. Final curated completion requires exact generation, Succeeded and Joined, with no quarantine/unexpected handles.

The parent waits for `behavior_verified` before releasing the held model and before final joins; `complete` follows only after the child writes its successful operation/cleanup receipt. New MCP Ready precedes any post-release tool request that could drive dirty refresh itself. Existing MCP prewarm/transport custody is explicitly unverified: all reports preserve `whole_host_clean: false`. Optional transport observation is not arbitrary process-group completeness. The unchanged strict runner's original drain and all raw adopted-child statuses remain required independent evidence.

This does not cover different-home UnsupportedHome, failed-generation retry resubscription, HTTPS TLS/body shutdown, durable publication, store accepted-write draining or pending C2b/MCP proposals. A fixture setup rejection cannot be bypassed by fake Git, PATH replacement, relaxed CA/proxy/DNS/allowlist, or disabled native startup. Final cleanup force/uncertainty is always failure. Original errors and outputs remain evidence.

## Validation and license

Independent static review checked trusted executable lookup, API fields, real skill namespace, callback-count retirement semantics, experimental request admission and phase ordering. The earlier unbounded backend pipe-drain concern was corrected with finite post-kill waits and explicit unconfirmed failure. Python AST parsing passed. Both patches replay with zero fuzz/no offsets in this recovery directory and every proposed output matches its recorded hash. No Rust compile/lint/fmt/tests, Git fixture, host, MCP, hook, network or acceptance execution occurred.

Repository Apache-2.0 LICENSE and NOTICE are preserved verbatim. All new implementation is authored for this proposal; no new external dependency implementation/license is imported. The original immutable acceptance plan is referenced at `/workspace/recovery-backups/20260930T165936Z/p03-curated-replacement-production-acceptance-plan/PLAN.md` and is not replaced. New proposals remain outside the earlier frozen 204-file preservation allowlist; root controls any later source-only WIP preservation.
