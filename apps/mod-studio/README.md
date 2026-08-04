# GORE Mod Studio

> **Development status:** Mod Studio has not been released. There are no active
> legacy user projects; Managed R3 is the sole product project model and
> Snapshot V2 is the sole backup/restore format.

A no-code Windows GUI under active development for *authoring* one Gothic 1
Remake mod, over the same Rust mod engine as the
[`gore` CLI](../../docs/guide/README.md) (via a `dart:ffi`
bridge, `gore_ffi.dll`). The current Managed-R3 surface covers NPC, Quest,
Dialog/Voice, localization, DataAsset, and existing-item scalar workflows plus
installed Texture browsing. Existing-item fields use exact signed-32-bit
integer, finite-32-bit-float, or boolean domains from a closed per-class
schema on explicitly audited game generations; unknown targets fail closed and
Studio does not guess gameplay-specific bounds. Existing patches with obsolete
catalog provenance are invalid current input, not migration or compatibility
cases. Home now presents the exact current saved work as **My mod / Changes**:
author content is grouped by domain, uniquely proven helpers are nested under
their owner, and unresolved or unproven helpers remain visible under collapsed
Technical content. Dialog, Localization, and Voice rows reopen their uniquely
proven context directly in **Text & Voice**; an exact Voice take is revealed
without changing selection, and ambiguous graphs fail closed. Other exact rows
continue into their current owning workspace. Home no longer repeats five large
workspace task rows and retains one existing
**Create** flow for an empty mutable project. Wider layouts keep direct tabs for
all five jobs; narrow/high-scale layouts use one accessible selector over the
same canonical navigation and lazy-page state. The view creates no separate
project state and grants no build, deployment, publication, game/save-write, or
runtime authority. General Audio/Script authoring, vanilla localization and
runtime-topic patches, new-item identity, Texture mutation, and the complete
Managed-R3 build/deploy path are still being implemented and must not be
treated as released capabilities. The exact current capability boundary is
listed under [What it can do](#what-it-can-do) and
[What it can not do](#what-it-can-not-do) below.

## What it can do

- Create or open a **managed revision-3 project** and author bounded offline NPC
  Drafts, Quest Drafts and existing-Quest outline/context/lifecycle edits,
  Voice takes, and reviewed DataAsset stages through exact-head transactions.
  The recommended **Character + first greeting** flow saves an NPC Draft and
  then creates and inserts its first localized greeting through two explicit
  project checkpoints; stopping after the first keeps the honest NPC-only
  Draft. Selected managed Quests and NPCs expose **Source/Profile & checks**,
  including an evidence-only game-compiler check with structured diagnostics,
  exact restoration/recovery handling, and no retained compiler artifact.
- See the exact current project's saved work under **My mod / Changes**, grouped
  as Quests, NPCs, Items, DataAssets, Dialog, Text, and Voice instead of a
  count/readiness-card grid. Proven generated and related helpers stay nested
  under their owning author content; unresolved or unproven helpers remain
  visible in a collapsed Technical section. Dialog, Localization, and Voice
  rows reopen their uniquely proven context directly in **Text & Voice**;
  opening a Voice take reveals it without changing the saved selection, and
  ambiguous graphs fail closed. Other rows continue into the exact current
  Story, Items, DataAsset, or Content workspace. Wider layouts keep direct tabs
  for all five jobs, while narrow/high-scale layouts expose the same canonical
  navigation through one accessible selector. This projection reads the
  managed-R3 content index and verified DataAsset stage registry; it grants no
  build, deploy, publication, game/save-write, or runtime authority.
- Preview the exact current managed project's build coverage across
  localization, dialog, Voice, NPCs, Quests, scripts, items, and verified
  DataAsset stages. This readiness inventory creates no files and does not
  unlock build, install, deployment, or runtime authority.
- Edit supported **existing item/stat scalar values** inside the exact current
  managed R3 project. The categorized editor publishes typed `ItemPatch`
  entities and can fully revert an item to game defaults. Its closed schemas
  use exact signed-32-bit integer and finite-32-bit-float domains plus boolean
  values for explicitly audited game generations. Existing patches must still
  match that exact current catalog layer, class source seal, field set, and
  types; unknown, obsolete, or unsupported provenance fails closed instead of
  entering a compatibility/revert path. Studio does not invent
  gameplay-specific limits. It does not create a new item identity and has no
  item build/deploy/runtime authority yet.
- Edit **localized text and dialog-line IDs**, and stage selectable runtime
  topics with explicit participant, authored AngelScript class, and vanilla
  sentinel identities. The GUI preserves their insertion order and emits the
  same `BuildSpec.dialog_topics` contract as the CLI without inference.
- Replace **audio** — browse a bank's samples, preview, and swap in your own.
- Replace **textures** — pick an asset, preview, drop in a PNG.
- Edit **AngelScript** — stage a module, compile, and splice it into the game's
  script cache (experimental).

## What it can not do

- Build, deploy, spawn, or runtime-qualify managed revision-3 NPC/Quest Drafts;
  compiler acceptance proves only one exact generated source and keeps output
  discarded. Managed Voice has a separately labelled offline-only bundle.
- Edit **save files** — that's the [Save Editor](../save-editor/README.md).
- Hand-write custom Lua logic — use `gore scaffold` plus the
  [gore-lua helpers](../../lua/README.md).
- Patch arbitrary game files outside the supported domains.
- Manage a *collection* of mods together — that's the
  [Mod Manager](../mod-manager/README.md).

## Bundled CLI

Development builds of the installer (and portable zip) include the standalone
**`gore.exe`** CLI beside the app, plus its `shared/` Lua SDK. It gives you the
power tools the GUI does not surface — `gore as disasm`/`decompile` (deep AngelScript RE),
`catalog`/`dump`/`stubs` (data-model regen), and `mgr` (multi-mod management).
Open a terminal in the install dir and run `gore --help`.

The release infrastructure is configured to update the bundled copy with
Studio; the CLI can also be released on its own (`gore-cli-v*`) for
terminal/CI-only use.

## Build / run

Driven by the top-level orchestrator (see [Building](../../docs/development.md)):

```powershell
python build.py gore-mod-studio run          # build (if needed) + launch
python build.py gore-mod-studio dist         # portable zip (incl. bundled gore.exe)
python build.py gore-mod-studio installer    # Windows installer
python build.py gore-mod-studio test         # cargo (gore-ffi) + flutter analyze + test
```
