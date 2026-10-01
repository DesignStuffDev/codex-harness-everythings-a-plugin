# P02B: opt-in persistent component payload limits

This checkpoint adds **logical request/reply size limits and cleanup-result handling to existing component transport**. It supports a future search process adapter; it is not installed search integration or extraction of another native subsystem.

The post-format source is preserved at WIP **`98c540863b9868eb216dba18f963c220c4be5187`**, tree `d193d69db07c0ca378a9f95dae27d6dcee9d6019`, on `wip/p02b-bounded-search-20261001`. All **47 recorded component-host file fingerprints** match that immutable commit after the documented formatting. Later working-tree changes are outside this evidence.

## Implemented contract

`SessionPayloadLimits` in `codex-rs/component-host/src/session_limits.rs` supplies optional ordinary request/reply JSON-byte limits. `connect_with_limits` and `ComponentServer::stdio_with_limits` opt in; existing constructors keep their defaults. Ordinary storage/history payloads remain unrestricted by these new optional limits. The wire format/version is unchanged.

Local request preflight runs before admission. A bounded lower-bound walk rejects obviously oversized nested strings or containers, then a counting serializer stops at the first over-budget write. Incoming declared lengths are checked before payload spooling or body decoding. These limits cover serialized logical bodies; they do not bound total process memory or undo allocations already used to construct a caller's `Value`.

Cleanup controls retain their separate **64 KiB** allowance. A configured reply cap smaller than that is rejected before process launch rather than silently raised. Reserved control responses keep their classification even if public request metadata changes. An oversized server reply uses the existing remote error format and leaves the control lane available.

`DeferredControl::release_reply` returns the actual cleanup response, retaining operation failure and unconfirmed-cleanup receipts. Locally generated `PayloadLimitExceeded` remains a typed cause; a remote error string with identical wording cannot forge it. A terminal oversized peer declaration preserves that cause through the operation, cleanup and close results and reaps the process. If the reader reports a size violation before a later writer failure, the earlier cause remains observable.

## Exact passing scope

The shared library run passed **166/166 executed tests, 1 ignored entry**, exit 0 and subreaper exit 0. Its **105 source fingerprints were unchanged before and after**. Package counts were component host **75**, native search **41**, Nucleo **23** and Matcher **27**. This is the same run referenced by [bounded-index evidence](P02B_BOUNDED_INDEX_EVIDENCE.md); do not add the two reports as separate runs.

The new size-limit coverage comprises **17 cases within the host's 75**:

| Scope | Cases | What was exercised |
| --- | ---: | --- |
| Serialization/declaration checks | **8** | Exact serialized bytes and escaping; bounded lower-bound traversal; stopping serializer; declaration checks without payloads; control-budget validation; default large-history behavior. |
| In-memory server/wire checks | **4** | Oversized reply preserves the control lane; immutable response classification; request rejected before body decode and retained through finish; reader failure survives a later writer failure. |
| Host payload cases | **5** | Four launch real independent fixture processes; one rejects invalid configuration before launch. |

The four actual-process payload cases verify oversized local requests never reach peer admission while reserved cleanup survives full ordinary capacity; `release_reply` retains an unconfirmed receipt and operation error; legacy remote text cannot forge a local typed cause; and an oversized peer declaration is terminal, reaped and reported consistently through cleanup/close. The fifth case asserts that an invalid reply limit creates no process state directory.

Existing startup/cleanup regression coverage comprises **eight real-process cases within those same 75**: seven `session::tests::paired_start` cases plus `session::tests::reaping::paired_release_timeout_is_uncertain_and_forced_close_reaps_the_process`. They exercise abandoned waiters, rejection cleanup, capacity admission, immediate cleanup, startup timeout, reserved release, and forced-close uncertainty/reaping. They are distinct from the 17 size-limit cases, not 25 additional cases beyond the host total.

Retained host coverage also passed multiplexed large payloads, a history larger than 16 MiB, ordered framing, partial results, close fencing, reader/writer/peer errors, deadlines and parent-death cleanup. **There were no failing size-limit tests in this run.**

The one ignored top-level entry is the parent-death subprocess helper, which the passing parent test invokes explicitly with `--ignored`; the aggregate still records one skipped entry. The subreaper adopted three children with signal-termination status `-9`, while its own and the test command's statuses were 0. Lifecycle claims rely on passing unchanged fixture assertions and runner results, not dismissal of lingering processes as zombies.

## Lint/format timeline and evidence

After the passing test run, scoped `just fix` exited 0 and changed six vendor files mechanically; the component-host source stayed unchanged. Whole-workspace `just fmt` exited 0 and reformatted eight component-host files. A separate successful vendor `rustfmt` affected only three new vendor modules because the workspace formatter excludes that workspace. No test rerun was performed solely for those lint/format changes.

The [machine-readable report](p02b-transport-limits-results.json) keeps the tested hashes separate from every later transition and verifies final component-host bytes against the immutable WIP commit. It also identifies all 17 limit cases and eight paired-start/release regressions, exact commands, log/source/subreaper hashes, and source-preservation records. The passing run was not mislabeled as testing an unchanged published baseline.

Original artifacts are `/workspace/acceptance/p02b-bounded-and-limits.{log,source.json,subreaper.json}`, the separately named `-fix`/`-format` logs/manifests, and `p02b-bounded-vendor-format.{log,source.json}`. Only whitelisted metadata is published; no private runtime reports, bearer URLs, credentials or session contents are copied.

## Remaining limits

These caps are opt-in support machinery. Native-search admission, shared service budgets, search worker selection, independent package build/install/removal and compatibility acceptance remain separate work. No new CLI/App Server executable or GUI runtime gate was run for this checkpoint; earlier GUI proof is not relabeled. The wire remains compatible with existing defaults, but this run is not whole-harness or cross-platform acceptance.
