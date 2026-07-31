# Catalogs & data models

GORE's GUIs and its override validation need to know which classes exist, which
fields they have, what those fields are typed as, and what their real in-game
default values are. That knowledge lives in generated JSON: **catalogs** (what
exists) and **reflection models** (what shape it has).

**Most people never run any of this.** The catalogs bundled with the save editor
(`apps/save-editor/assets/*_catalog.json`) already cover everyday modding. Reach
for this page only when you are regenerating that data yourself — after a game
update, or when you want `gore gen --model` to validate your overrides.

## Catalogs from a UE4SS object dump

```powershell
gore catalog --kind item      UE4SS_ObjectDump.txt -o item_catalog.json
gore catalog --kind npc       UE4SS_ObjectDump.txt -o npc_catalog.json
gore catalog --kind knowledge UE4SS_ObjectDump.txt -o knowledge_catalog.json
```

`--kind` is one of `item`, `npc`, `knowledge`. For the knowledge catalog,
`--script-cache <CACHE>` enriches entries with captions read from the shipping
script cache. It must be a module cache: the `0x9e377abe` magic is checked
before the captions are read, so `Binds.Cache` is refused by name.

## The story catalog

```powershell
gore story-catalog --exe   "$GAME\G1R\Binaries\Win64\G1R-Win64-Shipping.exe" `
                   --cache "$GAME\G1R\Script\PrecompiledScript_Shipping.Cache" `
                   --binds "$GAME\G1R\Script\Binds.Cache" `
                   -o story_catalog.json
```

Builds a strict, **generation-sealed** NPC and quest-parent catalog
(`story_catalog.v1`). All three inputs must belong to the exact same installed
generation — the executable, the shipping precompiled AngelScript cache, and
the Binds cache. Mismatched or ambiguous evidence fails closed rather than
producing a catalog that silently describes a different build.

This is the catalog behind the managed NPC and Quest workflows in
[Mod Studio](mod-studio.md).

## Reflection model from an SDK header dump

```powershell
gore dump  CXXHeaderDump -o model.json      # field schema (names + types)
gore stubs model.json -o stubs              # optional LuaLS/EmmyLua type stubs
gore stubs model.json -o stubs --filter It  # …only classes with this name prefix
```

`model.json` is what `gore gen --model` validates overrides against, and what
the GUI shape model is derived from.

## Real in-game default values

A header dump gives types, not values. For the *real* defaults (so the GUI and
editor show accurate numbers), run the `gore-dump` UE4SS mod in game and fold
its output back in:

```powershell
# 1. generate the dump mod into the game's Mods dir
gore dump-mod --model model.json --catalog item_catalog.json -o "$GAME\...\Mods"

# 2. launch the game once with it enabled → writes gore_game_data.json

# 3. fold the runtime values back into the model
gore sync --dump gore_game_data.json --catalog item_catalog.json -o model.json
```

The catalog acts as the item allow-list in both steps. The mod source lives in
[`mods/gore-dump`](../../mods/gore-dump/README.md).

## GUI shape model

```powershell
gore gui-model --model model.json --catalog item_catalog.json -o gui_model.json
```

Converts a reflection model plus an item catalog into the shape the GUI apps
consume.

## Command summary

| Command | Input | Output |
|---|---|---|
| `catalog --kind item\|npc\|knowledge` | `UE4SS_ObjectDump.txt` | catalog JSON |
| `story-catalog` | exe + shipping cache + Binds cache | `story_catalog.v1` JSON |
| `dump` | `CXXHeaderDump/` | reflection `model.json` |
| `stubs` | `model.json` | LuaLS/EmmyLua `.lua` stubs |
| `gui-model` | `model.json` + item catalog | GUI shape JSON |
| `dump-mod` | `model.json` + item catalog | the `gore-dump` UE4SS mod |
| `sync` | `gore_game_data.json` + item catalog | refreshed GUI model |

## Related

- [Item & stat values](items.md) — where `--model` validation is used.
- [Localization catalog](text-and-dialogs.md#the-shared-catalog) — the separate
  shared catalog that resolves text ids to readable names.
