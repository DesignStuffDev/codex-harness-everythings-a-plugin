# Development capacity and cold-artifact preservation

Read-only assessment, 2026-10-05. **Native admission remains closed. No upload,
archive deletion, provider purchase, environment change or replacement is approved
by this assessment.** Continue bounded maintenance-plugin work on the unchanged
host. Preserve both source worktrees, accepted binaries, original index and WIP.

## Latest observed capacity

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

Small Python contract, planner, inspector and installed-package checks can fit with
fresh admission: provisionally no more than 16 MiB total new durable writes,
256 MiB scratch and 128 MiB extra process memory, preserving the 256 MiB disk
floor. Count concurrent writes, failed-run evidence and package copies. This is not
blanket admission for every test or a new GUI run.

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

**Certified safe retirement: zero bytes.** The one process snapshot saw no open
archive descriptors or symlink backing, but had three access errors and cannot
certify inactivity. Refresh owner/reference/alias checks immediately before any
eventual removal. Current CLI, storage worker aliases, Rust toolchains, extracted
Cargo sources, recovery metadata, both worktrees and viewer setup remain protected.

The narrow 700 MB shortlist alone would remain below the historical native output
comparator. Individually clearing enough of the broader 8.17 GB pool could materially
help; no whole-pool retirement or guaranteed freed-space figure is established.

## Why the native sequence is still blocked

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
source protects only included source. Next: continue admitted small plugin work;
independently resolve exact native dependency closure and a supported private
transfer/restore route before requesting a concrete retention action.
