# ⚔️ GORE

**GORE** (Go-thic Re-make) is a vibe-coded modding and save-editing toolsuite for Gothic 1 Remake. One Rust engine, one CLI, and three Windows apps built on top of it.

[<img src="docs/images/screenshot_dark.png" alt="GORE Save Editor" width="600"/>](docs/images/screenshot_light.png)

## 🧰 Tools

| <div style="width:150px">Tool</div> | What it does | <div style="width:150px">Status</div> |
|---|---|---|
| **[CLI](docs/guide/README.md)** | All modding from the terminal: item values, text and dialogs, audio, voice, textures, DataAssets, scripts. Start here. | ⚗️ Experimental use |
| **[Mod Studio](apps/mod-studio/README.md)** | No-code Windows GUI over the same engine, for *authoring* one mod. | 🚧 Work in progress |
| **[Mod Manager](apps/mod-manager/README.md)** | Windows GUI for installing and ordering *many* mods together. | ⚗️ Experimental use |
| **[Save Editor](apps/save-editor/README.md)** | Windows GUI for editing your save files. Never touches the game install. | ✅ Ready to use |
| **[gore-lua](lua/README.md)** | Small Lua helper library that ships into the game, for hand-written UE4SS mods. | 🚧 Work in progress |
| **[Assistant plugin](plugins/gore/README.md)** | The MCP server and the modding skill, installed into Claude Code, Codex or Cursor in one step. | ⚗️ Experimental use |

The Flutter GUIs reuse the same Rust engine as the CLI through a `dart:ffi`
bridge. The CLI is the expert and automation surface; the GUIs package those
contracts into guided workflows instead of maintaining a second engine.

## ✅ Compatibility

**Save Editor** is not tied to a specific game version. It edits the save file
itself, never the game installation, and is designed to preserve data it does
not understand. Its core save editing is stable across game versions, although
a future patch can add or change save fields or make the bundled item and
location catalogs stale. Keep backups of important saves.

The **CLI** has been tested with Gothic 1 Remake **1.0.5 (Steam BuildID
24878692)**; **Mod Manager** has been tested with **1.0.4a (CL171864)**.
Neither uses a simple version-number lock. Mod Manager
compatibility also depends on the individual mod and whether its target files,
localization IDs, assets, and script targets exist in the installed game.
Import validates the package, but cannot prove runtime compatibility; Apply
checks the current installation and reports missing or incompatible targets.

Many offline CLI commands are independent of the installed game version, while
commands that read, build, or deploy game data depend on the relevant formats
and APIs. In particular, the bundled standalone AngelScript compiler checks the
installed Shipping cache format and complete ordered Binds API instead of the
displayed game version. If they do not match, strict standalone compilation
fails safely; the default `standalone-then-game` mode reports the reason and can
use the game's embedded compiler as a fallback.

## ⬇️ Downloads

| Tool | Version | Release page |
|---|---|---|
| **CLI** | 0.3.0 | [gore-cli-v0.3.0](https://github.com/dh0er/gore/releases/tag/gore-cli-v0.3.0) |
| **Mod Manager** | 0.2.0 | [gore-mod-manager-v0.2.0](https://github.com/dh0er/gore/releases/tag/gore-mod-manager-v0.2.0) |
| **Save Editor** | 1.4.0 | [gore-save-editor-v1.4.0](https://github.com/dh0er/gore/releases/tag/gore-save-editor-v1.4.0) |

Every release page lists its own assets and changes. The full history is on the
[releases page](https://github.com/dh0er/gore/releases). Mod Studio has no
release yet — build it from source.

## 📊 Status

| Area | Status | What you can do | The catch |
|---|---|---|---|
| [Item & stat values](docs/guide/items.md) | Full | Change what items are worth, what weapons do, what NPCs have | Needs UE4SS, which GORE does not install |
| [Characters](docs/guide/npc-authoring.md) | Read and author | See which characters the game ships, the class chain one is made of, and which world points spawn it; add a new character to the world, change a shipped one's values, or stop one being placed | An authored character was built, deployed and seen in game: it stands at its world point, animates, can be focused and spoken to, and the save records it under its own identity. Not seen: a character following its routine, `--modular-visuals` reproducing the template's look (it renders, but as the player character), or `checkout` in game at all |
| [Text & dialogs](docs/guide/text-and-dialogs.md) | Full | Replace localized game text across all 19 catalog slots | Twelve slots are ordinary languages; German and English use multiple generations, while `foreign` and `stagedirections` are not languages |
| [Dialog authoring](docs/guide/dialog-authoring.md) | Mostly | Edit shipped topics and build new roots, submenus, multi-level trees and complete conversations with game effects | A first conversation needs an exact already-loaded per-NPC settings module; cross-module new-symbol dependencies need a separate selective complete-cache compile, not dialog minis |
| [Audio](docs/guide/audio.md) | Full | Replace music and sound effects | Finding which sound plays where is guesswork |
| [Voice-over](docs/guide/voice.md) | Mostly | Replace spoken lines and add voice to authored new lines | Publication requires Vorbis; a new member also needs matching script and localization, and receives generic lip movement rather than exact new lip sync |
| [Textures](docs/guide/textures.md) | Full | Replace textures | A few, like the mouse cursors, are stored somewhere this cannot reach |
| [DataAssets](docs/guide/dataassets.md) | Partly | Edit cooked game data | Only assets the engine describes natively; Blueprint ones are refused |
| [Scripts](docs/guide/scripts.md) | Mostly | Read the game's script code, change it, add your own | Splicing recompiles the WHOLE module, so it also rewrites functions you did not touch. 6,982 of 7,317 modules recompile with no known semantic difference; of the rest most only differ in wording, but 15 hold a loop that recompiles with a bound of zero and never runs — they sit in crime assessment, combat, fear and search AI. `emit` and `compile-module` name the module and the risk before you splice |
| [Mods & load order](docs/guide/bundles.md) | Full | Ship all of the above as one mod, run many together, and install mods that GORE did not build — plain zips, pak files, UE4SS mod folders | Four Nexus mods have run through one real-install campaign; third-party AngelScript and three-way script conflicts remain unqualified |

GORE will not edit your saves — that is the
[Save Editor](apps/save-editor/README.md) — and everything above was seen by one
person, on one install.

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
| 🏁 [Getting started](docs/guide/getting-started.md) | Install, configure, first mod, which tool for which job |
| 🍎 [Item & stat values](docs/guide/items.md) | `overrides.toml` → UE4SS Lua CDO override mod |
| 🧍 [Characters](docs/guide/npc-authoring.md) | `gore npc`: which characters exist, what one is made of, where it spawns, and authoring a new one |
| 💬 [Text & dialogs](docs/guide/text-and-dialogs.md) | Decrypt, edit, re-encrypt the localization `.lcache` |
| 🌳 [Dialog trees](docs/guide/dialog-trees.md) · ✍️ [Dialog authoring](docs/guide/dialog-authoring.md) | Inspect conversations; edit defaults and behavior; add roots, submenus, multi-level trees and complete conversations |
| 🔊 [Audio](docs/guide/audio.md) · 🎙️ [Voice-over](docs/guide/voice.md) | FMOD bank samples; voice-over ZIP archives |
| 🖼️ [Textures](docs/guide/textures.md) · 📦 [DataAssets](docs/guide/dataassets.md) | Additive UE5 IoStore Zen triplets |
| 📜 [Scripts](docs/guide/scripts.md) | Decompile, recompile, and splice the AngelScript cache |
| 📦 [Bundling & deploying](docs/guide/bundles.md) | One spec → one mod that deploys as a unit |
| 🧩 [Running many mods](docs/guide/mod-manager.md) | `gore mgr`: library, load order, conflict evidence, preflight/recovery, Apply and Reset |
| ⌨️ [CLI reference](docs/guide/cli-reference.md) | Every command, subcommand, and flag |
| 🤖 [AI assistants](docs/guide/mcp.md) | Install the plugin, or wire the MCP server up by hand; what gets confirmed with you |
| 🖥️ [Mod Studio](docs/guide/mod-studio.md) | The no-code GUI: NPCs, quests, voice, project backups |
| 🔧 [Building](docs/development.md) | Toolchain, `build.py`, repo layout, crates, versioning |

The CLI release zip carries the same guide offline: `docs\guide.html` is one
browsable file with a collapsible sidebar, and `docs\*.md` is the same content in
Markdown, for `grep`. The MCP server answers from its own copy, compiled into
`gore.exe`, so editing those files changes what you read and not what an
assistant is told. Regenerate the HTML any time with `gore guide html`.

Implementation contracts behind the commands — receipt semantics, seal
guarantees, why a patch is refused — live separately in
[`docs/reference/`](docs/reference/README.md). They are not part of the guide.

## 🔨 Build

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

## 📄 License

MIT. See [LICENSE](LICENSE).
