# Context semantics extraction provenance

This is an unlinked source extraction from the native Codex harness in this
checkout. It is not activated or compiled yet. The baseline is OpenAI Codex
`d42056091aded7feb1d88ac7e83972108b2aa478` from
<https://github.com/openai/codex>. Repository `LICENSE` and `NOTICE` apply.
No external harness code was borrowed.

The classification source also contains this fork's existing
`ComponentContextFragment` recognition. That behavior is deliberately retained;
it is not claimed to be part of the pinned upstream revision. Source hashes below
identify the actual files read during this extraction.

## Boundaries and reuse

`semantics/mod.rs` exports the native user/developer contextual classifiers,
Guardian context and user authorization classifiers, visible hook parser,
`parse_turn_item`, model-switch/persistent-mode matching helpers, and
`OMITTED_OBJECTIVE_KIND`. Pure user-goal, internal-context, and Guardian-retained
fragment types are also available for native history restoration and rendering.

Dependencies: `codex-context-fragments`, `codex-protocol`, `serde_json`, `tracing`,
and `uuid`. Native tests additionally require `pretty_assertions`. No dependency
on `codex-core`, `codex-skills-extension`, `codex-tools`, or an extension runtime
is introduced by these modules.

The existing context-fragments crate directly supplies the shared fragment trait,
additional user context, and component context. Pure native fragment types were
copied without behavior edits. The Guardian retained fragment's type visibility
and the goal omission constant's visibility are widened for the new package API.
Context classifiers and event mapping change imports and public visibility only,
except for replacing runtime-owned producer types with their exact pure matcher.

The native default `matches_marked_text` helper is private in
`codex-context-fragments`. Its exact body is temporarily copied here, with native
marker pairs for environment context, legacy message-board notifications, skill
instructions, recommendations, model switches, and persistent mode. Importing the
runtime producers solely to recognize their persisted markers would pull their
service dependencies back into the extracted context engine.

## Preserved semantics

- Default marker matching is ASCII case-insensitive, trims boundary whitespace,
  requires both boundaries, and uses checked UTF-8 string slicing.
- Custom external-context and internal-context matchers retain their native
  case sensitivity and validation; legacy warnings keep exact native prefixes
  and suffixes. The legacy goal wrapper remains recognized.
- User authorization comes from complete host annotations. Missing, empty,
  incomplete, unknown, and media-fallback annotations retain native conservative
  handling. Merely matching a wrapper does not establish provenance.
- A host-annotated user goal is exempt from hidden Guardian context. Internal
  steering using the same source name without that annotation is not exempt.
- Guardian retained evidence needs nonempty annotations covering all content
  items. Its existing 900-token byte-budget limit and error are unchanged.
- Hook parsing remains the protocol parser. Visible hook messages require only
  hook fragments and recognized contextual text; ordinary text or media prevents
  visibility as a hook-only message.
- The full user event parser is preserved, including adjacency checks for media
  label removal, image details, audio URLs, and exclusion of contextual input.
  Assistant phase/IDs, reasoning content, web-search action details, and image
  generation fields are copied without simplifying their mappings.

## Validation status and integration obligations

Read-only source-equivalence checks confirmed the copied pure modules, event
mapper, web-search helpers, default matcher, and existing tests after the listed
import/visibility adaptations. All production modules are below 500 lines.
The native contextual-user and event-mapping tests were copied with import
adaptations. Additional source tests cover annotation provenance and exact
case/whitespace behavior. **No Rust command or runtime test has been run on this
staged extraction.** These are test sources, not passing test evidence.

The parent extraction must add the package manifest/dependencies, expose
`semantics`, format under the coordinated workspace slot, and run focused tests
plus the native-vs-extracted history/replay differential checks. It must activate
core call sites explicitly; none are changed by this stage. A standalone plugin
requires the remaining history engine and lifecycle integration as well.

## Read source fingerprints

Paths are relative to `codex-rs`. SHA-256 fingerprints capture the extraction
input, including existing fork changes.

| Source | SHA-256 |
| --- | --- |
| `core/src/context/user_goal.rs` | `92ac398ee8f0066c18c24b848b9c0588e414174c023bb2b15bfe80c54e11fff6` |
| `core/src/context/internal_model_context.rs` | `f5cf87835ffad3de6a8ece56aa218ecd8d3444b9878e79fed88f620bcf110a3d` |
| `core/src/context/guardian_retained_instructions.rs` | `49aaf719bdc221f0909c1a5919cfd77695e42d03ffb9b5368fb28cd2d49221b2` |
| `core/src/context/user_instructions.rs` | `fd331f381cd81c788c50b951fb7f5641581cdd5593ff1cc92dd47428b42ea7b9` |
| `core/src/context/user_shell_command.rs` | `da53ea5b48e3bfce73130e8fa531eb08592838c740e15ed4fbe078acb9dc15da` |
| `core/src/context/turn_aborted.rs` | `d9450f4e3e0f7e6ed81b38dd5cad8e91f21e9fdfc0297778843bec0eefa858c1` |
| `core/src/context/subagent_notification.rs` | `e4824dfe4e6602039fcd27626993985714936b1a6855daa07c221b73a40380d0` |
| `core/src/context/legacy_apply_patch_exec_command_warning.rs` | `6198c619ff05df5a9dc3fdd3ea78035b24124d2ab4f6d70bfbeacf9ba1869ba3` |
| `core/src/context/legacy_model_mismatch_warning.rs` | `9eba5d2fdde56ae5169565485357b6269d18fe2fac4038bc85e6aa38c88df379` |
| `core/src/context/legacy_unified_exec_process_limit_warning.rs` | `9a42ce8c54031b7e5b9eed59bafd5cfacdf2a7d3b6be12b4cf4fae8b37ca8950` |
| `core/src/context/contextual_user_message.rs` | `a783d4fd9d2e94b2e0bf88693176786c765eee249f26906a135bc85cb4356be0` |
| `core/src/context/contextual_user_message_tests.rs` | `5164fc616bc44e98c88d20fa1deb58c7e08ec5ee5265442d8aa70f65b0559d15` |
| `core/src/event_mapping.rs` | `3df7b3ea05c83485c5d6994e6db997368da6507be89968e1aac5a6f3d6b9efee` |
| `core/src/event_mapping_tests.rs` | `2be7f7e89b9a7a1973474b81031ab26c587ac4e44445e802b02f10b923a21d31` |
| `core/src/web_search.rs` | `7029c8ad137965387cba1b499d653913110d6b10e209689bea0c78a891854d37` |
| `context-fragments/src/fragment.rs` | `b48f8533c5f4eb7e8f102c2ab86a9e436e1daae0669f6cb214b06c32365c5dff` |
| `context-fragments/src/additional_context.rs` | `2a837d023deec9416dff460789555b015892d3dc714a0888e1d1135cb1a5e760` |
| `context-fragments/src/component_context.rs` | `50e4ff84c4398376a8cdac73f4716440b71f8e04897a856b8ffcee7a4e9c1b1b` |
| `core/src/context/world_state/environment.rs` | `76af9f375d42c840ce554b6f215d1603d481c60f9a417c1a7008e149b4625fd6` |
| `core/src/context/agent_message_board_notification.rs` | `2be43431555ee221015eb36cd13530c8b98b046850e87fc82273537e34d7380a` |
| `core/src/context/recommended_plugins_instructions.rs` | `42cedd51b837c6e0d101172978201273502e6747deda2b4224207823c08b59d2` |
| `core/src/context/model_switch_instructions.rs` | `99230e5d0e491c597211b00e61b019617cf55a30d2c8dbbe522c6ae1eaf1a205` |
| `core/src/context/world_state/persistent_mode.rs` | `3ea6da5bc0c7dd3fb5e3899baaac4a724234b099a3f45018061cf489ac62781b` |
| `core/src/context/approved_command_prefix_saved.rs` | `5723d9d246f0f04639148d10a654add82e083447aa94ad0494a341805f1503de` |
| `ext/skills/src/fragments.rs` | `4e61b21994402a08554ae02352a3eb5c99f13f7a7f974ec656a22a3b5eb05a42` |
| `ext/skills/src/lib.rs` | `d18c2d509dc9a7fc543a885c6497105cf2ede568aa5336243168254dad282c54` |
