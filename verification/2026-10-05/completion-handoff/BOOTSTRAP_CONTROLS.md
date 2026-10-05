# Worker bootstrap controls: reviewed requirements, not a runnable bundle

This infrastructure-only continuation reviews the selected private source/control
packet after the interrupted planning turn. It does not change product source,
execute helpers, provision a worker, export private bytes or admit a native build.
The [readiness manifest](MANIFEST.json) binds exact source and private review records.
The [completion handoff](../../../COMPLETION_HANDOFF.md) remains the ordered plan.

The packet contains 32 selected payloads / 1,550,203 bytes. Its seal and hashes
survived the pause. The original index, eight auth candidate paths and nine
separate variants are unchanged. This is VM-local custody, not external backup.
Content review and byte identity do not establish runtime portability or authorize
publication of the private packet. Keep its originals and all correction history.

## Required worker derivations

| Input / boundary | Concrete issue found by static review | Required worker preparation |
|---|---|---|
| Source identity | The portable `SOURCE_MAP.json` is the canonical path/hash map. Some wrapper/auth consumers expect an envelope with `source_map`, not the bare map. | Create and seal a new worker candidate envelope referencing the actual pinned checkout/overlay and this map. Capture the worker's own source/index identities; never require or forge the original VM's index hash on a new checkout. |
| Environment order | The preserved base environment script sets `CARGO_BUILD_JOBS=2`; selected verification settings require an explicit later override to `1`. | Preserve source-script order and apply the final one-job setting before dispatch. Record effective non-secret toolchain/target/profile/flags. Serial nextest remains separate from compiler/linker memory admission. |
| Resource observer | The old observer hardcodes the original recovery root and expects `FIXED_PLAN.json`, per-command admission and the original index identity. | Use a separately reviewed worker derivative with new paths, plan/source identities, per-filesystem/RAM/inode admission and receipt destinations. Original controls remain unchanged. Attach monitoring to the actual strict/source-wrapped command, not a detached sample. |
| Observer outcome | A successful child exit can survive observer guard/finalization failure in the returned process code. | Require the observer's valid, matching `complete_success` receipt as well as child/strict/source results. Missing, stale, failed or incomplete resource evidence blocks acceptance; exit zero alone is insufficient. |
| Attachment fixture | `attachment_production_acceptance.py` reads an adjacent `INPUTS.json`. | Review and bind the required companion metadata, derive current worker producer/package proof links and stage it explicitly. The selected Python source alone is not its input closure. |
| GUI observation | `gui_browser_driver.cjs` requires adjacent `gui_search_observer.cjs`. | Stage the exact published companion helper with its hash and expected relative layout; also bind the chosen browser/runtime and current GUI/package sources. Do not silently substitute one of the nine separate variants. |
| Held-host fixtures | Receipt schemas, current source-binding controls and package/producer proofs are separate inputs. | Inventory and deliver the selected exact closure, with original-to-derived provenance and unchanged ownership, deadlines and cleanup assertions. No old proof may be edited into new acceptance. |
| Independently built components | Old commands reference historical source exports, resolved locks, package hashes, timestamps and source-absence assumptions. | Build fresh thread/search/attachment/desktop packages separately; preserve original/resolved locks and source before any helper removes its own export, then generate new proofs against the frozen new host. |

All references above concern source reviewed in the sealed packet, not evidence
that these commands ran on the proposed worker. Do not alter expected outcomes or
weaken Linux process/subreaper/tracing, approval or durability assertions to make
porting easy. An unavailable capability is a missing gate.

## Content, licensing and private evidence

Review source/control code separately from the raw files it will produce. GUI
ready/driver/error reports can contain the per-run bearer URL; keep them private
alongside runtime configuration, process traces, conversations and state. Public
evidence must be a reviewed projection with fresh source/package/test identities,
not a raw-report upload. Fixture literals must stay distinguishable from actual
credentials; a static inspection is not proof that every future runtime log is safe.

Preserve upstream Apache LICENSE/NOTICE, prominent modification notices and source provenance, SDK notices and all
applicable third-party notices in any later source handoff. Selected helper copying
does not constitute a complete dependency/license audit or a release SBOM. Any
source eligible for future delivery still needs the exact approved channel and
selection; no private packet, archive or raw evidence export is performed here.

## Remaining external and execution prerequisites

1. Obtain the identified worker's authenticated command/file access and retained
   storage, with independently admitted disk/RAM/process resources. Cloud remains
   primary; worker builds and recovery are explicit offloads.
2. Pin a complete immutable OS/native ABI/tool/browser acquisition recipe. Existing
   Rust/Cargo/Git/V8 pins and nine GStreamer/ORC package identities are useful inputs,
   not a complete image or recursive native-library closure. Verify acquisition and
   executable identities on the worker rather than reuse the damaged target cache.
3. Finish the derived controls and companion-input inventory above. Bind exact
   original and derived hashes, effective configuration and per-command acceptance
   receipts; establish fresh deterministic fixtures without personal credentials.
4. Deliver/read back the pinned source plus explicitly unverified eight-file
   candidate and eligible controls through the approved path. Preserve the nine
   other variants as separate custody. Neither hashing nor source publication is
   native acceptance.
5. Execute the admitted scoped build/runtime/GUI/recovery sequence and close the
   completion handoff's I1–I5 gate. Product phases resume automatically afterward.
   Full workspace testing retains its separate approval requirement.

The source/control review closes a preparation task. Portable controls, native
dependency closure, monitoring integration, fresh package proofs, worker access,
delivery, native acceptance and verified durable recovery remain open.
