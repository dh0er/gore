# Getting started

This is the entry point for modding Gothic 1 Remake with GORE. It covers
installing the `gore` CLI, pointing it at your game, and choosing the right
tool for the job.

If you only want to edit a **save game**, you do not need any of this — use the
[save editor](../../apps/save-editor/README.md) instead. It never touches the
game install.

## Install the CLI

Two ways to get `gore.exe`:

**Download a release.** Grab a `gore-cli-v*` asset from the
[releases page](https://github.com/dh0er/gore/releases). The zip contains
`gore.exe`, the `shared\` Lua SDK, and this whole guide under `docs\` — so the
documentation is available offline, right next to the binary.

**Build it yourself.** Requires a stable Rust toolchain:

```powershell
cargo build --release -p gore
# → target\release\gore.exe
```

See [Building](building.md) for the full toolchain requirements and the
`build.py` orchestrator that also builds the GUI apps.

GORE is Windows-only. Every example in this documentation is PowerShell, assumes
`gore` is on your `PATH`, and uses the variable `$GAME` for your install root —
the folder that contains `G1R\`:

```powershell
$GAME = 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake'
```

Use double quotes when you build a path from it (`"$GAME\G1R\..."`); single
quotes do not expand variables.

## Point GORE at the game

Set the game path once, and every command that needs it can omit `--game`:

```powershell
gore config set game-path $GAME     # an install root or the game .exe
gore config detect                  # …or auto-detect a Steam install and save it
gore config list                    # show the stored value + resolved root
gore config get game-path           # print just the value (non-zero exit if unset)
gore config unset game-path         # clear it
gore config path                    # where config.json lives
```

Resolution precedence for every command that needs the install:

1. an explicit `--game` on the command line (`--lcache` for `gore loc`, `--exe`
   for `gore as diagnostics-check`),
2. the configured `game-path`,
3. Steam auto-detect.

The value is stored in a shared `config.json` (`gore config path`) that the GUI
apps read too, so the install is configured in exactly one place.

## Which tool for which job

| You want to… | Use |
|---|---|
| change values, text, audio, textures or scripts of the game | the `gore` CLI (this guide) |
| do the same without a terminal, for one mod | [Mod Studio](../../apps/mod-studio/README.md) |
| install and order **many** mods at once | [Mod Manager](../../apps/mod-manager/README.md) or [`gore mgr`](mod-manager.md) |
| edit your saved progress | [Save Editor](../../apps/save-editor/README.md) |
| hand-write custom Lua behavior | [gore-lua](../../lua/README.md) |

The Flutter GUIs call the exact same Rust crates as the CLI through a
`dart:ffi` bridge. The CLI is always the most complete surface.

## How each domain becomes a mod

Every domain produces a mod a different way, and each one is usable on its own:

| Domain | Mechanism | Touches | Guide |
|--------|-----------|---------|-------|
| Item/stat values | UE4SS Lua CDO override, applied at runtime | a new mod folder under `ue4ss\Mods\` | [items.md](items.md) |
| Text & dialogs | re-encrypted `.lcache` | the localization cache, in place | [text-and-dialogs.md](text-and-dialogs.md) |
| Audio | re-packed FMOD `.bank` | the sound bank, in place | [audio.md](audio.md) |
| Voice-over | copy-on-write localized ZIP edit | the selected language archive, in place | [voice.md](voice.md) |
| Textures | additive UE5 IoStore Zen triplet | the game's `~mods\` folder | [textures.md](textures.md) |
| Cooked DataAssets | additive Zen triplet from a fixed-leaf patch | the game's `~mods\` folder | [dataassets.md](dataassets.md) |
| Scripts | edited precompiled AngelScript cache | the script cache (experimental) | [scripts.md](scripts.md) |

You can ship each on its own, or combine them into one deployable **bundle** —
see [Bundling & deploying](bundles.md).

## Backups and safety

- In-place edits (localization, audio, script cache) write a `*.gore-bak`
  backup of the original file first.
- Texture and DataAsset mods are **additive**: they drop a container into
  `~mods\` and never modify an original game file.
- `gore mod undeploy` restores everything a bundle changed;
  `gore mgr reset` does the same for the whole managed loadout.
- Voice-over, texture pack, and DataAsset commands never modify their input and
  refuse to overwrite an existing output path.
- Close the game before any command that writes into the install. Script
  compilation and deployment take an install-wide lock
  (`.gore-install-mutation.lock`) so two GORE processes cannot fight, but the
  game itself does not participate in that lock.

## A first mod

Make apples worth 500 gold. Save this as `overrides.toml`:

```toml
[meta]
name = "MyBalanceMod"

[[override]]
class = "ItFo_Apple"
field = "m_Value"
value_int = 500
```

Then compile it into the game's UE4SS mods folder:

```powershell
gore gen overrides.toml -o "$GAME\G1R\Binaries\Win64\ue4ss\Mods"
```

Start the game with UE4SS enabled and the value is applied at load. Details and
the full override format: [Item & stat values](items.md).

## Next steps

- [Item & stat values](items.md)
- [Text & dialogs](text-and-dialogs.md)
- [Bundling & deploying](bundles.md)
- [CLI reference](cli-reference.md) — every command and flag
