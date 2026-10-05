# Selected build backing and headroom repaired; native verification pending

Root completed the second reversible capacity repair on2026-10-05 in the original
cloud checkout. **No native build/test or new GUI run occurred in this repair.**
The first repair, publishedfd28a6c, restored895 metadata files and902 genuine
registry archives, then reclaimed1,786,339,328 bytes from55 verified duplicate paths.

The second repair preserved1,047 selected unchanged Rust libraries in a
565,190,455-byte exact-byte archive, fully restored their payloads and portable
metadata, then journaled replacement of their original cache names with verified
temporary-backing aliases. The archive remains on the VM's persistent filesystem;
it is not an offsite backup. No source, accepted executable, existing recovery
archive or unique evidence was retired.

| Verified result | Value |
|---|---:|
| Restored library files / verified canonical aliases |1,047 /1,047|
| Restored backing allocation |2,634,592,256 bytes|
| Journal events |3,141:1,047 intent/exchanged/relocated triplets|
| Postcheck persistent free |4,126,289,920 bytes|
| Postcheck hard-unused memory |8,842,485,760 bytes|
| Reviewed starting guards |3,850,000,000 persistent bytes /8GiB hard-unused RAM|

Root verified8,977 source files (map`f27ab9e949b997ff910baf0c891465e3847879a94b0b29b498b04c01246ed4b2`),
16,906 fingerprint/depinfo inputs, original index and accepted CLI/manager identities.
Archive SHA256:`0691ef18f30d705eacb611a0637e59dcecb241b04488952767cd72323254c353`.
Payload proof belongs to the executed preserve/restore/final-alias readback. The
independent postcheck checked identities, metadata, ownership and journal consistency;
it deliberately read zero payload bytes. Existing unreadable process entries were
handled by the required exclusive kernel write-lease guard, not claimed globally absent.
Root's bounded clean-page advice targeted only the exact fsynced verified archive;
there was no global cache drop. Current-epoch OOM/max/kill counters stayed zero.

The repaired guards conditionally support the [phased native sequence](../../../CAPACITY_PLAN.md).
Its3,797,387,752-byte planning envelope includes new tests/production outputs,
536,870,912 bytes growth contingency, source/evidence, serial proof archives,
ordinary/link scratch and a256MiB recovery floor. Missing ordinary core/TUI pairs
(442,593,280 bytes) are counted once, already inside the earlier focused comparator.
The9-root auth plan excludes the giant TUI/App Server and other unresolved P03 gates.
Historical sizes plus contingency are not measured future compiler bounds; recheck
actual source, owners and disk/tmpfs/RAM before every stage. An8GiB-free disk target
is not mandatory after this verified phased repair.

Keep the archive, ARCHIVE.json, RESTORE.json, ownership receipts and alias journals
in private R/p03-cache-relocation-20261005-02. If backing is absent, the implemented
`relocate.py rehydrate` path checks fresh resources/archive/owned aliases, preserves
current regular compiler outputs and writes a new restore-epoch receipt. Guarded
`recover` returns owned aliases to regular durable files after separately admitting
approximately2.63GB plus reserve. Recovery also passed three disposable synthetic
cases and18 phase calls: normal recovery, absent temporary backing while preserving
a newer regular output, and injected interruptions after actual forward/reverse
exchange syscalls followed by successful recovery. These are fixture tests, not
power-loss or production-disaster rehearsals. Full restoration of the actual
1,047-file archive also executed; the evidence keeps those claims separate.
Do not replay the completed preserve/restore/relocate phases.

Next: finish only the exact paused installed-overlay cancellation acceptance,
including both primary cleanup cases and subsequent successful manager invocations;
then immediately adopt/validate the coherent eight-path native auth-install cohort,
rebuild production and rerun source-bound installed-storage/migration/GUI/shutdown/
recovery gates. The collector and broader updater work stay paused. Python acceptance
cannot satisfy native capacity success. P03 and whole-harness extraction remain incomplete.

EVIDENCE.json contains sanitized outcomes and exact private receipt references.
Publishing these references does not export their private payloads or runtime state.
