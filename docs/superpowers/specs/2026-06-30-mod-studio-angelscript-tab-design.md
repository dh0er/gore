# mod-studio — AngelScript tab (stage → compile → splice)

**Date:** 2026-06-30
**Status:** Design approved (pending written-spec review)
**App:** `apps/mod-studio` (Flutter) + `crates/{gore-ffi,gore-mod,gore-as}` (Rust)
**Related:** [`2026-06-20-gore-as-angelscript-decode-design.md`](2026-06-20-gore-as-angelscript-decode-design.md) (original gore-as decode/inject design — partly superseded; see "Divergence" below)

## Goal

Add a **Scripts** tab to mod-studio that lets a modder ship AngelScript changes the
same way the Audio/Texture tabs ship asset changes: stage edits, hit **Build / Deploy**,
get a patched game. Each staged item is one AngelScript module — either **new** or an
**edit** of a shipped module. Modules are compiled by the game (the shipping exe has the
AngelScript bytecode loader but no compiler) and the resulting bytecode is **spliced** into
the vanilla precompiled-script cache.

## Decisions (locked during brainstorming)

1. **Scope:** real modding with a thin UI — stage `.as`, compile, splice. No in-app editor (author externally).
2. **Compile:** explicit **Compile** button. It launches the game once with
   `-as-generate-precompiled-data`, captures the regen cache, extracts the module, and caches
   the compiled **mini-cache** in the project. Build/Deploy then only splices (offline, fast).
3. **Module ops:** both **add new** (`splice_auto`, case-b/-a) and **edit existing**
   (`extract_remap` → `replace_module`).
4. **Deploy:** in-place repack of `PrecompiledScript_Shipping.Cache` + `.gore-bak` backup
   (matches audio/loc deploy; supports Undeploy). Not UE4SS runtime injection.
5. **Plan:** one implementation plan covering the full feature end-to-end.

## Divergence from the original gore-as spec

The 2026-06-20 spec chose an **offline `libangelscript` host compiler** and **UE4SS runtime
`LoadByteCode` injection** (no file repack). The implemented and proven path is different and
is what this design builds on:

- **Compile = the game itself** via `-as-generate-precompiled-data` (no offline compiler host).
- **Delivery = repack the cache file** by splicing one module into vanilla (no runtime injector).

The `crates/gore-as` cache splice/extract/remap machinery (and the `gore as` CLI) already
implement the offline half of this. This feature wires that machinery into mod-studio plus
the still-missing pieces (game-launch compile orchestration, gore-mod deploy component, FFI).

## Backend ground truth (what already exists vs. must be built)

**Exists** (`crates/gore-as`, surfaced by the `gore as` CLI in `crates/gore/src/cmd/as_cache.rs`):
- `model::parse_modules`, `emit::emit_module`, `decompile`, `disasm` — read/inventory/emit `.as`.
- `splice::splice_auto(base, mini)` — append a 1-module mini-cache (case-b primitive-only;
  case-a merges native-ref tail tables).
- `splice::replace_module(base, mini, target)` — swap an existing module by name.
- `splice::extract_module(cache, name)` — pull one module → 1-module mini (keeps tail tables).
- `remap::remap_module_to_base(mini, base)` (`ExtractRemap`) — remap a regen module's refs to
  the vanilla cache's keys, emitting an empty-tail-table mini that `replace_module` accepts
  without growing global tables. This is the key step for **editing existing** modules.

**Must be built:**
- **Compile orchestration** (`.as` → mini-cache via the game). Not automated anywhere; today
  the CLI consumes a *pre-made* regen/mini-cache. New code: emit vanilla source tree, overlay
  the user `.as`, launch the game with `-as-generate-precompiled-data`, locate/await the regen
  cache, `extract_module` / `extract_remap` → mini bytes.
- **gore-mod deploy**: `BuildSpec.scripts`, a `Component::AngelScriptPatch`, and prepare/commit
  logic. `BuildSpec` ([`crates/gore-mod/src/lib.rs`](../../../crates/gore-mod/src/lib.rs) ~line 65)
  currently has `overrides / loc_edits / audio / texture` only — no scripts.
- **FFI**: new `gore-ffi` commands for module listing, emitted-source export, and compile;
  `mod_build` already routes `BuildSpec`.
- **Flutter `lib/scripts/`** feature + plumbing into the 5 project/hub files.

## Unit of work

One **script mod** = one `.as` file = one module. The game maps `ScriptRelativeFilename` →
module 1:1 (mirrored by `gore as emit-all`, which writes each module to its `m.file`). So a
module is identified by both:
- `relPath` — the `ScriptRelativeFilename` it compiles under (e.g. `Gameplay/MyMod.as`).
- `moduleName` — the `Modules` TMap key used by `extract_module` / `replace_module`.

For **edit**, both come from the picked vanilla module. For **add**, the user supplies/derives
`relPath`; `moduleName` follows the game's naming convention and is confirmed by reading the
regen cache after compile.

## End-to-end data flow

```
author .as (external editor)
   │
   ├─ Add new module:    pick .as  + choose relPath          → op=add
   └─ Edit existing:     pick vanilla module (Export emitted .as to start) + edit → op=edit
   │
   ▼  Compile  (explicit button → FFI script_compile)
   1. emit-all vanilla cache → source tree   (cached per game/cache version)
   2. overlay this mod's .as at relPath
   3. launch game  -as-generate-precompiled-data ; await regen cache
   4. add  → extract_module(regen, name)
      edit → extract_remap(regen, name, vanillaBase)   (empty tail tables)
   5. mini-cache bytes → cached in project (embedded in .goremod)
   │
   ▼  Build / Deploy   (offline; FFI mod_build → gore_mod build_bundle + deploy)
   • prepare: read pristine PrecompiledScript_Shipping.Cache
   • fold each mod onto running bytes:  add → splice_auto ;  edit → replace_module
   • stage spliced cache → write in-place + .gore-bak backup   (Undeploy restores)
```

The slow/fragile game launch happens only at **Compile**, and only for mods whose `.as`
changed since last compile (stale flag). Deploy is pure offline cache surgery.

## Components

### 1. Flutter feature `apps/mod-studio/lib/scripts/` (mirrors `audio/`, `textures/`)

**`domain/script_mods_notifier.dart`**
- `enum ScriptOp { addNew, editExisting }`
- `class ScriptMod` — fields: `op`, `moduleName`, `relPath`, `asPath` (user source),
  `miniPath` (compiled mini-cache; empty until compiled), `compiledStale` (derived/persisted).
  `key => moduleName`. `toJson`/`fromJson`. `withAsPath` / `withMiniPath` copyWith helpers
  (so `project_io` can swap in embedded relative paths, like `withWavPath`/`withImagePath`).
- `class ScriptModsState { Map<String,ScriptMod> items; int count; List<ScriptMod> entries; }`
- `class ScriptModsNotifier` — `setMod / remove / clearAll / loadAll`.
- `final scriptModsProvider = StateNotifierProvider<…>`.

**`domain/script_modules_provider.dart`**
- `FutureProvider` listing vanilla module names + rel paths (for the edit-target browser),
  via FFI `script_list_modules`. AutoDispose + invalidate on game-path change (mirror
  `texture_index_provider`).

**`ui/script_tab.dart`** (`ConsumerStatefulWidget`, layout mirrors texture/audio tabs)
- Left pane: searchable list of staged script mods + actions **Add new module** and
  **Edit existing module** (the latter opens a searchable vanilla-module browser).
- Right pane (selected mod): op + module name; `.as` source picker (`file_selector.openFile`,
  `.as` type group); **Compile** button with status badge (compiled / stale / error +
  message); **Export emitted .as** (for edit: writes `script_emit_module` output to a file to
  start editing from); remove.
- Staged-list badges show per-mod compile state.

### 2. Compile pipeline — FFI `script_compile`

Single FFI command (orchestration in `gore-mod` or a new `gore-as` orchestration module,
reusing `gore-as` library fns; game launch via `std::process::Command`):

- **Input:** `{ game_dir, op, module_name, rel_path, as_path, work_dir }`.
- **Steps:**
  1. Ensure an emitted vanilla source tree exists under `work_dir`, keyed by the vanilla
     cache hash (emit-all once; reuse across compiles — emitting the 122 MB cache is expensive).
  2. Overlay `as_path` at `rel_path` in the tree.
  3. Launch the game with `-as-generate-precompiled-data`; await the regen cache (poll for the
     output path to appear and its size/mtime to stabilize, with a timeout).
  4. `add → extract_module(regen, module_name)`; `edit → extract_remap(regen, module_name,
     vanilla_base)`.
  5. Write the mini-cache to `work_dir`; return its path (+ resolved `module_name` for add).
- **Errors:** compile failures surfaced best-effort from the game log (v1). Headless
  error-capture harness (DLL inject) is a later enhancement.
- **Open detail (resolve at impl):** exact exe + args, the source-tree location the game reads
  loose `.as` from, and the regen cache output path. These come from the proven manual
  procedure documented in the (gitignored) `work/reversing/gore-as` findings.

### 3. Deploy — gore-mod + gore-as

- `BuildSpec.scripts: Vec<ScriptModule>` where
  `ScriptModule { op: String /* "add"|"edit" */, module_name: String, mini_cache: PathBuf }`.
- `enum Component { … AngelScriptPatch { path: String, mods: Vec<ScriptModRef> } }`.
- `build_bundle`: copy each mini-cache into the bundle (`scripts/<name>.mini`) + manifest;
  emit one `AngelScriptPatch` component.
- `prepare` (mirror `LocPatch`/`AudioPatch`): read pristine
  `G1R\Script\PrecompiledScript_Shipping.Cache`; fold mods onto running bytes in order
  (`add → splice_auto`, `edit → replace_module`); stage the final cache.
- `stage`/`apply`: snapshot + `.gore-bak`, then write the cache in place. Existing Undeploy
  restores from `.gore-bak`.

### 4. FFI additions (`crates/gore-ffi`)

- `script_list_modules(cache) → [{ name, file }]` — `parse_modules`.
- `script_emit_module(cache, name) → .as text` (or to a path) — `emit_module`.
- `script_compile({ … }) → { mini_path, module_name }` — the §2 orchestration.
- `mod_build` already deserializes `BuildSpec`; extend with `scripts`.

Dart wrappers added to `core/mod_ffi.dart` alongside `audioList`/`textureIndex`.

### 5. Project serialization (`.goremod`)

- `ModProject` gains `scripts: List<ScriptMod>` in the ctor, `copyWith`, `toJson`, `fromJson`,
  and `toBuildSpec` (key `scripts`).
- `project_io.dart`: in `saveProject`, embed each mod's `.as` (`assets/scripts/{i}_{base}`)
  **and** its compiled mini-cache (`assets/scripts_cache/{i}_{base}`) and rewrite the model's
  paths to the relative entries (same loop pattern as audio/textures). In `loadProject`,
  extract both to the reused temp dir under the same untrusted-path guards (no `..`, not
  absolute, under `assets/`); silently drop unsafe/missing entries.

## Touch-point checklist

**New files:** `lib/scripts/domain/script_mods_notifier.dart`,
`lib/scripts/domain/script_modules_provider.dart`, `lib/scripts/ui/script_tab.dart`.

**Modified (Flutter):**
- `lib/home_page.dart` — imports; `DefaultTabController` length 6 → 7; add `Tab` (icon e.g.
  `Icons.code`, label "Skripte"/"AngelScript", hardcoded like Audio/Textures); add `ScriptTab()`
  to `TabBarView` (placed after Textures); add `ref.watch(scriptModsProvider).count > 0` to the
  dirty flag (line ~171–174).
- `lib/project/project_model.dart` — field + copyWith + toJson + fromJson + toBuildSpec.
- `lib/project/project_io.dart` — embed/extract `.as` + mini-cache.
- `lib/project/project_controller.dart` — dirty / gather / apply / new include scripts.
- `lib/export/ui/build_deploy_dialog.dart` — count in `hasContent` gate + summary line.
- `lib/editor/ui/overrides_panel.dart` — add script entries to the unified Changes list + total
  + `clearAll`.
- `lib/core/mod_ffi.dart` — `scriptListModules` / `scriptEmitModule` / `scriptCompile` wrappers.
- `lib/l10n/*.arb` — only if a tab/UI string is localized (neighbors hardcode their labels).

**Modified (Rust):**
- `crates/gore-mod/src/lib.rs` — `BuildSpec.scripts`, `Component::AngelScriptPatch`,
  `build_bundle` + `prepare`/commit arm.
- `crates/gore-ffi/src/lib.rs` — `script_list_modules` / `script_emit_module` / `script_compile`.
- Compile orchestration module (in `gore-mod` or `gore-as`) reusing `gore-as` library fns.

## Testing

- **Rust (offline):** `gore-as` splice/replace/extract/remap already covered. Add:
  `build_bundle` emits the scripts component; `prepare` folds add/edit correctly against a
  fixture base (golden: module count +1 for add; unchanged count for edit). Unit-test the
  extract/remap glue against a checked-in fixture regen cache.
- **Rust (game, gated):** `script_compile` end-to-end is an integration test behind an
  `#[ignore]`/feature gate (needs the game installed).
- **Flutter:** `ScriptMod` toJson/fromJson; project round-trip embeds + restores `.as` and
  mini-cache; notifier add/remove/count; dirty flag flips.
- **Manual:** compile a trivial primitive-only **new** module → Deploy → observe effect in
  game; then an **edit** of a shipped module → Deploy → observe; Undeploy restores vanilla.

## Deferred (YAGNI v1)

- In-app `.as` editor (syntax highlight, inline errors). Author externally for now.
- Headless compile-error capture harness (v1 parses the game log best-effort).
- Tier-2 new UE-reflected content (engine container metadata / class-generation hooks).
- Multi-`.as`-per-module bundles (v1 = one `.as` ⇒ one module).

## Risks

- **Game-launch compile fragility** — detecting regen completion reliably; the game must be
  installed and launchable. Mitigation: stabilize-poll + timeout; clear error surfacing.
- **case-a maturity** — primitive-only (case-b) splice is solid; native-ref/class modules
  (case-a) are still maturing. Surface splice errors verbatim; don't claim success on a
  rejected splice.
- **Decompiler stubs** — emitted `.as` for complex *edited* modules can contain stub bodies;
  the "Export emitted .as" output may need manual fixup before it recompiles.
- **Compile cost** — full-tree regen is slow. Mitigation: cache the emitted vanilla tree;
  only re-launch the game for mods whose `.as` changed (stale flag).
- **122 MB in-place repack** per deploy — acceptable (matches loc/audio), backed up to `.gore-bak`.
