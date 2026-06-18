# gore-dump

UE4SS Lua mod that reads the live default values of every catalog item and
writes `gore_game_data.json`, which `gore-cli sync` ingests to refresh the
gore-mod editor with real defaults.

## Use

1. Copy this `gore-dump/` folder into `<game>/Binaries/Win64/ue4ss/Mods/`.
2. Make sure UE4SS lists it (it ships with an empty `enabled.txt`).
3. Launch the game, load any save (or reach the main menu), wait ~15s.
4. `gore_game_data.json` is written to the game's working directory
   (usually `Binaries/Win64/`). Watch the UE4SS console for the path.
5. `gore-cli sync --dump gore_game_data.json --catalog item_catalog.json -o model.json`
   then rebuild/point gore-mod at the new model.

Regenerate this mod (`gore-cli dump-mod`) after the item set changes so
`items.lua` covers new classes/fields.
