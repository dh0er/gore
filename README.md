# ⚔️ GORE

**GORE** (Go-thic Re-make) is a vibe-coded modding and save-editing toolsuite for Gothic 1 Remake. One Rust engine, one CLI, and three Windows apps built on top of it.

## 🧰 Tools

| Tool⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀ | What it does | Status⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀ | Download |
|---|---|---|---|
| **[Save Editor](apps/save-editor/README.md)** | Windows GUI for editing savegames. | ✅ Ready to use | [1.4.1](https://github.com/dh0er/gore/releases/tag/gore-save-editor-v1.4.1) |
| **[CLI](docs/guide/README.md)** | All-in-one command-line tool for all modding tasks. | ⚗️ Experimental use | [0.3.0](https://github.com/dh0er/gore/releases/tag/gore-cli-v0.3.0) |
| **[Mod Manager](apps/mod-manager/README.md)** | Windows GUI for installing and ordering *many* mods together. | ⚗️ Experimental use | [0.2.0](https://github.com/dh0er/gore/releases/tag/gore-mod-manager-v0.2.0) |
| **[AI Plugins](plugins/gore/README.md)** | MCP server and skill for the CLI. | ⚗️ Experimental use | ⠀⠀⠀⠀⠀ |
| **[Mod Studio](apps/mod-studio/README.md)** | No-code Windows GUI over the GORE engine, for *authoring* mods. | 📋 Planned | ⠀⠀⠀⠀⠀ |

## 📊 Status

| Area | Status | What you can do | What's missing |
|---|---|---|---|
| [Savegames](apps/save-editor/README.md) | Mostly | Edit Player and NPC values, inventories, quests and much more | Armor upgrades, chest and corpse loot, other loot points |
| [Item & stat values](docs/guide/items.md) | Partly | Change what items are worth, what weapons do, what NPCs have | Still needs UE4SS; [native implementation plan](docs/items-values-without-ue4ss-plan.md) |
| [Text & dialogs](docs/guide/text-and-dialogs.md) | Full | Replace all localized game text | ⠀⠀⠀⠀⠀ |
| [Dialog authoring](docs/guide/dialog-authoring.md) | Full | Edit shipped topics and build new roots, submenus, multi-level trees and complete conversations with game effects | ⠀⠀⠀⠀⠀ |
| [Audio](docs/guide/audio.md) | Full | Replace music and sound effects | ⠀⠀⠀⠀⠀ |
| [Voice](docs/guide/voice.md) | Mostly | Replace spoken lines and add voice to authored new lines | Other formats than Vorbis; Lip sync |
| [Textures](docs/guide/textures.md) | Mostly | Replace supported cooked `Texture2D` assets; bundle loose images and cursor PNGs | Some pixel formats and virtual-texture layouts cannot yet be rewritten |
| [DataAssets](docs/guide/dataassets.md) | Partly | Edit cooked game data | Only assets the engine describes natively; Blueprint ones are refused |
| [Scripts](docs/guide/scripts.md) | Full | Read the game's script code, change it, add your own | ⠀⠀⠀⠀⠀ |
| [Mod managing](docs/guide/bundles.md) | Mostly | Ship all of the above as one mod, run many together, install third-party mods — plain zips, pak files, UE4SS mod folders | Mods have not yet been tested after game patches |

## 📸 Screenshots
[<img src="docs/images/screenshot_dark.png" alt="GORE Save Editor" width="600"/>](docs/images/screenshot_light.png)

## ✅ Compatibility

**Save Editor** is compatible with any vanilla game version. It may also work for small mods, but there's no guarantee.

**CLI** has been tested with **1.0.5 Hotfix 1 (CL173255)**.
Many CLI commands are independent of the installed game version, while commands that read, build, or deploy game data depend on the relevant formats and APIs.

**Mod Manager** has been tested with **1.0.4a (CL171864)**.
Its compatibility depends on the individual mod and whether its target files, localization IDs, assets, and script targets exist in the installed game.

## 🚀 Quick start

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

Check what you have before you rely on it:

```powershell
gore doctor
```

It answers whether that path really is the game, whether UE4SS is there, what is
deployed, and what an interrupted run left behind. Every line that is not `ok`
carries a `fix:` line. Worth running now: the mod below is a UE4SS mod, and
without UE4SS it installs cleanly and then does nothing at all.

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

## 🤖 Vibe Modding

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
`gore mcp serve` to pre-approve writes or game launches. Explicit
`--backend standalone` compilation needs neither; `game`, the default
`standalone-then-game`, and an omitted backend need both because they may use the
game compiler.

## 📚 Documentation

Everything lives in [`docs/`](docs/README.md).

| | |
|---|---|
| [Getting started](docs/guide/getting-started.md) | Install, configure, first mod, which tool for which job |
| [Item & stat values](docs/guide/items.md) | `overrides.toml` → UE4SS Lua CDO override mod |
| [Text & dialogs](docs/guide/text-and-dialogs.md) | Decrypt, edit, re-encrypt the localization `.lcache` |
| [Dialog trees](docs/guide/dialog-trees.md) · [Dialog authoring](docs/guide/dialog-authoring.md) | Inspect conversations; edit defaults and behavior; add roots, submenus, multi-level trees and complete conversations |
| [Audio](docs/guide/audio.md) · [Voice-over](docs/guide/voice.md) | FMOD bank samples; voice-over ZIP archives |
| [Textures](docs/guide/textures.md) · [DataAssets](docs/guide/dataassets.md) | Additive UE5 IoStore Zen triplets |
| [Scripts](docs/guide/scripts.md) | Decompile, recompile, and splice the AngelScript cache |
| [Bundling & deploying](docs/guide/bundles.md) | One spec → one mod that deploys as a unit |
| [Running many mods](docs/guide/mod-manager.md) | `gore mgr`: library, load order, conflict evidence, preflight/recovery, Apply and Reset |
| [CLI reference](docs/guide/cli-reference.md) | Every command, subcommand, and flag |
| [AI assistants](docs/guide/mcp.md) | Install the plugin, or wire the MCP server up by hand; what gets confirmed with you |
| [Mod Studio](docs/guide/mod-studio.md) | The no-code GUI: NPCs, quests, voice, project backups |
| [Building](docs/development.md) | Toolchain, `build.py`, repo layout, crates, versioning |

The CLI release zip carries the same guide offline: `docs\guide.html` is one
browsable file with a collapsible sidebar, and `docs\*.md` is the same content in
Markdown, for `grep`. The MCP server answers from its own copy, compiled into
`gore.exe`, so editing those files changes what you read and not what an
assistant is told. Regenerate the HTML any time with `gore guide html`.

## 🔨 Build

```powershell
python build.py <project> build|run|dist|installer|test
python build.py all test
```

Registered projects: `gore-cli`, `gore-save-editor`, `gore-mod-studio`, `gore-mod-manager`.

Details, repo layout, and the crate table: [Building](docs/development.md).

## ⚖️ Game content and trademarks

GORE is an independent, unofficial modding toolkit. This repository and its
release packages do not include game binaries, assets, or decompiled game-script
output. Any required game data is read or generated locally from a game
installation supplied by the user.

Game names, trademarks, assets, and other game content remain the property of
their respective owners. GORE is not affiliated with or endorsed by the game's
developers or publishers.

## 📄 License

MIT. See [LICENSE](LICENSE).

The MIT License applies to original GORE source code. Third-party components
remain subject to their respective licenses; see
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
