# Whole supplied-inventory scope validation

The new pure `final_scope.audit_scope` function compares complete **supplied**
before/after inventory records against an exactly revalidated selection plan.
It checks every derived difference against the selected paths and every declared
owner's operation permissions. It has no filesystem collector, process, persistent
state or new installed tool surface. This is additive C27 maintenance support,
not another native subsystem extraction or completed final-diff enforcement.

All **50 focused fixtures passed**: 18 new inventory/scope cases, 19 existing
selection-plan cases and 13 existing adapter cases. The exact tested source is
8,977 scoped paths, SHA-256 map
`f27ab9e949b997ff910baf0c891465e3847879a94b0b29b498b04c01246ed4b2`.
Both source maps match before/after. All 8,975 previous scoped paths remain
identical; only `final_scope.py` and `test_final_scope.py` were added. The scope is
`codex-rs`, `component-sdk`, `MODULE.bazel` and `MODULE.bazel.lock`, not every root
document or recovery file. `just fmt` returned 0 without changing source.

The source wrapper and strict subreaper returned 0 with no runner error or new OOM
event. The strict runner recorded its sole acceptance command, PID 55351, reaped
with exit 0. These are pure-data/adapter fixtures; that process result adds no
installed lifecycle, active-child cancellation or real session proof. No tests
were rerun while projecting this evidence.

The validator first recomputes the immutable plan and checks its expected identity.
It binds the entire supplied before-inventory envelope, including unchanged or
unmapped records, then derives differences from the union of both supplied path
sets. It accepts no worker-filtered change list. Unselected/generated differences,
incorrect before objects, missing selected changes and absent permissions block.
Content and executable-bit changes require their distinct grants for every declared
owner. New executable files need both `add` and `mode`; renames need separately
selected/scoped delete and add endpoints. Wrong category/origin bindings and
unresolved ownership or impact findings stay blocking.

Malformed kinds/modes, symbolic links, noncanonical paths, ambiguous case aliases,
file/path-prefix collisions, inconsistent declared blob/hash/size pairs, invalid
numeric keys and stale/tampered plan identities are refused. Case-only renames and
file-to-directory transitions deliberately remain unsupported under the union
rules, even with delete/add grants. Inventories are limited to 16,384 regular-file
records each, with the existing 4 MiB canonical input/output bound. Oversized
receipts reject rather than clip changes. These are finite data bounds, not a new
measured CPU deadline or preemptive cancellation guarantee.

The positive fixtures use **invented content hashes, blob IDs, sizes and owner
mappings**. The validator checks internal consistency but does not prove that
records correspond to disk bytes. The `completeness` field is a caller assertion.
A coherent but false inventory can satisfy this library; trusted collection,
race-resistant snapshot ownership, real object verification, semantic category/
component mapping and binding to the final candidate filesystem remain required.
`supplied_inventory_conforms` never means an update is safe or authorized.

One new fixture passes the retained historical 13-path upstream impact projection
through an otherwise conforming synthetic inventory comparison. It remains blocked
on historical composition/ownership gaps. This is inherited fixture refusal, not
a fresh Git invocation, a resolved real mapping or another real upstream-integration
pass. Plan/before/after identities and all findings are included in each successful
calculation's content-addressed receipt. All update, activation, complete-candidate
and enforcement-authority flags remain false.

No package was rebuilt or installed for this two-file slice. The earlier package
0.4.0 runtime result remains bound to its 8,975-path `d84cf069…` source identity;
it is not rebound to this new module. No GUI/browser, storage or migration regression,
native build/capacity acceptance, complete updated candidate, A/B routing,
activation or write-preserving rollback was run or passed here.

The earlier full, unadopted selection/scope proposal is intact: all five sealed
members were read and hash-checked. The adopted two-file proposal's seven listed
members were also verified. Original source/strict/log receipts and their compressed
copies remain retained. Their compressed hashes and decompressed bytes were checked
against the originals; this README does not publish the huge full source map or
private runtime state. Workspace receipt preservation is not external backup.

`EVIDENCE.json` records exact source, test, preservation and receipt identities.
`upstream/p18u-final-scope-lineage.json` records the custom symbols/contracts and
their limits. The next boundary is trustworthy inventory collection and current
source/component classification, followed by bounded installed integration and
the interactive maintenance owner. Full staged update, consistent recovery, A/B
session routing and later rollback after new writes remain required.
