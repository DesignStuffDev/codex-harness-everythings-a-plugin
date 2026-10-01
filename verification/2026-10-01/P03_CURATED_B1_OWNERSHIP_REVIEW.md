# Curated sync B1 ownership review

Status: read-only design review, not an implementation or passing checkpoint. Stage A is a scoped Linux transport correction under verification. Its full-host lifecycle, uncertain-cleanup fallback and filesystem ownership limitations remain unresolved. No new code proposal is staged by this review.

## Required split

The planned single B1 slice, “typed stop/failure + cancellable lock/Git pipeline,” needs an ownership prerequisite. Its estimated 430 complex changed lines do not establish that the prerequisite and meaningful tests fit the 500-line limit. Do not implement a nominally safe typed fallback gate while dropping the resources that protect concurrent mutation.

The smallest coherent immediate boundary is **typed transport failure plus actual retained cleanup/admission ownership**, integrated into the real pipeline and manager together. Cancellable lock/Git propagation follows that boundary. Parts of C1a/C1b must therefore move before the proposed B1 exit contract; full HTTP/host shutdown remains later work. If this atomic ownership change exceeds the limit, first make separately reviewed tests/mechanical extractions. Keep the behavior-changing ownership transfer and its real callers in one adoption unit; do not claim closure from an unused registry or error enum.

## Concrete current limitations

1. `bounded_command::run_bounded_background_command` returns String. On its cleanup deadline it returns an error, then `OwnedGroup::drop` signals the group and transfers only `ChildToReap::Native(pid)` to the shared reaper. The reaper polls that direct PID without a completion acknowledgment. It retains neither output pipes nor the sync's lock/staging resources. Direct-child reaping alone cannot resolve unknown helper/pipe state.
2. `sync_openai_plugins_repo_with_transport_overrides` falls back on every Git error and releases its `_file_guard` when the function returns. `sync_openai_plugins_repo_via_git` drops `staged_repo_dir` on an error. Other native temporary repositories, such as the trusted `ls-remote` working repository, also leave scope. A typed early stop prevents another transport but still drops these resources unless ownership is changed.
3. Git is not confined to disposable staging: when the existing repository has `.git`, the pipeline fetches into that existing repository before copying the fetched revision into staging. Releasing its lock during uncertain cleanup can admit another sync while a previous mutator may still be active. Returning an error is not evidence that this mutation has ended.
4. `read_local_git_or_sha_file` currently converts every HEAD error into optional SHA-file fallback. Cancellation and cleanup uncertainty must not be swallowed there.
5. Git, HTTP and export paths activate the repository before writing `plugins.sha`. Activation drops the previous-repository backup on success. Git/HTTP publication errors are still ordinary Strings at fallback boundaries, so a subsequent transport can run after partial or uncertain publication. Typed transport errors alone do not fix this. At minimum those publication errors must stop further fallback; retaining/restoring the old repo/SHA pair and recovery journaling remain the separate publication transaction work.
6. `manager::start_curated_repo_sync` discards its native JoinHandle and resets the process-global `CURATED_REPO_SYNC_STARTED` flag on every sync error. Keeping only a String manager entry loses the distinction needed to prevent retry admission during unknown cleanup. An observed timed-out/unknown attempt cannot be rewritten as successful merely because a later cleanup observation completes.

These are current source facts, not evidence that a new runtime failure occurred during this review. Stage A's successful command return confirms its direct wait and pipe EOF after a group signal; it does not by itself certify every group member's disappearance/reaping. ECHILD handling assumes exclusive waiter ownership and does not turn lost identity into a cleanup confirmation.

## B1a: one real ownership transition

Before launching a child, establish an attempt owner that can retain the stable lock, admission generation and each relevant staging/trusted-repository resource. The child owner must expose typed cause and cleanup state at the point of failure, without parsing error text. Keep original String/Display behavior at public facades; the real manager consumes the typed internal result.

The failure categories only need to serve decisions this slice actually makes:

- Ordinary confirmed transport failure or confirmed per-step timeout: eligible for the existing policy-checked fallback when no publication occurred and owner stop is absent.
- Owner stop/deadline: no new child, fallback, publication, cache refresh or callback.
- Lost identity or unconfirmed cleanup: no fallback or new sync admission; retain the attempt's resources and its original uncertain result.
- Publication begun or rollback uncertain: no fallback. Do not label the previous state unchanged or discard recovery ownership.

Uncertain cleanup must transfer the **whole relevant ownership bundle**, not only a raw PID, to an already established, acknowledged cleanup owner that remains reachable after the caller/observer is dropped. That owner must have a completion contract precise enough for the resources it protects. Do not release lock/admission solely on a shared direct-PID reap, an observer timeout, a worker's String return, or Drop of a report. If ownership identity is lost, fail closed pending explicit recovery; never signal a potentially recycled identifier. Do not solve this by forgetting a lock, detaching a new untracked thread or adding an isolated unused framework.

The public String facade may format the outcome only after completion or acknowledged transfer; formatting must not destroy the last owner. The manager may reopen the existing process-global gate only for its exact generation after confirmed drained failure. Preserve the current success latch and cross-home deduplication semantics. Full host cancellation and callback descendant tracking are not implied by this narrower prerequisite.

Meaningful acceptance before claiming this boundary:

- A held cleanup-completion barrier with a real stable lock and real owned staging directory: an unknown result and dropped observer preserve both; a competing attempt cannot enter; confirmed completion releases them correctly.
- A stale generation cannot reopen admission while a replacement/unknown owner remains. Existing successful global-latch behavior remains.
- Typed cleanup-unknown and stop results cannot reach HTTP/export; ordinary confirmed failures still take the existing allowed fallback. Preserve exact CA/proxy/trusted Git policy checks.
- Local HEAD does not hide stop/unknown as None. Publication failure cannot enter another transport or claim PreviousUnchanged.
- The transport-to-cleanup-owner transfer itself is tested, including unwind/error and lost-identity paths; a mock fallback predicate alone is insufficient.

The final cleanup acknowledgment must state what it proves about direct child, pipes and owned mutators. Until that proof exists, resources/admission remain retained and whole-host acceptance remains blocked. No universal process-tree certainty is inferred from Stage A's direct-child result.

## B1b: cancellable stable lock and Git controls

Once B1a exists, add the real control to the actual synchronous pipeline. Use the existing File::try_lock idiom, stable lock file with truncate(false), a 30-second ordinary acquisition ceiling and a wakeable cancellation/deadline wait. Never unlink the lock file. Recheck stop immediately after successful lock acquisition and before each Git command and fallback transition. Earliest shutdown deadline can shorten but cannot be extended or reset per command.

Pass the same control through HEAD and local-SHA selection as well as init/fetch/reset/clean. Preserve typed failures through those helpers. Source resolution and system PATH/environment scrubbing remain unchanged. Tests must cover held-lock cancellation, stopped admission with no Git spawn, ordinary fallback, stop before next fallback, and ownership remaining retained on cleanup uncertainty.

B1b may prevent admission of a new HTTP/export attempt, but it must not claim that an already admitted HTTP send/body, extraction or filesystem operation is cancellable. Those futures and publication transactions belong to the next bounded stages. Checking a stop flag before a blocking HTTP call is not complete HTTP shutdown.

## Review scope and provenance

The accompanying JSON binds the exact design inputs and current source files read. Original Stage A and installer evidence stages remain unchanged. No Rust commands, runtime fixtures, source changes, Git writes or cache changes were performed. A second read-only reviewer independently confirmed the retained-resource gap; this is static review, not compiled or runtime evidence.
