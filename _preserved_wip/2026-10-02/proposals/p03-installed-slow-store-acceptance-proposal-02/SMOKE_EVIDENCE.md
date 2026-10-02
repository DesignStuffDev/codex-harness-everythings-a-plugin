# Proposal02 synthetic wire evidence

Executed only the relay against a tiny Python protocol peer. No real Codex host,
native storage, component install, browser or long-duration acceptance was run.
The held synthetic write waited about 0.2 seconds; this is **not** the >45-second
Launch gate. Exact acceptance boundary remains installed fixture before native
forwarding for the planned real-host test.

Results: four cases passed in 1.271659146 seconds under the unchanged outer
subreaper using its default five-second drain. Runner exit 0, command exit 0,
`runner_error: null`. The only raw reaped process was the smoke driver's exit 0.
Each relay separately waited on its exact synthetic native child and the driver
confirmed its PID absent, including zombies. No emergency cleanup was used.

1. Gated forward: native did not see the retained request before release. Exact
   logical payload and physical request bytes matched after release. Native
   append reply and shutdown/exit observed. Both task outcomes recorded before
   finished; pending input was deliberately canceled after native EOF.
2. Clean native EOF: no selected write, normal shutdown receipt, native zero
   exit, both task outcomes observed; owned pending input cancellation allowed.
3. Late input error: synthetic native closed stdout, retained its process for a
   controlled delay, and host supplied malformed JSON while relay awaited exit.
   Corrected relay exited 1, recorded `JSONDecodeError` from `host_to_native`,
   and wrote **no finished receipt**.
4. Preserved-original negative control: the exact original relay was subjected
   to that same delayed-exit/error sequence. It incorrectly exited 0 and wrote
   finished, reproducing the reviewer-reported bug. This expected false success
   is diagnostic evidence about proposal01, not a passed cleanup claim.

Runtime report:
`/workspace/acceptance/p03-installed-slow-store-synthetic-02/report.json`
SHA256 `52ad3b188f7c0b6237ed3643371ec5ace5dac653825fd555152497e3c25bdeee`.

Strict report:
`/workspace/acceptance/p03-installed-slow-store-synthetic-02.subreaper.json`
SHA256 `5a3734ccb66c62211da296bd3799f3ee11da12d519ff778f6a3b418acabfe097`.

Script inputs were hashed before and after execution by the smoke driver and
remained unchanged. Proposal01's final manifest remains
`05eacbd2f21720dd0776d566c306a7e4771f83c17e74a18dda629c1222fa2b4a`.
Original proposal sources remain intact. Evidence files and archives are in this
VM only; this task did not publish them or establish external backup durability.

The second blocker (hold origin) was fixed in the real-host acceptance driver:
wait for the fresh manager acknowledgment, capture its observation monotonic
time, and compute both the 46-second hold deadline and >45-second assertion from
that observation. This has AST/source-review evidence only until root runs the
real manager and GUI against the rebuilt host.
