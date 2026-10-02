# Adoption and validation order

All proposals below are frozen evidence inputs, not executed or adopted claims.
Finish the current compiled lifecycle gate and actual host/GUI acceptance before
adopting the MCP stack. Recheck every preimage against the then-published source;
if formatting or other authorized changes moved the base, author a new mechanical
rebase overlay and preserve the original frozen inputs.

1. Upper session/task custody: apply
   `p03-mcp-session-custody-proposal/combined-upper-02.patch`, bound by manifest
   `d99b4b1ce641d51952eecb0adf9ba761777c1f5bea45ee2c9d7e5d961714dcef`.
   Then apply `propagation-lifetime-review-03/lifetime-retirement.patch`, bound by
   `9431799f6a8331e5bd2e57db2122c2c25738685eb22d14028e5a869d8a1f319d`.
   This establishes typed session-loop observation and exact successful-owner
   retirement, with its existing scope and idle-retention limits.
2. Lower workspace ownership: apply `LOWER-WORKSPACE-01.patch`, bound by
   `2b587ab16cf31e8798b0f5ceead85f6d1324ecf3ba78694292df0323b4bba2b4`.
   Its nine-file codex-mcp adapter is included; do not apply that subpatch twice.
   Review the service, transport, local-process, generation-roster and typed
   observation units separately. Acceptance requires their combined wiring.
3. Dependency source: review this additive SDK patch against exact upstream
   revision `3e636cab26c013eca5131103c03d20237f12c4df`, including existing protocol
   regressions and the new actual-worker tests. Root must materialize the reviewed
   changes in an explicit immutable patched dependency source and record its
   actual revision/content identity before changing any workspace pin. Never
   mutate the shared Cargo checkout or silently relabel the old revision.
   No dependency destination or new revision has been created by this proposal.
4. Workspace consumption remains a required, separate small review unit. An
   HTTP transport adapter must call observed close on the concrete worker,
   retain that transport while pending, and require both its exact join and
   the cleanup receipt before replacing the current HTTP `Unconfirmed` status.
   Preserve original protocol/DELETE outcomes separately; optional ordinary
   failures alone must not fail otherwise successful turns. Missing/failed
   ownership evidence must not silently become `Confirmed`.
5. Run scoped dependency and workspace tests, required fix/fmt and exact source
   binding. Then implement/review the outstanding scoped historical-generation
   and process-final roster aggregate/admission fence, and run real core/host
   lifecycle acceptance. None of the preceding stages alone certifies whole-host
   cleanup, executor completion or descendant process-group emptiness.

## Dependency and license bookkeeping

- Upper/lower use the existing workspace `codex-async-utils`; lower adds that
  edge to `rmcp-client/Cargo.toml`. Root owns Cargo and Bazel lock updates.
- The SDK changes add no crate, version, feature or toolchain requirements.
  The optional real HTTP test uses existing SDK dev dependencies and is gated
  by `transport-streamable-http-client-reqwest`.
- If the patched source is a Git revision, `rmcp` and its path-associated
  `rmcp-macros` source identities must be updated consistently in `Cargo.lock`;
  do not invent a commit hash. Keep `=3.3.0` only if the actual patched package
  retains that version. Refresh `MODULE.bazel.lock` with the required repository
  command after the approved source/pin is concrete.
- Preserve `UPSTREAM-LICENSE` and `UPSTREAM-PROVENANCE.json` with the patch and
  mapping. The exact upstream notice describes a MIT-to-Apache transition;
  this proposal does not relabel all upstream code as exclusively Apache.
- Update the upstream-maintenance mapping with the exact base, patch hash,
  patched-source identity, affected paths and regression command/results.
  There are no online documentation or upstream-submission claims here.

The authored tests, static reviews, hashes and patch replay are review evidence.
They are not substitutes for compilation, the required full core gate, or the
held real transport/embedded replacement/slow installed-store/GUI acceptance.
