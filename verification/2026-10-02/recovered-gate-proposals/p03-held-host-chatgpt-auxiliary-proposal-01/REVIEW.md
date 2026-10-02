# One bounded auxiliary ChatGPT CONNECT during held-host acceptance

Staged only. No fixture, host, Git command, listener or test was executed, and
no checkout, configuration, PATH, DNS, proxy setting or native binary changed.
Original failed attempts01 and02 remain preserved.

## Source ordering and attribution

Attempt02 fixes the real Git observation pin and records a trusted Git process.
It records GitHub CONNECT, then the rejected ChatGPT authority approximately
33 ms later, followed by target held readiness before forced fixture cleanup.
No Git502 was sent. It remains a failure before complete case readiness.

The most direct expected startup source path is the featured-plugin warmup:

1. `codex-rs/exec/src/lib.rs:1023` starts `InProcessAppServerClient`.
2. `app-server/src/in_process.rs:608` enables current-config plugin startup
   tasks; `app-server/src/message_processor.rs:557–581` dispatches them.
3. `core-plugins/src/manager.rs:2877–2885` starts curated repository sync first.
   The same method independently spawns a featured-plugin cache warmup at
   `2985–2992`; it does not wait for the curated worker to finish.
4. `manager.rs:1936–1956` calls the legacy featured-plugin endpoint whenever
   plugins are enabled and that cache is cold.
5. `remote_legacy.rs:123–151` requests the ChatGPT-base `/plugins/featured`
   endpoint with a 10-second request timeout. ChatGPT authentication is optional;
   a non-ChatGPT auth mode does not skip the request.

The curated archive fallback has different ordering. `startup_sync.rs:159–242`
first waits for Git to fail, then waits for GitHub API failure, checks admission
and local snapshot state, and only then tries the ChatGPT export archive. That
sequence is inconsistent with the same attempt's still-held Git and zero502
at the recorded time. This conclusion concerns the same sync attempt only.

Selected model discovery is also a weaker explanation here: the fixture gives
the custom provider an explicit loopback base URL. The native model-catalog
rewrite to ChatGPT (`model-provider/src/models_endpoint.rs:112–120`) requires
the provider base URL and catalog URL both to be absent. Authenticated remote
plugin recommendations explicitly require a Codex-backend auth mode, unlike
the featured warmup. These source facts narrow the possibilities; they do not
establish the actual HTTP path behind the observed CONNECT.

**Runtime attribution remains limited:** TLS never starts, so the proxy sees
only authority, not request path or native caller identity. The report must say
“one auxiliary ChatGPT connection, consistent with featured-plugin warmup”,
not “verified featured-plugin request” or “verified archive fallback”.

## Proposed bounded correction

The candidate adds a distinct branch for exact `CONNECT chatgpt.com:443` only.
It admits at most **one auxiliary connection per case**, matching the one
observed connection and the independent featured warmup. Other authorities,
method mismatches, trailing request bytes, post-stop arrivals and a second
auxiliary connection still fail. A count failure must be diagnosed before any
later revision; do not automatically increase the allowance.

The auxiliary branch sends no bytes and forwards nothing. It holds the
connection until peer EOF, fixture failure cleanup or its own 270-second
absolute safety deadline (existing READY60 + OUTER210). That safety limit does
not extend host acceptance: the original 200-second graceful requirement,
210-second failure rescue and one-second post-exit EOF observation tail remain.

Auxiliary counters/events are separate from target Git/API counters/events.
The auxiliary branch cannot set target readiness, trusted-Git observation,
Git failure count or the stop trigger. Existing Git and API authority checks
and the required real-Git502 before API fallback remain unchanged. All original
`require(...)` assertions were statically verified present and unchanged.

Before a case can pass, both target and auxiliary held counts must be zero,
and auxiliary accepted count must equal explicitly observed peer EOF count.
Closing a fixture socket is never counted as peer EOF. Early failure and
forced cleanup remain failed evidence. The proxy report separately exposes
auxiliary accepted/held/EOF counts and `auxiliary_request_path_attributed=false`.
The existing `whole_host_clean=false` limitation remains unchanged.

Only `HeldProxy` and the existing post-host-exit EOF checks in `run_case` differ.
The model fixture, child environment, Git sampler, executable observer,
subreaper, case matrix, native receipt requirements, exit-code expectations,
drain policy and other top-level fixture code remain identical.

## Artifacts and validation status

- `proposed/held_production_host.py`: candidate fixture.
- `auxiliary-hold.patch`: exact diff against attempt02 diagnostic fixture.
- `preimage.py`: preserved exact input.
- `STATIC_REVIEW.json`: AST comparison and unchanged original assertions.
- `proposed/test_auxiliary_policy.py`: seven staged pure fixture-contract tests
  covering isolated readiness, count bound, post-stop rejection, forced release,
  unexpected bytes/authority, and unchanged Git502 behavior. They construct
  inert objects and fake connections; even when run, they will not establish
  actual host cleanup or native ownership.

Only syntax/AST checks and read-only source/evidence inspection were performed.
Root must review and, if adopted for acceptance, run those fixture checks and a
new uniquely named real-host attempt. Do not promote these staged results to
runtime proof or reinterpret attempt02 as passing.
