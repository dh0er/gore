# gore-dump

UE4SS Lua mod that reads live CDO values and writes two files:

- `gore_game_data.json` — item stat defaults; `gore-cli sync` ingests it to
  refresh the gore-mod editor with real defaults.
- `gore_loc.json` — localized item/NPC/dialog names + descriptions in every
  shipped culture; ingested into the loc catalogs both gore-save and gore-mod use.

## Use

1. Copy this `gore-dump/` folder into `<game>/Binaries/Win64/ue4ss/Mods/`.
2. Make sure UE4SS lists it (it ships with an empty `enabled.txt`).
3. (Optional) edit `Scripts/config.lua` to choose what auto-dumps and which
   cultures/kinds the loc dump covers.
4. Launch the game, reach the main menu (or load a save), wait ~15s for the
   auto-dump. Both JSONs land in the game's working dir (usually
   `Binaries/Win64/`). Watch the UE4SS console for paths and the discovered
   culture list.
5. Re-dump on demand from the UE4SS console without relaunching:
   - `gore-dump stats`            — stats only
   - `gore-dump loc`              — loc, config scope
   - `gore-dump loc item en,de`   — only items, only EN+DE
   - `gore-dump all`              — both
6. Feed the outputs back:
   - `gore-cli sync --dump gore_game_data.json --catalog item_catalog.json -o model.json`
   - (loc ingest: see `gore-cli` loc command)

If `gore-dump loc` reports "no cultures resolved", pin the list in `config.lua`
(`cultures = {"en","de",...}`) using the codes the console logs at load.

Regenerate this mod (`gore-cli dump-mod`) after the item/NPC/knowledge sets
change so the bundled allow-lists stay current.
