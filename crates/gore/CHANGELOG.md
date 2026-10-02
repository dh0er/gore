# Changelog

Notable user-facing changes to gore-cli are recorded here. Release automation
uses the matching version section as the GitHub release notes.

## [Unreleased]

- Add 142 `gore save` commands covering the Save Editor: saves, profiles,
  recovery, backups, actors, attributes, skills, inventories, positions, world
  time, progression, factions, locks and merchant stock/timing.
- Share the pending-edit planner with the Editor. Add guarded JSON drafts,
  real byte-level dry runs, partial-commit results and progress reporting.
- Expose the native save protocol and complete CLI surface through MCP.
  Add shared catalogs, ten-language presentation, screenshots, verified game
  images and standalone HTML reports using the Editor's display preferences.

## [0.4.0] - 2026-09-29

- Add `gore npc levels` to list the level scripts available for character
  placement. `gore npc list|show|sites` read characters, their class chain,
  world points and spawn sites from the game's script cache.
- Add `gore npc new|clone|checkout|delete|check|stage|text` to author characters,
  change shipped character defaults and control placement. Add `npc routine`
  commands for daily schedules and interaction spots. An authored character
  was built, deployed and observed in game under its own identity. `checkout`
  uses the single-module standalone compiler and refuses unproven writes.
- Author item, ability and character class-default values from the Shipping
  script cache with `gore value inspect` and a bundle's `values` section. Builds
  compile script mini-caches instead of using the retired `gore gen` and
  `overrides.toml` path. Value and damage changes were observed in game on
  Steam build 25414091.
- Support multi-module script mini-caches. `gore as compile --mini` builds them
  together; bundle build, deploy, the Manager and `as splice --upsert` compose
  them as a unit. A two-module Diego dialog fixture was observed in game.
- Improve AngelScript source recovery and qualify the Gothic Remake 1.0.5
  hotfix source tree with all 164,724 functions aligned and no semantic
  differences or alignment loss. `gore as emit-all --skip <module>` leaves a
  selected module out of the emitted tree.
- `gore as compile` and `compile-module` use a deployed script mod's pristine
  backup, without first undeploying it. `--expect-base` and
  `--expect-base-sha256` can pin the exact original cache; invalid work-directory
  and output layouts fail before a full-tree compile starts.
- Add `GORE_AS_SIDECAR_TRACE=1` for immediate compiler diagnostics, including
  the module and line when a compilation crashes.
- Harden NPC workspaces, value builds and bundle targets against ambiguous
  names, mismatched caches, overlapping edits and unsafe output paths.

## [0.3.0] - 2026-09-02

- Add `gore dialog list`, `tree`, `show`, `export` and `text` for inspecting
  conversations and preparing their localization edits.
- Add the checked `dialog checkout` -> `check` -> `stage` workflow. Shipped
  topics can edit behavior plus complete reconstructed `Caption`,
  `PriorityRank`, `Rules` and flag defaults without silently losing data.
- Add native same-module root and submenu topics with explicit menu placement
  and rank control. New topics may define fields, helpers, strings, conditions
  and persistent game effects.
- Add complete conversations and action-bearing all-new trees for dialogless
  NPCs that already have an exact runtime-loaded conversation-settings module.
- Add selective complete-cache compilation for coordinated, acyclic
  cross-module changes. Independent dialog mini-caches still cannot depend on
  one another, and missing base modules remain unsupported deletes.
- Qualify the new dialog path in game: native roots and subtopics, automatic
  opening, three-level trees, 20-choice menus, ordering, inventory/knowledge/
  quest persistence and clean return of control. Exact evidence and remaining
  structural limits live in the dialog authoring guide.
- Qualify new authored voice lines with Vorbis. 48 kHz mono, 44.1 kHz mono and
  48 kHz stereo played in game; Opus stayed silent and is now inspection-only.
  New lines receive generic facial movement, not generated line-specific lip
  sync.
- Preserve Unreal method metadata and new `BlueprintOverride` hooks across
  full-module recompilation, preventing dialog input locks caused by ordinary
  callable replacements.
- Harden dialog workspaces, manifests, generated defaults, call classification,
  new-symbol remapping and FullGraph composition with fail-closed validation.
- Expose the complete dialog workflow through MCP and synchronize CLI help,
  guides, references and the GORE assistant skill.

## [0.2.3] - 2026-09-01

- Add `gore texture story-images` for loose glossary, tutorial and writing
  artwork outside the asset container.
- Report whole-module script recompilation risk before `as emit` and
  `as compile-module`; 6,982 of 7,317 modules have no known semantic drift and
  the 15 known broken-loop modules are named explicitly.
- Improve decompiled enum returns, increments, copies, loops, construction
  order and scalar/default reconstruction.
- Report crashed standalone compiler processes accurately and refresh stale
  guide flags, paths and counts.

## [0.2.2] - 2026-08-29

- Fix standalone AngelScript compilation for patch 1.0.5.

## [0.2.1] - 2026-08-28

- Add support for Gothic 1 Remake patch 1.0.5.

## [0.2.0] - 2026-08-27

- Bundle the qualified standalone AngelScript compiler and use
  `standalone-then-game` by default, with strict standalone and explicit game
  modes still available.
- Compile complete projects with added and edited modules, cross-module
  references and colliding global function names. Missing base modules remain
  explicit unsupported deletes.
- Match installations by Shipping cache format and complete Binds API instead
  of a whole-executable hash, covering compatible Steam, GOG and repacked
  builds.
- Add strict-standalone MCP compile tools, compiler authentication in
  `gore doctor`, and structured native diagnostics.
- Add `gore mod inspect` for bounded offline bundle validation and deterministic
  manifest/tree hashes.
- Add voice archive validation for Ogg/Vorbis and Ogg/Opus, including duration
  and end-of-stream checks.
- Emit editable class defaults and substantially improve namespace, const,
  accessor, constructor, call-expression, control-flow and temporary recovery
  in decompiled AngelScript.

## [0.1.0] - 2026-08-18

First release of the Gothic 1 Remake command-line modding toolkit.

- Build and transactionally deploy bundles containing item overrides,
  localization, audio, voice, textures/assets, files and AngelScript.
- Manage verified mod libraries and ordered loadouts with import, analysis,
  preflight/recovery, Apply and Reset.
- Edit localization, FMOD banks, voice archives, IoStore textures, cooked
  DataAssets and the AngelScript cache.
- Build and query location catalogs, qualify game generations, and expose the
  CLI through MCP with protected-write consent.
- Ship the complete guide in the binary and as generated HTML/Markdown docs.
