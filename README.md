# GORE

**GORE** (Go-thic Re-make) is a modding and save-editing toolsuite for the
Gothic Remake.

## What's in the repo

- **[`gore`](crates/gore) CLI** (`gore.exe`) — the one binary that does *all*
  modding from the terminal: item values, text/dialogs, audio, textures,
  scripts. Start here if you want to mod. **⚗️ Ready for experimental use.**
- **[`mod-studio`](apps/mod-studio)** — a no-code Windows GUI over the same
  modding engine (incl. experimental AngelScript editing), for *authoring* one
  mod. **🚧 Work in progress.**
- **[`mod-manager`](apps/mod-manager)** — a Windows GUI for installing and
  managing *many* mods together (load order, conflict detection, one-click
  apply); complements mod-studio (authoring). **🚧 Work in progress.**
- **[`save-editor`](apps/save-editor)** — a Windows GUI for editing your save
  files (a separate job from modding — it never touches the game install).
  **✅ Ready to use.**
- **[`gore-lua`](lua)** — a small Lua helper library that ships into the game,
  for hand-writing UE4SS mods. **🚧 Work in progress.**
- **Supporting Rust crates** — the format codecs and data models the above
  share (see [Projects / layout](#projects--layout)), plus example mods under
  [`mods/`](mods).

The Flutter GUIs reuse the exact same Rust crates as the CLI through a
`dart:ffi` bridge — the CLI is always the most complete surface.

---

# Modding with the `gore` CLI

This is the full how-to-mod guide. Every domain below is driven entirely from
the `gore` binary — no GUI required.

Get the binary by building it (`cargo build --release -p gore` →
`target/release/gore`) or downloading a `gore-cli-v*`
[release](https://github.com/dh0er/gore/releases). Examples below assume
`gore` is on your `PATH` and use `"$GAME"` for your install root (the folder
that contains `G1R/`). The exact `--game`/path each subcommand expects is always
in `gore <cmd> --help`.

Set the game path once and skip `--game` everywhere:

```sh
gore config set game-path "$GAME"   # an install root or the game .exe
gore config detect                  # …or auto-detect a Steam install & save it
gore config list                    # show the stored value + resolved root
```

Every command that needs the game (`deploy-shared`, `mod`, `mgr`, `texture`,
`loc`) then resolves it automatically. Precedence is an explicit `--game` (or
`--lcache` for `loc`) > the configured path > Steam auto-detect. The path is
stored in a shared `config.json` (`gore config path`) that the GUI apps read
too, so you configure the install in one place.

Each domain produces a mod a different way:

| Domain | Mechanism | Touches |
|--------|-----------|---------|
| Item/stat values | UE4SS Lua CDO override, applied at runtime | a new mod folder under `ue4ss/Mods/` |
| Text & dialogs | re-encrypted `.lcache` | the localization cache, in place |
| Audio | re-packed FMOD `.bank` | the sound bank, in place |
| Textures | additive UE5 IoStore Zen triplet | the game's `~mods/` folder |
| Scripts | edited precompiled AngelScript cache | the script cache (experimental) |

You can ship each on its own, or combine them into one deployable
**bundle** (see [Bundling & deploying](#bundling--deploying)).

## Item & stat values (overrides)

Change any default value on an item/NPC/ability class — weapon damage, item
value, weight, etc. Write an `overrides.toml`:

```toml
[meta]
name = "MyBalanceMod"
delay_ms = 0            # 0 = apply on first tick; >0 = apply after N ms

[[override]]
class = "ItFo_Apple"    # AngelScript class name
field = "m_Value"
value_int = 500

[[override]]
class = "ItMw_1H_Sword_01"
field = "m_Weight"
value_float = 1.5
```

Compile it into a runtime UE4SS Lua mod, written straight into the game:

```sh
gore gen overrides.toml -o "$GAME/G1R/Binaries/Win64/ue4ss/Mods"
# optional: validate field names/types against a reflection model first
gore gen overrides.toml -o "$GAME/.../Mods" --model model.json
```

The emitted mod looks up each class's CDO
(`StaticFindObject("/Script/<module>.Default__<class>")`) and sets the field at
load. It is self-contained — it does **not** need the gore-lua helpers.

**Finding class & field names.** Item/NPC/knowledge classes are listed in the
bundled catalogs (`apps/save-editor/assets/*_catalog.json`). To (re)generate the
data model yourself from a UE4SS dump:

```sh
gore catalog --kind item "UE4SS_ObjectDump.txt" -o item_catalog.json   # also: npc, knowledge
gore dump   "CXXHeaderDump/" -o model.json        # field schema (types) for validation
gore stubs  model.json -o stubs/                  # optional LuaLS/EmmyLua type stubs
```

For the *real* in-game default values (so the GUI/editor shows accurate
numbers), run the `gore-dump` mod in-game, then fold its output back in:

```sh
gore dump-mod --model model.json --catalog item_catalog.json -o "$GAME/.../Mods"  # generate the mod
# …launch the game once with it enabled → writes gore_game_data.json…
gore sync --dump gore_game_data.json --catalog item_catalog.json -o model.json
```

## Text & dialogs (localization)

All UI text and NPC dialog lines live in the encrypted AlkimiaLocalization
`.lcache`. Decrypt every language to JSON, edit, re-encrypt:

```sh
gore loc export --lcache "$GAME/.../AlkimiaLocalization_Game.lcache" -o loc.json
# edit loc.json:  { "some_text_id": { "german": "Neuer Text", "english": "New text" }, … }
gore loc import --lcache "$GAME/.../AlkimiaLocalization_Game.lcache" --edits loc.json
```

`gore loc import` overwrites the cache in place (pass `-o` to write elsewhere) —
keep a copy first, or use the [bundle](#bundling--deploying) path, which backs up
to `*.gore-bak`. Helpers: `gore loc extract` auto-detects the game and writes the
shared catalog; `gore loc status` shows what's loaded.

## Audio

The game's sounds and music are encrypted FMOD `.bank` files at
`$GAME/G1R/Content/FMOD/Desktop/*.bank`. `gore audio` reads and replaces samples
in pure Rust (no FMOD install needed).

```sh
gore audio list    --bank "$GAME/.../SFX.bank"               # name, codec, rate, channels, length
gore audio extract --bank "$GAME/.../SFX.bank" -o wavs/      # all samples to .wav (or --sample NAME)
# map.json:  { "SampleName": "path/to/new.wav", … }
gore audio replace --map map.json --bank "$GAME/.../SFX.bank"   # in place, *.gore-bak backup
gore audio restore --bank "$GAME/.../SFX.bank"                  # undo from *.gore-bak
```

Replacement re-encodes your WAV as PCM16 in an appended sub-bank and repoints the
sample — any length, no whole-bank re-encode. Share a patch without shipping game
audio:

```sh
gore audio export-patch --map map.json -o patch.zip
gore audio apply-patch  --patch patch.zip --bank "$GAME/.../SFX.bank"
```

## Textures

Replace any `Texture2D` packed in the UE5 IoStore container. Output is an
**additive** Zen triplet (`.utoc`/`.ucas`/`.pak`) dropped into the game's
`~mods/` folder — no original game file is modified.

```sh
gore texture list    --game "$GAME" --filter T_Hardware           # find asset paths
gore texture extract --game "$GAME" /Game/UI/Textures/Common/T_HardwareCursor -o cur.png
# edit cur.png (RGBA8/RGB8; dimensions need not match the original)
gore texture replace --game "$GAME" /Game/UI/Textures/Common/T_HardwareCursor \
                     --image new.png --mod-dir moddir/
gore texture pack    --game "$GAME" --mod-dir moddir/ --name zzz_MyMod_P -o out/
gore texture deploy  --game "$GAME" --triplet-dir out/ --name zzz_MyMod_P
gore texture undeploy --game "$GAME" --name zzz_MyMod_P            # remove it
```

(`gore texture index` builds/caches the asset→package-id map used to resolve
assets. Compression is opt-in via `--compress` on `pack`, but uncompressed
containers are what currently load reliably in-game.)

## Scripts (AngelScript) — experimental

The game's compiled AngelScript lives in a precompiled cache
(`PrecompiledScript_Shipping.Cache`). `gore as` can read it and splice edited
modules back in. This is reverse-engineering-stage tooling; a compiled module
can also be folded into a deployable bundle (see
[Bundling & deploying](#bundling--deploying)).

```sh
gore as info       PrecompiledScript_Shipping.Cache         # module count + splice point
gore as decompile  PrecompiledScript_Shipping.Cache <needle>   # → readable AngelScript
gore as emit-all   PrecompiledScript_Shipping.Cache out_as/    # all modules as recompilable .as
gore as disasm     PrecompiledScript_Shipping.Cache <needle>   # asBC bytecode listing
```

### Recompiling: the game is the compiler

There is no standalone AngelScript compiler — the shipping game **is** the
compiler. Its executable takes a command-line flag,
**`-as-generate-precompiled-data`**, which makes it read the loose `.as` scripts
under `<install>/G1R/Script/`, compile them, and (over)write
`PrecompiledScript_Shipping.Cache` in that same folder.

`gore as compile` wraps that flag as an ordinary compiler: give it a source tree
and (optionally) an output path, and it does the file juggling — backup, staging
into `Script/`, launching the game, then restoring the install — itself. The
install is resolved from `--game`, else the configured game path / Steam
auto-detect.

```sh
# dump the vanilla modules as an editable .as tree:
gore as emit-all "$GAME/G1R/Script/PrecompiledScript_Shipping.Cache" out_as/
# …edit modules in out_as/ …

# compile the tree to a cache file, leaving the install untouched:
gore as compile out_as/ -o regen.Cache --game "$GAME"
# …or install the fresh cache in place (previous one saved to *.gore-bak):
gore as compile out_as/ --game "$GAME"
# with no source tree, recompile whatever `.as` are already in Script/:
gore as compile --game "$GAME"
```

The `-o` form leaves the install exactly as it was, so the live
`PrecompiledScript_Shipping.Cache` is still the pristine `vanilla.Cache` below.
Rather than shipping the whole regenerated cache, splice just your edited module
back into the vanilla one:

```sh
# existing module — remap refs to the vanilla cache, then replace in place:
gore as extract-remap regen.Cache <Module> vanilla.Cache -o mini.Cache
gore as replace       vanilla.Cache mini.Cache <Module>  -o modded.Cache
# new primitive-only module — splice directly:
gore as splice        vanilla.Cache mini.Cache -o modded.Cache
```

Decompilation/emit resolve native-call arities from a `Binds.Cache` placed next
to the input cache (or `GORE_AS_BINDS`).

## Bundling & deploying

Combine overrides + text + audio + textures + scripts into one mod, then
deploy/undeploy it against your install. Write a build spec (`spec.json`):

```json
{
  "meta": { "name": "MyMod", "version": "1.0.0", "author": "you" },
  "overrides": [ { "class": "ItFo_Apple", "field": "m_Value", "value_int": 500 } ],
  "loc_edits": { "some_text_id": { "german": "…" } },
  "audio":   [ { "bank": "SFX.bank", "sample": "Foo", "wav_path": "foo.wav" } ],
  "texture": [ { "asset": "/Game/UI/.../T_Foo", "image_path": "foo.png" } ],
  "scripts": [ { "op": "add", "module_name": "MyModule", "mini_cache": "MyModule.cache" } ]
}
```

```sh
gore mod build   --spec spec.json -o build/            # → build/MyMod/ (gore-mod.json manifest + payloads)
gore mod deploy  --bundle build/MyMod --game "$GAME"   # overrides→Mods, loc/audio in place(*.gore-bak), textures→~mods
gore mod undeploy --game "$GAME"                       # restore everything
```

This is the same engine [`mod-studio`](#gore-mod-studio) drives. Other helpers:

```sh
gore scaffold MyMod -o "$GAME/.../Mods"   # empty hand-written gore-lua mod skeleton
gore deploy-shared --game "$GAME"         # install the gore-lua helpers (for custom Lua mods)
gore package mod_dir/ -o MyMod.zip        # zip a Lua mod for sharing
```

## CLI reference

Every subcommand of the `gore` binary:

| Command | Action(s) | Purpose |
|---------|-----------|---------|
| `config` | `set` · `get` · `unset` · `list` · `path` · `detect` | Persist shared settings (the game path) so other commands can omit `--game`. |
| `gen` | — | Compile `overrides.toml` → a UE4SS Lua override mod. |
| `mod` | `build` · `deploy` · `undeploy` | Build/deploy/undeploy a unified bundle (overrides + loc + audio + textures + scripts). |
| `mgr` | `import` · `list` · `enable` · `disable` · `order` · `analyze` · `apply` · `status` · `reset` · `remove` | Multi-mod manager: library, load order, conflict analysis, composed deploy (the CLI behind mod-manager). |
| `loc` | `extract` · `status` · `export` · `import` | Read/edit localized text & dialogs in the encrypted `.lcache`. |
| `audio` | `list` · `extract` · `replace` · `restore` · `export-patch` · `apply-patch` | Read/replace FMOD `.bank` audio (PCM injection, `*.gore-bak`). |
| `texture` | `list` · `extract` · `replace` · `pack` · `deploy` · `index` · `undeploy` | Extract/replace IoStore textures → Zen triplet in `~mods`. |
| `as` | `compile` · `info` · `decode-header` · `walk` · `decompile` · `disasm` · `emit` · `emit-all` · `replace` · `splice` · `extract` · `extract-remap` | AngelScript precompiled-cache tooling: recompile `.as` via the game, decode/emit/decompile/splice modules (experimental). |
| `catalog` | `--kind item\|npc\|knowledge` | Generate a catalog JSON from a UE4SS object dump. |
| `dump` | — | Parse a UE4SS SDK header dump into a reflection model JSON. |
| `stubs` | — | Emit LuaLS/EmmyLua type stubs from `model.json`. |
| `gui-model` · `sync` · `dump-mod` | — | Build/refresh the data model (incl. real in-game defaults via the `gore-dump` mod). |
| `scaffold` | — | Create a hand-written gore-lua mod skeleton. |
| `deploy-shared` | — | Install the gore-lua helpers into `ue4ss/Mods/shared`. |
| `package` | — | Zip a mod folder into distributable UE4SS layout. |

---

# GORE Mod Studio 🚧

> **Work in progress** — not yet ready for general use. For stable modding today,
> use the [`gore` CLI](#modding-with-the-gore-cli).

A no-code Windows app over the same bundle engine as the CLI — point-and-click
modding with live previews and `.goremod` project files. Auto-updates on launch
(WinSparkle). The install bundles the [`gore` CLI](#modding-with-the-gore-cli)
(`gore.exe`) alongside, for the power tools the GUI does not surface (AngelScript
`disasm`/`decompile`, `catalog`/`dump`/`stubs`, multi-mod `mgr`).

**It can:**
- Edit **item/stat values** by browsing the categorized item catalog and editing
  fields (the override domain).
- Edit **localized text & dialogs**.
- Replace **audio** — browse a bank's samples, preview, and swap in your own.
- Replace **textures** — pick an asset, preview, drop in a PNG.
- Edit **AngelScript** — stage a module, compile, and splice it into the game's
  script cache (experimental).
- **Build a bundle** and **deploy/undeploy** it to your game install
  (overrides + loc + audio + textures + scripts, with backups), or **export a
  standalone Lua override mod** to share.

**It can not:**
- Edit **save files** — that's [save-editor](#gore-save-editor).
- Hand-write custom Lua logic — use `gore scaffold` + the [gore-lua helpers](#gore-lua-helper-library).
- Patch arbitrary game files outside the five supported domains.
- Manage a *collection* of mods together — that's [mod-manager](#gore-mod-manager).

---

# GORE Mod Manager 🚧

> **Work in progress** — not yet ready for general use. For stable modding today,
> use the [`gore` CLI](#modding-with-the-gore-cli).

A Windows app for running **many** mods at once. Where [mod-studio](#gore-mod-studio)
*authors* a single mod, mod-manager owns the multi-mod story: build a library,
order it, see what collides, and apply the whole enabled set to your install.
Auto-updates on launch (WinSparkle). It consumes the mod bundles mod-studio (or
`gore mod build`) produces, plus foreign mods it did not build.

**It can:**
- **Import** into a local library: built mod-bundle folders/zips (with a root
  `gore-mod.json`), foreign mod zips/folders, loose `_P.pak` files, IoStore
  triplets (`.utoc`/`.ucas`/`.pak`), UE4SS Lua mod folders, and raw game-file
  replacements.
- **Enable/disable** mods and **drag to reorder** the load order (later wins).
- **Detect conflicts** across mods — localization, audio, texture/asset, item
  overrides (CDO), scripts, and raw-file replacements — and show which mod wins.
- **Apply** declaratively: full-recompute the modded state from a pristine base
  and deploy the whole enabled set (backups first), or **undeploy all** to
  restore.
- **Take over** a mod-studio test-deploy so both tools do not fight over the
  install.

**It can not:**
- *Author* a mod (edit item values, text, audio, textures) — that's
  [mod-studio](#gore-mod-studio) or the [`gore` CLI](#modding-with-the-gore-cli).
- Edit **save files** — that's [save-editor](#gore-save-editor).
- Download mods (no Nexus API integration) — import files you already have.

---

# GORE Save Editor

A Windows app for editing your **save games**, backup-first. This is
*not* modding — it changes your saved progress, never the game install.
Auto-updates on launch (WinSparkle). Tested against Steam build CL168781; should
work across versions.

[<img src="docs/images/screenshot_light.png" alt="GORE Save Editor" width="600"/>](docs/images/screenshot_light.png)

**It can:**
- **Profile** — change difficulty settings; set the in-game time (play clock).
- **Player & NPCs** — edit stats, attributes, all talents/skills, location, and
  more; revive a dead NPC (restore health, strip the death state).
- **Inventory** — change counts of existing items; add new items from a bundled,
  categorized catalog; reset an
  inventory to a clean starting state.
- **Faction crimes** — clear an NPC's crimes to reset the hostility its guild
  holds against you.
- **Progression** — edit quest markers, NPC knowledge, and events.
- **Raw properties** — edit almost any internal property value directly
  (experimental; can corrupt a save).
- Write an automatic **backup** before every change.

**It can not:**
- Mod the game (no overrides, audio, textures, or scripts) — use the
  [`gore` CLI](#modding-with-the-gore-cli) or [mod-studio](#gore-mod-studio).
- Touch the game install in any way.
- Guarantee experimental raw edits are safe — keep your own copy of important
  saves.

---

# GORE Lua Helper Library 🚧

> **Work in progress** — a thin convenience layer, not a full SDK.

[`gore-lua`](lua) is a small shared UE4SS **helper library** for hand-writing
Gothic Remake mods in Lua — the path for behavior the override generator can't
express (hooks, keybinds, console commands, live attribute tweaks). It's a
handful of `pcall`-guarded wrappers over UE4SS reflection, not a large API.
Override mods produced by `gore gen`/`mod-studio` do **not** use it; it's for
custom mods.

How a mod uses it:

```sh
gore deploy-shared --game "$GAME"             # copy gore-lua into ue4ss/Mods/shared (once)
gore scaffold MyMod -o "$GAME/.../Mods"       # new mod with the loader wired in
```

```lua
local gore = require("gore-lua")           -- load the shared gore-lua helpers
gore.cheat.god(true)                       -- toggle god mode on the live CombatConfig + CDOs
gore.gas.heal()                            -- set Health to MaxHealth
gore.ui.text("hello from my mod")          -- on-screen message via the game's HUD
gore.cmd.command("mycmd", function() … end)-- register a console command
```

The API spans `gore.obj` (objects/CDOs/properties), `gore.player`
(controller/pawn/world), `gore.ui` (on-screen text + log), `gore.gas` (gameplay
attributes), `gore.cheat`, `gore.cmd` (commands/keybinds/game-thread), and
`gore.help`/`gore.selftest`. Every helper pcall-guards its reflection and returns
`nil`/`false` on failure — it never crashes the consuming mod. Full reference:
[`lua/README.md`](lua/README.md) (or `gore.help.list()` at runtime).

`gore deploy-shared` installs the helpers; UE4SS loads any mod folder containing
an `enabled.txt`.

---

# Projects / layout

```
gore/
├─ Cargo.toml              flat workspace (members = ["crates/*"])
├─ build.py                orchestrator: python build.py <project> build|run|dist|installer|test|release
├─ crates/
│  ├─ gore/                THE unified CLI binary (gore.exe)
│  ├─ gore-reflect/        UE reflection model + UE4SS SDK dump parser
│  ├─ gore-catalog/        item/npc/knowledge catalog model + pipelines
│  ├─ gore-loc/            AlkimiaLocalization .lcache crypto + game-dir discovery + shared paths
│  ├─ gore-modgen/         overrides.toml → UE4SS Lua mod generation + validation
│  ├─ gore-mod/            unified mod-bundle engine (overrides + loc + audio + textures)
│  ├─ gore-fmod/           FMOD .bank decrypt/parse + Vorbis (audio backend, pure Rust)
│  ├─ gore-tex/            UE5 IoStore texture extract/replace (Zen .utoc/.ucas/.pak)
│  ├─ gore-ffi/            cdylib dart:ffi bridge for mod-studio (gore_ffi.dll)
│  ├─ gore-save/           GSAV savegame parse/edit core + its cdylib (gore_save.dll)
│  ├─ gore-oodle/          Oodle/Kraken codec (pure Rust, no oo2core DLL)
│  └─ gore-as/             AngelScript precompiled-cache decoder/emitter/splicer (surfaced via `gore as`)
├─ apps/
│  ├─ save-editor/         Flutter (Windows) savegame editor — WinSparkle auto-update
│  ├─ mod-studio/          Flutter (Windows) no-code mod authoring GUI
│  └─ mod-manager/         Flutter (Windows) multi-mod library/load-order/apply GUI
├─ lua/                    gore-lua UE4SS helper library (deployed into the game's Mods/shared)
├─ mods/                   first-party UE4SS mod folders
│  ├─ example/             sample mod using gore-lua
│  └─ gore-dump/           generated dump mod (regen: `gore dump-mod`)
├─ vendor/
│  └─ retoc/               vendored IoStore reader fork (Oodle decode routed to gore-oodle)
├─ scripts/                release helpers (appcast.py — WinSparkle appcast generator)
└─ docs/
```

| Crate | Kind | What it does |
|-------|------|--------------|
| [`gore`](crates/gore) | Rust CLI (`gore.exe`) | The unified binary — see [CLI reference](#cli-reference). |
| [`gore-reflect`](crates/gore-reflect) | Rust lib | UE reflection model + UE4SS SDK dump parser. |
| [`gore-catalog`](crates/gore-catalog) | Rust lib | Item/NPC/knowledge catalog model + generation pipelines. |
| [`gore-loc`](crates/gore-loc) | Rust lib | AlkimiaLocalization `.lcache` crypto, game-dir discovery, shared paths. |
| [`gore-modgen`](crates/gore-modgen) | Rust lib | `overrides.toml` → UE4SS Lua mod generation + field-level validation. |
| [`gore-mod`](crates/gore-mod) | Rust lib | Unified bundle engine: `BuildSpec` → bundle (manifest + payloads) → deploy/undeploy. |
| [`gore-fmod`](crates/gore-fmod) | Rust lib | FMOD `.bank` decrypt/parse + Vorbis decode (audio backend; pure Rust). |
| [`gore-tex`](crates/gore-tex) | Rust lib | IoStore texture extract/replace; cooks + packs a Zen triplet. Built on vendored [`retoc`](vendor/retoc) + `gore-oodle`. |
| [`gore-ffi`](crates/gore-ffi) | Rust cdylib | `dart:ffi` bridge for mod-studio (`gore_ffi.dll`) over the full mod engine. |
| [`gore-save`](crates/gore-save) | Rust lib + cdylib | GSAV savegame parse/edit core (`gore_save.dll`). |
| [`gore-oodle`](crates/gore-oodle) | Rust lib | Oodle/Kraken codec (pure Rust; no proprietary `oo2core` DLL). |
| [`gore-as`](crates/gore-as) | Rust lib | AngelScript precompiled-cache decoder/emitter/decompiler/splicer. |

# Build

## Requirements

- Windows 10 or newer.
- Rust toolchain (stable).
- Flutter with Windows desktop support (for the GUI apps).
- Visual Studio 2022 with "Desktop development with C++".
- Python 3 (drives `build.py` / `test.py`).

The local development setup used for this repository is Flutter 3.44.0, Dart
3.12.0, and Rust 1.96.0.

The Rust workspace spans every crate:

```sh
cargo build
cargo test
```

Products (apps and shippable artifacts) are driven by the top-level orchestrator.
The registered projects are **`gore-save`** (save-editor), **`gore-mod`**
(mod-studio), **`gore-manager`** (mod-manager), and **`gore`** (the CLI):

```sh
python build.py <project> build      # debug build
python build.py <project> run        # build if missing, then launch
python build.py <project> dist       # release bundle (+ packaged zip)
python build.py <project> installer  # dist + Windows installer
python build.py <project> test       # run the project's test suite
```

`python build.py all test` runs every project's suite (Rust `cargo test`, Python
tools, Flutter `analyze` + `test`). CI runs the equivalent checks via
[`apps/save-editor/test.py`](apps/save-editor/test.py) (invoked from that
directory) plus the mod-studio/mod-manager `analyze` + `test` steps. Per-project
build/run details live in each component's own README (e.g.
[`apps/save-editor/README.md`](apps/save-editor/README.md)).

## Versioning

Per-product independent semver, with prefixed release tags driving the Release
workflow:

- `save-editor` → `gore-save-v*` (publishes with `make_latest=true`; its updater
  polls `releases/latest`)
- `mod-studio` → `gore-mod-v*` (updater reads a fixed `gore-mod-appcast` release)
- `mod-manager` → `gore-manager-v*` (updater reads a fixed `gore-manager-appcast`
  release)
- the CLI → `gore-cli-v*`

Internal libraries share one workspace version.

# License

MIT. See [LICENSE](LICENSE).
