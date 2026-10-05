# Installed selection planning and revalidation

Maintenance package **0.4.0** adds `tool:upstream_selection_plan@1` and
`tool:upstream_selection_restore@1`. It was built outside the harness source with
installed SDK 0.1.0, installed through the unchanged component manager, exercised,
and removed. Its three existing review/capsule/overlay contracts also passed fresh
normal runtime checks. This is additive C27 maintenance functionality; it does not
extract another pre-existing native Codex subsystem or complete the updater.

The source fingerprint for both completed runs is **8,975 paths**, map
`d84cf069cea7be4da7aa68cbc43d56f72483d3ea05546f1b340eab28b9e30c99`.
The recorded scopes are `codex-rs`, `component-sdk`, `MODULE.bazel` and
`MODULE.bazel.lock`; this is not a fingerprint of every root document or recovery
artifact. Before/after maps match exactly. `just fmt` completed without changing
these source bytes. Package zipapp SHA-256:
`913e0842bfae1ec72e80c118612b8aed73575625ea13b7bd71d4bf073dd87a29`
(46,687 bytes). Manager SHA-256:
`f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.
No host or Rust compilation occurred.

All **79 focused cases** passed: 19 selection-primitive, 13 adapter, 17 overlay,
11 capsule, 13 maintenance-review and 6 owned-Git cases. These include pure supplied
policy fixtures, bounded local Git fixtures and mocked lifecycle cases. They are
not 79 installed end-to-end update scenarios.

The installed acceptance completed **36 direct commands**: 31 manager commands,
one SDK identity check, one external build, one standalone full impact review, and
two independent retained-overlay inspections. Seventeen tool responses included
four deliberate invalid refusals and four successfully computed blocked plans.
Seven command exits were intentionally nonzero: duplicate installation, five
removed-tool refusals, and the standalone review's documented exit 2. These are
expected rejections, not ignored failures. Both source wrappers and strict runners
returned 0 with no runner error or new OOM event. The strict runners recorded only
their respective acceptance command as reaped with exit 0; this is not an
active-merge cancellation test or a claim that every descendant exited normally.

The positive selection cases use explicitly **synthetic supplied graphs and object
identities** through the real installed package. They verify deliberate exclusions,
required dependencies without silent additions, every declared owner's scope,
catalog staleness, bounded invalid shapes, changed profile identities, exact plan
revalidation, altered-result refusal and incompatible contract refusal. The adapter
does not attest those graphs against the filesystem or enforce a final diff.

The real source input remains official Codex
`2e5fea64eefcaa19f48458b2386011b619f69c70`, relative to imported
`d42056091aded7feb1d88ac7e83972108b2aa478`, with committed custom composition
`914cc59374c1149463e78bc33851d83e3f14d0a4`. These are historical review inputs,
distinct from the currently tested package source. The installed package's
standalone reviewer regenerated the same complete 13-path report, SHA-256
`97b0977a14d1a54a1f6adf7605a8f364f61350b72f98f3b533ad84862c0b1100`.
The new planner received its required projection with all actual unresolved
findings preserved. An explicitly empty/unresolved catalog retained fixture-only
composition/inventory placeholders; no historical ownership label was promoted to
a trusted mapping. The result stayed blocked: zero mapped selected changes and
22 findings. This proves refusal, not a valid real component selection.

Small responses contain complete canonical caller-owned plan bytes and their
identity. The real blocked plan was 7,567 bytes before response embedding; the
bounded response explicitly omitted the complete plan and returned the first
16 findings with omission metadata. A summary/digest is not an exported restorable
plan. Observed reconstructed maximum-u64 SDK frames ranged from 327 to 6,274 bytes.
Those frames were reconstructed from manager-decoded results and roundtripped as
JSON; they were not captured raw SDK traffic. Direct manager transport and the
8 KiB text bound do not prove nontruncation under smaller native model tool-output
budgets. Larger CLI arguments were prevented before process launch; no `E2BIG`
was counted as plugin input validation.

The isolated package upgrade was **non-atomic remove/install**, with its inactive
intermediate state observed. Old package objects and byte-exact copies of retained
capsule/overlay state survived. The new package reproduced the existing impact
report, produced a fresh equivalent source capsule, and produced an overlay whose
13 path records and output bytes exactly matched the retained prior overlay.
The overlay's existing schema retains its 0.3 package-version field; the new
installation is independently identified as package 0.4. No coordinated release
activation, live production migration or installation rollback is established.

Planning and restoration did not create or modify component state files; directory
metadata stability is not claimed. Original source, packages, capsule, overlay,
manager, Git metadata and bound inputs stayed unchanged. After removal, all five
registered tools were unavailable, both package objects and test state remained,
and the caller-owned plan bytes retained their exact identity. The old overlay
remained inspectable without host or Git on PATH. There is **no supported offline
selection-plan restore entry point** in this slice; private imports are not
presented as recovery.

No GUI/browser, storage or migration regression rerun is claimed here. Prior UI
evidence remains bound to its earlier source/package and Chromium fallback, not
the requested in-app Browser. Trusted inventory generation, final-diff scope
enforcement, persistent exclusion/profile service, the interactive walkthrough,
consistent recovery sets, A/B routing and session ownership, candidate external
effect isolation, full upstream candidate validation, activation and later
write-preserving rollback remain required. Every update/activation/authority flag
remains false. Native auth/P03 capacity is not cleared by these Python checks.

The private runtime export is checksum/readback verified: 956,383 compressed bytes,
4,618,748 uncompressed bytes, SHA-256
`889e743fa68e54fdb5e770d598c26525bb36fb529734f8e8939f7d09a465303a`.
Original runtime data remains intact. This is a workspace-local checkpoint, not an
external backup or a complete live-installation recovery set. Raw packages/state
are excluded from public evidence. Selected source/evidence become externally
durable only through separately verified GitHub publication.

A root prelaunch seal-check script used the wrong collection type and raised a
`TypeError`. It did not gate the subsequent separate launch. Root had read the
helper/card beforehand and later verified every sealed helper file correctly.
The correction receipt is hash-bound in `EVIDENCE.json`; no product testcase was
replayed and no completed runtime result was relabeled.

`EVIDENCE.json` contains sanitized outcome projections and original receipt hashes.
`SOURCE_INPUTS.json` distinguishes source manifests, normalized package bytes,
archive members and reproduction inputs. The original full source/strict/log
receipts remain separately retained; reproduction source is included.
