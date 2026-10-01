# P02B: retained high-level client shutdown

The corrected `codex-app-server-client --lib` regression passed **40/40 tests, 0 skipped**, with command/subreaper exit 0 and **8,680 unchanged source fingerprints**. This includes real embedded App Server requests and cleanup, plus controlled lifecycle tests. It is not a new GUI, component-manager Launch or installed search acceptance run.

## Behavior and tested source

`app-server-client/src/shutdown.rs` starts shutdown on the captured owning runtime before returning an observation future. Dropping an unpolled or polled observation cannot discard the owned drain. One deadline covers command admission, embedded shutdown acknowledgement and worker join: the shared 200-second embedded budget plus a five-second forwarding margin. Missing acknowledgement, a closed command channel, worker panic or forced timeout cannot report successful cleanup. The first operation error survives later worker failure; forced timeout reports uncertainty while an owner continues observing the aborted worker.

The production implementation is preserved in WIP `bb03f3a81a828871e4e35d3e46b9c8528f3b7d00`, tree `6ad17d58936b6c9f519c06f2932efd1c8ecd4724`. **The corrected test tree is newer than that WIP:** its only change across the recorded 8,680 paths is the lag-marker fixture in `app-server-client/src/lib.rs` (eight added, two removed lines). That tested file has SHA256 `c33e258a27ec120216ef66f40427ab135829d2819bac3780d98cfba18a27c5bc` before the separately recorded formatting. The corrected/formatted fixture is now published in WIP `cb977e7d664bc752c28854d468619039bd3ad167`, tree `40207f354efb3bee85265d8f3d83312489b56709`. The fixture retains its command receiver, receives the actual shutdown command and acknowledges it before worker completion. Existing lag-event and successful-shutdown assertions remain unchanged; production code did not change between the failed retry and corrected run.

[Machine-readable evidence](p02b-client-shutdown-results.json) records all 40 case names, exact source/log/subreaper artifact hashes, canonical full-manifest digests, the fixture diff and 11 selected source bindings against the immutable WIP. The runner's older baseline label is not represented as the tested tree identity.

## Separate runs and failures

| Run | Result | Source |
| --- | --- | --- |
| `p02b-client-shutdown` | ENOSPC during `codex-core` compilation; **no tests executed**, exit 101 | 8,680 fingerprints unchanged |
| `p02b-client-shutdown-retry` | **39 passed, 1 failed**, 0 skipped, exit 100; failing case failed both attempts | 8,680 fingerprints unchanged |
| `p02b-client-shutdown-corrected` | **40 passed**, 0 skipped, exit 0 | 8,680 fingerprints unchanged after the isolated fixture correction |

The failed retry's `next_event_surfaces_lagged_markers` fixture discarded the shutdown receiver and used an already completed worker, yet expected shutdown success. The retained client correctly returned `BrokenPipe` with unconfirmed runtime/storage cleanup. Its failure and the earlier ENOSPC run remain preserved separately; they are not relabeled as passing runs.

All **12 new controlled-worker lifecycle tests** passed in both executed runs. They cover the shared budget, saturated admission, one deadline across acknowledgement/join, acknowledgement error/loss, panic and error ordering, dropped observers, enum dispatch, calls outside an entered runtime and runtime destruction. These tests use actual Tokio workers and channels; deadline cases use paused time. They do not prove a real storage drain taking the full 205 seconds.

Existing real embedded-runtime cases initialize App Server/config/state, exercise typed requests, thread creation/list visibility, session-source propagation, small channels and unread notifications, and verify prompt successful shutdown. Other existing cases cover mock remote WebSocket/Unix-socket peers, metadata, events and errors. These groups are subsets of 40, not additional totals.

The corrected subreaper completed with exit 0. Its ten adopted children had return codes 0 (one), 128 (six) and -13 (three), recorded independently of test results. No child status is silently counted as a passing test or omitted from the preserved report.

## Remaining gates and custody

Scoped `p02b-client-shutdown-fix` completed with exit 0, nine unchanged scoped fingerprints and no warnings in its log. Final `p02b-client-shutdown-format` completed with exit 0; among its 8,679 fingerprints, only `app-server-client/src/lib.rs` changed, wrapping the new fixture acknowledgement call. Before/after hashes are separate from the passing test bytes; no extra test run is inferred or required solely for formatting.

The actual native backend staging was applied **after** these client gates. This 40-test run does not cover that later native source. The corrected and formatted client source is now included in WIP `cb977e7d`; this preserves source without extending the scope of the earlier client tests.

This validates the client library's recorded source, not a rebuilt CLI/GUI or Launch Ctrl+C path. The earlier [StageA runtime evidence](P02_SEARCH_PREREQUISITE_EVIDENCE.md) remains the latest complete new-engine/storage/GUI acceptance. No independent native search installation or whole-harness extraction is claimed.

Original `/workspace/acceptance/p02b-client-shutdown*` artifacts are cloud-local. GitHub WIP `cb977e7d` now protects the included corrected/formatted client source. Public evidence contains whitelisted source identities, test outcomes and fingerprints, without credentials, private bearer URLs or session data.
