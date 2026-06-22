# gore-lua — shared UE4SS modding SDK

**Date:** 2026-06-21
**Status:** Design approved (pending written-spec review)
**Project:** `projects/gore-lua/` (new monorepo sibling) + `gore-cli` integration

## Goal

A shared Lua library for the Gothic 1 Remake UE4SS mods: one place where common helpers
(on-screen text, object/CDO access, player/world, GAS, console-command + keybind
registration) are **used** (`require("gorelib")`) and **inspected** (in-game `gorehelp` +
README). Eliminates the helper duplication currently copy-pasted across GoreCheats,
GiveCheats, GoreCheatUnlock, MyBalanceMod, gore-dump.

## Decisions (from brainstorming)

- **Scope:** full modding SDK (not just on-screen text), organized by area.
- **Placement:** source versioned in the repo (`projects/gore-lua/`); `gore-cli` deploys it
  to `<game>/ue4ss/Mods/shared/gorelib/`. Consistent with the existing repo-sourced mods
  (`projects/gore-dump/Scripts/`) and the gore-cli mod tooling (`scaffold`/`sync`/`package`).
- **Discoverability:** in-game `gorehelp [filter]` console command (live registry) **plus** a
  hand-maintained README API reference.
- **API style:** namespaces by area — `gore.obj`, `gore.player`, `gore.ui`, `gore.gas`,
  `gore.cheat`, `gore.cmd`, `gore.help`.
- **Adoption:** ship the SDK only; do NOT refactor existing mods now. Validate via a
  `gore.selftest()` command + a minimal example mod that consumes the SDK.
- **Deploy tooling:** build the `gore-cli` deploy command + `scaffold` require-wiring now.

## Why a single deployable file

UE4SS `require` is finicky (GiveCheats already needs a `loadfile` fallback). So the SDK
deploys as **one file** `Mods/shared/gorelib/gorelib.lua` with no internal `require`s — it
assembles the whole `gore` table itself. The repo source is that same well-sectioned file
(split later only if it grows unwieldy). Consumers load it with a small robust snippet
(require + loadfile fallback) that `scaffold` wires into new mods.

## Repo layout

```
projects/gore-lua/
  shared/gorelib/gorelib.lua   # the SDK — one deployable file, sectioned by namespace
  example/Scripts/main.lua     # minimal example mod consuming the SDK (also the smoke test)
  example/enabled.txt
  README.md                    # browsable API reference (mirrors the gorehelp registry)
```

## API surface (v1 — grounded in the proven mod code)

Every function pcall-guards its reflection calls and returns `nil`/`false` on failure
(never crashes the mod). Each registers `name + signature + 1-line doc` into the help
registry at load.

- **`gore.obj`** — `valid(o)`, `find(cls)` (FindFirstOf), `findAll(cls)` (FindAllOf),
  `cdo(path)` (StaticFindObject of a `/Script/...Default__X`), `prop(o,name[,default])`
  (safe get), `setProp(o,name,v)` (safe set).
- **`gore.player`** — `pc()` (PlayerController), `pawn()` (GothicPlayerCharacter→Pawn),
  `asc()` (AbilitySystemComponent), `loc(actor)` → x,y,z, `setLoc(actor,x,y,z)`,
  `rot()` (ControlRotation pitch,yaw), `forward()` (unit vector from rot).
- **`gore.ui`** — `ftext(s)` (KismetTextLibrary::Conv_StringToText), `text(s)`/`notify(s)`
  (on-screen via `FindFirstOf("HUDSimpleTextMessageController"):ShowSimpleTextMessage`,
  with the `W_SimpleTextMessage_C` / `SettingsMessageWidget:AddMessage` fallbacks),
  `log(...)` (UE4SS print with a tag).
- **`gore.gas`** — `setAttr(setPath,name,v)`, `getAttr(setPath,name[,default])`,
  `heal()` (Health←MaxHealth), `buff(tbl)` (apply a {setPath→{name=v}} table).
- **`gore.cheat`** — `god(on)` (write `m_GodMode` on live CombatConfig + the two CDOs),
  `enableCheats()` (`pc():EnableCheats()` on the game thread; returns ok).
- **`gore.cmd`** — `command(name,fn)` (RegisterConsoleCommandHandler wrapper; fn gets
  `(args, Ar)`), `keybind(key,fn)` (RegisterKeyBind + ExecuteInGameThread), `onGameThread(fn)`
  (ExecuteInGameThread wrapper).
- **`gore.help`** — `register(ns,name,sig,doc)` (used internally by each helper),
  `help([filter])` (returns/prints the registry by namespace). The SDK registers the
  `gorehelp [filter]` console command once on load.

`gore.selftest()` runs each helper in a safe probe and reports per-namespace OK/fail
in-game (the validation path, since no existing mod is converted).

## gore-cli integration (built now)

- **`gore-cli deploy-shared [--game <dir>]`** — copy `projects/gore-lua/shared/` →
  `<game>/ue4ss/Mods/shared/`. Resolves the game dir the same way existing commands do
  (the established `--out`/dir-resolution pattern in `cmd/sync.rs` / `cmd/scaffold.rs`);
  default to the known install if those commands have one, else require `--game`.
- **`scaffold`** — extend the generated `main.lua` template to include the robust SDK
  loader snippet at the top:
  ```lua
  local ok, gore = pcall(require, "gorelib")
  if not ok then local f = loadfile([[...\Mods\shared\gorelib\gorelib.lua]]); gore = f and f() end
  ```
  so new mods get `gore.*` out of the box.

## Error handling

- Every SDK function wraps reflection in `pcall`; on failure returns a falsy value and
  optionally `gore.ui.log`s a one-line reason. No SDK call throws to the caller.
- `gore.ui.text` degrades through its fallback chain (controller → widget → settings
  widget) and logs if none is live (e.g. called at the menu with no gameplay HUD).
- The loader snippet itself is pcall+loadfile-guarded so a missing/incompatible SDK never
  hard-fails a consuming mod.

## Testing

- **In-game:** `gore.selftest()` exercises each namespace's helpers safely and prints a
  per-function OK/fail summary via `gore.ui.log` (and `gore.ui.text` when a HUD is up). The
  example mod registers a `goretest` command that calls it.
- **Example mod** (`projects/gore-lua/example/`) is the reference consumer + smoke test:
  it `require`s gorelib, registers one command that calls `gore.ui.text` + `gore.player.pawn`,
  proving the deploy + load + API path end-to-end in normal mode.
- **Repo-side:** the gore-cli deploy command gets a unit test (copies a temp `shared/`
  tree to a temp dest, asserts files land). `luacheck` lint if available (optional, no hard
  dependency).

## Out of scope (deferred)

- Refactoring existing mods (GoreCheats/GiveCheats/...) onto the SDK — later, incrementally.
- gore-cli-generated API docs (README is hand-maintained for v1; the live source of truth is
  the `gorehelp` registry).
- Item-give helpers from GiveCheats (`gore.items`) — add once the core SDK is proven.
- Splitting the SDK into multiple source files + a concat build — only if the single file
  grows unwieldy.
