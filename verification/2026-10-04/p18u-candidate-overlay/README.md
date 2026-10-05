# Verified sparse source transformation and package upgrade

Package0.3.0 of `codex.maintenance.upstream-review` now prepares a bounded source
overlay through `tool:upstream_candidate_overlay` contract1. The separately built
package passed actual installation, explicit0.2→0.3replacement, normal transformation,
older-tool compatibility, removal and offline inspection through unchanged managerf054.
This is additive C27 maintenance, not another native subsystem extraction or a complete updater.

All47 focused cases passed:17overlay,11capsule,13review-adapter and6owned-Git cases.
The real acceptance passed61 commands:20manager calls,34read-only Git queries,
5standalone inspections, one installed-SDK identity check and one external build.
Five deliberate manager refusals returned1: duplicate installation, incompatible
component API and invocation of each of the three removed tools. They are expected
rejections, not ignored failures. Both source wrappers and strict subreapers returned0,
with no runner error or new OOM event. Each strict runner reaped its sole recorded
acceptance command with exit0; this is not an active-merge cancellation test.

The chosen source revision is actual `openai/codex@2e5fea64eefcaa19f48458b2386011b619f69c70`,
a child of imported `d42056091aded7feb1d88ac7e83972108b2aa478`. Input custom composition
remains committed `914cc59374c1149463e78bc33851d83e3f14d0a4`; the newly tested package
source follows published0.2checkpoint `5e3e9128d77e3c39b724cc3e85c52401ae6f6ad1`.
These are distinct identities. The running harness upstream revision did not advance.

All13 actual changed paths match an independent byte oracle. Twelve select exact
chosen-upstream bytes; the scenarios file combines the upstream change with the
custom deletion of one exact ReasoningEffort import. The oracle derives that custom
difference directly from base/custom objects, independently of the overlay merger.
The result has13source outputs, zero unresolved source paths and all seven release
gates still pending. `prepared` means every changed path has a **source result**;
direct selections may preserve binary blobs. A clean textual merge establishes no
semantic compatibility or update approval.

The test built outside the harness with separately installed SDK0.1.0, removed its
new disposable build copy before installation, and reused the existing host without
recompilation. Package zipapp SHA256:
`4a860c3c55d69bbfe3554cb3a1812d3bb797dfde3fd02dd9e05d074c98c681bf` (40,359B).
Overlay manifest SHA256:
`81fb203ded8a7dc3dc87b3a8938e8f946eccf147d9306c4f24b63874bae4e1c2`.
Original package/capsule bytes, old installed object, input-index/receipt and object
repository metadata remained unchanged.

Upgrade is explicitly **non-atomic**: remove old activation, observe the inactive
intermediate state, then install the same componentID at0.3. A byte-exact copy of a
completed0.2capsule was restored into the fresh isolated test home first. Both older
tool-v1 contracts still worked; the upgraded package generated an equivalent fresh
capsule. This proves that scoped restored-state compatibility, not a live installation
migration, atomic update or failed-update rollback.

Standalone inspectors worked with Git and host absent from PATH, before and after
component removal. They require both retained capsule and overlay in their state
layout, check current artifact integrity, and do not replay the merge or attest
historical crash durability. Terminal marker persistence remains best effort under
additional filesystem failure. Three bad tool requests and an incompatible API
package were rejected without changing retained test state.

`FOCUSED_AND_SOURCE.json` and `REAL_RUNTIME.json` are compact projections of completed
receipts. `SOURCE_INPUTS.json` binds all seven changed files to source, tests and the
actual external package. The SDK reserializes the manifest: its packaged JSON hash
is separate from its source hash, with parsed equality independently checked.
The candidate receipt's changed-path after-hashes retain adopted preformat bytes;
its source_map identifies formatted bytes. Lineage labels both without rewriting
original receipts. Current tests bind8970paths/map
`37656871852bcc56799a3b8340c1a90786d1cee7aebd012e94b7556f84bb1c90`.

No0.3GUI/storage/migration rerun is claimed. Earlier0.2regressions on map`0b7c68b9…`
remain separately referenced in `PRIOR_REGRESSION_CONTEXT.json`: real Chromium/
Playwright fallback and deterministic inference, not requested in-app Browser or
live-provider proof. All8949native baseline paths remain unchanged in recorded maps;
that fact alone is not a new runtime pass. New installed active-merge shutdown,
abrupt termination, child cleanup and subsequent recovery remain open despite the
47fixtures and older0.2active-read lifecycle proof.

The overlay is sparse: untouched paths are inherited by reference to the exact
committed custom tree. It is not a checkout, full backup or runnable candidate.
Full candidate assembly/build, updated-host custom-plugin/UI/headless behavior,
security/contract review, coordinated versions, state migration and external failed-
update recovery/rollback remain required. All update/activation flags remainfalse.
Native extraction stays at three bounded families; the auth/P03capacity gate stays
closed. No polling, live deployment or hot replacement was added.

The private runtime export contains116regular files (3,712,356B) in an823,031B
checksum/readback-verified archive, SHA256
`276f6e3ee6d74527138f3d270e56f399b5e09abfcaf4c79e43734e85de4b2978`.
Source tmp data remains intact. That archive is a same-workspace recovery checkpoint;
raw state is not included here and is not an externally durable backup. GitHub
publication/readback establishes external durability only for selected source and
sanitized evidence. Reproduction helper source is included separately from results.

After testing, the package README changed “textual result” to “source result” to
cover unchanged/direct binary copies. Runtime code and test/package identities
remain unchanged; see [POST_TEST_DOCUMENTATION.json](POST_TEST_DOCUMENTATION.json)
for the separate documented source identity. Tests were not rebound to new bytes.
