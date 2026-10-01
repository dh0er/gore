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

## Codex

Register the checkout you maintain, then install the plugin:

```powershell
codex plugin marketplace add C:\path\to\gore
codex plugin add gore@gore
```

Start a new session after installation so it loads the plugin's skill and tools.
The bundled GORE server is required in Codex and has a 30-second startup timeout.
Codex waits for its MCP discovery before building the initial tool catalog;
startup failure is reported instead of silently omitting the tools.

Codex otherwise gives optional servers a one-second startup grace. GORE's early
read-only compiler/API checks can take several seconds, so an older optional
plugin can be absent from the first turn even with a working CLI and a completed
Codex restart. Update the plugin from the maintained marketplace; a CLI rebuild
is not needed for this configuration change. See the
[official MCP startup options](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

## `gore.exe` has to be on PATH

This is the one prerequisite the plugin cannot satisfy for you. Every bundled
MCP declaration starts the server as `gore mcp serve` — by name, not by absolute
path, because a plugin is shared across machines and an absolute path would be
wrong on most of them.

GORE is a Rust binary, not something a package manager fetches on demand, so
install it first. Either unpack a `gore-cli-v*`
[release](https://github.com/dh0er/gore/releases) and put that directory on
`PATH`, or build it from the checkout root:

```powershell
python build.py gore-cli dist             # recommended installable zip → dist\gore-cli\
python build.py gore-cli build --release  # CLI + compiler → target\release\
```

For installation, unpack the complete zip from `dist\gore-cli` into a stable
directory such as `C:\Tools\gore-cli`, and put that directory on `PATH`. Keep
`gore.exe`, `compiler\`, `docs\`, and the other companion files together when
installing or updating. A `PATH` change only reaches processes started
afterwards, so restart the client once it is set.

Normal CLI builds require the nonempty embedded compiler catalog prepared by
`build.py`; raw `cargo build -p gore` without it fails. The debug-only
`development-cli` feature produces a visibly marked `development-unbundled`
CLI for development and tests, not installation. It cannot bypass the catalog
requirement in release builds. See [Building](../../docs/development.md) for
toolchain requirements and development commands.

**If it is missing**, the `gore_*` tools simply will not appear. The client
reports a server that failed to start; what it says depends on the client, and
none of them can say "add gore.exe to PATH" because none of them knows what
`gore` was supposed to be. Check it yourself:

```powershell
gore --version
```

A command-not-found error establishes a command-resolution problem. Check the
client's PATH and restart it after correcting that environment. If the command
works but tools are missing, check the client's MCP startup status and plugin
registration instead:

```powershell
codex plugin list
codex mcp get gore --json
```

These commands show configuration, not a completed connection. A pending optional
server needs startup handling; a disabled server or actual startup error needs
its reported fix. Offline authoring can still use the working CLI, with Doctor
and the same consent boundaries.

## Updating and repairing the plugin

Keep the complete CLI package in a stable directory on `PATH`, such as
`C:\Tools\gore-cli`, and replace that package when updating. Both shipped MCP
files use `"command": "gore"`. Codex caches the plugin files, but this command
still resolves the CLI through `PATH` when its server starts; it must not be
rewritten to a versioned executable path.

Check which CLI and marketplace you are using:

```powershell
Get-Command gore | Select-Object Source
gore --version
codex plugin list
```

MCP command results print the exact executable they ran. If that path differs
from the expected `Get-Command gore` result, inspect the source plugin's
`.mcp.json`. Reinstalling from an old local copy will reproduce its pinned path.
For a local `gore` marketplace that points at an obsolete copy, replace its
registration with your maintained checkout:

```powershell
codex plugin marketplace remove gore
codex plugin marketplace add C:\path\to\gore
codex plugin add gore@gore
```

Restart Codex after a CLI or `PATH` update, then use a new session. A running
MCP process retains its executable, embedded guides and tool list. Updating
the CLI does not require changing the plugin's command or its release version.
Plugin skill/config changes do require reinstalling the plugin from its current
source. Do not hand-edit the installed cache or keep an old release path there.

## What it contains

| | |
|---|---|
| `.mcp.json`, `mcp.json` | the server: every `gore` command, dedicated offline standalone-compile tools, read-only bundle-inspection and Manager-preflight aliases, plus `gore_guide` and `gore_help`. Both files carry the same server map under the `mcpServers` wrapper expected by their clients |
| `skills/gore-modding/` | when to reach for which tool, the checked dialog-authoring workflow, the consent gate, and what a deploy does and does not prove |
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

The skill deliberately carries no volatile asset paths, ids or sample names. It
routes dialog work through the checked MCP surface and names only the durable
safety boundaries needed to choose that workflow. Detailed capability evidence
and build-specific facts live in the guide compiled into `gore.exe`, which is
therefore the version you are actually running.
