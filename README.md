# GORE

**GORE** (Go-thic Re-make) is a vibe-coded modding and save-editing toolsuite for Gothic 1 Remake. One Rust engine, one CLI, and three Windows apps built on top of it.

[<img src="docs/images/screenshot_light.png" alt="GORE Save Editor" width="600"/>](docs/images/screenshot_light.png)

## Tools

| Tool | What it does | Status |
|---|---|---|
| **[`gore` CLI](docs/guide/README.md)** (`gore.exe`) | All modding from the terminal: item values, text and dialogs, audio, voice, textures, DataAssets, scripts. Start here. | ⚗️ Experimental use |
| **[Mod Studio](apps/mod-studio/README.md)** | No-code Windows GUI over the same engine, for *authoring* one mod. | 🚧 Work in progress |
| **[Mod Manager](apps/mod-manager/README.md)** | Windows GUI for installing and ordering *many* mods together. | 🚧 Work in progress |
| **[Save Editor](apps/save-editor/README.md)** | Windows GUI for editing your save files. Never touches the game install. | ✅ Ready to use |
| **[gore-lua](lua/README.md)** | Small Lua helper library that ships into the game, for hand-written UE4SS mods. | 🚧 Work in progress |

The Flutter GUIs reuse the exact same Rust crates as the CLI through a
`dart:ffi` bridge — the CLI is always the most complete surface.

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

## MCP server

`gore mcp serve` exposes the whole CLI over the
[Model Context Protocol](https://modelcontextprotocol.io), so an AI assistant can
drive GORE for you. It is part of `gore.exe` — nothing extra to install.

Claude Code:

```powershell
claude mcp add gore -- C:\path\to\gore.exe mcp serve
```

Claude Desktop, or any client with a JSON config:

```json
{
  "mcpServers": {
    "gore": {
      "command": "C:\\path\\to\\gore.exe",
      "args": ["mcp", "serve"]
    }
  }
}
```

Reading and creating new files works out of the box. Changing the game
installation needs `--allow-write`, and compiling AngelScript — which starts the
game — needs `--allow-game-launch`. Details: [MCP server](docs/guide/mcp.md).

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
| [MCP server](docs/guide/mcp.md) | Drive the whole CLI from an AI assistant |
| [Mod Studio](docs/guide/mod-studio.md) | The no-code GUI: NPCs, quests, voice, project backups |
| [Building](docs/development.md) | Toolchain, `build.py`, repo layout, crates, versioning |

The CLI release zip carries the same guide offline: `docs\guide.html` is one
browsable file with a collapsible sidebar, and `docs\*.md` is the Markdown the
MCP server serves. Regenerate the HTML any time with `gore guide html`.

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

Details, repo layout, and the crate table: [Building](docs/development.md).

## License

MIT. See [LICENSE](LICENSE).
