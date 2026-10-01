# Worker04 compatibility with the frozen full Codex host

Worker04 was built separately, installed through the component manager, and exercised through the real App Server mode of the **previously frozen** full Codex CLI. The acceptance run passed on 2026-10-01, 10:25:07–10:25:16 UTC, under the strict subreaper runner. This is backward runtime compatibility evidence, not proof that the newly changed host or its Preparing cancellation path has run.

## Exact artifacts

| Artifact | SHA-256 |
|---|---|
| Frozen full CLI host | `7a63e9406d1605bac0a84e1b703735caeb211ceccf148337acf07614c7c0032d` |
| Component manager | `eb969e83bcff6ebf531907870efa9e071c939712eb48ec4f0ee13a2cab7c048a` |
| Independent/installed worker04 | `a3f73fed15dd7b19ab0f1daed55a74212d4b2f5d2ba5cdebee2a616d2f2419a1` |
| Acceptance report | `b1a415f796ccefd7cab257326c4b00da3a130bd49a72c5fb4aee49b3035e70d6` |
| Outer subreaper report | `e6ac01029ad156bcc5a50251a8a3f3aac2d1b9801d8d322498632a135340920d` |

The package declares version 0.2.0, wire contract 1 and `preparing_cancel_receipt: 1`. That declaration is not a runtime demonstration of constructor cancellation. Worker04 used a fresh external target with 10 local crates and 102 resolved packages; host/core/runtime/TUI crates were excluded. The source and install input were parked before runtime execution. All independently built and installed package files remained unchanged, as did the host, manager, tooling, build reports and subreaper script.

The exported source inventory contains 196 entries. The audit matched 195 parked files directly; `Cargo.lock` matches the separately recorded resolved/normalized final lock `dbe166dc08aa0c038fd4e7d9c5625e3f0a66af08b0c9e61c014b1f1192c6c3ac`. The lineage preserves this override instead of describing the initial export inventory as the exact final compiled tree.

## Behavior actually exercised

- Native baseline, selected installed worker, and native restoration after removal returned identical ordered JSON for seven cases: lexical alpha, Unicode, beta, no match, empty query, the 50-result cap and multiple roots. Comparisons include root/path/name/type, scores and highlight indices; gitignore behavior is checked.
- Installed and restored runs exercised two concurrent sessions, concurrent updates, query A→B→A, empty-query clearing, no-match completion, joined stop with a still-working sibling, and no notifications after the stop acknowledgment. One sibling remained active until normal stdin-EOF shutdown.
- A real native snapshot cap of 65,536 bytes with a legal 65,536-byte query produced one bounded `resourceExhausted` failure. The sibling stayed usable; stop and EOF preserved the real allocation error. This App Server process deliberately exited 1 for that retained operation error, while cleanup passed separately.
- Invalid selected plugin configuration exited 1 before initialize, with empty protocol output and no native fallback. Removal cleared installation/enabling/selection and restored native parity/streaming; the removed worker was not observed during restoration.

The two installed worker PIDs were 609100 and 609261. Across four RPC processes and six manager/startup commands, **40 unique observed PIDs** were absent after their checks, including zombies; the audit found all 40 still absent. No emergency cleanup or retry was used to turn a failure into a pass. Normal RPC exits were 0 except the deliberate retained-resource-error exit1.

The outer subreaper recorded eight waits: seven adopted descendants plus the acceptance harness. Six adopted descendants and the harness exited0. Adopted PID609002, recorded as a Git helper, exited by SIGPIPE (`-13`, raw wait status13). All eight reaped PIDs were absent. This evidence does **not** claim every descendant exited0.

## Limits and references

No new host binary, delayed native constructor/Preparing cancellation race, TUI, GUI, Browser interaction, model turn, or multiple-client connection was tested here. Whole-harness extraction is not complete. These cloud-local proof files and parked sources do not become external backups merely by being hash-checked.

- [Machine-readable results](p02b-worker04-old-host-compat-results.json)
- [Artifact/source lineage](../../upstream/p02b-worker04-old-host-compat-lineage.json)
- Raw acceptance: `/workspace/acceptance/p02b-worker04-old-host-app-server/app-server-acceptance.json`
- Raw subreaper result: `/workspace/acceptance/p02b-worker04-old-host-app-server.subreaper.json`

Audit verification rehashed the reports, raw logs, tooling, complete package inventories and parked source files. Host/manager before-and-after hashes come from the unchanged acceptance report; the audit rechecked their current metadata instead of rereading the 634MB host while root's Rust gate was active. No runtime/build/Git command was rerun for this evidence audit.
