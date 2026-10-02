# Native auth reload ownership — adopted work in progress

Five source paths now retain the committed authentication source identity alongside the cache and policy revisions. A delayed result from a replaced provider is rejected even when the credentials are equal or the original provider is later reinstalled. Pending installation alone does not revoke the currently committed source. No credential lock crosses the provider await.

The change was applied only after exact preimage checks, with originals preserved. `just fmt` passed. Five causal test functions were added, but **compilation and tests have not run**. This is lifecycle support inside the native authentication service, not an independently extracted authentication component. The separate permanent-refresh-failure proposal remains unadopted.

The previously tested production executable was fully restored from its archive; its real version command returned zero. It belongs to the earlier source7a and does not validate this change. Its two output paths and sixteen existing aliases were restored without overwriting a live installation. Before future compilation can reuse its path, the compiled-only CLI library was separately archived with47 provenance members, fully restored and hash checked. Only that original cached executable and the new verification restore were retired; the archive and older recovery material remain. These archives are VM-local.

`EVIDENCE.json` records the actual source identity and allowlisted preservation receipts. The upstream source-to-symbol mapping is `upstream/p03-auth-reload-source-lineage.json`. Scoped tests, lint, rebuilt-host/plugin/UI verification and the remaining ownership boundaries stay open. The TUI OOM failure remains a separate failed run, not a test pass.
