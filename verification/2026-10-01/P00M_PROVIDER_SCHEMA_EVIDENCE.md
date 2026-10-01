# P00M provider schema validation supplement

The adopted provider-lineage adapter passed **22 root-run Python fixtures** (exit 0).
The root's read-only object check returned the expected **unresolved** status (exit 2):
**0 invalid findings, 1,167 unresolved findings, 28 recognized historical maps**,
1,092 changed paths and 27 explicit relationships. This is metadata validation;
complete provenance, runtime behavior and updater acceptance remain unverified.

The [original P00M evidence](P00M_LINEAGE_EVIDENCE.md) and
[original results](p00m-lineage-results.json) remain unchanged: their 13-fixture,
1,160-unresolved result describes the earlier validator. The explicit provider adapter
replaces one unsupported-schema finding with eight specific limitations: one missing
parent-tree binding, one unavailable pre-format tested source, four nonliteral symbol
labels, one unverified historical-evidence group and one unresolved semantic group.
The increase to 1,167 records more precise gaps; it is not a count of runtime failures.

The adapter checks the actual nested provider schema, the two formatted source hashes
and byte lengths, and 11 reference hashes against the indexed d04 tree. Base hashes,
new-file absence and unchanged-reference comparisons require an available parent
commit/tree; that parent commit is unavailable in this check and remains unresolved.
Parent context is compared to recorded index metadata, not proved from checkpoint
commit-parent headers. Literal symbol matches do not prove Rust ownership or semantics.
The historical map's adoption, archive, tested-byte and formatting-equivalence claims
remain unverified by the validator.

Both root commands recorded identical before/after maps for all **34 scoped upstream
source files**. The two maps also match one another, the adopted file hashes and the
files inspected for this record. The wrapper's baseline is metadata, not a claim that
the dirty workspace equals a published commit. The [pinned index](../../upstream/CHECKPOINT_LINEAGE_INDEX.json)
still describes `d04d5a77d4dcaf51759666fd042ab65a3597d900`; neither the index nor the
historical provider map was rewritten. Nine added fixture methods exercise corrupted
source identities, revisions, references, field shapes, path coverage and symbol lists.

[Exact tested hashes, root commands, timestamps, log/report digests and unresolved
categories](p00m-provider-schema-results.json) bind this supplement to the actual runs.
The adoption receipt's zero-tests count is its earlier adoption-time state; the later
root command receipts record the completed checks. The archive and raw receipts are
cloud-local preservation, with no external availability or restore test claimed.
No test was rerun to prepare this evidence, and this record asserts no later global
formatting, Rust/native test, release activation, updater, migration or rollback gate.
