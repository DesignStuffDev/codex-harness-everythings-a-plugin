# MCP transport custody proposal

Recovery-only source proposal. No checkout, dependency checkout, lockfile, cache, or Rust command was changed or executed. Rust compilation, formatting and test execution remain unverified.

Apply over the complete frozen upper MCP session custody unit and its ownership-retirement follow-on. Existing-file preimages bind the exact authored base; upper-derived codex-mcp files use their explicit overlay provenance. The only workspace dependency addition is `codex-async-utils` in rmcp-client; lockfile ownership remains with the root reviewer.

## Caller contract

`codex_rmcp_client::RmcpClient::shutdown()` returns `Result<McpShutdownConfirmation, McpShutdownFailure>`. `Confirmed` records successful exact observation of the instrumented service and transport handles, and local direct-child/stderr ownership. It does not prove descendant process-group emptiness. `Unconfirmed` is a normal operation result with incomplete lifecycle evidence, not an automatic host operation error. Known service, transport, process, task or closed-admission failures have fixed payload-free classifications; the original results remain privately owned.

The codex-mcp adaptation explicitly preserves confirmation in connection ownership and exposes a current-snapshot receipt getter. It does not establish a full historical-session or process-final aggregate. Whole-host clean promotion remains blocked until that aggregate and the remaining upstream evidence are implemented and verified.

## Ownership and retirement

Construction and handshake observers never own the sole cleanup task. Admission guards close late generations after an observer disappears, including the interval around OAuth refresh. A closed client rejects new reservations and attaches already-reserved late generations before signaling them. Concrete transports move into independently registered ownership before their close task starts. Local process ownership independently registers the exact child and stderr reader before worker start; its private publication hook also places launch-error children in the originating client roster.

Client, transport and process registries keep pending and failed ownership reachable. HTTP `Ok(Unconfirmed)` can retire its generic task entry, but independent client and transport rosters still hold the same result and concrete ownership. Expected constructor/handshake operation errors use a separate successful-task-completion wrapper; resource obligations are observed independently. Successful roster entries retire only after the exact join, with destruction outside mutexes. A bounded rotating sweep observes up to 32 completed entries per roster on later admission. Idle unobserved final tasks remain retained until another observer/sweep; no detached cleanup reaper was added.

Stderr read/encoding errors are optional diagnostics after its reader is closed. Their original errors stay with referenced owners; the bounded local receipt records a count. These errors alone do not retain a fully cleaned process in the global registry. Framing, signaling, reaping and task errors remain lifecycle failures.

## Limits requiring follow-on work

- Pinned RMCP discards transport close errors and its private HTTP worker result. HTTP workers also have all-exit SSE drain and remote DELETE outcome gaps. This proposal reports HTTP cleanup as `Unconfirmed`; it does not infer clean completion from the outer join. The approved additive pinned-SDK follow-on must provide observed worker completion and all-exit local cleanup, separately recording optional remote DELETE.
- Cancelling an ordinary service shutdown observer preserves the concrete RMCP service join. Cancelling the primary service-owning worker or destroying its runtime can still drop the private handle inside upstream `RunningService::waiting()`. The outer task failure remains retained and cannot classify as clean, but recovery of that private handle requires an upstream safe observation API.
- Local ownership proves direct-child reaping and stderr/framing closure. Unix group signals and Windows job termination do not prove descendant emptiness. The local receipt explicitly labels process-group completion `unconfirmed`.
- Runtime-unavailable or cancelled cleanup retains registered local ownership, but this proposal adds no process-wide recovery/drain API for those owners.
- A panic inside one sequential connection shutdown stops later entries in that snapshot task. Its immutable task failure prevents a clean result; independent lower registries keep their resources reachable, but that path does not finish every connection observation.
- Executor-backed process/transport completion remains `Unconfirmed` beyond its observed termination acknowledgment.

## Review evidence

New sibling tests cover real RMCP peer requests and gated service close, abandoned constructor/publication orderings, concurrent/repeated/cancelled observations, exact transport errors, primary-task cancellation, local-child retention, diagnostic stderr, late roster attachment, typed unconfirmed evidence and successful-entry pruning. Existing lower shutdown test callers consume the new result. These are authored tests, not executed evidence.

Pinned SDK inspected read-only: revision `3e636cab26c013eca5131103c03d20237f12c4df` at `/workspace/toolchains/cargo/git/checkouts/rust-sdk-773cd6d57c4837f3/3e636ca`.

| File | SHA-256 |
| --- | --- |
| `crates/rmcp/src/service.rs` | `9d911d04244112b690bf2f5c5cd1f8d9292da3b645b9f1a5493fb713f7b709b7` |
| `crates/rmcp/src/transport/worker.rs` | `efc23380046726fb20daea8ad868ca38825155f9b6ddb1ec3fe51644632aac76` |
| `crates/rmcp/src/transport/streamable_http_client.rs` | `a4b7793c7317de592d4703407c241f42ca139c0a19c6db99e9d33dc664aa6dcb` |
| `LICENSE` | `0382b0057770ca05e9c350a50aa3b1c1fea84da0bc81d723bf00b9aa841be58a` |

No upstream files or license text were copied into this proposal.
