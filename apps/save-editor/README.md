# GORE Save Editor

`gore-save` is the savegame editor for Gothic Remake. It
provides a Flutter interface backed by a Rust savegame core. It is one project
in the [gore](../../README.md) monorepo.

## Features

- Profile: Change difficulty settings
- Player: Edit stats, skills, location and much more
- Inventory: Change count of existing items; add new items from a bundled catalog with categorized browsing.
- Glossary: Browse and edit NPC, creature, and location entries, including
  their individual text entries and NPC discovery states.
- Progression: Edit quest markers, NPC knowledge and events
- Almost all data can be changed by changing the value of the internal property. Only for experimental use.
- Automatic backup creation.

## Screenshots

[<img src="../../docs/images/screenshot_light.png" alt="Screenshot Light" width="600"/>](../../docs/images/screenshot_light.png)
[<img src="../../docs/images/screenshot_dark.png" alt="Screenshot Dark" width="600"/>](../../docs/images/screenshot_dark.png)

## Compatibility

Tested with Steam game version CL168781. Should work with all versions.

## Installation & Updates

Download `GoresaveSetup-<version>.exe` from the
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
python build.py gore-save dist
```

Add `installer` instead of `dist` to also compile the Inno Setup installer.

## Safety

`gore-save` writes backups before modifying save files. Even so, keep your own
copy of important saves before editing them.

## License

This project is licensed under the MIT License. See [LICENSE](../../LICENSE).
