# Immutable selection-plan primitive: fixture evidence

This checkpoint adds a **pure selection-policy planner over supplied evidence** to
maintenance support (C27). It does not expose a new installed tool or extract another
native subsystem. Nineteen focused fixtures pass on the exact 8,973-path source map
`ee5e3ad3afed565b6e701b35c59ed497d4c6ced7b7485bf451e7c92461f57718`.
See [structured evidence](EVIDENCE.json) and
[source lineage](../../../upstream/p18u-selection-plan-lineage.json).

`plan_selection` validates explicit component/feature/coherent-group choices,
retains exclusions, checks dependencies without silently adding them, and applies
ancestor-component and atomic-group constraints. Every declared owner needs an
exact whole-file rule binding path, category, before object/mode and original-source
identity. `SelectionPlan` freezes canonical input/result bytes; `restore_plan`
recomputes them and checks the expected content ID. Hash identity is not publisher
authentication, semantic source verification or durable persistence.

All 19 cases passed in 0.024 seconds as reported by unittest. The source wrapper
recorded identical candidate/before/after maps; strict subreaper exit was 0 with no
runner error and its sole tracked command (PID 44565) reaped with status 0. OOM/kill
counters were unchanged. `just fmt` completed with unchanged source. Full compressed
source/log/strict receipts were independently rehashed and decompressed; EVIDENCE
retains their identities and the exact command. These are fixture results, not a
production build, installed plugin lifecycle test or GUI result.

The fixture's 13-path historical impact projection was independently reconstructed
from retained source-capsule manifest `b44b3d6a…` and report `97b0977a…`; original and
factored projections both hash to `05059f8235edff0ed7ddd1153bf3cd0d10f7603cfff7002bdc4eaf8e49c7817e`.
It refers to actual pinned upstream `d4205609… → 2e5fea64…` and custom `914cc593…`.
Its missing mappings, historical composition and unknown ownership remain blocking.
This verifies preservation/refusal mechanics for retained evidence; no fresh Git
or real-update invocation occurred in this slice. Positive planning cases use
synthetic supplied graph/policy data.

Important limits:

- The module does not collect a filesystem inventory or compare a final diff.
  Allowed operations are validated and bound, not checked against actual changes.
  Baseline inventory SHA is recorded without validating baseline inventory bytes.
  Bound symbol literals do not establish symbol-level permission enforcement.
- Supplied graph ownership and dependency assertions still need independent current
  evidence. A restored historical plan need not describe today's repository.
- Profiles/receipts are not persisted. There is no installed capability, package
  upgrade, GUI, sandbox, migration, candidate build, activation, A/B route or rollback.
- The 4 MiB canonical JSON bound is not a measured CPU/deadline/cancellation bound.
  Installed exposure needs separate work admission and cancellation acceptance.
- All authority flags remain false and later gates pending. The native capacity
  gate remains closed. Prior 0.3 installed and 0.2 GUI evidence keep their own source
  identities; this checkpoint does not rerun or relabel them.

Exactly three files were added to documented 0.3 map `bdcc5e78…`; removing those
additions from the tested map reproduces all 8,970 base entries exactly. The
unadopted full final-inventory audit proposal remains preserved separately and is
not validated by these 19 cases. Next work is actual inventory/diff auditing and
separately admitted installed-tool packaging/runtime proof, followed by durable
planning UI and the full update/recovery gates. Local receipts alone are not an
externally durable backup; source/evidence publication requires separate readback.
