# All-mode independent Git acknowledgment proposal

Staged only; no tests or runtime executed by the author. This is an R-only fixture proposal based on diagnostic fixture `281173ec49467ed291b5212cd54837b5a2e72b4b3489d17e66034549c460fe1c`, with its exact source and observer retained under `preimage/`. No checkout/native edits were made.

The original Git-mode fixture allowed its internal sampler to enable shutdown before the separate observer pinned Git executable bytes. In the later failed matrix EOF case, the observer saw the same sampled generation but recorded no executable path/token. This does not establish a different executable or a zombie state. The earlier single diagnostic independently retained the exact stable Git digest and remains a passing single-case diagnostic; neither failed matrix is repaired.

## Narrow change

Both Git and fallback modes now require the independent observer acknowledgment before target transport readiness. The wait uses the same existing CONNECT deadline; the overall original readiness deadline also remains active. Fallback still requires that acknowledgment before sending the original real Git 502. Every case supplies the observer's four acknowledgment flags and validates the final sidecar, root/generation binding, executable digest and ancestry observation. The observer's case-id allowlist expands to the eight existing matrix cases; its sampling, descriptor hashing, process ownership and exit behavior are otherwise unchanged. Its new hash is pinned by the fixture and INPUTS.json.

All 89 original require() calls remain AST-identical, all existing shutdown/readiness constants and strict-runner drain behavior remain intact, and the original 23 pure tests remain byte-for-byte preserved. Eight additional pure test methods are proposed. Parsing source is not execution evidence.

This wait deliberately narrows pre-trigger observation timing. It does not fix native admission, detached auxiliary work, the auxiliary initialization/shutdown race, MCP custody, or whole-host cleanup. No auxiliary readiness barrier was added. Auxiliary rejection, one-connection limit, peer EOF and all other assertions remain unchanged.

## Review and dispatch

Review `all-mode-ack.patch`, `INPUTS.json`, and the full `COMMANDS.json` before dispatch. Root must verify all pins/current production source and binary, run proposed pure checks first, then use only the fresh no-clobber matrix output. Each case's acknowledgment must be earlier than its held-ready and stop markers; fallback also requires acknowledgment before 502. Preserve adopted exit statuses, including native child signals; an outer zero exit is not an all-zero descendant claim. All eight cases and their independent outer checks must pass in one run. Never combine cases or rewrite prior receipts.
