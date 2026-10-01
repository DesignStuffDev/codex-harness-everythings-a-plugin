# P00M checkpoint lineage inventory

`CHECKPOINT_LINEAGE_INDEX.json` binds the published source checkpoint
`d04d5a77d4dcaf51759666fd042ab65a3597d900`, tree
`8464b8683d1443718f357130b88d4597deb940de`, to the original upstream/import tree.
It inventories all 1,092 changed paths, all 28 historical lineage maps and 27
representative source relationships for native storage/migration, inline attachments,
native search, custom adapters/infrastructure and the additive GUI. It is a concrete
maintenance input, with unresolved semantics retained. It is not a complete symbol
graph, semantic equivalence proof, updater implementation or release approval.

The pinned checkpoint includes the accepted provider-owned endpoint prerequisite,
its evidence and the maintenance roadmap refinements. That provider seam is compiled
native support, not independently installed catalog extraction. This tooling and later
policy-epoch/full-CLI work remain outside the pinned source. Future indexes must name
their own immutable checkpoint; do not silently reinterpret this as the working tree.
The frozen 9cf index remains separate historical evidence. Its 27 destination blobs
are unchanged at d04 and are rebound explicitly; historical mappings stay unchanged.

Each path has exact old/new modes and object identities, inventory owners and area,
and explicit unresolved semantic status. Ownership assignment follows the package
ledger and documented support/staged areas; it does not certify exclusive behavioral
ownership. Native derivation/adaptation edges retain original upstream repository,
revision, path, blob and literal symbols. Original custom code has no invented
native origin. Dependency-use edges identify integration dependencies separately
from source derivation. Destination objects bind the published checkpoint while
historical mapping values and their original evidence scope remain available.

Run from the repository root with Python 3 and locally retained Git objects:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 upstream/test_validate_checkpoint_lineage.py
PYTHONDONTWRITEBYTECODE=1 python3 upstream/validate_checkpoint_lineage.py \
  upstream/CHECKPOINT_LINEAGE_INDEX.json --repository . \
  --publication-receipt upstream/CHECKPOINT_LINEAGE_PUBLICATION_RECEIPT.json
```

The validator first resolves a real local checkpoint commit and checks its tree.
A mismatch is invalid even if a supplied receipt agrees with the index. When that
commit object is available and correct, no publication receipt is required. Only
when it is unavailable does validation fall back to the exact digest-bound supplied
receipt; with neither available, the relation stays unresolved. Upstream/import
revision-to-tree checks also report unavailable objects explicitly. Fetching any
needed source objects is a separate operator action; the validator never fetches.

The publishable `CHECKPOINT_LINEAGE_PUBLICATION_RECEIPT.json` is a sanitized
normalization of the retained d04 publication receipt. It maps `commit` to `sha`
and `verified` to `remote_verified`, preserves `tree`, and records the original
receipt SHA-256 and parent/ref/non-force metadata. The generator checked those
fields against the original receipt. The validator checks this normalized receipt's
recorded digest and values, not a live GitHub ref or publisher identity. The file
contains no private VM path or runtime state and removes any need for that VM-only
receipt path. Artifact retrieval and restore durability remain separate gates.

The validator checks exact immutable changed-path coverage, old/new tree entries,
map-file blobs/hashes and a registry of 28 reviewed historical schema shapes.
The provider prerequisite has a separate adapter for its actual nested fields:
two source paths, their formatted hashes and byte lengths, and 11 unchanged
references. It checks the recorded upstream revision against the index and the
parent revision against the index's recorded publication context. Available local
parent commit/tree objects bind base hashes, new-file absence and unchanged tree
entries; an unavailable parent leaves those comparisons explicitly unresolved.
The frozen index's original `unsupported_unresolved` adapter label records what
was supported when that index was generated; validator dispatch uses its own
reviewed schema registry. The index and historical map are not rewritten.

Provider symbol labels are checked literally, without inventing Rust declaration
anchors. Nonliteral labels stay unresolved. The pre-format tested hash is not
compared to formatted bytes when they differ; the source-chain, local adoption and
archive receipts, test results, formatting equivalence, native origin semantics and
runtime claims remain unverified. Those receipt paths are never read or executed.
Unknown or missing nested fields and wrong types remain unsupported. The older
historical adapter reads common literal blob/anchor forms;
nonliteral or absent anchors stay unresolved. It separately checks explicit source origins,
current destination objects, symbol anchors and mapping pointers. A matching schema
shape or literal anchor does not validate every historical field, ownership claim,
behavior, test, package, state migration or acceptance result.

Exit 1 means invalid evidence/input; exit 2 means unresolved coverage or acceptance;
exit 0 is reserved for a future fully resolved metadata result. This version always
keeps release/updater acceptance unresolved. Output contains aggregate categories
and at most 16 short findings, at most two per category. JSON inputs are capped at
8 MiB, individual blob reads at 4 MiB, and the blob cache at 16 MiB. Git command output
is size-checked after capture, so these are not a streaming subprocess allocation
or whole-process memory guarantee. Commands use local `cat-file`/`ls-tree` only,
with optional locks, lazy fetching and replacement objects disabled. There are no
fetches, checkout/index/ref/object writes, source execution or activation operations.

The fixture suite checks corruption rather than asserting the generated contents:
wrong hashes, missing native origins, unknown schemas, omitted paths, invented
anchors, malformed nested documents, unrelated or wrong-revision destination edges,
current custom code mislabeled as native origin, and absent publication evidence.
A correctly bound edge, a local checkpoint commit without a receipt, and a local
commit/tree mismatch despite a matching receipt are also exercised. Provider fixtures
check source hashes/byte lengths, parent hashes and new-file absence, unchanged
reference hashes/tree entries, duplicate/missing paths, revision mismatches, strict
nested shapes, nonliteral symbols and intentionally different tested bytes. Fixture success is not native runtime
or updater acceptance.

The initial object check retained the provider schema as unsupported. The explicit
adapter removes that one schema gap while preserving all 1,092 unresolved path
semantics, 27 incomplete boundary closures, 39 earlier historical anchor gaps,
provider-specific source/evidence/anchor limitations and release/updater gates.
A recognized schema does not close those gaps. Preserve them in the maintenance queue. Later official upstream integration still
requires the separately installed updater, coordinated version/state tuple, isolated
real candidate, unchanged compatible plugins, breaking-candidate refusal and
external-bootstrap recovery after failed activation described in
`UPSTREAM_MAINTENANCE.md`.
