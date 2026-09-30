# Native component authentication validation

- Toolchain: Rust 1.95.0 via `/workspace/toolchains/env.sh`.
- Focused command: `just test -p codex-login component_auth`.
- Focused run ID: `5624fa02-ad7d-4191-88b6-779d69908bc8`.
- Focused result: 6 passed, 243 filtered out, 0.424 seconds of test execution.
- Regression command: `just test -p codex-login --status-level fail --final-status-level fail`.
- Regression run ID: `b0a14dd7-03c2-499d-855e-8e3a7ae5c449`.
- Regression result: 249 passed, 0 skipped, 72.258 seconds of test execution.
- Full regression report: `login-junit.xml`.

The six new tests compile a std-only Rust credential plugin in a temporary project outside the repository, install its package, remove original package/source, and exercise native AuthManager acquisition/refresh and policy boundaries. Credentials are dummy fixture values. This evidence covers credential acquisition/refresh replacement and native login regressions; it does not establish replacement of OAuth login, credential stores, or all authentication internals.

The initial focused attempt failed during dependent-crate compilation because the 32 GiB cloud volume was full. The coordinator reclaimed inactive compiler incremental caches, and both successful runs above completed afterward.
