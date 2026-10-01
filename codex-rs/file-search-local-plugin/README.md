# Native file-search component

This worker puts the actual Codex ignore-aware walker and Nucleo matcher behind
the persistent `file_search:default` contract, version 1. It uses the shared
process service and native backend, without depending on Core, App Server, the
TUI, or the callback runtime. It is not a separate inference harness.

The host starts one immutable selected provider in an explicit absolute working
directory. The worker preserves lexical search roots, including relative roots;
it never changes cwd per request or normalizes roots into different spellings.
Its package assets are resolved independently of cwd. The search backend itself
does not persist search indexes or use the supplied state directory for data.
Components execute trusted code; explicit roots and path validation are not an
OS sandbox, and upstream symlink behavior is preserved.

## Admission and accounting

Initial worker policy permits at most 16 retained leases, with aggregate ceilings
of 1,000,000 indexed entries, 512 MiB of charged index storage, and 64 dedicated
native workers. `file_search/initialize` negotiates these and the shared protocol
limits downward. Every `open` still supplies an explicit per-session allocation;
these provider ceilings are not defaults for each session. No resource limit is
silently increased or truncated into a successful partial result.

These are initial bounded policy choices, not benchmark-backed sizing claims.
Before choosing production session budgets, record the fixed allocation floor
computed by the pinned `IndexAllocationPlan<IndexedEntry>` at the intended entry
capacity/thread count, then allow for indexed path/matcher-column payload. A
budget can be below the provider ceiling and still fail native preflight because
it cannot pay its own fixed storage. Index or payload exhaustion is a typed
failure; it cannot appear as a completed index.

Native worker admission charges `2 * threads + 2` per session: matching and
traversal pools plus supervisor/outer walker. Existing TUI options with two
threads require six workers; twelve-thread App Server options require 26.
The executable separately fixes its shared Tokio pool to two async workers and
at most 32 blocking workers. Native index budgets exclude that shared pool,
thread stacks, allocator metadata, traversal state, every output copy, and RSS.

Wire bounds keep their existing meanings: query text is at most 64 KiB UTF-8,
serialized roots/options at most 256 KiB, at most 1,024 returned matches, and a
serialized frame at most 1 MiB. Native input accounting also charges path/string
headers; the factory derives a checked conservative allocation allowance from
the negotiated wire input bound. This is an accounting conversion, not permission
for the protocol to accept a larger request.

Native callback snapshots have a separate allocation charge: the result vector,
query bytes, path/root payload, and prepaid highlight-index capacity. Its default
ceiling is 16 MiB per snapshot, configurable from 64 KiB through 64 MiB:

```json
{"native_snapshot_bytes": 16777216}
```

Empty configuration uses the default. The factory also accepts null for direct
embeddings; the component catalog writes object configuration. Unknown fields,
wrong types, and unsupported limits fail initialization without native fallback.
`config.schema.json` documents this package's configuration shape. The native
snapshot ceiling does not replace or equal the exact encoded frame ceiling.
Long repeated roots/highlight allocations can exhaust the native output budget
even if their potential wire representation is smaller. Failure remains explicit.

Dropping an open/query/poll observer does not abandon accepted work. Close and
provider shutdown report the first operation failure separately from joined or
unconfirmed cleanup. Unconfirmed leases retain their reservations. A new selection
activates on restart; live replacement is not implemented.

## Build, package, install

After this crate is registered and verified, export it with the existing SDK:

```sh
python3 "$REPO/component-sdk/rust_component_package.py" export \
  --repo "$REPO" --package codex-file-search-local-plugin --output "$SOURCE"
cd "$SOURCE/codex-rs"
cargo build --offline --locked -p codex-file-search-local-plugin \
  --bin codex-file-search-local-plugin
python3 "$REPO/component-sdk/rust_component_package.py" assemble \
  --repo "$SOURCE" --binary "$BINARY" --output "$PACKAGE" \
  --id native.file-search-local --kind file_search --name default \
  --contract-version 1 --version 0.1.0
codex-component --codex-home "$TEST_HOME" install "$PACKAGE"
codex-component --codex-home "$TEST_HOME" select file_search default native.file-search-local
```

Use fresh destinations outside the harness and a separate target directory. Review
any initial exported-lockfile pruning before a `--locked` build. Assembly must use
the inventoried export and preserve Apache LICENSE/NOTICE, original provenance,
and the vendored Nucleo/Matcher MPL-covered source and notices. See the SDK's
`RUST_PACKAGING.md` for dependency provisioning and export limitations.

Installation and selection do not prove consumer integration. The component
manager's generic one-shot call intentionally rejects this persistent contract.
A client uses `ComponentCatalog::load(home)?.selected("file_search", "default")`,
then `ProcessSearchBackend::connect(binding, InitializeRequest { ... })`, choosing
an explicit base directory, fresh UUID-v4 provider identity, and resource policy.
Use the returned `negotiated_limits()` when constructing runtime scopes; open,
update, poll to a current `Idle` snapshot, close, and shut down through the neutral
backend traits. Always inspect both operation and cleanup outcomes.

## Validation boundaries

The source includes configuration contract tests and real-process native tests.
The latter install the compiled worker into a temporary component catalog, remove
the temporary input package, select the installed object, and exercise native
search/ignore/highlight results, independent cwd contexts, scope-like sibling
leases, quota reuse, typed resource failure, and removal. These are workspace
build tests; their transient minimal packages are not distribution artifacts.
Run them under the existing subreaper and retain actual source fingerprints.

The outside-source build/install with an unchanged host, mixed-root differential
acceptance against upstream CLI behavior, GUI/App Server/TUI/headless activation,
failure injection and publication remain separate gates. Source/test presence is
not evidence that those gates have run.
