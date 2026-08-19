# GORE Mod Manager

> **Experimental 0.1.0:** this first public prerelease is intended for testing on
> a non-critical installation; keep your own backups and expect rough edges.
> For the established expert workflow, use the
> [`gore` CLI](../../docs/guide/README.md).

A Windows app for running **many** mods at once. Where
[Mod Studio](../mod-studio/README.md) *authors* a single mod, Mod Manager owns
the multi-mod story: build a library, order it, see what collides, and apply the
whole enabled set to your install. It consumes the mod bundles Mod Studio (or
`gore mod build`) produces, plus foreign mods it did not build.

It is a GUI over the same engine as [`gore mgr`](../../docs/guide/mod-manager.md),
through the `dart:ffi` bridge (`gore_ffi.dll`), and shares that CLI's library
and loadout files. Both packages check for updates; the installer updates
itself, the portable zip points you at the download.

## What it can do

- **Import** into a local library: built mod-bundle folders/zips (with a root
  `gore-mod.json`), foreign mod zips/folders, loose `_P.pak` files, IoStore
  pairs (`.utoc`/`.ucas`, with an optional same-stem `.pak`), UE4SS Lua mod
  folders, and raw `.lcache`, `.bank`, or `PrecompiledScript*.Cache` game-file
  replacements. Extract `.7z` and `.rar` downloads first. Multipart or
  incomplete IoStore sets and unknown, unsafe, or corrupt inputs are refused
  without publishing a partial library entry or changing the loadout.
- **Enable/disable**, **remove**, and **drag to reorder** mods in the load order
  (later wins). Removing a library entry updates the target loadout; if that
  mod is already deployed, choose **Apply** afterwards to update the game.
- **Detect recognized conflicts** across mods — localization, audio, voice
  archives, texture/asset containers, item overrides (CDO), scripts, loose and
  packed game-file claims, and raw-file replacements — and show intended
  winners. Loose-file versus packed-file precedence is advisory, and an opaque
  UE4SS component cannot produce a complete target inventory. Each component
  is graded Complete, Partial, Estimated, or Unknown so incomplete knowledge
  stays visible; these grades do not claim the game's runtime priority is
  proven. The grades and the per-component target lists live behind the
  **Advanced details** setting.
- **Apply** declaratively: full-recompute the modded state from a pristine base
  and deploy the whole enabled set (backups first), or use the normal
  **Remove all from game** action to restore a validated Manager-owned
  deployment. A
  Mod Studio deployment is preserved unless you explicitly choose and confirm
  the separate **Take over** workflow below.
- **Recover an interrupted Manager change** after confirmation when the native
  setup check can identify a clearly abandoned Manager operation. The app binds
  its action to that exact report; the equivalent expert route is `gore mgr
  preflight` followed by `gore mgr recover --expected-guard-id <TOKEN>`. Active
  changes stay on the wait path. Script-build recovery and recovery data that
  GORE cannot identify stay in recovery help. Do not delete the installation
  lock by hand; check the status again before removing mods from the game.
- **Inspect the files GORE manages** in the status details for an exact
  Manager-owned record: replaced files, backups of the originals, added mod
  files, UE4SS directories, and repair paths. These bounded, selectable paths
  are record evidence only; they do not claim that a path still exists or grant
  a cleanup action. The record is shown while **Advanced details** is on.
- **Turn Advanced details on or off** in Settings. Off by default, so the app
  reads as a mod manager rather than a diagnostic tool; on, it adds the
  technical layer described above plus the import source and match reason.
- **Take over** a Mod Studio test-deploy so both tools do not fight over the
  install.
- **Start the game** from the button at the far right of the tab row. It
  resolves the executable from the configured game path and is disabled while a
  Manager operation is writing to the installation.

## Real-install evidence

On 2026-08-18 one packaged real-install campaign exercised four genuine Nexus
mods: Main Menu Replacer — Remake (#244), Mainmenu Sleeper Enhanced (#512),
Gothic UI Reposition (#269), and Attack Input V4. Numeric container priorities
were observed in both order directions: `#244 -> #512` showed the Sleeper /
Gothic-II menu, while `#512 -> #244` showed the red Remake artwork. A new game
and an existing save loaded, and the tested enable, disable, reorder, Apply, and
Reset paths behaved as expected.

The same campaign rendered the GORE-authored Viper choice `[Gore probe] UI
fixture`; `UE4SS.log` recorded `ARMED`, `CHOICE_PASS`, and `RENDER_PASS` with
`exact_count=1`. That is live AngelScript composition evidence, but not evidence
for a third-party AngelScript mod or a three-way script conflict: this probe used
the PR #91-fixed app-local Core DLL. #269 was disabled for the probe after an
earlier crash was isolated to its own UE4SS Lua loop calling `FindAllOf` off the
game thread; no GORE or AngelScript frame was present. No save was written by
the probe.

After testing, the captured loadout was restored byte-for-byte, all temporary
campaign entries and game-tree payloads were removed, the original signed Core
DLL was restored, and the original four-mod deployment reported in sync. This
is one person and one Windows installation. Clean-Windows portable, installer,
recovery, Reset, and uninstall acceptance remains open as a known experimental
limitation.

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
- The **portable zip** has no installer, uninstaller, or updater binaries. It
  still checks the same release feed and, when a newer version exists, offers
  to open its download page; replace the extracted app folder yourself.
  Extract it to a normal writable folder and run `gore_manager.exe`.
  “Portable” describes the app package; user data is not stored beside the
  executable.
- Both checks can be turned off, and run on demand, under **Settings →
  Updates**.
- Both packages normally share `%LOCALAPPDATA%\gore\mod-manager\` for the
  imported mod library and loadout, `%LOCALAPPDATA%\gore\config.json` for
  shared GORE settings such as the selected game, and
  `%LOCALAPPDATA%\gore\gore-manager\` for Manager-only UI preferences. If
  `%LOCALAPPDATA%` is unavailable, the app uses the same relative paths under
  `%APPDATA%` instead.
- Before uninstalling or deleting a portable copy, use **Remove all from
  game** if Manager has applied mods. Removing the app does not undo a deployment in the game
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
