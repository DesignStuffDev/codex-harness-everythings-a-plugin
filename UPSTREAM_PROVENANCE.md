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
