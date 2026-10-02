# Preserved, unadopted HTTP constructor proposal 01

This source archive preserves the original native HTTP ownership proposal for review and recovery. **It is not activated or verified implementation.** Review found that cleanup tied permanently to an old Tokio runtime could retain abandoned results and eventually exhaust the bounded registry. A corrected cross-runtime ownership proposal is being prepared separately.

The archive contains source, exact preimages, patches, contract and provenance notes; no build outputs, runtime state or credentials. All 55 archived file payloads were checked against the backup receipt. The original files and VM-local archive are also retained. Root Apache LICENSE/NOTICE and the recorded OpenAI Codex upstream provenance continue to apply.

The isolated watchdog test was adopted and tested separately; its [evidence](../../p03-runtime-drop-watchdog/README.md) does not validate the HTTP proposal. Never apply the archived native patches as accepted code. The archived receipt records that the backup was VM-local at creation; publishing these archive bytes on the WIP branch provides external preservation for this proposal only, not every current VM file or later proposal.
