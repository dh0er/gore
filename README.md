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
| **[Assistant plugin](plugins/gore/README.md)** | The MCP server and the modding skill, installed into Claude Code, Codex or Cursor in one step. | ⚗️ Experimental use |

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

## Modding from an AI assistant

`gore mcp serve` exposes the whole CLI over the
[Model Context Protocol](https://modelcontextprotocol.io), so an assistant can
drive GORE for you. The server is part of `gore.exe`; what you install is the
**[plugin](plugins/gore/README.md)**, which registers it *and* adds the
`gore-modding` skill — the workflow around the tools, which the tools themselves
cannot carry. This repository is its own marketplace:

```powershell
claude plugin marketplace add dh0er/gore
claude plugin install gore@gore
```

In the Claude desktop app, the same without a terminal: the **+** beside the
prompt box, then **Plugins → Add plugin**.

One prerequisite the plugin cannot satisfy: it starts the server as `gore mcp
serve`, by name, so **`gore.exe` has to be on `PATH`**. If it is not, no `gore_*`
tool appears at all — `gore --version` in a terminal is the check.

To wire a client up by hand instead — a client with no plugin support, or one you
want to pass [flags](docs/guide/mcp.md#answering-in-advance) to:

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

Spell `command` as an absolute path to `gore.exe` if you would rather not touch
`PATH`.

Reading works out of the box, and so does writing to a free path outside the game
installation. What is **confirmed with you first** is changing something that is
already there: the installation itself, an output file that exists, a directory
that already holds files, an in-place rewrite. Aim a command somewhere new and it
runs without asking. A handful ask whatever is on disk, because what they change
is not a path you chose — the installation, the shared catalogs and library the
tools keep, or a target read out of a file the server does not open (`gen`,
`texture replace`, `loc extract`, `mgr import`, and the deploy/undeploy pairs).

Your client shows a dialog naming the command and the file, and nothing runs
unless you agree. Claude Code, Cursor and Codex can all show that dialog — but
only while they are interactive. Claude Code driven non-interactively answers for
you, and every such call is refused without you seeing anything.

Where no dialog reaches you, the assistant asks you in the chat instead: the
refusal shows it the command line to put in front of you, and your answer comes
back as `user_approved`. The result then says the command ran on that claim — the
server saw no confirmation of its own, so the transcript is what you check. See
[MCP server](docs/guide/mcp.md).

For unattended use, `--allow-write` and `--allow-game-launch` answer in advance,
and `--no-consent-prompts` refuses instead of asking. Those go in the `args`
above, which is why the hand-wired route is worth keeping: the plugin's own
`.mcp.json` starts the server without them. Details:
[MCP server](docs/guide/mcp.md).

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

Details, repo layout, and the crate table: [Building](docs/development.md).

## License

MIT. See [LICENSE](LICENSE).
