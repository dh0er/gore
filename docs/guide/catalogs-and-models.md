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

## The named-location catalog

Where a character stands is stored in a save as a coordinate triple plus, when
they are using one, the *name* of an interaction spot (`UsedSpot > Spotname`).
The game ships every spot of the main map as loose, unencrypted JSON, so the
catalog that turns those names into a browsable list needs no dump and no
running game:

```powershell
gore location-catalog "$GAME\G1R\Script\Map\MainMap\InteractionSpots.json" `
    -o apps\save-editor\assets\location_catalog.json
```

The source path is optional: without it the file is read from the resolved game
install, like every other command that needs the game. It prints what it dropped
and why. The ~10 MB source becomes ~860 KB:

```json
{ "version": 1,
  "areas": [{"id":"OC","label":"Old Camp","locId":"area_oldcamp_notification"}],
  "spots": [{"n":"FP_OC_STAND_YARD_1","x":0.0,"y":0.0,"z":0.0,"w":0.0,"a":"OC"}] }
```

The keys are short because there are ten thousand spots. `w` is **yaw only** —
a spot's pitch would visibly tilt a standing pawn, so pitch and roll are absent
from the asset rather than merely ignored by the editor. Placeholder spots at
the world origin, duplicate names, and level-design-only data layers (greybox
`*_Blockout`, `LoadingScreenShots`, `Demo_*`) are dropped; conditional quest
layers are kept, because relocating a character into a quest area is a
legitimate edit.

`a` is an area code from the `areas` table, assigned first from the spot name
(`FP_`**`OC`**`_STAND_YARD_1`) against a curated list of real territory codes,
then — for spots whose name carries none — by a majority vote of the five
nearest already-labelled spots. A spot with no labelled neighbour within
20,000 uu keeps an empty area. Areas that have a localized name carry its
`locId`, which the editor resolves through the game's localization catalog
first. For the eight areas the game does not name cleanly, the editor uses an
explicit localized UI mapping when one is present, then falls back to the
generated English `label` as a safety net:
`game localization ?? editor ARB mapping ?? area.label`.

One quirk of these 17 `area_*` ids: their German string lives in `german`, and
`german_new` is null. That is the **inverse** of the usual rule, where `german`
is populated on only a fraction of ids and `german_new` carries the shipping
translation. Reading `german_new` alone leaves every area name untranslated.

The curated code list is worth a second look after a patch, because the codes in
the territory classes and the codes in the spot names do not always agree — the
Tundra is `HC` in `TerritoryConfigs_Tundra.as` but `TA` in every spot name, and
the generator aliases the two together. A code that matches no spot at all is
the symptom; the command prints it under `curated codes with no spot:`.

The spot names and coordinates come from one cook. **Regenerate the catalog
after a game patch** — a moved or renamed spot is silently wrong otherwise.

## Checking a spot name before the game swallows it

```powershell
gore location resolve FP_OC_STAND_YARD_1   # area, coordinates, yaw
gore location resolve fp_oc_stand_yard_1   # the same spot: matching is case-insensitive
gore location list --area OC --prefix FP   # what an area actually contains
```

`TeleportToWaypointAndExchangeDailyRoutineToClass(npcState, routine, FName
Waypoint)` and `TeleportToSpot(charState, FName)` both resolve their name
through `FInteractionSpotHandle`, and its invalid branch is **empty**: an unknown
name does nothing, logs nothing and fails nothing — the character simply stays
put. A typo in a mod script is swallowed whole. `resolve` exits non-zero on a
miss and suggests the near names, which is the cheap place to catch it.

Knowing a name is not the same as being able to use it. Neither subcommand
places, spawns or moves anybody — they only say whether a spot exists, for a
script that is going to name it. What the toolkit can and cannot do about
putting a character somewhere is [Spawn and
placement](mod-studio.md#spawn-and-placement).

Both subcommands read the catalog compiled into `gore.exe`, so they answer with
no game install, no dump and no regeneration step. Names are compared
case-insensitively because `FName` comparison in the game is, and the spellings
drift: the same waypoint is `WP_ExF_…` in AngelScript and `WP_EXf_…` in a save.
`list` stops at `--max` names and says how many it left out, and a `--prefix`
nothing starts with comes back with the prefixes that do exist in that scope —
an empty listing on its own cannot be told apart from an empty area.

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
# 1. derive the GUI shape model, then generate the dump mod into the game's Mods dir
gore gui-model --model model.json --catalog item_catalog.json -o gui_model.json
gore dump-mod --model gui_model.json --catalog item_catalog.json -o "$GAME\...\Mods"

# 2. launch the game once with it enabled → writes gore_game_data.json

# 3. fold the runtime values back into the GUI model
gore sync --dump gore_game_data.json --catalog item_catalog.json -o gui_model.json
```

The catalog acts as the item allow-list in every step. The mod source lives in
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
| `location-catalog` | `InteractionSpots.json` | `location_catalog.json` |
| `location resolve\|list` | a spot name (the bundled catalog) | area, coordinates, yaw |
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
