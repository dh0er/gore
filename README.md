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

## What GORE can change, and what it cannot

Two different things are worth keeping apart, because "the tests pass" and
"somebody saw it in the game" are not the same claim. Every row below is
sourced from the linked page, which carries the exact build ids and wording.

| Area | What it changes | Has it been seen in the game? |
|---|---|---|
| [Item & stat values](docs/guide/items.md) | class defaults, as a UE4SS Lua mod | **Yes** — the running game writes one `UE4SS.log` line per applied override |
| [Text & dialogs](docs/guide/text-and-dialogs.md) | the encrypted `.lcache`, in place | **Yes** — on BuildID 24539464, in both directions |
| [Audio](docs/guide/audio.md) | samples inside FMOD `.bank` files | **Yes** — five `SFX.bank` samples and the menu music, heard on 24539464 |
| [Voice-over](docs/guide/voice.md) | `replace` a recording in a language ZIP | **Yes** — two Diego lines, heard on 24539464 |
| [Textures](docs/guide/textures.md) | additive UE5 IoStore triplet in `~mods\` | **Yes** — the main-menu logo, on 24539464, and it survived a game update |
| [DataAssets](docs/guide/dataassets.md) | cooked DataAsset leaves, receipt-sealed | **Yes** — two edits watched on 2026-08-07 |
| [Dialog topics](docs/guide/dialog-authoring.md) | a compiled AngelScript topic in a conversation | **Once**, on game 1.0.3 — with named gaps; read the status table on that page first |
| [Bundling & deploying](docs/guide/bundles.md) | all of the above as one transactional unit | **Yes** — undeploy restored the 123 MB script cache to its recorded SHA-256, 92 of 93 saves byte-identical |

Offline and needing no game running: [`doctor`](docs/guide/getting-started.md)
(diagnose the setup), [`find`](docs/guide/find.md) (search the catalogs and the
effect register), `location`, and the catalog builders.

**What it cannot do:**

- **Blueprint-generated exports.** A class whose name ends in `_C` has no schema
  in the USMAP and cannot be bound — `inspect` refuses it rather than guessing.
- **Voice on a new dialog line.** `voice add` writes a valid archive member, but
  nothing plays a brand-new voice path until an authored topic speaks it, and
  recorded voice on an authored topic is exactly what the dialog proof does not
  certify. Replacing an existing recording is the path with evidence behind it.
- **Edit your saves.** That is the [Save Editor](apps/save-editor/README.md).
  The CLI has no save command and does not even link the save library.
- **Install UE4SS.** Item and stat overrides do nothing without it, and GORE
  neither ships nor installs it — `gore doctor` only tells you whether it is
  there. See the UE4SS section of [Getting started](docs/guide/getting-started.md).
- **Promise any of this on a build nobody has run it on.** The observations
  above are one person, one install, and the build ids they name.

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
