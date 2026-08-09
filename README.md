# GORE

**GORE** (Go-thic Re-make) is a vibe-coded modding and save-editing toolsuite for Gothic 1 Remake. One Rust engine, one CLI, and three Windows apps built on top of it.

[<img src="docs/images/screenshot_dark.png" alt="GORE Save Editor" width="600"/>](docs/images/screenshot_light.png)

## Tools

| Tool | What it does | Status |
|---|---|---|
| **[GORE CLI](docs/guide/README.md)** | All modding from the terminal: item values, text and dialogs, audio, voice, textures, DataAssets, scripts. Start here. | ⚗️ Experimental use |
| **[Mod Studio](apps/mod-studio/README.md)** | No-code Windows GUI over the same engine, for *authoring* one mod. | 🚧 Work in progress |
| **[Mod Manager](apps/mod-manager/README.md)** | Windows GUI for installing and ordering *many* mods together. | 🚧 Work in progress |
| **[Save Editor](apps/save-editor/README.md)** | Windows GUI for editing your save files. Never touches the game install. | ✅ Ready to use |
| **[gore-lua](lua/README.md)** | Small Lua helper library that ships into the game, for hand-written UE4SS mods. | 🚧 Work in progress |
| **[Assistant plugin](plugins/gore/README.md)** | The MCP server and the modding skill, installed into Claude Code, Codex or Cursor in one step. | ⚗️ Experimental use |

The Flutter GUIs reuse the exact same Rust crates as the CLI through a
`dart:ffi` bridge — the CLI is always the most complete surface.

## What you can change, and what does not work yet

Each row links the page that carries the evidence — build ids, what was seen,
and the exact wording. "Works" here means somebody watched it happen in the
running game; where that is not so, the right-hand column says it.

| Area | What you can do | Not yet |
|---|---|---|
| [Item & stat values](docs/guide/items.md) | Change any class default the game reads at startup — item value and weight, weapon damage, NPC stats — from a small `overrides.toml`. Ships as a UE4SS Lua mod. | Needs [UE4SS](docs/guide/getting-started.md); without it the mod sits there and does nothing. GORE does not install it. |
| [Text & dialogs](docs/guide/text-and-dialogs.md) | Replace **any** text in the game: all 43,851 ids across 19 languages — dialog lines, item names, journal entries, UI. New ids can be added. | An id often carries several language generations, and the game reads the newest. Write the wrong one and the file changes while the screen does not — `gore mod deploy` warns, `gore loc import` does not. |
| [Audio](docs/guide/audio.md) | Replace music and sound effects inside the encrypted FMOD banks, at any length — the replacement does not have to match the original's duration. | Which surface plays which sample is documented nowhere — you pick by listening. Your WAV is re-encoded to PCM16, so the bank grows by the audio you add. |
| [Voice-over](docs/guide/voice.md) | Replace any existing recording in a language archive, copy-on-write — the original is never modified. | Adding a **new** voice path is written and validated, but nothing has been heard playing one — a new line needs a new dialog to speak it. Encode Vorbis: Opus passes the CLI and is then refused by Mod Studio's build, and no Opus take has been heard in game. |
| [Textures](docs/guide/textures.md) | Replace game textures. Ships additively as a UE5 IoStore triplet in `~mods\`, so no game file is touched. | Anything living in `G1R-Windows.pak` — the mouse cursors, `DefaultEngine.ini` — is not reachable this way and needs the pak route instead. |
| [DataAssets](docs/guide/dataassets.md) | Edit cooked DataAsset values byte-exactly, receipt-sealed, also additively. | A Blueprint-generated export (class name ending `_C`) has no schema and cannot be bound — `inspect` refuses it rather than guessing. |
| [Scripts (AngelScript)](docs/guide/scripts.md) | Decompile the shipping cache to readable AngelScript, edit it, and compile with the game's own executable. Add whole new modules, and splice one module back into the vanilla cache. A new dialog option authored this way has been seen in a real conversation. | Decompilation is lossy: it is reverse-engineering-stage tooling, some helpers do not round-trip, and `emit-all` does not emit generated `__InitDefaults` as editable source. `gore as bytediff` exists to measure what a recompile changed. Do not ship a whole regenerated cache. What happens when a player *clicks* an authored option is untested. |
| [Bundling & deploying](docs/guide/bundles.md) · [many mods](docs/guide/mod-manager.md) | Put all of the above in one bundle that deploys and undeploys as a unit, transactionally. `gore mgr` runs several together with load order and a conflict report, and imports **foreign** mods too: zips and folders, loose `_P.pak`, IoStore triplets, UE4SS Lua mod folders, raw file replacements. | The foreign path has been walked end to end once, with one triplet. |

Offline and needing no game running: [`doctor`](docs/guide/getting-started.md)
(diagnose the setup), [`find`](docs/guide/find.md) (search the catalogs and the
effect register), `location`, and the catalog builders.

Two things GORE will not do at all: **edit your saves** — that is the
[Save Editor](apps/save-editor/README.md), and the CLI does not even link the
save library — and **install UE4SS** for you. And everything above was seen by
one person, on one install, on the builds those pages name.

## Quick start

Get `gore.exe` from a `gore-cli-v*`
[release](https://github.com/dh0er/gore/releases), or build it:

```powershell
cargo build --release -p gore     # → target\release\gore.exe
```

Point it at your game once:

```powershell
$GAME = 'D:\SteamLibrary\steamapps\common\Gothic 1 Remake'
gore config set game-path $GAME     # or: gore config detect
```

Then make apples worth 500 gold. Save this as `overrides.toml`:

```toml
[meta]
name = "MyBalanceMod"

[[override]]
class = "ItFo_Apple"
field = "m_Value"
value_int = 500
```

```powershell
gore gen overrides.toml -o "$GAME\G1R\Binaries\Win64\ue4ss\Mods"
```

Full walkthrough: [Getting started](docs/guide/getting-started.md).

## Vibe Modding

You can mod with AI agents by installing the plugin, or by manually installing the skill and mcp tools.

### Claude plugin

```powershell
claude plugin marketplace add dh0er/gore
claude plugin install gore@gore
```

### Codex plugin

```powershell
codex plugin marketplace add C:\path\to\gore
codex plugin add gore@gore
```

### Manual installation (all clients)

Add the MCP server to the client configuration:

```json
{
  "mcpServers": {
    "gore": {
      "command": "gore",
      "args": ["mcp", "serve"]
    }
  }
}
```

Link the `gore-modding` skill from a checkout into the agent's personal skills
directory:

```powershell
New-Item -ItemType Junction `
  -Path <agent-skills-directory>\gore-modding `
  -Target C:\path\to\gore\plugins\gore\skills\gore-modding
```

`gore.exe` must be on `PATH`; check with `gore --version`.

For unattended use, add `--allow-write` and/or `--allow-game-launch` to
`gore mcp serve` to pre-approve writes or game launches. Compiling requires both.

## Documentation

Everything lives in [`docs/`](docs/README.md).

| | |
|---|---|
| [Getting started](docs/guide/getting-started.md) | Install, configure, first mod, which tool for which job |
| [Item & stat values](docs/guide/items.md) | `overrides.toml` → UE4SS Lua CDO override mod |
| [Text & dialogs](docs/guide/text-and-dialogs.md) | Decrypt, edit, re-encrypt the localization `.lcache` |
| [Audio](docs/guide/audio.md) · [Voice-over](docs/guide/voice.md) | FMOD bank samples; voice-over ZIP archives |
| [Textures](docs/guide/textures.md) · [DataAssets](docs/guide/dataassets.md) | Additive UE5 IoStore Zen triplets |
| [Scripts](docs/guide/scripts.md) | Decompile, recompile, and splice the AngelScript cache |
| [Bundling & deploying](docs/guide/bundles.md) | One spec → one mod that deploys as a unit |
| [Running many mods](docs/guide/mod-manager.md) | `gore mgr`: library, load order, conflicts |
| [CLI reference](docs/guide/cli-reference.md) | Every command, subcommand, and flag |
| [AI assistants](docs/guide/mcp.md) | Install the plugin, or wire the MCP server up by hand; what gets confirmed with you |
| [Mod Studio](docs/guide/mod-studio.md) | The no-code GUI: NPCs, quests, voice, project backups |
| [Building](docs/development.md) | Toolchain, `build.py`, repo layout, crates, versioning |

The CLI release zip carries the same guide offline: `docs\guide.html` is one
browsable file with a collapsible sidebar, and `docs\*.md` is the same content in
Markdown, for `grep`. The MCP server answers from its own copy, compiled into
`gore.exe`, so editing those files changes what you read and not what an
assistant is told. Regenerate the HTML any time with `gore guide html`.

Implementation contracts behind the commands — receipt semantics, seal
guarantees, why a patch is refused — live separately in
[`docs/reference/`](docs/reference/README.md). They are not part of the guide.

## Build

Requires Windows 10+, a stable Rust toolchain, Python 3, Visual Studio 2022
with "Desktop development with C++", and — for the GUI apps — Flutter with
Windows desktop support.

```powershell
cargo build
cargo test
```

Shippable products are driven by the top-level orchestrator. Registered
projects: `gore-cli`, `gore-save-editor`, `gore-mod-studio`, `gore-mod-manager`.
A project name is also its release-tag prefix and its artifact name.

```powershell
python build.py <project> build|run|dist|installer|test
python build.py all test
```

Release tags and manual smoke builds run the [same CI quality gates](docs/development.md#release-quality-gates)
on the exact commit before any product build.

Details, repo layout, and the crate table: [Building](docs/development.md).

## License

MIT. See [LICENSE](LICENSE).
