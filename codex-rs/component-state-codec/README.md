# Trusted component state codec

This package preserves native in-memory state across an explicitly selected
component process boundary. Storage and context replay share the same codecs;
neither subsystem depends on the other. It has no dependency on core, a replay
algorithm, a context engine, or a process transport.

The public API consists of typed serde adapters: `rollout_item`, `envelopes`,
`response_item`, `guardian_checkpoint`, `turn_context`, `token_usage`,
`permission_profile`, `reasoning_effort`, `thread_source`, `native_path`,
`absolute_path`, `dynamic_tool`, `image_generation_item`, and `model_context`. Each provides borrowed
serialization and owned deserialization wrappers, `serialize`/`deserialize`,
and `option`, `vec`, and `option_vec` adapters. Internal remote DTOs remain private. `optional_json` additionally provides
serialize/deserialize functions that distinguish absent data from captured JSON null.

For example, a storage-owned transport DTO can annotate its native items with
`#[serde(with = "codex_component_state_codec::rollout_item::vec")]`. Its receiver
then passes the restored native items into the existing writer unchanged. This
does not alter the writer's normal persistence encoding or later rollout parsing.

These codecs admit fields that ordinary provider deserialization deliberately
strips, including executed-call evidence and tool-result provenance. They must
never decode provider output, user JSON, or unauthenticated persisted records.
Selection, connection ownership, request identity, lifecycle, authorization and
stale-response checks belong to the calling adapter. Decoding establishes no
authority. Raw state and serde errors must not appear in public diagnostics.

Native paths carry explicit Unix-byte or Windows-wide-unit tags. Cross-platform
tags are rejected. Absolute paths must already be absolute and normalized; no
current directory, home expansion or filesystem lookup occurs. `PathUri` keeps
its native canonical URI encoding. Path-key maps use entry arrays and reject
duplicate decoded keys.

This is a new transport shape, so an existing component contract must change its
version when adopting it. Native/provider/persistence serde remains unchanged.
The protocol package exposes one separately named trusted executed-call snapshot
helper to restore otherwise private invariants; ordinary deserialization still
strips those fields.

The source is extracted from this fork's staged replay codec, based on the pinned
OpenAI Codex native domain types. Repository Apache-2.0 LICENSE and NOTICE apply.
See `src/wire/ACTIVATION.md` for exact activation and unverified coverage.
