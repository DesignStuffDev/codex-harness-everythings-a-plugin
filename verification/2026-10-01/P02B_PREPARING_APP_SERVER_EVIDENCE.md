# P02B App Server Preparing cancellation evidence

The strict focused gate passed **460/460 tests: 418 App Server and 42 client,
zero skipped and zero retries**. This adds retained Preparing cancellation to
an existing selected file-search consumer; it does not extract another subsystem.
Source is preserved on cancellation WIP `9bd3bc30f4259245b3e4c769840289fcda0d2dcd`; accepted main runtime is unchanged.

`SearchObserver` retains exact pending-start authority before waiting for Open.
A cancellation raced with publication is latched, forwarded outside locks and
kept separate from the passive startup result. The startup owner retains both
original finish and typed cancellation receipt through cleanup and the ready
handoff fence. Token retirement holds the existing slot until cleanup completes;
an old guard cannot remove a newer token owner. Genuine startup errors and
Unconfirmed cleanup remain distinct from deliberate clean cancellation.

| Gate | Observed result |
| --- | --- |
| `p02b-preparing-app-server-tests` | 460/460, exit 0, no skips/retries; **strict subreaper omitted**, no runner reaping proof. |
| `…-fix` | Exit 0; one Clippy let-and-return simplification in `request_processors/search.rs`. No assertion changes. |
| `…-format` | Exit 0; eight AS Rust files and three SDK Python packager files formatted. |
| `…-tests-02` | Same 460-test suite, all passed under unchanged strict subreaper; exit 0, no skips/retries. |

The second run corrects the runner omission; these are **460 unique tests, not
920**. Its 18 raw wait records and original nonzero statuses remain in the JSON:
return-code histogram `{"-13": 9, "0": 4, "128": 4, "141": 1}`. The command was reaped
with 0 and `runner_error` is null. This establishes the runner's bounded descendant
drain, not that every subprocess exited 0 or that whole-host/GUI shutdown was tested.
The runner source hash is unchanged between both runs.

The three new direct-processor tests prove: same-token cancellation reaches the
pending owner without cancelling a sibling and keeps quota until cleanup;
controlled session Stop retains the starting entry until its receipt; and a
genuine ClosedLease remains an error after requested retirement. All three pass.
Their gated fixtures are deliberate controlled providers, not native OS cleanup
or public JSON-RPC scheduling proof. Existing AS/client lifecycle assertions also
pass, including abandoned one-shot ownership, stop/update cleanup, failure after
shutdown intent, client-factory lifetime and in-process drain behavior.

**Public pending Start/Stop remains a known gap at this tested source.** Session
RPC serialization queues Stop behind a pending Start of the same session ID.
The three direct tests bypass that queue. The reader-intent fix and actual public
pending-stop acceptance are a separate slice; do not relabel this library result.

The desktop GUI instead uses legacy `fuzzyFileSearch`, whose protocol explicitly
has no serialization queue. Clear/root/task/close sends an immediate same-token
empty-query RPC while the original request is pending. Current browser response
barriers withhold an already completed HTTP response; they prove stale-result
fencing, not backend Preparing cancellation. A newly built host with real GUI
held-Open cancellation, selected-worker installation, launcher shutdown and
in-app Browser checks is **not proved by this gate**. The coordinated relay gate
will distinguish host/process Preparing from a native worker already Ready.

Each test run captured 10,142 source fingerprints unchanged during execution.
Use those full hashed maps, not the runner's historical `f2cc6c2…` baseline label.
First-test, post-lint, post-format and strict-test hashes are kept separately for
all 12 scoped files: eight AS integration paths plus four earlier AS/client
required-begin fixtures. Recorded source maps connect continuously between gates.
The root formatter delegates to `scripts/format.py`, including repository-wide
Ruff formatting, explaining the three Python paths. Their semantic validation
belongs to the separate SDK tests; Rust tests do not validate Python behavior.

The eight adopted AS source members and four relevant earlier fixture members
were re-read from their original archives and checked against adoption hashes.
AS archive SHA256 `14b8ae32f7860792627858e378e41b9ebe819d35ce731471724fc88903569064`; earlier fixture/runtime archive SHA256
`7d1de79cee23f4de26a4939da5f00ba961cc8ab6c05d56ff325ab90e2d15c424`. Separate first-run and strict-run generated library
test executables are preserved by root's hash/stream-verified cache reports.
Their post-gate hashes are not stronger executable attestation emitted by the
runner. These archives are cloud-local recovery checkpoints, not demonstrated
outside backups. No source reset, checkout edit, build or Git action occurred
while generating this evidence.

TUI behavior, fresh full hosts, independent worker packaging, GUI/runtime
acceptance, upstream updates and whole-harness completion remain separate.
