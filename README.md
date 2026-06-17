# gore-tools

A monorepo for Gothic 1 Remake modding and editing tooling. Each tool is a
project under `projects/`, prefixed `gore-`. Shared code lives in its own
`gore-*` project.

## Projects

| Project | Kind | What it does |
|---------|------|--------------|
| [`gore-save`](projects/gore-save) | Flutter app + Rust core | Savegame editor — edit player, inventory, progression, difficulty in GSAV saves. Backup-first. (Formerly the standalone `goresave` repo.) |
| `gore-core` | Rust lib + pipelines | Shared reflection/catalog model + the catalog generation pipelines (item/npc/knowledge) that both modding front-ends build on. |
| `gore-cli` *(planned)* | Rust CLI | Programmer-facing: generate Lua type stubs, catalogs, mod scaffolding; compile declarative override configs into UE4SS Lua mods; package mods. |
| `gore-mod` *(planned)* | Flutter app | Designer/no-code GUI: browse the catalog, edit values, export a ready-to-use UE4SS Lua mod — never touching Lua. |

## The mod artifact

The modding tools (`gore-cli`, `gore-mod`) all produce the same thing: a
**UE4SS Lua mod folder** the player drops into
`<game>/Binaries/Win64/ue4ss/Mods/`. The Lua applies **CDO overrides** (and
optionally hooks) at game load — e.g. set an item's value via
`StaticFindObject("/Script/Angelscript.Default__<Class>")` then
`cdo.m_Value = ...`. The tools are authoring front-ends; they do not modify
game files directly — the produced mod does, at runtime.

`gore-save` is a different axis: it edits **save files**, not game behavior.

## Layout

```
gore-tools/
├─ Cargo.toml            root workspace (members = projects/*/crates/*)
├─ docs/                 monorepo-wide docs (design specs, plans, images)
├─ projects/
│  ├─ gore-save/         save editor (app/ + crates/ + installer/ + tools/)
│  ├─ gore-core/         shared lib (crates/gore_core) + catalog pipelines
│  ├─ gore-cli/          (planned)
│  └─ gore-mod/          (planned)
└─ .github/workflows/    CI + release (gore-save)
```

## Build

The Rust workspace spans all projects:

```powershell
cargo build
cargo test
```

Per-project build/run instructions live in each project's README
(e.g. [`projects/gore-save/README.md`](projects/gore-save/README.md)).

## License

MIT. See [LICENSE](LICENSE).
