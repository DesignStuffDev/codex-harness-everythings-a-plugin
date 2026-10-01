# Codex Harness Compartmentalized

A work-in-progress fork of [OpenAI Codex](https://github.com/openai/codex), pinned
at `d42056091aded7feb1d88ac7e83972108b2aa478`, with separately installable engine
components and a separately packaged desktop-style web interface.

**This is a partial implementation.** Native thread storage and inline attachment
storage can run as independently built packages. Selected inference streaming and
credential acquisition/refresh have replacement adapters. The complete engine,
tool executor, context management, policy, and other subsystems are not yet all
independently replaceable. The GUI is our own new client of the real App Server;
it is not the official desktop application's source or an extracted native TUI.

- [Canonical implementation roadmap](IMPLEMENTATION_ROADMAP.md)
- [Execution state and ordered next actions](EXECUTION_STATE.md)
- [Complete source/component inventory](COMPONENT_INVENTORY.md)
- [Upstream update and recovery workflow](UPSTREAM_MAINTENANCE.md)
- [Historical component checkpoint](COMPONENTS.md)
- [Validation evidence and limitations](VALIDATION.md)
- [Recovery and publication checkpoint](RECOVERY.md)
- [Plugin SDK, templates and packaging](component-sdk/README.md)
- [GUI component and presentation contract](component-sdk/examples/desktop/README.md)
- [Upstream provenance](UPSTREAM_PROVENANCE.md), [LICENSE](LICENSE), [NOTICE](NOTICE)

## Build the host and add the GUI

Use the pinned Rust toolchain and upstream [build prerequisites](docs/install.md).
This initial host build can be substantial. Linux is the validated platform for
this checkpoint; broader platform validation remains open.

```sh
(cd codex-rs && cargo build -p codex-cli -p codex-component-host --bin codex --bin codex-component)
```

Choose a fresh external project/package directory and a Codex home. From the
repository root, build the GUI separately with Python 3.10+:

```sh
export PYTHONPATH="$PWD/component-sdk"
cp -R component-sdk/examples/desktop /tmp/my-codex-desktop
python3 -m codex_component_sdk build /tmp/my-codex-desktop --output /tmp/my-codex-desktop-package
mkdir -p /tmp/my-codex-home
./codex-rs/target/debug/codex-component --codex-home /tmp/my-codex-home install /tmp/my-codex-desktop-package
./codex-rs/target/debug/codex-component --codex-home /tmp/my-codex-home launch desktop --codex-bin "$PWD/codex-rs/target/debug/codex"
```

Configure model access in that Codex home using the CLI before ordinary inference.
The verification fixtures provide deterministic model responses solely for tests.
The launcher prints a private loopback URL. Open it on the machine running the
host; keep its bearer token private. The first Ctrl+C allows graceful gateway,
App Server and storage cleanup. A forced stop reports uncertain write durability.

No supported cloud-to-local preview/port-forwarding route is exposed in this
managed environment. The loopback URL is not a link that a browser on another
computer can open. [Recovered GUI screenshot](verification/2026-09-30/recovery-gui-recovery-cold-continued.png)
shows the independently installed interface after cold session recovery.

Custom component packages can be built and installed after the host's initial
build without recompiling it. CLI/headless operation remains available when the
GUI package is absent. See the SDK for native storage exports, explicit selection,
removal, compatibility, state ownership and cancellation contracts.

The original upstream README is retained in [UPSTREAM_README.md](UPSTREAM_README.md).
Its official installers install upstream Codex, not this fork. No DeepSeek or
Cordis code is included.
