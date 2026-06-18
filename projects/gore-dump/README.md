# gore-dump

UE4SS Lua mod that reads live CDO values and writes:

- `gore_game_data.json` — item stat defaults; `gore-cli sync` ingests it to
  refresh the gore-mod editor with real defaults.
- `gore_loc_<lang>.json` — localized item/NPC/dialog names + descriptions for the
  language the game is currently set to. One file per language; `gore-cli` merges
  them into the loc catalogs both gore-save and gore-mod use.

## Use

1. Copy this `gore-dump/` folder into `<game>/Binaries/Win64/ue4ss/Mods/`.
2. Make sure UE4SS lists it (it ships with an empty `enabled.txt`).
3. Launch the game and wait ~15s — the stats dump (`gore_game_data.json`) runs
   automatically and lands in the game's working dir (usually `Binaries/Win64/`).

### Localized names (manual, one pass per language)

The engine language/culture API does **not** drive the Alkimia localization, so
there is no programmatic switch. Instead, per language:

1. Set the language in the game's **options menu**.
2. **Load a save** (NPC `CharacterDefinition` CDOs only exist in a loaded game;
   from the main menu kind=npc comes back empty).
3. In the UE4SS console run `gore-dump loc <lang>`, e.g. `gore-dump loc de`.
   The `<lang>` is just a label for the output file — use it consistently
   (`de en pl ru fr it es ja zh-Hans pt-BR`).
4. Repeat for each language. Each run writes `gore_loc_<lang>.json`.

Scope a run with a kinds list: `gore-dump loc de item,npc` (default: all three).

The loc dump blocks the game thread for up to ~80s with all kinds; items-only is
a few seconds.

## Feed the outputs back

- `gore-cli sync --dump gore_game_data.json --catalog item_catalog.json -o model.json`
- loc merge: `gore-cli` loc command over the `gore_loc_*.json` files.

Regenerate this mod (`gore-cli dump-mod`) after the item/NPC/knowledge sets
change so the bundled allow-lists stay current.
