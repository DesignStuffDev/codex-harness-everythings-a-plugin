# Separate native component packages

`rust_component_package.py` exports one native component's production source
dependencies from a virtual Cargo workspace, then assembles a separately built
executable into an installable component directory. It requires **Python 3.11+**
for `tomllib`; the Python plugin SDK continues to support Python 3.10+.

This is source packaging, not an implementation generator. A custom native
component must implement its service contract through the
[persistent Rust server API](PERSISTENT_PROTOCOL.md), or the one-invocation wire
protocol in [the SDK README](README.md). Installed components execute trusted code
with the user's privileges. Native binaries need a compatible operating system,
architecture and runtime libraries; a source export does not make them portable.

## Export the native thread store

Set `REPO` to the harness checkout and `SOURCE` to a new directory outside it:

```sh
python3 "$REPO/component-sdk/rust_component_package.py" plan \
  --repo "$REPO" --package codex-thread-store-local-plugin
python3 "$REPO/component-sdk/rust_component_package.py" export \
  --repo "$REPO" --package codex-thread-store-local-plugin --output "$SOURCE"
```

The plan lists local crates, dependency edges, copied bytes and resource warnings
without copying or compiling anything. Export requires a nonexistent destination;
it stages the copy and renames it into place. The source checkout is unchanged.
`--workspace` defaults to `codex-rs`. Repeat `--resource relative/path` for an
additional repository resource read indirectly by a build script.

The exporter follows normal, build, target-specific and optional path dependencies,
including inherited workspace aliases and local Cargo overrides. It keeps all
targets and optional dependencies conservatively. It removes development dependency
tables, trims unused workspace members and inherited aliases, and preserves the
remaining manifest data. It does not edit Rust source. This means exported tests
that require removed development dependencies are not a supported test workspace;
run the original component's tests before packaging.

Selected crates retain their source, migrations, schemas, templates, native files
and other resources. `target`, `.git`, Python caches and unselected nested Cargo
packages are excluded. Direct literal Rust `include_*` paths are audited and their
resources are included. Symlinks and dependencies outside the selected workspace
are rejected. Arbitrary build scripts, computed resource paths and runtime reads
still require an actual separate build and runtime check.

Root `LICENSE`, `NOTICE`, upstream provenance, Cargo configuration, toolchain pins,
lockfile, profiles and git patches are retained. `COMPONENT_SOURCE_EXPORT.json`
records the dependency inventory and SHA-256 of every source and exported file,
including manifests whose build metadata changed. These hashes describe the
export at creation; Cargo can subsequently rewrite the exported lockfile.

## Resolve and build independently

Run Cargo **from the exported workspace directory** so its copied parent Cargo
configuration and pinned toolchain apply:

```sh
cd "$SOURCE/codex-rs"
cargo metadata --offline --no-deps --format-version 1 > ../local-metadata.json
cargo metadata --offline --filter-platform x86_64-unknown-linux-gnu \
  --format-version 1 > ../resolved-metadata.json
cargo build --offline --locked -p codex-thread-store-local-plugin \
  --bin codex-thread-store-local-plugin
```

Choose the intended Rust target instead of the Linux example when appropriate.
Cargo may prune the copied full-workspace lockfile on the first resolution. Review
the resulting versions, checksums and git revisions before building with `--locked`.
The helper preserves upstream pins; it neither resolves new versions nor runs Cargo.

The export does **not** vendor registry crates, git checkouts or the Rust toolchain.
Offline commands need an already populated compatible Cargo cache and installed
toolchain. A clean offline machine also needs those dependencies provided through
its normal Cargo vendor/cache workflow. Online dependency acquisition, if needed,
must preserve the reviewed lockfile and git revisions. Native dependencies require
the platform compiler/linker and any build tools selected by that dependency graph
(for example CMake, pkg-config and protobuf tooling). Those tools are not bundled.

Use a separate `CARGO_TARGET_DIR` when preserving a built harness fingerprint or
sharing build resources with other work. The commands above build the plugin, not
the Codex CLI, core engine or app-server. Metadata validation alone does not prove
the plugin compiles or functions.

## Assemble and install

After the independent build, set `BINARY` to that build's executable and `PACKAGE`
to another new directory:

```sh
python3 "$REPO/component-sdk/rust_component_package.py" assemble \
  --repo "$SOURCE" --binary "$BINARY" --output "$PACKAGE" \
  --id native.thread-store-local --kind thread_store --name default \
  --contract-version 2
codex-component --codex-home "$TEST_HOME" install "$PACKAGE"
codex-component --codex-home "$TEST_HOME" select thread_store default native.thread-store-local
```

Assembly copies the executable and root license notices and emits
`codex-component.json` with API version 1, the explicitly selected component
contract version, and package version `0.1.0`. The current thread-store contract
is version 2; other components must select their own supported contract version.
Review its version, component metadata, required plugin dependencies and arguments
before distributing it. Additional native runtime libraries and third-party
license obligations remain the package author's responsibility. The install command
copies the completed package into the host's immutable object store; the original
source and build tree are not required at activation.

The thread-store plugin uses the persistent protocol. The one-invocation CLI `call`
command intentionally cannot drive it. Exercise it through an actual engine
session or a client using `ComponentBinding::connect()`, then verify persistence,
recovery, cancellation, fork/delete ordering and shutdown. Capture host hashes
before installation and after the acceptance run to substantiate that the host
was not rebuilt.

## Current validation scope

The current storage-v2 native thread-store export contains 81 local crates and
917 resolved packages. It excludes `codex-core`, the CLI, TUI, app-server implementation and
`core_test_support`. Its current production closure still includes substantial
native protocol, extension, configuration and tool dependencies; source packaging
does not prove those dependencies are independently replaceable.

The [storage-v2 independent-build report](../verification/2026-09-30/thread-store-native-v2-independent.json)
records a fresh offline, locked build from the exported source in 109.286 seconds.
Every local path stayed inside the export. The copied lockfile was pruned from
1,482 to 1,071 identities with no new identities. After explicit contract-2
assembly and stripping the package copy, the exported source was removed before
installation. The [actual CLI gate](../verification/2026-09-30/thread-store-native-v2-cli.json)
passed, and the same package passed the [real GUI/manager gate](../verification/2026-09-30/gui-manager-native-storage-v2.json):
native approval/execution, interruption, two graceful manager Ctrl+C shutdowns,
all tracked PIDs absent, cold recovery and a new completed turn. Both host binary
hashes stayed unchanged. These checks reused cached dependencies/the shared Cargo
target and deterministic inference; they do not constitute a clean-cache build,
arbitrary-backend conformance or a blocked-maintenance GUI test.

The preserved storage-contract-v1 export contained 80 local crates (about 26.6 MB)
and passed local manifest checks, offline Linux dependency resolution
and an independent native build from the exported workspace. The build reused the
existing dependency cache and Cargo target; its source paths stayed inside the
export. After packaging, the exported source was deleted before installation.
Actual CLI turns verified external native storage, cold resume, calculator output,
context and model event streaming, followed by native storage restoration with
retained history. All three observed storage workers were reaped. The Codex CLI and
component manager hashes, sizes and modification times stayed unchanged across the
export, build and runtime checks. Inference used deterministic test providers.

That historical run does not validate the later storage-contract-v2 wire changes.
See [the independent native storage report](../verification/2026-09-30/thread-store-native-independent.json)
and [final source inventory](../verification/2026-09-30/thread-store-native-final-source-inventory.json).
This is Linux validation with cached dependencies, not a clean-cache or
cross-platform build; an earlier all-platform offline metadata query encountered
an uncached Android crate. The previously completed native inline
attachment plugin's separate build/install is recorded in
[its acceptance report](../verification/2026-09-30/attachment-inline-independent.json).

Test the exporter without compiling Rust:

```sh
python3 -m unittest discover -s "$REPO/component-sdk/tests" -p test_rust_package.py -v
```
