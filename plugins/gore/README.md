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

This is the one prerequisite the plugin cannot satisfy for you. `.mcp.json`
starts the server as `gore mcp serve` — by name, not by absolute path, because a
plugin is shared across machines and an absolute path would be wrong on most of
them.

GORE is a Rust binary, not something a package manager fetches on demand, so
install it first: download a `gore-cli-v*` release, unpack it, and put that
directory on `PATH`.

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
| `.mcp.json` | the server: every `gore` command, as twelve command-family tools plus `gore_guide` and `gore_help` |
| `skills/gore-modding/` | when to reach for which tool, the consent gate, and what a deploy does and does not prove |

The skill deliberately carries no asset paths, ids or sample names. Everything
factual lives in the guide compiled into `gore.exe`, which is therefore always
the version you are actually running; a skill that restated any of it would drift
the first time the game updated.
