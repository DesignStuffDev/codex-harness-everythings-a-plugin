# Development capacity and cold-artifact preservation

## Current infrastructure direction

The latest instruction makes durable expandable development capacity the exclusive
priority and allows a future separate native worker. Preserve the original VM and
all unique work/recovery state. [NATIVE_BUILD_WORKER_HANDOFF.md](NATIVE_BUILD_WORKER_HANDOFF.md)
now defines source custody, clean dependency reconstruction, exact native/runtime
recipes and missing worker access/portability prerequisites. It does not provision
or admit a worker. No Rust attempt, cache rework, private export or product feature
is authorized by this documentation. The completed original-VM review below remains
historical evidence; do not repeat it or apply its warm-cache allowances to a clean
worker. Private cold-archive transport is a separate retention workstream, not a
prerequisite for the recommended source-first build-worker setup.

Updated 2026-10-05 after the additional registry repair and a failed native retry.
**Native admission is closed pending writable compiler-output handoff and a renewed
budget for the dirty dependency closure.** The exact paused installed-overlay gate
is complete; the next priority remains the coherent eight-path native auth sequence.
Collector and broad updater/planner/scope-validator work stay paused. Preserve both
worktrees, accepted binaries, original index, all recovery archives and failed-run
state. No upload, replacement VM or cold-archive retirement is implied.

## Current blocker after successful offline metadata

[Current evidence](verification/2026-10-05/p03-token-install-build-inputs/README.md):
227 genuine current-lock archives (93,459,106 logical / 93,941,760 allocated bytes)
were restored without extracting sources. The previous 902 remain exact; combined
1,129 payloads plus marker occupy 229,072,896 tmpfs bytes. The 166 entries lacking
existing source/history bindings were excluded. The exact standalone offline
metadata command passed: 1,315 packages and 7,058,322 output bytes, unchanged source
and original index, no new OOM. It ran no compiler or native tests.

Focused retry `03` then exited 101 at the first dependency compile because restored
`libproc_macro2-92b5deb833e57d8a.rmeta` was mode 0444. No native tests ran. The failed
interval left one proc-macro2 canonical rlib name absent; the specific unlink actor
is unproven. All 1,046 other relocated
aliases and all 1,047 protected backing payloads remain. Preserve the failed
fingerprint state and existing recovery archives. Simply making protected backing
writable would allow compiler output to overwrite recovery input; it is not an
approved repair. Resetting timestamps/fingerprints to pretend cache freshness is
also not a valid reuse proof.

Required admission work: isolate writable compiler output ownership while retaining
exact protected input bytes; determine the affected dirty dependency descendants
for focused/full tests, lint and production; count both retained and replacement
payloads, link overlap, ongoing evidence/recovery and runtime/UI working state
across persistent disk, tmpfs and the shared 16 GiB memory cgroup. No next Rust
attempt is admitted by the old comparator alone. The independent bounded diagnosis
and current capacity comparison remain inputs to that review, not build proof.

| Last retry observer measurement | Bytes |
|---|---:|
| Persistent available after failed retry |3,957,968,896|
| Cgroup current / limit after failed retry |8,428,298,240 /17,179,869,184|
| Sampled peak cgroup current during retry |8,521,949,184|
| `/tmp` available |5,808,566,272|
| `/dev/shm` available |9,212,276,736|

These are historical point/sample observations; no OOM counters changed, but no
compiler/linker peak or complete-sequence success was demonstrated. Do not repeat
the same failed build or retire unique recovery material to recover headroom.

## Current deduplicated phase ledger — no admission

[Handoff design](NATIVE_OUTPUT_HANDOFF.md) and [exact evidence](verification/2026-10-05/p03-writable-output-capacity/README.md)
replace the earlier aggregate comparisons as the current planning reference. The
CoW probe measured ENOTSUP within workspace and tmpfs, EXDEV across them; no copy
saving is credited. Its baseline is 3,957,055,488 persistent free bytes, 5,808,566,272
/tmp free, 9,212,276,736 /dev/shm free and 8,647,585,792 hard-unused cgroup bytes.
The two tmpfs mounts share one 17,179,869,184-byte limit. This timestamped baseline
is retained for the following arithmetic; root's later measurements do not silently
replace it.

Only proc-macro2 compilation was observed. The 895 restored metadata seeds / 963
direct consumers / 1,483-node reverse closure remain a conditional graph, not a
Cargo dirty schedule. The following byte scenarios deduplicate paths across phases;
new test/production ELFs use rounded logical allocations. No output-slot replacement,
tmp output placement, future reuse or old-backing retirement is credited.

| Phase | Distinct rlib increment | Metadata increment | Test ELF increment | Cumulative new outputs | Peak scenario with allowances |
|---|---:|---:|---:|---:|---:|
| Focused |2,046,898,176|575,684,608|354,086,912|3,419,262,976|5,122,912,256|
| Full login |0|0|139,878,400|3,559,141,376|5,262,790,656|
| Provider |0|0|111,263,744|3,670,405,120|5,374,054,400|
| Lint |0|0|0|3,670,405,120|5,374,054,400|
| Production |416,124,928|0|0|4,729,806,848|6,433,456,128|

Core/TUI missing pairs are counted once: 442,593,280 bytes. Production adds a
643,276,800-byte gross CLI/manager comparator. Lint's 429,985,792 bytes of other
regular outputs are a replacement/overlap comparator, not another lasting copy.
The peak column adds growth 536,870,912, shared source/evidence 134,217,728, recovery
floor 268,435,456, historical ordinary-archive overlap 227,254,272 and other linker/
build-script scratch 536,870,912. These are estimates, not measured upper bounds.
**6,433,456,128 bytes is neither an exact minimum nor guaranteed sufficient space.**
The previous 3.797/6.836/7.128 GB totals remain historical comparisons, not admission.
A global handoff of all 1,047 rlibs is not bounded by the all-metadata-seed scenario;
five rlibs lie outside that recorded upper output set. Reject global eager handoff.

Already-resident selected tmp inputs total 3,559,895,040 bytes. Do not charge their
existing archives again or treat tmpfs free space as independent memory. The old
non-shmem peak plus current probe shmem plus a 2 GiB contingency is 12,834,566,144
bytes, a provisional comparison only. Extra writable metadata on tmp would add
575,684,608 bytes; routing 792 rlibs there would add 2,463,023,104, with no routing
proof. Compiler, browser and compressor phases must not overlap. Optional new-host
proof archival needs a separate 192 MiB compressed estimate and 643,276,800-byte
full-restore probe; the accepted-host archive is already paid.

After successful production, the twelve existing runtime recipes plus the separate
new auth consumer still require current-source evidence. Known package copies are
666,832,896 bytes within a 1.5 GiB provisional runtime tmp allowance: 512 MiB state/
profile growth, 128 MiB browser transient and 272,691,200 unassigned. Add auth-consumer
64 MiB tmp and 128 MiB process estimate separately; that excludes native host memory.
A fresh 4 GiB hard-unused runtime/browser guard and the shared 128 MiB evidence
allowance are provisional; count earlier failed receipts and later screenshots.
Neither these estimates nor historical Chromium/deterministic-model passes prove a
new runtime peak, in-app Browser behavior or live-provider behavior.

The staged alternative remains inadmissible. Corrected historical accepted-inode
allocation is 634,273,792 bytes; full tmp restore is 643,276,800. No relocation
credit is approved. Even granting old-inode relocation, the focused scenario with
required durable metadata fallback is short 531,582,976 bytes. A later full-login
selected-rlib recreation scenario is short 766,865,408 after the hypothetical
post-focused handoff. Preserving a new archive, fully restoring it and releasing
inactive old tmp backing can rebalance RAM only after a complete admitted command;
it cannot fund the initial peak or guarantee subsequent reuse. The sealed erratum
corrects original ledger attribution/arithmetic without changing its original bytes.

## Concrete capacity/access decision

An additional 4 GiB usable persistent capacity remains a planning target, not a
native pass guarantee. The provisional 28 historical binary/proof archives occupy
4,320,051,200 bytes; certified reclaim is zero. Preserve full source/workspace,
current accepted baseline and new cache recovery locally. The small 162,336,768-byte
disposable-candidate review is also uncertified and cannot close the multi-GB gap.

The exact private destination/instance identifiers are intentionally absent from
public docs. The sealed access card bound in EVIDENCE.json requests only its single
exact HTTPS hostname, preserving restricted mode, package-manager and unrelated
grants, normal inherited HTTPS proxy and certificate verification. Raw TCP grants
are not required merely for HTTPS CONNECT. An operator must provide a supported
same-instance apply/private DNS-route workflow without reset/replacement; no such
apply/resize/export tool is exposed here. Read back the actual task-bound enforced
policy and readiness afterward; publishing saved configuration alone is insufficient.

Then establish an existing scoped private DAV account/directory and 12 GiB usable
quota through the private credential surface. Transport/TLS and authenticated
read-only directory/quota checks remain separate gates. Only separately authorized
transfer may start the encrypted pilot, independent download/decryption/full restore
and recovery-key verification. Remote checksums alone do not justify retirement;
explicit retention approval and fresh local ownership checks are still required.
No unchanged access retry, upload, public tunnel, paid provider or archive deletion
is authorized by this review. If no same-instance route or genuine capacity change
is available, preserve state and report that material blocker before any handoff.

## Earlier completed repairs and guard measurements

[First repair](verification/2026-10-05/p03-capacity-repair/README.md) restored895 exact
metadata files and902 genuine registry archives (827,656,451 logical bytes), then
reclaimed1,786,339,328 allocated bytes from55 verified duplicate-only paths.
[Second repair](verification/2026-10-05/p03-capacity-restored/README.md) archived1,047
selected unchanged rlibs, fully restored them to verified temporary backing, and
journaled canonical-path aliases. Archive565,190,455 bytes/SHA256
`0691ef18f30d705eacb611a0637e59dcecb241b04488952767cd72323254c353` remains durable
on this VM. Backing allocation2,634,592,256 bytes; all1,047 aliases and3,141 journal
events verified. Source/index/accepted binaries and16,906 fingerprint inputs stayed
unchanged. No recovery archive or unique evidence was retired.

| Completed postcheck measurement | Bytes | Reviewed requirement |
|---|---:|---|
| Persistent free |4,126,289,920|3,850,000,000|
| Hard-unused cgroup memory |8,842,485,760|8GiB /8,589,934,592|
| Memory current / limit |8,337,383,424 /17,179,869,184|Shared by compilers, tmpfs and runtime|
| `/tmp` free |5,831,925,760|Remeasure before each phase|

These values passed the repaired-headroom check at the recorded moment. They do not
reserve capacity or demonstrate Cargo reuse, native build/test success or future
peak memory. Current-epoch OOM/max/kill counters were zero. Preserve prior failures.

## Earlier conditional native phase envelope — requires renewed admission

The earlier reviewed 9-root auth-specific plan used **3,797,387,752 bytes** of
persistent planning allowance against a **3,850,000,000-byte** starting-free
requirement. It did not prove Cargo would reuse repaired outputs; the observed
read-only output failure now requires a dirty-closure and writable-overlap review:

| Term | Bytes |
|---|---:|
| Missing ordinary core/TUI pairs |442,593,280|
| CLI/login/integration/provider test ELFs |605,217,752|
| New production CLI + manager |643,274,256|
| Aggregate growth/variant allowance |536,870,912|
| Source/evidence allowance |134,217,728|
| Serial new proof archives |402,653,184|
| Untouched recovery floor |268,435,456|
| Historical ordinary-output overlap |227,254,272|
| Additional linker/output-parent scratch |536,870,912|

The442.6MB pair debt is already inside the older792.5MB focused-output comparator;
never add both totals. Warm affected output slots are reused, with replacement
scratch charged; accepted proof executables and archive bytes remain preserved.
Actual runtime package-copy inventory is666,832,896 bytes; the reviewed new temporary
runtime envelope is1.5GiB including state/browser/fixture growth. Run compiler,
archive verification and GUI/runtime phases serially. Recheck actual residual
space, owners, package hashes, source and memory at each transition. Do not kill Rust.
The512MiB growth and512MiB scratch values are explicit contingencies, not measured
upper bounds. Unexpected selectors/overruns require review before another phase.

Required sequence: coherent eight-path adoption/format; grouped focused then full
login and provider `just test`; scoped login lint; locked/offline production
CLI/manager; fresh-source installed storage/migration/GUI/attachment/slow-shutdown/
recovery/held-ownership gates plus the real auth-install consumer. Preserve failed
attempts and rollback proof. The giant TUI6/App Server16 and other P03 debts are
separate; this plan does not admit them or a full workspace suite.

Original8GiB-free recommendation is a conservative fallback, not a mandatory
minimum. Old production's near16GiB cgroup total included10.35GB standing shmem;
selected repaired/moved backing is much smaller. Same16GiB capacity is plausible,
not a claim that a future compiler peak is known. Jobs1 already applied historically.
Fresh guard measurements and actual native results must decide success.

Private exact card: R/p03-capacity-sequence-20261005-02/PHASED_REPORT.md;
separate seal MANIFEST.phased.json SHA256
`278b4741b379de56b08259c297a2ddde8bd943702e76659cc010039bade2ca1c`.
Do not overwrite canonical older MANIFEST.json, which remains37653c7a.
Completed repair seal: R/p03-cache-relocation-20261005-02/COMPLETE_MANIFEST.json,
SHA256 `8ce4184c7a95a18b3cb94bf465775e08a98add362970a8ae117eff2fbb58db6b`.

## Recovery and external-storage limits

Keep selected-rlibs.tar.zst, ARCHIVE.json, RESTORE.json, alias journals and ownership
receipts. If temporary backing is absent, the implemented/static-reviewed `rehydrate`
entry point restores exact archive bytes to temporary storage after fresh guards,
preserves current regular compiler outputs and emits a new epoch receipt. Returning
to ordinary persistent files uses guarded `recover` after separately admitting~2.63GB
plus reserve. Three disposable synthetic cases/18 phase calls passed: normal
restoration, missing temporary backing with newer regular output preserved, and
injected interruptions after real forward/reverse exchanges followed by recovery.
This is fixture behavior, not power-loss or production-disaster proof. Full
restoration during the actual repair also executed; keep these claims distinct.

No source or private artifact was uploaded. The PC/home destination remains unproven:
the one cloud DNS/HTTPS check failed (no address; proxy CONNECT403). No supported
same-instance resize/export route is exposed. Do not retry unchanged access, create
a public tunnel or expose Proxmox management. External storage remains an unproven candidate;
resolve writable-output ownership and re-admit the critical native sequence before
broader product work. External access remains a separate, unproven option.

## Historical observations before the completed repairs

The following snapshots, inventories and blocked conclusions describe their original
observations. The two repairs and phased guard measurements above supersede them
as current capacity facts; old receipts remain unchanged.

Snapshot: 2026-10-05 01:00:58 UTC. Values are measurements, not reserved resources.

| Resource | Bytes | Consequence |
|---|---:|---|
| Persistent filesystem total | 33,770,192,896 | No supported same-instance expansion operation was exposed |
| Persistent available | 306,339,840 | 37,904,384 above the 256 MiB reserve |
| RAM-backed `/tmp` available | 9,186,902,016 | Volatile and charged to the same memory cgroup |
| RAM-backed `/dev/shm` available | 9,441,349,632 | Not additional independent RAM |
| Cgroup current / limit | 2,894,921,728 / 17,179,869,184 | Not a compiler/linker peak measurement |

Current epoch OOM counters are zero. Prior counters and services did not survive
reattachment; this is not evidence that older processes shut down cleanly.

At that historical checkpoint, small Python checks were considered with
fresh admission: provisionally no more than 16 MiB total new durable writes,
256 MiB scratch and 128 MiB extra process memory, preserving the 256 MiB disk
floor. Count concurrent writes, failed-run evidence and package copies. This is not
blanket admission for every test or a new GUI run.

Latest completed0.4 runtime snapshot (2026-10-05 01:42:09 UTC): persistent
293,318,656 bytes, tmp9,170,685,952 bytes, hard-unused cgroup memory13,695,922,176
bytes, OOM/kill0 unchanged. A subsequent956,383-byte private runtime export is
preserved. These are point observations; remeasure before each new admitted stage.

After the next50-case pure final-scope run at2026-10-05 01:58:06 UTC:
persistent287,522,816B, tmp9,163,165,696B, hard-unused memory13,681,459,200B;
OOM/kill0 unchanged. No native admission or recovery retirement followed.

## Protected inputs versus conditional cold storage

The bounded inventory deduplicated allocations by device/inode and kept cross-class
hardlinks separate. Allocated bytes are not guaranteed reclaimable bytes.

| Class | Allocated bytes | Treatment |
|---|---:|---|
| 291 recovery archive inodes | 8,174,936,064 | Pool for individual review, not a deletion list |
| Historical shortlist of seven proof/cache archives | 700,407,808 | Subset of that pool with inspected prior receipts; still not cleared |
| Primary Rust target | 10,890,289,152 | Protected inputs/outputs, not assumed disposable |
| Acceptance artifacts excluding cross-class links | 4,131,401,728 | Mixed packages, evidence and state; preserve ownership individually |
| Cross-class hardlinks | 267,460,608 | Counted once; removing a name may free nothing |

**At that archive inventory: zero bytes certified for archive retirement.** The one process snapshot saw no open
archive descriptors or symlink backing, but had three access errors and cannot
certify inactivity. Refresh owner/reference/alias checks immediately before any
eventual removal. Current CLI, storage worker aliases, Rust toolchains, extracted
Cargo sources, recovery metadata, both worktrees and viewer setup remain protected.

The narrow 700 MB shortlist alone would remain below the historical native output
comparator. Individually clearing enough of the broader 8.17 GB pool could materially
help; no whole-pool retirement or guaranteed freed-space figure is established.

## Historical reason for the earlier closed native gate

The existing output/link-overlap/reserve comparator is **2,160,322,264 bytes** before
selected missing-input regeneration, lint/production closure growth, remaining
App Server/TUI gates and runtime proof. It is a lower comparison, not a sufficient
additional-disk request. Missing volatile backing includes:

- 1,875 metadata files, 2,601,776,224 logical bytes; 445 lack a paired live rlib.
- 11 static-library payloads.
- 1,484 registry archives, historically 269,908,290 bytes; extracted sources survive.

Do not recreate every old variant or assume paired rlibs restore exact metadata.
Intersect the actual next test/lint/production closures with the missing-input map,
bound regeneration plus simultaneous outputs/link peaks on both filesystems and
RAM, then admit the complete sequence. That sequence includes all eight auth-install
paths, scoped `just fix`/`just test`, affected login/provider/core/TUI/App Server
checks, production CLI/host rebuild, actual external auth/storage/migration and
GUI/session/shutdown/replacement proof, followed by retained evidence and rollback
artifacts. Complete workspace testing still requires separate approval.

Neither a sufficient extra-disk amount nor a RAM increase can honestly be certified
until those remaining closure costs are bounded. No unchanged Rust build/admission
was repeated. No available environment tool exposed a disk-size/resize operation.
An operator would need to identify a supported quota increase that preserves this
task's filesystem; a saved environment edit or guest resize command is not proof.

## Historical refined dependency audit, before restoration

A new read-only audit resolves nine recorded test/lint/production roots to2,268
unique fingerprint nodes, with no ambiguous/unresolved recorded edges. It narrows
recovery terms without running Cargo or reopening native admission:

- 895 selected missing metadata files:694,368,532 historical logical bytes. All have
 paired live rlibs. Exact ELF-section hashing authenticated16/76,049,503 bytes;
 remaining879/618,319,029 bytes were unprobed then. The later full probe matched all895;
 both completed repairs above supersede this missing-backing snapshot.
- The old eleven relocated rlibs are outside this union, but two additional ordinary
 core/TUI test-library pairs are absent:442,593,280 historical allocated bytes.
- 902 selected missing registry archives:133,287,919 compressed bytes. Their retained
 hashes match Cargo.lock and all902 unpacked source directories exist; the subsequent repair restored all902 exact selected archives. The preserved offline
 Inflector failure shows unpacked sources alone did not suffice.

Those rows are historical planning terms, not a sufficient free-space demand or
proof Cargo will reuse surviving outputs. New-source regeneration, library/linker
peaks, complete affected native tests/lint/production and runtime/retention growth
remain unbounded. No honest sufficient extra disk/RAM request follows yet. The
read-only audit and independent review are sealed at
`p03-native-closure-readonly-20261005-01/MANIFEST.json`, SHA256
`dabf8eaf994bf724ec7aeb27f86e3637c9c3ff40c77a7f32ac3cbca1ec3a1260` under the
recovery root. This supersedes indiscriminate whole-cache restoration as the next
investigation at that time. Native auth/P03 proof remains pending after repair; no
 recovery archive was retired.

## Google Drive and other offsite candidates

Drive exposes connector upload, metadata and download operations. The visible
interface exposes no quota-specific action; no profile or private files were read.
Quota, destination permissions, resumability, binary limits, buffering and transfer
of this VM's exact bytes through the connector remain unknown. Conflicting uploader
path/reference guidance was not resolved by an unauthorized test upload. Connector
availability does not prove a network path from the VM. Context7 is unavailable.

Drive could hold cold binary archives, not active build inputs or a live rollback
filesystem. Before using it or another provider:

1. Confirm authorized private destination, quota, supported bounded transport and
   retention/access controls. No account creation or purchase is presumed.
2. Freeze individual existing archive identities; freshly hash payloads and bind
   complete member manifests, modes/links, source versions and historical receipts.
3. Review sensitive content. Use authenticated encryption where exact private
   evidence must travel, with separately recoverable keys; protect manifests too.
   Credentials and private runtime state do not belong in public source evidence.
4. Stream existing compressed archives rather than creating another full archive
   or multi-GB encrypted copy on this disk. Bound buffers, chunk spooling and failure
   journals; prove the chosen transfer's behavior rather than assuming streaming.
5. Independently download the exact stored object, verify ciphertext/plaintext
   hashes and fully restore every selected object in isolated bounded scratch.
   Validate all members and reject unsafe paths, links, special files and expansion
   overflow. Exercise recovery without the candidate host.
6. Keep local originals until remote verification and full restoration pass, keys
   and manifests are independently available, and an explicit per-object retention
   decision permits retirement. Recheck ownership/aliases and measure freed blocks.
   Retain an agreed known-good local baseline.

The proposed first transport/restore pilot is an existing 26,936,562-byte historical
archive with 117,608,448 bytes of previously observed expanded allocation. Its
provisional 256 MiB tmpfs plan includes transfer/verification and metadata allowance;
encryption/spooling must fit or be rebudgeted. Allow 128 MiB process memory and
4 MiB durable receipts separately. This is a proposal only. Restore larger objects
sequentially using their own measured member bounds; do not inherit the small pilot's
budget or remove failed-run evidence to make a transfer fit.

## Future live-updater preflight

Reserve simultaneous **A + B + recovery + build/link/migration workspace + ongoing
writes + later-rollback rescue**, plus transfer/restore staging and safety floors.
Cap background CPU, RAM, I/O, process/FD use while A stays responsive. Temporary
files consume the same RAM budget. Retain the previous usable release locally;
cold offsite objects cannot substitute for an already-online rollback generation.
Share only verified immutable objects with reference tracking/read-only enforcement;
never create writable hard-link aliases to active state or build outputs. Reject
insufficient capacity while A remains usable. Never promote a stale candidate
snapshot over writes made during staging. See [UPDATE_WALKTHROUGH.md](UPDATE_WALKTHROUGH.md).

## Evidence and next action

The full private metadata inventory is preserved in the workspace under
`/workspace/recovery-backups/20260930T165936Z/`:

- `p18u-offsite-capacity-proposal-20261005-01/MANIFEST.json`:
  `13cd69e9f4b33fd1edec0c8dd180d767eae63202362f183156a9d30d8a8b4bad`.
- `p18u-google-drive-capacity-supplement-20261005-01/MANIFEST.json`:
  `bb78ca66673afd8d5f3b285ef70da6c445d9e022d52e86fc89ef91db3ae0f5d4`.

These seals identify audit reports, not fresh archive payload integrity. They are
workspace-local checkpoints, not demonstrated external backups. Published GitHub
source protects only included source. Current next action is the necessary operator access decision or a real capacity
change, followed by bounded handoff/helper recovery testing and renewed full native
admission. The installed-overlay gate and input/duplicate/rlib/registry repairs are
complete; preserve their evidence and do not replay them. PC and home Proxmox are
potential private destinations; no supported bulk cloud transfer is proven. No
public management endpoint or unapproved upload. Prepare the concrete route/access,
integrity/restore and retention decision before any protected archive retirement.
