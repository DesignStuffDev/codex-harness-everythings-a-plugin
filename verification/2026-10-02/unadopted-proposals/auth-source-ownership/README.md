# Preserved native auth source-ownership proposals — not adopted

These two archives preserve unfinished implementation source for review and later resumption. Neither proposal has been adopted, compiled, formatted, linted or tested. The active HTTP Stage B implementation was unchanged by their preparation or this preservation package. Existing production/package results do not validate either archive. Publishing these files preserves proposals; it does not enable their code, establish a new extracted subsystem, or complete P03/auth/catalog work.

## Contents and dependency

| Archive | Proposed change | Source review and unexecuted tests |
| --- | --- | --- |
| `UNADOPTED_AUTH_RELOAD_SOURCE.tar.gz` | Coherent native source/cache/policy capture and conditional reload publication. Five Rust paths, +259/-11. | Independent static review found no material blocker. Five authored test functions contain ten internal race scenarios; none ran. |
| `UNADOPTED_AUTH_REFRESH_SOURCE.tar.gz` | Conditional publication of permanent refresh failures through both actual public refresh callers. Three follow-up Rust paths, +285/-43. | Independent static review found no material blocker. Four authored test functions contain twelve internal scenarios; none ran. Native ChatGPT HTTP failure has no new held-response test. |

The refresh proposal **depends on the exact reload proposal**, not directly on the current checkout. Its `manager.rs`, `auth_reload.rs` and `auth_source.rs` preimages equal the reload proposal's corresponding proposed files byte-for-byte. `INTEGRITY.json` records this check. Review/adopt reload first, then mechanically rebase refresh after any accepted formatting/lint changes; never replace current files using stale full-file copies.

The reload sealed manifest is `5f77301a8c3a8c35f69f9b46da5ac363a05ae3fd54ac0af0ce6de8dc91a8f5a6`. The refresh sealed manifest is `fe3ba2d69f600264aaea9c4adeda68eb60c8f3673103e53d2d422bf50cdc75ce`.

Each archive includes readable contracts, validation plans, original/current preimages, proposed Rust source, an exact patch, source/provenance manifests and the independent static-review report. Test source is included for future execution; its presence is not test evidence. The archives contain no build caches, runtime executables, installed state or Git metadata.

## What remains unfixed

Reload publication and permanent-refresh-failure publication are bounded native ownership prerequisites. They do not make authentication independently installable or replaceable.

Successful refresh/install saves remain unfenced. A source check immediately before saving cannot prevent another manager/process from writing between that check and the old save; checking afterward cannot undo an overwrite or deletion reversal. Same-provider-Arc A→B→A and equal credential bytes require retained source/store revisions, not pointer or payload equality alone.

The refresh archive's `CONDITIONAL_STORAGE_AND_REQUEST_AUTHORITY.md` records the still-unimplemented work: conditional storage publication, shared-home generations, coordinated writers and cross-process exclusion, file/keyring/Auto recovery, explicit uncertain outcomes, legacy synchronous-clear settlement, and actual per-attempt request/body/cache authority. Mutable native token state, gateway settlement, broker/model composition and separately installed native/custom auth/catalog acceptance also remain open. Its two-manager test proposal covers failure-cache isolation only, not storage transactions.

## Integrity and provenance

- Reload archive: 112,190 bytes; SHA256 `617f5816116688d96a7136d9afafe75ca848ef2db9dc635498a7d23b7733df6c`.
- Refresh archive: 116,768 bytes; SHA256 `9d681b658cf34b9058f78768f18d4dcab4df7c321b9399f52219f37379ad5c90`.
- This package verified all 51 ordinary-file members against both root preservation receipts and the sealed manifests, checked unique relative paths and rejected links/special/sparse/set-id entries. It copied the original compressed bytes without repacking. No extraction or new full restore was performed here; root previously performed and verified full restores, as recorded in the copied preservation receipts.
- `PACKAGE_MANIFEST.json` and `SHA256SUMS` pin this publication package. `RELOAD_PRESERVATION.json` and `REFRESH_PRESERVATION.json` retain the original receipts, including VM-local paths. Those paths are provenance references, not portable restore destinations.
- Original source is OpenAI Codex at `d42056091aded7feb1d88ac7e83972108b2aa478`. Per-archive `PROVENANCE.json` distinguishes actual upstream symbols from earlier custom owner/reload modules and new authored tests. The repository's root Apache-2.0 LICENSE and NOTICE remain applicable and must be retained.

Before publication, this directory and the original archives are VM-local preservation only. Once this exact package is committed, pushed and read back from GitHub, the included source has external durability; that would still not establish native runtime acceptance. This package does not certify other unfinished work or all VM backups.

## Resume safely

Inspect the archives in a separate review location, never over the active checkout. Verify hashes and exact preimages, preserve current changes, and follow each archive's `GATES.md` using scoped `just test` and the repository's approval rules. Root owns source adoption, resource admission, compiler/cache operations and Git publication. No build/test/network/Git action was performed while preparing this package.
