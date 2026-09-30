# Persistent component protocol

This document describes the implemented multiplexed session protocol, version 1, in
[`codex-component-host`](../codex-rs/component-host/src/lib.rs). It supports multiple
concurrent unary calls to one component process. The process can own live writers,
reservations and other state across calls. The thread-store adapter uses this transport.
The attachment-store adapter currently uses the one-invocation transport with staged
files for large blobs.

**The Python SDK implements the separate, one-invocation protocol only.** Its `Plugin`
handlers and zipapp examples do not implement this persistent protocol. A persistent
plugin can use the Rust server API below, or implement this wire contract in another
language. Neither persistent streaming events nor component-to-host dependency calls are
implemented by this version. See [the SDK README](README.md) for one-invocation plugins
and [the model-services plan](../MODEL_COMPONENT_PLAN.md) for proposed future work.

## Process and handshake

`ComponentBinding::connect()` starts the installed executable directly with its declared
arguments, without a shell. Its working directory is the immutable installed package;
`state_dir` is the plugin's persistent writable state directory. On Windows a `.pyz`
entrypoint is launched through Python. The host creates `state_dir` before connecting.
Stdin and stdout carry protocol frames exclusively. Stderr is inherited, except for
`auth` and `attachment_store` bindings, whose stderr is discarded. Configuration and
protocol payloads can contain sensitive data and must not be logged indiscriminately.

Each frame is one UTF-8 JSON object followed by LF. Handshake frames use the component
API version independently of the multiplexed session version:

Host to component:

```json
{"type":"initialize","api_version":1,"plugin_id":"example.store","config":{},"state_dir":"/absolute/component-state","session":{"mode":"multiplexed","version":1}}
```

Component to host:

```json
{"type":"ready","api_version":1,"session":{"mode":"multiplexed","version":1}}
```

`config` must be a JSON object. Both peers require the exact session object shown above;
adding fields inside `session` is not compatible with this version. The host requires
the explicit acknowledgement before admitting calls. A plugin that only returns the
one-invocation `ready` frame fails with “component does not support multiplexed session
version 1”. There is no implicit downgrade or native implementation fallback.

The binding's `timeout_ms` bounds initialization waits. Cancellation or failure while
connecting terminates the child. Unix termination also targets its process group and
reaps the direct child; Windows currently terminates the direct child only. A Windows
plugin must not depend on this runtime to terminate its descendants.

On Linux, every component spawn also arms `PR_SET_PDEATHSIG` with `SIGKILL`, with a
`getppid` race check after arming it. This terminates a blocked component when its owning
engine is killed, even though the component has a separate process group. Linux ties
this setting to the **creating OS thread**, so the spawning thread must outlive the
child; the current engine uses long-lived runtime workers. Moving an async future does
not change the kernel's parent-thread association. Credential-changing executable
transitions can clear the setting. This is forced termination, with potentially unknown
write outcomes, not a graceful durability guarantee. Arbitrary escaped grandchildren
are not covered, and macOS/Windows do not gain Linux parent-death protection.

## Message framing and direction

After the handshake, every request and successful result has a start frame, zero or more
chunk frames, and an end frame. The body is a single serialized JSON value: request
parameters or the result value. It is not the start frame and has no extra envelope added
by the transport. Normal JSON values require at least one nonempty chunk.

| Frame | Direction | Required fields after `type` |
| --- | --- | --- |
| `request_start` | Host to component | `id`, `component: {kind, name}`, `method`, `is_control`, `bytes` |
| `result_start` | Component to host | `id`, `bytes` |
| `chunk` | Direction of the corresponding start | `id`, `index`, `data` |
| `end` | Direction of the corresponding start | `id`, `chunks` |
| `error` | Component to host | `id`, `message` |
| `shutdown` | Host to component | None |
| `shutdown_complete` | Component to host | None |

Post-handshake frames and the `component` object reject unknown fields. `id`, `bytes`,
`index` and `chunks` are unsigned 64-bit integers. IDs are positive. The host allocates
IDs sequentially from 1 for both ordinary and cleanup calls; it never reuses an ID in a
session. A response echoes its request ID. Responses can arrive out of order. There is
only one request-origin direction in version 1: a component cannot send `request_start`
to request a host service.

`bytes` is the exact length of the body as serialized UTF-8 JSON, before base64 encoding.
Each `chunk.data` uses standard padded base64. Chunk indexes start at 0 and must be
contiguous within an ID. `end.chunks` must equal the number of chunks received, and their
decoded lengths must total `bytes`. IDs can interleave, but every chunk needs a preceding
start. A second start, a repeated completed response, unknown response ID, wrong-direction
frame, truncated frame or unfinished body at EOF is a protocol error.

For example, a call and response whose parameters and result are both `{}` use these
lines. `e30=` decodes to the two UTF-8 bytes of `{}`.

Host to component:

```json
{"type":"request_start","id":1,"component":{"kind":"example","name":"default"},"method":"example/inspect","is_control":false,"bytes":2}
{"type":"chunk","id":1,"index":0,"data":"e30="}
{"type":"end","id":1,"chunks":1}
```

Component to host:

```json
{"type":"result_start","id":1,"bytes":2}
{"type":"chunk","id":1,"index":0,"data":"e30="}
{"type":"end","id":1,"chunks":1}
```

Instead of a result the component can return:

```json
{"type":"error","id":1,"message":"unsupported operation"}
```

An error can also terminate an already started result for that ID; the partial result
is discarded. An error cannot replace a request body. A well-formed correlated error
fails that call without automatically closing the session. Service-level failures should
usually use the service's typed JSON result envelope; transport `error` is reserved for
malformed calls or handler/transport failures. There are no implicit retries.

## Limits and buffering

| Resource | Implemented bound |
| --- | --- |
| Physical frame, including terminating LF | 4 MiB |
| Decoded bytes in one nonempty chunk | 192 KiB |
| Encoded `chunk.data` | 262,144 bytes |
| Incomplete logical bodies per reader | 64 |
| Missing lower IDs tracked while accepting out-of-order messages | 64 |
| Host ordinary calls awaiting a response | 32 |
| Host reserved or outstanding cleanup calls | 32 |
| Host ordinary / control writer queue | 32 entries each |
| Server active ordinary / control requests | 32 each |
| Server ordinary / control response queue | 32 entries each |
| Server incoming request queue | 64 entries |
| Writer's deferred control queue | 64 entries |
| `component.kind` / `component.name` | 1–128 UTF-8 bytes each |
| `method` | 1–256 UTF-8 bytes |
| Control request body and response emitted through the control lane | 64 KiB serialized JSON each |

The bounds are enforced, not negotiation hints. The reader tracks a maximum ID and a
bounded set of missing IDs instead of retaining an unbounded history. Consequently an ID
jump that would make the total missing-ID set exceed 64 is rejected even if few bodies
are active. The control-response size bound is enforced by the Rust writer's control
lane; `result_start` has no control flag, so the reader does not apply that smaller cap
to a third-party peer's response based on its request class.
The final encoded `error` frame must fit the physical frame limit, including escaping,
metadata and LF; the message string alone cannot consume the whole limit.

There is intentionally no transport-wide size cap on an ordinary logical JSON body:
native history can exceed one physical frame. Bodies are serialized and reassembled in
private temporary files, with bounded chunk buffers; serialization and JSON decoding use
blocking worker tasks. Disk exhaustion and I/O errors are reported. This is not a fixed
memory or disk quota: callers and handlers still own full `serde_json::Value` values when
encoding or decoding, and an ordinary body can consume large temporary-file space.
Service contracts may impose tighter limits. A control body stays small so release
operations can progress while an ordinary body is being transferred.

Malformed JSON in a physical frame fails the connection. A non-JSON logical request
body fails the server reader and connection. A non-JSON logical result body is decoded
in the call or cleanup waiter and fails that waiter; an abandoned result can remain
unparsed. The framing layer validates byte counts and ordering independently of logical
JSON decoding.

The writer prioritizes eligible controls between ordinary physical frames. It cannot
interrupt a frame already being written, and it finishes one eligible control message
before continuing an ordinary message. Ordinary messages retain writer-queue order;
concurrent handler completion and control priority can reorder responses. Application
ordering, write durability and lease semantics belong to the service contract.

## Cancellation, cleanup and outcome ownership

Cancelling a persistent call's waiting future is different from cancelling a
one-invocation `ComponentStream`. Once admitted, a persistent call remains owned by the
session. Its pending entry and capacity reservation stay alive until a response or a
terminal connection failure. The reader drains a response even when its original waiter
has disappeared. It can discard an abandoned response body without decoding it.

`timeout_ms` bounds each call's wait, including admission and result decoding. A timeout
does not kill a healthy shared process or cancel an accepted write. An operation that
was admitted may still commit after the waiter times out. A disconnect or crash can
likewise leave its outcome unknown. There is no automatic reconnect, resubmission,
deduplication cache or replay; the service must define explicit recovery. A returned
result means the component answered, not that the transport itself made storage durable.

`call_with_cleanup` reserves ordinary capacity, cleanup capacity and the cleanup writer
slot before admitting the paired operation. Cancellation before admission performs no
operation. After admission, an owned `DeferredControl` ensures cleanup is queued even if
the call fails, times out, or its waiter is dropped. On success the application receives
the guard alongside the result and retains it for the lifetime of its resource.

Dropping the guard enqueues exactly one control call without blocking. `release().await`
enqueues it and waits for an acknowledgement; timing out stops waiting but retains the
accepted cleanup. A send fence prevents cleanup overtaking its paired request: the
paired request's complete body and end frame must be flushed first. This is a delivery
ordering guarantee, not a guarantee that the service has finished handling preparation.
The service must handle release concurrent with preparation, including released-lease
tombstones when necessary. It must also release leases on disconnect.

Control handlers must be able to run while ordinary handlers are blocked. For example,
deleting a source thread may await an active fork reservation; its release must not sit
behind that deletion in one serial handler loop. The protocol supplies capacity and
priority, while the service owns concurrent dispatch and domain ordering.

## Rust client API

The public entrypoints are exported from `codex_component_host`:

```rust,ignore
impl ComponentBinding {
    async fn connect(&self) -> anyhow::Result<ComponentSession>;
}

impl ComponentSession { // Clone; clones share one process and admission state.
    async fn call(&self, method: &str, params: serde_json::Value)
        -> anyhow::Result<serde_json::Value>;

    async fn call_with_cleanup(
        &self,
        method: &str,
        params: serde_json::Value,
        cleanup_method: &str,
        cleanup_params: serde_json::Value,
    ) -> anyhow::Result<(serde_json::Value, DeferredControl)>;

    fn begin_close(&self);
    async fn close(&self) -> anyhow::Result<()>;
}

impl DeferredControl {
    async fn release(self) -> anyhow::Result<()>;
}
```

`release` checks transport success and decodes the response, but does not interpret a
service-specific result envelope. Choose a cleanup contract whose acknowledged result
has the required resource-release semantics.

Release all application-held cleanup guards before calling `close`. Closing stops new
ordinary admission, drains admitted calls and cleanup, sends `shutdown`, then requires
`shutdown_complete` and a successful process exit. The binding's `timeout_ms` bounds this
entire drain-and-exit phase. Forced shutdown reports unknown outcomes and terminates the
process. Error cleanup has an additional two-second budget to kill and await the direct
child and join the asynchronous reader/writer tasks. Startup rejection, handshake timeout
and connection failures are returned after this cleanup; an exceeded cleanup budget or
failed supervisor explicitly reports unconfirmed reaping. A forced exit never establishes
accepted-write durability, even when the child was successfully reaped. This acknowledgement
covers the direct child, not arbitrary orphaned descendants, service background work or
the legacy codec's detached blocking serialization/decoding tasks.

Dropping `connect` during initialization signals an independent startup owner that keeps
the child guard through cancellation and performs the same bounded kill-and-wait cleanup.
The absent waiter receives no acknowledgement. The Tokio runtime must remain alive for
that cleanup; abruptly dropping the runtime still uses the unconfirmed guard fallback.
On Linux, the OS thread that spawns the child must also outlive it because parent-death
termination follows the creating thread.

Dropping the final session handle requests the same supervised drain; a cleanup
guard itself retains a session reference. A caller cancelling `close` does not cancel the
supervisor's shutdown work. A closed or failed session cannot be reused.
`begin_close` starts that same shutdown synchronously without waiting, fences every
retained session clone, and is idempotent. Lifecycle owners can use it during cancellation
or `Drop`, then await `close` when an async shutdown path is available.

## Rust server API and shutdown

```rust,ignore
impl ComponentServer {
    async fn stdio() -> anyhow::Result<Self>;
    fn initialization(&self) -> &SessionInitialization;
    async fn next(&mut self) -> anyhow::Result<Option<ComponentServerRequest>>;
    fn was_disconnected(&self) -> bool;
    async fn finish(self) -> anyhow::Result<()>;
}

struct SessionInitialization {
    api_version: u32,
    plugin_id: String,
    config: serde_json::Value,
    state_dir: std::path::PathBuf,
}

struct ComponentServerRequest {
    id: u64,
    component: SessionComponent, // Public kind: String and name: String.
    method: String,
    params: serde_json::Value,
    is_control: bool,
    // Private transport ownership fields.
}

impl ComponentServerRequest {
    async fn respond(self, result: Result<serde_json::Value, String>)
        -> anyhow::Result<()>;
}
```

The fields shown above are public. `stdio` validates the handshake and owns dedicated
reader/writer tasks. `next` is cancellation-safe, including when used in `tokio::select!`
alongside completed handler tasks. Parsing buffers, partial bodies and pending I/O belong
to the reader task, not the temporary `next` future. Services should spawn and retain
ordinary handler tasks so they continue consuming incoming control requests.

`respond` transfers a response to the writer; it does not wait for a peer acknowledgement.
Dropping an unanswered request queues an error saying the handler abandoned the request
and its outcome is unknown. Services should avoid raw payloads in diagnostics.

`next` returns `None` for an explicit shutdown or clean stdin EOF, and an error for a
protocol or I/O failure. On EOF/failure, `was_disconnected` is true. In every case the
service should stop new work, release connection-owned leases, drain owned handlers and
flush its durable writers before `finish`. `finish` rejects outstanding handler ownership,
flushes queued responses, and sends `shutdown_complete` only after explicit shutdown.
EOF is not a successful graceful shutdown; `finish` returns an error in that case.

Neither shutdown frame may interrupt a partially assembled body. The host accepts
`shutdown_complete` only after it requested shutdown and all accepted requests completed;
it then checks the child's exit status. Dropping a server aborts its transport tasks; it
does not substitute for the service's resource-release and durability steps.

## Implementation and verification references

- [Client admission and cleanup](../codex-rs/component-host/src/session.rs)
- [Process supervision and response routing](../codex-rs/component-host/src/session_supervisor.rs)
- [Framing, spooling and writer priority](../codex-rs/component-host/src/session_wire.rs)
- [Server lifecycle](../codex-rs/component-host/src/session_server.rs)
- [Real-process session tests](../codex-rs/component-host/src/session_tests.rs),
  [wire tests](../codex-rs/component-host/src/session_wire_tests.rs), and
  [server cancellation tests](../codex-rs/component-host/src/session_server_tests.rs)
- [Native thread-store plugin](../codex-rs/thread-store-local-plugin/src/main.rs) and
  [thread-store service DTOs](../codex-rs/thread-store-component/src/contract.rs)
