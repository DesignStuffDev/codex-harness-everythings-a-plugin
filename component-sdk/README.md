# Codex component developer SDK

This SDK builds separate process plugins for this fork's engine component host.
It does not load plugins into the installed official Codex desktop app. Components
are trusted executable code with the user's privileges; installing one grants
engine access. They are not ordinary skills or MCP servers. The current adapters
do not make every Codex subsystem independently replaceable.

The Python implementation requires Python 3.10+ and uses only its standard
library. The resulting zipapp embeds this SDK, so deployment does not need this
source checkout, pip, Cargo, or a host rebuild. A Python interpreter is still
required. Native executable implementations may implement the same wire protocol.
For native source exports, production dependency closure and executable assembly,
see [separate Rust component packaging](RUST_PACKAGING.md).

## Install the versioned SDK

The Python SDK has a standard `codex-component-sdk` wheel, version `0.1.0`.
Install a locally built wheel into a Python 3.10+ environment:

```sh
python3 -m venv /tmp/codex-sdk-env
/tmp/codex-sdk-env/bin/python -m pip install /absolute/path/to/codex_component_sdk-0.1.0-py3-none-any.whl
/tmp/codex-sdk-env/bin/codex-component-sdk init /tmp/my-greeting --id custom.greeting
# Edit /tmp/my-greeting/plugin.py and codex-component.json.
/tmp/codex-sdk-env/bin/codex-component-sdk build /tmp/my-greeting --output /tmp/greeting-package
```

No `PYTHONPATH` or source checkout is needed after wheel installation. The
`python -m codex_component_sdk` entry point also works. Built plugin zipapps embed
the runtime and notices, so uninstalling the developer SDK does not prevent them
from running. Python itself remains a runtime dependency. SDK distribution
version `0.1.0`, manifest API version `1`, and component contract versions are
separate compatibility values; installing a newer wheel does not upgrade a
host's component contracts.

To build the wheel from an independent copy, set `SDK` to this directory and
choose a new output directory:

```sh
SDK_BUILD=$(mktemp -d)
cp "$SDK/pyproject.toml" "$SDK/README.md" "$SDK_BUILD/"
cp -R "$SDK/codex_component_sdk" "$SDK_BUILD/"
python3 -m pip wheel --no-deps --wheel-dir /tmp/codex-sdk-wheels "$SDK_BUILD"
```

The build uses setuptools 77+ and wheel 0.45+; runtime has no third-party Python
dependencies. The distribution contains only `codex_component_sdk`, including
LICENSE/NOTICE at their runtime package paths and in wheel license metadata.
The generated plugin preserves those notices internally and alongside its
manifest. The Rust source exporter, examples and acceptance helpers remain
separate source tools, not bundled Python runtime dependencies. This wheel is
local and unpublished; no package registry upload is required.

[sdk_distribution_acceptance.py](tests/sdk_distribution_acceptance.py) verifies
an outside-checkout wheel build, fresh-venv installation with `PYTHONPATH` unset,
custom template/static-asset packaging, source removal and SDK uninstall before
actual manager install/invoke/remove. It freezes a separate manager copy and
records its unchanged hash, all source/artifact inventories and command results:

```sh
python3 "$SDK/tests/sdk_distribution_acceptance.py" \
  --host /absolute/path/to/codex-component \
  --work-dir /tmp/new-sdk-distribution-acceptance
```

This distribution gate verifies Linux/Python 3.12. Other supported Python
versions and platforms still need conformance runs; the wheel's `py3-none-any`
tag describes its pure-Python packaging, not completed platform testing.

## Create and build outside the harness

Set `SDK` to the absolute path of this `component-sdk` directory:

```sh
export PYTHONPATH="$SDK"
python3 -m codex_component_sdk init /tmp/my-greeting --id custom.greeting
# Edit /tmp/my-greeting/plugin.py and codex-component.json.
python3 -m codex_component_sdk build /tmp/my-greeting --output /tmp/greeting-package
```

The output directory contains `codex-component.json`, executable `plugin.pyz`,
and SDK license notices. Install that directory with the host's `codex-component`
CLI. Tool/context/lifecycle/model bindings are captured for each new engine
session; existing sessions retain their bindings. Credential selection is captured
when the native `AuthManager` is created, so changing it requires a new manager.
Restarting Codex is sufficient for both cases. Build outputs must not
already exist. The builder includes Python modules and nonhidden files below
directories named `static`; use `importlib.resources` to read packaged assets.
It rejects symlinks and excludes hidden files and Python caches. Include your own
`LICENSE` and `NOTICE` in your project to distribute them beside the SDK notices.

The calculator example provides a real arithmetic tool and bounded context
contributor. The model fixture provides deterministic streaming text for offline
integration checks; it does not perform model inference. Copy either example to
your own source directory and build it with the same command. Tests intentionally
delete that independent source before running each built plugin:

```sh
python3 -m unittest discover -s "$SDK/tests" -p test_sdk.py -v
```

For host acceptance after its initial build:

```sh
python3 "$SDK/tests/acceptance.py" --host /absolute/path/to/codex-component
```

This creates an isolated Codex home, builds outside the harness source, deletes
the plugin source, installs and invokes the calculator and context components,
then removes the package. The retained JSON report includes the commands and
unchanged host SHA-256, modification time, and size. This checks the package host;
full engine behavior needs the separate harness integration checks.

Exercise the actual built CLI as well with:

```sh
python3 "$SDK/tests/harness_acceptance.py" \
  --host /absolute/path/to/codex-component \
  --codex /absolute/path/to/codex \
  --work-dir /tmp/new-harness-acceptance
```

This uses an independently built deterministic model plugin to request the
calculator through the real engine, contribute context, stream a reply, and
resume the persisted session. It then resets the model replacement and tests
the native transport against a loopback response fixture with the retained
history. Both executable hashes must remain unchanged. These checks exercise
the actual engine with simulated inference; they do not contact a live model.

## Component contracts

A project exports `app = Plugin()` from `plugin.py`. Register a handler with
`@app.handle(kind, name, method)`. It receives `(params, context)` and returns a
JSON value. `context.emit(value)` streams an event immediately.
`context.initialization` carries `plugin_id`, JSON `config`, and an absolute
`state_dir`. Startup and shutdown hooks receive that same initialization object.
Use `@app.on_start` and `@app.on_shutdown` for lifecycle hooks.

Long-lived handlers can call `context.watch_shutdown()` to obtain a
`threading.Event` that is set by a terminal shutdown frame, stdin EOF, or invalid
control input. One SDK reader owns stdin while the handler is active; handlers
must not read stdin themselves. On this event, stop admitting work, drain owned
resources, and return only after cleanup succeeds. Raise `PluginError` on a
failed cleanup so the host cannot mistake it for success. The SDK rejects a new
request in place of shutdown, including after cleanup. This is opt-in; ordinary
tool and model handlers retain their existing request/response lifecycle.

Each host invocation owns a new process and serves one request. In-memory state
is per invocation. Store durable state explicitly under `state_dir`, using atomic
file replacement and synchronization if concurrent calls share data. The host
owns dependencies, selection, configuration, and process cancellation. Shutdown
or stdin EOF triggers cleanup after the request, or signals an opted-in active
handler. Forced cancellation may kill the process
without cleanup, so shutdown hooks must not be the only persistence mechanism.

`codex-component launch` bounds initialization and initial request delivery using
the binding's startup timeout (currently 120 seconds), then permits the presentation
to run until it finishes or shutdown is requested. There is no fixed run-lifetime
cutoff. Normal result completion allows up to 210 seconds for plugin exit cleanup.
For an active launch, the first Ctrl+C sends a protocol shutdown while
continuing to read events and allows up to 210 seconds for cleanup. A second
Ctrl+C or an expired deadline forces termination and exits unsuccessfully with
an explicit warning that accepted writes may not have completed and durability
is unknown. Success requires the final result and a successful plugin exit.
The Rust `ComponentStream::shutdown_handle().request(grace)` API offers the same
bounded shutdown request. Keep the stream and its current `next()` future alive
while requesting cleanup; dropping either still forces termination. The first
request fixes the deadline, and further requests do not extend it. The binding's
original invocation deadline remains an upper bound for ordinary tool/model
invocations. Launch uses `ComponentBinding::start_service_stream` instead; its
startup deadline cannot shorten the later cleanup phase. A result begins its
bounded exit phase, and a later request cannot extend that exit deadline.
An interrupt during initialization, before the component is ready to receive a
shutdown request, forces termination and reports the same durability uncertainty.
Forced launch termination awaits the direct child's actual exit and reap before
the manager runtime exits, with a further two-second bound. Failure to confirm
reaping is reported explicitly. Rust callers that need startup cancellation can
use `ComponentBinding::start_stream`, obtain its `shutdown_handle`, then await
`ready()`. Keep that pending startup or stream read alive while awaiting
`force_and_reap()`; dropping the owner instead retains an immediate cancellation
fallback whose detached reap requires a live runtime. Reaping acknowledgement
covers the owned child. Unix process-group killing and Linux parent-death handling
do not make the host the reaper of arbitrary orphaned descendants; Windows still
does not provide descendant-tree termination here.

| Kind | Method | Parameters | Result |
| --- | --- | --- | --- |
| `tool` | `invoke` | `{call_id, name, arguments}` | `{text: string, success: bool}` |
| `context` | `contribute` | `{session_id, thread_id, turn_id}` | `{text: string}`, capped by host at 1000 rendered UTF-8 bytes |
| `lifecycle` | `event` | `{event, session_id, thread_id, turn_id, details}` | `{}` |
| `model_transport` | `model.stream` | `{thread_id, request: ResponsesApiRequest}` | `{}` after a `completed` event |
| `auth` | `auth.resolve` | `{}` | Tagged credential or safe error result, described below |
| `auth` | `auth.refresh` | `{reason: "unauthorized", previous_account_id: string or null}` | Tagged credential or safe error result |

The `default` model transport selection replaces native inference for the turn;
it must emit compatible Codex response items and streaming events. See
`examples/model-fixture/plugin.py` for text event ordering. Native tools retain
their host approval and cancellation behavior. External tool implementations
are trusted engine code and must not be installed on the assumption that this
process protocol creates a security sandbox.

Each model invocation receives the native `ResponsesApiRequest`, including full
history and available client metadata; persistent caches belong in `state_dir`.
Invalid model events and plugin failures fail the turn without falling back to
native inference. The initial external error contract does not encode specialized
provider errors such as context-window exhaustion, rate limits, or retry hints.
Model discovery, native auth policy/storage, compaction policy, realtime, and unary
memory summaries retain their native contracts. Credential acquisition can use
the separate selected adapter below. Compaction that calls the shared model
stream can use the selected transport, with native output validation retained.

## Credential provider contract

Declare kind `auth`, name `default`, and contract version `1`; explicitly select
the package with `codex-component ... select auth default PLUGIN_ID`. Register
handlers for `auth.resolve` and `auth.refresh`. Return one of these JSON shapes:

- `{type: "api_key", api_key: string}`
- `{type: "chatgpt", access_token: string, chatgpt_account_id: string, chatgpt_plan_type: string or null}`
- `{type: "error", kind: "transient"}` or `{type: "error", kind: "permanent"}`

Secrets must be nonempty and at most 64 KiB; account IDs are capped at 1024 bytes,
and optional plan strings at 128 bytes. The native auth manager validates login
restrictions and commits accepted credentials. A refresh with a previous account
ID cannot switch to a different account. Workload identity selection rejects an
external credential replacement. OAuth/login interfaces, keyrings, authentication
policy, and the full provider implementation are not replaced by these methods.

Each resolution or refresh starts a fresh process. Keep the refreshed credential
current in the provider's explicit durable storage; a later `auth.resolve` must
not return a superseded token from process-local memory. Do not emit credential
events, put secrets in errors, or log request/credential payloads. Use the tagged
safe error result for provider failures; do not raise `PluginError` containing
token or account details. The host suppresses authentication child stderr,
replaces transport/parser errors with static diagnostics, and disables generic
CLI `call auth ...` output. Native credential Debug formatting is redacted.
These precautions do not make arbitrary plugin-authored files or logs safe.
Current source/test status is tracked in [VALIDATION.md](../VALIDATION.md).

## Manifest and wire compatibility

`codex-component.json` declares `api_version: 1`, unique `id`, semantic `version`,
relative package `entrypoint`, string `args`, `dependencies` mapping plugin IDs to
semantic version requirements, and `components`. Each component declares `kind`,
`name`, its supported integer `contract_version`, and JSON `metadata`. Native
package assembly accepts `--contract-version` (default `1`); use the version
required by the selected host adapter. Tool metadata contains
`description` and JSON `input_schema`. Protocol version and component contract
version are separate: a matching process protocol alone does not authorize a
host adapter to accept an incompatible component. Dependency validation and
component replacement policy belong to the host, not this SDK.

Wire transport is UTF-8 newline-delimited JSON on stdin/stdout, limited to 4 MiB
per frame including the newline. The host sends `initialize` with `api_version`,
`plugin_id`, `config`, and `state_dir`; the plugin responds `ready` with its
`api_version`. The host sends `request` with integer `id`, `component: {kind,name}`,
`method`, and `params`. The plugin sends zero or more `{type:"event",id,event}`
frames, then `{type:"result",id,result}` or `{type:"error",id,message}`. The host
sends `{type:"shutdown"}` or closes stdin. Diagnostics go to stderr.

Unexpected handler exception details are withheld from both protocol and SDK
diagnostics. Raise `PluginError("public explanation")` for intentional public
errors, bounded to 128 UTF-8 bytes. Plugin-authored logging remains the developer's
responsibility; do not log credentials or entire model requests. Protocol errors
terminate with exit status 2. Timeouts and cancellation are enforced by the host.
