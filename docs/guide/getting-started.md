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

To read it offline, open `docs\guide.html`: one browsable file with every page,
a collapsible sidebar and a filter box. The `docs\*.md` files next to it are the
same content in Markdown, for `grep`. The [MCP server](mcp.md) serves that guide
too, but from a copy compiled into `gore.exe` rather than from these files —
editing them changes what you read, not what an assistant is told. You can
regenerate the HTML at any time with `gore guide html`.

**Build it yourself.** Requires a stable Rust toolchain:

```powershell
cargo build --release -p gore
# → target\release\gore.exe
```

See [Building](../development.md) for the full toolchain requirements and the
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

## Check the setup

```powershell
gore doctor
```

One read-only pass over everything the rest of this guide assumes. Ten checks,
one line each: where the install is and which of the three sources above
answered, whether that folder holds Gothic 1 Remake, whether [UE4SS](#ue4ss) is
present, which UE4SS mods are enabled, what is deployed, what an interrupted run
left behind, whether the executable is running, whether the authenticated
standalone AngelScript compiler matches the installed cache/API, and whether
the shared localized-text catalog still describes the installed `.lcache`.

Every `problem` carries a `fix:` line. A `note` is informational; a `skipped`
check can point to the earlier missing prerequisite instead. Abridged example:

```
ok      game path     D:\SteamLibrary\steamapps\common\Gothic 1 Remake (source: config)
ok      UE4SS         installed at …\G1R\Binaries\Win64\ue4ss
ok      UE4SS mods    22 mod folder(s), 7 enabled
ok      deployment    nothing is deployed (no deploy record in the install)
ok      AS standalone authenticated standalone compiler is compatible with this cache/API; native diagnostics are available without a game launch
problem loc catalog   43851 ids in 19 language(s), but stale: extracted from 37081808 bytes and the installed cache is now 37093440
                      fix: … Run 'gore loc extract' so the shared catalog describes the file that is actually installed

10 check(s): 9 ok, 0 note, 1 problem, 0 skipped
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
the installed compiler inputs match a qualified cache/API.

What each check reads — and therefore what it can and cannot prove — is in the
[CLI reference](cli-reference.md#doctor).

## Which tool for which job

| You want to… | Use |
|---|---|
| change values, text, audio, textures or scripts of the game | the `gore` CLI (this guide) |
| do the same without a terminal, for one mod | [Mod Studio](mod-studio.md) |
| install and order **many** mods at once | [Mod Manager](../../apps/mod-manager/README.md) or [`gore mgr`](mod-manager.md) |
| edit your saved progress | [Save Editor](../../apps/save-editor/README.md) |
| hand-write custom Lua behavior | [gore-lua](../../lua/README.md) |

The Flutter GUIs call the same Rust engine as the CLI through a `dart:ffi`
bridge. Use the CLI for expert and automated workflows; use a GUI when its
guided presentation is more useful. They share contracts and state rather than
maintaining separate implementations.

## How each domain becomes a mod

Every domain produces a mod a different way, and each one is usable on its own:

| Domain | Mechanism | Touches | Guide |
|--------|-----------|---------|-------|
| Item/stat values | [UE4SS](#ue4ss) Lua CDO override, applied at runtime | a new mod folder under `ue4ss\Mods\` | [items.md](items.md) |
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

## UE4SS

Of the domains in the table above, item and stat values are the only one applied
while the game runs: instead of changing a file the game loads, GORE emits a
small Lua mod that sets the value in memory. Something has to run that Lua, and
that something is UE4SS — a third-party loader that attaches to the running game
and executes the Lua mods it finds in a `Mods\` folder. It is a separate
community project, not part of Gothic 1 Remake and not part of GORE.
Hand-written [gore-lua](../../lua/README.md) mods run the same way; the other
rows of the table above never involve it.

GORE does not install it, and no command checks whether it is there. `gore gen`
and `gore mod build` produce a well-formed mod either way, and `gore mod deploy`
creates `ue4ss\Mods\` itself when it is missing — so a deploy that reports
success means the files are in place, never that anything will run them.

`gore doctor` answers whether you have it — the `UE4SS` line — along with which
mods in it are enabled. To look for yourself:

```powershell
ls "$GAME\G1R\Binaries\Win64\ue4ss"
```

An install has `UE4SS.dll`, `UE4SS-settings.ini` and a `Mods\` directory sitting
beside each other. If the `ue4ss` folder is not there at all, you do not have it,
and an override mod will sit in the install doing nothing, with nothing on either
side reporting a problem.

**Where to get it.** UE4SS is [UE4SS-RE/RE-UE4SS](https://github.com/UE4SS-RE/RE-UE4SS)
on GitHub (MIT). GORE neither ships nor installs it.

Read the release list before you download, because the newest *tagged* release
is not the newest build:

| Channel | What it is |
|---|---|
| `v3.0.1` | the latest stable tag — published **February 2024**, which predates this game |
| `experimental-latest` | a rolling prerelease, rebuilt continuously from `main` |

Everything in this guide was checked against an **experimental** build, the one
this machine runs: `v3.0.1 Beta #0`, git `272ce2f8` (7 June 2026). Note the
version string — experimental assets are still named `UE4SS_v3.0.1-<n>-g<sha>.zip`,
so "v3.0.1" alone does not tell you which of the two you have. The git SHA does.

To see what you have, read the second line of `UE4SS.log`:

```
[…] UE4SS - v3.0.1 Beta #0 - Git SHA #272ce2f8
```

Nothing here has been tested against the 2024 stable tag.

Inside `Mods\`, each mod is one folder holding `Scripts\main.lua` and an empty
`enabled.txt`. That empty file is the switch — UE4SS loads a folder because
`enabled.txt` is present. `gore gen` and `gore scaffold` write both for you.

**Confirming an override applied.** UE4SS writes a log next to itself,
`G1R\Binaries\Win64\ue4ss\UE4SS.log`, and a generated override mod prints one
line there for every override it applies:

```
[<timestamp>] [Lua] [MyBalanceMod] ItFo_Apple.m_Value 10 -> 500
```

That line is the only machine-readable evidence that the change took effect, and
it is worth reading before you conclude a mod failed: it separates "nothing
happened" from "something happened and you were looking at the wrong thing".

Do not judge it in the first seconds. The class defaults an override targets do
not exist yet when the mod starts, so the generated Lua polls for them — every
1000 ms, up to 120 attempts. In the one run that was measured, the line appeared
about four seconds after the mod started, after several retries. If a class never
appears at all, the log says that instead:

```
[<timestamp>] [Lua] [MyBalanceMod] gave up after 120 attempts; 1 CDO(s) never appeared
```

## A first mod

Make apples worth 500 gold. This is an override, so it needs UE4SS in the game
install — see [UE4SS](#ue4ss) above if you have not checked. Save this as
`overrides.toml`:

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

Start the game. The value is applied a few seconds in rather than at the moment
of launch, and `UE4SS.log` is where you see it happen — see [UE4SS](#ue4ss)
above. Details and the full override format: [Item & stat values](items.md).

If apples still cost what they did, run [`gore doctor`](#check-the-setup) before
anything else. Nothing in the build or the deploy would have told you that UE4SS
is missing, that another enabled mod is setting the same value, or that the
folder has no `enabled.txt`; that one command checks all three.

## Next steps

- [Item & stat values](items.md)
- [Text & dialogs](text-and-dialogs.md)
- [Bundling & deploying](bundles.md)
- [CLI reference](cli-reference.md) — every command and flag
