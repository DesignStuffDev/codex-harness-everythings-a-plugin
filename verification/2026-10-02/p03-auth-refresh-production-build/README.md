# Source42 production build

The canonical CLI and component-manager production build completed with exit 0. Its before/after source fingerprints match all 8,949 files in source42. The locked, offline command selected only `codex-cli --bin codex` and `codex-component-host --bin codex-component`; it was not a workspace test suite. Wrapper timestamps span 263 seconds.

The new 635,999,960-byte CLI is bound to SHA256 `8e8a5dac859825a910403fc85dc7aed4a4ddaf3cc1c7d585f3a0fb27a7d64dc0`. The 7,274,296-byte component manager retains SHA256 `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`: it is the previously preserved unchanged artifact, not evidence of new manager compilation. Root's completed binder double-hashed both actual canonical outputs. This report reads that binding and compares metadata; it does not rehash the binaries.

The terminal receipt originally marked binary binding pending. The later completed postbuild binding resolves that stage explicitly. Runtime gates remain pending; successful compilation/binding is not storage, migration, shutdown, replacement or GUI acceptance.

OOM counters stayed 10/5 before and after the build. The later metadata-only snapshot records 511,156,224 overlay bytes, 1,140,502,528 `/tmp` bytes and 1,417,003,008 hard-unused RAM bytes. `/tmp` and `/dev/shm` remain RAM-accounted. Roughly 0.5 GB overlay remains: run the next bounded runtime gates sequentially with their own fresh resource guards. This is not admission for another substantial native build. No process was signalled and no cache or runtime directory was changed.

The earlier resource evidence remains frozen and separate. Existing preservation archives remain VM-local. `BUILD_CHECKPOINT.json` includes the generation-checked process metadata sample and its visibility limitations. `RECEIPT_INDEX.json` provides exact completed receipt hashes without copying the full source maps or log contents.
