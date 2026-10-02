# Installed upstream impact review checkpoint

The new `codex.maintenance.upstream-review` package passed separate-build and
actual component-manager installation, invocation, explicit replacement and removal
without rebuilding the manager. This adds a read-only tool-v1 maintenance capability;
it does not add an extracted native family or complete P18U.

Three scoped Python suites passed: 17 planner, 22 lineage and 11 adapter cases
(50 total). All four source receipts, including installed runtime acceptance, bind
8,962 source files/map `930af666e1add9736265a8ca7631f7851657731e9f34e5efead6abfc836667b5`
with equal before/after maps. The original 8,949 files, including Rust, remain unchanged.
Root required formatting passed before the tests.

The real acceptance executed 23 commands through the unchanged strict subreaper:
exit 0, no runner error, sole reaped command exit 0. Two packages were built from
independent exported projects using the installed SDK; those disposable source copies
were removed before installation. The test rejects a distinct-name API2 package,
ambiguous provider installation, invalid request version/digest and removed-provider
invocation. Explicit selection executes the replacement's marker. Packaged bootstrap
returns the identical report with the host absent from PATH. Both packages are removed.
The manager remains SHA `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.

`EVIDENCE.json` preserves exact source/input/package and zip member hashes, full short
test logs, command results, strict receipt and blocking report. `INPUTS.json` records
raw receipt references. Original large maps and packages remain in the VM; references
alone do not export or externally preserve their content.

The report intentionally withholds approval. Candidate revision is the existing
upstream pin; published composition `ad2fc047` is unavailable in the local object
repository, and the index is historical. The report retains those blockers and 1,167
unresolved lineage findings. Success here means installed inspection works, not that
any revision is safe to integrate.

Review corrected two issues before tests: the API2 rejection case now uses a distinct
component name; plan identity now binds both packaged planner digests. Original
adoption, reseal, correction and formatting identities remain separately recorded.
No copied planner/validator implementation changed.

Cancellation admission cases are mocked. No actual active-Git cancellation, network
negative proof, complete repository nonmutation snapshot, engine-session or new GUI
cycle is claimed by this checkpoint. Git output bounds remain post-capture. A later
GUI supplement must name its own tested source and real browser route. P18U still
requires real later-upstream integration, custom-plugin/UI preservation, coordinated
versions/migrations, incompatible-update refusal and host-independent failed-update
recovery. No polling or live update is enabled.
