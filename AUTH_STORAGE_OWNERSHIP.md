# Durable auth: bounded dependency audit

Source-only audit of reload cohort `0742356b9ff442a4d38f6704928930602f92df0fdfe89bfb250897fda470729f`, using targeted source reads. Upstream: `openai/codex@d42056091aded7feb1d88ac7e83972108b2aa478`. No patch, full-tree rehash, build, test, cache/process action, backend access or credential inspection. Paths below are repository-relative.

## Implementations and complete mutation entrypoints

`codex-rs/login/src/auth/storage.rs:175` defines unconditional load/save/delete. Its implementations are:

| Implementation | Source line | Side effects / existing coordination |
| --- | --- | --- |
| FileAuthStorage |203|Truncates/writes/flushes auth.json; deletion removes it. No transaction or generation.|
| DirectKeyringAuthStorage |309|OS keyring save plus best-effort fallback-file cleanup; deletion touches both. KeyringStore exposes no CAS.|
| SecretsKeyringAuthStorage |387|Encrypted CodexAuth namespace plus file cleanup; deletion also removes direct-keyring credentials.|
| AutoAuthStorage |478|Keyring miss/error can read file; save errors can fall back to file; deletion spans resources.|
| EphemeralAuthStorage |543|Global process-local mutex/map, individual operations only; no version or absent tombstone.|
| ObservedStorage |storage_telemetry.rs:73|Delegating telemetry wrapper, no extra authority; Auto handles its own telemetry.|

All direct production trait mutations found converge on four manager.rs entrypoints: `persist_agent_identity_record:971` (save984), `save_auth:1192` (save1203), `persist_tokens:1642` (save1663), and public `logout:1017` (delete1027). Their caller cohort is:

| Writer family | Actual path/symbol |
| --- | --- |
| Initial/API/PAT/agent/external credentials |manager.rs::login_with_api_key1058, login_with_access_token1084, login_with_chatgpt_auth_tokens1172; auth/bedrock_api_key.rs30 and bedrock_access_keys.rs35 -> save_auth.|
| Browser/device login |login/src/server.rs::persist_tokens_async833 saves in spawn_blocking; server callback447 and device_code_auth.rs226 call it.|
| External install / refresh |auth_source.rs::install_external_auth80 and commit_external_auth162 -> persist_external_auth168 -> save_auth(Ephemeral). Install saves before final owner validation; refresh updates cache unconditionally after save.|
| Native token refresh |manager.rs::refresh_and_persist_chatgpt_token3123 -> persist_tokens1642, which loads the current record after the network response and patches/saves it.|
| Agent identity update |manager.rs774 -> ChatgptAuth helper963 -> persist_agent_identity_record971; its state mutex is not shared between independent auth objects/managers.|
| Logout/policy rejection |public logout_with_revoke1030, manager logout2996/logout_with_revoke3009, forced logout_with_message1485 -> logout_all_stores1505 -> ephemeral and configured-store deletion.|
| Public raw replacement |Exported save_auth/logout can be called without any AuthManager; preserve their explicit replacement/deletion semantics while routing through coordination.|

Workspace save_auth matches in TUI local_chatgpt_auth and remote-control websocket are cfg(test), not extra production writers. Nested fallback deletions and secrets initialization are additional physical effects behind these entrypoints.

## Existing locks and the hidden dependency

- Manager inner RwLock plus credentials→policy commit order (`auth_reload.rs:160–241`), per-manager refresh/agent semaphores, Arc-based source/cache revisions and owner+epoch policy stamps protect local state. Watch counters notify consumers; none is a durable store version.
- `codex-rs/secrets/src/local.rs:123/189/240` uses unlocked load/modify/encrypted save. Temp-file sync/rename at301 is not CAS and does not directory-fsync; Windows can remove before retry. A **load can create a missing keyring passphrase** at259. `secrets/src/lib.rs:184–202` makes CodexAuth, ManagedSecrets and McpOAuth share that key. Auth-only locking therefore misses sibling namespace first-key creation.
- Gateway provides a useful existing lifetime pattern: `gateway_auth_storage.rs::lock_credentials:19` locks a stable OS sidecar with a60s wait; `gateway_auth.rs::refresh:290` retains it through reread/exchange/save, and persist_pending264 clones it into the blocking writer. It has a separate namespace/passphrase. Cache token equality and watch-change checks (`gateway_auth.rs:298`, `gateway_auth_login.rs:205`) are **not persisted generations** and cannot represent equal-value/absence ABA across released operations.

## Smallest coherent contract

Introduce a small credential-storage transaction interface, separate from auth source/policy: **resolve domain → read versioned snapshot → conditionally mutate → observe/recover retained outcome**. Required semantics:

1. A domain covers actual file, keyring backend/service/account, encrypted namespace and fallback/cleanup resources. File/Keyring/Auto touching the same resources share coordination; mode-specific locks are insufficient. Preserve existing key derivation while resolving aliases before tickets. Shared secrets key initialization needs its own common physical-resource lock, including read-side initialization, in a fixed order.
2. Snapshot returns opaque `(incarnation, revision, Present/Absent)`. Every accepted save, equal-value save and absent delete advances revision. Keep one bounded domain record through absence, not an unbounded tombstone log. Exhaustion fails closed; metadata recreation/migration/recovery changes incarnation and invalidates outstanding tickets. Payload equality/mtime/ciphertext is not a version.
3. Under cooperative cross-process exclusion, compare the snapshot and admit one retained operation using exact source/cache/policy or install-intent ownership. Preparation and backend I/O stay outside manager/policy guards. All conflicting source transitions/writers join this order: B cannot complete publication while an earlier admitted A can still overwrite B. Immediate revoke can deny use, but legacy synchronous void clear cannot claim settled durable cleanup; its admission/settlement adapter is an explicit prerequisite.
4. File requires secure staged data, durable intent/generation metadata, replacement and file/directory sync. Keyring lacks native CAS: hold common exclusion and a write-ahead operation identity across write, fallback cleanup and revision commit. Ambiguous results stay Unconfirmed and block conflicting publication until recovery. Auto must preserve transactional genuine-unavailability fallback while **never translating Stale/Revoked/Unsupported/Unconfirmed into file fallback success**.
5. Cancellation ends observation, not an accepted write or its lock. Retain custody and a queryable operation receipt through blocking work, with one caller deadline. Only settled commits publish successor cache state. Async timeout does not establish bounded OS/keyring completion or clean process exit; later supervised process termination must report uncertainty.

A pre-save check cannot close check→write replacement/logout races. Post-save detection cannot undo the side effect, and blind rollback can overwrite a third writer. Strong guarantees require the whole participating writer cohort, not only refresh.

## Finite implementation order and component fit

1. After failure-only refresh acceptance, implement snapshot/mutation/outcome types and directly activate the complete Ephemeral caller slice (external install/refresh/save/delete), where no durable backend I/O is needed. This is a real caller change but remains process-local. Prepare File transaction/domain resolution next; **do not activate a File-only strong guarantee while legacy Keyring/Auto cleanup can still mutate the same file**. All overlapping raw writers must first use the same domain coordinator. Factor shared secrets first-key coordination separately.
2. Add File, DirectKeyring and encrypted/Auto journal/recovery adapters, then enable conditional durable operations only for a complete coordinated resource cohort. Nonoverlapping legacy domains may retain their existing behavior; unsupported conditional mode combinations remain explicit. Captured refresh/install versions cannot be replaced with a fresh snapshot simply to force a stale save through. Public save_auth/logout adapters represent explicit replacement/delete intents, not an implicit refresh CAS. Gateway can later reuse physical coordination while retaining its OAuth/cache and namespace semantics.
3. Acceptance: actual same-domain two-manager/two-process equal writes, absent delete/recreate, held old native/external response, competing login/logout and first-key creation; crash points around intent, backend write, cleanup, rename and revision commit; restart recovery or truthful unavailability. Preserve old read formats/migrations. **Unmodified older/third-party writers bypassing the protocol remain unsupported for transactional concurrency**: cooperative locks cannot detect their equal-value ABA. Require cooperating/exclusive writers; do not claim otherwise from successful merges or byte comparisons.

Existing ComponentAuth (`login/src/component_auth.rs:24`) supplies credentials; native login still owns persistence/policy. Reuse ComponentBinding/versioned capability negotiation for a later independently selected credential-storage service with bounded opaque domain/operation handles. Kernel retains grants, framing, supervision and budgets; storage owns transactions; auth owns source/policy. Existing `HostDependencyService` (`component-host/src/broker_api.rs:128`) forbids invoking another selected component from a leaf service. Do not bypass that with recursive auth→host-leaf→storage calls: begin with the native typed adapter and define an allowed peer-dependency route before external activation.

No code/extracted-family/runtime pass is claimed. Public clear settlement, shared-key initialization, durable journaling and migration of every writer are required implementation gates, not proven infrastructure.
