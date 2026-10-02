# P03 process-final lineage and impact audit — 2026-10-02

This is independent P00M maintenance/provenance work completed while the original cloud
executor could not start. It changes no harness implementation or accepted-runtime status.

## Source and verification

[The machine-readable map](../../upstream/p03-process-final-lineage.json) binds the adopted
56-path P03 candidate to immutable source `f25b069357e5f41ee73b4430abadf827d56fc8d6`,
parent `ba87799c2e9ab821a326f10715c11cf6ec256896`, and exact upstream import
`ae720ae9a98bad29ca2cff998e7d5baaf05cec86`. Official upstream remains
`openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`.

The [validation record](P03_PROCESS_FINAL_LINEAGE_VALIDATION.json) records:

- All56 current Git blob identities and UTF8 byte lengths match the frozen candidate manifest.
- All56 paths were read at the parent and import:42 modify parent files and14 are new to that
  parent;31 exist in upstream and25 are project-added paths.
- All31 existing upstream paths were also read directly from official OpenAI Codex at the
  pinned revision; every blob matches the import.
- All56 selected literal source anchors match their named line and immutable blob.
- Two manager launcher/main references outside the56-file delta are unchanged from the parent.
- Both Cargo.lock files contain1,487 package blocks. Four local package blocks add five existing
  workspace dependency edges; external package entries remain unchanged. This is not a claim
  about unchanged feature unification, target/platform resolution or runtime behavior.

These were static source/data checks through GitHub and the functions JavaScript runtime.
No shell, native/fixture tests, SHA256 recomputation or local normalized-index validator ran.
SHA256 values are explicitly identified as values from the frozen manifest. Matching added
paths or literal declarations does not establish semantic equivalence or independent extraction.

## Review units and update hazards

The map groups review responsibility into eight units: native curated ownership, the
process-final clock, executable teardown, embedded/executable entrypoints, App Server
service lifetimes, client/stdio shutdown, the desktop gateway and build/acceptance support.

Original stdio already had dedicated signal/input/output threads and a45s watchdog. Our
adaptation changes watchdog authority, shared deadlines and EOF-triggered arming. It must
not be presented as inventing those upstream threads.

Keep these invariants together when evaluating a future upstream revision:

1. Process-final shutdown closes singleton admission irreversibly. Embedded replacement
   closes its own scope and must not invoke the process-global stop.
2. The native worker retains exact generations/handles/outcomes; callback actions reserve
   ownership before spawn. Observer cancellation does not release primary custody.
3. Same-home delivery uses lexical PathBuf equality, scope identity and the inherited
   process-wide success latch. Multi-home support remains absent.
4. Repository activation and SHA persistence remain separate steps. Cooperative stop fencing
   does not establish transactional publication, crash recovery or durable completion.
5. Curated native join plus callback completion does not prove unrelated/nested MCP work,
   detached descendants or storage durability.
6. The shared200s graceful/205s forced-initiation clock remains armed until actual runtime
   teardown and native-main join.205s is not an absolute OS-exit guarantee.
7. GUI207/209s and manager210s budgets require actual slow-storage/Launch acceptance;
   synchronous cleanup cannot be proven bounded from constants alone.
8. Upstream early-return, error-wrapping, policy/auth or service-order changes require semantic
   review, even when merging succeeds without conflicts.

## What remains open

This supplement does not advance or rewrite the frozen d04 normalized index, its1,167 reported
findings, or historical lineage fields. A reviewed adapter/current-index closure remains future
work. New upstream paths and semantics outside these selected boundaries remain unresolved.

The latest full-host evidence remains older source922. Current-source production build failed
ENOSPC, while TUI test compilation separately failed SIGKILL. Current installed storage/migration,
GUI, actual Launch Ctrl+C, held Git/HTTP,46s slow-storage/forced uncertainty and same-process
replacement gates remain unrun. MCP proposals remain preserved but unadopted.

P18U remains a required, unimplemented installable updater with an external recovery bootstrap.
A real later-upstream isolated integration preserving custom components and breaking/failed-update
rollback must still pass. No polling, source application, live activation or deployment was enabled.

## Executor resume result

The selected original environment was explicitly marked starting. Supported
`wait_for_environment` failed before tool setup:

`exec-server protocol error: failed to query executor configuration capabilities`.

The environment context then reported failed. The status service separately reported
running/connected with current observed revision1463, but no terminal/edit tools were exposed.
Those status facts do not establish filesystem or process recovery. No fresh build outcome
could be inspected; the latest recorded production build remains terminal101 at03:36:30Z.

No replacement VM, checkout, local execution, reset or repeated remote-viewer probe was used.
The source and WIP map publication is through the GitHub connector; synchronize the local
ledger only after recovery and comparison. Preserve the [temporary archive restoration and
exact recovery instructions](P03_PRODUCTION_BUILD01_AND_EXECUTOR_RECOVERY.md).

The first usable-executor actions remain: inspect any live build/process state, verify both
original source trees and real index, check source/archive hashes and temporary archive survival,
obtain guarded resource headroom, then run a uniquely identified production build and the
current-source acceptance sequence. Source checks here do not substitute for those gates.
