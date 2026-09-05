# MCP server (AI agents)

`gore mcp serve` exposes the whole CLI over the
[Model Context Protocol](https://modelcontextprotocol.io), so an AI assistant
can drive GORE directly: list textures, export dialog text, build a bundle,
check a load order for conflicts.

Every tool call runs a real `gore` subcommand as a child process and returns its
output, with the exact command line shown first — whatever the agent did, you can
re-run it in a shell yourself. All 89 leaf commands are reachable, and this guide
ships inside the binary so the agent can read it before acting.

## Setup

The server is part of `gore.exe`, so there is no second binary to fetch. What you
install is the **plugin**: it registers the server *and* adds the `gore-modding`
skill, so a client gets the tools and the workflow around them in one step
instead of two. The tools alone are the sharper edge — they will faithfully
replace an asset the engine never reads, and the skill is what stops that.

`plugins/gore/` is that plugin, and this repository is its own marketplace:

```powershell
claude plugin marketplace add dh0er/gore
claude plugin install gore@gore
```

This is a monorepo with a vendored toolchain in it, so a full clone costs more
than the plugin needs. `--sparse` limits the checkout to the two directories that
matter:

```powershell
claude plugin marketplace add dh0er/gore --sparse .claude-plugin plugins
```

In the Claude desktop app, the same thing without a terminal: the **+** button
beside the prompt box, then **Plugins → Add plugin**. The browser lists what your
configured marketplaces offer, so the `marketplace add` above still has to happen
once. **Manage plugins** in that menu enables, disables and uninstalls.

To run it straight from a checkout, with no marketplace and no install:

```powershell
claude --plugin-dir path\to\gore\plugins\gore
```

### Codex and Cursor

All three clients bundle MCP servers into plugins, and all three want it spelled
their own way — so this repository carries a marketplace manifest for each:

| Client | Marketplace manifest | Plugin manifest | MCP config |
|---|---|---|---|
| Claude Code | `.claude-plugin/marketplace.json` | `.claude-plugin/plugin.json` | `.mcp.json` |
| Codex | `.agents/plugins/marketplace.json` | `.codex-plugin/plugin.json` | `.mcp.json` |
| Cursor | `.cursor-plugin/marketplace.json` | `.cursor-plugin/plugin.json` | `mcp.json` |

The two MCP files contain the same server map under an `mcpServers` wrapper.
Claude Code and Codex read `.mcp.json`; Cursor reads `mcp.json`. The Codex
manifest points to `.mcp.json` explicitly. `scripts/check_plugin.py` validates
both wrapped files and keeps their server maps in step.

**Codex** takes a local marketplace directly, which makes it the easiest of the
three to try from a checkout:

```powershell
codex plugin marketplace add C:\path\to\gore
codex plugin add gore@gore
```

**Cursor** has no command-line install for plugins, and a marketplace of your own
is a Teams or Enterprise feature — Dashboard → Plugins → Add Marketplace, pointed
at this repository. The public marketplace takes submissions, reviewed by hand.
Without either, install nothing and wire the two halves up yourself:

```powershell
cursor --add-mcp '{"name":"gore","command":"gore","args":["mcp","serve"]}'
New-Item -ItemType Junction -Path $HOME\.cursor\skills\gore-modding `
         -Target C:\path\to\gore\plugins\gore\skills\gore-modding
```

A junction rather than a copy, so the skill tracks the checkout instead of
becoming a second version of itself. `~/.cursor/skills/` is the documented place
for a personal skill and `.cursor/skills/` for one shared through a repository;
`~/.cursor/skills-cursor/` is Cursor's own and must be left alone. The same
shape works for Codex under `~/.codex/skills/`.

### `gore.exe` has to be on `PATH`

Every bundled client declaration invokes `gore` by name rather than by absolute
path, because a plugin is shared across machines and an absolute path would be
wrong on most of them. That leaves one prerequisite it cannot satisfy for you,
since the binary is a Rust build rather than something a package manager fetches
on demand.

If it is missing, no `gore_*` tool appears at all: the client reports a server
that failed to start, and it cannot say why, because nothing on that side knows
what `gore` was supposed to be. `gore --version` in a terminal is the check. A
`PATH` change only reaches processes started afterwards, so restart the client.

### Wiring a client up by hand

Any client with a JSON config can register the server directly, without the
plugin. Two reasons to do it this way: the client has no plugin support, or you
want to pass [flags](#answering-in-advance), which the bundled plugin
declarations do not carry.

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

Claude Code has a one-liner for the same thing:

```powershell
claude mcp add gore -- gore mcp serve
```

Both spell the command as `gore`, so they need `PATH` set exactly as above.
Replace it with the absolute path to `gore.exe` if you would rather not.

What this route does not bring is the skill. The server's own primer reaches
every client on connect and carries the orientation an agent needs to start
safely — read the guide first, how the consent gate behaves, how to relay an
answer — but the judgement that grows with use lives in the skill, and only the
plugin installs it.

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
| Runs straight away | Reading, commands writing to a free path outside the game installation, and explicit strict standalone compilation | `texture list`, `loc export` to a new file, `as decompile`, `voice extract`, `as compile --backend standalone`, `as compile-module --backend standalone` |
| Asks you first, whatever is on disk | Changing the game installation, deleting or replacing Manager-library content, deleting other user content, or replacing a shared catalog — nothing you pass can make these harmless | `mod deploy`, `mod undeploy`, `mgr apply`, `mgr reset`, `mgr recover`, `mgr remove`, `mgr import`, `texture deploy`, `texture undeploy`, `texture replace`, `gen`, `deploy-shared`, `loc extract`, `audio restore` |
| Asks you only when something is in the way | Overwriting an output file that already exists, writing into a directory that already holds files, rebuilding a bundle folder that is there, or rewriting a file in place | `catalog dump` onto an existing file, `texture index` without an output path, `loc import` without an output path, `mod build` over an existing bundle, `stubs`, `audio extract` and `as emit-all` into a non-empty directory |
| Asks you first | A compiler policy that may start the game and stage sources in the installation | `as compile --backend game`, `as compile-module --backend standalone-then-game`, or either command with the backend omitted |

Many commands sidestep the question entirely: passing an output argument turns an
in-place rewrite into a new file. `loc import --out new.lcache` runs straight
away; omitting `--out` overwrites the game's own file and asks.

Three Manager snapshot calls are a deliberate exception to the simple
read/write wording: `mgr list`, `mgr analyze`, and `mgr status` may finish an
interrupted library replacement and persist the canonical reconciliation of a
valid loadout. They remain ungated because that repair is part of opening
authoritative Manager-owned state, but the MCP annotations do not call them
read-only. `mgr preflight` is the separate genuinely read-only inspection.

`mgr enable`, `mgr disable`, and `mgr order` are another explicit Manager-state
class: they update the reversible target loadout immediately, are labelled
`updates Manager loadout`, and intentionally remain ungated. They do not touch
the game until a separately confirmed `mgr apply`.

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
    "args": { "lcache": "…/AlkimiaLocalization_00000000.lcache", "edits": "…/edits.json" },
    "approval_request_id": "gore-consent-…",
    "user_approved": "ja, überschreib die Datei"
  }
}
```

The refusal supplies `approval_request_id`. It is opaque, expires after 15
minutes, works once, and is bound to the exact normalized command. Changing an
argument, inventing the id, or reusing it is refused. The user's answer does not
need a ritual phrase: a clear instruction such as “do it and stop asking” is a
valid answer for that exact displayed call. It does not become a permanent
permission for different future mutations.

The exact retry then runs without a second question, and the result records what
happened:

```
This ran with a one-time approval request bound to this exact command, carrying
the assistant's verbatim relay: "ja, überschreib die Datei". The binding was
verified; who authored those words was not.
```

Read that sentence literally. The server verifies that the retry is exactly the
call it refused and that the id has not been reused. It still cannot prove who
typed the relayed words. What you get is the whole exchange in your transcript:
the refusal, bound id, question put to you, your answer, and note on the run.

The field is refused under `--no-consent-prompts` — that flag exists so that an
agent nobody is reviewing cannot talk its own way past the gate.

### Answering in advance

Where nobody is watching — CI, a scripted batch, an agent that already has its own
approval layer — you can answer once at startup instead:

| Flag | Environment variable | Effect |
|---|---|---|
| `--allow-write` | `GORE_MCP_ALLOW_WRITE` | Installation changes, Manager-library mutations, deletions, and in-place rewrites run without asking |
| `--allow-game-launch` | `GORE_MCP_ALLOW_GAME_LAUNCH` | The same for compiler policies that may start the game. `game`, `standalone-then-game`, and an omitted backend need **both** flags; explicit strict `standalone` needs neither |
| `--no-consent-prompts` | `GORE_MCP_NO_CONSENT_PROMPTS` | Never ask, and refuse anything that would need it. The strict posture, for a server exposed to an agent whose calls nobody reviews. It cannot be combined with the two above — that would be asking for a looser and a stricter server at once, and the server refuses to start rather than pick one |

```powershell
gore mcp serve --allow-write
```

**Under the plugin, these are the route.** A plugin's MCP config carries a fixed
`args` array and nothing between it and the server can add to it, so there is
nowhere to type a flag — but `env` is part of that same config, and the plugin
maps both permissions through it.

In Claude Code you never touch a variable: the plugin declares them under
`userConfig`, so enabling it asks you, in a dialog, whether GORE may change the
game, replace Manager-library content, or perform another protected write
without confirming each time. Leave both off unless you have a reason.
`${user_config.…}` is a Claude Code substitution. Codex reads the wrapped
`.mcp.json` file and Cursor reads the wrapped `mcp.json` file; both pass the
placeholder text through untouched, which the server reads as "not set" rather
than as an error.

Anywhere else, set the variable in whatever your client launches from — the shell
it inherits, or its own environment settings — and restart it, since a running
process keeps the environment it started with:

```powershell
$env:GORE_MCP_ALLOW_WRITE = '1'
```

Either source turning a permission on is enough, and setting both is not an
error: a flag and its variable are the same person saying the same thing twice.
Off is `0`, `false`, `no`, `off`, empty, or simply unset. Anything else refuses
to start and says which variable — a permission setting that cannot be read is
the one case where either default misleads, one by granting what nobody wrote
and the other by leaving someone to wonder why every call still stops to ask.

Six rules are worth knowing because they have no equivalent on the command
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
  Four commands write a path no argument spells out, and are checked all the
  same: `texture extract` also writes `<out>.png.json`, `dump-mod` writes a
  `gore-dump/` folder inside the directory it is given, `scaffold` writes
  `<out>/<mod_name>/`, and `mod build` clears and rebuilds
  `<out>/<meta.name from the spec>` — so a fresh mod name runs straight away and
  only a collision with a folder already there asks. A spec that cannot be read,
  or whose name is not a single folder name, counts as a collision: not knowing
  what would be deleted is not the same as knowing nothing would be.
- **A directory only matters when something is in it.** `audio extract`,
  `stubs` and `as emit-all` each write one file per sample, class or module,
  under names that come from the bank, the model or the cache — so which files
  they land on cannot be known beforehand. An empty or absent output directory
  has nothing to lose either way, and those calls run without asking; one that
  already holds files asks first, and names the directory. Where the names
  cannot be checked *and* the folder cannot either, the command asks outright:
  `gen` (target folder named inside the `overrides.toml` it reads),
  `texture replace` (cooked files under a path derived from the asset name,
  deleting a stale `.ubulk`), and `mgr import`. The latter resolves a verified
  existing entry by entry id, source identity, or content identity; a changed
  match replaces that library payload during import, while an unchanged match
  is a no-op. The consent gate runs before that identity decision, so import
  still asks up front.
  Every check here happens before the command starts, so it is a decision point
  and not a lock: a file created *while* a command runs is still overwritten.
  Closing that window would mean making these commands refuse to re-run at all,
  which is the one thing they exist to do.
- **Where a file lands can matter more than what writes it.** `texture pack`,
  `asset pack` and `mod build` normally produce an artifact you deploy later,
  and `dump-mod` and `scaffold` normally produce a mod folder you install
  afterwards — so none of them asks. Point an output inside the game tree and
  the same call
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
- **Only game-capable compilation is both at once.** Explicit
  `--backend standalone` runs offline and needs no game-launch consent. With a
  fresh generated work tree and ordinary outputs outside the installation it
  also needs no install-write consent. An occupied generated work tree or an
  output aimed into the installation remains protected. `game`,
  `standalone-then-game`, and the omitted default may drive the game to
  regenerate the script cache and stage sources, so pre-approving only the
  launch is not enough. `mgr remove` asks for a different reason: it deletes an
  imported mod from your library.

Two more flags tune behaviour rather than permissions:

| Flag | Default | Effect |
|---|---|---|
| `--timeout-secs <SECS>` | `0` | Override every per-command wall-clock cap. `0` keeps the built-in ones (60 s / 300 s / 1800 s, and 2700 s for `as compile`). |
| `--max-output-kib <KIB>` | `256` | Cap on captured stdout per command. Truncated output says so. `0` keeps the default, as it does above. |

## The tools

Twenty CLI-backed tools mirror the CLI's command families and safe aliases. Namespace tools take a
`subcommand` plus an `args` object. `gore_doctor` and `gore_find`, plus the
dedicated `gore_mod_inspect`, `gore_mgr_preflight`, `gore_as_compile`, and
`gore_as_compile_module` aliases, already
select one command, so their typed arguments go directly at the top level; the
old envelope remains accepted for compatibility. Two more tools are specific
to the server.

| Tool | Covers | Guide |
|---|---|---|
| `gore_guide` | Search and read these pages | — |
| `gore_help` | `gore <cmd> --help` for any command | [cli-reference](cli-reference.md) |
| `gore_config` | `config` | [getting-started](getting-started.md) |
| `gore_doctor` | `doctor` | [getting-started](getting-started.md) |
| `gore_find` | `find` | [find](find.md) |
| `gore_catalog` | `dump` · `stubs` · `catalog` · `story-catalog` · `location-catalog` · `gui-model` · `sync` · `dump-mod` | [catalogs](catalogs-and-models.md) |
| `gore_location` | `location` | [catalogs](catalogs-and-models.md) |
| `gore_project` | `scaffold` · `gen` · `package` · `deploy-shared` | [items](items.md) |
| `gore_dialog` | `dialog` | [dialog-trees](dialog-trees.md) |
| `gore_npc` | `npc` | [npc-authoring](npc-authoring.md) |
| `gore_loc` | `loc` | [text-and-dialogs](text-and-dialogs.md) |
| `gore_audio` | `audio` | [audio](audio.md) |
| `gore_voice` | `voice` | [voice](voice.md) |
| `gore_texture` | `texture` | [textures](textures.md) |
| `gore_asset` | `asset` | [dataassets](dataassets.md) |
| `gore_mod` | `mod` | [bundles](bundles.md) |
| `gore_mod_inspect` | read-only `mod inspect` alias | [bundles](bundles.md) |
| `gore_mgr` | `mgr` | [mod-manager](mod-manager.md) |
| `gore_mgr_preflight` | read-only `mgr preflight` alias | [mod-manager](mod-manager.md) |
| `gore_as_compile` | strict standalone `as compile` alias | [scripts](scripts.md) |
| `gore_as_compile_module` | strict standalone `as compile-module` alias | [scripts](scripts.md) |
| `gore_as` | `as` | [scripts](scripts.md) |

`gore_help` is deliberately different from the namespace tools: it accepts one
space-separated **CLI** path in `command`. For example, ask for
`{"command":"loc export"}`. Do not pass the MCP tool name (`gore_loc`) and do
not add a separate `subcommand` property. The tool schema already carries the
ordinary arguments, so use help only when that schema is not specific enough;
a blanket help call for every later step adds work without improving safety.

Twenty-two tools rather than 99 leaves keeps a client's tool list navigable while
still covering every command. The extra bundle-inspection and Manager-preflight
routes exist because MCP annotations apply to a whole tool: mixed `gore_mod`
and `gore_mgr` must advertise their install-changing worst cases, while
`gore_mod_inspect` and `gore_mgr_preflight` can truthfully advertise read-only,
while the standalone compile aliases advertise non-destructive offline writes.
`gore_catalog` and `gore_project` have no matching CLI subcommand — they group
top-level commands that belong to one workflow.

## The documentation, over MCP

Every page of this guide **and** of the [technical reference](../reference/README.md)
is compiled into `gore.exe`, so both are available wherever you unpacked it.

The reference is included on purpose. The guide says which command to reach for;
the reference says what a receipt seals and why a patch was refused. An assistant
that can only read the guide will hit a refusal it cannot explain.

- **`gore_guide`** — `search` ranks individual sections globally across both
  bodies and labels each hit `[guide]` or `[reference]`; call it once without
  `page`, not once per candidate page. If the page and section are already
  known, use `read` directly. `read` fetches a page or one section of it, and
  `list` shows the outline grouped by body. This is what the agent uses. A page
  too long for one result comes back in parts split at
  heading boundaries, each naming the sections the other parts hold, so `read`
  never silently drops the half an agent needed.
  [`gore guide search`](cli-reference.md#guide) runs the same ranking from a
  shell, so a bad result can be reproduced without a client.
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
- Every command has a wall-clock limit and is killed if it exceeds it. For a
  timed-out game-capable compile, check for a running game; strict standalone
  never starts one.
- Game-capable `as compile` gets a longer cap than anything else on purpose. It
  hands the game its own 30-minute deadline and restores the installation
  afterwards, so the outer limit has to outlast the inner one. This install
  recovery warning does not apply to explicit strict standalone.
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
| Destructive calls are refused *instantly* and no dialog ever appears | Your client is answering for you — Claude Code does this whenever it is not interactive. Have the assistant ask you in the chat and retry with the refusal's `approval_request_id` plus your words in `user_approved`, use an interactive client, or start the server with `--allow-write`. |
| A command ran that you never clicked a dialog for | Look for the note at the end of its result: it used a one-time request bound to the exact call and carried the words relayed in `user_approved`. Scroll back to check that you gave them. `--no-consent-prompts` disables that route entirely. |
| A call sits there doing nothing | It is waiting on the confirmation dialog. Answer it, or let your client's request timeout cancel it. |
| A command cannot find the game | Run `gore config detect`, or `gore config set game-path <dir>`. |
| Output ends with `… [truncated]` | Narrow the query with the command's own filter, or write to a file with its output argument. |
