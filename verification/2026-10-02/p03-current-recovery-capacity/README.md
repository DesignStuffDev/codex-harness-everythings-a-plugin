# Capacity recovered for the next native verification gate

Root completed three independently reviewed, bounded resource actions without changing the 8,947-file source cohort, original Git index or protected runtime-selector metadata. These are resource receipts, **not new build, test, runtime, extraction or whole-platform acceptance results**.

| Completed action | Exact scope | Observed result |
| --- | --- | --- |
| First ordinary-cache retirement | 63 files; 528,506,880 allocated bytes | Receipt reports 786,001,920 bytes free on overlay afterward. |
| Clean-page advice | 31 exact ordinary files; 2,057,631,673 logical bytes below 2 GiB | Bounded fsync/DONTNEED, no candidate content read/hash, deletion or rewrite. Hard unused cgroup memory increased from 435,855,360 to 1,267,326,976 bytes between the recorded snapshots; OOM counters remained 9/4. This is an observation, not guaranteed reclaimed memory or a future peak budget. |
| Further exact-context retirement | 47 files; 1,084,129,280 allocated bytes | Receipt reports 1,867,800,576 bytes free on overlay afterward. |

The two retirements removed 110 ordinary cache files with 1,612,636,160 allocated bytes in their metadata. Actual available-space snapshots differ from simply adding file sizes because evidence and other ordinary activity also consume space. No action used a global cache drop or replayed an earlier retirement.

The frozen source map is `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`; original index is `0dc35ffe37ae1f620b6d14d4db0a6f6056a03ac6a20cf3af8925d5d2f4223d59`. The existing CLI hash is `78d9194cd4329e9b83b75fe07389d524d8b1f425353d71df9a68607be2945e83`; manager is `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`. These hashes come from sealed build/runtime evidence. Resource actions checked their metadata and excluded the executables; this evidence assembly did not rehash any ELF.

The second retirement changed the protection policy from **138 recorded family/current roots to 21 explicitly reviewed current/prospective roots**. Ten primary roots bind actual production CLI/manager, CP B and HTTP A proof targets, and future CLI65eee/login-dcb test contexts. Eleven related ordinary/lint roots remain conservatively protected. The chosen union contains 2,769 records. Historical proof ELFs, archives/restores, source, symlinks/backings, compiler metadata and runtime homes remained outside the deletion scope; excluded archives were not globally rehashed as unchanged.

Two stale CLI edges remain explicitly qualified. CLI65eee test-bin and CLI-bfdf library reference an absent old run-build digest. The new graph retains all four observed same-package build/run contexts, verifies build.rs/no package or target build-dependencies, and verifies empty dependency lists on both compile-script records. This is conservative preservation, **not exact resolution of the missing digest**. The earlier budget JSON retained those edges, but its prose incorrectly described zero unresolved edges. Original artifacts remain preserved and a correction is linked by hash.

Only ordinary variants outside the reviewed current/prospective contexts were retired. Explicit future Cargo feature/profile choices may regenerate them; that is recorded rebuild debt. Current CLI tests still need source-bound compilation and execution. The ledger's cold dependency estimates, temporary-disk limit, compiler memory and link scratch must be reviewed in a fresh dispatch preflight. Recovered capacity alone admits no build and proves no runtime behavior.

`EVIDENCE.json` contains minimal typed assertions, numeric snapshots and exact plan/audit/result hashes. `LOCAL_REFERENCES.json` pins the larger local control/provenance reports without copying raw process scans, arguments, environment/config values, auth, private fixtures or payloads. Original immutable plans, fsynced journals and terminal receipts remain under the referenced recovery directories. This package is VM-local until root publishes and verifies the selected source/evidence files; it does not create a new external backup of the full VM.

Terminal receipt hashes:

- 63-file retirement: `62fe7f98b882612ec72f0f75ba6c4b0e992d0dc532e4cc53bd27509b4e94e023`.
- 31-file advice: `dcf6af5df49e454369ae679fbeea17aaf31a681a536d149065cb91e572c581ac`.
- 47-file retirement: `074af2e0fbffa0503d4795fe588c5b48639d949ea06f54b096b4176b445c9271`.
