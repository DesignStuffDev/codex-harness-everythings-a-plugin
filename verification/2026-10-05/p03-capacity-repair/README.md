# Same-environment capacity repair, 2026-10-05

Actual repair completed; full native acceptance remains open.

- Restored895 standalone metadata files:694,368,532B (696,229,888 allocated),
  freshly parsed from the paired ar/ELF `.rmeta` sections. All895 original alias
  readbacks matched. Preserved source preimages, index and16,906 fingerprint inputs.
- Restored902 genuine registry archives:133,287,919B (135,127,040 allocated plus
  one4,096B marker), checked against Cargo.lock and historical hashes. All902 exact
  final readbacks and original aliases passed. All8,977 bound source paths unchanged.
  One wrong URL caused by prerelease-name parsing failed; its result/helper remain.
  Reviewed correction uses exact lock name/version and completed the repair.
- Retired55 duplicate paths:24 cold storage-plugin copies and31 duplicated raw
  source receipts, totaling1,786,339,328 allocated bytes. Measured net free-space gain
  was1,786,236,928B. Before retirement, all32 distinct payloads were actually restored
  and hash/mode verified. Afterward all33 retained recovery paths reverified, including
  two independent plugin inodes. No archives, unique evidence, state or source removed.
- Process visibility for three system/old processes was incomplete. A demonstrated
  Linux exclusive write-lease guard rejected existing readers/mmaps and detected
  new opens; every candidate lease was held through exact checks and unlink.
  Full alias scan found no extra links/references. No claim of complete `/proc` access.
  All110 retirement-journal events and hash chain verified. Restoration remains
  possible from retained exact bytes; restored inode/ctime identity necessarily changes.

Both tmpfs mounts share16GiB RAM. Recovery is useful but is neither Cargo freshness
proof nor a source-bound native test pass. New metadata timestamps were not forged.
Historical production's near16GiB sample included10.35GB constant shared memory;
compiler RSS alone did not require16GiB. A full phased disk/RAM/validation plan is
still being reviewed. No Rust build was dispatched and no feature work resumed.

The private home-storage endpoint failed this VM's single check: DNS no address,
HTTPS proxy CONNECT403. No routes, tunnels, service configuration or uploads changed.
No supported same-instance resize or bulk binary export tool is exposed.

All recovery manifests/helpers/results remain under
`/workspace/recovery-backups/20260930T165936Z/`: `p03-metadata-restore-20261005-02`,
`p03-registry-restore-20261005-02`, `p03-capacity-reclaim-20261005-02`, and
`p03-capacity-repair-checkpoint-20261005-02`. Published evidence contains concise
measurements and private-receipt digests only. This does not publish private archives.

After capacity restoration, finish only the paused installed-overlay cancellation
acceptance, then return to native P03. The collector stays preserved and paused.
