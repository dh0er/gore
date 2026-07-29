# Building GORE

How to build the CLI, the three Windows GUI apps, and how the repository is
laid out.

This page is for people working *on* GORE. If you only want to use it, you need
nothing from here — start with [Getting started](guide/getting-started.md).

## Requirements

- Windows 10 or newer.
- Rust toolchain (stable).
- Flutter with Windows desktop support — only for the GUI apps.
- Visual Studio 2022 with "Desktop development with C++".
- Python 3 — drives `build.py` and `test.py`.

The local development setup used for this repository is Flutter 3.44.0,
Dart 3.12.0, and Rust 1.96.0.

## The Rust workspace

The workspace spans every crate:

```powershell
cargo build
cargo test
cargo build --release -p gore   # just the CLI → target\release\gore.exe
```

## Products

Shippable artifacts are driven by the top-level orchestrator. The registered
projects are **`gore-cli`**, **`gore-save-editor`**, **`gore-mod-studio`** and
**`gore-mod-manager`**:

```powershell
python build.py <project> build      # debug build
python build.py <project> run        # build if missing, then launch
python build.py <project> dist       # release bundle (+ packaged zip)
python build.py <project> installer  # dist + Windows installer
python build.py <project> test       # run the project's test suite
```

`python build.py all test` runs every project's suite — Rust `cargo test`,
the Python tools, and Flutter `analyze` + `test`. CI runs the equivalent checks
via [`apps/save-editor/test.py`](../apps/save-editor/test.py) (invoked from
that directory) plus the mod-studio and mod-manager `analyze` + `test` steps.

Per-project build and run details live in each component's own README, e.g.
[`apps/save-editor/README.md`](../apps/save-editor/README.md).

`python build.py gore-cli dist` also stages the user guide into the CLI zip:
every `docs/guide/*.md` is copied to `docs/` beside `gore.exe`, and links that
leave the guide tree (component READMEs, crates, `docs/reference/`) are rewritten
to absolute `github.com/dh0er/gore` URLs so they still work offline. Those URLs
are pinned to the exact commit the zip was built from and use `/tree/` for
directories and `/blob/` for files, so a shipped link keeps resolving to the tree
it was written against. The staging is declared by the `doc_dirs` key in
`build.py` and runs in CI, which packages the release with the same command.

The same step then runs `gore guide html` with the freshly built binary to write
`docs/guide.html` into the zip — one self-contained browsable file, since Windows
has no handler for `.md`. Only `docs/guide/` is shipped and rendered.
`docs/reference/` and this page stay in the repository: `gore.exe` embeds the
reference pages so the MCP server can serve them, but they are contracts rather
than instructions and are not part of the user guide.

## Repository layout

```
gore/
├─ Cargo.toml              flat workspace (members = ["crates/*"])
├─ build.py                orchestrator: python build.py <project> build|run|dist|installer|test|release
├─ crates/
│  ├─ gore/                THE unified CLI binary (gore.exe)
│  ├─ gore-mcp/            MCP server (stdio JSON-RPC) + the embedded guide and reference
│  ├─ gore-reflect/        UE reflection model + UE4SS SDK dump parser
│  ├─ gore-catalog/        item/npc/knowledge catalog model + pipelines
│  ├─ gore-loc/            AlkimiaLocalization .lcache crypto + game-dir discovery + shared paths
│  ├─ gore-modgen/         overrides.toml → UE4SS Lua mod generation + validation
│  ├─ gore-mod/            unified bundle engine (overrides + loc + audio + voice + textures + scripts)
│  ├─ gore-fmod/           FMOD .bank decrypt/parse + Vorbis (audio backend, pure Rust)
│  ├─ gore-vo/             safe voice ZIP index/extract/copy-on-write editor
│  ├─ gore-asset/          USMAP + unversioned-property + lossless package primitives
│  ├─ gore-tex/            UE5 IoStore texture extract/replace (Zen .utoc/.ucas/.pak)
│  ├─ gore-authoring/      durable managed-project authoring primitives (Mod Studio)
│  ├─ gore-story-catalog/  generation-sealed NPC + quest-parent catalogs
│  ├─ gore-npc-catalog/    generation-sealed NPC archetype catalogs
│  ├─ gore-story-build/    deterministic, non-publishing story build plans
│  ├─ gore-story-inventory/ sealed base-game AngelScript collision inventories
│  ├─ gore-ffi/            cdylib dart:ffi bridge for the GUI apps (gore_ffi.dll)
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
└─ docs/                   this documentation
```

## Crates

| Crate | Kind | What it does |
|-------|------|--------------|
| [`gore`](../crates/gore) | Rust CLI (`gore.exe`) | The unified binary — see the [CLI reference](guide/cli-reference.md). |
| [`gore-mcp`](../crates/gore-mcp) | Rust lib | Model Context Protocol server over stdio JSON-RPC, and the `include_str!`-embedded copy of `docs/guide/` + `docs/reference/`. See the [MCP server](guide/mcp.md) page. |
| [`gore-reflect`](../crates/gore-reflect) | Rust lib | UE reflection model + UE4SS SDK dump parser. |
| [`gore-catalog`](../crates/gore-catalog) | Rust lib | Item/NPC/knowledge catalog model + generation pipelines. |
| [`gore-loc`](../crates/gore-loc) | Rust lib | AlkimiaLocalization `.lcache` crypto, game-dir discovery, shared paths. |
| [`gore-modgen`](../crates/gore-modgen) | Rust lib | `overrides.toml` → UE4SS Lua mod generation + field-level validation. |
| [`gore-mod`](../crates/gore-mod) | Rust lib | Unified bundle engine: `BuildSpec` → bundle (manifest + payloads) → deploy/undeploy. |
| [`gore-fmod`](../crates/gore-fmod) | Rust lib | FMOD `.bank` decrypt/parse + Vorbis decode (audio backend; pure Rust). |
| [`gore-vo`](../crates/gore-vo) | Rust lib | Safe voice ZIP indexing/extraction and verified copy-on-write Ogg add/replace. |
| [`gore-asset`](../crates/gore-asset) | Rust lib | USMAP flattening, bounded read-only complex-property spans, unversioned primitive codec, and verified `.uasset`/`.uexp` carrier. |
| [`gore-tex`](../crates/gore-tex) | Rust lib | IoStore texture extract/replace; cooks + packs a Zen triplet. Built on vendored [`retoc`](../vendor/retoc) + `gore-oodle`. |
| [`gore-authoring`](../crates/gore-authoring) | Rust lib | Durable, deployment-independent authoring primitives for managed Mod Studio projects. |
| [`gore-story-catalog`](../crates/gore-story-catalog) | Rust lib | Strict, generation-sealed NPC and quest-parent catalogs (`story_catalog.v1`). |
| [`gore-npc-catalog`](../crates/gore-npc-catalog) | Rust lib | Generation-sealed NPC archetype catalogs and their structural linkage. |
| [`gore-story-build`](../crates/gore-story-build) | Rust lib | Deterministic, non-publishing build plans over revision-3 story content. |
| [`gore-story-inventory`](../crates/gore-story-inventory) | Rust lib | Sealed base-game AngelScript collision inventories bound to one game generation. |
| [`gore-ffi`](../crates/gore-ffi) | Rust cdylib | `dart:ffi` bridge for the GUI apps (`gore_ffi.dll`) over the full mod engine. |
| [`gore-save`](../crates/gore-save) | Rust lib + cdylib | GSAV savegame parse/edit core (`gore_save.dll`). |
| [`gore-oodle`](../crates/gore-oodle) | Rust lib | Oodle/Kraken codec (pure Rust; no proprietary `oo2core` DLL). |
| [`gore-as`](../crates/gore-as) | Rust lib | AngelScript precompiled-cache decoder/emitter/decompiler/splicer. |

The Flutter GUIs reuse the exact same Rust crates as the CLI through the
`dart:ffi` bridge, so the CLI is always the most complete surface.

## Versioning

Per-product independent semver, with prefixed release tags driving the Release
workflow. A project name is the whole naming scheme — it is the tag prefix and
the artifact name:

| Project | Tag | Portable zip | Installer |
|---|---|---|---|
| `gore-cli` | `gore-cli-v*` | `gore-cli-X.Y.Z-windows-x64.zip` | — |
| `gore-save-editor` | `gore-save-editor-v*` | `gore-save-editor-X.Y.Z-windows-x64.zip` | `gore-save-editor-X.Y.Z-setup.exe` |
| `gore-mod-studio` | `gore-mod-studio-v*` | `gore-mod-studio-X.Y.Z-windows-x64.zip` | `gore-mod-studio-X.Y.Z-setup.exe` |
| `gore-mod-manager` | `gore-mod-manager-v*` | `gore-mod-manager-X.Y.Z-windows-x64.zip` | `gore-mod-manager-X.Y.Z-setup.exe` |

Both artifact names are derived from the project in `build.py`
(`zip_basename` / `installer_basename`), and the installer stem is handed to
Inno Setup as `/DOutputBaseName`, so the three cannot drift apart.

### Updater feeds

Each GUI app checks for updates through a WinSparkle appcast. All three read a
**fixed-tag feed release** whose single `appcast-windows.xml` asset CI
overwrites on every release:

| App | Feed release |
|---|---|
| Save Editor | `gore-save-editor-appcast` |
| Mod Studio | `gore-mod-studio-appcast` |
| Mod Manager | `gore-mod-manager-appcast` |

These tags are compiled into the apps, so renaming one requires shipping a new
build of that app first.

`gore-save-editor` additionally publishes with `make_latest=true`. That decides
only which release people land on at `/releases/latest`; no updater depends on
it, so the flag can move to another product when the flagship changes. The one
exception is historical: Save Editor builds up to 1.2.0 read their appcast from
`releases/latest/download/`, so the flag has to stay put until those are gone.
Both paths currently serve the identical asset.

Internal libraries share one workspace version.

## Documentation checks

Relative links across the README, `docs/`, and the component READMEs are
verified by:

```powershell
python scripts/check_docs_links.py
```

It is a standalone check and is deliberately not wired into
`python build.py all test`.
