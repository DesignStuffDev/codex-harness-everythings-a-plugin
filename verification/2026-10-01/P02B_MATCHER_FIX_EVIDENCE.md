# P02B: pinned matcher matrix extent correction

This is a narrow correction to an inherited **Nucleo Matcher dependency defect**, discovered while preparing bounded native search. It does not extract a subsystem, implement bounded indexing, or establish independently installed search behavior.

Source commit **`ac5fdae1b7db8134318891eeb991b9c330e6ea8b`**, tree `d59a58799acde091d508b99e27bdf7fa3ae74564`, changes exactly three paths relative to `612bf2a4a8d7537bcac38c8bef5c537fe1965c6e`:

- `codex-rs/third-party/nucleo/matcher/src/matrix.rs`: corrects the raw matrix slice length and registers its regression module.
- `codex-rs/third-party/nucleo/matcher/src/matrix_extent_tests.rs`: two ASCII/Unicode extent regressions.
- `codex-rs/third-party/nucleo/PROVENANCE.md`: records the intentional dependency modification.

All three blobs match preserved WIP `812b2d3e7df40dde146f4a33b3ad6fbacd53542c`. All **37 vendor file fingerprints** captured in the passing test run match this source commit. Subsequent working-tree vendor development was not used as immutable evidence.

## Defect and safe reproduction

Nucleo is pinned to `https://github.com/helix-editor/nucleo` revision `4253de9faabb4e5c6d81d946a5e35a90f87347ee`, under **MPL-2.0**. The inherited `matrix.rs` Git blob was `a91ed95f5beda8d9decc5c615cf5384bfbda4d5b`, recorded as unchanged in the [earlier vendor source audit](p02-nucleo-source-audit.json). Licenses and upstream provenance remain intact.

`MatrixLayout::new` reserves `(haystack_len + 1 - needle_len) * needle_len` matrix cells. The inherited `MatrixLayout::fieds_from_ptr` instead described a raw slice multiplied by `haystack_len`. For admitted inputs with a shorter needle, `MatrixSlab::alloc` could then form a mutable slice extending beyond its allocation. The fix uses the same needle dimension as the reserved layout; scoring and match-selection algorithms are unchanged.

The regression allocates a real `MatrixSlab`, confirms that the computed reserved layout fits, and calls the raw-pointer helper. It examines only the returned raw slice's length and address metadata. It **does not dereference that slice, construct a potentially invalid reference, or call the production `MatrixSlab::alloc` path**. Pointer offsets used by the helper remain within the real allocation. This permits a failing pre-fix assertion without deliberately constructing the invalid production reference.

On the uncorrected production expression, both ASCII and Unicode cases observed **260,608 cells and an end outside the slab**, rather than **2,036 cells within the slab**, for haystack length 512 and needle length 4. The corrected baseline was exactly the inherited `matrix.rs` plus test-module registration. Its regression-test bytes were identical to those used in the passing run.

Each passing test checks `(haystack, needle)` lengths `(512, 4)`, `(2048, 2)`, `(512, 128)` and `(1, 1)`. This is focused regression evidence, not an exhaustive memory-safety audit or an exploit demonstration.

## Recorded checks

| Run | Result | Meaning |
| --- | --- | --- |
| Initial regression compilation | **Exit 101; no tests ran** | The first ASCII fixture used `u8`, which does not implement the pinned `Char` trait. The fixture was corrected to `AsciiChar`; the failed build log remains separate. |
| Corrected pre-fix baseline | **2 executed, 0 passed, 2 expected failures; 25 skipped; exit 100** | Both raw-metadata extent tests failed, including one retry each. The source scope remained unchanged. Retries are not additional distinct cases. |
| Fixed vendor plus native WIP | **78/78 passed, 0 skipped; exit 0** | **Nucleo 10 + Matcher 27 = vendor 37**, plus **41 native search tests from newer WIP**. Scoped source was unchanged and the subreaper exited 0 with no runner error. |
| Scoped `just fix` | **Exit 0; source unchanged** | Scope included native search and component host; all 37 recorded vendor fingerprints remained unchanged. |
| `just fmt` | **Exit 0** | Only `codex-rs/component-host/src/session_start_tests.rs` changed in the recorded scope. All 37 vendor fingerprints stayed unchanged. No test rerun was made solely for formatting. |

The vendor 37 includes the existing 35 tests plus these two matrix regressions; it is not 37 additional tests. The passing run retains matching behavior tests and the existing supplied-pool/shutdown tests. Existing vendor style warnings remain.

The **41 native tests are not acceptance of the leaf commit's native implementation**: they exercised newer WIP queue/completion work. All 15 native source-file fingerprints match the preserved WIP; eight paths differ from this narrow leaf commit. Those differences and the exact source manifests are listed in [the machine-readable evidence](p02b-matcher-fix-results.json). No WIP native changes are silently included in this publication.

Original artifacts remain separately named under `/workspace/acceptance/`:

- `p02b-matrix-baseline.{log,source.json,subreaper.json}` — original fixture compilation failure.
- `p02b-matrix-baseline-corrected.{log,source.json,subreaper.json}` — intentional failures before the production correction.
- `p02b-matrix-fixed-native.{log,source.json,subreaper.json}` — passing vendor and newer-WIP native run.
- `p02b-native-startup-fix.{log,source.json}` and `p02b-native-startup-format.{log,source.json}` — final lint/format scope.

The JSON record contains exact artifact fingerprints, commands, per-run source identities, all 37 vendor-to-commit bindings and changed-path Git blobs. It includes only whitelisted metadata; credentials, bearer URLs and private runtime state are absent.

## Limits

No new CLI/App Server executable, installed search package, or GUI runtime was built or exercised for this leaf checkpoint. The earlier [StageA runtime/UI evidence](P02_SEARCH_PREREQUISITE_EVIDENCE.md) predates this matcher correction and is not relabeled. No sanitizer, Miri, cross-platform runtime or exhaustive safety check is claimed. Bounded indexing, native component integration and independent search installation remain separate work under the [canonical roadmap](../../IMPLEMENTATION_ROADMAP.md).
