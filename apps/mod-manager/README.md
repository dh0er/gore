# GORE Mod Manager

> **Development status:** work in progress — not yet ready for general use. For
> stable modding today, use the [`gore` CLI](../../docs/guide/README.md).

A Windows app for running **many** mods at once. Where
[Mod Studio](../mod-studio/README.md) *authors* a single mod, Mod Manager owns
the multi-mod story: build a library, order it, see what collides, and apply the
whole enabled set to your install. It consumes the mod bundles Mod Studio (or
`gore mod build`) produces, plus foreign mods it did not build.

It is a GUI over the same engine as [`gore mgr`](../../docs/guide/mod-manager.md),
through the `dart:ffi` bridge (`gore_ffi.dll`), and shares that CLI's library
and loadout files. Auto-updates on launch (WinSparkle).

## What it can do

- **Import** into a local library: built mod-bundle folders/zips (with a root
  `gore-mod.json`), foreign mod zips/folders, loose `_P.pak` files, IoStore
  triplets (`.utoc`/`.ucas`/`.pak`), UE4SS Lua mod folders, and raw game-file
  replacements.
- **Enable/disable** mods and **drag to reorder** the load order (later wins).
- **Detect conflicts** across mods — localization, audio, texture/asset, item
  overrides (CDO), scripts, and raw-file replacements — and show which mod wins.
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

## Build / run

Driven by the top-level orchestrator (see [Building](../../docs/development.md)):

```powershell
python build.py gore-mod-manager run          # build (if needed) + launch
python build.py gore-mod-manager dist         # portable zip
python build.py gore-mod-manager installer    # Windows installer
python build.py gore-mod-manager test         # cargo (gore-ffi) + flutter analyze + test
```
