# Native auth reload — complete login regression

The current reload-source ownership change passes **297/297 tests**, zero retries and zero skipped tests. The five focused reload cases are included in this total. Both the login library and its `all` integration target ran through `just test`, with one test at a time and the existing local retry/timeout policy unchanged.

Source map `0742356b9ff442a4d38f6704928930602f92df0fdfe89bfb250897fda470729f` (8,948 files) stayed unchanged. The strict subreaper returned0 without a runner error; its one adopted exit was0. No new cgroup OOM or OOM-kill event occurred. The focused login library was reused; the new integration ELF and actual invocations are bound in EVIDENCE.json and CASES.json. The CLI library was a compiled feature anchor, not executed CLI coverage.

No selected invocation observed the existing sandbox-network guard set. A conditional occupied-port early return is still not independently attested. This serial run is not default-concurrency coverage. Counts must not be added to overlapping focused/historical runs as a unique project total.

The retained production CLI is unchanged source7a, older than the tested auth source. Provider regression, scoped lint and rebuilt-host/plugin/GUI verification remain separate gates. This native in-memory ownership work does not extract authentication or implement durable conditional credential writes. No installed updater or upstream integration was exercised.

Raw receipts remain under `/workspace/acceptance/p03-auth-login-full-01.*`; their hashes are recorded. Published evidence does not make the VM-local binary archives externally durable.
