# MCP server (AI agents)

`gore mcp serve` exposes the whole CLI over the
[Model Context Protocol](https://modelcontextprotocol.io), so an AI assistant
can drive GORE directly: list textures, export dialog text, build a bundle,
check a load order for conflicts.

Every tool call runs a real `gore` subcommand as a child process and returns its
output, with the exact command line shown first — whatever the agent did, you can
re-run it in a shell yourself. All 77 leaf commands are reachable, and this guide
ships inside the binary so the agent can read it before acting.

## Setup

The server is part of `gore.exe`; there is nothing extra to install.

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

Point GORE at the game once (see [Getting started](getting-started.md)) and the
agent will not have to pass a game path at all:

```powershell
gore config detect
```

To see the exact tool definitions a client will receive, without starting a
server:

```powershell
gore mcp tools
```

## What the agent may do

Reading anything, and writing new files, always works. Two tiers are gated, and
the gate is decided **per subcommand**, not per tool.

| Flag | Unlocks | Examples |
|---|---|---|
| *(none)* | Reading, and commands that create a file that is not there yet | `texture list`, `loc export`, `mod build`, `as decompile` |
| `--allow-write` | Changing the game installation, rewriting a file in place, deleting user content, replacing a shared catalog, or overwriting an output file that already exists | `mod deploy`, `mgr apply`, `mgr reset`, `mgr remove`, `texture deploy`, `gen`, `deploy-shared`, `loc import` without an output path, `loc extract`, `catalog dump` onto an existing file |
| `--allow-game-launch` **and** `--allow-write` | Starting the game to compile AngelScript | `as compile`, `as compile-module` |

```powershell
gore mcp serve --allow-write
```

A refused call is not an error the agent can work around — it comes back with a
message naming the flag, and only you can restart the server with it.

Many commands sidestep the gate entirely: passing an output argument turns an
in-place rewrite into a new file. `loc import --out new.lcache` needs no flag;
omitting `--out` overwrites the game's own file and does.

Three rules are worth knowing because they have no equivalent on the command
line:

- **An existing output file is an overwrite, not a creation.** Every command
  that writes a *named* output file replaces it without asking, so pointing one
  at a path that already exists needs `--allow-write`; a fresh path needs
  nothing. That covers the catalog and model generators, `loc export`,
  `loc import`, `audio replace`, `audio export-patch`, `audio apply-patch`,
  `texture extract` and `project package`. Passing an input's own path as the
  output counts too — that is an in-place rewrite wearing a safe name.
  Commands writing *into* a directory (`stubs`, `audio extract`, `texture pack`,
  `as emit-all`, anything targeting the Mods folder) are not gated this way:
  the directory ordinarily exists and there is no single path to check. The
  `asset` and `voice` families and `as patch-default` need no gate at all —
  their CLI refuses an existing output on its own.
  Two commands write a path no argument names, and are checked all the same:
  `texture extract` also writes `<out>.png.json`, and `dump-mod` writes a
  `gore-dump/` folder inside the directory it is given. `gen` is the case that
  cannot be checked — the folder it rewrites is named inside `overrides.toml` —
  so it needs `--allow-write` outright.
  The check happens before the command starts, so it is a permission boundary
  and not a lock: a file created *while* a command runs is still overwritten.
  Closing that window would mean making these commands refuse to re-run at all,
  which is the one thing they exist to do.
- **`loc extract` is gated even though it never touches the game.** On the
  command line it asks *Proceed? [y/N]* before replacing the shared
  `loc_catalog.json` that the save editor and Mod Studio also read. Over MCP
  that prompt cannot be answered — stdin is the protocol channel — so it is
  suppressed, and the flag stands in for the confirmation.
- **Compiling needs both flags.** `as compile` drives the game to regenerate the
  script cache *and* installs the result, so `--allow-game-launch` alone is not
  enough. `mgr remove` is gated for the same family of reason: it deletes an
  imported mod from your library and nothing puts it back.

Two more flags tune behaviour rather than permissions:

| Flag | Default | Effect |
|---|---|---|
| `--timeout-secs <SECS>` | `0` | Override every per-command wall-clock cap. `0` keeps the built-in ones (60 s / 300 s / 1800 s by command). |
| `--max-output-kib <KIB>` | `256` | Cap on captured stdout per command. Truncated output says so. |

## The tools

Eleven tools mirror the CLI's command families; each takes a `subcommand` plus
its arguments. Two more are specific to the server.

| Tool | Covers | Guide |
|---|---|---|
| `gore_guide` | Search and read these pages | — |
| `gore_help` | `gore <cmd> --help` for any command | [cli-reference](cli-reference.md) |
| `gore_config` | `config` | [getting-started](getting-started.md) |
| `gore_catalog` | `dump` · `stubs` · `catalog` · `story-catalog` · `gui-model` · `sync` · `dump-mod` | [catalogs](catalogs-and-models.md) |
| `gore_project` | `scaffold` · `gen` · `package` · `deploy-shared` | [items](items.md) |
| `gore_loc` | `loc` | [text-and-dialogs](text-and-dialogs.md) |
| `gore_audio` | `audio` | [audio](audio.md) |
| `gore_voice` | `voice` | [voice](voice.md) |
| `gore_texture` | `texture` | [textures](textures.md) |
| `gore_asset` | `asset` | [dataassets](dataassets.md) |
| `gore_mod` | `mod` | [bundles](bundles.md) |
| `gore_mgr` | `mgr` | [mod-manager](mod-manager.md) |
| `gore_as` | `as` | [scripts](scripts.md) |

Eleven tools rather than 77 keeps a client's tool list navigable while still
covering every command. `gore_catalog` and `gore_project` have no matching CLI
subcommand — they group top-level commands that belong to one workflow.

## The documentation, over MCP

Every page of this guide **and** of the [technical reference](../reference/README.md)
is compiled into `gore.exe`, so both are available wherever you unpacked it.

The reference is included on purpose. The guide says which command to reach for;
the reference says what a receipt seals and why a patch was refused. An assistant
that can only read the guide will hit a refusal it cannot explain.

- **`gore_guide`** — `search` ranks individual sections across both bodies and
  labels each hit `[guide]` or `[reference]`, `read` fetches a page or one
  section of it, `list` shows the outline grouped by body. This is what the
  agent uses.
- **Resources** — the same pages as `gore://guide/<page>` and
  `gore://reference/<page>`, for clients that let you attach a document by hand.
  A page is only reachable through its own namespace.
- **Server instructions** — a short primer every client loads on connect: the
  tool list, how the game path is resolved, and which tiers are unlocked.

Only the guide ships in the release zip and only the guide is rendered by
`gore guide html`; the reference stays in the repository.

## How it behaves

- One command runs at a time. A call blocks until it finishes, and some commands
  (`texture index`, `as emit-all`) walk the whole installation and take minutes.
- Every command has a wall-clock limit and is killed if it exceeds it. Killing
  the CLI does not stop anything it started — after a timed-out `as compile`,
  check for a running game.
- Commands with a `--json` flag always get it, and their parsed output is
  returned as structured content alongside the text.
- A command that fails is reported as a normal result carrying the CLI's own
  error text, so the agent can read it and correct itself.

## Protocol notes

The server speaks stdio JSON-RPC 2.0 and implements the handshake-based protocol
revisions up to `2025-11-25`. That covers every current client. Clients that only
speak the newer per-request negotiation (`2026-07-28` and later) are not
supported yet.

## Troubleshooting

| Symptom | Cause |
|---|---|
| `does not identify itself as the gore CLI` at startup | The server verifies the binary it will re-exec by running `--version` once. Point the client at the real `gore.exe`. |
| Everything is refused with `--allow-write` | The server was started without it. Restart it with the flag. |
| A command cannot find the game | Run `gore config detect`, or `gore config set game-path <dir>`. |
| Output ends with `… [truncated]` | Narrow the query with the command's own filter, or write to a file with its output argument. |
