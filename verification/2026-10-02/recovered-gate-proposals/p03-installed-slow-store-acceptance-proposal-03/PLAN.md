# Slow03: hold an accepted canonical user-history event

Staged only: no tests, native host, installation, browser, rerun or checkout mutation performed. Keep proposal02 and failed `p03-process-final-slowstore-normal-01` unchanged. `FAILED_NORMAL01_DIAGNOSIS.json` contains read-only evidence and safe projections; it does not relabel failure as success.

## Why the original GUI assertion failed

The normal01 manager exited0 after46.4945s; the relay observed native append acknowledgment, native shutdown_complete, native exit0 and both directional task outcomes. The held prompt is present in durable rollout ordinal36 as ResponseItem::Message(role=user). Its interrupted turn has no canonical ItemCompleted(UserMessage). SQLite has that interrupted turn but no projected item/first_user_item_id; projection checkpoint is ordinal40, equal to all40 rollout lines. Therefore neither missing disk write nor merely waiting for a stale projection explains the absent cold UI message.

Core `session/mod.rs` records prepared conversation input before awaiting emit_turn_item_started/completed. Shutdown canceled the later canonical event while the raw response append was held. `app-server-protocol/.../thread_history_projection.rs` projects changed_items only for ItemCompleted and explicitly ignores ResponseItem. AppServer paginated thread/turns/list hydrates those stored items; GUI loadTurns collects the page before synchronous rendering. Extending the DOM wait cannot create the absent canonical item.

This is a real interrupted-input/history-projection limitation, tracked under canonical P07 reconstruction and P16 events/client projection, with P14 turn atomicity. This fixture correction DOES NOT fix that limitation or prove raw response-only input visible in cold GUI. Provider/inference P05 is not its roadmap owner.

## Narrow correction

The relay now gates only thread_store/call, is_control=false, append_items for the expected exact thread ID, containing a RolloutItem tagged event_msg whose payload is item_completed, same thread ID and a nonempty turn ID, with item.type exactly `UserMessage`, nonempty item ID and one text-content element exactly equal to the unique prompt. Wire TurnItem is capitalized `UserMessage`; UI API rendering uses the separate camelCase `userMessage` representation. Both were verified in source and preserved actual rollout shape.

Raw response_item/message input, wrong event/item kind, wrong thread, prompt substrings/trailing characters, arbitrary metadata containing the prompt, and multiparts are not selected. Nonselected complete frames continue unchanged to native storage. The accepted receipt now includes canonical event, turn ID and item ID; the driver requires these before interrupting.

The relay's entire suffix from the95s gate deadline through forwarding/native task settlement/cleanup is byte-identical to02. The driver's entire suffix from signal_time through acknowledged46s hold, normal/forced results, process assertions, cold recovery and final cleanup is byte-identical to02. No waiting/assertion relaxation was introduced. Source fingerprints and exact diff are in CHANGES.json.

Normal still requires accepted canonical append held46s after actual manager acknowledgment, native acknowledgment+cleanup, zero manager/native exit, no tracked descendants including zombies, cold GUI prompt recovery and another completed turn. Forced still requires a second manager interrupt, nonzero/unknown-durability outcome, no forwarded canonical event, no test-forced rescue, and cold GUI absence of that held canonical item. Earlier raw model input may already be persisted in forced mode; its absence is NOT claimed. Durability remains explicitly unknown.

Admission remains the installed acceptance relay before native forwarding, not native StorageService admission. This is a real Launch budget test, not App Server watchdog-origin proof. Plugin, native package, source removal proof, strict5s drain and original failed evidence remain unchanged.

## Validation to run after independent root review

COMMANDS.json contains exact commands and fresh no-clobber paths, bound to current58 and successful build02. It runs staged03 directly; no checkout adoption/native rebuild is needed. Do not use the old run-plan printer unchanged because it pins02.

1. Five authored unit tests verify typed selector/IDs plus raw-response, wrong-event/item, substring/inexact, mismatched thread, missing IDs, control/method/component and multipart/nontext rejection. No result exists yet.
2. Eleven authored synthetic wire cases verify one exact canonical event held until release and forwarded byte-identically; seven nonmatching payloads—including exact raw user input—forwarded byte-identically without gate receipts; clean EOF; late directional parser failure; and unchanged original01 negative control reproducing false success. The four original case roles are preserved; these are Python-peer/0.2s fixture checks only. No result exists yet.
3. On passing fixture checks, create dedicated NEW manual migration fixtures for normal03 and forced03, then run each beneath unchanged strict runner. No reuse of failednormal01, ordinary GUI, selected GUI or other slow homes. Root must bind actual reports/screenshots/normalized gate receipts and current source/CLI/manager/package hashes.
4. Any runtime failure remains failed. Report only the new run's observed outcome. Even success closes this canonical-history accepted-work scenario; the original response-only interrupted-history gap remains in the roadmap.

Use the existing private environment/TMPDIR/Chromium setup; no proxy, DNS, CA, sandbox guard, network-policy or credential changes. In-app Browser remains unavailable; actual Chromium fallback must be labeled as such. Root checks output absence and every final manifest/hash immediately before execution. The authored tests and scripts were AST parsed only by this worker.
