# MCP server (AI agents)

`gore mcp serve` exposes the whole CLI over the
[Model Context Protocol](https://modelcontextprotocol.io), so an AI assistant
can drive GORE directly: list textures, export dialog text, build a bundle,
check a load order for conflicts.

Every tool call runs a real `gore` subcommand as a child process and returns its
output, with the exact command line shown first — whatever the agent did, you can
re-run it in a shell yourself. All 79 leaf commands are reachable, and this guide
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

### As a Claude Code plugin

`plugins/gore/` in this repository is a plugin that registers the server *and*
installs the `gore-modding` skill, so a client gets the tools and the workflow
around them in one step instead of two. Its `.mcp.json` invokes `gore` by name,
so `gore.exe` has to be on `PATH` — that is the one thing the plugin cannot do
for you, since the binary is a Rust build rather than something a package manager
fetches on demand.

The skill deliberately carries no asset paths, ids or sample names. Everything
factual lives in this guide, which ships inside the binary and is therefore always
the version you are actually running; a skill that restated any of it would drift
the first time the game updated.

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

Reading anything always works, and so does writing to a path that is free and
outside the game installation. Anything that changes something already there is
**confirmed with you first**: the server puts the question to your client, the
client shows you a dialog naming the command and the file, and the command runs
only if you agree. Nothing has been started at that point, so saying no leaves
every file exactly as it was.

Where your client cannot show that dialog, the assistant asks you in the chat
instead and relays your answer — see [Approving in the
conversation](#approving-in-the-conversation).

The decision is made **per subcommand**, not per tool.

| | What it covers | Examples |
|---|---|---|
| Runs straight away | Reading, and commands writing to a free path outside the game installation | `texture list`, `loc export` to a new file, `as decompile`, `voice extract` |
| Asks you first | Changing the game installation, rewriting a file in place, deleting user content, replacing a shared catalog, or overwriting an output file that already exists | `mod deploy`, `mod build`, `mgr apply`, `mgr reset`, `mgr remove`, `mgr import`, `texture deploy`, `texture replace`, `texture index` without an output path, `gen`, `stubs`, `audio extract`, `as emit-all`, `deploy-shared`, `loc import` without an output path, `loc extract`, `catalog dump` onto an existing file |
| Asks you first | Starting the game to compile AngelScript | `as compile`, `as compile-module` |

Many commands sidestep the question entirely: passing an output argument turns an
in-place rewrite into a new file. `loc import --out new.lcache` runs straight
away; omitting `--out` overwrites the game's own file and asks.

### Which clients can ask

Confirmation uses MCP [elicitation][elicit], a client feature. **Claude Code,
Cursor and Codex all support it**, and there is nothing to configure — the dialog
appears on its own.

A client that does not advertise the capability cannot be asked, so for that
client the same calls are refused instead. The refusal names the flag below, and
only you can restart the server with it.

If nobody answers the dialog, the call waits. Your client's own request timeout
ends it: the client sends a cancellation, and the server treats that as a no.

> **A client can advertise the capability and still never ask you.** Claude Code
> driven non-interactively — the desktop app, `claude -p`, a scripted run — logs
> `Elicitation request received in print mode` and answers on your behalf within
> milliseconds, showing nothing. Every confirmable call is then refused without
> you seeing anything.
>
> The server cannot tell that apart from a person clicking "no": all it sees is
> an answer arriving. So it never claims one — a refusal names the raw answer
> (`decline` or `cancel`) and says that some clients answer for you. If yours
> does, approve in the conversation instead (below), use an interactive `claude`
> terminal, Cursor or Codex, or start the server pre-approved with
> `--allow-write`.

[elicit]: https://modelcontextprotocol.io/specification/2025-06-18/client/elicitation

### Approving in the conversation

Where no dialog reaches you, the assistant can still ask you the ordinary way —
in the chat. Every refusal tells it to do exactly that, and shows the command
line to put in front of you. If you agree, it sends the same call again with one
extra argument:

```json
{
  "name": "gore_loc",
  "arguments": {
    "subcommand": "import",
    "args": { "lcache": "…/Alkimia.lcache", "edits": "…/edits.json" },
    "user_approved": "ja, überschreib die Datei"
  }
}
```

The command then runs without a second question, and the result records what
happened:

```
This ran on the assistant's assertion of prior approval, quoted as: "ja,
überschreib die Datei". No confirmation reached this server; the claim was not
verified.
```

Read that sentence literally. The server sees a claim, never a confirmation — an
assistant that sets the field without asking you is not caught here. What you
get instead is the whole exchange in your transcript: the refusal, the question
put to you, your answer, and the note on the run itself. That is strictly more
than `--allow-write` shows you, which grants everything silently for the life of
the server, and strictly less than a dialog you clicked yourself.

The field is refused under `--no-consent-prompts` — that flag exists so that an
agent nobody is reviewing cannot talk its own way past the gate.

### Answering in advance

Where nobody is watching — CI, a scripted batch, an agent that already has its own
approval layer — you can answer once at startup instead:

| Flag | Effect |
|---|---|
| `--allow-write` | Installation changes and in-place rewrites run without asking |
| `--allow-game-launch` | The same for starting the game. Compiling needs **both**, because it also stages files in the installation |
| `--no-consent-prompts` | Never ask, and refuse anything that would need it. The strict posture, for a server exposed to an agent whose calls nobody reviews. It cannot be combined with the two above — that would be asking for a looser and a stricter server at once, and the server refuses to start rather than pick one |

```powershell
gore mcp serve --allow-write
```

Five rules are worth knowing because they have no equivalent on the command
line:

- **An existing output file is an overwrite, not a creation.** Every command
  that writes a *named* output file replaces it without asking, so pointing one
  at a path that already exists asks first; a fresh path does not. That covers the catalog and model generators, `loc export`,
  `loc import`, `audio replace`, `audio export-patch`, `audio apply-patch`,
  `texture extract`, `texture index --out`, `project package`, and the
  cache-producing `as` commands (`replace`, `splice`, `extract`,
  `extract-remap`, and `bytediff --json`). Passing an input's own path as the
  output counts too — that is an in-place rewrite wearing a safe name.
  The `asset` and `voice` families, `texture pack` and `as patch-default` need
  no confirmation at all — their CLI refuses an existing output on its own. `scaffold`
  refuses too, but only when `Scripts/main.lua` is already there, which is why
  its mod folder is checked here as well.
  Three commands write a path no argument spells out, and are checked all the
  same: `texture extract` also writes `<out>.png.json`, `dump-mod` writes a
  `gore-dump/` folder inside the directory it is given, and `scaffold` writes
  `<out>/<mod_name>/` — so a fresh mod name runs straight away and only a
  collision with an existing mod folder asks. Commands whose targets cannot be
  checked ask outright, and writing into a directory is no exception —
  the folder may exist harmlessly while the files inside it are replaced one by
  one. That covers `gen` and `mod build` (target folder named inside the spec
  file they read; `mod build` clears it first), `texture replace` (cooked files
  under a path derived from the asset name, deleting a stale `.ubulk`),
  `stubs`, `audio extract` and `as emit-all` (one file per class, sample or
  module), and `mgr import` (re-importing the same mod deletes the payload of
  the entry it replaces).
  The check happens before the command starts, so it is a decision point and not
  a lock: a file created *while* a command runs is still overwritten.
  Closing that window would mean making these commands refuse to re-run at all,
  which is the one thing they exist to do.
- **Where a file lands can matter more than what writes it.** `texture pack`
  and `asset pack` normally produce an artifact you deploy later, and `dump-mod`
  and `scaffold` normally produce a mod folder you install afterwards — so none
  of them asks. Point an output inside the game tree and the same call
  writes straight into the live installation — the `~mods` override, `ue4ss\Mods`,
  or the game's own `.lcache` or `.bank` — which is a deployment however new the
  path is. That case asks, recognised either from an explicit `--game` or from a
  `G1R` folder in the path, and it applies to every command's declared output
  rather than a chosen few.
- **`gore config` is the one exception to all of this.** `set`, `unset` and
  `detect` rewrite the shared `config.json` without asking, even though it
  already exists. What they change is a preference — one path, visible in
  `config list`, restored by setting it again — and it is what an assistant
  needs when a command cannot find the game. Putting it behind a question would
  turn the most common setup failure into an interruption every time.
- **`loc extract` asks even though it never touches the game.** On the command
  line it asks *Proceed? [y/N]* before replacing the shared `loc_catalog.json`
  that the save editor and Mod Studio also read. Over MCP that prompt cannot be
  answered — stdin is the protocol channel — so it is suppressed and the MCP
  dialog stands in for it, which is the same question by another route.
- **Compiling is both at once.** `as compile` drives the game to regenerate the
  script cache *and* installs the result, so pre-approving only the launch is
  not enough. `mgr remove` asks for the same family of reason: it deletes an
  imported mod from your library and nothing puts it back.

Two more flags tune behaviour rather than permissions:

| Flag | Default | Effect |
|---|---|---|
| `--timeout-secs <SECS>` | `0` | Override every per-command wall-clock cap. `0` keeps the built-in ones (60 s / 300 s / 1800 s, and 2700 s for `as compile`). |
| `--max-output-kib <KIB>` | `256` | Cap on captured stdout per command. Truncated output says so. `0` keeps the default, as it does above. |

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

Eleven tools rather than 79 keeps a client's tool list navigable while still
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
- `as compile` gets a longer cap than anything else on purpose. It hands the
  game its own 30-minute deadline and restores the installation afterwards, so
  the outer limit has to outlast the inner one — a wrapper killed mid-compile
  never runs that restore. Setting `--timeout-secs` below 30 minutes removes
  that headroom and makes a timed-out compile leave the install staged.
- Commands with a `--json` flag always get it, so what comes back is machine
  readable. It is returned as text like everything else: a result carrying
  structured content instead would be treated by some clients as *the* result,
  and the agent would never see the command's output at all.
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
| Destructive calls are refused instead of asking | Either your client does not support elicitation, or the server was started with `--no-consent-prompts`. The refusal says which. |
| Destructive calls are refused *instantly* and no dialog ever appears | Your client is answering for you — Claude Code does this whenever it is not interactive. Have the assistant ask you in the chat and relay it (`user_approved`), use an interactive client, or start the server with `--allow-write`. |
| A command ran that you never clicked a dialog for | Look for the note at the end of its result: it ran on `user_approved`, which is the assistant's claim that you agreed in the conversation. Scroll back to check that you did. `--no-consent-prompts` disables that route entirely. |
| A call sits there doing nothing | It is waiting on the confirmation dialog. Answer it, or let your client's request timeout cancel it. |
| A command cannot find the game | Run `gore config detect`, or `gore config set game-path <dir>`. |
| Output ends with `… [truncated]` | Narrow the query with the command's own filter, or write to a file with its output argument. |
