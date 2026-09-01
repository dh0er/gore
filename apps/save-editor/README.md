# GORE Save Editor

`gore-save` is the savegame editor for Gothic Remake. It
provides a Flutter interface backed by a Rust savegame core. It is one project
in the [gore](../../README.md) monorepo.

This is *not* modding — it changes your saved progress and never touches the
game install. For modding, use the [`gore` CLI](../../docs/guide/README.md) or
[Mod Studio](../mod-studio/README.md).

## Features

- Profile: Change difficulty settings; set the in-game time (play clock).
- Player & NPCs: Edit stats, attributes, all talents/skills and much more;
  revive a dead NPC (restore health, strip the death state).
- Position: Move the player to exact coordinates or a named location, with an
  optional facing direction. An NPC can be moved the same way, and his
  daily routine switched off so he stays where he was put instead of walking
  back; the editor records what it overwrote so the move can be undone.
- Inventory: Change count of existing items; add new items from a bundled
  catalog, grouped by the game's own inventory tabs; remove items; detect and
  repair damaged slot IDs; reset an inventory to a clean starting state.
  Hovering an item shows the stat block the game shows: type, damage or spell
  levels, protection, requirements, value and description.
- Game images: With the game installed, item pictures and the game's own
  attribute, protection, skill, quest and category icons are extracted from your
  installation and shown next to their labels, and the glossary shows the game's
  own character portraits. Everything works without them.
- Faction crimes: Clear an NPC's crimes to reset the hostility its guild holds
  against you.
- Glossary: Browse and edit NPC, creature, and location entries, including
  their individual text entries and NPC discovery states.
- Progression: Edit quest markers, NPC knowledge and events
- Trade: See what a merchant sells and how much ore he has to buy with; change
  stock counts, add or remove lines, and edit the restock baseline.
- Almost all data can be changed by changing the value of the internal property. Only for experimental use.
- Backups: Create them automatically, give them optional display labels,
  restore them, or permanently delete them after confirmation. Deletion cannot
  be undone.

## What it can not do

- Mod the game (no overrides, audio, textures, or scripts).
- Touch the game install in any way.
- Guarantee experimental raw property edits are safe — keep your own copy of
  important saves.

## Screenshots

[<img src="../../docs/images/screenshot_light.png" alt="Screenshot Light" width="600"/>](../../docs/images/screenshot_light.png)
[<img src="../../docs/images/screenshot_dark.png" alt="Screenshot Dark" width="600"/>](../../docs/images/screenshot_dark.png)

## Compatibility

Tested with Steam game version CL168781. Other game versions are not guaranteed:
the bundled item, item-stat and named-location catalogs come from specific game
cooks and can become stale when a patch moves, renames or changes game data.
Keep backups and use a Save Editor build qualified for the updated game
version.

The bundled item stats (`assets/item_stats.json`) carry what the shipped script
cache says about every item — its type tag, damage, requirements, value and
description key — plus the game's own inventory filter tables.
`assets/glossary_images.json` maps each glossary entry to its portrait file.
Regenerate both after a game update with the scripts in
`apps/save-editor/tools/`; each file header documents the commands.

The glossary portraits themselves are not bundled: they are loose PNGs in the
installation under `G1R/Story/Conversation/images/Glossary/`, read from there
directly.

## Installation & Updates

Download `gore-save-editor-<version>-setup.exe` from the
[latest release](https://github.com/dh0er/gore/releases/latest) and run it.
The app checks for updates on startup and prompts you when a new version is
available.

A portable zip is also attached to each release; the portable build does not
auto-update.

## Requirements

- Windows 10 or newer.
- Rust toolchain.
- Flutter with Windows desktop support.
- Visual Studio 2022 with Desktop development with C++.

The local development setup used for this repository is Flutter 3.44.0, Dart
3.12.0, and Rust 1.96.0.

## Build And Test

Dev commands run from this directory (`apps/save-editor/`). `cargo` resolves
the workspace at the monorepo root automatically.

Run the gore-save test suite:

```powershell
python test.py
```

Run the Flutter app (build the native core first so the loader's upward search
finds `gore_save.dll` under the workspace `target/`):

```powershell
cargo build
flutter run -d windows
```

Build a Windows release bundle (native DLL + Flutter release + packaged zip)
via the monorepo build script, from the repository root:

```powershell
python build.py gore-save-editor dist
```

Add `installer` instead of `dist` to also compile the Inno Setup installer.

## Safety

`gore-save` writes backups before modifying save files. Even so, keep your own
copy of important saves before editing them.

## License

This project is licensed under the MIT License. See [LICENSE](../../LICENSE).
