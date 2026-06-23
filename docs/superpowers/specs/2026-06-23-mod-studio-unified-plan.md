# Mod Studio — Unified Mod Plan (overrides + loc/dialog + audio, one bundle)

Date: 2026-06-23
Status: Draft for review
Merges: `docs/fmod-audio-modding-plan.md` (audio, built) + `docs/superpowers/specs/2026-06-23-mod-studio-loc-editing-design.md` (loc/dialog, specced only).

## Goal

One Mod Studio that edits **item stats** (CDO overrides), **localized text** (item names +
dialog lines), and **audio** (SFX/music/VO) — and exports them as **one bundled mod** that
deploys to the game and is shareable. A mod is also saveable/loadable as a **project file**.
The architecture must absorb a **future "Scripts" tab** without reworking the bundle/deploy.

## Current state (verified, both worktrees)

- **Audio**: `crates/gore-fmod` + `gore audio` CLI (list/extract/replace/restore/export-patch/
  apply-patch) DONE and in-game proven. No mod-studio audio UI, no FFI.
- **Loc/dialog**: spec only, **0 implemented**. Primitives exist: `gore-loc` `Lcache`
  (decode/set_value/encode, byte-faithful), `apps/mod-studio/lib/loc/game_lang.dart`
  (10 GameLangs, `resolveGameText`), loc catalog extract + `locCatalogProvider`.
- **mod-studio export today**: AppBar button → `ExportDialog` → `export_notifier.dart` → FFI
  `generate_mod` (payload `{meta, override[]}`) → Rust returns in-memory `{files:{enabled.txt,
  Scripts/main.lua}}` → **Dart** materializes folder/zip. No loc, no audio, no manifest, no
  deploy, no project file. Tabs (3): Items, Overrides, Settings (`home_page.dart:146`).
- **gore-ffi** (`crates/gore-ffi/src/lib.rs`): `generate_mod`, `validate`, `loc_status`,
  `loc_find`, `loc_extract`. No `audio_*`, no loc-edit, no deploy.

→ Both editing features are **greenfield in the GUI**; nothing to de-dup.

---

## Core architecture — the unifying spine

A single pipeline, with each content domain a pluggable **component**:

```
Project (.goremod, editable)
   └─ Build ──► Bundle (folder/zip: gore-mod.json manifest + per-component payloads)
                   └─ Deploy ──► Game   (Undeploy restores from backups)
```

### Delivery model — the key reconciliation

Three component *kinds*, two delivery mechanisms, one manifest-driven engine:

| Component | Edit state | Bundle payload | Deploy action | Undeploy |
|---|---|---|---|---|
| **overrides** (CDO stats) | `{class.field: value}` | built `ue4ss/<Mod>/` Lua mod | copy → `ue4ss/Mods/<Mod>/` | delete mod dir |
| **loc** (names+dialog) | `{id:{set:text}}` | `loc/edits.json` (declarative) | backup live `.lcache`→`*.gore-bak`, decode **pristine** → `set_value` → encode → write live | restore `*.gore-bak` |
| **audio** | `{bank: {sample: wav}}` | `audio/manifest.json` + `audio/*.wav` | backup live `.bank`→`*.gore-bak`, `replace_samples` on pristine → write live | restore `*.gore-bak` |
| **scripts** *(future)* | TBD | `scripts/…` | TBD handler | TBD |

**DECIDED: the bundle ships declarative PATCHES (loc `edits.json`, audio wavs+manifest),
applied at DEPLOY against the user's own pristine game files** — not a pre-baked
`.lcache`/`.bank`. Rationale: (1) one consistent apply path for loc+audio; (2) bundles stay
small and ship **no copyrighted game audio/text**; (3) robust to the recipient's game version
(re-encode/inject against *their* pristine files). Overrides stay a runtime UE4SS Lua mod
(FMOD/loc have no runtime hook; CDO does).

### Scope boundary — mod-studio vs the future mod-manager app

**DECIDED: full mod management (installing several mods at once, stacking, conflict
resolution, ordering) will be a SEPARATE future app.** mod-studio's job is **authoring**:
edit → Build the bundle → Save/Load the project → a *simple single test-deploy* so the author
can try their own mod in-game.

Consequences for this plan:
- The **bundle + `gore-mod.json` manifest is the hand-off contract** to that future manager.
  It must be self-describing: each component declares its concrete deploy target(s) and a
  source sha256, so an external manager can detect overlaps and stack/merge loose-file edits.
  Design the manifest now for that consumer (it's cheap; it's just metadata).
- `gore-mod` provides **build + a basic single-mod deploy/undeploy** (in-place with
  `*.gore-bak`, one deploy-record `<game>/gore-mod.deployed.json`). This powers the CLI and
  mod-studio's test-deploy. **Stacking/multi-mod is explicitly out of scope here** — the
  future app builds it on the same `gore-mod` primitives + manifest.
- mod-studio test-deploy is single-mod: deploy = undeploy-previous (restore backups) then
  apply, warning on source-hash drift (game patched / foreign mod present), per loc spec §F.

### Extensibility for the Scripts tab (and beyond)

Adding a domain later is **additive**, touching no existing component:
1. a new editor tab + StateNotifier (declarative edit state),
2. a new `project.json` section,
3. a new bundle component `{type:"script_…"}` + one deploy/undeploy handler in the engine.

The manifest's `components[]` list + per-`type` handler dispatch is the seam that makes this
open/closed.

---

## Schemas (define FIRST — these are the contracts the parallel work builds against)

### `gore-mod.json` (bundle manifest)
```jsonc
{
  "format": 1,
  "mod": { "name": "MyMod", "version": "1.0.0", "author": "" },
  "components": [
    { "type": "ue4ss_lua",  "path": "ue4ss/MyMod",        "deploy_to": "ue4ss/Mods/MyMod" },
    { "type": "loc_patch",  "path": "loc/edits.json",     "target_glob": "G1R/Story/Cache/AlkimiaLocalization_*.lcache" },
    { "type": "audio_patch","path": "audio",              "target": "G1R/Content/FMOD/Desktop", "banks": ["SFX.bank","Music.bank"] }
  ]
}
```

### `project.goremod` (editable project — a zip)
```
project.json
assets/audio/<hash>.wav        # embedded source wavs (self-contained, portable)
```
```jsonc
// project.json — union of all notifiers' declarative state
{
  "format": 1,
  "mod": { "name": "...", "version": "...", "author": "" },
  "overrides": [ { "class": "...", "field": "...", "value_int": 1 } ],
  "loc_edits": { "itfo_cheese": { "german_new": "Käse" }, "dia_x": { "english_newer": "..." } },
  "audio":     [ { "bank": "SFX.bank", "sample": "SFX_UI_...", "asset": "assets/audio/ab12.wav" } ]
}
```
Build derives the bundle from the project: overrides→Lua, loc_edits→edits.json,
audio→manifest+wavs. Load restores each notifier; Save serializes them + embeds wavs.

### FFI command contracts (gore-ffi, JSON in/out like existing)
- `audio_list  {bank}` → `{samples:[{index,name,codec,freq,channels,seconds}]}`
- `audio_extract {bank, sample}` → `{ogg_path}` (writes temp .ogg for preview)
- `mod_build  {project|inline-state, out_dir|return_files}` → `{ok, files|written}` (full bundle incl. manifest)
- `mod_deploy {bundle_dir, game_path}` → `{ok, deployed:[…]}`
- `mod_undeploy {game_path}` → `{ok, restored:[…]}`
(`generate_mod` is subsumed by `mod_build`; keep as thin alias during transition.)

---

## New/changed code

### Rust
- **`crates/gore-mod` (new)** — the bundler + deploy engine. Orchestrates `gore-modgen`
  (Lua), `gore-loc` (`Lcache`), `gore-fmod` (`replace_samples`). API:
  `build_bundle(BuildSpec) -> Files`, `deploy(bundle, game) -> Report`,
  `undeploy(game) -> Report`, manifest (de)serialize, deploy-record, pristine-source +
  sha-drift checks, atomic file ops. Per-component-type handlers (extensible).
- **`crates/gore-ffi`** — add `audio_list`, `audio_extract`, `mod_build`, `mod_deploy`,
  `mod_undeploy`; route to `gore-mod`/`gore-fmod`.
- **`crates/gore/src/cmd/mod_*.rs`** — CLI `gore mod build|deploy|undeploy` over `gore-mod`
  (parity with FFI; reuses existing `gore audio`/`gore loc`).

### Flutter (`apps/mod-studio/lib`)
- **loc/domain/`LocEditsNotifier`** — declarative `{locId:{setName:text}}` (loc spec §A).
- **loc primary-set helper** next to `game_lang.dart` — `(id,GameLang)→set`, read+write.
- **shared per-language field widget** (Dialoge + Items).
- **dialog/** — Dialoge tab: provider (filter `info_/dia_/gvl_/svm_`, group by speaker,
  search by id+text, virtualized) + UI (loc spec §B).
- **editor/ui/field_editor.dart** — add Name section (per-language) (loc spec §C).
- **audio/** — Audio tab: `AudioReplacementsNotifier`, sample list (FFI `audio_list`),
  preview (FFI `audio_extract`→`just_audio`), replace picker.
- **project/** — `.goremod` save/load: serialize/restore all notifiers, embed/extract wavs.
- **export/ → build+deploy** — "Build Mod" (unified bundle via `mod_build`) + "Deploy to
  game"/"Undeploy" (via `mod_deploy`/`mod_undeploy`); enable when ANY domain dirty.
- **home_page.dart** — tabs become: Items, **Dialoge**, Overrides, **Audio**, Settings
  (Scripts later). Project menu (New/Open/Save/Save As).

---

## Parallelization

**Phase 0 — Spine (do first, small, blocks fan-out).** Lock the three schemas + FFI command
signatures above; scaffold `crates/gore-mod` (types + stub fns) and the FFI command stubs
(return `unimplemented`); add empty Flutter `audio/`, `dialog/`, `project/` dirs with notifier
class stubs. After this, every workstream codes against fixed contracts.

**Phase 1 — Parallel workstreams** (independent files; same Opus-agent fan-out):

| WS | Scope | Files | Depends on | Parallel-safe with |
|----|-------|-------|-----------|--------------------|
| **A** | `gore-mod` bundle builder | `crates/gore-mod` build_bundle + manifest | P0 | B,C,D,E,F,G |
| **B** | `gore-mod` basic single-mod deploy/undeploy + deploy-record (no stacking) | `crates/gore-mod` deploy.rs | P0 (shares crate w/ A — split by file) | all |
| **C** | FFI audio_list/extract + mod_build/deploy/undeploy wiring | `crates/gore-ffi` | A,B API (stubbed in P0) | D… |
| **D** | CLI `gore mod build/deploy/undeploy` | `crates/gore/src/cmd/mod_*.rs` | A,B | all Flutter |
| **E** | Flutter LocEditsNotifier + primary-set + per-lang widget | `lib/loc/` | P0 | all |
| **F** | Flutter Dialoge tab | `lib/dialog/` | E | G,H,I,J |
| **G** | Flutter Items Name section | `lib/editor/ui/field_editor.dart` | E | F,H,I,J |
| **H** | Flutter Audio tab | `lib/audio/` | C (audio_list/extract) | F,G,I |
| **I** | Flutter project save/load | `lib/project/` | E + H notifiers (interfaces from P0) | F,G |
| **J** | Flutter Build/Deploy UI + home_page tabs | `lib/export/`, `home_page.dart` | C | F,G,H |

Critical path: P0 → (A+B) → C → (H,J). Loc Flutter (E→F,G) runs fully in parallel to the
Rust/audio path. ~9 workstreams concurrent after P0.

**Phase 2 — Integration + tests + manual verify.**
- Rust: `gore-mod` build→deploy→undeploy roundtrip; loc edit→encode idempotent on real-shaped
  data; audio `replace_samples` 2-FSB5 invariant; manifest (de)serialize; sha-drift detection.
- Flutter: notifier unit tests (loc edits set/revert/clear/dirty; primary-set resolver;
  audio replacements); dialog provider grouping/filter; project save→load roundtrip restores
  all three notifiers; build payload contains all dirty domains.
- Manual: edit an item name + a dialog line + swap a sound, Build Mod, Deploy, launch game,
  confirm all three live, Undeploy, confirm all originals restored; save/load project.

---

## Decisions

1. **DECIDED — Loc delivery = patch-at-deploy** (`edits.json`, unifies with audio). Changes
   loc spec §D/E to emit `edits.json` and apply at deploy.
2. **DECIDED — Mod management (stacking/multi-mod/conflict) = separate future app.**
   mod-studio = authoring + Build + Save/Load + single test-deploy. The bundle manifest is the
   hand-off contract; `gore-mod` ships build + basic single deploy only.
3. **Recommended — Project file = zip `.goremod` embedding source wavs** (self-contained).
4. **Recommended — build + deploy engine in Rust (`gore-mod`)**, shared by CLI + GUI.
5. **Deferred — audio compact-repack via user-supplied FMOD `fsbank`** (audio plan Pfad B);
   PCM injection suffices and bundles ship wavs, not banks.

## Carried-over decided defaults (from source docs)
- Audio: pure-Rust PCM injection, no FMOD/third-party; in-place + `*.gore-bak`. (audio plan)
- Loc: edit 10 menu langs, write each to its primary set; item = name only; dialog = text only;
  no new ids/langs; `gvl_/svm_` barks secondary. (loc spec)
