# CLI reference

Every command of the `gore` binary. Run `gore <cmd> --help` (or
`gore <cmd> <sub> --help`) for the authoritative, always-current text — this
page mirrors it and links to the guide that explains each area.

```
gore <COMMAND> [OPTIONS]
gore --version
```

## Overview

| Command | Subcommands | Purpose | Guide |
|---------|-------------|---------|-------|
| `config` | `set` · `get` · `unset` · `list` · `path` · `detect` | Persist shared settings (the game path) so other commands can omit `--game`. | [getting-started](getting-started.md#point-gore-at-the-game) |
| `mcp` | `serve` · `tools` | Serve the whole CLI over the Model Context Protocol (stdio JSON-RPC) for AI assistants. | [mcp](mcp.md) |
| `guide` | `html` | Render this guide into one self-contained HTML file for offline reading. | [below](#guide) |
| `gen` | — | Compile `overrides.toml` → a UE4SS Lua override mod. | [items](items.md) |
| `mod` | `build` · `deploy` · `undeploy` | Build/deploy/undeploy a unified bundle. | [bundles](bundles.md) |
| `mgr` | `import` · `list` · `remove` · `enable` · `disable` · `order` · `analyze` · `apply` · `status` · `reset` | Multi-mod manager: library, load order, conflicts, composed deploy. | [mod-manager](mod-manager.md) |
| `loc` | `extract` · `status` · `export` · `import` | Read/edit localized text & dialogs in the encrypted `.lcache`. | [text-and-dialogs](text-and-dialogs.md) |
| `audio` | `list` · `extract` · `replace` · `restore` · `export-patch` · `apply-patch` | Read/replace FMOD `.bank` audio (PCM injection, `*.gore-bak`). | [audio](audio.md) |
| `voice` | `list` (`index`) · `match-line` · `extract` · `add` · `replace` · `apply-manifest` (`apply`) | Index/extract/copy-on-write edit voice-over ZIP archives. | [voice](voice.md) |
| `texture` | `list` · `extract` · `replace` · `pack` · `deploy` · `index` · `undeploy` · `paklist` | Extract/replace IoStore textures → Zen triplet in `~mods`. | [textures](textures.md) |
| `asset` | `extract` · `inspect` · `patch-fixed` · `pack` | Extract, inspect, copy-on-write patch, and offline-pack one cooked DataAsset. | [dataassets](dataassets.md) |
| `as` | see [below](#as) | AngelScript precompiled-cache tooling (experimental). | [scripts](scripts.md) |
| `catalog` | — | Generate an item/npc/knowledge catalog from a UE4SS object dump. | [catalogs](catalogs-and-models.md) |
| `story-catalog` | — | Build a generation-sealed NPC and quest-parent catalog. | [catalogs](catalogs-and-models.md) |
| `location-catalog` | — | Build the named-location catalog from the game's `InteractionSpots.json`. | [catalogs](catalogs-and-models.md) |
| `location` | `resolve` · `list` | Look a waypoint/spot name up in the bundled catalog — offline, no install. | [catalogs](catalogs-and-models.md#checking-a-spot-name-before-the-game-swallows-it) |
| `dump` | — | Parse a UE4SS SDK header dump into a reflection model JSON. | [catalogs](catalogs-and-models.md) |
| `stubs` | — | Emit LuaLS/EmmyLua type stubs from `model.json`. | [catalogs](catalogs-and-models.md) |
| `gui-model` | — | Convert a reflection model into the GUI shape JSON. | [catalogs](catalogs-and-models.md) |
| `sync` | — | Refresh the GUI model from a runtime game-data dump. | [catalogs](catalogs-and-models.md) |
| `dump-mod` | — | Generate the `gore-dump` UE4SS mod that produces that dump. | [catalogs](catalogs-and-models.md) |
| `scaffold` | — | Create a hand-written gore-lua mod skeleton. | [bundles](bundles.md#other-helpers) |
| `deploy-shared` | — | Install the gore-lua helpers into `ue4ss\Mods\shared`. | [gore-lua](../../lua/README.md) |
| `package` | — | Zip a mod folder into distributable UE4SS layout. | [bundles](bundles.md#other-helpers) |

Commands that touch the install (`deploy-shared`, `mod`, `mgr`, `texture`,
`asset`, `as`, `loc`) resolve it from an explicit `--game` (or `--lcache` /
`--exe`), then the configured game path, then Steam auto-detect.

## `config`

| Subcommand | Arguments |
|---|---|
| `set <KEY> <VALUE>` | `KEY` = `game-path` (an install root or the `.exe`) |
| `get <KEY>` | prints the value, non-zero exit if unset |
| `unset <KEY>` | clears it |
| `list` | all values plus the resolved root and its source |
| `path` | path of `config.json` |
| `detect` | Steam auto-detect, saved as `game-path` |

## `gen`

`gore gen [OPTIONS] --out <OUT> <OVERRIDES>`

| Flag | Meaning |
|---|---|
| `<OVERRIDES>` | Path to `overrides.toml`. |
| `-o, --out <OUT>` | Mods directory to write the mod folder into. |
| `--model <MODEL>` | Validate field names/types against this reflection model. |

## `mod`

| Subcommand | Flags |
|---|---|
| `build` | `--spec <SPEC>` (asset paths resolve against its directory) · `-o, --out <OUT>` (bundle goes to `<out>/<mod-name>`) |
| `deploy` | `--bundle <BUNDLE>` · `--game <GAME>` |
| `undeploy` | `--game <GAME>` |

## `mgr`

All subcommands except `reset` accept `--library <DIR>` and `--loadout <FILE>`.

| Subcommand | Arguments |
|---|---|
| `import <PATH>` | source folder / `.zip` / game file |
| `list` | — |
| `remove <ID>` | library entry id |
| `enable <ID>` / `disable <ID>` | library entry id |
| `order <ID> <POS>` | `POS` is 0-based; 0 mounts first and loses conflicts |
| `analyze` | — |
| `apply` / `status` | `--game <GAME>` |
| `reset` | `--game <GAME>` |

## `loc`

| Subcommand | Flags |
|---|---|
| `extract` | `--lcache <PATH>` (`.lcache`, game dir, or Steam library; else auto-detect) · `-y, --yes` |
| `status` | — |
| `export` | `--lcache <PATH>` · `-o, --out <OUT>` · `--keep-empty` |
| `import` | `--lcache <PATH>` · `--edits <EDITS>` · `-o, --out <OUT>` · `--add-missing` |

## `audio`

| Subcommand | Flags |
|---|---|
| `list` | `--bank <BANK>` · `--filter <TEXT>` · `--max <N>` (default 100) · `--json` · `--key <KEY>` |
| `extract` | `--bank` · `-o, --out <DIR>` · `--sample <NAME\|all>` · `--key` |
| `replace` | `--map <MAP>` · `--bank` · `-o, --out <BANK>` · `--key` |
| `restore` | `--bank` |
| `export-patch` | `--map <MAP>` · `-o, --out <ZIP>` |
| `apply-patch` | `--patch <ZIP>` · `--bank` · `-o, --out <BANK>` · `--key` |

Without `-o`, `replace` and `apply-patch` overwrite the bank in place and back
it up to `*.gore-bak`.

## `voice`

| Subcommand | Flags |
|---|---|
| `list` (alias `index`) | `--archive <ZIP>` · `--filter <TEXT>` · `--max <N>` (default 100) · `--directories` · `--json` |
| `match-line` | `--archive` · `--loc-id <ASCII_ID>` (no `.ogg` suffix) · `--json` |
| `extract` | `--archive` · `--basename <NAME>` \| `--path <ARCHIVE_PATH>` · `-o, --out <DIR>` |
| `add` | `--archive` · `--path <ARCHIVE_PATH>` · `--ogg <OGG>` · `-o, --out <ZIP>` |
| `replace` | `--archive` · `--basename` \| `--path` · `--ogg` · `-o, --out <ZIP>` |
| `apply-manifest` (alias `apply`) | `--archive` · `--manifest <JSON>` · `-o, --out <ZIP>` |

Inputs are never modified; `-o` must not already exist.

## `texture`

| Subcommand | Flags |
|---|---|
| `list` | `--game` · `--filter <TEXT>` |
| `extract` | `<ASSET>` · `--game` · `-o, --out <PNG>` |
| `replace` | `<ASSET>` · `--game` · `--image <PNG>` · `--mod-dir <DIR>` |
| `pack` | `--game` · `--mod-dir` · `--name <NAME>` · `-o, --out <DIR>` · `--compress` |
| `deploy` | `--game` · `--triplet-dir <DIR>` · `--name` |
| `index` | `--game` · `-o, --out <PATH>` |
| `undeploy` | `--game` · `--name` |
| `paklist` | `--game` · `--filter <TEXT>` · `--max <N>` · `--json` |

## `asset`

| Subcommand | Flags |
|---|---|
| `extract` | `--game` · `--asset </Game/...>` · `-o, --out <DIR>` · `--json` |
| `inspect` | `--uasset <FILE>` · `--usmap <FILE>` · `--export-index <N>` · `--json` |
| `patch-fixed` | `--uasset` · `--usmap` · `--extract-receipt <JSON>` · `--selector <JSON>` · `--expected-hex <HEX>` · `--replacement-hex <HEX>` · `-o, --out <FILE>` · `--json` |
| `pack` | `--game` · `--uasset` · `--patch-receipt <JSON>` · `--asset </Game/...>` · `--name <MOD>` · `-o, --out <DIR>` · `--json` |

Output directories must not exist and are never placed in the game tree.

## `as`

| Subcommand | Arguments and flags |
|---|---|
| `info <FILE>` | module count + `TAIL_OFF` |
| `decode-header <FILE>` | outer cache header |
| `walk <FILE>` | `--max <N>` (default 100) |
| `decompile <FILE> [NEEDLE]` | `--max <N>` (default 20) |
| `disasm <FILE> [NEEDLE]` | `--max <N>` (default 20) |
| `emit <FILE> [NEEDLE]` | `--max <N>` (default 5) |
| `emit-all <FILE> <OUTDIR>` | every module, mirroring `ScriptRelativeFilename` |
| `static-names <FILE> [INDICES]...` | no indices → count + first 10 |
| `default-sites <CACHE>` | `--module` · `--class` · `--field` · `--json` |
| `patch-default <CACHE>` | `--selector <JSON>` · `--expected-hex` · `--replacement-hex` · `-o, --out` · `--json` |
| `tag-map-sites <CACHE>` | `--module` · `--class` · `--field` · `--tag` · `--json` |
| `patch-tag-map <CACHE>` | `--selector <JSON>` · `--expected-hex` · `--replacement-hex` · `-o, --out` · `--json` |
| `qualify` | `--game` · `--usmap <FILE>` · `--catalog <JSON>` · `--id <ID>` · `--label <TEXT>` · `--json` |
| `diagnostics-check` | `--exe <EXE>` · `--game <GAME>` |
| `compile [SRC]` | `-o, --out` · `--game` · `--no-backup` · `--no-diagnostics` · `--diagnostics-hook <DLL>` · `--diagnostics-inject-delay-ms <MS>` |
| `compile-module` | `--op add\|edit` · `--module` · `--rel-path` · `--source` · `--work-dir` · `--allow-new-symbols` · `-o, --out` · `--game` · diagnostics flags |
| `replace <BASE> <MINI> <TARGET>` | `-o, --out` |
| `splice <BASE> <MINI>` | `-o, --out` |
| `extract <CACHE> <MODULE>` | `-o, --out` |
| `extract-remap <REGEN> <MODULE> <BASE>` | `--allow-new-symbols` · `-o, --out` |
| `bytediff <VANILLA> <REGEN>` | `--module` · `--func` · `--verdict` · `--show-benign` · `--context <N>` · `--norm-slots` · `--no-norm-scope` · `--no-norm-reguard` · `--json <PATH>` · `--fail-on-semantic` |

`patch-default`, `patch-tag-map` and `asset patch-fixed` never overwrite an
existing output path. The `as` extract/splice family — `replace`, `splice`,
`extract`, `extract-remap` — writes over whatever is at `-o`.

Every `as` subcommand that takes a cache file checks the `0x9e377abe`
module-cache magic before walking it, so pointing one at `Binds.Cache` or any
other side table names the format mismatch and the offending path rather than
failing somewhere inside the container parse. The same check guards
`catalog --kind knowledge --script-cache`, which feeds the caption extractor the
same walkers.

`tag-map-sites` and `patch-tag-map` additionally require exact bounded
`Binds.Cache` and `.usmap` evidence, discovered from the game layout or from
`GORE_AS_BINDS` / `GORE_AS_USMAP`. Missing, ambiguous, or mismatched evidence
fails closed.

## Data-model commands

| Command | Arguments and flags |
|---|---|
| `catalog <DUMP>` | `--kind item\|npc\|knowledge` · `--script-cache <CACHE>` · `-o, --out` |
| `story-catalog` | `--exe` · `--cache` · `--binds` · `-o, --out` |
| `location-catalog [SOURCE]` | `-o, --out` (SOURCE defaults to the resolved game install) |
| `location resolve <NAME>` | `--json` |
| `location list` | `--area` · `--prefix` · `--max` · `--json` |
| `dump <SDK_DIR>` | `-o, --out` |
| `stubs <MODEL>` | `-o, --out <DIR>` · `--filter <PREFIX>` |
| `gui-model` | `--model` · `--catalog` · `-o, --out` |
| `sync` | `--dump` · `--catalog` · `-o, --out` |
| `dump-mod` | `--model` · `--catalog` · `-o, --out <MODS_DIR>` |

## Mod-packaging commands

| Command | Arguments and flags |
|---|---|
| `scaffold <MOD_NAME>` | `-o, --out <MODS_DIR>` |
| `deploy-shared` | `--src <SRC>` · `--game <GAME>` |
| `package <MOD_DIR>` | `-o, --out <ZIP>` |

## `mcp`

Serves the whole CLI to an AI assistant. See [MCP server](mcp.md) for client
setup, the tool list, and how the guide is exposed.

| Subcommand | Arguments and flags |
|---|---|
| `serve` | `--allow-write` · `--allow-game-launch` · `--no-consent-prompts` · `--timeout-secs <SECS>` · `--max-output-kib <KIB>` |
| `tools` | — (prints the tool definitions as JSON and exits) |

`serve` speaks JSON-RPC on stdin/stdout; it is not interactive. Every command
that changes the installation or rewrites a file in place is confirmed with you
through your client before it runs, and `as compile` counts as both a launch and
a write. The two `--allow-*` flags answer in advance for unattended use;
`--no-consent-prompts` refuses instead of asking and cannot be combined with
them.

## `guide`

The whole guide is compiled into `gore.exe`. `guide html` writes it out as a
single browsable file — every page, its stylesheet and its script inlined, no
external requests — so it can be opened by double-click from anywhere. Only the
guide is rendered; the [reference](../reference/README.md) is embedded for the
[MCP server](mcp.md) but is not part of the browsable document.

| Subcommand | Arguments and flags |
|---|---|
| `html` | `-o, --out <PATH>` (default `guide.html`) · `--repo-ref <REF>` (default `main`) |

```powershell
gore guide html -o guide.html
```

The release zip already contains a rendered `docs\guide.html` beside the
Markdown pages; regenerating is only needed after editing the guide yourself.
`--repo-ref` pins the handful of links that leave the guide tree (component
READMEs, crates) to a commit on GitHub; the release build passes the exact
commit it was built from.

## Environment variables

| Variable | Used by |
|---|---|
| `GORE_AS_BINDS` | `as` decompile/emit native-call arities; tag-map evidence |
| `GORE_AS_USMAP` | `as tag-map-sites` / `patch-tag-map` evidence |
| `GORE_AS_DIAGNOSTICS_HOOK` | explicit trusted diagnostics helper for `as compile*` |
