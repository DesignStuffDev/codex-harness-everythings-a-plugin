# Attachment storage component

`attachment_store:default` version 1 replaces image upload/storage through the
existing `AttachmentStore` interface. `attachment-store-api` owns the native
trait and data types; `attachment-store-inline` owns the original inline
implementation; `attachment-store` preserves its previous public imports.
This crate contains the process adapter and a separately executable native
inline plugin. None of those crates depends on core orchestration.

## Native plugin built outside the harness source

With Python 3.11+, export the exact native implementation and its small API
dependencies as a standalone Cargo project:

```sh
python3 codex-rs/attachment-store-component/package/build.py export /absolute/external-inline-source
cargo build --manifest-path /absolute/external-inline-source/Cargo.toml --bin codex-attachment-inline-component
python3 codex-rs/attachment-store-component/package/build.py assemble /absolute/external-inline-source/target/debug/codex-attachment-inline-component --output /absolute/inline-package
codex-component --codex-home /absolute/codex-home install /absolute/inline-package
codex-component --codex-home /absolute/codex-home select attachment_store default codex.attachment-inline
```

The standalone project contains no path back to the harness checkout and no
host dependency. Its binary calls the same extracted `InlineAttachmentStore`
implementation. Building/installing this plugin does not rebuild the host.
Source export retains Apache-2.0 LICENSE and NOTICE. Installation is inert until
the replacement is selected; restart the affected harness session to activate.
Reset selection to return to native default behavior. Removing registration does
not delete durable plugin storage or rewrite existing session references.

## Contract and state ownership

The standard process initialize/ready/request/result/shutdown protocol surrounds
these methods. Results use exactly one key: `{"ok":...}` or
`{"error":"not_found"|"invalid_attachment"|"backend"}`. Error messages, raw
bytes and signed URLs are excluded from diagnostic errors. Backend-specific
details must remain in appropriately protected plugin diagnostics.

`upload` input:

```json
{
  "thread_id": "thread-id",
  "file_name": "image.png",
  "blob": {"path": "/private/invocation/input.bin", "size_bytes": 12345}
}
```

The host owns original bytes and private staging. It supplies an absolute,
read-only blob file inside a mode-0700 invocation directory on Unix, outside
the 4-MiB JSON framing limit. The blob limit is 1 GiB, matching upstream's
encoded prompt-image input limit. File names, thread IDs and returned file IDs
are limited to 4096 bytes. The plugin must finish reading before returning.
Staging is removed after completion, error or cancellation. The plugin must not
retain staging paths as durable storage or modify staged bytes.

Successful upload returns `{"ok":{"kind":"inline"}}` or
`{"ok":{"kind":"file","file_id":"provider-file-id"}}`. Inline retains the
original host-owned bytes regardless of staging changes. File references must
already be usable by the selected model provider. Arbitrary local blob IDs do
not become valid model file IDs through this adapter. Persistent custom storage,
metadata/indexes and URL generation belong to the plugin under its supplied
`state_dir` or an explicitly configured backend.

`resolve` input is `{"file_id":"provider-file-id","download_url_ttl_ms":null}`
or a nonnegative integer TTL in milliseconds. Sub-millisecond native durations
round upward. A successful result is:

```json
{
  "ok": {
    "metadata": {
      "file_name": "image.png",
      "size_bytes": 12345,
      "mime_type": "image/png",
      "file_url": "https://storage.example/signed-url"
    },
    "url_lifetime": {"kind":"expires","expires_at_unix_ms":1900000000000}
  }
}
```

Metadata uses the existing serde `AttachmentMetadata` shape, including optional
MD5 `digest` and `format_specific:{kind:"image",width,height}`. Individual
metadata strings are limited to 64 KiB and size metadata to 1 GiB. With no TTL,
both `file_url` and `url_lifetime` must be absent/null. With a TTL, a nonempty URL
and declared lifetime are required. Expiration must be at least the requested
duration after the adapter receives the completed operation, including plugin
shutdown; providers should include transport/shutdown margin. A backend with a
permanent URL may explicitly declare `{"kind":"permanent"}`. The host validates
the declaration and timestamp, not the remote service's actual availability.

Each operation starts a fresh process; process memory is not persistent state.
Dropping the native future cancels that invocation. Process-group cancellation
and private staging cleanup are tested on Linux. Staging observes cancellation
before queued work starts and between 64 KiB writes; an already running filesystem
call finishes before its temporary files can be removed. The component invocation
timeout starts after staging. An interrupted upload may have
committed a backend object before its reply was lost; plugins own idempotency and
garbage collection. The adapter does not automatically retry an uncertain upload.

## Engine integration boundaries

Default component selection must happen with session isolation and injection
provenance intact. Explicitly injected native stores take precedence. Isolated
internal sessions must not inherit a default plugin selected by their parent.
The native replay path intentionally uses inline preparation to avoid uploading
or migrating recorded history. Existing file references remain unchanged.
The core's existing upload-failure fallback preserves inline images.

At the pinned upstream revision, `AttachmentStore::upload` is consumed by image
preparation, while `resolve` has no production caller. Resolve coverage is
adapter-level evidence until a separate model/provider integration needs it.
The app-server `thread/attachment/add|list|remove` endpoints manage structured
metadata through `ThreadStore`; those are a different contract.

Focused checks:

```sh
just test -p codex-attachment-store -p codex-attachment-store-component
```

Tests cover byte preservation beyond the JSON frame size, private staging,
typed/redacted failures, URL TTL enforcement, cancellation cleanup and the native
process package. Foundation adapter tests do not themselves prove that every
engine construction path has been switched to a selected component.
