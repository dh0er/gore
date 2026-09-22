# Item and value mods without UE4SS — implementation plan

Status: plan only. No item/value behavior changes have been implemented by this document.

## Goal and scope

An authored item/stat mod must build, deploy, run, and undeploy on a clean Gothic 1
Remake install **without UE4SS**. Keep `overrides.toml` and bundle `overrides`
usable. Preserve their advertised integer, float, boolean, and string edits on
item, NPC, and ability defaults; add explicit support for existing weapon damage
tag entries. The same native path must serve the CLI and Mod Studio. Existing
save instances can retain their serialized values, as with today's CDO edits.

Importing third-party UE4SS mods may remain an optional Mod Manager feature. It
must not be a prerequisite for any GORE-authored item/value workflow.

## What exists, and what is missing

| Area | Existing capability | Gap to close |
|---|---|---|
| Authoring | `gore gen` accepts `overrides.toml`; bundles accept `overrides`. | Both emit UE4SS Lua, not a native item mod (`crates/gore/src/cmd/gen.rs`, `crates/gore-mod/src/lib.rs`). |
| Offline editing | `gore as patch-default` edits one proven scalar initializer; `patch-tag-map` edits one existing damage-map value. | No typed, declarative batch; unsupported strings, computed defaults, absent assignments/keys and other shapes; no bundle/manager integration (`docs/guide/angelscript-defaults.md`). |
| Native script route | `gore as emit` reconstructs class defaults, and module compilation/splicing can ship edits without UE4SS. The pristine BuildID 25168047 corpus had zero semantic differences in its whole-tree round trip. | No safe `overrides` → targeted default statements → compiled module pipeline; current checks must be rerun for each game generation (`docs/guide/scripts.md`, `crates/gore-as/DECOMPILER_STATUS.md`). |
| Discovery and typing | Shipping script cache and `Binds.Cache` provide much of the class/default schema. | `--model`, catalogs and some native-ancestry patch proofs still rely on UE4SS dumps or a `.usmap` found below `ue4ss`; replace that dependency for the native workflow (`docs/guide/catalogs-and-models.md`, `docs/guide/angelscript-defaults.md`). |
| Deployment and conflicts | Bundles already compose script mini-caches against the pristine cache and restore backups. | `overrides` still become `Ue4ssLua`; separate edits in one module need field-level composition instead of whole-module last-wins. An explicit conflict is needed when an authored script also changes the same default (`crates/gore-mod/src/lib.rs`). |
| Studio and docs | Mod Studio can stage typed `ItemPatch` entities. | ItemPatch build/deploy is blocked; README, getting-started, items and bundle guides still present UE4SS as necessary (`apps/mod-studio/README.md`). |

The old override schema also accepts a non-default `module` such as `G1R` for
classes outside the AngelScript package. Those targets cannot automatically be
lowered to an AngelScript class initializer. Inventory them explicitly; any
supported non-script target needs a proven native runtime or cooked-asset route,
and an unsupported target must fail before publication rather than fall back to
UE4SS silently.

The currently deployed test mod changes the live `PrecompiledScript_Shipping.Cache`.
Use GORE's pristine-source selection / owned `.gore-bak` for investigation and
builds, never the already modified live cache as the vanilla baseline. On that
pristine BuildID 25168047 cache, `default-sites` finds `UItFo_Apple.m_Value`
and `tag-map-sites` finds `UItMw_1H_Sword_Old_01.m_DamageBase` / physical-edge.
Those reads do **not** establish that the existing patch commands can replace
the entire old runtime-override feature set.

## Implementation sequence

1. **Inventory the old contract against the pristine game.** Enumerate actual
   item/NPC/ability target classes and field types used by `overrides.toml`, the
   bundle spec, and Studio. For each, record its owning module, default
   expression, type, and whether the native compiler can reproduce it. Include
   `m_Value`, `m_Weight`, `m_MaxStack`, one bool, one string, NPC health, and
   `m_DamageBase` tag entries and any non-`Angelscript` module targets. This
   determines which defaults need an authored statement rather than a
   fixed-byte edit, and which need a different native route. Reject unknown or
   ambiguous targets before writing a bundle.
2. **Build one native lowering path.** Convert validated declarations into
   targeted class-scope AngelScript `default` edits, grouped by module. Use the
   emitted source and existing standalone compiler, preserving every unrelated
   class/default in that module. Match class, field owner and type semantically;
   do not use byte offsets or free-form text replacement. Encode the four
   existing value kinds and damage-tag entries with exact type/range checks.
   Reinspect the compiled mini-cache to prove the requested defaults changed
   and unrelated defaults/functions remained semantically equivalent. Fail
   closed when a class or expression cannot be represented.
3. **Make the bundle and manager own the result.** Store declarative item edits
   as a native bundle component. At apply, resolve all enabled item edits in
   load order, report same-class/field conflicts, then compile each affected
   module once from the pristine base. Detect overlap with `scripts` edits;
   compose only when target identity can be proven, otherwise report a hard
   conflict. Reuse the existing script-cache backup/recovery and undeploy
   transaction. No generated UE4SS folder for native item bundles.
4. **Expose the workflow and remove mandatory dump inputs.** Keep the TOML/JSON
   authoring shape, add a CLI inspect/build path that shows current values,
   supported type, provenance and a precise refusal reason. Generate its schema
   from the game's shipped cache and `Binds.Cache`, or another GORE-owned
   exact-build source; do not require a UE4SS object dump, SDK header dump or
   `.usmap` for a supported native edit. Wire Studio `ItemPatch` to the same
   builder. Retain old Lua generation only as an explicitly named legacy option
   if backward compatibility warrants it; never select it implicitly.
5. **Verify once the slice is complete, then update claims.** Test compile and
   deployment on a clean install with no `ue4ss` directory; representative
   integer/float/bool/string and damage-map edits; two mods touching different
   defaults in one module; same-target conflict; coexistence with an unrelated
   script mod; reapply, undeploy and game-update refusal/rebase. Ask the user to
   confirm visible values in game, including a new-game case and the expected
   existing-save limitation. Only then switch README/guide quick starts and
   `gore doctor` from mandatory UE4SS to the native path.

Historical `dialog_topics` Lua insertion and third-party UE4SS import are
separate compatibility paths. The supported native dialog workflow already
avoids them. Audit remaining first-party callers before claiming that the
*entire* GORE suite has no UE4SS requirement; do not delete third-party import
support as a side effect of item migration.
