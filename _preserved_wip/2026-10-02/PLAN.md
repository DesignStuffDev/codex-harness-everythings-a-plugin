# Source-only WIP preservation plan

This is an exact allowlist for a possible external **WIP preservation branch**.
It performs no publication, source adoption, build, installation, Git operation
or checkout creation. Root's current 56-file candidate and its verified baseline
are separate and are not part of this selection.

`ALLOWLIST.json` records 204 regular UTF-8 text files, **7,971,037 bytes** total,
with absolute source path, preservation destination, role, size, mode and SHA-256.
`ALLOWLIST.tsv` is the same exact selection for a copying tool. Preserve their
listed bytes rather than recursively copying any directory. The destination
layout keeps proposals under `proposals/` and license notices under `licenses/`;
these are archival source copies, not active repository paths.

## Selected proposals and status

| Selection | Files | Bytes | Status and relationship |
| --- | ---: | ---: | --- |
| MCP upper02 and retirement03, including preserved original sources/corrections | 94 | 5,590,702 | Upper02 has 31 effective source files; retirement03 is a five-file additive overlay. Unadopted, uncompiled, unformatted and untested. |
| MCP lower workspace | 80 | 1,834,697 | 43 proposed source files, 30 preimages and seven frozen metadata/patch documents. Depends on upper02 plus retirement03. Unadopted and uncompiled; 27 new tests are authored only. |
| Pinned RMCP SDK completion | 16 | 425,177 | Five proposed source files, two upstream preimages and nine provenance/license/patch documents. Unadopted, uncompiled and untested; 15 authored tests. |
| Installed slow-store original01, corrected02 and both reviews | 12 | 109,293 | Original01 is preserved with its known blockers. Corrected02 has synthetic wire smoke evidence only; real installed storage, host, GUI and >45-second acceptance remain unrun. |
| Existing Codex LICENSE and NOTICE | 2 | 11,168 | Public repository licensing inputs; no current candidate source is copied. |

The upper selection includes the original and corrected held-host Python fixture
under its existing `acceptance-review/acceptance-fixture-02` source directories.
They are authored source/preimage attachments referenced by the frozen upper
proposal, not fixture homes, execution outputs or new acceptance claims.

Upper historical manifests, patches and correction preimages remain intact.
`COMBINED-MANIFEST-02.json` identifies the effective upper files, which include
specific correction overlays; copying only its top-level `proposed/` directory
would lose some final bytes. The original combined01 and ownership/propagation
manifests describe preserved earlier states. The outcome-provenance addendum
qualifies the older statement about retaining every original persistence error.

## Frozen bindings verified for this plan

| Manifest | SHA-256 |
| --- | --- |
| Upper combined02 | `d99b4b1ce641d51952eecb0adf9ba761777c1f5bea45ee2c9d7e5d961714dcef` |
| Upper retirement03 | `9431799f6a8331e5bd2e57db2122c2c25738685eb22d14028e5a869d8a1f319d` |
| Lower workspace01 | `2b587ab16cf31e8798b0f5ceead85f6d1324ecf3ba78694292df0323b4bba2b4` |
| Pinned SDK01 | `3882d8b918b2ba3f411488210b4a7cac24dc1f78ea01bfa3c4fb22420e2b8095` |
| Slow-store original01 | `05eacbd2f21720dd0776d566c306a7e4771f83c17e74a18dda629c1222fa2b4a` |
| Slow-store corrected02 | `313fbe4f70d7b550df3e50b9e7b5d330d5e0062a2008ada0e5843f6e35308a63` |

All six manifests retain their previously recorded hashes. Cross-checks of all
selected manifests verified 98 distinct referenced source files, 64 preimages,
and 23 patch/metadata paths against their recorded hashes. Every selected file
was then independently sized and hashed. The correction overlays are checked
against their frozen preimages; this makes no claim that the current active
checkout still matches an older adoption base. Future adoption requires its own
base verification or a new reviewed rebase.

## Explicit exclusions and privacy boundary

- Exclude 103 generated replay copies beneath `combined-*-replay-*`,
  `combined-replay-01`, `replay/` and `replay-01/`. Their exact final source bytes
  and patches are already selected from the frozen authored locations.
- Exclude all other recovery directories, including independent review replay
  trees, resource audits, binary/ELF archives, build outputs and caches.
- Exclude every `/workspace/acceptance/` payload and `.subreaper.json` report,
  logs, screenshots, browser files, fixture homes/state, auth/keyring material,
  installed packages, relay configs, native-PID/gate/release receipts and source
  or binary build-report payloads. Textual references and hashes inside frozen
  source/provenance documents remain as provenance; never follow those references
  to expand the copy set.
- Do not include `.git`, symlinks, compiled Python files, compiled executables, runtime
  dumps, or newly created descendants that were not enumerated in this allowlist.
  Python and Rust source scripts themselves remain included as text source.

Selection is bounded to the five authorized frozen proposal roots, two frozen
slow-store review documents and the two public Codex license notices. All 204
entries are regular text files with no NUL bytes. A targeted check found no
high-confidence API-key/private-key patterns in these selected source/doc files.
This is a bounded source selection, not permission to publish runtime data or
future unreviewed files. Do not glob the whole recovery root.

## Licensing

Codex-derived source preserves the repository's Apache-2.0 `LICENSE` and existing
`NOTICE`, including its third-party attribution. These files are listed from
the existing checkout as license inputs only. No license headers are stripped
or changed by this plan.

The SDK proposal preserves exact upstream `UPSTREAM-LICENSE` and
`UPSTREAM-PROVENANCE.json`, bound to
`3e636cab26c013eca5131103c03d20237f12c4df`. Its notice describes the MIT-to-Apache
transition and documentation licensing; do not relabel all inherited SDK code
as exclusively Apache-2.0. No new upstream revision, fork publication, license
replacement or dependency pin is created here.

## Preservation and later adoption are separate actions

For a later root-authorized backup, re-read and verify the allowlisted bytes
immediately before copying. Stop on a missing file, symlink, size/hash mismatch
or unexpected destination. Copy only the exact rows into the inert preservation
layout, retain this plan and its manifest, then verify every destination hash.
An external commit/receipt should state **unadopted WIP source preservation** and
record the allowlist hash and copied-file verification; it must not represent a
main/verified-candidate promotion or passing build/runtime evidence.

Any later implementation adoption remains ordered: upper02, retirement03, then
lower workspace. The lower nine-file codex-mcp adapter is already included and
must not be applied twice. The SDK patch needs a separately approved immutable
patched source identity, Cargo/Bazel lock updates, workspace HTTP consumption,
and actual validation. Historical-generation/process-final MCP aggregation,
executor/descendant proof and the real host/GUI/storage acceptance remain open.
Preserve slow-store01 as history; corrected02 is the proposed future fixture.

No external durability has been established by creating this local plan.
