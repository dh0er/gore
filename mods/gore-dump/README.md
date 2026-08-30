# gore-dump

UE4SS Lua mod that reads the live default values of every catalog item and
writes `gore_game_data.json`, which `gore sync` ingests to refresh the
gore-mod editor with real defaults. The same run also recovers the FMOD bank
encryption key (`gore_fmod_key.json`) so gore can open the game's encrypted
`.bank` audio files.

## Use

1. Copy this `gore-dump/` folder into `<game>/G1R/Binaries/Win64/ue4ss/Mods/`.
2. Make sure UE4SS lists it (it ships with an empty `enabled.txt`).
3. Launch the game, load any save (or reach the main menu), wait ~15s.
4. `gore_game_data.json` and `gore_fmod_key.json` are written to the game's
   working directory (usually `G1R/Binaries/Win64/`). Watch the UE4SS console for
   the paths.
5. `gore sync --dump gore_game_data.json --catalog item_catalog.json -o gui_model.json`
   then rebuild/point gore-mod at the new GUI model.

`gore_fmod_key.json` looks like `{"found":true,"encryption_key":"…","master_bank_name":"Master"}`.
The key is read from the live `UFMODSettings` CDO (`/Script/FMODStudio`,
`StudioBankKey`); it stays constant until a game patch changes it.

Regenerate this mod (`gore dump-mod`) after the item set changes so
`items.lua` covers new classes/fields.

Localized names/descriptions are not produced here — use `gore loc export`
to read them straight from the game's `.lcache`.
