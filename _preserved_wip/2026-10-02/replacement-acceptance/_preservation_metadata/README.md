# Source preservation for replacement acceptance proposals

Prepared only; root owns any later WIP publication. This plan does not adopt implementation or execute tests. All four frozen proposal directories remain **unadopted, uncompiled and unrun**.

`ALLOWLIST.json` maps exactly **40 files / 890,669 bytes** to `_preserved_wip/2026-10-02/replacement-acceptance/`. It includes original preimages, proposed Rust/Python/documentation source, patches, static source bindings/replay transcript, review/application instructions, manifests, and preserved repository Apache-2.0 LICENSE/NOTICE. It excludes all 13 `replay/` duplicate files. No runtime homes, auth material, process logs, caches, binaries, archives or unrelated recovery content are selected. Every selected file and ancestor is symlink-free, every selected file is regular UTF-8 text, and every selected path belongs to its frozen manifest.

## Order and meaning

1. Native lifecycle observation proposal01: read-only exact-generation operation/handle state; four test additions are unrun.
2. Callback activity proposal01: read-only callback record state; two test additions are unrun. Its startup parent preimage matches the preceding proposal output.
3. Production acceptance proposal01: two review units, 554 Rust child/wiring lines and 561 Python/documentation lines. Three real same-process pending/replay/race cases remain unrun. Static source review and patch replay are not runtime proof.
4. Ordered application/test plan01: exact preimages/hashes and direct source anchors for A=0/B=1 behavior and required actual final joins.

Preservation is independent of that future application order. Existing source31f828/root candidate tests do not include these proposals. UnsupportedHome remains unchanged; HTTP MCP behavior does not certify pending MCP shutdown custody, and `whole_host_clean` stays false for this scoped fixture. No primary checkout/source/Git/network publication occurred while preparing this plan.

## Review and publication

`VERIFICATION.json` records each manifest/dependency/base/source binding check and excluded replay paths. All preimage/proposed/artifact sizes and hashes match; ordered-unit rows exactly match their frozen source manifests. Current read-only API source references and their Git blob hashes also match. No private runtime values were copied: appearances of home/auth names in source are program fields and fresh-fixture setup logic. Absolute provenance paths inside unchanged frozen documents are historic references, not authority to publish those referenced directories.

`PUBLICATION.json` supplies an inert destination, commit wording and root-only verification steps. The main allowlist contains only the40 proposal files. To preserve these new metadata files too, root may use the five explicit optional metadata mappings and verify `MANIFEST.json` hashes (the manifest itself is checked by its separately supplied digest). Never copy this whole recovery root. Do not follow symlinks. Do not change executable mode to make these stored sources active; use regular Git blob mode100644 throughout.

The three source proposals contain verbatim repository LICENSE (`d17f227e4df5da1600391338865ce0f3055211760a36688f816941d58232d8dc`) and NOTICE (`9d71575ecfd9a843fc1677b0efb08053c6ba9fd686a0de1a6f5382fd3c220915`). The ordered plan describes those same repository-derived proposals; there is no newly imported external implementation or dependency license. The earlier204-file preservation allowlist stays immutable and separate.
