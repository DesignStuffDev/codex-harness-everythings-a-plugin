# Current CLI proof preservation and capacity supplement

The root's completed preservation workflow retained a VM-local archive of the existing CLI test proof and 39 supporting members. All 40 members passed a full restoration and hash comparison before the original test executable was retired. The subsequent release removed only the newly created verification restore: 40 files and 11 directories. Older restorations and archives were outside the action.

The retained archive is 68,283,641 bytes, with recorded SHA256 `f5197d265f3742c497d44eff0252936438b5cd88fb9e406ae41d3da21ca7101f`. Its original test executable was SHA256 `2c6a89e4c5386bdc2300208b4648be7bec8d6368d530d4c953180647ee9e9c8d`. These are root receipt references; this assembly did not read or hash archive or executable contents. External backup has not been established.

A preceding bounded advice operation used `fsync` and `POSIX_FADV_DONTNEED` on one exact inactive ordinary library, over 178,696,526 logical bytes. Its receipt reports no candidate content read, hash, deletion, or rewrite. Cgroup usage fell by an observed 174,096,384 bytes between the recorded snapshots. This difference is not a causal attribution or a guarantee of reclaimed memory or future build capacity.

The proof-preservation workflow checked the scoped 8,947-file source map `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e` and original Git index `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`. Its full preservation receipt records matching source guards before and after preservation. Retirement and release invoke the same guards, but their compact terminal receipts do not embed a new source map. The one-library advice operation does not independently check source or index. Runtime output selectors receive metadata protection; this supplement makes no new runtime ELF hash claim.

After release, the receipt records 856,887,296 bytes of overlay space and 1,139,408,896 bytes between the cgroup limit and current usage. Conditional clean-file credit is explicitly not guaranteed RAM. These are historical snapshots, not current measurements or build admission.

`EVIDENCE.json` contains only selected status, count, hash, and resource fields. `LOCAL_REFERENCES.json` binds small VM-local receipts and action source by hashes computed during this assembly; it does not copy private process records or archive members. `MANIFEST.json` binds the three public supplement files. The source map, artifacts, archives, process scans, and cache actions were not rerun by this assembly.

This supplement records resource actions and retention of an existing proof. It does not assert a new test pass, including the separately reported grouped CLI/Exec gate, or new runtime, extraction, deployment, or P03 completion. Publication of this supplement would not publish or externally preserve the private archive. Completed actions must not be replayed after path reuse.
