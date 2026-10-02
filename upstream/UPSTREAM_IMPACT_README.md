# Offline upstream impact planning

`plan_upstream_impact.py` is an initial read-only maintenance tool. It compares
explicit local Git revisions, reuses the existing lineage validator, and reports
changed paths with recorded component owners and unresolved impact. It performs
no fetch, checkout, patch application, build, migration or activation.

This is not an installed maintenance component or an upstream integration. The
normalized lineage index still describes the historical d04d5a7 checkpoint;
requesting a newer composition explicitly reports missing coverage. Recorded path
matches cannot establish semantic, contract, security or migration compatibility.

Run with exact 40-character commit IDs:

```sh
python3 -B upstream/plan_upstream_impact.py upstream/CHECKPOINT_LINEAGE_INDEX.json \
  --repository . \
  --candidate-revision CHOSEN_UPSTREAM_COMMIT \
  --composition-revision CURRENT_COMPOSITION_COMMIT \
  --publication-receipt upstream/CHECKPOINT_LINEAGE_PUBLICATION_RECEIPT.json
```

The program writes versioned JSON to stdout. Exit 1 means invalid input/evidence;
exit 2 means an unresolved plan. This version never approves an update. Missing
objects remain unresolved and are never fetched implicitly. A commit hash alone
does not authenticate a publisher or establish later-revision ancestry.

Output includes exact available revision/tree identities, input/tool digests,
additions/deletions/mode changes, conservative review hints, and bounded pointers
into the hash-bound lineage index. Missing mappings, multiple owners, stale
composition, omitted details and unknown semantics remain visible. There is no
rename inference or complete transitive dependency analysis. The existing Git
adapter's process/output limits are inherited; the report bounds are not a new
streaming-memory or process-supervision guarantee.

Validation so far: 17 in-memory planner fixtures and the existing 22 lineage
corruption checks passed. A real local-object invocation at the same upstream
revision returned the expected unresolved exit 2, including no revision advance
and stale composition. That invocation integrated no upstream changes. Original
tested bytes and subsequent formatting are recorded separately in the evidence.

P18U still requires current provenance closure, a separately installed maintenance
worker, isolated candidate preparation preserving customizations, coordinated
host/package/schema versions, real host/plugin/UI gates, and an external recovery
entrypoint. Release acceptance must integrate a real later upstream revision and
demonstrate incompatible-update rejection and failed-update/state recovery. No
scheduled polling or live update is enabled here.
