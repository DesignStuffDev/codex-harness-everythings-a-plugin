# Native token-install checkpoint: dependency inputs blocked — 2026-10-05

The coherent eight-path native change is adopted and formatted in the existing
working tree. It is **not compiled, tested, accepted or published**. The first
focused `just test` invocation failed before compilation while Nextest resolved
offline metadata: `alsa 0.11.0` was missing. Zero native tests ran; no new OOM or
source/index drift was observed. Preserve the failed attempt and repair the input
closure before another source-bound attempt.

## Exact source and intended behavior

Last published/read-back WIP: `073cdeda69f08221c76c4017c09e0d7a09a57576`, tree
`d45435be8b712f24a7e4fac19e72a7fe3453007b`. New source map: 8,980 paths,
`b75efc9751977009a9b28f2893c120db45a8f6e79f4993125e3685ae90d6cf5b`.
Formatted candidate manifest SHA256:
`5bc49e31186aa91be93e7c1ffb356d4a7e656276a643881896664a07aefc06fc`.
`just fmt` exited 0. The final diff has 596 added/deleted lines combined, including
new tests and formatter expansion. [Evidence](EVIDENCE.json) binds the eight exact
before/after paths and preserves the original proposal/adoption/format receipts.

The native process-local Ephemeral store gains install tickets. External-token
installation captures a ticket before resolving a provider and commits prepared
credentials together with the source/cache publication under checked ownership.
Raw store mutations invalidate pending tickets. The implementation routes the real
native caller through the store; splitting it into a pre-save check followed by an
unconditional publication would retain the race. These are intended source-level
properties awaiting validation, not runtime guarantees or a new separately
installable auth component.

Original `storage.rs::EPHEMERAL_AUTH_STORE` and `EphemeralAuthStorage` move into
`ephemeral_install_store.rs`; existing native key derivation/factory behavior stays
in storage. Custom `auth_source.rs::install_external_auth` and
`auth_reload.rs::replace_auth_owner` use the new checked commit. Telemetry visibility
and sibling tests form the rest of the eight-path cohort. Upstream remains
`openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`; license/NOTICE retained.
Legacy external refresh, other raw writers/storage modes and request-dispatch
ownership remain outside this slice. The store/caller operation must stay coherent.

## Preserved baseline and admission

The first pre-adoption guard refused before any mutation/build: 8,486,326,272
hard-unused bytes were below the 8 GiB requirement (8,589,934,592 bytes). That refusal
is retained separately; it was not a failed compilation.

The exact accepted CLI/manager were then archived (140,182,904 bytes), SHA256
`5c5a49bfd0bbe4f9774d9a008ade1e0e96e9fd39c0815d7a7b8f40970afcedfb`.
A full two-binary restoration/hash check passed, allocating 643,276,800 bytes.
Only those owned duplicate probe files were released; original binaries and archive
remain. Scoped clean-cache advice on the exact verified archive/binaries, without
global cache dropping or content/metadata changes, yielded 9,139,257,344 hard-unused
memory bytes and 3,978,465,280 persistent free bytes. These are point measurements,
not reservations or a successful native capacity gate. The archive is VM-local.

## Actual first focused invocation

From `codex-rs`, through the unchanged source wrapper and strict subreaper:

```sh
just test --locked --retries 0 -p codex-cli -p codex-login --lib   -E 'package(codex-login) & (test(auth::manager::auth_install_tests::) | test(auth::manager::auth_source_tests::) | test(auth::storage::ephemeral_install_store::tests::))'   --test-threads=1
```

Nextest's metadata subprocess requests `--all-features --filter-platform
x86_64-unknown-linux-gnu --locked`; its offline download attempt for `alsa 0.11.0`
failed with exit 101. The recipe, source wrapper and strict result retain exit 102.
This exposes a metadata input dependency; it does not establish that the scoped
auth test executes audio code. The strict runner recorded no runner error and reaped
only its own failed command. The observer confirms unchanged bindings/source/index,
no new memory events and retained recovery floor. Its two samples do not bound a
future compiler/linker peak. No compilation or native test executed.

Raw source/strict/observer/log receipts remain at
`A/p03-token-install-focused-20261005-02.*`; adoption/preimages/formatting and accepted
binary recovery remain under their separate `R/p03-token-install-…-20261005-02`
directories. [EVIDENCE.json](EVIDENCE.json) binds these originals without duplicating
source maps or binary payloads. No old source42 pass is rebound to this source.

## Exact next action

Repair and verify the bounded offline metadata/input closure, then recheck fresh
resources and source bindings before retrying. Do not blindly repeat this failed
command or replay adoption/formatting. Focused/full login/provider `just test`,
scoped login `just fix`, production CLI/manager rebuild, all 12 retained runtime
recipes and the new native auth-install consumer remain pending. GUI/runtime
validation must bind the rebuilt host and explicitly identify any Chromium/model
fixture fallback. Full-workspace testing still needs its separate approval.
Collector and broader updater feature work remain paused.
