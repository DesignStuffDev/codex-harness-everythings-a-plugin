# Rebuilt HTTP Stage B host: five passing runtime gates

This is a read-only public-safe projection of five completed runtime reports, their unchanged-source wrappers, strict subprocess receipts, and the production build binder. It adds no runtime execution or new compilation. Original failed reports remain intact.

Exact tested source: **8,947 files**, map `7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e`, candidate manifest `f9f2bf498167a8ab7fb68360af7e22f5aada7ea11304b51223661479a1c86092`. All five before/after maps equal that candidate. Build evidence records publication `d229296b29947ef1328cbe931c810276f220a23c`; this assembly did not query GitHub.

Exact executables, obtained from root's stable output binder and runtime reports without rereading any ELF:

- Codex CLI: 635,994,944 bytes, SHA256 `78d9194cd4329e9b83b75fe07389d524d8b1f425353d71df9a68607be2945e83`.
- Component manager: 7,274,296 bytes, SHA256 `f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954`.

| Gate | Actual passing scope |
| --- | --- |
| Storage | 13 successful runtime commands; unchanged historical independently built native package installed/selected against the rebuilt host; real CLI turns, tool execution, streaming, cold resume; model and storage reset restore built-in behavior with retained history; three observed storage children terminated. |
| Migration-normal | 10 successful commands; native/selected dry-run parity and unchanged storage bytes, selected apply and idempotent reapply, cold CLI/App Server recovery, post-migration cancellation and observed process absence. |
| GUI-normal | Two real Playwright Chromium cycles using the separately installed GUI and selected native storage. Cycle 1 covers streaming, approval/Allow once, a native command, UI Stop, reload and continuation. Cycle 2 covers cold recovery and continued streaming. Both cover built-in native file search and component-manager Launch shutdown on first SIGINT, exit 0, tracked process absence and subreaper drains. Manager-exit waits were 0.26519s and 0.26459s; these are not total cleanup durations. |
| Attachment | Five case groups, 17 successful nested commands. Built-in baseline, selected native inline upload, counted upload/cold resume (1→1), typed-error inline fallback, unselected-native removal, reset and built-in restoration. Host/package bytes unchanged. Native upload path is supported by the preserved trace inspector; no raw traces copied. |
| Migration-search | A second fresh migration fixture passes the same 10-command migration scope. This does not establish the subsequent selected-search GUI gate. |

Every outer strict receipt has command/exit status 0 and no runner error. Storage and migration receipts include adopted descendant statuses -9; attachment's nested receipts also retain such statuses. These are recorded without causal attribution and must not be presented as every child exiting gracefully. The GUI gate's own observed process/drain checks pass. None establishes universal whole-host cleanliness.

Package reuse proves compatibility with this rebuilt host **without rebuilding the installed native package or changing the host during installation/replacement**. It is not a new independent Rust package build. Python fixture plugins were built during acceptance. Storage reset is covered; native attachment removal is covered; a storage uninstall is not claimed.

Limits remain explicit: deterministic model fixtures, no live provider; automated Chromium fallback, no manual in-app Browser; screenshots were not reviewed by this evidence assembly. GUI-normal uses built-in fuzzyFileSearch, not the selected external search provider. Attachment does not exercise a production resolve caller or inspect the native response envelope. Its retained native state directory had zero files, so this is not a nonempty-state retention proof.

**Selected-search GUI attempt01 failed** (wrapper/strict exit 1, runner error null, source unchanged) and is excluded from the passing subset. Its partial observations are not counted as success and diagnosis belongs to the separate runtime review. Slow normal/forced shutdown, the held Git/HTTP matrix and featured-owner gates require their separate current-source receipts. This report does not close P03, HTTP transport teardown, all affected consumer tests, or whole-platform/updater acceptance, and adds no extracted component family. Unadopted auth proposals are not part of this host.

`REPORT.json` contains exact counts/status histograms and bounded limitations. `LOCAL_REFERENCES.json` pins private input reports by path/hash without copying their contents. `GUI_REVIEW.json` is the independent safe GUI review. `assemble.py` reproduces the safe projection; it does not run tests, inspect credentials, change source, or open ELF/raw-trace files. Original private reports remain on the VM. This directory is an in-VM checkpoint until root publishes and verifies a GitHub copy.
