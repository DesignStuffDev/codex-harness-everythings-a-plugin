# Codex Harness Compartmentalized: upstream provenance

- Upstream: https://github.com/openai/codex
- Pinned revision: `d42056091aded7feb1d88ac7e83972108b2aa478`
- Acquired: 2026-09-30, in the selected managed Linux cloud environment.
- Target repository: https://github.com/DesignStuffDev/codex-harness-everythings-a-plugin
- Import: `git archive` of the pinned upstream revision into the provided empty
  checkout. The target's `origin` and unborn `work` branch were preserved. No
  embedded upstream `.git` directory was imported.
- Review baseline: after substantive implementation, fetched that same commit
  from the preserved local upstream repository and set `work` to it with a mixed
  reset. This records the original source as the Git baseline while retaining
  every implementation edit in the working tree. No new commit or remote write
  was created; the target's `origin` remains unchanged.
- Upstream `LICENSE`, `NOTICE`, and `AGENTS.md` are retained. New component work is
  under the repository's Apache-2.0 license. No DeepSeek or Cordis code is used.

This is an independently developed harness fork. Compatibility with the installed
official Codex desktop application's engine-loading mechanism is not established
or part of this project.

The upstream snapshot is a baseline, not evidence that existing crates support
independent installation or runtime replacement. The component inventory records
those distinctions and the validation still required.

## Recovery publication history

The source publication uses a new root import commit whose tree is exactly the
pinned upstream tree. Separate sibling checkpoints preserve the recovered primary
source and isolated unverified work. This avoids inventing the missing ancestry
of the shallow local baseline. The import records the original upstream revision;
it does not claim authorship of OpenAI's source. Both original local branches,
working edits and the pre-publication Git bundle remain preserved.

At recovery publication, `main` was the recovered component/GUI checkpoint.
Later accepted source and extraction checkpoints are recorded in
[UPSTREAM_MAINTENANCE.md](UPSTREAM_MAINTENANCE.md); the original upstream pin above
has not advanced. Its [current provenance closure](UPSTREAM_MAINTENANCE.md#current-provenance-closure)
distinguishes the existing immutable lineage records from the still-required
complete current ownership/evidence closure. The pinned checkpoint index and
read-only checker are implemented maintenance support, with unresolved semantic
and update-acceptance obligations recorded explicitly. The separate
`wip/recovered-next-components-20260930` branch preserves newer unverified source;
its README and WIP_STATUS explain that copied older evidence is not proof of its
newer implementation. Neither branch claims complete harness compartmentalization.
