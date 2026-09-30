# Core component composition gates

The combined integration gate ran after the storage composition and attachment selection
changes, including attachment staging cancellation handling.

```sh
source /workspace/toolchains/env.sh
cd /workspace/codex-harness-everythings-a-plugin/codex-rs
just test -p codex-core --test all -E 'test(component_model) | test(component_attachments) | test(/suite::resume::/) | test(/suite::compact_resume_fork::/)' --retries 0
```

Result: **18 passed, 0 failed** (2,212 other tests excluded by the filter), nextest run
`a2dbb1f0-541f-40f7-b8b6-3f1042ecdbc7`, standard test build, local nextest profile.
Build: 3m 28s. Test runtime: 3.100s.

- Four installed model component cases: streamed text/tool execution/resume/deselection,
  explicit failure without backend fallback, process cancellation, and session isolation.
- Four attachment component cases: live upload and resumed reuse, explicit injected store
  precedence, isolation, and native inline fallback after upload failure.
- Eight native resume cases, including both local history formats and the store default.
- Two native compaction/resume/fork history regressions.

An initial compilation attempt exposed an incorrect `Feature` import in the new
`component_persistence.rs`; it was corrected to `codex_features::Feature` before the
successful run. The only warnings in the successful run were existing unused imports in
`openai_file_mcp.rs` and `scenarios.rs`.

Artifacts: [log](core-components-and-resume.log),
[JUnit](core-components-and-resume.junit.xml).

The library gate used runtime module names (the source filenames are not test names):

```sh
just test -p codex-core --lib -E 'test(/tools::registry::tests::/) | test(/context::contextual_user_message::tests::/) | test(isolated_delegate_resets_default_attachments_but_keeps_explicit_store)' --retries 0
```

Result: **36 passed, 0 failed** (2,659 other tests excluded), nextest run
`4e705ca3-86e1-4406-ab09-8410ac570158`. Initial library build: 4m 16s;
the corrected filter reused that binary. Test runtime: 0.383s.

This includes explicit tool replacement, retained tool-policy and reserved-name checks,
component context not granting user authorization, and isolated delegate attachment
selection preserving caller-injected implementations. Artifacts:
[log](core-registry-context-delegate.log),
[JUnit](core-registry-context-delegate.junit.xml).
