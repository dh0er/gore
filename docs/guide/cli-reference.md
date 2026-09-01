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
| `doctor` | — | Diagnose the setup in one read-only pass: game path, install, UE4SS, enabled mods, deployment, leftovers, catalog staleness. | [getting-started](getting-started.md#check-the-setup) |
| `mcp` | `serve` · `tools` | Serve the whole CLI over the Model Context Protocol (stdio JSON-RPC) for AI assistants. | [mcp](mcp.md) |
| `guide` | `search` · `html` | Search this guide and the reference from a shell, or render the guide into one self-contained HTML file. | [below](#guide) |
| `find` | — | Search the bundled catalogs and the effect register: class names, ids, categories, display names, and what an id does in game. | [find](find.md) |
| `dialog` | `list` · `tree` · `show` · `text` · `new-topic` · `new-conversation` · `checkout` · `check` · `stage` · `export` | Inspect dialog trees and prepare checked structural/default edits or complete same-module conversations from the installed script cache. | [dialog-trees](dialog-trees.md) |
| `gen` | — | Compile `overrides.toml` → a UE4SS Lua override mod. | [items](items.md) |
| `mod` | `build` · `inspect` · `deploy` · `undeploy` | Build, validate, inspect, deploy, or undeploy a unified bundle. | [bundles](bundles.md) |
| `mgr` | `import` · `list` · `remove` · `enable` · `disable` · `order` · `analyze` · `preflight` · `recover` · `apply` · `status` · `reset` | Multi-mod manager: library, load order, readiness/recovery, conflicts, composed deployment, status and Reset. | [mod-manager](mod-manager.md) |
| `loc` | `extract` · `status` · `export` · `import` | Read/edit localized text & dialogs in the encrypted `.lcache`. | [text-and-dialogs](text-and-dialogs.md) |
| `audio` | `banks` · `list` · `extract` · `replace` · `restore` · `export-patch` · `apply-patch` | Read/replace FMOD `.bank` audio (PCM injection, `*.gore-bak`). | [audio](audio.md) |
| `voice` | `validate` · `list` (`index`) · `match-line` · `extract` · `add` · `replace` · `apply-manifest` (`apply`) | Validate/index/extract/copy-on-write edit voice-over ZIP archives. | [voice](voice.md) |
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

Commands that need the install (`deploy-shared`, `mod`, `mgr`, `texture`,
`asset`, `as`, `audio banks`, `loc`, `location-catalog`, `doctor`) resolve it
from an explicit `--game` (or `--lcache` / `--exe` / an explicit source path),
then the configured game path, then Steam auto-detect.

## `config`

| Subcommand | Arguments |
|---|---|
| `set <KEY> <VALUE>` | `KEY` = `game-path` (an install root or the `.exe`) |
| `get <KEY>` | prints the value, non-zero exit if unset |
| `unset <KEY>` | clears it |
| `list` | all values plus the resolved root and its source |
| `path` | path of `config.json` |
| `detect` | Steam auto-detect, saved as `game-path` |

## `doctor`

`gore doctor [--game <GAME>] [--json]`

| Flag | Meaning |
|---|---|
| `--game <GAME>` | Install root to diagnose. Without it, the configured game path, then Steam auto-detect. |
| `--json` | One JSON document instead of the human-readable report. |

Ten checks, one line each: `game_path`, `install`, `ue4ss`, `ue4ss_mods`,
`deployment`, `mods_folder`, `leftovers`, `game_process`,
`standalone_compiler`, `loc_catalog`. Every line that is not `ok` carries a
`fix:` line.

**Exit code 0 whether or not it found something** — a problem is a finding, not
a failure of the command, and this is the command people run when something has
already gone wrong. Gate on `--json` instead: the document carries `ok`, `note`,
`problem` and `skipped` counts at the top level, and every check carries its own
`verdict` plus a stable `id`. In `--json` the per-check `items` lists are
complete; the human report caps them at 12 and says how many more there are.

Read-only: it never writes, creates or removes anything. `deployment` reads and
hashes the files the deploy record claims (the same work as `gore mgr status`),
and `standalone_compiler` authenticates the compiler package and installed
compiler inputs. Those are the two broadest checks.

What the five checks worth knowing the limits of actually read:

- `ue4ss` keys off `UE4SS.dll` in `G1R\Binaries\Win64\ue4ss`. It reports whether
  the loader is installed, never whether it loaded — for that, read `UE4SS.log`
  (see [getting-started](getting-started.md#ue4ss)). `Mods\` on its own proves
  nothing: `gore mod deploy` creates that directory itself.
- `ue4ss_mods` treats a folder as a GORE override mod when its `Scripts\main.lua`
  opens with the `-- Generated by gore-cli` line `gore gen` and `gore mod build`
  write. Two of those enabled at once is reported as a problem: where both set
  one class default, whichever UE4SS loads last wins and nothing records which.
  Mods UE4SS itself ships are listed but never counted for that warning.
- `leftovers` asks the same probe `mod deploy` and game-backed AngelScript
  compilation consult before they refuse to start (install-mutation lock,
  compile lock, recovery journal, cache/JIT/proxy backups), plus a scan for
  `*.gore-bak` in the four directories this toolkit rewrites in place. The
  reported blocker count covers the lock/recovery artifacts only; separately
  listed backups are counted as backups. Strict `--backend standalone` does not
  enter this install-mutation window. A `*.gore-bak` is ordinary while a
  deployment is active; with no deploy record beside it, a rewritten file was
  probably never restored.
- `standalone_compiler` runs the product compiler's own read-only resolver. It
  authenticates the package beside the GORE executable and checks whether the
  installed Shipping ScriptCache and Binds match a qualified compiler API. It
  neither launches the game nor creates compiler scratch files. Compatibility
  is based on the cache/API, not a whole-file Steam/GOG executable checksum.
  A compatible authenticated package provides native file/line/column/severity
  diagnostics on the strict standalone compile path; Doctor does not run a
  compilation to manufacture a sample error.
- `loc_catalog` compares the byte count `gore loc status` recorded against the
  `.lcache` now installed. `gore loc import` re-encrypts that file in place, so a
  count that no longer agrees usually means the shared catalog describes text the
  install no longer carries.

Observed: a healthy install (the abridged report in
[getting-started](getting-started.md#check-the-setup) is a real run) and an
install root that does not exist, which yields one `problem` and nine `skipped`
lines instead of restating one cause nine times. The missing-UE4SS,
competing-override and leftover-lock verdicts are pinned by tests over fixture
trees, not by a run against a real broken install.

## `find`

`gore find [OPTIONS] <QUERY>...`

| Flag | Meaning |
|---|---|
| `<QUERY>...` | Words to search for. Several words are one query and **all** must match, so no quoting is needed. |
| `--domain <NAME>` | One id namespace: `item`, `npc`, `knowledge`, `texture`, `loc`, `audio`, `voice`, `asset`. An unknown one is refused with the list. |
| `--max <N>` | Stop after N hits (default 50). The result says how many matched. |
| `--json` | One JSON document instead of the human-readable blocks. |

Searches the item, NPC and knowledge catalogs compiled into `gore.exe` together
with the effect register, matching ids, categories, class paths, knowledge
captions and the register's own `effect` and `note` text. Every hit says which
layer it came from and, for register annotations, which provenance.

Display names are matched only when the shared text catalog is present. Every
result carries one line saying whether they were searched, and when they were
not, that `gore loc extract` is what fixes it — a name search that quietly
skipped its index would answer "no such item" about an item that is there. Full
detail in [Finding things](find.md).

**Exit code 0 whether or not anything matched.** An empty result is an answer,
and failing would bury the line explaining what was not searched.

## `dialog`

`gore dialog <list|tree|show|text|new-topic|new-conversation|checkout|check|stage|export> [OPTIONS]`

| Subcommand | Flags | Meaning |
|---|---|---|
| `list` | `[FILTER]` | The conversations the game ships: participants, topic count, module. `FILTER` keeps the ones whose participant or module contains it. |
| `tree` | `<NPC>` · `--lang` · `--depth <N>` · `--ids` | One NPC's complete dialog tree: options in menu order, their rules and `IsVisible` checks, the lines each side speaks, the effects, and nested sub-menus. |
| `show` | `<TOPIC>` · `--lang` | One topic class in full, with class names and localization keys. |
| `text` | `<NPC>` · `--lang` · `--out <FILE>` | That conversation's lines as a `gore loc import` edits document, each under the column the game actually reads. |
| `new-topic` | `<NPC>` · `--caption`/`--caption-key` · `--class` · `--subdialog-of <TOPIC>` · `--subdialog-position <N>` · `--priority-rank <N>` · `--mod-name` · `--out <DIR>` | Write a complete same-module edit workspace with the new class inserted into the conversation namespace. Without `--subdialog-of`, make a native direct root and automatically choose a normal rank before the recognized final End/Back row (or the current last root-rank group). With it, shift the named shipped parent's fixed 20-slot `Subdialog` entries and insert the child at optional 1-based position `N`; by default a trailing `TEXT_BACK`/Zurück entry stays last, otherwise the child appends. Sub-topics default to rank 0, so equal-rank slot order is preserved. An explicit rank wins exactly; `-1` intentionally requests the game's forced-topic behavior and is never chosen automatically. This is not an isolated `--op add` recipe and does not generate a UE4SS adapter. |
| `new-conversation` | `<NPC>` · `--caption`/`--caption-key` · `--class` · `--priority-rank <N>` · `--mod-name` · `--out <DIR>` | Start a complete conversation for an exact NPC with no root topics. The command requires that NPC's one already-loaded per-NPC conversation-settings module, preserves its settings class, and appends the private root plus first choice in the same module. The first choice defaults to rank 2; an explicit rank wins, and `-1` should be passed only for an intentionally forced topic. A partial/ambiguous NPC, existing rooted conversation, or missing/malformed settings anchor fails closed; there is no unreferenced Add-module fallback. Further new topics and new-to-new `Subdialog` levels stay in this source module. |
| `checkout` | `<NPC>` · `--out <DIR>` | The conversation module's compiler-ready AngelScript, including reconstructed class defaults, plus an untouched copy and a manifest bound to this game build. |
| `check` | `<DIR>` | Check method/default edits, complete default supersession, fixed shipped ABI and intentional new symbols. Partial or loss-prone defaults and consecutive actionless new-to-new `Subdialog` transitions fail closed. Offline; no compile. |
| `stage` | `<DIR>` · `--mod-name` · `--cache` · `--game` | Write the build spec and strict standalone `compile-module` command. It uses `--op edit`, adds `--allow-new-symbols` when required, includes the resolved `--game`, and refuses a cache hash that does not match that installation. |
| `export` | `--out <DIR>` | One JSON file per conversation. |

Every subcommand also takes `--cache <PATH>` to read an exact script cache and
`--game <ROOT>` to pick the install; without either, the configured game path is
used, then Steam auto-detect. `list`, `tree` and `show` take `--json`.

`--lang` takes a language family (`german`, `english`) and reads the newest
populated column of it, or an exact column name (`german_new`) to pin one. Text
comes from the shared catalog, so it needs `gore loc extract` once; without it
lines print as their localization keys and the output says so.

`new-topic`, `new-conversation`, `checkout` and `stage` write files and read the install; they
compile and deploy nothing. `check` is read-only. Existing topics may change
method bodies and the emitted `Caption`, `PriorityRank`, `Rules` and flag
defaults. If any existing class authors defaults, every shipped
default-bearing class and semantic target must remain covered. Otherwise the
byte-exact carry may preserve existing initializers while an appended class
authors new defaults under `--allow-new-symbols`. Matching `Binds.Cache` type
evidence and the existing shipped class/member/callable ABI remain mandatory.

A checked edit may append a new topic class at the end of the same existing
conversation module and wire it from an existing `Subdialog` body. New classes,
free functions and strings require `--allow-new-symbols`, which `stage` selects.
A new root from `new-topic` is instead a native direct topic in that same module.
Its `gore-dialog-edit.json` records the checked module, source/pristine paths,
participant and cache hash but omits `dialog_topics`; `stage` writes a
script-only spec with `meta` plus one `scripts` entry using `"op": "edit"`, and
prints a compile command containing `--allow-new-symbols`. The generated bundle
therefore has no automatic UE4SS component. Legacy workspaces may still carry a
low-level explicit `dialog_topics` row; `check` accepts it only when its new
class, participant and vanilla sentinel bind to the checked cache. The adapter's
`allow_hidden` field belongs to that low-level bundle schema, not to this CLI.

`new-conversation` is the same-module path for an NPC without root topics. Its
workspace edits the NPC's exact already-loaded conversation-settings module;
the shipped settings class, private root, first choice and every manually added
deeper topic stay together. On BuildID `24878692`, that shape opened
automatically for a previously dialogless shipped Guard, rendered two new
submenu choices, accepted one and returned control. A separate unreferenced Add
module was not discovered and is therefore refused. An all-new three-level tree
also ran end to end when an unconditional top-level `Say` separated consecutive
nested menu transitions; `check` rejects the actionless two-hop shape that
soft-locked. The generated bundle has no automatic UE4SS component.

Separate add/edit mini-caches cannot depend on each other, so an isolated
cross-module `--op add` is not the dialog recipe. Full detail, practical limits
and the separate compile/package/deploy/runtime proof boundaries are in
[Dialog authoring](dialog-authoring.md).

**Read-only apart from the files it is asked to write.** Nothing is deployed or
launched, and the tree describes
what the cache declares rather than what a given save would show. Full detail in
[Reading and editing dialog trees](dialog-trees.md).

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
| `build` | `--spec <SPEC>` (asset paths resolve against its directory) · `-o, --out <OUT>` (bundle goes to `<out>/<mod-name>`) · `--model <MODEL>` (validate field names/types; skipped if absent) |
| `inspect <BUNDLE>` | bundle directory or ZIP · `--json`; read-only full offline validation plus a bounded metadata/component/hash report |
| `deploy` | `--bundle <BUNDLE>` · `--game <GAME>` |
| `undeploy` | `--game <GAME>` |

## `mgr`

All subcommands except `reset` and `recover` accept `--library <DIR>` and
`--loadout <FILE>`. Supply either both overrides or neither; a lone override is
refused rather than being paired with unrelated default Manager state.

| Subcommand | Arguments |
|---|---|
| `import <PATH>` | source folder / `.zip` / game file |
| `list` | — |
| `remove <ID>` | library entry id; updates the target loadout only, so Apply afterwards if it was deployed |
| `enable <ID>` / `disable <ID>` | library entry id |
| `order <ID> <POS>` | `POS` is 0-based; 0 mounts first and loses conflicts |
| `analyze` | — |
| `preflight` | `--game <GAME>` · `--json`; read-only fixed setup/recovery checks |
| `recover` | `--game <GAME>` · `--expected-guard-id <TOKEN>` · `--yes` · `--json` (`--json` requires `--yes`) |
| `apply` | `--game <GAME>` |
| `status` | `--game <GAME>` · `--json` |
| `reset` | `--game <GAME>`; Manager-owned deployment only, refuses Studio ownership |

`recover` accepts only the exact current abandoned-Manager `action_token`
reported by `preflight`; it re-probes before mutation. See
[Running many mods](mod-manager.md#interrupted-manager-changes) for the consent,
JSON, and refusal contract.

## `loc`

| Subcommand | Flags |
|---|---|
| `extract` | `--lcache <PATH>` · `-y, --yes` |
| `status` | — |
| `export` | `--lcache <PATH>` · `-o, --out <OUT>` · `--keep-empty` |
| `import` | `--lcache <PATH>` · `--edits <EDITS>` · `-o, --out <OUT>` · `--add-missing` |

`--lcache` is optional on all three: it accepts a `.lcache`, a game dir or a
Steam library, and without it the configured game path and then Steam
auto-detect are tried. The installed cache is
`G1R\Story\Cache\AlkimiaLocalization_00000000.lcache`, with a generated suffix.

## `audio`

| Subcommand | Flags |
|---|---|
| `banks` | `--game <GAME>` · `--json` · `--key <KEY>`; lists the install's `.bank` files and each one's sample count |
| `list` | `--bank <BANK>` · `--filter <TEXT>` · `--max <N>` (default 100) · `--json` · `--key <KEY>` |
| `extract` | `--bank` · `-o, --out <DIR>` · `--sample <NAME\|all>` · `--filter <TEXT>` · `--key` |
| `replace` | `--map <MAP>` · `--bank` · `-o, --out <BANK>` · `--key` |
| `restore` | `--bank` |
| `export-patch` | `--map <MAP>` · `-o, --out <ZIP>` |
| `apply-patch` | `--patch <ZIP>` · `--bank` · `-o, --out <BANK>` · `--key` |

Without `-o`, `replace` and `apply-patch` overwrite the bank in place and back
it up to `*.gore-bak`.

## `voice`

| Subcommand | Flags |
|---|---|
| `validate` | `--ogg <OGG>` · `--json` |
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
| `emit <FILE> [NEEDLE]` | `--max <N>` (default 5) · `--no-defaults` (omit class `default` statements) |
| `emit-all <FILE> <OUTDIR>` | `--no-defaults` (omit class `default` statements); every module, mirroring `ScriptRelativeFilename` |
| `static-names <FILE> [INDICES]...` | no indices → count + first 10 |
| `default-sites <CACHE>` | `--module` · `--class` · `--field` · `--json` |
| `patch-default <CACHE>` | `--selector <JSON>` · `--expected-hex` · `--replacement-hex` · `-o, --out` · `--json` |
| `tag-map-sites <CACHE>` | `--module` · `--class` · `--field` · `--tag` · `--json` |
| `patch-tag-map <CACHE>` | `--selector <JSON>` · `--expected-hex` · `--replacement-hex` · `-o, --out` · `--json` |
| `qualify` | `--game` · `--usmap <FILE>` · `--catalog <JSON>` · `--id <ID>` · `--label <TEXT>` · `--json` |
| `diagnostics-check` | `--exe <EXE>` · `--game <GAME>` |
| `compile <SRC>` | `-o, --out` · `--work-dir <DIR>` · `--game` · `--backend standalone\|game\|standalone-then-game` (default `standalone-then-game`) · `--generation-receipt <RECEIPT.json>` · `--no-diagnostics` · `--diagnostics-hook <DLL>` · `--diagnostics-inject-delay-ms <MS>` |
| `compile-module` | `--op add\|edit` · `--module` · `--rel-path` · `--source` · `--work-dir` · `--allow-new-symbols` · `-o, --out` · `--game` · `--backend standalone\|game\|standalone-then-game` (default `standalone-then-game`) · `--generation-receipt <RECEIPT.json>` · diagnostics flags · five `--development-*` compiler-development overrides |
| `replace <BASE> <MINI> <TARGET>` | `-o, --out` |
| `splice <BASE> <MINI>` | `-o, --out` |
| `extract <CACHE> <MODULE>` | `-o, --out` |
| `extract-remap <REGEN> <MODULE> <BASE>` | `--allow-new-symbols` · `-o, --out` |
| `bytediff <VANILLA> <REGEN>` | `--module` · `--func` · `--verdict` · `--show-benign` · `--context <N>` · `--norm-slots` · `--no-norm-scope` · `--no-norm-reguard` · `--json <PATH>` · `--fail-on-semantic` |

`compile <SRC>` resolves the complete source graph, but publishes a selective
complete-cache product: only source-classified Add/Edit modules replace or join
the exact target cache, while untouched modules and every pre-existing global
tail record remain pristine; only records required by new symbols are appended.
Start from a current `emit-all` tree. Missing base sources request
Delete and are rejected; cyclic dependencies among new modules also fail
closed. The raw whole-tree compiler regeneration is never the published cache.
On BuildID `24878692`, one published selective product booted and loaded
gameplay, rendered and selected its new same-module root, and executed a new
provider call across modules from a shipped automatic topic. This runtime proof
does not turn two dependent module minis into a supported package shape.

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
through your client before it runs. `as compile` with explicit
`--backend standalone` is offline and needs no confirmation. The `game` and
`standalone-then-game` policies (including an omitted backend) count as both a
possible launch and an installation write. The two `--allow-*` flags answer in advance for unattended use;
`--no-consent-prompts` refuses instead of asking and cannot be combined with
them.

## `guide`

The whole guide is compiled into `gore.exe`, and so is the
[reference](../reference/README.md). `guide search` ranks single **sections** of
both against your words and prints page, anchor and a snippet, best first —
the same ranking the [MCP server](mcp.md)'s `gore_guide` tool serves, reachable
from a shell so it can be tried and argued with. `guide html` writes the guide
out as a single browsable file — every page, its stylesheet and its script
inlined, no external requests — so it can be opened by double-click from
anywhere. Only the guide is rendered; the reference is embedded but is not part
of the browsable document.

| Subcommand | Arguments and flags |
|---|---|
| `search <QUERY>` | `--limit <N>` (default 8, max 25) |
| `html` | `-o, --out <PATH>` (default `guide.html`) · `--repo-ref <REF>` (default `main`) |

```powershell
gore guide search "click sound music menu"
gore guide html -o guide.html
```

A handful of specific words beats a natural-language question; words every page
of a modding guide contains (`game`, `mod`, `file`) carry almost no signal.

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
