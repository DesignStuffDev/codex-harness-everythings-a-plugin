# Native token-install input repair and remaining build blocker

The archive/metadata blocker is closed; native validation is still blocked before
any tests execute. The eight-path auth change remains local, unaccepted and
unpublished. This checkpoint publishes documentation and evidence only.

## Exact current source and prior publication

Original repository: DesignStuffDev/codex-harness-everythings-a-plugin. The coherent
adopted/formatted candidate contains 8,980 mapped paths, source-map SHA256
`b75efc9751977009a9b28f2893c120db45a8f6e79f4993125e3685ae90d6cf5b`.
Candidate manifest SHA256:
`5bc49e31186aa91be93e7c1ffb356d4a7e656276a643881896664a07aefc06fc`.
Original index SHA256:
`0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`.

Prior published/read-back WIP is `25e9732a7350d31551ff58181b23d747b8c1dcf0`,
tree `c178f8de696759e3fa372bbfca6473f8963dce8d`, on
`wip/p03-process-final-and-mcp-preservation-20261002`. Main stays at
`781080f7e3c8bfe1953378001d777dff33d74bc3`. The eight native working paths and all
unadopted proposals are excluded from this documentation selection. Preserve
original source/index, accepted CLI/manager, both worktrees and recovery archives.

## What passed

After the earlier focused exit 102 on missing `alsa 0.11.0`, a bounded repair added
227 genuine current-lock registry archives whose unpacked source manifests and
historical aliases already existed. Logical bytes: 93,459,106; allocated payload
bytes: 93,941,760. Ten bounded chunks and final exact verification all passed.
No source extraction or Cargo operation occurred in that repair. The remaining
166 entries without source/history bindings were left untouched. All 902 earlier
archives and 994 sealed evidence files remained byte-exact; combined 1,129 archive
payloads plus marker allocate 229,072,896 tmpfs bytes. Repair evidence allocated
1,515,520 bytes at sealing; helper maximum RSS was 39,727,104 bytes and memory events
were unchanged. This establishes exact inputs, not native acceptance.

Then the actual production-toolchain command ran once:

```text
cargo metadata --format-version 1 --all-features --filter-platform x86_64-unknown-linux-gnu --locked --offline
```

It ran in the original codex-rs directory under the unchanged source wrapper and
strict runner, returned 0, emitted 1,315 packages / 7,058,322 bytes, and retained
candidate source, original index and memory-event identities. Output SHA256:
`cb4d41389ab89df7a8a6ba455dcee4d51120fa681e78e6c66076f5861c58f540`.
All-features metadata resolution did not request an all-features compilation.
No compiler or native tests ran in this metadata probe.

## What failed afterward

The fresh focused retry used the unchanged planned entrypoint:

```text
just test --locked --retries 0 -p codex-cli -p codex-login --lib -E 'package(codex-login) & (test(auth::manager::auth_install_tests::) | test(auth::manager::auth_source_tests::) | test(auth::storage::ephemeral_install_store::tests::))' --test-threads=1
```

Suffix `03` returned 101 at its first dependency compile, proc-macro2 1.0.106:

```text
error: output file .../target/debug/deps/libproc_macro2-92b5deb833e57d8a.rmeta is not writeable -- check its permissions
```

The restored metadata target is mode 0444. **Zero native tests executed.** The
8,980-path source and all observer-bound inputs stayed unchanged; no new OOM or
sampling error occurred. The strict runner reaped only its own command, with
return code 101 and no runner error. This preserves cleanup evidence for this
failed invocation; it is not a passing suite or general cleanup claim. The final
observer recorded persistent free 3,957,968,896 bytes and cgroup usage
8,428,298,240 / 17,179,869,184 bytes. Observed peak was 8,521,949,184 bytes, sampled
rather than a guaranteed compiler peak.

The proc-macro2 canonical rlib alias became absent during the failed interval,
without a syscall trace identifying the unlink actor; 1,046
relocation aliases remain intact. All 1,047 preserved temporary rlib payloads remain.
The failed fingerprint state and absent canonical output are retained for diagnosis;
no cleanup or timestamp repair has concealed this failure. The old core/TUI absent
pairs remain separate known debt. Protected immutable backing cannot safely be made
writable in place merely to satisfy rustc.

## Resume condition and limits

Native admission is closed pending a recoverable writable-output handoff and a
renewed whole-sequence budget. Keep exact recovery payloads while giving Cargo
independently owned writable outputs; do not falsify fingerprints or source mtimes
to force reuse. Account for the potentially dirty reverse dependency closure,
persistent and tmpfs copies, compiler/linker overlap, scoped lint, production
CLI/manager, current-source runtime/UI regressions, evidence and recovery floor.
No further unchanged Rust attempt is admitted by this checkpoint.

The read-only retained-fingerprint graph gives a conditional 1,483-node reverse
closure, including 792 relocated rlibs (2,463,023,104 bytes) and 645 restored metadata
payloads (575,684,608 bytes). With the previous auth envelope this is 6,836,095,464
bytes of potential coexistence. Including all restored metadata seed nodes gives
a broader 7,127,788,008-byte comparison. Neither is a proven Cargo dirty schedule
or complete peak bound. Four GiB additional usable persistent space is a planning
target pending reviewed handoff/admission, not a guaranteed build result. No upload,
archive retirement or environment replacement follows from this estimate.

The metadata probe, repair and compiler failure each have separate original
receipts. They do not prove native external-token installation, full P03, whole-
harness extraction, real-provider behavior or any new GUI behavior. The installed-
overlay lifecycle gate is already complete within its recorded exec-boundary scope;
collector and broader updater/planner/scope-validator work remain paused. In-app
Browser is unavailable in this cloud task; no fresh Chromium or browser evidence
was produced by this input repair. Required native and later UI gates remain open.

Private evidence roots use R=/workspace/recovery-backups/20260930T165936Z and
A=/workspace/acceptance. EVIDENCE.json binds original paths, exact hashes and small
result summaries; large source manifests/logs remain preserved at those paths.
These private archives are VM-local recovery checkpoints. Publication gives external
durability to the selected public documentation/evidence, not those private bytes.
