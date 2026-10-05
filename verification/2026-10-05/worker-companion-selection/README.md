# Static worker companion selection — 2026-10-05

This closes the bounded **static selection** requested after `e8470e0`.
It does not make the packet runnable or establish worker admission. The
[selection](SELECTION.json) records exact retained origins, SHA-256 values,
published Git blobs, relative layout/import relationships and custody boundaries.
[Bootstrap controls](../completion-handoff/BOOTSTRAP_CONTROLS.md) and the
[completion handoff](../../../COMPLETION_HANDOFF.md) remain authoritative for
execution. No product source or acceptance expectation changed.

The original 32-payload / 1,550,203-byte packet remains sealed and private. Its
existing content review was reused, not repeated. Comparing exact Git blobs with
the cached complete `c0412a00d90cca162e7a54b5c6aa06cb67979f55` source tree corrects
an earlier classification: **12 of its 14 runtime helpers already exist byte for
byte in published source**. Private custody did not mean unpublished code. The
two without an exact pinned-tree match are `token_install_consumer.py` and
`run-process-final-check.py`. Across all 32 payloads, 20 lack such a match: the
eight candidate source files, those two helpers, five bootstrap controls, four
command references and the portable source map. None is exported by this change.

The merged inventory contains **89 origin/hash records, 84 distinct hashes**:
61 records have exact published blobs; 28 remain private without a match in that
pinned tree. Twenty-one records were outside the original packet and existing
44-helper/22-control recipe. These include notices and references, not 21 newly
required executable files. Ten source-level wire references remain separate
semantic references. These counts are static inventory, not tests or dependency
acquisition results. Shared bytes do not collapse distinct import roots.

## Exact companion relationships

| Consumer | Selected retained relationship | Fresh work still required |
|---|---|---|
| Attachment acceptance | Adjacent proposal02 `INPUTS.json` (`46205fa8…`) and `MANIFEST.json` (`029b0b0a…`); exact copies are in the published recovered-gate-proposals directory. The separately retained source42 `INPUTS.json` is `63c6aab2…`. | Both inputs are historical. Derive new adjacent metadata from actual worker source, CLI/manager, tracing tool and independently built package proofs. Never rename old results into current evidence. |
| Diagnostic GUI driver | Adjacent `gui_search_observer.cjs`, SHA-256 `ffa67eb1abb8ac52ba9255b535d95cea1cb622351578ef20ab27c8603fb04190`, from the published diagnostic-fixture/proposed directory. | Bind actual browser, source, package and producer identities. Observer finish markers alone do not prove pending collectors completed. |
| Ordinary GUI / slow storage | Preserve ordinary SDK `p01_gui_acceptance.py` and its driver import root, plus their selected SDK/fixture helpers and complete desktop/model/calculator source groups. | Do not substitute the diagnostic override globally. Create isolated migration fixtures and new private outputs for each gate. |
| Held host | Adjacent `featured_owner_receipts.py`; explicit `--receipt-schema` reference to `RECEIPT_BINDINGS.json` (`66998234…`); separately selected historical postbuild-binding template. | All MCP and both exec session-loop observations remain null in that declaration. Hashing a source-binding file does not attest its producer or labels; review current-source provenance separately. |
| Native observer / auth consumer | Retained fixed plan, source-envelope shape, command inventory, admission documentation, source wrapper, RPC/support helpers and model catalog. | Worker-specific paths/index/envelope, fresh admission and producer proofs remain uncreated. Require matching `complete_success` plus child/strict/source results; apply the one-job override after sourcing the preserved environment scripts. |
| Independent packages | Published storage/search/attachment exporters, search build worker and adjacent lock normalizer are explicitly selected. | New disjoint exports/targets, original and resolved locks, exact packages and source-absence/install proofs. Preserve the thread source export before its helper removes it. Freeze the host for replacement acceptance. |

Full hashes, origins, exact published paths and multiple consumer layouts are in
the selection; shortened hashes above are navigation aids only. Historical
metadata and source code are immutable references, not a worker-specific control
bundle. Dynamic exporter closure still requires complete pinned source and fresh
package proofs. No speculative worker destination was assigned.

## Delivery and notices

Published bytes can be obtained through the existing authorized source channel
at the pinned revision, verified, and placed into the recorded reference layouts.
This checkpoint publishes coordination metadata only. The private payload
selection still needs an exact approved delivery channel and readback; it does not
include archives, personal state, bearer-capable GUI reports or credential values.

Retain Apache-2.0 `LICENSE`/`NOTICE`, the included Ratatui MIT attribution, SDK and
desktop notices, upstream provenance and prominent original-to-derived
modification records. The inventory binds 11 license/provenance/packaging files.
Search also requires complete modified MPL-2.0 Nucleo covered source: published
subtree `2c52188a791552b54254c3708375e83ba9c18be0`, 41 files, upstream
`4253de9faabb4e5c6d81d946a5e35a90f87347ee`. Licenses alone are insufficient.
Fresh source-export/third-party-notice proofs and the full redistributed native
dependency license closure remain pending.

## Next dependency-ready action

The infrastructure owner must first identify authenticated worker command/file
access and admit resources, including **VM105's 32 GiB onboot commitment and
physical storage/IO**. Running-only headroom is not admission. Then review the
portable controls, complete OS/native/browser acquisition recipe, exact private
delivery selection and measured per-command budget before execution. Fresh source,
build, package, fixture and resource receipts must come from that worker.

The original checkout/index, nine separate variants, packet, accepted binaries,
raw evidence and recovery archives are retained. No helper/control execution,
native build, runtime/browser test, dependency download, private export, new
archive, worker provisioning or network-policy change occurred. VM-local metadata
is not external backup. Only this reviewed coordination checkpoint is published;
the I1–I5 infrastructure gate and all remaining product acceptance stay open.
