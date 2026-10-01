# P03 declaration manager runtime evidence

**Passed:** the newly built component manager completed eight actual commands: six successful install/select/list/remove operations and two expected declaration rejections (exit 1). Both required and optional host-service declarations failed with the inactive-catalog diagnostic. Existing catalog/config/package files and the selected state remained unchanged after each rejection. Plain-package removal cleared activation/selection and retained the immutable package object.

The manager is `/workspace/component-checkpoint-candidate-p03-declarations-20261001/codex-component`, 6,746,832 bytes, SHA256 `e34edd90256bafb3b3aec94360a4de2ee885e0082085df7b91d33449aea748fb`. Its locked build exited 0 with all 8,864 recorded source hashes unchanged. The runtime used the same formatted source map. Three declaration files differ mechanically from the earlier 177-pass/1-skipped test bytes; the prior formatting review and both exact snapshots remain preserved.

| Command | Exit |
| --- | ---: |
| Plain install | 0 |
| Select file-search implementation | 0 |
| List selected catalog | 0 |
| Install with required services | 1, expected rejection |
| Install with optional services | 1, expected rejection |
| List after rejection | 0 |
| Remove plain package | 0 |
| List after removal | 0 |

The package reused the independently built worker04 unchanged (worker SHA256 `a3f73fed15dd7b19ab0f1daed55a74212d4b2f5d2ba5cdebee2a616d2f2419a1`). This gate installed and inspected the package; it did not run the worker or rebuild it. The manager, package, receipt, source report, script and independent build proof retained their recorded fingerprints. The script hash exactly matches the prior reviewed copy; the unchanged acceptance helper and subreaper are included in the frozen runtime source scope.

All eight observed manager PIDs were absent after their commands. There were no timeouts or emergency cleanup actions. The outer strict subreaper exited 0 with `runner_error: null`, recording the acceptance process exit 0. The source recorder also exited 0 with unchanged before/after hashes. Exact commands, source bindings, artifact/log hashes and process observations are in `p03-service-declarations-manager-results.json`.

This proves actual installation/catalog compatibility and fail-closed rejection. It does **not** activate a broker, execute granted host services, extract another native subsystem or validate the full GUI on the new manager. Publication binding is pending. Prior test failures and formatting evidence are preserved separately.

The next queue is the separate broker-budget validation slice, then versioned offer/acknowledgement DTOs with canonical exact-grant comparison, followed by the accepted retained lifecycle/accounting and guarded activation slices. The broker remains inactive until those coordinated gates pass.
