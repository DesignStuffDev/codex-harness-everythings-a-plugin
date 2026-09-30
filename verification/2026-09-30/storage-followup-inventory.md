# Remaining persistence boundaries after thread-store contract 2

Read-only source review of the working tree based on upstream
`d42056091aded7feb1d88ac7e83972108b2aa478`. This inventory records remaining work;
it does not claim that all persistence has been extracted. No Rust commands or
production edits were made for this review.

The selected native process implementation deliberately shares the native file
layout. Its passing storage/runtime tests do not establish that every consumer
works with a custom backend that has no host-readable rollout files. The
`ThreadStore` contract explicitly makes local paths optional (`store.rs:130` and
`types.rs:616`). The cases below need tests where the selected backend is
authoritative and native files are absent or deliberately contain different data.

## Functional consumers still coupled to native storage

| Production path | Remaining dependency and consequence | Smallest coherent contract/change | Meaningful gate |
| --- | --- | --- | --- |
| `codex-rs/app-server/src/request_processors/daemon_continuation.rs:29` | Continuation requires `CodexThread::rollout_path`, then directly calls `RolloutRecorder::load_rollout_items` at line 33. A valid pathless backend silently loses interrupted-turn continuation; stale local completion records can suppress it. | Use the existing selected `CodexThread::load_history(false)` port (`core/src/codex_thread.rs:834`). Preserve completed/aborted-turn and permission checks. | Cold recovery where selected history contains an interrupted turn and local history is absent or says completed; verify one actual continuation and no replay of completed work. |
| `codex-rs/app-server/src/request_processors/thread_goal_processor.rs:195`, `:323`, `:384` | Loaded-thread persistence decisions inspect native file existence; cold discovery reads native headers/history for thread identity, source and multi-agent version; reconciliation directly scans rollouts into auxiliary SQLite. | Obtain authoritative thread existence/source/history through selected `read_thread`/`load_history`. Keep goal job/state storage in the already explicit auxiliary service. Add a typed materialization query where lazy persistence must be distinguished from missing threads. | Pathless stored thread supports goal operations; a V2 subagent remains protected when stale local metadata identifies a root thread; goal changes still persist on a materialized thread. |
| `codex-rs/memories/write/src/phase1.rs:204`, `:267` | Memory jobs claim auxiliary records, then read `claim.thread.rollout_path` directly with `RolloutRecorder`. Missing paths cause failed extraction; stale paths select the wrong transcript. | Supply a selected history-reader dependency in `MemoryStartupContext`; use the claimed thread ID with `load_history`. Job discovery/leasing remains the acknowledged auxiliary-database dependency. | Claim an eligible thread whose native path is missing/stale, provide selected-store history, and inspect the real extraction model request and successful job completion. |
| `codex-rs/app-server/src/request_processors/feedback_processor.rs:99`, `:184`, `:309`, `:388`; `feedback_rollout_history.rs:39`, `:72`, `:108` | Model/effort/prompt-hash metadata, main and guardian artifacts, and referenced ancestor byte prefixes come from local files or native path lookup. Backend-only history is omitted. | Existing selected history reads suffice for metadata. Exact diagnostic lineage needs a separate bounded export capability with stable snapshot/ancestor identity, byte limits, ownership and cancellation; reconstructing logical history is not equivalent to exporting original ancestor bytes. | Verify backend-only feedback metadata; compare exported native lineage/prefix bytes to current fixtures; test cancellation/size bounds and absent optional export explicitly. |
| `codex-rs/core/src/session/mod.rs:2439`; caller `session/thread_settings.rs:118` | Whether a settings event should be appended is inferred from native path existence. `None` means append, while a nonmaterialized local path suppresses it. Lazy pathless stores can receive different preparation history and can materialize it on later flush/shutdown. | Add an explicit thread materialization/status query or an atomic append-if-materialized operation. State ownership belongs to the store, with defined ordering against first persistence. | Compare settings-only preparation, first real turn, shutdown and cold resume for native and pathless stores; confirm no unexpected durable empty thread or lost settings event. |
| `codex-rs/core/src/shell_snapshot.rs:1123`, `:1156`, `:1163`; spawned by `:279` | Snapshot garbage collection discovers threads using the native rollout scanner and tests rollout mtime. It can delete snapshots belonging to valid threads without native files. | Keep snapshot bytes owned by the execution/snapshot subsystem; use selected thread existence/archive/activity facts for retention. Prefer a bounded batch query; retain snapshots on unknown/error outcomes. | With two live backend-only thread IDs, run cleanup from one thread and prove the other's valid snapshot survives; separately verify old/deleted threads expire and query failures do not delete. |
| `codex-rs/exec/src/lib.rs:1737`, `:1809`, `:1844`; `codex-rs/tui/src/worktree_startup.rs:96`, `:184` | CLI resume uses native header validation and reverse-scans rollout files for latest cwd; exact-title lookup can return a native SQLite/name-index hit before asking the selected backend. Worktree startup also scans the file to choose cwd. | Route title resolution and latest persisted execution settings through selected queries. Existing `load_latest_model_context` can supply latest turn context; expose the necessary metadata through app-server for clients. Keep user-specified cwd precedence. | Resume by title/`--last` and worktree fork with different latest cwd in backend versus local file; verify selected ID, actual execution cwd and no stale-title short-circuit. |
| `codex-rs/tui/src/named_session_lookup.rs:95`; `worktree_browser.rs:74` | Named lookup rejects local-filesystem paths outside native `sessions`/`archived_sessions`; worktree owner lookup rejects pathless threads and infers archive status from path prefixes. | Treat selected `thread/read` facts as authoritative; expose archive/existence state directly in the app-server response. Native-path validation belongs behind a declared native-layout capability. | TUI named resume and worktree owner status work for a pathless backend and a filesystem backend using a different root; archived/resumable states remain correct. |
| `codex-rs/external-agent-migration/src/sessions/append.rs:66` | Incremental imports already use selected read/resume/append/shutdown, but unnecessarily require `StoredThread.rollout_path`. Pathless persisted backends return `false` before resume. This is a portability restriction, not a direct raw-file write. | Resume by thread ID using the existing optional path field; retain fresh-history verification, writer ownership, durability and import checkpoint ordering. | Import, extend the source, re-import into a pathless store, and verify exactly the delta is appended. Cancel before/after acquisition and before ledger commit without leaking writer ownership or fabricating success. |

## Native-default lifecycle gap

`codex-rs/core/src/thread_manager.rs:477` still starts a detached task calling
`migrate_rollouts_on_startup` directly and then invokes
`spawn_rollout_compression_worker`; the compression-only branch at line 495 is
also detached. These jobs bypass the newly owned `LocalThreadStore` maintenance
supervisor. Consequently its `shutdown_store` fence does not join jobs started
by this native default factory. This is a real native-default lifecycle gap,
not a fallback taken by selected process composition.

Route startup through the owned maintenance admission/scheduling port, preserving
migration-before-compression ordering and warning-only domain errors. Cover the
actual factory with shutdown during lock contention and during a blocked current
migration/compression unit. Assert that an acknowledged shutdown has drained
accepted work, stopped subsequent scheduling and left no cursor advancement for
a cancelled prefix. Existing native-process maintenance evidence remains scoped
to the process composition that already uses the owned port.

## Explicit compatibility/diagnostic dependencies

- `tui/src/resume_picker_transcript_preview.rs:47` uses a local legacy tail fast
  path, but correctly falls back to selected app-server history on scan failure.
  It should be capability-gated to avoid reading stale compatible-looking files;
  it is not an unconditional failure for pathless stores.
- `tui/src/app_server_session/rollout_history.rs:184` takes a native maintenance
  lock while resuming known legacy history. A custom backend's maintenance is
  not coordinated by this lock. The future port needs a snapshot/resume lease
  or an atomic mode-plus-history operation; preserve the existing mode-change
  race protection instead of just removing the lock.
- `core/src/session/mod.rs:5001` and `core/src/hook_runtime.rs:408` expose optional
  native transcript paths to hooks. They already use selected store queries and
  tolerate `None`, but equivalent transcript access for non-file stores needs
  an explicit exported-artifact capability with a bounded lifetime.
- `cli/src/doctor/thread_inventory.rs:92` audits native rollout files against
  native SQLite; `app-server/src/codex_home_metrics.rs:30` measures native
  sessions directories. Neither reports selected-backend health/size. Keep
  these reports explicitly native or add a typed backend audit/statistics port.
  A test should distinguish a healthy custom store from an empty native layout.

## Separate durable subsystems, not ThreadStore fallback bugs

The following remain natively owned and need their own component boundaries for
the whole-harness goal. Moving them into `ThreadStore` indiscriminately would mix
state lifetimes:

- Cross-session prompt history: `tui/src/app/thread_routing.rs:557` and
  `tui/src/app_server_session.rs:2463` call the concrete `codex-message-history`
  file functions (`message-history/src/lib.rs:104`, `:285`, `:300`). A separate
  history port needs append, metadata/generation, stable batch cursors, trimming,
  cross-process locking and shutdown. Gate actual TUI older/newer navigation,
  concurrent append and restart across trimming.
- Memory artifacts: `memories/write/src/storage.rs:54`, `:77`, `:97`, `:135`
  maintain raw memories and rollout-summary Markdown; phase2 owns its workspace
  and publication. Extract typed publication/snapshot/retention together with
  the memory subsystem, retaining its model-visible filesystem requirements.
- Worktree owner metadata: `worktree/src/metadata.rs:30`, `:49` reads/writes a
  no-clobber Git-local owner record. Its ownership belongs with the worktree
  component; preserve concurrent bind conflict and retained checkout recovery.
- Optional diagnostic trace bundles: `rollout-trace/src/thread.rs:106` and
  `writer.rs:51` create native event/payload files when enabled. This is a
  tracing sink boundary; failure remains best effort and must not break turns.

Known separate follow-ups remain: manual `migrate-rollouts` needs its full typed
dry-run/options/progress/report/cancellation interface; auxiliary state services
(queues, goals, memories, agent graph and metadata compatibility) remain explicit
shared-SQLite dependencies; attachment composition/retention is a separate
boundary. This review does not count those already documented dependencies as
new hidden fallbacks.

## Scope and next steps

Production native constructors found are the deliberate unselected default,
the native worker, and the already documented manual migration CLI. Test-only
constructors and `thread_summary::read_summary_from_rollout` were excluded.
The single remaining `LocalThreadStore` downcast is the explicit legacy
`PersistenceServices::from_legacy_native` composition adapter, not session
runtime selection. Primary thread CRUD/resume/fork/query composition is selected.

After the checkpoint, fix the native-default maintenance ownership gap and
continue the agreed manual migration extraction. The smallest later consumer
wave is daemon continuation plus memory/feedback logical history reads, using
existing selected APIs. Materialization and exported diagnostic snapshots need
explicit contracts before changing their callers. All new gates above are
proposals; this read-only audit did not execute them. The 653-case scoped
evidence remains in `storage-maintenance-regressions.json`, with its split-run
qualification and native-layout scope intact.
