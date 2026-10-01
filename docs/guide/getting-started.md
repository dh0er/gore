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
`gore.exe`, its required `compiler\` tree, and this whole guide under
`docs\` — so the compiler and documentation are available offline, right next
to the binary. Unpack the complete zip into a stable directory such as
`C:\Tools\gore-cli`, add that directory to `PATH`, and keep all companion files
together when installing or updating.

To read it offline, open `docs\guide.html`: one browsable file with every page,
a collapsible sidebar and a filter box. The `docs\*.md` files next to it are the
same content in Markdown, for `grep`. The [MCP server](mcp.md) serves that guide
too, but from a copy compiled into `gore.exe` rather than from these files —
editing them changes what you read, not what an assistant is told. You can
regenerate the HTML at any time with `gore guide html`.

**Build it yourself.** Requires Python 3, a stable Rust toolchain, and the
Visual Studio C++ tools; see [Building](../development.md) for the full
toolchain requirements. From the repository root:

```powershell
python build.py gore-cli dist             # recommended installable zip → dist\gore-cli\
python build.py gore-cli build --release  # CLI + compiler → target\release\
```

For installation, unpack the complete zip as described above; `dist` includes
the compiler and offline guide. Keep the complete output together, including
`compiler\` and `docs\`, rather than copying only `gore.exe`.

Normal CLI builds require a nonempty embedded compiler catalog prepared by
`build.py`. Raw `cargo build -p gore` without it fails. The explicit debug-only
`development-cli` feature is for development and tests; see
[Building](../development.md#the-rust-workspace). It cannot bypass the catalog
requirement in release builds.

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

## Check the setup

```powershell
gore doctor
```

One read-only pass over everything the rest of this guide assumes. One line
each: where the install is and which of the three sources above
answered, whether that folder holds Gothic 1 Remake, what is deployed, whether the `~mods`
override folder is there, what an interrupted run left behind, whether the
executable is running, whether the authenticated standalone AngelScript
compiler matches the installed cache/API, and whether the shared localized-text
catalog still describes the installed `.lcache`.

Every `problem` carries a `fix:` line. A `note` is informational; a `skipped`
check can point to the earlier missing prerequisite instead. Abridged example:

```
ok      game path     D:\SteamLibrary\steamapps\common\Gothic 1 Remake (source: config)
ok      deployment    nothing is deployed (no deploy record in the install)
ok      AS standalone authenticated standalone compiler is compatible with this cache/API; native diagnostics are available without a game launch
problem loc catalog   43851 ids in 19 language(s), but stale: extracted from 37081808 bytes and the installed cache is now 37093440
                      fix: … Run 'gore loc extract' so the shared catalog describes the file that is actually installed

check(s): the counts on the last line say how many of each verdict you have
```

| Verdict | Meaning |
|---|---|
| `ok` | checked, nothing to do |
| `note` | worth knowing, not a fault — the executable is running, the catalog was extracted elsewhere |
| `problem` | a reason a mod would silently do nothing, or a mess somebody has to clean up |
| `skipped` | not answerable, because something this check reads is absent; whatever reported that absence is on a line above |

**Exit code 0 either way.** A finding is not the command failing, and this is the
command you reach for once something has already gone wrong — no wrapper should
read its exit code as "this is broken too". To act on the findings from a
script, use `gore doctor --json`: each check carries a stable `id` and its own
`verdict`, and the top level carries `ok` / `note` / `problem` / `skipped`
counts.

Nothing here writes, creates or removes anything. The `deployment` check hashes
the files the deploy record claims, exactly as `gore mgr status` does. The
standalone-compiler check separately authenticates its package and verifies that
the installed compiler inputs match a qualified cache/API. It also reports
`native_api: ready` or `native_api: missing` for extended native reference
authority. Missing authority is an actionable problem even when the compiler
itself is compatible; ordinary references already present in the pristine
cache may still work.

What each check reads — and therefore what it can and cannot prove — is in the
[CLI reference](cli-reference.md#doctor).

## Which tool for which job

| You want to… | Use |
|---|---|
| change values, text, audio, textures or scripts of the game | the `gore` CLI (this guide) |
| do the same without a terminal, for one mod | [Mod Studio](mod-studio.md) |
| install and order **many** mods at once | [Mod Manager](../../apps/mod-manager/README.md) or [`gore mgr`](mod-manager.md) |
| edit your saved progress | [Save Editor](../../apps/save-editor/README.md) |

The Flutter GUIs call the same Rust engine as the CLI through a `dart:ffi`
bridge. Use the CLI for expert and automated workflows; use a GUI when its
guided presentation is more useful. They share contracts and state rather than
maintaining separate implementations.

## How each domain becomes a mod

Every domain produces a mod a different way, and each one is usable on its own:

| Domain | Mechanism | Touches | Guide |
|--------|-----------|---------|-------|
| Item/stat values | rewritten class `default`, compiled into the script cache | the Shipping script cache | [items.md](items.md) |
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

## A first mod: apple prices

Change the default value of an apple with a small native script bundle. This
uses the same class-default builder as larger value mods and needs no UE4SS.
The build and inspection steps write only your work and output directories.

### 1. Inspect the target

```powershell
gore value inspect --class UItFo_Apple --game "$GAME"
```

Find `m_Value` in the recovered defaults. An unknown class, an unsupported
field or an incompatible compiler fails with a reason; resolve that before
building. [Item & stat values](items.md) explains the supported types and how
class defaults affect existing saves.

### 2. Build the bundle

Save this as `first-mod.spec.json`:

```json
{
  "meta": { "name": "MyFirstMod", "version": "0.1.0", "author": "" },
  "values": [
    { "class": "UItFo_Apple", "field": "m_Value", "value": { "int": 500 } }
  ]
}
```

```powershell
gore mod build --spec first-mod.spec.json --game "$GAME" `
  --work-dir work/first-mod -o build
gore mod inspect build\MyFirstMod
```

The bundle is `build\MyFirstMod`. Keep it outside the game installation;
`build` compiles the changed module with the standalone compiler and does not
launch the game.

### 3. Apply, observe, remove

Close the game, then deploy the inspected bundle:

```powershell
gore mod deploy --bundle build\MyFirstMod --game "$GAME"
gore doctor --game "$GAME"
```

Start the game yourself and compare apple prices at a trader. The class's
`m_Value` is now 500; buy and sell prices also use the trader's multipliers, so
they need not display 500. This edit does not change saved inventory counts.

Close the game again and remove the direct deployment:

```powershell
gore mod undeploy --game "$GAME"
gore doctor --game "$GAME"
```

The second check should show no direct deployment and restored original files.
If you already use a Manager loadout, import the bundle and use that loadout's
Apply/Reset workflow instead; see [Running many mods](mod-manager.md).

For a new NPC with dialogs or a quest, continue with
[Characters](npc-authoring.md) and the shipped source examples. A new character's
private conversation is authored in its source module. `dialog new-conversation`
reads a pristine cache's already-loaded settings anchor; it does not read an
uncompiled `npc new` workspace.

## Next steps

- [Characters](npc-authoring.md)
- [Dialog authoring](dialog-authoring.md)
- [Text & dialogs](text-and-dialogs.md)
- [Bundling & deploying](bundles.md)
- [CLI reference](cli-reference.md)
