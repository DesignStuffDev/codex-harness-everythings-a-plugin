# Current-host storage shutdown and held-task runtime evidence

All six continuation commands returned zero on the production CLI built from the same 8,947-file source map. This is a bounded runtime checkpoint, not completion of P03 or whole-harness extraction. The earlier selected-search GUI failure remains preserved and unresolved by this evidence.

## Exact tested identity

- Source map (sorted compact JSON SHA256): `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`.
- Published source checkpoint: `d229296b29947ef1328cbe931c810276f220a23c`.
- Actual CLI SHA256: `78d9194cd4329e9b83b75fe07389d524d8b1f425353d71df9a68607be2945e83`.
- Actual component-manager SHA256: `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
- Successful build02/source/postbuild receipts remain separate from these runtime results. Every continuation source receipt independently has identical before/after maps and return code zero. The local main ref printed inside historical wrappers is explicitly not the tested-source label.

## Real runtime results

| Gate | Result and boundary |
| --- | --- |
| Migration for normal-shutdown fixture | Ten manager/CLI commands passed, including independent native-package installation/selection, native-versus-selected dry-run parity, migration, repeated apply and CLI resume. Real App Server history pagination and post-migration cancellation were also exercised. |
| Normal Launch shutdown | A real installed relay held `ItemCompleted(UserMessage)` before forwarding to native storage. First SIGINT was acknowledged after 25.467ms. The write stayed held for 46.026778s after acknowledgment, exceeding the former 45-second deadline. Manager exit0 occurred 46.465796s after first SIGINT, after forwarding, native append acknowledgment and native cleanup. Cold native-selection GUI recovery displayed the held prompt and completed a new turn. |
| Migration for forced-shutdown fixture | The same ten-command migration sequence passed in a separate retained fixture home. |
| Forced Launch shutdown | A second SIGINT after a two-second acknowledged hold produced manager exit1 at 2.039273s after the first SIGINT and an explicit durability-unknown warning. No forwarding receipt existed. Cold GUI recovery omitted that canonical history item and completed a new turn. Canonical UI-history absence does not establish raw-input absence or a general durability guarantee. |
| Eight held production-host cases | Git and fallback paths each exercised exec success, exec failure, App Server stdin EOF and App Server SIGTERM. Actual nested strict reports passed; there is no outer strict wrapper for this matrix. See the separate held review for exact ownership gates and limits. |
| Featured-task typed postcheck | All eight cases passed the separate typed receipt postcheck. Each recorded two clean process snapshots and one Joined/Some(Cancelled) scope receipt. Zero joined-task process snapshots are not lifetime counts or independent proof that work started. New native HTTP-constructor events were not collected by this fixture. |

The two slow modes each recorded four held-browser interactions and seven cold-recovery interactions, with zero page errors. Cold manager shutdowns returned zero in 0.424295s and 0.466606s respectively. All tracked held/cold process identities were absent according to the fixture; the unchanged strict subreaper results are retained separately because sampling can miss short-lived children.

## Meaningful limitations

These are actual component-manager, App Server, native storage, installed GUI and Chromium processes using deterministic model/transport fixtures. They are not live-provider tests. Browser interaction used Chromium/Playwright fallback; the required in-app Browser was unavailable. The screenshots were subsequently visually reviewed and corroborate the connected, completed GUI state; they do not independently prove persistence.

The slow-storage gate proves ownership across an installed fixture's pre-forwarding boundary. It does not prove native internal admission before release and does not fix the separate response-only interrupted-history gap. Forced termination continues to report durability uncertainty. Held-task and featured receipts do not establish pooled HTTP/internal constructor cleanup, complete MCP custody or whole-host cleanliness. `HELD_REVIEW.json` independently rechecks all eight nested case/strict/observer/receipt/ack hashes and recomputes the featured postchecker result without rerunning the runtime. The held fixture has 89 static `require(...)` call sites; no executed-assertion counter exists, so this is not reported as 89 runtime assertions per case.

Strict reaped return-code histograms include the direct command. Migration wrappers retained SIGKILL-derived -9 statuses (four and five); Git held cases retained two -9 statuses each, and fallback held cases one128 each. They are preserved without reassignment or dismissal. A passing strict runner does not make every descendant exit successful.

No new OOM or OOM-kill counter was observed in the six step-boundary snapshots (counters remained9/4). These snapshots do not measure transient memory peaks. The continuation used fresh `/dev/shm` paths on the same VM, preserving earlier `/tmp` runtime homes and the failed GUI-search run; this tmpfs still consumes cgroup RAM.

## Artifacts

`GATES.json` contains source/strict bindings and step-boundary resource observations; `MIGRATION_AND_SLOW.json` contains public-safe selected runtime facts. `LOCAL_REFERENCES.json` hashes the original local reports without embedding private command/config/log payloads. `HELD_REVIEW.json` contains exact per-case receipt/ack/observer results and typed limits. `FEATURED_TYPED_RESULT.json` preserves the compact typed postcheck result. The two PNGs show normal and forced cold recovery. Original reports and backups remain VM-local until separately published or exported; this bundle alone is not outside-VM durability.
