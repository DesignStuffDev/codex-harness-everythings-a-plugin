# Native compiler-output handoff and recovery contract

**Reviewed design only. No helper, cache handoff, recovery drill or native admission
has executed.** Read EXECUTION_STATE.md and CAPACITY_PLAN.md before resuming.
Keep original checkout/index, the 9535903 published checkpoint and the adopted,
unaccepted 8,980-path b75efc97 candidate. Installed-overlay acceptance is complete;
collector and broader updater work remain paused.

## Observed problem and ownership choice

Focused retry03 reached only proc-macro2 compilation, then exited101 because its
rmeta resolved to mode0444 recovery backing; zero tests ran. Its canonical rlib
alias became absent in the failed interval; the unlink actor is unproven. Preserve
that absence and the zeroed fingerprint. The larger graph is conditional propagation,
not an observed Cargo schedule. Do not reset fingerprints or invent old mtimes.

Give admitted compiler units independently writable outputs while retaining exact
old bytes under separate recovery ownership. Existing tmp rlibs are mode0600 but
remain protected inputs by policy. No chmod of recovery backing, writable hardlinks,
global alias removal or eager handoff of every relocated unit. CoW returned ENOTSUP
within workspace/tmpfs and EXDEV across them; every independent copy costs space.
A symlink alone does not prove where a future output writer will allocate bytes.

## Exact recovery coverage and remaining drill

- The 565,190,455-byte selected-rlibs archive, SHA256
  `0691ef18f30d705eacb611a0637e59dcecb241b04488952767cd72323254c353`,
  contains 1,047 exact libraries. Full restoration/hash/portable-metadata readback
  allocated 2,634,592,256 bytes. It does not directly contain standalone rmeta files.
- All895 restored metadata payloads map by exact original identity and section
  hash to paired libraries in that archive. Their executed restoration/readback
  covered 694,368,532 logical /696,229,888 allocated bytes. Original standalone
  mode/mtime is not established; current protected targets have new mtimes.
- Library archive restoration and library ELF-section-to-metadata derivation each
  executed separately. **Combined archive-to-all895-metadata recovery has not run.**
  A bounded drill must verify each full member hash and section length/hash using
  at most one member-sized staging file (largest89,776,342 bytes), plus admitted
  process/output room. Do not retire a sole metadata backing before this proof.
- The 140,182,904-byte accepted CLI/manager archive, SHA256
  `5c5a49bfd0bbe4f9774d9a008ade1e0e96e9fd39c0815d7a7b8f40970afcedfb`,
  has full two-file restore/hash proof. Restored allocation is643,276,800 bytes;
  original historical inode allocation is634,273,792. Only duplicate probe files
  were released. Canonical/deps hardlink pairs require separate executable ownership.

These are protected VM-local checkpoints, not externally durable backups. A future
accepted-runtime recovery extractor must work without starting the candidate host,
verify old executables in fresh private paths, and select them without overwriting
new binaries or changing conversations/configuration. That launcher is not implemented.

## Bounded cohort transaction

1. Review the exact admitted units and all mutable outputs/fingerprint/depinfo/
   build-script members, source/toolchain/target/flags, capacity and recovery map.
   Prove no writer, mapping or live consumer; names alone are insufficient. Reject
   ambiguous ownership, hardlinks or symlinked parents. Guard directory descriptors
   and leases where supported; a lease is not complete ownership proof.
2. Bind the actual starting epoch, including failed/absent paths, canonical alias
   inode/text, backing identity/hash, archive member and small fingerprint preimages.
   This evidence is not a recipe for restoring old freshness under new outputs.
3. Stream-copy only admitted members to exclusive independent same-filesystem
   staging files; verify exact hash, set owner-writable mode on the NEW inode, fsync.
   Preserve observed new mtimes. Record absent paths as absent. Charge copying and
   any additional dependency invalidation before proceeding.
4. Append/fsync PREPARED intent, recheck identities and exchange the exact alias
   with the verified regular file using guarded RENAME_EXCHANGE; fsync parents and
   append/fsync HANDED_OFF. Hold the old alias; retain backing. Fail closed on any
   mismatch. No unconditional overwrite/unlink.
5. Record COMMAND_START before the exact admitted repository entrypoint. Track
   descendants/resources; after every owned descendant exits, bind all new/partial/
   absent output and fingerprint state in COMMAND_END. A single rustc exit does
   not authorize releasing the command's recovery inputs.

The journal is bounded, append-only and chained. Preserve torn tails and reconcile
actual identities; never replay rename actions from a textual status alone. An
absent-output/no-precopy variant requires a proved unit-start interception boundary;
none exists now. It is deferred, and cannot be applied to the graph eagerly.

## Failure and rollback epochs

| State | Required recovery |
|---|---|
| PREPARED; original canonical and staged copy exact | Preserve canonical; release only a separately verified receipt-owned duplicate if permitted. |
| Exchange complete, command never started, entire epoch unchanged | Guarded reverse exchange may restore exact ownership; fsync and journal. |
| COMMAND_START exists or startup is ambiguous | Preserve forward/new/failed state and old backing. Never restore old output beneath newer fingerprints. |
| New regular/partial output, unexpected alias/owner or torn journal | Preserve everything, fail closed and review exact mismatch. |
| Old tmp backing lost | Restore only missing owned backing through a reviewed archive recovery epoch; never replace newer canonical output. |

After possible Cargo execution, default to forward preservation. A later forced
rebuild must quarantine the complete exact mutable output/fingerprint cohort after
no-writer checks; this consumes retained blocks and honestly forces rebuild. It is
not implemented, and its expanded closure must be admitted first.

After a complete successful command, optional rebalancing could archive exact newly
produced rlibs, fully restore/read back to fresh tmp backing, then journal canonical
exchange. Preserve actual emitted metadata and a new immutable archive; prospective
compression, byte identity and later reuse earn no capacity credit. Old tmp backing
can be released only after full recovery proof, all aliases/references checked and
an explicit retention decision. That frees RAM, not persistent disk. New mutable
backing may later change or be unlinked; budget reallocation and renewed preservation.

## Required gates before using the design

First obtain the necessary operator access/capacity change described in the private
sealed card. Then review a bounded helper and disposable tests covering every journal
state, compiler-style unlink, zero/partial/new output, mismatched aliases, torn tails,
writer overlap/failed leases and post-start fingerprint changes. Complete the combined
archive-to-metadata drill and independent accepted-runtime recovery path. No stale
cache restoration is allowed. Re-admit the whole focused/full-login/provider/lint/
production/runtime/UI sequence across both filesystems and shared RAM before Rust.
Full-workspace testing keeps its separate approval boundary. This document grants
no transfer, archive retirement, network mutation or build admission.
