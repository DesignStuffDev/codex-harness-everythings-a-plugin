# Project status — 2026-10-02

This is a working **partial** Codex component platform. Three bounded families of
pre-existing native functionality have separately installed replacements, and our
own separately packaged GUI uses the real engine. Most engine services remain
coupled. The finished minimal-host “everything is a plugin” platform is not complete.

This report supersedes the earlier same-day status text. Historical failures and
source-specific results remain in verification records and Git history; they are
not relabeled as current passes. The precise next action lives in
[EXECUTION_STATE.md](EXECUTION_STATE.md), and the full ordered plan is
[IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md).

## Source, recovery and publication

- Repository: [DesignStuffDev/codex-harness-everythings-a-plugin](https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin).
- Official source remains pinned at `openai/codex`
  `d42056091aded7feb1d88ac7e83972108b2aa478`. Apache LICENSE, NOTICE and provenance
  are retained. No DeepSeek/Cordis dependency was introduced.
- Main remains `781080f7e3c8bfe1953378001d777dff33d74bc3`. It is distinct from the
  ongoing P03 work on `wip/p03-process-final-and-mcp-preservation-20261002`.
- Latest P03 source checkpoint: `d229296b29947ef1328cbe931c810276f220a23c`.
  Its production/runtime evidence is published at
  `c463a4f46318ccbbc811c202639d53f733d5831f`.
- Work continues in the original recovered VM and checkout. The sibling worktree,
  unfinished source, original failed runs, test artifacts and recovery archives
  are preserved. No replacement VM, checkout reset or local-runtime substitution
  was used. VM-local archives are recovery checkpoints; only actually published
  GitHub files have verified external durability.

## What has actually been separated

| Native functionality | Implemented and exercised | Remaining scope |
| --- | --- | --- |
| Local thread storage and manual rollout migration | Native external worker selected by production callers; independently built package, CLI session storage/resume, migration dry-run/apply/idempotence/cancellation/close. | Auxiliary SQLite domains, all background maintenance, repair and reclamation remain coupled. Some durable-admission/history conditions remain open. |
| Inline attachment implementation | Native separately packaged upload and original `NotFound` resolve behavior. Current production caller tests cover native upload, custom file-reference cold reuse, typed fallback, removal/default restoration. | Inline storage has no durable blobs. Production resolve, remote upload/storage and comprehensive transfer/durability semantics remain unfinished. |
| Bounded native file search | Separate native traversal/matching worker with installed selection, parity/streaming/cancellation/failure tests and real clients, including GUI. | Filesystem/watch/Git/worktree/import services and remaining lifecycle cases are not all extracted. |

Separate builds and installation without rebuilding the host are real acceptance
requirements and have been exercised for these bounded packages. Reusing those
independently built packages with a newer host is additional compatibility evidence;
it is not a fresh plugin build. Rust crates, adapters and example tools alone are
not counted as native subsystem extraction.

## Infrastructure and new capabilities

The component API, process host, adapters and manager implement package installation,
selection/removal, manifest/contract checks, configuration and lifecycle handling.
Storage has persistent transport. Dependency-service declarations, bounds, handles,
grants and negotiation data have scoped tests; generic runtime negotiation,
accounting and granted service execution remain inactive.

The SDK includes a versioned Python wheel, templates, zipapp packaging, native Rust
package tooling and examples. Earlier acceptance includes outside-checkout builds,
fresh installation and unchanged host hashes. This is not a public package-registry
release or proof across every supported platform. Installed components are trusted
executable code; process separation does not establish hostile-plugin isolation.

Real host adapters exist for model streaming and authentication resolve/refresh.
Tool, context and lifecycle contributors add capabilities. The full native model,
authentication, context and tool-execution implementations are not thereby extracted.
Catalog/model-v2/context work remains staged or coupled.

## GUI and usable behavior

Our desktop-style GUI is its own installable presentation package with HTML/CSS/JS
and a gateway using the real Codex App Server. It supports saved sessions, streamed
responses, native shell approvals, cancellation, continuation, cold recovery and
selected search. CLI/headless use works with the GUI absent. This is our interface,
not source reuse or modification of the official installed Codex desktop app.

The latest tested production baseline runs two real Chromium cycles for both ordinary and selected
search configurations. Approval, native command and Stop are first-cycle checks;
cold history, interrupted state, continuation and search are second-cycle checks.
Actual manager Launch shutdown is included. Inference is deterministic. The in-app
Browser is unavailable here, so these are Chromium/Playwright fallback results,
not claimed manual Browser or live-provider validation. A user-accessible interactive
remote viewer remains unverified and is an optional independent workstream.

## Latest development and evidence

Recent implementation work addresses actual native callback, worker, featured-task
and HTTP-constructor ownership: closing admission, retaining cleanup responsibility,
sharing shutdown deadlines, rejecting late publication and reporting uncertain
cleanup. This is required support for replaceable services, not another extracted
native family. HTTP error classification preserves certificate trust policy.

- Native HTTP Stage A passed139 scoped package tests and four configured-CA
  classification checks. Earlier failures and the later style-only transition
  retain their own source identities.
- Stage B passed553 core-plugins library tests, one real native HTTP composition
  case and scoped lint. Consumer-specific tests remain separate.
- The latest tested offline production build passed on8,947 scoped source files, map
  `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`.
- All12 planned production-runtime slots have passing attempts:13 successful
  top-level commands because the selected-search retry required fresh migration.
  Storage, migration, attachments, both GUI modes, normal/forced slow shutdown,
  eight held Git/HTTP cases and a featured-owner postcheck are represented.
- The first selected-search GUI attempt failed its cold-cycle60s result wait and
  is preserved. The diagnostic retry passed with12 observed search replies per
  cycle. Disk exhaustion warnings existed on the first run, but its cause is
  unproved; a retry pass does not explain or erase it.
- Normal slow Launch shutdown waited46.4658s and recovered the canonical message.
  Forced second-SIGINT returned1 at2.0393s with durability explicitly unknown;
  cold continuation passed. The hold precedes native forwarding, so it does not
  prove every native storage-admission or response-only-history condition.
- Strict process reports retain nonzero reaped-child statuses and their limits.
  A scoped clean result is not a claim that every host service shut down gracefully.

These counts overlap earlier checkpoints and are not one project-wide test total.
The corrected September30 core library rerun passed2,694 cases; the original
run's four failures remain preserved in [its analysis](verification/2026-09-30/recovery-core-failure-analysis.md).
That result is historical, not a complete workspace run or a current-source
substitute. Full workspace approval is not recorded;
verification uses scoped `just test` entrypoints.

Current-source CLI shutdown tests now pass3/3, with297 filtered tests, two
reviewed snapshots and no retries. Their current test ELF and unchanged source/host
bindings are recorded in the [focused CLI evidence](verification/2026-10-02/p03-http-stage-b-cli-focused/README.md). This is a focused check, not the
full CLI suite. The subsequent [grouped CLI+Exec gate](verification/2026-10-02/p03-http-stage-b-exec-grouped/README.md)
passed nine Exec instances across its library and binary, with64 filtered tests
and no retries. The CLI library was compiled but not executed. This uses the
CLI dependency-feature composition; it is not a standalone Exec package suite.
The TUI compilation subsequently received SIGKILL while cgroup OOM/kill counters
increased; zero tests ran. [That failure is preserved](verification/2026-10-02/p03-http-stage-b-tui-grouped-failure/README.md).
TUI, App Server lifecycle and actual same-process replacement acceptance remain
open. The production executable has been restored from its hash-verified archive
and its version command succeeds. The next five-path auth reload-source fix is
now applied and formatted, but remains uncompiled and untested; it is newer than
that production executable. The permanent-refresh-failure follow-up remains an
unadopted proposal. Successful-refresh durable persistence needs its own conditional
ownership protocol. Memory/disk constraints are being addressed with bounded
preservation and target selection, without removing test dependencies or guards.

Detailed evidence:
[build](verification/2026-10-02/p03-http-stage-b-production-build/REPORT.md),
[storage/migration/GUI/attachments](verification/2026-10-02/p03-http-stage-b-current-runtime-first-five/README.md),
[shutdown](verification/2026-10-02/p03-http-stage-b-current-runtime-shutdown/README.md),
[selected-search retry](verification/2026-10-02/p03-http-stage-b-current-search-gui/README.md),
[reviewed GUI images](verification/2026-10-02/p03-http-stage-b-current-ui-images/README.md).

## Roadmap and remaining work

The [inventory](COMPONENT_INVENTORY.md) maps28 functional ownership families and
169 explicit Rust workspace members plus supporting/staged packages. Coverage of
the inventory is not evidence that all those services have been extracted.

| Sequence | Remaining outcome |
| --- | --- |
| P00/P00M | Maintain exact source/provenance/evidence and close normalized semantic ownership gaps. |
| P01/P02 | Retain accepted storage/manual migration and search; finish their bounded gaps and related workspace services. |
| P03 — current phase | Finish consumer/replacement/lifecycle gates, HTTP/MCP/authority custody, negotiated dependency services and transport limits. |
| P04–P06 | Independently install native configuration/authentication/credentials, model catalog/providers/inference, remaining metadata and attachment backends. |
| P07–P09 | Extract reconstruction, live history/context/prompts, compaction and summarization. |
| P10–P12 | Separate permissions/approvals/policy, process execution/sandbox mechanisms, native tools/routing/code mode. |
| P13–P16 | Separate MCP/connectors/skills/marketplace, session/turn/multi-agent orchestration, background services/extensions and domain events/observability. |
| P17–P18 | Complete clients/presentation/cloud features, SDK/version/dependency compatibility, installation/removal/upgrade/failure behavior and release packaging. |
| P18U | Deliver installed upstream maintenance plus independent recovery, then integrate a real later upstream revision and prove incompatible/failed-update rollback preserving custom plugins and state. |
| P19 | Clean-install real custom-plugin/GUI acceptance, regression/security/migration gates and final minimal-kernel dependency audit. |

V1 permits restart activation; live replacement of active sessions is not required.
The kernel may own bootstrap, compatibility, supervision, capability/security
ceilings and minimal ordered transport. Session/model/tool/history business logic
must not be silently left there. The final minimal host must not link native core
implementations merely to preserve an “everything is a plugin” label.

[Upstream maintenance](UPSTREAM_MAINTENANCE.md) is required. Provenance and offline
impact-planning support exist; the installed updater, real later-revision integration
and recovery demonstrations do not. No periodic polling, unattended live deployment
or automatic activation has been enabled.

Development continues from the ordered ledger with verified milestones published
separately from unfinished source. Execution is finite and may require continuation;
this is not a promise of a permanently running unattended service.
