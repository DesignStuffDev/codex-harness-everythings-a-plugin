# Upper custody outcome provenance

This immutable addendum qualifies COMBINED-MANIFEST-02.json (d99b4b1ce641d51952eecb0adf9ba761777c1f5bea45ee2c9d7e5d961714dcef). It changes no source.

RetainedTask retains the original Tokio JoinError or spawn-panic payload and the exact task output. For Session that output is already a bounded SessionShutdownFailure category. Existing persistence I/O errors are logged and mapped to Persistence; their original error object is not retained by this patch. Thus PROPAGATION-MANIFEST-01's broad sentence about original Session/MCP errors remaining in task owners must not be read as retaining every underlying I/O error object.

The returned shutdown result distinguishes task failure from an observer timeout. A timeout alone is pending and does not poison the immutable failure latch. A successful session-loop observation certifies only the declared upper task result. It is not a certificate of lower MCP service, local child, descendants, HTTP worker, or whole-host completion.

No Rust, formatter, test, runtime acceptance or primary checkout modification was performed by this author.
