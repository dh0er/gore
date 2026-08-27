# GORE plugin

Registers the GORE MCP server and installs the `gore-modding` skill, so an
assistant gets the tools and the workflow around them in one step.

## Install

```powershell
claude plugin marketplace add dh0er/gore
claude plugin install gore@gore
```

In the Claude desktop app: the **+** button beside the prompt box, then
**Plugins → Add plugin**. The browser lists what your configured marketplaces
offer, so the `marketplace add` above still has to happen once.

From a checkout, with no marketplace and no install:

```powershell
claude --plugin-dir path\to\gore\plugins\gore
```

## `gore.exe` has to be on PATH

This is the one prerequisite the plugin cannot satisfy for you. Every bundled
MCP declaration starts the server as `gore mcp serve` — by name, not by absolute
path, because a plugin is shared across machines and an absolute path would be
wrong on most of them.

GORE is a Rust binary, not something a package manager fetches on demand, so
install it first. Either unpack a `gore-cli-v*`
[release](https://github.com/dh0er/gore/releases) and put that directory on
`PATH`, or build it from a checkout:

```powershell
cargo build --release -p gore     # → target\release\gore.exe
```

and put `target\release` on `PATH`. A `PATH` change only reaches processes
started afterwards, so restart the client — not just the plugin — once it is set.

**If it is missing**, the `gore_*` tools simply will not appear. The client
reports a server that failed to start; what it says depends on the client, and
none of them can say "add gore.exe to PATH" because none of them knows what
`gore` was supposed to be. Check it yourself:

```powershell
gore --version
```

No output, or "not recognized as an internal or external command", means PATH —
not the plugin. Fix that and restart the client.

## What it contains

| | |
|---|---|
| `.mcp.json`, `mcp.json` | the server: every `gore` command, dedicated offline standalone-compile tools, read-only bundle-inspection and Manager-preflight aliases, plus `gore_guide` and `gore_help`. Both files carry the same server map under the `mcpServers` wrapper expected by their clients |
| `skills/gore-modding/` | when to reach for which tool, the consent gate, and what a deploy does and does not prove |
| `.claude-plugin/`, `.codex-plugin/`, `.cursor-plugin/` | the same plugin, described the way each client wants it |

Enabling the plugin in Claude Code asks whether GORE may change your game
installation or perform protected Manager import, replacement, removal,
recovery, or Reset calls without confirming each call, and whether it may start
the game for a game-capable AngelScript backend. Explicit strict standalone
compilation is offline and never needs either permission. Reversible `enable`, `disable`, and `order`
edits update the target loadout immediately and are intentionally ungated. Both
settings are off unless you say otherwise; they map to
`GORE_MCP_ALLOW_WRITE` and `GORE_MCP_ALLOW_GAME_LAUNCH`, which is also how to set
them on a client that does not ask.

The skill deliberately carries no asset paths, ids or sample names. Everything
factual lives in the guide compiled into `gore.exe`, which is therefore always
the version you are actually running; a skill that restated any of it would drift
the first time the game updated.
