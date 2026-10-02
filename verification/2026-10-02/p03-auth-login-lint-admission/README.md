# Login scoped lint — resource admission blocked

The proposed `just fix -p codex-login` did **not start a compiler**. Its single resource check required2 GiB unused RAM and observed1,849,307,136B after six exact fsync/DONTNEED cache hints. Effective headroom and disk passed their separate floors; cgroup OOM10/kill5 was unchanged. The hints did not change artifact bytes and do not guarantee reclamation.

This is neither a Clippy failure nor a lint pass. No source-fixing command ran. Keep the completed297-test login evidence against its actual unchanged source. Further lint requires materially improved measured resources; the floor was not lowered and this attempt was not replayed.

The original receipt and advice remain under `/workspace/acceptance/p03-auth-login-fix-01.*`. The guarded plan is `R/p03-auth-login-lint-plan-01`; R is the preserved recovery directory. Provider regression is an independent lower-cost gate, not a substitute for lint.
