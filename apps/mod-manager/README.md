# GORE Mod Manager

> **Early Alpha:** this is a public preview, published as a GitHub prerelease.
> Use it on a test installation, keep your own backups, and expect rough edges.
> For the established expert workflow, use the
> [`gore` CLI](../../docs/guide/README.md).

A Windows app for running **many** mods at once. Where
[Mod Studio](../mod-studio/README.md) *authors* a single mod, Mod Manager owns
the multi-mod story: build a library, order it, see what collides, and apply the
whole enabled set to your install. It consumes the mod bundles Mod Studio (or
`gore mod build`) produces, plus foreign mods it did not build.

It is a GUI over the same engine as [`gore mgr`](../../docs/guide/mod-manager.md),
through the `dart:ffi` bridge (`gore_ffi.dll`), and shares that CLI's library
and loadout files. Installer builds can check for updates through WinSparkle;
portable builds are deliberately updater-free.

## What it can do

- **Import** into a local library: built mod-bundle folders/zips (with a root
  `gore-mod.json`), foreign mod zips/folders, loose `_P.pak` files, IoStore
  triplets (`.utoc`/`.ucas`/`.pak`), UE4SS Lua mod folders, and raw game-file
  replacements.
- **Enable/disable**, **remove**, and **drag to reorder** mods in the load order
  (later wins). Removing a library entry updates the target loadout; if that
  mod is already deployed, choose **Apply** afterwards to update the game.
- **Detect recognized conflicts** across mods — localization, audio,
  texture/asset, item overrides (CDO), scripts, and raw-file replacements — and
  show intended winners. Each component is marked Exact, Partial, Advisory, or
  Opaque so incomplete target knowledge stays visible; these grades do not
  claim the game's runtime priority is proven.
- **Apply** declaratively: full-recompute the modded state from a pristine base
  and deploy the whole enabled set (backups first), or **undeploy all** to
  restore.
- **Take over** a Mod Studio test-deploy so both tools do not fight over the
  install.

## What it can not do

- *Author* a mod (edit item values, text, audio, textures) — that's
  [Mod Studio](../mod-studio/README.md) or the
  [`gore` CLI](../../docs/guide/README.md).
- Edit **save files** — that's the [Save Editor](../save-editor/README.md).
- Download mods (no Nexus API integration) — import files you already have.

## Packages, updates, and persistent data

- The **installer** adds an uninstaller and enables best-effort WinSparkle
  update checks after launch. Updates download the next installer and update
  the existing install location in place.
- The **portable zip** has no installer, uninstaller, or updater binaries.
  Extract it to a normal writable folder, run `gore_manager.exe`, and replace
  that extracted app folder manually when updating. “Portable” describes the
  app package; user data is not stored beside the executable.
- Both packages normally share `%LOCALAPPDATA%\gore\mod-manager\` for the
  imported mod library and loadout, `%LOCALAPPDATA%\gore\config.json` for
  shared GORE settings such as the selected game, and
  `%LOCALAPPDATA%\gore\gore-manager\` for Manager-only UI preferences. If
  `%LOCALAPPDATA%` is unavailable, the app uses the same relative paths under
  `%APPDATA%` instead.
- Before uninstalling or deleting a portable copy, use **Undeploy** if Manager
  has applied mods. Removing the app does not undo a deployment in the game
  directory. The installer uninstaller removes the installed app and its
  normal `%LOCALAPPDATA%` UI preferences; it intentionally preserves the
  shared config, imported library, and loadout. In the `%APPDATA%` fallback
  case, remove `%APPDATA%\gore\gore-manager\` manually to discard UI
  preferences. Delete the active data root's `gore\mod-manager\` directory
  manually only if you also want to discard the retained library and loadout.

## Build / run

Driven by the top-level orchestrator (see [Building](../../docs/development.md)):

```powershell
python build.py gore-mod-manager run          # build (if needed) + launch
python build.py gore-mod-manager dist         # portable zip
python build.py gore-mod-manager installer    # Windows installer
python build.py gore-mod-manager test         # cargo (gore-ffi) + flutter analyze + test
```
