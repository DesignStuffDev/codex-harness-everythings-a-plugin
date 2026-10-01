# Native Preparing owner verification

This checkpoint adds retained cancellation for one native constructor, private startup signals, and guards for lost startup/close result observers. It preserves the original native matching implementation and legacy error wrapper. It is a partial lifecycle milestone, not another whole subsystem extraction or independently installed runtime proof.

Source is published on `wip/p02b-preparing-cancellation-20261001` at [`a79072a3d592e20e08a8f882a4dabc716013f375`](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin/commit/a79072a3d592e20e08a8f882a4dabc716013f375), tree `ffc1806f2fa7d2b37b5536af30ac5bde8b52b02a`. The preceding production-only commit is `ed000f05f9ab8a6fad6b201769f7cae0b639ed6c`; the final WIP commit adds the test module and registration. Root's publication report records the matching remote head. This evidence step made no Git/GitHub mutations.

## Actual checks

| Gate | Result | Scope |
| --- | --- | --- |
| `just test --retries 0 -p codex-file-search`, unchanged subreaper wrapper | 88 passed, 0 failed/skipped; no retries | 9 new native owner cases plus 79 unchanged existing cases; all 10,127 captured source hashes unchanged during execution |
| First `just fix -p codex-file-search` | Exit 0, one warning retained | One mechanical `finish` async-function rewrite; test MutexGuard scope warning |
| Second scoped fix | Exit 0, no warnings/source changes | After replacing explicit guard drop with a lexical test block; assertions retained |
| `just fmt` | Exit 0 | Five native paths formatted; tests were not rerun solely for lint/format |

The test wrapper reports command exit 0, runner_error null and one reaped command PID with exit 0. No descendant was ignored or relabelled. The report's historical `baseline=f2cc6c2…` field is not the tested commit: actual inputs are captured source fingerprints. The JSON preserves tested and published file hashes separately.

## What the real native tests establish

- Dropping an unpolled ticket or finish observer requests cancellation even with a surviving control clone. Capacity remains held while accepted constructor/result work is queued; only the actual NotAdmitted receipt allows reuse, followed by a real matching query.
- Cancellation reaches a real constructed Rayon pool, waits for it to join, and leaves a sibling serving queries. Closure never changes a sibling's shared external cancellation flag.
- Real constructor budget failure survives cancellation during callback destruction; a destructor panic retains the original ResourceExhausted operation separately from Unconfirmed cleanup and quarantines capacity.
- Real native snapshot-budget failure is preserved when cancellation races a ready native result before public observation. Startup and cancellation receipts retain the same failure and Confirmed cleanup.
- Destroying a runtime before its startup or close result observer first polls produces retained Unconfirmed receipts and no capacity refund. Repeated observers receive the same receipt.

Independent review caught and corrected helper shadowing, bounded Released-cause masking, and abandonment tests that had cancelled explicitly before Drop. These were source-review corrections before execution, not failed test runs. The executed 88-case gate passed on its first recorded run.

## Source and provenance

The five paths are `file-search/src/{async_owner.rs,async_owner_start.rs,async_owner_start_tests.rs,native_session.rs,lifecycle.rs}` under `codex-rs`. The new startup module uses the already published neutral SearchStartControl primitives. Upstream remains OpenAI Codex `d42056091aded7feb1d88ac7e83972108b2aa478`; existing Apache LICENSE/NOTICE are unchanged. The lineage artifact maps original native search symbols to these intentional lifecycle customizations rather than claiming these custom paths existed upstream.

The checksum-verified source archive is `p02b-preparing-native-checkpoint.tar.gz`, SHA-256 `d68420b078d600d06b9c64e9c95890ef40ef7c7fa7d1c02ef1f83b0343a25673`. Every archived source member matches the published five-file binding and final formatting report. That archive is cloud-local; the GitHub WIP commit provides external durability for its included source only.

## Limits and next gates

NativeSearchBackend begin_open is a separate staged change at this checkpoint. Process/worker, service/runtime, full-host and UI Preparing cancellation remain unverified by this gate. There was no new external plugin build/install, UI or Browser run, storage migration, live-provider turn or updater trial. Prior published CLI/App Server/TUI/GUI evidence remains separate.

Cancellation remains cooperative while native calls or user callbacks execute. Unknown cleanup retains reservations rather than implying joined work. The next gate is the native backend ticket bridge, followed by process/service ownership and real installed host/UI cancellation; this checkpoint does not complete whole-harness compartmentalization.

Machine evidence: `p02b-preparing-native-owner-results.json`. Source mapping: `p02b-preparing-native-owner-lineage.json`. Both contain exact report/file identities without copying full source manifests or private runtime material.
