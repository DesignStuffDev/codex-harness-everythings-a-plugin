# Desktop presentation component

An independently built graphical client for this Codex harness. It provides a
responsive task sidebar, conversation view, text composer, streamed assistant
and command output, approval decisions, interruption, and saved task recovery.
The HTML, CSS, JavaScript and Python gateway ship inside this plugin's zipapp.
The host does not bundle the interface. Removing the plugin leaves CLI and
headless Codex available.

This is our own interface, not the official desktop application's source. The
plugin is one presentation boundary; it does not claim that wrapping app-server
alone compartmentalizes the rest of the engine.

## Build separately and install

Copy this directory outside the harness source tree. With Python 3.10+ and the
component SDK on `PYTHONPATH`, build and install it using the already-built host:

```sh
python3 -m codex_component_sdk build /absolute/desktop-project --output /absolute/desktop-package
codex-component --codex-home /absolute/codex-home install /absolute/desktop-package
codex-component --codex-home /absolute/codex-home launch desktop --codex-bin /absolute/codex
```

Open the private loopback URL from the `presentation/ready` event. Keep that CLI
process running. The first Ctrl+C requests graceful shutdown and allows up to
210 seconds for the presentation, app-server and selected storage to drain.
Initialization and delivery of the launch request are bounded by the binding's
startup timeout (currently 120 seconds). Once ready, the presentation has no
fixed run-lifetime deadline. Normal completion also allows up to 210 seconds
for plugin exit cleanup; the startup timeout cannot cut that cleanup short.
A second Ctrl+C or an expired deadline forces termination and reports that
accepted writes may have an unknown outcome. Use the
rebuilt harness binary to exercise installed engine component adapters. The
selected Codex home supplies model credentials, configuration, installed
components, and durable sessions. A cloud loopback URL requires a supported
private preview/tunnel to access from another machine; this plugin does not
publish a service or bind to a public interface.

## Presentation contract v1

Manifest capability: `kind = presentation`, `name = desktop`,
`contract_version = 1`. The component implements `launch`:

```json
{
  "codex_bin": "/absolute/executable/codex",
  "codex_home": "/absolute/codex-home",
  "cwd": "/absolute/workspace",
  "host": "127.0.0.1",
  "port": 0
}
```

`codex_bin` is required. `codex_home` and `cwd`, when supplied, must be existing
absolute directories and are forwarded explicitly to the app-server process;
otherwise its environment and working directory are inherited. `host` defaults
to `127.0.0.1` and no other address is accepted. `port` defaults to an ephemeral
port. The host CLI supplies its selected home and invoking working directory.
No credentials travel in component request parameters.

Startup launches the supplied executable with `app-server`, completes the real
stdio `initialize` / `initialized` handshake, starts a local HTTP gateway, and
emits `{"method":"presentation/ready","params":{"url":"…","app_server_pid":123,
"gateway_pid":124}}`. The request remains active for the UI lifetime. The SDK's
`context.watch_shutdown()` consumes a terminal `{"type":"shutdown"}` frame or
stdin EOF while this handler is running. The gateway stops accepting app-server
requests, closes its stdin and allows 200 seconds for cleanup. It returns the
launch payload only after the child exits successfully. Unexpected disconnects,
nonzero exits and failed cleanup produce an error rather than a success result.

After the graceful budget expires, the gateway escalates to SIGTERM and then
SIGKILL, allowing two seconds for each, and reports an unknown write outcome even
if termination succeeds. Direct SIGTERM or SIGINT to the plugin invokes the same
cleanup hook; on POSIX, signals to the entire process group also reach app-server
and cannot guarantee graceful completion. The manager owns the outer deadline
and forced process cleanup. Abrupt termination does not depend on a final
state-save hook, and an interrupted accepted write must not be blindly replayed.

## Client service and ownership

The adapter uses the pinned upstream schemas under
`codex-rs/app-server-protocol/schema/typescript/`:

| Behavior | App-server inputs and outputs |
| --- | --- |
| Startup | `initialize {clientInfo:{name,title,version}}`, then `initialized` notification |
| Saved task list | `thread/list {limit,cursor?,sortKey:"updated_at"}` → `{data,nextCursor}` |
| Create | `thread/start {cwd?,ephemeral:false}` → `{thread}` |
| Recover | `thread/resume {threadId,excludeTurns:true}` → `{thread}`; `thread/read` also remains available through the adapter |
| History | `thread/turns/list {threadId,cursor?,limit,sortDirection:"desc",itemsView:"full"}`; incomplete item pages use `thread/items/list {threadId,turnId,cursor?,limit}` |
| User input | `turn/start {threadId,input:[{type:"text",text,text_elements:[]}]}` → `{turn}` |
| Live output | `turn/started`, `turn/completed`, `item/started`, `item/completed`, `item/agentMessage/delta`, `item/commandExecution/outputDelta`, `item/mcpToolCall/progress` |
| Approvals | Server request ID retained; command/file requests return `{decision:"accept"\|"decline"\|"cancel"}`; permissions return `{permissions,scope:"turn"}` |
| Questions | `item/tool/requestUserInput` → `{answers:{questionId:{answers:[text]}}}` |
| Cancellation | `turn/interrupt {threadId,turnId}`; completion determines final UI state |

Interrupted and failed turns receive explicit status text. If a tool has no
final item event, its card preserves output and identifies the interruption or
missing final status rather than claiming success. Completed earlier turns and
their tool results remain intact.

The HTTP service exposes authenticated `POST /rpc`, `POST /reply`, `GET /status`
and `GET /events` (SSE). It forwards only the listed thread and turn RPCs. Server
requests remain pending until a correlated reply or `serverRequest/resolved`.
Unknown interaction types require explicit cancellation; the UI never
automatically approves them. The gateway maintains a bounded transient replay
buffer and pending-request set, with SSE IDs and replay after reconnect. A
buffer gap triggers an authoritative history reload.

App-server owns execution, cancellation semantics, model access, tool progress,
approval policy, sandbox enforcement and durable history. The gateway owns child
lifetime, transient RPC correlation, notifications and pending interactions.
The browser owns rendering, unsent text, and its last selected task ID. It uses
text rendering, not HTML interpretation of model or command output. Session
history survives plugin restart in the selected Codex home; an ephemeral port
can change browser origin, so select the saved task again from the sidebar.

The listener enforces exact Host/Origin, a random private bearer token, a
same-origin content policy, bounded request frames, and no access logs containing
tokens. The token begins in a URL fragment and is moved into session storage.
The browser event channel sends it in a local query parameter because native
EventSource cannot set an Authorization header. Do not share the private URL.

## Validation and current scope

`python3 -m unittest discover -s component-sdk/tests -p test_desktop.py -v`
exercises a clearly labeled subprocess app-server fixture: stream delivery,
approval round trip, cancellation, history across gateway restart, explicit home
and cwd, rejected cross-origin requests, separate zipapp build/resources and
child cleanup. A real subprocess delays EOF cleanup beyond three seconds and
must persist its completion marker before the packaged plugin acknowledges
shutdown. Controlled subprocess waits verify bounded escalation and unknown
write errors; a killed app-server must produce an error and a nonzero plugin
exit. A second-turn regression checks unique identities and preservation
of the first completed turn after interruption. It is transport evidence, not
proof of real model execution.

`python3 component-sdk/tests/test_desktop.py --serve` starts that same test fixture
for browser interaction checks. Production has no fixture fallback and always
requires `codex_bin`. Real harness/model integration evidence is recorded in the
project verification report. In-app Browser manual checks require that tool to
be available; headless browser checks must be described separately.

Current UI scope is text tasks, common command/file/permission approvals and
user-input questions. It does not implement account login, voice, realtime,
image attachments, hosted MCP app widgets, native biometric verification,
project/worktree management or all app-server methods. Configure authentication
through the existing Codex CLI. The gateway is an initial Python standard-library
local client, not a public multiuser application server.
