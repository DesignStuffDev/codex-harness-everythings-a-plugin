# Recovery core regression diagnosis

The original recovery run is preserved in `recovery-core-lib-20260930.log`.
Its four failures are not recorded as passing. The evidence below motivated two
test-only portability changes and an external test runner; production behavior,
injected proxy settings, and sandbox constants remain unchanged.

## Descendant termination and container init

The failures were:

- `exec::tests::kill_child_process_group_kills_grandchildren_on_timeout`
- `exec::tests::process_exec_tool_call_cancellation_allows_sigterm_cleanup`

Read-only `/proc` inspection found the reported PIDs 315423 and 315461 in state
`Z`, with parent PID 1 and raw wait status 9 (SIGKILL). PID 1 was `tail` and had not
reaped them. Their process groups were 315422 and 315460 respectively. These were
terminated descendants, not running commands. However, their PIDs still existed,
so the tests' strict `kill(pid, 0) == -1 && errno == ESRCH` checks correctly failed.
The SIGTERM test had already passed its cleanup-marker assertion before failing
the descendant-disappearance assertion.

Both `core/src/exec.rs` and `core/src/exec_tests.rs` matched pinned upstream
`d42056091aded7feb1d88ac7e83972108b2aa478` byte for byte. No execution implementation
or strict disappearance assertion was changed. The new
`component-sdk/tests/subreaper_runner.py` supplies the orphan reaping normally
provided by container init. It owns the command and adopted descendants, retains
the command's actual wait status, forwards interrupts, and reports a bounded
post-command drain failure without killing descendants. An orphan's later exit
cannot overwrite a failed command's result. Its real-process tests cover failed
status preservation, signal exit status, successful drain, and `/proc` absence.

This runner does not establish that native Codex independently reaps arbitrary
grandchildren when hosted under a non-reaping PID 1.

## Network decider fixture depends on DNS

`session::tests::managed_network_proxy_decider_survives_full_access_start` sent
a request to its own local managed proxy for `example.com`. It received a native
`403` response with reason `not_allowed_local`, source `baseline_policy`, and zero
decider calls. A subsequent read-only DNS lookup returned `EAI_AGAIN`.

The unchanged native implementation in `network-proxy/src/runtime.rs:1128`
deliberately treats a DNS error or timeout as non-public. In
`network-proxy/src/network_policy.rs:363`, the decider is called for `NotAllowed`,
but not for `NotAllowedLocal`. The response therefore does not demonstrate a lost
decider or prove an upstream cloud proxy rejected the request. It is consistent
with native fail-closed DNS handling. The test function and
`Session::start_managed_network_proxy` matched pinned upstream before this fix.

The fixture now uses a public IP literal in both URI and Host header. The local
decider still must run exactly once and deny the request, and the response must
still be `403` with the blocked header. No upstream connection is allowed. This
follows the existing DNS-independent fixture approach in
`network-proxy/src/runtime.rs:1436` and does not change network policy.

## User-shell fixture assumes no inherited proxy

`session::tests::user_shell_commands_do_not_inherit_managed_network_proxy`
expected `HTTP_PROXY` to be absent. The cloud executor legitimately injects that
variable. `core/src/tasks/user_shell.rs:169` builds the inherited environment and
strips proxy variables only when the Codex-managed marker is present. Its exec
request separately sets `network: None`. This implementation and `exec_env.rs`
matched pinned upstream byte for byte, as did the test function before this fix.

The test now captures the inherited `HTTP_PROXY` baseline and requires the user
shell to preserve it (or the original `not-set` sentinel if absent/empty). It
still requires a managed proxy on the session and successful shell execution;
substitution with the session's managed proxy would fail the equality assertion.
No process environment variable or injected security setting is mutated.

## Revalidation

Keep the original run and its failures as evidence. Run the unchanged exec tests
and corrected proxy tests through the subreaper, then rerun the complete core
library test selection. Example, from `codex-rs` with the verification toolchain:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 ../component-sdk/tests/subreaper_runner.py \
  --report /tmp/recovery-core-subreaper.json -- \
  just test -p codex-core --lib --test-threads 2 --retries 0 \
  --status-level fail --final-status-level fail
```

Use a fresh report path on every invocation. This diagnosis does not claim those
follow-up Rust checks have already passed; their terminal reports are separate.

### Completed corrected gate

The complete core library selection subsequently passed all 2,694 executed cases,
including both unchanged strict descendant-disappearance tests and the two
corrected proxy fixtures. Run `30e4ea84-f664-40f8-ba22-fac95cb27b80` had zero failures,
errors or retries; one helper was excluded. The subreaper recorded command exit 0
and no drain error. See `recovery-core-lib-portable-20260930.json`, its JUnit/log,
and its actual-source manifest. The earlier failed result remains separate.
