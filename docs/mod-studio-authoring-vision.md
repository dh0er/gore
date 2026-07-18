# Mod Studio authoring architecture and product specification

Status: design target grounded in the repository as of July 2026. This document
is deliberately stricter than a feature wish list: it separates capabilities
that are already proven from those that are partial, missing, or still require
runtime research.

Related evidence and operating boundaries:

- [GORE Mod Studio today](../README.md#gore-mod-studio)
- [Bundle and deployment contract](../README.md#bundling--deploying)
- [AngelScript dialog authoring and live proof](dialog-authoring.md)
- [AngelScript quest authoring and discovery proof](quest-authoring.md)
- [AngelScript logical NPC authoring](npc-authoring.md)
- [Managed revision-3 Voice authoring](voice-authoring.md)
- [Cooked DataAsset fixed-leaf workflow](dataasset-authoring.md)
- [Offline AngelScript default patching](angelscript-default-patching.md)
- [Managed project snapshot export](managed-project-export.md)
- [Managed project snapshot import V2](managed-project-import.md)

### Current implementation delta (2026-07-18)

**Unreleased format policy:** Mod Studio has never been released and there are
no active legacy projects. Managed revision 3 is the only product project model
and Snapshot V2 is the only backup/restore format. Superseded internal project
schemas, project readers, Snapshot V1, converters, dual-write, and fallback
paths should be removed rather than migrated. The UI currently labelled
`Legacy` remains a compact UX/reference surface only; preserving its useful
tabs does not preserve its format-1 project backend. Any older migration wording
below is superseded until the obsolete sections are deleted in the dedicated
cleanup checkpoint.

Every surviving format-1 or schema-R1/R2 startup, session, persistence, and
build path is therefore deletion debt, not a compatibility surface. It may
supply an implementation or UX lesson, but no new feature may write to it, read
it as a fallback, migrate it, or expose it as another project mode. Snapshot V2
is the sole backup/restore wire format.

The current landed checkpoints advance previously partial rows below; these
statements supersede older “not wired yet” wording where the matrix has not yet
been consolidated:

- The managed shell now presents all nine canonical destinations as direct,
  horizontally scrollable classic top tabs. Desktop and compact layouts share
  one navigation model; programmatic routes reveal an off-screen selection,
  while lazy mounting, same-project state, secondary-route memory, keyboard
  activation, and project-identity reset remain intact. This is presentation
  only and grants no format, mutation, build, deployment, or runtime authority.
- Managed **Settings & Expert** now opens directly on compact
  **Settings | DataAsset Lab** secondary navigation. Both views mount lazily and
  retain same-project state; the global Settings command selects the embedded
  Settings view, while recovery/no-project states retain a dialog fallback.
  The responsive Lab has one scroll surface and remains read-only local snapshot
  inspection, separate from project-owned verified DataAsset edits.
- **Name & objectives** is no longer disabled after a Quest receives generator-
  V4 behavior. A native/Dart managed Outline V2 transaction keeps the active
  objective-slot set, next ordinal, conditions, effects, identities, and all
  unrelated entities exact while allowing display name, Quest title, objective
  titles, and presentation order to change. Studio loads and carries the stable
  slots invisibly and publishes only through the existing exact-head managed
  checkpoint lane. This remains offline, build-blocked, runtime-unqualified,
  and undeployed.
- The generic Scripts workspace now consumes the compiler's bounded structured
  report. It presents file/line/column/severity diagnostics, states when the
  normal fallback was used, and accepts only a regular marker-bound mini under
  the invocation's uniquely owned staging child after `restored_exact`. One
  app-scoped read-only safety state keeps Compile and Deploy blocked across
  source/mod changes while the game runs, inspection is uncertain, or compiler
  or deploy recovery persists; only a fresh safe native probe clears it.
  Compile, deploy, manager apply, and undeploy share one cross-tool install
  mutation guard and recheck the process immediately before their first live-
  content or recovery write. This narrows rather than eliminates a later game-
  launch race because the game does not honor the toolkit lock.
- Managed Quest and NPC **Source/Profile & checks** now add an evidence-only
  compiler action on top of that safety boundary. The request carries only the
  Store root, configured game root, exact head, and selected entity ID. Native
  code re-derives the persisted entity, owned ScriptModule, namespace, relative
  path, and source hash before taking the install guard; it regenerates the
  exact source under the guard, compiles in an unreported native-private
  workspace with fixed additive policy, restores the installation, and
  neutralizes the mini-cache through its retained create-new/no-follow handle.
  The strict result remains bound to the original
  project/entity/module revisions and preserves diagnostics, closing drift, and
  recovery evidence without exposing a cache or staging path. Studio can say
  only that the compiler accepted the exact current source; managed production
  build, deploy, runtime, spawn, and publication integration remain missing.
- The selected managed-R3 Quest/NPC detail is one responsive, exact-current
  Story Workbench. Quest exposes the four canonical homes **Journey**, **Dialog
  & Voice**, **References**, and **Problems & Checks**; legacy Story/Logic
  selections normalize to Journey. NPC now likewise exposes only four
  productive homes: **Profile**, **Dialog & Voice**, **References**, and
  **Problems & Checks**. Legacy NPC Story/Routine/Inventory selections normalize
  to Profile; those three unmodeled areas remain discoverable in one initially
  collapsed, honest summary instead of occupying empty primary tabs. Journey
  retains the bounded Quest editor handoffs, Dialog & Voice owns transcript or
  greeting authoring, Profile owns bounded NPC name/archetype editing and its
  greeting continuation, Problems & Checks owns source/compiler inspection,
  and More actions owns safe Draft removal.
  Projected reference problems are not full build/runtime readiness, and the
  distinct Draft/build-blocked/runtime-unqualified boundaries remain visible.
- The exact-current Quest Journey now carries a persistent **Write this
  Quest** path that matches the real publication boundary: the valid Quest
  details are one atomic saved checkpoint and the opening dialog is a separate
  second publication. Generator-v2/v3 Quests additionally expose a conditional
  legacy-behavior review; modern V4 Quests do not invent a universal extra
  step. Progress is derived from the same reopened Journey projection, not
  stored as another flag. It exposes one recommended continuation through the
  already-owned Dialog & Voice or Transitions editor, remains responsive at
  compact/high-scale layouts, and keeps Voice counts supplemental. A complete
  Draft setup is still neither Build-ready nor playable and grants no runtime
  authority.
- The persistent Project Work Bar now also exposes **Undo last change**. Each
  invocation loads fresh authenticated history, selects only its immediate
  predecessor, confirms the project-only append operation, and restores that
  content as a new current+1 revision through the existing serialized restore
  lane. Dirty text, recovery/reopen, another project action, head drift, late
  results, malformed receipts, and missing history fail closed. This is one
  global previous-change continuation, not labeled Redo, a semantic diff, or a
  general cross-domain transaction system; it never touches the game or saves.
- The primary managed-R3 **Story** destination is now a direct authoring
  workspace instead of a card page that redirects authors into Content. It
  loads the exact-current content index, projects only `NpcDraft` and
  `QuestDraft`, and keeps friendly search, All/NPC/Quest filters, creation,
  selection, and the existing Story Workbench together. Wide hosts use an
  inline list/workbench split; compact or short hosts open the same workbench
  in a details sheet. Same-project checkpoint advances retain only still-valid
  selection/tab state, newly published drafts can be selected at their exact
  revision, and non-Story references route to their exact Content owner. This
  is a UI projection over the existing managed session and grants no additional
  mutation, build, deploy, runtime, game, or save authority. The useful Legacy
  tabs remain the UX baseline and are unchanged.
- A selected exact-current managed NPC or Quest Draft now exposes one direct
  **Remove Draft...** action in both wide and compact Story workbenches. The
  Studio derives the complete two-entity closure from the current content
  index, blocks local backlinks, kind mismatches, or additional ownership, and
  routes a blocker to its exact source entity. Confirmation names both the
  Draft and its generated ScriptModule and states that V1 has no undo while the
  game installation and saves remain unchanged. The pure native transaction
  independently proves the exact three-edge ownership closure, deterministic
  module regeneration, and preservation of every other entity plus the full
  AssetStore. Its strict FFI prepares and fully reopens only an immutable
  unpublished candidate; the serialized managed session alone may publish by
  fixed-head CAS and full reopen. This is semantic project deletion only: it
  performs no blob garbage collection, build, deploy, game/save access, or
  runtime action, and project-wide deletion plus shared undo/history remain
  missing.
- A selected managed-R3 NPC now also exposes **Edit name & archetype** directly
  from **Profile**. The normal form contains only the friendly display name and
  a verified archetype picker; stable entity/module IDs, `UniqueName`, module
  namespace/path, origins, and generated technical identities are not editable.
  Studio refreshes the sealed Story and NPC catalogs when the form opens and
  immediately before save. Native preparation independently resolves both the
  stored and selected catalog records and treats an archetype change as a change
  to their complete three-parent provenance, not merely to a catalog ID. A
  name-only edit preserves the owned ScriptModule byte-for-byte and at the same
  revision; a changed parent triple atomically replaces all three provenances
  and deterministically regenerates and revisions only that owned module. The
  prepare-only FFI returns an immutable fully reopened candidate, while the
  serialized managed session alone may publish by fixed-head CAS and full
  reopen. This is project-only authoring: the configured installation supplies
  read-only catalog evidence, and the operation performs no build, deployment,
  game/save write, spawn, or runtime qualification.
- Managed R3 **Validate & Test** now hosts bounded **Problems & Readiness V1**.
  It derives unresolved entity and asset references from the exact-current
  content index, loads the managed DataAsset-stage registry independently, and
  retains usable content diagnostics when that registry is unavailable. Search,
  category filters, responsive detail, exact source-entity/asset navigation,
  DataAsset-registry and Settings routes, and exact-head verification are
  integrated. Its assessments remain independent: reference integrity,
  DataAsset registry, game configuration, compiler evidence **Not evaluated**,
  managed build **Blocked**, and runtime **Unqualified**. It is not a complete
  incremental validator, quick-fix system, compiler result, build plan,
  deployment result, or runtime proof.
- Managed Voice entry actions fail closed against the exact-current projection.
  Add, Manage, and Resolve require at least one intact Voice-authorable
  `DialogLine` with a resolved same-project `LocalizationEntry`; Add and Resolve
  also require a configured installation. A guided V1 prerequisite now lets a
  fresh managed-R3 project create one project-owned `LocalizationEntry` plus
  `DialogLine` and optional empty unresolved `VoiceSlot`, or revisions-bind the
  new line to one exact existing, currently unused managed localization without
  changing it. The native transaction prepares and fully reopens an immutable
  candidate, checks the fixed head twice, and leaves guarded publication to the
  managed session. It accepts no game root, accesses no game or save, and grants
  no topic, AngelScript, build, runtime, deployment, or native publication
  authority. The unsealed global localization catalog remains excluded: this is
  project-local creation/exact reuse, not vanilla adoption.
- **Localization & Voice** now defaults to a bounded **Work list**, with the
  existing **Project texts** editor one switch away. It derives only two
  evidence-backed item kinds: a project authoring locale absent from one safely
  editable localization, and one intact already-existing `VoiceSlot` for an
  exact line/locale. It never treats absence of a slot as missing-recording
  intent. Existing slots receive one next step in strict order: add recording,
  review/approve, select/repair, resolve target, or production decisions
  complete. The final state opens **Validate & Test** and grants no Ready,
  build, deployment, audibility, or runtime authority. Actions reuse the exact
  project-text/Add/Manage/Resolve flows and recheck root/project/revision/head,
  catalog identity, line/locale, and the derived next step around asynchronous
  work. The normal list is capped at 500 rows, reports omissions, and preserves
  known language work with an explicit warning when Voice evidence is
  unavailable. Broader coverage, batch/team review, CSV/XLIFF, and multi-locale
  production remain missing; no World work is included.
- **Manage Voice takes** now owns review status as well as selection. An author
  can move one retained take through Draft/Recorded/Reviewed/Approved and then
  select a newly Approved take without re-importing it or leaving the dialog.
  Status and selection remain separate exact-head transactions; a selected
  take may only be changed to Approved until the selection is changed or
  cleared. The status
  transaction changes only project revision plus the chosen take's revision
  and workflow label, preserves slot/selection/media and unrelated content,
  accepts no game or source path, and grants no audio-quality or runtime proof.
- **Project > Close** now closes the current coordinator-owned Legacy or
  managed-R3 session after the existing dirty-Legacy confirmation. It neither
  deletes project data nor supplies Save As, history, or recovery.
- Managed-R3 uncertainty now has a first bounded **Try recovery** action instead
  of a warning-only dead end. It stays inside the serialized lease while the
  exclusive project lock remains held, repairs the fixed publication journal,
  fully reopens assets, and adopts only the same project/schema/game target at
  the previous revision or its one-step successor. Failure keeps the prior
  in-memory checkpoint and `requiresReopen` lock. This is safe journal recovery,
  not undo, history, Save As, project import, or a game/save operation.
- Current-hotfix Viper and Asghan dialog candidates have deterministic copied-
  cache remaps and offline parse/disassemble/decompile/undeploy evidence. Viper
  still needs the controlled render-only live gate; Asghan still needs the
  separate disposable-save state matrix. Neither offline result is gameplay
  proof.

## 1. Product outcome

A complete Mod Studio lets a non-technical author build a substantial mod in
terms of game concepts: NPCs, quests, dialog, spoken lines, items, locations,
spawns, and reusable assets. The normal workflow must not expose archive member
paths, reflected class paths, byte offsets, mini-caches, sentinel classes, or
raw `BuildSpec` JSON. Those remain inspectable in an Expert view and available
through the CLI.

The shortest large-mod workflow is:

1. Create a project from a small template and select the target game version
   and authoring languages.
2. Find a vanilla entity in one content browser, then **Override**, **Clone as
   new**, or **Reference** it. The distinction is explicit; cloning does not
   silently change the identity of vanilla content.
3. Use an NPC or quest wizard to create a valid skeleton with generated stable
   IDs, localization records, dialog roots, and dependency placeholders.
4. Work primarily in the quest/dialog graph. Selecting a line opens its text,
   speaker, conditions, effects, and voice takes in one inspector.
5. Resolve inline diagnostics and use quick fixes. Technical names are
   generated, collision-checked, and editable in Advanced mode.
6. Preview a deterministic build plan and content diff, then build without
   touching the game.
7. Deploy to an explicit test profile only after a clean-install and generation
   preflight. Undeploy restores the recorded pristine state.

The project, not the game installation and not an extracted temporary file, is
the source of truth. Build artifacts are reproducible derivatives.

## 2. Product principles

### 2.1 Concepts first, storage second

The primary navigation is a unified content browser with filters for NPCs,
quests, dialog, lines, items, DataAssets, world/spawn records, audio, textures,
scripts, and localization. Search covers display names, technical IDs, tags,
source paths, and referenced-by relationships. Selecting an entity shows one
consistent inspector and a **References** panel rather than switching the user
between unrelated storage-format tools.

### 2.2 Progressive disclosure

The default inspector uses semantic controls: faction pickers, item references,
quest-state dropdowns, condition builders, localized text fields, and waveform
voice controls. A per-entity Expert panel exposes exact Unreal/AngelScript
identity, origin selector, generated source, raw-but-typed DataAsset properties,
and final lowering output. Unsupported raw values are visible but read-only;
the UI must never guess a serializer.

### 2.3 Safe composition, not a sequence of file edits

Every authoring action is a transaction over a typed project graph. Validation
happens continuously, while build performs a complete closed-world validation
and deterministic lowering. A failed action or build publishes no partial
project or bundle. Build is offline by default. Deploy and undeploy are separate
operations with receipts, no-clobber behavior, generation checks, and rollback.

### 2.4 Fast repetition

Large mods require templates, multi-select, clone-with-dependencies, find all
references, keyboard navigation, copy/paste of graph subtrees, and batch import
and export. CSV is useful for tables such as localization, stats, inventories,
and spawn lists; it is not allowed to flatten arbitrary graph semantics. Every
batch operation first shows a dry-run diff and is one undoable transaction.

### 2.5 Honest proof levels

The UI labels an operation as one of:

- **Supported**: the complete relevant offline path is tested and the required
  runtime behavior has been demonstrated where runtime behavior is part of the
  claim.
- **Experimental**: a real bounded subset works, but the authoring or runtime
  boundary is incomplete.
- **Unavailable**: Mod Studio prevents the build instead of emitting a guessed
  artifact.

Capability labels apply to a concrete operation for the selected game version,
not to an entire content kind. Editing one proven field of an existing NPC can
therefore be Supported while creating a new NPC identity remains Unavailable.
The Create and template surfaces show this before an author starts work, rather
than revealing it only at build time.

### 2.6 Information architecture and vocabulary contract

The Studio shell is organized by authoring task, not by backend or file format:

```text
Global bar
  Project | Back/Forward | Search/Commands | Undo/Redo
  Autosave + compatibility status | Validate | Test | Build

Primary navigation
  Home
  Content Library
  Story
  World
  Localization & Voice
  Validate & Test
  Build & Release
  History
  Settings / Expert mode

Persistent workspace
  Left: scope, outline, collections, and saved searches
  Center: table, form, graph, transcript, preview, map, or timeline
  Right: Properties | References | Problems
  Bottom drawer: Changes | Diagnostics | Build log | Test log
```

Normal Legacy startup now shows a localized managed-project entry banner with
direct Create/Open actions, labels the unchanged tabs as Legacy compatibility
tools, and reuses the existing guarded transition/adoption flows; the canonical
nine-destination shell appears after managed adoption. Managed Studio Shell v1
now implements this canonical nine-destination shell
for managed R3 projects: **Home**, **Content**, **Story**, **World**,
**Localization & Voice**, **Validate & Test**, **Build & Release**, **History**,
and **Settings & Expert** remain visible at every support level. **DataAssets** is a
secondary view inside Content rather than a top-level format destination. The
workspace lazily mounts pages, preserves the selected primary destination,
per-section secondary route, and mounted page state across revisions of the
same project, and resets to Home when project identity changes.

This shell remains an evolving product surface, but Home now follows the compact
task-router direction. The compact Legacy tabs remain the minimum discoverability
and task-efficiency baseline for workflows that already work there. Do not retire
or hide one of those useful workflows until its managed equivalent is at least as
direct and capable. The destinations above define stable product
responsibilities; related responsibilities may be presented together in one
productive workspace through progressive disclosure instead of multiplying
dashboards, cards, and modal launchers. Major changes to that presentation
require user review before they become the product direction.

Home loads the exact-current `Revision3ContentIndex` and leads with **Story**,
**Dialog & Voice**, **Problems**, **Content**, and **Build & Release** before
compact counts and readiness. It no longer duplicates low-level authoring,
import, build, or export launchers: Story owns NPC/Quest creation, the domain
workspaces own their operations, and Project owns export. Story opens a direct
exact-current NPC/Quest search, filter, list, creation, and Workbench surface;
Localization & Voice exposes the bounded take, selection, and target actions;
Validate & Test can verify the exact current head and inspect references; Build
& Release exposes only the bounded Voice bundle; and Settings is available.
World authoring, runtime testing, full managed build/deploy, and Expert tooling
remain visibly unavailable instead of disappearing. Root, project ID, revision,
and head stay collapsed as technical details, while the `requiresReopen`
recovery gate remains outside the workspace. This shell grants no new game/save
writes, deployment, project-wide build, or runtime authority.

The Content Library now exposes explicit `This mod`, `Base game`, `Installed`,
and `Search all` scopes and does not render a fake `Dependencies` scope before
the managed dependency model exists. Scope pages mount only when visited,
retain state across revisions of the same project, and reset when project
identity changes. `Base game` currently presents one generation-consistent
bounded NPC/Quest starting-point projection; broader static NPC evidence is
search-only and grants no Draft action. `Installed` is a search-first, capped
metadata view over the exact installed DataAsset package snapshot and hands an
exact selected path to the existing inspector.

`Search all` is the first bounded global cross-source search slice. A nonempty
explicit query loads `This mod`, `Base game`, and `Installed` independently and
scans each resulting projection in memory; each source reports loading,
complete, partial, or error state independently and retains at most 100
results. Search is case/accent tolerant, while every available action carries
the exact identity already owned by its source: open a current mod entity/asset,
start a Base-game NPC/Quest Draft, or inspect an installed DataAsset target. The
combined screen is not an atomic snapshot across all three sources, and a
result does not imply a dependency, typed reference, backlink, or cross-source
authoring authority.

Within **This mod**, selecting an exact-current `QuestDraft` or `NpcDraft` shows
a friendly discovery summary and one **Open in Story** continuation. Content
renders no duplicate Quest/NPC Workbench, edit, or source-check actions. The
continuation revalidates the exact project, revision, head, index, selection,
entity, and kind before Story selects that Draft in Journey; while reopening is
required, the same action remains visible but disabled with the recovery reason.
The primary **Story** destination owns the responsive Workbench, search, filters,
creation, selection, and mutation workflows. The Workbench shares the screen
with its entity list only when the entity area is at least 900 logical pixels
wide and 430 logical pixels high; falling below either bound opens the same
tabbed detail in the existing 78%-height modal sheet. The selected entity and
its last supported tab survive a revision advance of the same project only while
that entity still exists. Project identity changes clear both, and a removed or
non-Story entity cannot retain stale tab state. **References** shows only exact
projected outgoing entity/asset links and derived incoming links from the current
index. **Problems & Checks** counts only unresolved projected links and reuses
the bounded inspection action; it is not a project-wide validator or a
build/runtime readiness claim.
DataAssets, archive members, scripts, textures, and generated artifacts may
become broader searchable content types or Advanced details, but they do not
become an ever-growing row of top-level format tabs. Editors still need
resizable panes, document history, breadcrumbs, pinned/recent entities, and a
command palette so a large project does not depend on repeatedly navigating one
tree.

Default UI copy uses player- and author-facing concepts. The following are
Advanced terms and must not be required to complete a normal workflow:

- `BuildSpec`, lowerer, mini-cache, CDO, `FName`, unversioned header;
- reflected class, sentinel class, runtime identity, source seal;
- archive/member path, deployment operation, and byte offset.

For example, the three source operations are presented as **Edit the original
game object**, **Duplicate into my mod**, and **Use without changing**. Their
precise `Override`, `Clone as new`, and `Reference` semantics remain visible in
Advanced help and in the build plan. `GameGenerationAnchor` appears as **Game
version**. Diagnostics lead with what the author was editing, what will not
work, and how to fix it; technical evidence is expandable.

### 2.7 Draft, build, and runtime readiness

Authoring readiness and implementation proof are separate. Every entity and
project view can report:

- **Draft**: valid author intent that may still contain incomplete references or
  require an unavailable generator. Drafts save, clone, compare, and participate
  in dependency analysis, but cannot silently disappear from a build.
- **Build-ready**: the complete selected target can lower and pass offline
  validation for this game version.
- **Runtime-qualified**: the relevant behavior has also passed the required
  bounded runtime proof. The qualification records its game version and scope.

This permits an author to outline a new quest or NPC safely before every runtime
mechanism exists. Templates that rely on an unavailable mechanism are marked
**Draft only** before creation, and production build remains blocked with a
specific diagnostic. A compiled artifact alone never upgrades runtime
readiness. Experimental content is allowed in an explicitly non-production
test build only when its risk profile permits it. Build-ready and
Runtime-qualified are derived for a concrete profile from validators and a
versioned evidence/capability registry; they are never user-editable checkboxes.

## 3. Current capability matrix

“Proven” below means evidence exists in current code, tests, or the linked live
proof. It does not widen that evidence to adjacent use cases.

| Authoring capability | Current status | Evidence and exact boundary |
|---|---|---|
| Managed R3 workspace shell | **Canonical nine-destination shell and Project Work Bar V1 implemented; direct Story workspace landed; domain depth remains partial** | Managed R3 exposes Home, Content, Story, World, Localization & Voice, Validate & Test, Build & Release, History, and Settings & Expert through one responsive shell. DataAssets is a Content secondary view. A persistent Work Bar above every destination shows the friendly project name and current primary area and exposes global **Search**, **Create**, and **Problems** continuations. Search opens the existing Content `Search all` scope and focuses its existing query without loading any source before a nonempty submission. Create reuses the existing NPC Draft, recommended Quest plus opening-line, and new dialog-line flows with their exact configured-game, clean-workspace, recovery, checkpoint, and post-dialog revalidation gates. Problems routes to the existing Validate & Test owner. Compact/high-scale layouts keep Search direct and place Create/Problems in an accessible overflow with visible disabled reasons; callbacks are single-flight and late project/checkpoint/disposal continuations fail closed. Lazy-mounted pages preserve primary selection, per-section secondary routes, and page state across same-project revisions; a different project resets Home, the Work Bar state, and its project-bound Search handoff. Story is no longer a card/link landing page: it directly loads the exact current index, lists only NPC/Quest Drafts with friendly search and All/NPC/Quest filters, offers the bounded create actions, and opens the existing responsive Workbench in place or in a compact/short-host details sheet. Exact-revision selection after creation and same-project state retention fail closed on drift or deletion; non-Story references route to Content. Localization & Voice exposes the guided dialog-line prerequisite V1 plus take/selection/target actions; Validate hosts bounded Problems & Readiness V1 plus exact-head verification while keeping compiler, managed-build, and runtime evidence explicitly separate; Build exposes only the sealed Voice bundle; History exposes the sealed bounded project timeline and append-only restore; and Settings is available. The Work Bar adds orientation and routing only—no read, mutation, publication, build, deployment, game/save-write, or runtime authority. World authoring was not started; runtime test, full managed build/deploy, full command palette/undo/build/test chrome, and Expert tools remain visible but unavailable. Explicit exact-current Quest/NPC compiler checks temporarily stage the exact generated source and mutate the authenticated compiler/cache paths in the selected installation under the shared install guard, while restoring every touched install path or retaining recovery evidence. They discard output and grant no save write, deployment, general managed build, or runtime qualification. Legacy remains available as the usability baseline and was not replaced by this projection. |
| Project save/load | **Partial Studio paths; managed revision-3 create/open/close and bounded History/Undo proven** | The compatibility session still saves `.goremod` format 1, and the separate Story flow still owns a schema-revision-2 directory. The Project menu can now create a new managed R3 project from friendly name/version/author/locales plus an existing empty real directory. Creation authenticates one exact registered V1 or current Steam-build-`24169431` executable/Shipping/Binds triple, generates a secure nonzero project ID, builds canonical empty revision-3 JSON, publishes by absent-head CAS, fully reopens it, and adopts it only after exact identity/project-byte checks. Unknown or cross-paired generations fail closed. A valid head produced before a late create failure is reopened and recovered; a mismatched candidate is closed, while every nonempty or game-overlapping destination—including a prior lock-only scaffold—is rejected before generation hashing or native creation. A live current-install test created, read, closed, and reopened the empty project while proving the executable and both caches unchanged by length and SHA-256. The same coordinator opens existing R3 directories, drives Home and `Ctrl+S`, verifies exact-current heads, preserves dirty Legacy work on failed transitions, and surfaces `requiresReopen` and cleanup failures. **Project > Close** releases the current session after dirty-Legacy confirmation; bounded same-lock `requiresReopen` recovery is available. History lists only the exact current snapshot's sealed newest-first window (at most 256 entries) and restores a retained older member as a fresh current+1 revision through the same crash-recoverable publication lane; it never scans CAS or moves the head backwards. Project deletion, Save As, named checkpoints, semantic diffs, unlimited archival history, and general recovery remain missing. While R3 is authoritative, legacy editors and Build/Deploy are hidden. Home exposes the strict semantic index and bounded Quest/NPC/Voice/DataAsset mutations through the same lease. NPC/Quest and generic DataAsset work remain outside a production build; the separately labelled Voice bundle and exact reviewed Footstep stage can each emit a bounded offline artifact without deployment/runtime authority. General semantic editors, migration/import/clone/Save As, named checkpoints/semantic history diffs/general recovery UI, dependency search and collections, unified blob ownership, and all-domain transactions remain missing. |
| Unified content browser | **Bounded global cross-source search and canonical Story handoff integrated; semantic breadth remains partial** | Content keeps the exact-current project Library and verified DataAsset stages under one responsive, lazy scope host that resets on a different project. `Base game` exposes curated NPC/Quest starting points plus search-gated inspect-only experimental NPC evidence; missing setup performs no load and routes to Settings. `Installed` reads exact package-index metadata and opens the existing inspector by canonical path. `Search all` runs only after an explicit nonempty query, scans the three source projections independently in memory, and retains at most 100 rows per source with independent loading/complete/partial/error state. Results are case/accent tolerant and expose only exact same-source actions: open a current mod identity, create a Draft from one exact Base catalog identity, or inspect one exact installed target. Selected current-project Quest/NPC Drafts show a friendly discovery summary and one exact **Open in Story** continuation; Content contains no duplicate Quest/NPC Workbench, edit, or source-check actions, and preserves the disabled continuation with a recovery reason while reopening is required. The screen is not one atomic combined snapshot and grants no dependency, package/build/deploy/runtime/game-write, or cross-source mutation authority. No fake `Dependencies` scope exists. Items, complete dialog/localization, FMOD audio, textures, scripts, several Legacy tools, indexed/virtualized large-scale search, complete semantic NPC/Quest browsing, source-aware clone, collections, complete incremental semantic validation, transactional quick fixes, profile-specific build/runtime readiness, and a complete cross-domain editing workspace remain missing. |
| Existing item scalar edits | **Proven subset** | The categorized item browser and typed scalar field editor stage CDO overrides. The fallback schema is limited and does not imply arbitrary property or item creation support. |
| Existing NPC edits | **Bounded managed-R3 name/archetype editor integrated; broader semantic editor missing** | A selected exact-current managed NPC exposes **Edit name & archetype** from its Profile. The friendly form edits only `Entity.display_name` and chooses a verified catalog archetype; stable NPC/module IDs, `UniqueName`, namespace/path, origins, and other technical identity remain unchanged and hidden. Open and pre-save refresh sealed Story+NPC catalogs, and native preparation independently resolves the exact current and desired catalog selections. Archetype equality is the complete `CharacterDefinition` / `AIAgentConfig` / `SpawnAIAgentDefinition` provenance triple, including generation, source seal, layer, selector, and runtime class, so an alias catalog ID with the same triple is not a structural change. A name-only edit increments project/NPC revisions while preserving the complete owned ScriptModule and its revision; a changed triple atomically swaps all three provenances and deterministically regenerates/revisions only that owned module. The prepare-only FFI fully reopens an immutable candidate; guarded exact-head publication and full reopen remain solely in the serialized managed session. The configured installation is read only for catalog evidence, and no game/save write, build, deploy, spawn, or runtime qualification occurs. Visuals, stats, faction, inventory, routine, AI, dialog/quest links, localization/voice, and placement remain unmodeled. Generic CDO overrides remain a separate existing-class subset. |
| New NPC identity | **Managed revision-3 Draft publication and first Guided wizard proven; build/spawn/runtime missing** | A new `CharacterDefinition` with a new `UniqueName`, a linked `AIAgentConfig`, and a linked `SpawnAIAgentDefinition` compile and compose as one additive AngelScript module while leaving visual/actor defaults inherited from Asghan-derived parents. The revision-3 core atomically inserts the closed NPC/module pair against an exact head/project/revision/target while consuming fresh Story+NPC catalog selection and a base-game-plus-current-project collision inventory for modules, paths, symbols, and pinned-catalog runtime IDs. It regenerates the complete existing NPC/Quest/module closure, preserves valid Quests, and fails on residual/drift/collisions. Strict Dart DTOs validate the exact native candidate; the managed R3 session publishes by guarded fixed-head byte CAS and full reopen. Home now exposes a Guided wizard that asks only for display name and a searchable qualified archetype, refreshes catalog evidence immediately before publication, derives and hides technical identities, rejects stale/reopen-required publication, and refreshes the visible checkpoint/content view. The separate schema-revision-2 Story Draft flow remains available. This is only a logical-clone shell: visuals, faction, stats, inventory, routine, dialog, quests, and placement are not authored. Every result remains build-blocked/runtime-unqualified/not spawned, with no production lowerer or runtime workflow. Class residence, discovery, effective visuals, distinct identity, spawning, dialog/quest separation, and persistence are not proven. Runtime-ID coverage is limited to the pinned catalog projection. Cooked-asset creation is required for genuinely new visual/content assets and for any registry or collection change the recovered chain actually requires, but is not currently proven mandatory for a logical NPC identity. See [NPC authoring](npc-authoring.md). |
| Guided Character + first greeting | **Two-checkpoint managed-R3 Draft recipe integrated; runtime missing** | The recommended normal NPC route composes the existing exact Character Draft and NPC greeting Create-and-Insert transactions. Step 1 publishes the project-only NPC/ScriptModule pair as N+1. After a full exact root/project/revision/head reopen, step 2 creates one project-owned localized DialogLine and inserts it at greeting index zero as N+2. Completion opens the exact NPC in Story Dialog & Voice with the line selected. A cancelled or safely failed step 2 retains and opens the useful NPC-only N+1 result; uncertain publication requires reopen, drift locks the single-flight recipe, and nothing is rolled back or retried implicitly. The separate advanced Character-only Draft and the one-step Base-game/search starters remain available. This convenience flow creates no topic, choice, condition, effect, Quest relationship, playable conversation, runtime binding, spawn, build, deploy, game write, or save write. |
| Existing localized dialog lines | **Proven** | The Dialogs tab groups `info_`/`dia_`/`gvl_`/`svm_` IDs, edits languages, and can add an explicit missing localization ID. Localization alone does not create a selectable topic. |
| Managed-R3 dialog-line prerequisite | **Guided V1 project-local creation/exact reuse integrated; dialog runtime missing** | A fresh managed-R3 project can create one new `LocalizationEntry` and `DialogLine` plus an optional empty unresolved locale `VoiceSlot`, or create the line against one exact existing, currently unused managed localization bound by entity revision and localization identity. The pure transaction binds the exact head/project/revision/target, preserves an exact-reused entry byte-for-byte, and emits only a build-blocked/runtime-unqualified candidate with topic authority not granted. The prepare-only FFI accepts no game root, fully reopens the immutable candidate with asset verification, checks the fixed head after preparation and response construction, and never publishes it; only the serialized managed session may use guarded fixed-head CAS, repair, and full published reopen. This path reads or writes neither the game nor a save. It does not trust the unsealed global localization catalog, adopt vanilla identity or speaker evidence, create a topic or AngelScript, register runtime behavior, or produce playable dialog. |
| New dialog topic insertion/rendering | **Earlier narrow render proof; version-3 candidate offline-qualified, one controlled live visual run remains** | A compiled `UChoice` class plus explicit participant/topic/sentinel registration previously reached the natural choice UI and was visually confirmed. The current version-3 candidate is deterministic, passes the strengthened forbidden-operation and preflight-order verifier, and deploys/undeploys to exact sandbox tree identity. That is offline/sandbox evidence, not a live version-3 result. Requalification requires exactly one natural Viper-menu run with no selection and no save; automatic discovery and selection effects remain unproven. See [dialog authoring](dialog-authoring.md). |
| Dialog selection effects | **Unproven** | Topic selection, quest/knowledge changes, `ActedTopics`, and selection-side save effects are outside the render proof. The safe proof intentionally selected nothing. |
| Quest inspection/edit/create | **Managed revision-3 creation, Story Workbench V1, outline/context, bounded V4 states-and-transitions, and ordered transcript editing proven; build/runtime missing** | Managed R3 Home exposes the friendly one-to-eight-objective Quest Draft wizard. A selected Quest opens four canonical Story tabs: **Journey**, **Dialog & Voice**, **References**, and **Problems & Checks**. Journey presents the objective-centered behavior projection and owns the contextual **Name & objectives**, **Description & connections**, and **States & transitions** handoffs; legacy Story/Logic requests normalize to Journey. Dialog & Voice owns the bounded ordered transcript, References projects exact-current links without claiming readiness, and Problems & Checks owns source/compiler inspection. The transcript presents friendly speaker/text/locale/Voice coverage, attaches, reorders, groups by stable V4 objective slot, detaches without deleting shared lines, atomically creates and inserts one DialogLine/localization/optional empty Voice slot, and hands one exact line/locale to **Localization & Voice**. The V4 behavior dialog presents the main Quest/objectives against Available/Start/Success/Failure, a sequential template, independent direct-engine triggers and typed conditions, bounded Start/Succeed/Fail follow-up actions, and objective-parent completion. Conditions use the six reviewed lifecycle tests with optional negation and are capped at 8 DNF alternatives x 8 atoms; each transition has at most 8 effects. The closed native validator enforces canonical order, stable active objective slots, drivers, required availability/start and objective terminal edges, valid references, contradiction rules, same-handler terminal conflicts, same-kind cycles, and at most 256 unique same-project transcript lines. It accepts no raw AngelScript, runtime topic, selection effect, journal, reward, item, or arbitrary effect. Schema revision remains 3: merely reading or deriving the effective plan or an empty transcript does not migrate a generator-v2/v3 Quest; an otherwise unchanged Quest keeps its JSON/source bytes. V4 retains a canonical plan with stable slots, and its legacy seed regenerates V2/V3 source byte-for-byte. Outline/context/transitions/source inspection preserve transcript metadata. Transcript edits increment project/Quest only and leave the owned module/source/revision unchanged; the compound create-and-insert path still publishes exactly one project revision. The prepare-only transition and transcript FFI routes accept no `game_root`, prepare and fully reopen only immutable candidates, and never replace the fixed head. Native status remains `blocked`, `runtime_unqualified`, topic authority `not_granted`, and publication `not_supported`; the managed session is the only publisher through guarded exact-head byte CAS, repair journal, and full published reopen. An isolated 1.0.3 qualifier compiled the four external-trigger fields and predicate-hook shapes, all three handler shapes, `bSucceedParent`, typed getters, and guarded lifecycle-call shapes twice and produced the same reopened 7,306-module composed cache (123,406,626 bytes, SHA-256 `FB041B3DF1CBD5A0AFC1D87F47BFCA6392AA19CE6475CE9DBD61A6D099D9C41A`). It did not compile one renderer-produced fixture spanning every state-test expression, and it is compiler/cache evidence rather than gameplay. Runtime transition order/effects, transcript-driven dialog selection, journal/rewards/items, persistence/save/reload/uninstall, production build/deploy, and a synchronized general Quest/dialog graph remain missing. Quest authoring transactions write neither the game installation nor a save; the separate explicit exact-current compiler check temporarily stages the exact generated source and mutates the authenticated compiler/cache paths under guarded restoration of every touched install path or retained recovery, then discards its output. See [quest authoring](quest-authoring.md). |
| Voice archive editing | **Compatibility existing-member slice proven; managed revision-3 Work list, entry, import, exact preview/QA, status, selection, safe removal, installed-target, and sealed offline-build foundation proven; production workflow incomplete** | The compatibility line-first editor remains separate. Managed R3 **Localization & Voice** defaults to a bounded Work list and keeps Project texts one switch away. It projects only absent authoring-locale membership and intact existing `VoiceSlot`s; absence of a slot never invents recording intent. Exact slot precedence is zero takes → add, no Approved take → review/approve, no valid Approved selection → select/repair, unresolved/ambiguous target → resolve, otherwise production decisions complete. Unreviewed alternatives do not regress that final decision, and completion opens checks without claiming Ready/build/runtime. Rows reuse the existing exact Add/Manage/Resolve or locale-edit flows. The list retains 500 normal rows, reports omitted work, preserves known language work when Voice evidence is unavailable, and fails closed on root/project/revision/head/catalog/next-step drift. Existing bounded take import, status, selection, preview/media QA, slot/take removal, target resolution, and deterministic sealed Vorbis existing-member offline build retain their independent exact prepare/full-reopen/session-publication contracts. | Line/localization/take project mutations write neither game nor save; offline output and desktop playback are not runtime evidence. Managed Voice still lacks deploy/undeploy, isolated runtime testing, audible-game qualification, ambiguous-member choice, recording/transcode, complete coverage and broader batch/team review, CSV/XLIFF, qualified Opus lowering, sealed vanilla adoption, and new-member lookup proof. “Production decisions complete” is intentionally narrower than any of those claims. See [Voice authoring](voice-authoring.md). |
| FMOD sound/music replacement | **Proven** | Studio browses samples, previews originals or staged WAVs, and stages replacements for the bundle engine. This is sound-bank replacement, not spoken-dialog voice authoring. |
| Texture replacement | **Proven subset** | Existing texture assets can be browsed and replaced with additive IoStore output. This is not general cooked-asset creation, visual-media round trip, or an Unreal Editor bridge. |
| Existing cooked DataAsset fixed leaves | **Managed revision-3 typed editor, reviewed Footstep quick start, verified registry, and one offline build proven** | Receipt-bound extract/patch/re-inspect/offline-pack and the generic inspector-proven Bool/integer/float/color/vector editor remain available under **Expert tools**. The normal quick start names the exact Human, Scavenger, and Wolf `FootstepTag` targets and opens installed data as the primary path to the reviewed `BoneData/BoneFeetData/FeetTextureSize` Vector4 form. Studio exposes X/Y scale presets, Before/After preview, and preserved Z/W; values are explicitly raw asset units. A successful typed publication is returned to the registry, accepted only as the next revision, reloaded, and expanded only when target plus staged revision match. The separate exact-current **Build files...** action can emit the reviewed offline package triplet. Neither editing nor build grants deployment, gameplay, runtime, structural/new-asset, or Unreal-bridge authority. Broader reviewed schemas, gameplay-qualified units, multi-edit/undo, structural editing, new assets, and the sealed Unreal handoff remain missing. Neither path writes the game installation or a save file. See [DataAsset authoring](dataasset-authoring.md). |
| DataAsset creation/reference/collection editing | **Missing** | New exports/packages, `FName`/object/package reference creation, map keys, variable-width values, unversioned-header growth, array/map shape changes, and the optional sealed Unreal handoff are not implemented. These are hard prerequisites for genuinely new visual/content assets and for any content path that is proven to require new cooked package/reference/collection shapes. Current evidence does not establish them as universal prerequisites for logical NPC, item, or quest identities. Stock Unreal Editor is not assumed to open cooked G1R packages or emit compatible output. |
| AngelScript source authoring | **Experimental** | Studio can stage new/edit modules and use the game compiler, with guarded diagnostics and mini-cache lowering. Existing generated `__InitDefaults` methods are not generally source-editable; new modules are the supported path for authored defaults. |
| Existing native default edits | **Proven narrow CLI path** | Scalar direct assignments and already-present `GameplayTag -> float32` entries can be patched offline under sealed selectors. Keys, maps, code size, and general generated source cannot be added. |
| Items/world/spawns | **Partial/missing** | Existing item scalar overrides are present. New item identity, placed world actors, spawn points, routines, level edits, and world-partition integration have no semantic Studio workflow or production proof. An optional sealed Unreal handoff is planned only for explicitly supported future operations; no bridge or compatible world writer exists today. |
| Localization | **Proven** | Multi-language edits and explicit new IDs lower to `BuildSpec.loc_edits`; deploy is backup/restore aware. Referential completeness across quests, dialog, and voices is not yet validated as one graph. |
| Build/deploy/undeploy | **Proven for represented Legacy domains; bounded managed-R3 Voice and reviewed-DataAsset offline lowerers proven; managed deployment and project-wide build missing** | Studio drives the same bundle engine as the CLI and can restore its deployment for represented compatibility-project domains. The bounded existing-member spoken-line replacement participates in that portable Save/Reopen and Build/Deploy path. When an R3 project is current, the shell hides Legacy editors and Legacy Build/Deploy instead of reading stale provider state. R3 NPC/Quest Drafts and generic DataAsset stages remain outside a production build. Managed Voice has an explicitly labelled all-or-nothing offline builder for verified selected Vorbis existing-member replacements; one exact-current reviewed Human/Scavenger/Wolf `FeetTextureSize` stage can separately emit a strictly reopened offline package triplet. Neither path deploys or grants project-wide pack/gameplay authority. Project-wide semantic roots, dependency/risk review, managed deploy/undeploy, rollback, and isolated runtime qualification remain missing. |
| Validation | **Bounded Problems & Readiness V1 integrated; full validator missing** | `Validate & Test` derives exact-current unresolved entity/asset references, game-configuration state, managed DataAsset-registry availability, and offline-only stage limitations. It supports search/filter/detail, exact source navigation, relevant-workspace actions, partial-source handling, and exact-head verification. Quest/NPC findings open the exact Draft in Story **Problems & Checks**; other entities and project assets use revision-and-head-bound Content navigation; DataAsset findings filter, select, and expand the exact staged edit. The report intentionally has no overall readiness value: compiler evidence remains `not evaluated`, general managed build remains `blocked`, and runtime remains `unqualified`. Existing scalar, codec, DataAsset, Quest-V4, build, and explicit Quest/NPC compiler checks retain separate evidence boundaries; Problems neither invokes nor records those compiler checks. Project-wide incremental semantic validation, quick fixes, reachability simulation, a general build plan, deployment, and runtime qualification remain missing. |
| Undo/redo/history | **Bounded snapshot History/Undo and publication-repair journal proven; command-level undo/redo missing** | Managed R3 retains an authenticated newest-first window of at most 256 exact project checkpoints and restores a retained version append-only as a fresh revision through the guarded publication lane. Fixed-head replacement uses an exact old/new-generation recovery journal with conservative ambiguity handling. There is still no shared inverse-command log, redo, named checkpoints, semantic diffs, unlimited archive, or general multi-domain recovery UI. |
| Templates/clone/batch/CSV | **First Empty/NPC Draft/Quest Draft starters integrated; reusable templates and batch tools missing** | New-project creation now offers **Empty**, **NPC Draft**, and **Quest Draft** before metadata entry. Every choice first creates and adopts the same canonical empty revision-0 project through absent-head CAS; an NPC or Quest choice then opens the existing guided exact-head wizard and may publish a separate revision 1. Cancel before publication leaves the exact empty project. If publication outcome cannot be verified, Studio requires reopening instead of claiming that the project stayed empty. Copy states this two-stage boundary before creation and never calls it one atomic template transaction. Multi-domain/playable-slice templates still require a native compound prepare/publication transaction. Dependency-aware templates, clone modes, transactional bulk editing, and CSV round trip remain missing. |
| Expert escape hatch | **Partial** | The CLI and script-source editor expose powerful low-level paths. Studio lacks a unified generated-source/raw-property/BuildSpec inspector and source override contract. |

The managed project core exposes the V2 **Create project backup** workflow from
the Project menu. Home deliberately does not duplicate this project-level
action. It serializes the exact current R3 head, its recursive historical
Quest-basis closure, and the current snapshot's bounded direct History closure
into a deterministic, strictly reopened, no-clobber `.goremod`. History
embedded in retained checkpoints is deliberately not followed. The three
sealed terminals distinguish published, published-with-cleanup-warning, and
publication-uncertain outcomes; the latter is never retried automatically.
This operation needs no configured game, writes neither game nor save data, and
does not adopt a new working path. Snapshot V2 is the sole backup/restore
format; the unreleased V1 experiment and its compatibility paths are removed
rather than migrated.

**Restore project backup** is visible from the Project menu in every project
state and on the empty/Legacy landing surfaces. One dialog owns exact V2
inspection, a real existing parent plus new absent-folder choice, and one
single-flight native materialization. It displays bounded labels rather than
source parent paths. Only confirmed success or cleanup warning yields a receipt;
publication uncertainty is receipt-free, close-only, and never opened or
retried automatically. The serialized current-project lane fully opens the
candidate and compares normalized destination, project ID, revision, canonical
head, and reopen state before adoption. Any mismatch leaves the current session
unchanged and closes the candidate exactly once. Native-import and prior-session
cleanup warnings remain distinct. Inspection/materialization is Windows-only
and fails closed on Unix. This grants no Clone/Save As, build, deployment,
game/save mutation, or runtime authority. See [Managed project snapshot
export](managed-project-export.md) and [Managed project snapshot import
V2](managed-project-import.md).

## 4. Complete authoring surfaces

### 4.1 Project dashboard

The implemented managed-project Home is a compact task router, not a second
editor or a collection of modal launch cards. It has exactly five continuations:
**Story**, **Dialog & Voice**, **Problems**, **Content**, and **Build & Release**.
Each opens the canonical workspace that owns the job. Story owns the recommended
**Character + first greeting** and **Quest + opening line** flows plus the
advanced NPC-only and Quest-only Drafts. Localization & Voice owns project text,
dialog-line, and Voice actions;
Validate owns Problems; Content owns discovery and DataAsset work; Build &
Release owns output actions. Export stays in the Project menu. Normal chrome
calls this a **Mod Studio project** and keeps schema/head vocabulary in technical
details. The nine stable shell destinations now use direct, horizontally
scrollable classic top tabs instead of the managed rail/compact popup. The
useful Legacy layout remains a UX reference only; it grants no format-1
compatibility promise.

Project Work Bar V1 is the first persistent project-level chrome shared by all
nine destinations. It keeps the index-derived friendly project name (using only
the directory name while that exact read is pending) and current primary area
visible above the lazy page stack. Its three commands are deliberately thin:
**Search** opens Content's existing `Search all` scope and focuses the existing
query; **Create** offers the recommended Character plus first greeting and Quest
plus opening line recipes, the advanced NPC-only Draft, and the new
dialog-line journey; **Problems** opens the existing
Validate & Test report. The chooser preserves each flow's truthful gates: NPC
and Quest require a configured game, and all creation remains disabled for a
dirty managed workspace, recovery, or `requiresReopen` and revalidates the exact
checkpoint after the chooser. Search mounting and focus perform zero source
loads before a nonempty submitted query. At compact width or 200% text, Search
remains direct and the other commands move to an accessible overflow with their
exact disabled reasons. Commands serialize, publish a screen-reader live busy
state, remain keyboard reachable, and drop late focus/action continuations after
checkpoint drift, project switch, detach, or disposal. A project identity
change remounts the chrome and invalidates its bound Search controller; a
same-project revision refreshes its displayed checkpoint-derived name without
inventing a second project state. This layer grants no new content, read,
mutation, publication, build, deployment, game/save-write, or runtime authority.

Story's recommended **Character + first greeting** route is a bounded
two-checkpoint recipe over those same owned editors. The existing Character
transaction publishes N+1, Studio fully rebinds to that exact checkpoint and
empty NPC greeting projection, and the existing Create-and-Insert greeting
transaction may publish one localized line at index zero as N+2. Completion
opens and selects that exact line in the NPC's **Dialog & Voice** tab. Cancelling
or safely failing the second form retains the NPC-only N+1 Draft and opens the
same empty greeting surface; there is no hidden rollback. Project/head drift
locks the single-flight continuation and uncertain publication requires reopen.
Base-game and cross-source search starters remain one-step Draft creation rather
than silently opting into this recipe. The result remains Draft-only and grants
no topic, choice, condition, effect, Quest relationship, runtime binding,
playable conversation, spawn, build, deployment, game-write, or save authority.

Creating a project is a guided, resumable onboarding flow:

1. Select or detect a game installation and show its edition and game version.
2. Choose project languages, an author-facing project name, and a blank,
   tutorial, or concept template. Studio generates a collision-checked technical
   namespace; changing it is an Advanced action.
3. Choose which installed mods are dependencies and which isolated loadout is
   the test profile; the normal user loadout is never selected implicitly.
4. Show a capability preview for that exact game version and template, including
   which operations are Supported, Experimental, Draft only, or Unavailable.
5. Index the sealed base-game and dependency catalogs, then open a useful first
   task rather than an empty backend tab.

The dashboard shows the game version, languages, entity and change counts,
unresolved diagnostics, readiness summary, build status, test-deploy state, and
dependency/loadout status. If a deployment is active, it shows a human-readable
ownership/recovery summary; the exact installation receipt is available in its
Advanced details. It also offers recent projects, crash recovery, tutorials,
and the next useful action. “Build” never means “Deploy.” A hotfix that changes
a sealed source invalidates affected derived artifacts and opens the rebase
workflow; it does not silently rebuild against a different version.

### 4.2 Unified content browser

The first unified scope row is `This mod`, `Base game`, `Installed`, and
`Search all`. `Installed` may identify named installed sources in result
badges, but the Studio does not label them `Dependencies` until a real
dependency model owns that relationship. The implemented source pages provide
exact-current project content, a bounded generation-bound NPC/Quest
starting-point catalog, and a search-first installed DataAsset metadata
snapshot.

The bounded `Search all` v1 adds one explicit, case/accent-tolerant query over
those three projections. It loads each source independently, scans its
projection in memory, caps retained rows at 100 per source, and preserves
independent loading, complete, partial, and error states. Its buttons pass exact
same-source identities to the already bounded open/create-Draft/inspect flows.
It is not an atomic combined source snapshot, a dependency or reference index,
an implicit background index, or coverage of every content kind.
Project Work Bar Search is an exact focus handoff into this same view, not a
second search controller or cache: it activates `Search all`, waits for that
project-bound page to mount, focuses the existing field, and still performs no
source load until the author submits a nonempty query. Project switch, detach,
disposal, or a superseding handoff resolves the old request inertly.

The complete browser makes every result carry a kind, display name, source
badge, readiness/diagnostic badge, and change state. Technical identity and
exact origin stay in Advanced details. Saved filters, collections, tags, and
modules let authors organize a large mod by chapter, location, quest line,
owner, or production status. Search becomes incrementally indexed and
virtualized for large result sets while preserving the landed global query's
exact source boundaries.
Reference queries include:

- uses this NPC/item/line/voice/asset;
- used by this quest or dialog branch;
- unresolved or generation-stale origins;
- changed from vanilla;
- unreachable new content;
- content missing a translation or voice take.

### 4.3 Templates and cloning

Three operations must remain distinct even though the default labels are
author-facing:

- **Edit the original game object** (`Override vanilla` in Advanced help) keeps
  the vanilla runtime identity and stores only the authored delta.
- **Duplicate into my mod** (`Clone as new`) creates new stable project and
  runtime identities, retains a lineage pointer for comparison, and copies only
  selected dependencies.
- **Use without changing** (`Reference`) leaves the target unchanged and adds a
  typed edge.

Clone previews list every dependent localization string, voice, item,
DataAsset, dialog subtree, script symbol, and spawn record. Authors choose
`reference vanilla`, `clone`, or `omit` per dependency class. Built-in templates
include blank/clone NPC, fetch/talk/kill quest, one-shot conversation, branching
conversation, item reward, ambient bark set, and spawn group. Templates are
versioned project graph fragments, not executable scripts.

### 4.4 NPC editor

The semantic NPC inspector should cover:

- identity and origin; display/internal names; archetype and visual references;
- faction, level, core stats, talents, combat/AI profile, attitude defaults;
- inventory, equipment, trade inventory, and rewards through typed item refs;
- routines, locations, spawn/despawn policy, and world references;
- quest roles and dialog roots;
- localization and voice coverage;
- generated definition/config/script/spawn artifacts and any required DataAssets
  in Expert mode.

For an existing NPC, unsupported fields are read-only and preserved. An author
may create a **Draft-only NPC** skeleton, organize it, write its localization
and dialog, and link project references before the runtime backend exists. The
template explains before creation that the NPC is not build-ready. Production
build remains blocked until runtime discovery, distinct identity, spawning,
dialog/quest separation, and persistence are proven. The exact three-class
definition/config/spawn chain is already compile- and compose-qualified offline
for a bounded logical clone whose visual/actor defaults remain inherited from
Asghan-derived parents. Cooked
package/reference/collection support becomes an additional requirement only
when the chosen template needs new visual/content assets or the recovered
identity chain proves it necessary.
The UI must not claim that an unreferenced AngelScript class is a working new
NPC or include it silently in a bundle.

The current Workbench V1 checkpoint is an honest projection of that still-small
NPC model.
Selecting a managed `NpcDraft` opens **Profile**, **Story**, **Routine**,
**Inventory**, **Dialog & Voice**, **References**, and **Problems & Checks**.
Profile presents the friendly display name and the direct bounded **Edit name &
archetype** action. That form deliberately exposes neither `UniqueName` nor
module/source identities. **Problems & Checks** retains the separate exact-
current profile/source inspection and evidence-only compiler check. **Dialog &
Voice** now owns the ordered project-only greeting list: authors can attach,
create, reorder, or detach exact managed lines, preview localized text and Voice
coverage, continue into the existing Localization & Voice workspace, and
explicitly plan an empty Voice setup for one selected nonblank line/language.
That plan records production intent only; it creates no audio, topic, runtime
speaker binding, playable conversation, or spawn. Story, Routine, and Inventory
remain visible but explicitly unavailable because the Draft schema does not
model them. Outgoing and incoming links come only from the current
`Revision3ContentIndex`, while the problem count covers only unresolved
projected links. The three separate Draft-only, build-blocked, and
runtime-unqualified badges therefore remain mandatory.

### 4.5 Story workspace: quest and dialog

Story authoring is not graph-only. The same typed quest/dialog model has
synchronized **Outline**, **Script/Transcript**, **Graph**, **State table**, and
**Preview** views. The outline is fastest for structure, the transcript is
fastest for writing, the state table is fastest for bulk review, and the graph
is used where branching relationships matter. Selection and edits stay in sync
across views; no view owns a second copy of the story.

The graph contains quest states and dialog flow without conflating them. Node
kinds include quest state, objective, topic, line, player choice, condition,
action, branch, join, and end. Edges are typed; an action edge cannot be
connected where a condition is required. Reusable conditions/actions are
subgraphs with explicit inputs. Large stories use quest-line modules,
collapsible groups, focus mode, subgraph breadcrumbs, a minimap, deterministic
auto-layout, and lazy rendering. Authors are not expected to keep an entire
chapter as one visible spaghetti graph.

The line inspector contains all languages and all voice takes beside the text,
speaker, subtitle policy, audio preview, and lip-sync status. The topic
inspector contains participant and visibility semantics. Exact reflected class
and sentinel identities are generated/resolved by the lowerer when possible and
shown only in Advanced mode; unresolved identities are blocking diagnostics,
not text boxes silently accepted as valid.

Quest nodes expose states, transitions, objectives, log text, participants,
rewards, conditions, and effects. The graph statically detects unreachable
states, cycles where forbidden, missing terminal paths, effects that refer to
unknown states, and dialog choices whose visible paths have no valid result.
An offline condition/state simulator enumerates or samples authored scenarios
and previews reachable dialog, objectives, rewards, and terminal paths. It is a
semantic validator, not evidence that an unqualified runtime effect works.

The current managed-R3 subset is deliberately smaller than that end state.
The primary Story destination now directly searches and filters exact-current
NPC/Quest Drafts, creates another bounded Draft, and opens the selected entity
in Workbench V1 without detouring through Content. Content retains the same
global discovery projection, while non-Story reference targets return there.
Selecting a `QuestDraft` opens Workbench V1 with **Journey**, **Dialog &
Voice**, **References**, and **Problems & Checks**. Journey is a responsive,
objective-centered surface rather than the earlier technical fact sheet. One
read-only fan-in composes the exact
current content index, validated lifecycle seed, and ordered transcript only
when project/head, Quest/module, target/plan seal, objective order/slots, and
every line reference agree. It presents main-Quest and objective behavior across
Available/Start/Success/Failure, keeps stable-V4 dialog beside its objective,
keeps ungrouped dialog in a separate General area, and labels older generator-
v2/v3 behavior without inventing objective grouping. Journey hands off to the
existing atomic **Name & objectives**, **Description & connections**, and
**States & transitions** operations; it does not duplicate project state.
Disabled handoffs remain visible with an exact author-facing reason, while
unaffected offline actions remain usable. Selecting a Journey dialog row
switches this same exact Quest to **Dialog & Voice** and selects that row.
Problems reuses the existing exact-
current source/compiler inspection. Dialog & Voice hosts the bounded exact-
current ordered Quest transcript: friendly project lines can be attached,
reordered, grouped by stable V4 objective slot, detached without deletion,
previewed lazily, or
created and inserted atomically; one exact row/language can open in
Localization & Voice. Transcript mutations remain gated while localization
text is dirty or the managed checkpoint is busy/stale. Story exposes a
recommended two-checkpoint **Quest + opening line** recipe: the existing Quest
transaction publishes N+1, Studio rebinds to that exact checkpoint, and the
existing transcript transaction may publish the first line at N+2. Cancelling
the line form retains the usable Quest and opens its **Dialog & Voice** tab;
completion now preserves and selects the exact created row. No combined atomic
or playable-conversation claim is made. Journey loading is bounded to 256 lines,
does not repeat on harmless parent rebuilds, discards late/stale results, and
turns lost authority into a reopen requirement. The reference
projection is not build or runtime readiness. The lifecycle action itself is
one friendly behavior table over the fixed root/objective edges, with a
sequential template and bounded typed condition/action dialogs. Both slices
share the exact project/session publication lane, but the transcript grants no
runtime topic or selection effect and the workspace still has no synchronized general graph,
journal, reward, item, raw-source, complete simulation, build, deploy, or
runtime-test surface. The Studio labels the saved checkpoint Draft-only,
build-blocked, and runtime-unqualified.

Content remains the discovery surface, not a second authoring destination.
Selected current-project Quest/NPC Drafts now show a friendly discovery
summary and one explicit **Open in Story** continuation. Content renders no
duplicate Quest/NPC Workbench, edit, or source-check actions. The handoff
validates the exact index, entity, revision, project, and head again, then opens
and selects that same Draft in canonical Story Journey. Compact details close
before handoff; stale, repeated, or failed handoffs stay inert and never expose
technical error details. When a project requires reopening, the same canonical
continuation remains visible but disabled with the exact recovery instruction.

Problems now preserves the stable target instead of merely opening a broad
destination. Quest/NPC findings open that exact Draft in Story **Problems &
Checks**; other entities and project assets use revision-and-head-bound Content
navigation; DataAsset findings filter, select, and expand the exact staged edit.
Buffered requests fail closed on project, revision, or same-revision-head drift.
No route follows a newer checkpoint, and failed handoffs surface localized,
sanitized copy without technical exception details.

### 4.6 Voice at the line

The first bounded managed-R3 transaction/import slice is now connected to the
canonical **Localization & Voice** workspace.
It searches exact-current `DialogLine` content by speaker, line name, or
localization identity, limits each visible result set to 50, and requires an
explicit line choice. It can import a validated Ogg into the managed AssetStore,
create or extend the line's unresolved locale slot, retain multiple take
candidates and their Draft/Recorded/Reviewed/Approved status, and select only
an Approved take. The existing-take manager can now change that author-managed
status and immediately expose a newly Approved take for selection; the selected
take stays locked to Approved. Status is workflow metadata, not audible-game
qualification. The normal wizard deliberately does not edit dialog text: it
sends no text change and preserves the existing `LocalizationEntry` exactly.
Publication reloads the content index, binds the exact managed checkpoint, and
refreshes the visible project revision/head. This is the landed subset below,
not the complete production surface described by the remaining requirements.

The workspace now opens on a bounded **Work list**, with **Project texts** as a
peer view. Its projection has exactly two evidence-backed inputs: authoring
locales absent from editable project-local localizations and intact existing
Voice slots. Missing locale membership means “language not added,” not blank or
poor translation. A line without a slot produces no Voice row because the graph
contains no recording intent to complete.

For each existing slot, the decision precedence is zero takes → add recording;
no Approved take → review/approve; no valid Approved selection → select/repair;
unresolved or ambiguous target → resolve; otherwise → production decisions
complete. Draft/Recorded alternatives are optional backlog and do not regress
an already approved, selected, and resolved slot. Completion leads to the
existing checks; it does not mean Ready, buildable, deployed, audible, or
runtime-qualified. The row actions reuse the current exact locale, Add take,
Manage takes, and Resolve target flows rather than introducing queue-specific
mutations.

The normal projection retains at most 500 rows, prioritizes actionable items,
reports the omitted count, and keeps search plus work-kind/status filters. If
the Voice catalog cannot be verified, known language work remains visible with
an explicit partial-evidence warning. Catalog project/revision mismatch fails
closed. The host retains the exact managed root/project/revision/canonical-head
lifecycle; callbacks recheck the same catalog objects, exact line/locale,
derived next step, and head token around asynchronous work. Same-revision head
replacement, root/project replacement, late completion, dirty-text drift, and
`requiresReopen` cannot authorize an old item. Queue mutations share one
single-flight interlock and visible disabled reason.

Exact managed-take media QA is integrated as a separate on-demand action in
**Manage Voice takes**. One pathless read-only request binds an exact current
managed take through the complete line/localization/slot/take/asset chain,
performs a second Store and CAS read to close races, and reports integer
sample-frame duration with explicit assurance. Vorbis receives full-PCM-decode
assurance after validating initial PCM origin and EOS trim; Opus remains
honestly packet-and-timing-only. The dialog caches only the exact project,
line, locale, take, and take revision, discards the result on reload/mutation/
context drift, and exposes no media path, digest, or entity ID. Subtitle-
duration comparison, loudness/clipping checks, and batch review remain product
work; the action grants no project/game/save write, build, deployment,
audibility, or runtime authority.

For every line and project language, the author can:

- work in the line's automatically created language slot and import, record, or
  select a take without typing an archive path;
- import common author formats, preview, trim metadata non-destructively,
  normalize/transcode through an explicit reproducible derived-asset step, and
  see duration/channel/quality validation;
- compare subtitle duration with audio duration and mark intentional mismatch;
- retain multiple candidate takes per locale, mark their production status, and
  choose exactly one approved take for a build;
- reuse a take explicitly with a typed reference rather than duplicate a file
  accidentally.

The normal UI binds a selected `VoiceTake` to the automatically derived language
slot on a `DialogLine`; it does not ask the author to choose or maintain a
separate slot, a global ZIP-edit list, or a backend `add`/`replace` operation. A
resolver uses the sealed voice catalog, speaker, line, language, and qualified
naming rules to offer human-readable deployment-target candidates. A rule may
auto-propose, but never silently commit, only one case-insensitive exact basename
match for `${locId}.ogg` inside the selected language archive/layer. Zero
matches leave the target unresolved. Multiple exact matches remain ambiguous
until the author confirms a human-readable candidate. Neither case authorizes
an invented member path.

Current German-archive evidence is deliberately narrow: the active
`german_new.zip` catalog has 33,323 entries, and sampled canonical entry
basenames case-fold to the case-folded localization ID plus `.ogg`, including
`GRD_263_ASGHAN_OPEN_INFO_06_02.ogg` for Asghan and
`STT_302_VIPER_GREET_INFO_11_02.ogg` for Viper. This supports the exact-match
candidate rule for that sealed catalog, not a general claim about every
language, edition, line, or new-member runtime lookup. Exact archive/member
paths and `add`/`replace` remain retained and previewable only in the Advanced
section of build details. A new member target is never guessed from a basename,
and it remains Experimental or build-blocking until new-path runtime resolution
is qualified.

The read-only 2026-07-12 probe bound that evidence to an archive of 915,670,575
bytes with SHA-256
`ff7aa5b219c2e0ec9840570e42597d3ea3053a7ad592cf7cfa5b61dd895ecd29`.
It resolved Asghan uniquely to
`german_new/OldMine/Asghan/GRD_263_ASGHAN_OPEN_INFO_06_02.ogg`
(16,729 uncompressed bytes, CRC32 `5ab0fcfa`) and Viper uniquely to
`german_new/OldMine/Viper/STT_302_VIPER_GREET_INFO_11_02.ogg`
(26,191 uncompressed bytes, CRC32 `5805c926`). One cold index took 9.054 s and
one streaming archive hash took 2.113 s on the development machine. These are
diagnostic observations, not performance qualification; any archive-seal drift
invalidates them and must force target re-resolution before deployment.

### 4.7 DataAsset inspector: semantic and expert layers

The read-only DataAsset Lab opens a selected local `.uasset`/`.uexp` pair with
the exact `.usmap`, reports walked/partial/unsupported exports, and lazily
searches only proven offset-free fixed leaves. It still has no patch, stage,
save, pack, deploy, or raw-offset control. A separate first semantic slice now
turns one `editable=true` result into typed Bool/integer/float/color/vector
input, a friendly Before/After preview, and a callback for one managed stage;
it does not mutate the Lab's evidence-only contract.

The native authoring layer can now turn one fully verified PatchReceipt-v2
chain into a project-owned revision-3 stage. It freshly reconstructs the live
package, verifies the semantic fixed-leaf edit, union-probes the live target
generation, binds the executable, and persists only content seals, generation
facts, the offset-free selector/replacement, and closed denial statuses. The
patched pair, exact USMAP, and sidecars are ordinary immutable AssetStore/CAS
objects; receipt bytes, local paths, raw offsets, and ad-hoc authority are not
persisted. Prepare and registry-only removal create unpublished candidates;
listing is read-only. All three are exact-head, use bounded full-history reopen,
and never replace `gore-project.json`.

A second prepare-only native route accepts a separately produced, exact
ExtractReceipt-v2, its explicitly confirmed `/Game` target, an offset-free
inspector selector, and a typed semantic replacement. A read-only native route
first exposes only a verified target/package/USMAP/length summary; the guided UI
matches it to the inspection and requires visible target confirmation. Native
code then encodes the value, creates a private temporary PatchReceipt-v2 chain,
passes it through the unchanged full live-generation verifier, and returns only
the same closed R3 candidate/stage/head response plus a domain-separated intent
digest over target, canonical selector, and encoded replacement. Strict Dart
code recomputes that digest and validates the exact stage/target closure.

The guided summary/confirmation/Before-After workflow is connected to the
shared managed session and **Content > DataAssets** surface. Publication is bound to the
exact root/project/revision/head and reloads the managed checkpoint; no temporary
path, receipt, raw offset, or additional authority escapes. This first slice
edits only proven fixed-width leaves in the managed project.

The normal installed-package browser no longer requires an independently
materialized ExtractReceipt for that same narrow edit. It resolves one sealed
candidate ordinal, inspects the bounded in-memory package, and lets only an
`editable=true` selector open the typed Before/After editor. Save sends no
caller-selected target, package ID, package bytes, receipt, output path, or raw
offset. Native code rebuilds the package and USMAP inventories, independently
reconverts the target, and requires the complete pair, role-bearing sidecars,
opened UTOC set, consumed chunk winners/hashes, and exact USMAP name/bytes to
match before reusing the existing closed stage transaction. The IoStore reader
binds the bytes it actually parsed for every opened UTOC, including zero-chunk
children, to the held installed inventory so transient same-path swaps fail.
Strict Dart binds the source echo and two domain-separated proof digests before
the managed exact-head publication lane may advance.

Neither typed route creates a new package identity or structural asset. They do
not build, pack, deploy, write the game installation, touch a save file, or
qualify gameplay/runtime behavior. Expected hotfix/source drift discards the old
inspection but keeps the managed project usable; an unconfirmed publication
outcome closes the evidence chain and requires a project reopen.

The semantic layer is a registry of reviewed schemas and widgets for known
gameplay concepts. It provides domain names, units, ranges, typed references,
and invariant-aware collection editors. The expert layer shows the complete
decoded property tree, schema owner/type, origin seal, on-wire support status,
and raw value. Unknown or structurally unsafe values are preserved and
read-only.

Editing an existing fixed leaf can lower to the current receipt-bound path.
Creating or structurally editing an asset requires a separate writer that can
correctly rebuild names, imports, exports, object/package references,
unversioned headers, and collection shapes, then reopen and semantically verify
the complete package. Until those proofs exist, the UI must not emulate them
with byte patches.

#### Planned optional Unreal Editor hybrid

For explicitly reviewed DataAsset, visual-media, and world-content operations,
the long-term architecture may use Unreal Editor as an optional specialist
surface instead of rebuilding its native visual tools in Mod Studio. This
bridge is **not implemented**. A valid handoff must be bounded, versioned, and
sealed to the selected game generation, exact input assets, declared semantic
identities/references, adapter version, and required editor/plugin identity.
Re-import verifies the declared outputs, reopens them through the applicable
bounded validators, and applies them as one ordinary Mod Studio transaction.
Launching an editor, exporting loose files, or noticing a new `.uasset` is not a
round trip.

Mod Studio remains the sole source of truth for semantic IDs, references,
project history, provenance, validation, and Build/Test/Release. The Unreal
workspace is an explicit tool workspace, not a second implicit project state,
content registry, deployment path, or qualification authority. The handoff
never writes the game installation; accepted source and output artifacts enter
the project only through its managed AssetStore/import contract.

Nothing here claims that stock Unreal Editor can open cooked G1R packages or
produce game-compatible cooked output. A nominal engine-version match is not
sufficient: game plugins/custom types, package/reference/cook/registration
chains, deterministic reopen, and runtime behavior require separate evidence
for the exact operation and game generation. Unsupported outputs remain
Draft-only and build-blocked. The bridge is contextual to eligible DataAsset,
visual, or World workflows and Expert mode, never a permanent backend-format
tab or a prerequisite for ordinary supported authoring.

### 4.8 Items, world, and spawns

Items use the same Override/Clone/New authoring choices as NPCs. The semantic
editor covers stats, category, visuals, equip behavior, descriptions,
recipes/rewards, and localization. Their runtime implementation must not be
assumed to share the NPC cooked-DataAsset chain: current game evidence includes
vanilla item definitions as AngelScript `UItemDefinition` subclasses, while a
new item's exact discovery, construction, persistence, and ancillary-reference
requirements remain unproven. An item that reuses qualified existing visuals
may eventually need only a script-class path; new visuals, placed instances, or
registries may additionally require cooked assets. Each operation stays blocked
until its actual chain is recovered and qualified.

World authoring starts conservatively with explicit spawn/routine records that
reference an existing qualified world anchor. A map view can visualize anchors
and coordinates, but writing arbitrary levels or world-partition cells is a
separate capability and must not be implied by a pin-on-map UI. Every spawn
shows which NPC, transform, activation condition, persistence policy, and quest
dependencies it uses.

The planned optional Unreal handoff does not widen this boundary. Until one
specific world operation has a sealed export/import contract, complete package
reopen, and generation-qualified runtime evidence, Studio must treat its output
as Draft-only and must not present an Unreal-authored map, actor, or partition
cell as writable game content.

### 4.9 Localization workspace

The workspace combines source text, every target language, fallback behavior,
speaker/context, references, character limits, voice duration, take status, and
missing coverage. Its virtualized table supports multi-select, fill/transform,
spreadsheet paste, mapped import, and transactional undo. Bulk CSV uses stable
line/entity IDs and locale columns, exports a schema/version header, and rejects
unknown or duplicate IDs on import. New IDs are explicitly project-owned and
collision-checked against the selected game version and other project entities.

The current bounded Work list is the first production projection toward this
end state. It covers only absent authoring-locale membership and next decisions
for already-existing Voice slots. It is not the complete translation/Voice
coverage dashboard, blank/quality analysis, batch assignment system, or team
review queue described above.

### 4.10 Validation, build, isolated test, deploy, and undeploy

Validation runs incrementally but build always revalidates a snapshot. The
review screen groups diagnostics by authoring concept and offers navigation and
transactional quick fixes. Its default build plan shows semantic changes and
conflicts in author-facing terms. A generated-artifact tree and the exact target
set used for conflict analysis are available through an explicit Advanced
expander, never required to complete the normal workflow.

Test deployment has an explicit risk profile:

- **Offline only** builds and inspects artifacts without game writes.
- **Render-safe dialog probe** permits only the already proven natural
  insertion/render path and asks the user not to select a topic.
- **Disposable-save gameplay test** is required for topic selection, quest
  effects, knowledge changes, persistence, or new NPC behavior.

A `TestScenario` records the intended content scope, risk profile, entry
instructions, optional qualified setup, expected observations, target loadout,
and cleanup policy. **Run isolated test** orchestrates the bounded lifecycle:

1. validate a canonical snapshot and show any experimental mechanisms;
2. require the game to be closed and preflight the installation/loadout;
3. create a named isolated profile and, only with explicit consent, copy a
   user-selected save into it;
4. build and deploy only to that profile, preserving an exact receipt;
5. launch through the qualified path or present manual entry instructions;
6. capture bounded logs and author observations without changing gameplay
   state merely to make the test pass;
7. after exit, compare the disposable save semantically where supported,
   undeploy, remove owned isolation artifacts, and verify the clean-state
   inventory and hashes.

The Studio never edits a normal save as part of authoring or testing and never
silently selects one. A scenario may automate starting state only through a
separately qualified mechanism; otherwise the setup remains explicit manual
instructions. Runtime adapters must not grant or activate abilities, run global
object scans, or use timer/console shortcuts to manufacture invalid
conversation state. Cancellation, game crash, Studio crash, or power loss leave
a durable recovery receipt, and the next Studio start offers verification and
cleanup before another deployment.

Deployment requires the game to be closed, verifies the target generation and
current install/loadout ownership, previews every write, and records backups and
hashes before publishing. Undeploy verifies current ownership and restores the
recorded state; drift produces a blocking recovery choice rather than an
overwrite. A successful test cycle ends with a clean-install report and no
loader, backup, staging, or isolation residue beyond deliberately retained
project/build artifacts.

### 4.11 Undo, redo, history, and expert source

All edits, imports, clones, graph rewrites, and quick fixes are named
transactions. Undo and redo apply inverse graph deltas atomically across
domains. Autosave appends a crash-recovery journal before updating the project
snapshot. Named checkpoints allow comparison and restore without pretending to
be a distributed version-control system.

Expert source is an explicit escape hatch:

- inspect and copy generated AngelScript, localization JSON, voice manifest,
  DataAsset selector/receipt facts, and `BuildSpec`;
- replace a generated script fragment with a project-owned source artifact;
- mark the entity as expert-owned so the semantic editor does not overwrite it;
- re-enter semantic mode only through a parse/validation/import operation, never
  by discarding source silently.

### 4.12 Compatibility, dependencies, rebase, and release

The product keeps four version concepts visibly separate:

- current project schema version, changed in lockstep before release without
  legacy readers or migrations;
- authored mod version and release channel;
- target game version/edition and qualified compatibility range;
- checkpoint, build, deployment, and release identities.

A project can declare required or optional mod dependencies, compatible
versions, and load-order constraints. The base-game and dependency catalogs are
indexed as separate sealed layers. Validation detects exact deployment-target
collisions, unresolved dependency entities, incompatible versions, and known
load-order conflicts. It explains the affected authoring concepts and offers a
merge or precedence choice only where a domain-specific merge is proven; byte
collisions are never resolved by guesswork.

When a hotfix changes an origin, **Rebase game version** compares the old sealed
base, the new sealed base, and the authored delta. Per entity/property, the
author can keep a still-valid delta, reapply it to a qualified new target,
accept the new base, or leave it blocked. The original project and last good
build remain available until the rebased snapshot validates, saves, and
reopens. Rebase never silently retargets by display name.

Build produces an inspectable artifact; test deployment installs that artifact
into an isolated profile; release packages a previously validated snapshot.
Release includes mod version, compatibility range, dependency/load-order
metadata, description, icon, authorship/license, changelog, content/risk
summary, payload hashes, and deterministic provenance. A release gate rebuilds
from the canonical snapshot, reopens the bundle, runs the supported clean
install/undeploy checks against fixtures or explicit test profiles, and refuses
to publish when production-blocking Draft/Experimental content remains.

## 5. `AuthoringProjectV2`

`AuthoringProjectV2` is the durable authoring model. It is not
`gore_mod::BuildSpec` with extra UI fields. `BuildSpec` remains a compact,
declarative deployment intermediate representation; the V2 graph retains
author intent, origins, dependencies, reusable source assets, and history.

Conceptual schema (names are normative; exact Rust/Dart representation may
vary):

```text
AuthoringProjectV2 {
  format: 2,
  project_id: ProjectId,
  meta: ProjectMeta,
  target: GameGenerationAnchor,
  settings: AuthoringSettings,
  entities: ordered map<EntityId, Entity>,
  roots: ordered set<TypedRef<Entity>>,
  asset_store: AssetStoreIndex,
  checkpoints: list<Checkpoint>,
  extensions: closed, versioned extension records
}

Entity {
  id: EntityId,
  kind: EntityKind,
  display_name: string,
  origin: OriginRef,
  revision: integer,
  payload: kind-specific closed record
}
```

### 5.1 Stable IDs and typed references

`ProjectId`, `EntityId`, `TransactionId`, and `CheckpointId` are immutable
128-bit IDs serialized in one canonical lowercase form. New IDs are random or
time-sortable and are never derived from a display name, technical runtime name,
list index, or file path. Renaming and reordering therefore cannot break a
reference.

`TypedRef<T>` contains an expected entity kind (or a closed allowed-kind set)
and one catalog-qualified target:

```text
ProjectTarget {
  project_id: ProjectId,
  entity_id: EntityId
}
CatalogTarget {
  layer: CatalogLayerRef,
  catalog_entity_id: CatalogEntityId,
  source_seal: Sha256
}
```

`CatalogLayerRef` names exactly one base-game or mod-dependency layer, including
its provider identity, edition, pinned version, and loadout-layer identity.
`CatalogEntityId` is canonical within that layer and is derived from a reviewed
domain selector, never from a display name or result-list position. Two layers
may intentionally contain the same selector or catalog-local ID without making
the reference ambiguous because the layer is part of the target. Local project
references carry `project_id` so copying a shard between projects cannot
silently retarget it.

Deserialization and resolution validate project/layer identity, source seal,
existence, and kind agreement. References never search other layers or fall back
to a display-name match. An optional human-readable hint may improve diffs but
has no resolution semantics. Deleting a referenced project entity requires an
impact preview and either cancels, cascades through explicitly owned children,
or leaves typed `UNRESOLVED_REF` diagnostics; catalog targets are read-only and
can become stale, but are never deleted or silently rebound by the project.

### 5.2 Origin references

Every entity has exactly one `OriginRef`:

```text
New { authored_runtime_id }
Catalog { layer: CatalogLayerRef, domain, canonical_selector, source_seal }
Imported { importer, source_seal, external_identity? }
Generated { generator_id, generator_version, owner: TypedRef }
```

A clone receives a new `OriginRef::New` and a separate non-resolving lineage
record pointing to its source. An override retains `OriginRef::Catalog`, whose
layer says whether the source is the base game or a pinned dependency.
`canonical_selector` is domain-specific and offset-free: for example an exact
class identity, localization ID, archive/member pair, package/export/property
selector, or script module identity. `source_seal` makes generation drift
diagnosable and prevents accidental cross-hotfix lowering.

### 5.3 Content-addressed `AssetStore`

Binary and source payloads are immutable blobs addressed by lowercase SHA-256:

```text
AssetRef { sha256, byte_len, media_type, logical_name, provenance? }
assets/sha256/<first-two-hex>/<remaining-hex>
```

The blob path is derived only from the digest. Import streams into private
staging, enforces per-type and aggregate limits, verifies the final digest, and
publishes without replacement. Equal content deduplicates even when filenames
differ. Logical names are metadata and cannot influence extraction paths.
Untrusted project archives reject duplicate logical entries, traversal,
absolute paths, links/reparse points, case aliases, undeclared blobs, hash/size
mismatch, unknown required fields, and resource-limit overflow.

Generated assets are also content-addressed but carry a generator/version and
input fingerprint. They may be evicted and rebuilt; authored imports may not.
Undo retains referenced blobs until project compaction, so undo never depends on
an external source file that may have moved.

### 5.4 Diagnostics

A diagnostic is structured data:

```text
Diagnostic {
  code, severity: info|warning|error,
  entity: EntityId?, property_path?,
  message, evidence[], related_entities[],
  blocks_build: bool,
  quick_fixes: list<QuickFixId>
}
```

Codes and blocking semantics are stable API. Examples include
`UNRESOLVED_REF`, `ORIGIN_GENERATION_MISMATCH`, `MISSING_TRANSLATION`,
`VOICE_TARGET_AMBIGUOUS`, `QUEST_STATE_UNREACHABLE`,
`DIALOG_SELECTION_EFFECT_UNPROVEN`, `DATAASSET_SHAPE_UNSUPPORTED`, and
`RUNTIME_DISCOVERY_UNPROVEN`. A quick fix returns a proposed transaction and
diff; it does not mutate during diagnosis.

### 5.5 Transactions and history

A transaction has an ID, label, timestamp, base project revision, ordered typed
deltas, inverse deltas, diagnostics delta, and asset-reference delta. It either
commits completely against the expected base revision or does nothing. Redo is
invalidated only by a new committed branch, not by navigation or validation.

Saving writes a complete canonical logical snapshot to private sibling staging,
reopens and validates it, then performs a no-clobber/replace-safe publication.
The crash journal is bounded, checksum-framed, and replayed only when every base
revision matches. Checkpoint compaction retains the current graph, named
checkpoints, and blobs reachable from either; it never deletes external files.

### 5.6 Large-project working format and indexes

The conceptual ordered entity map is the canonical logical model, not a mandate
to load or rewrite one giant JSON object. The primary editable form is a project
directory with a small manifest, deterministic entity/module shards, the
content-addressed AssetStore, and a bounded journal. A `.goremod` project archive
is a deterministic interchange/backup artifact; a release bundle is a separate
output. Opening an archive for editing imports it to a working directory rather
than treating ZIP members as mutable files.

Shard boundaries are stable by entity/module ID, never by display name or UI
order. Publication makes a new manifest visible only after every referenced
record and blob is durable and reopens successfully; the previous manifest
continues to describe a complete snapshot until then. External modification is
detected by revision/hash and produces a compare/reload choice rather than a
last-writer-wins overwrite. A canonical export provides useful text diffs, and
generated indexes/caches are explicitly disposable and excluded from semantic
project history.

Base-game and dependency entities live in read-only catalog indexes keyed by
edition, game version, loadout layer, and source seal. They are not copied into
the authored graph merely to make them searchable. The project stores
catalog-qualified `TypedRef` targets, typed `OriginRef`s for local deltas, and
the exact layer seals used for authoring; an index resolver supplies display
data, search, backlinks, schema, and capability facts. Hotfix or dependency
drift invalidates only the affected catalog targets, origins, and cache records
without mutating or opportunistically rebinding the authored snapshot.

Search, backlinks, diagnostics, and coverage use incremental indexes and lazy
loading. Lists/tables are virtualized; graph views load a focused subgraph; full
validation streams the closed logical snapshot rather than relying on what the
UI has opened. The initial reference fixture is at least 100,000 catalog rows,
10,000 authored entities, 50,000 typed edges, and 5 GB of deduplicated blobs.
The reference baseline is a release build on 64-bit Windows 11, an AMD Ryzen 5
5600X, 16 GiB RAM, and a PCIe-3 NVMe SSD. Every result records the exact OS,
Studio build, CPU, RAM, storage model, power plan, fixture seal, and cache mode.
On that baseline:

- a cold Studio process with valid on-disk indexes makes the project shell
  interactive within 3 seconds; background freshness checks do not block it;
- first-time index construction is a separate cold-index measurement, reports
  progress within 250 ms, remains cancellable within 1 second, and leaves
  unindexed scopes visibly unavailable rather than returning incomplete results;
- after indexes are ready and one unmeasured warm-up query, 1,000 deterministic
  mixed global-search and backlink queries return their first page at p95 within
  150 ms;
- 1,000 deterministic common edit/undo operations respond at p95 within 50 ms,
  with the corresponding durable journal acknowledgement at p95 within 250 ms;
- no UI operation materializes the complete catalog or an unbounded graph.

Phase 1 additionally uses a voice fixture with at least 35,000 entries, duplicate
basenames, and a ZIP size of at least 1 GiB. On the same baseline, indexing an
already prepared unique-path copy with a cold file cache completes within 8
seconds and consumes at most 256 MiB incremental resident memory. After one
warm-up, 1,000 exact/case-folded target lookups complete at p95 within 50 ms, and
an existing-member preview starts within 2 seconds. Index, preview, transcode,
and copy-on-write build work never runs synchronously on the UI thread, reports
progress within 250 ms where it lasts longer, and acknowledges cancellation
within 1 second at a safe boundary. Index/preview must not read or copy every Ogg
payload; the source voice archive is never imported into the project AssetStore.

The small structurally equivalent fixture runs per change. The full catalog,
5-GB AssetStore, and 1-GiB voice benchmarks run in the scheduled performance
suite and before each affected milestone release. These budgets are regression
gates, not proof of runtime gameplay behavior.

## 6. Entity and dependency model

The initial closed `EntityKind` set should include:

```text
ProjectModule, Collection, Chapter, QuestLine, Location,
Npc, Quest, QuestState, QuestObjective, GameFact,
DialogGraph, DialogTopic, DialogLine, Condition, Effect,
VoiceSlot, VoiceTake, LocalizationEntry,
Item, Inventory, DataAsset, ScriptModule,
WorldAnchor, Spawn, Routine,
FmodReplacement, TextureReplacement, ExpertArtifact,
ModDependency, BuildProfile, TestScenario, TestRun, Release
```

Kinds can gain versioned payload revisions, but an unknown required kind blocks
editing/build instead of being dropped. Organizational and workflow entities do
not become runtime content merely because they exist in the graph.

### 6.1 NPC dependency contract

An `Npc` owns or references:

- one logical runtime identity plus its qualified definition/configuration chain;
  current evidence points first to linked AngelScript `CharacterDefinition`,
  `AIAgentConfig`, and `SpawnAIAgentDefinition` classes/CDOs, with DataAssets
  required only where the recovered archetype or new content actually uses them;
- localized display names and optional descriptions;
- typed faction/stat/AI/visual configuration;
- inventory/equipment and item refs;
- zero or more routines and spawns;
- quest-role refs and dialog-graph roots.

Owned generated records are deleted or cloned with the NPC only after an impact
preview. Shared items, localization, voices, and quest graphs are referenced,
not implicitly owned. The first bounded new-NPC target is the offline-proven
script-only logical clone with a new `UniqueName`, new linked
definition/config/spawn classes, and inherited visual/actor defaults. It is
production-buildable only when runtime class residence, distinct identity,
spawn, dialog/quest separation, and persistence are supported for the chosen
archetype. Cooked package, reference-table, or
registry/collection lowerers are additionally required only when that template
introduces new visual/content assets or the recovered chain proves they are
necessary.

### 6.2 Quest dependency contract

A `Quest` owns states/objectives and references localization, participants,
dialog topics, items/rewards, conditions, and effects. Each transition has an
explicit source state, target state, trigger, condition set, and effect set.
There is exactly one declared initial state and at least one reachable terminal
state unless the quest template explicitly permits an ongoing quest.

Dialog can observe or transition quest state only through typed `Condition` and
`Effect` entities. A visible choice with an unproven selection effect is allowed
in the project as experimental content but blocks a production build. The graph
does not infer quest semantics from arbitrary source strings.

For the currently tested game version, automatic discovery and instantiation of
two added `UQuest` subclasses is runtime-proven narrowly: both appeared as
`Available` runtime instances after world/save load. Separately, the bounded V4
model and lowerer now cover the fixed Available/Start/Success/Failure lifecycle,
typed state predicates, cross-node Start/Succeed/Fail actions, and parent
completion. A 1.0.3 qualifier compiled, remapped, composed, reopened,
decompiled, and disassembled the four external-trigger fields and predicate-hook
shapes, all three handler shapes, `bSucceedParent`, typed getters, and guarded
lifecycle-call shapes in two independent runs with a byte-identical final cache.
It did not compile one renderer-produced fixture spanning every supported
state-test expression. That is stronger offline compiler/cache evidence, but
not evidence that the game polls or orders the hooks as modeled.

The native scan's internal discovery algorithm, other versions, exact runtime
transition and handler order, selection-driven effects, journal/rewards/items,
and persistence remain unqualified. Before a quest lowerer or effect is marked
production-supported, its generated source and exact target must validate
offline and every claimed transition/effect/persistence mechanism must pass
bounded disposable-save proof. Successful AngelScript compilation or class
discovery alone is not sufficient.

### 6.3 Dialog and voice dependency contract

A `DialogGraph` references participants and owns topics/branches. A
`DialogTopic` references its root line or choice subtree plus conditions. Each
`DialogLine` references a qualified speaker (or an explicit narrator/system
role), one `LocalizationEntry`, and zero or one automatically derived
`VoiceSlot` per locale. Creating or importing a take creates the slot when
needed; authors do not select a slot as a separate backend object. A `VoiceSlot`
contains candidate `VoiceTake` refs, at most one selected take, and one closed
target-resolution state:

```text
Unresolved { reason }
Ambiguous { candidates: list<VoiceTargetCandidate> }
Resolved {
  archive, member, operation,
  catalog_layer, source_seal,
  rule_id, rule_version
}
```

An existing-member exact match resolves to `replace`. An `add` target may
resolve only from a separately qualified, sealed new-member namespace/rule that
supplies the complete member path and proves exact absence; a zero-match
basename search alone remains `Unresolved`. `VoiceTargetCandidate` is deployment
evidence and is distinct from a `VoiceTake`, which references authored source
media, reproducible derived Ogg, duration/codec metadata, and production status.
A take does not acquire a deployment identity merely because it was imported.
Multiple slots may reuse a take only through explicit refs.

The currently proven runtime adapter is one possible lowering for topic
insertion. It does not become evidence for selection effects, quest state, or
automatic discovery.

### 6.4 Organization, dependencies, tests, and releases

`ProjectModule`, `Collection`, `Chapter`, `QuestLine`, and `Location` provide
stable organization and scoped queries without changing ownership implicitly.
Deleting a folder-like entity does not delete its members unless a separately
previewed ownership transaction says so. A `GameFact` represents a reviewed
knowledge/quest-state concept that conditions and effects can reference; an
unknown arbitrary source expression is not promoted into a semantic fact.

A `ModDependency` identifies a dependency and compatible version range, pins the
catalog/source seal actually used for authoring, and defines whether references
are required or optional. A `BuildProfile` selects a target game version,
dependency loadout, production/experimental policy, and output intent; it never
weakens a generator's proof gate. `TestScenario` owns declarative setup and
expected-observation records. `TestRun` is an immutable result/receipt referring
to the exact project revision, build, profile, logs, save comparison, and
cleanup verification.

A `Release` refers to an immutable validated project revision and exact build
provenance plus human-facing metadata, compatibility, dependencies, and
changelog. Editing content after a release creates a new project revision; it
cannot mutate the released payload in place.

## 7. Deterministic lowering to `gore_mod::BuildSpec`

The compiler pipeline is:

```text
canonical V2 snapshot
  -> resolve typed refs and origins
  -> project-wide semantic validation
  -> domain generators in dependency order
  -> reopen/verify generated artifacts
  -> gore_mod::BuildSpec + provenance map
  -> existing gore-mod bundle builder
  -> reopen/inspect bundle and target manifest
```

The `gore-as` diagnostics report path now also exposes a typed
`InstallRestoreDisposition` independently from compiler success and captured
diagnostics. It distinguishes a generator that was not started, an exactly
restored installation, an unconfirmed process exit requiring recovery, and a
restore/finalization failure with retained recovery artifacts. This is a safety
foundation for callers of `CompileModuleReport` and `GameRunRegenReport`; it
does not infer cleanup from a syntax-error string or treat compile success as
proof of restoration. The compatibility Script tab and managed exact-current
Quest/NPC checks consume this typed status through strict FFI, Dart, and UI
paths, keep recovery failures dominant, and accept compiled evidence only after
exact restoration. The managed action discards compiler output and grants no
production build, deploy, runtime, spawn, or publication authority.

The current `BuildSpec` fields are deployment IR and receive these results:

| V2 source | Existing `BuildSpec` target |
|---|---|
| Scalar existing-class overrides | `overrides` |
| `LocalizationEntry` deltas/new IDs | `loc_edits` |
| `FmodReplacement` | `audio` |
| `TextureReplacement` | `texture` |
| Generated or expert-owned compiled modules | `scripts` |
| Proven guarded topic registrations | `dialog_topics` |
| Selected locale-specific `VoiceSlot`/`VoiceTake` targets | `voice` |

NPC, quest, dialog, item, and spawn entities may generate multiple rows and
artifacts. Generator order is based on typed dependencies, not UI/list order;
all maps and sets serialize canonically. The provenance map links every
`BuildSpec` row and generated byte artifact back to entity ID, property path,
generator version, and input fingerprint, enabling meaningful build errors.

The existing `BuildSpec` has no general cooked-DataAsset or world-content input.
V2 lowering therefore must **block** a project that requires such output today.
Once a verified cooked-package generator exists, add a backwards-compatible,
typed `BuildSpec` component (or another explicit gore-mod deployment IR field)
rather than smuggling a triplet through texture or script fields. The high-level
contract remains V2 -> deployment IR -> gore-mod; V2 must not deploy files
directly.

Expert source follows the same path. It can replace a generator output only
after declaring its exact inputs/outputs and passing the same bounded parser,
compiler, artifact reopen, target, and generation validation.

## 8. Phased roadmap

Each phase requires its offline acceptance gates below. Runtime claims require
additional targeted runtime proof; offline green tests alone never certify
discovery, rendering, selection, persistence, or spawning.

Current execution priority is **completion before expansion**. Problems &
Readiness V1 and honest prerequisite gates are the first landed usability
checkpoint. Project-local dialog-line/localization creation or exact managed
reuse plus direct full-text/locale editing and full reopen are now landed
bounded slices, not vanilla adoption. The card-only Story destination has also
been replaced by a direct exact-current NPC/Quest search/filter/list plus the
existing Workbench; this improves access but does not make the underlying Drafts
buildable or runtime-qualified. One reviewed managed DataAsset stage can now be
built, reopened, and re-inspected in a receipt-owned offline output, and one
exact R3 checkpoint can be exported through the current V2 Studio workflow as a
deterministic restorable backup. The visible restore flow inspects it read-only,
materializes it on Windows into one absent exact managed-project directory
through archive CAS, private staging, head-last verification, and atomic
no-clobber promotion, then adopts only an exact receipt-bound full reopen. It
preserves project identity and returns no adoptable receipt on publication
uncertainty. Clone/Save As and deliberate uncertainty/staging recovery remain
missing. The first Voice production
Work list is also landed:
Localization & Voice defaults to bounded missing-language and existing-slot
next steps while Project texts stays one switch away. Project Work Bar V1 is
also landed across every destination: persistent project/current-area
orientation plus Search/Create/Problems routes into the existing owners,
including exact Search-all focus with zero pre-query source loads and compact
accessible overflow. It adds no new authority; full command-palette,
undo/redo, compatibility, build/test, and broader workspace chrome remain.
The recommended **Character + first greeting** recipe now also removes the NPC
creation-to-writing dead end by composing the existing Character and greeting
transactions across two explicit exact checkpoints. Its partial NPC-only result
persists safely, while completion selects the inserted first line; it is not a
playable conversation or spawn. The first explicit existing-line/locale slot
relationship is also landed: **Plan recording** creates one exact empty,
unresolved VoiceSlot without audio or a game-installation dependency, after
which the existing Work list owns the next recording step; the inverse empty-
slot removal remains separately confirmed. Next, (1) complete the broader
semantic dialog/NPC/topic relationships and localization/Voice production
tools; (2) finish semantic/project
deletion, shared undo/history, and broader recovery, with Close and bounded
same-lock recovery already landed; (3) deepen the existing NPC and Quest
journeys beyond their direct workspace; and (4) complete honest managed
build/release and qualified test paths. Broad World or level authoring remains frozen
until these workflows are usable end to end. World may stay visible as an
unavailable destination, but research or a placeholder must not displace this
queue.

### Phase 0: cross-cutting authoring contract

Define the task-based information architecture, onboarding/capability preview,
author-facing vocabulary, working-directory format, sealed catalog indexes,
stable IDs/refs, AssetStore, transactions, autosave/recovery, and
Draft/build/runtime readiness. Establish the benchmark harness, a small
structurally equivalent per-change fixture, baseline keyboard navigation,
screen-reader semantics, focus order, resizable panes, and command/search
contracts at the outset; these are architectural inputs, not final polish.

Phase 0 is not a separate big-bang implementation that must finish before users
receive Phase 1. Its contracts are delivered incrementally through vertical
slices: Phase 1 implements the minimum real subset, and later phases expand it
without creating another independent project state or permanent backend tab.
The full reference-scale catalog gate becomes mandatory when the unified browser
ships in Phase 2; the real-scale voice-archive gate applies to Phase 1.

### Phase 1: first vertical slice — Voice at a dialog line

Deliver the first usable subset of the Phase-0 contract through stable catalog-
qualified refs, AssetStore, exact transactions, and the managed revision-3
line-centric Voice editor. The author finds a line by visible speaker/text,
uses its derived language slot without seeing archive paths, imports and
validates a take, selects an existing Approved candidate, survives save/reopen
without external paths, resolves an exact installed existing-member target,
and builds a verified sealed format-3 offline bundle from owned Store bytes.
Exact-current preview is landed, while broader production and runtime
qualification remain part of completing the production editor. The first valid
multilingual acceptance build uses two exact existing-member replacements.
New-member `add` retains a separate offline Experimental acceptance case and is
not part of the managed production builder until its exact target rule and
runtime lookup are separately qualified.
This is first because the backend and deployment contract already exist and it
immediately turns three disconnected capabilities—dialog text, audio files, and
bundles—into one non-technical workflow.

The guided V1 prerequisite now lets a fresh managed-R3 project create its
project-local `DialogLine`/`LocalizationEntry` pair and optional empty slot, or
reuse one exact currently unused managed localization. Reuse now includes a
bounded, read-only, exact-head localization preview and permits only locales
with non-whitespace text; publication rechecks the same exact entity. The
optional speaker value remains a label and grants no NPC binding. This closes
the former fresh-project dead end. Localization & Voice also now provides a
direct search/list/editor workspace for the complete bounded text map of intact
project-owned localizations. It hides technical identity, shows shared-line
backlinks, preserves VoiceSlot locales, locks candidate-backed transcripts, and
publishes only the exact localization/project revision delta through the
managed session. The default Work list now adds the first truthful production
queue over that same authority: absent authoring locales and existing Voice
slots only, strict next-step precedence, exact line/locale handoff, a 500-row
retention bound, and explicit partial Voice evidence. “Production decisions
complete” remains narrower than Ready/build/runtime, and absence of a VoiceSlot
never invents missing-recording intent. These slices do not adopt vanilla data
or complete this phase:
the new line remains build-blocked and runtime-unqualified and still needs the
installed-target, broader production, deployment, and qualified audible-runtime
journey. Exact-current managed-CAS take preview is now landed as a separate
read-only in-app capability: it verifies the complete graph and selected Ogg,
materializes one fixed temporary leaf through a natively retained opaque cleanup
token, and grants no project, build, game, save, deployment, or runtime
authority. It creates no topic or AngelScript; bulk translation, broader
coverage and batch/team review queues, CSV/XLIFF, history, and general semantic
line/slot editing also remain.

### Phase 2: unified browser, references, templates, and history

Move the existing item/localization/dialog/FMOD/texture/script domains behind
the V2 graph. Deepen the landed bounded global search into indexed, virtualized
search plus dependency/reference views; add named transactions, undo/redo,
checkpoints, clone preview, and a multi-select table
editor, spreadsheet paste/import mapping, and transactional CSV. Complete the
command palette, shortcuts, focus/accessibility audits, saved collections, and
translation/voice coverage dashboard at this stage. Add isolated test profiles,
the receipt-driven test lifecycle, dependency/loadout conflict validation, and
the first deterministic release package. Rebind useful old domain-editor
widgets to managed R3, then delete their prior providers; do not maintain two
independent project states.

### Phase 3: semantic existing content and early Draft authoring

Add reviewed schemas for existing NPCs, items, known DataAssets, and dialog
relationships. Integrate the fixed-leaf DataAsset workflow through semantic
selectors and receipts. Existing-class and existing-asset overrides come before
production new-identity creation. Recover the vanilla quest catalog and enough
of its exact representation for a strict typed model. Add a **Draft-only NPC**
skeleton/editor around the offline-proven linked script-class chain and
**Draft-only Quest** templates/schema/offline source generator. The narrow new-`UQuest`
discovery proof is recorded in the capability registry, but unqualified behavior
keeps generated quests out of production builds. Add project-wide
generation/rebase diagnostics, the full three-way rebase workflow, offline
semantic diff/build-plan inspection, and batch edits for compatible semantic
fields.

The landed bounded NPC subset now includes the recommended two-checkpoint
Character-plus-first-greeting continuation. It publishes the NPC-only Draft at
N+1, fully rebinds the exact managed checkpoint, and may create and insert one
localized greeting at index zero at N+2. A stopped second step retains the
honest NPC-only checkpoint. This is workflow composition over two existing
transactions, not a compound atomic graph, runtime dialog, spawn, or build.

The landed bounded Quest subset already provides ordered Draft creation, a
guided two-checkpoint Quest-plus-opening-line continuation, separate
existing-Quest outline and catalog-bound context edits, and the first
fixed-lifecycle V4 **States & transitions** table. The guided continuation
composes existing exact transactions and retains a Quest-only N+1 checkpoint if
line authoring is cancelled; it is neither a synchronized general graph nor a
playable dialog. The V4 slice persists a
closed stable-slot plan, validates bounded typed conditions/actions, and lowers
deterministic source. Reading or deriving a V2/V3 plan is a byte-preserving
no-op; only an explicit behavior edit upgrades it, while separate outline or
context edits still regenerate their owned fields. It remains prepare-only at
the native boundary, publishes only through
the managed exact-head CAS lane, and is build-blocked/runtime-unqualified.
General story graphs/transcripts, journal/reward/item semantics, complete source
diagnostics/build integration, and runtime qualification still prevent Phase 3
or Phase 4 from being complete.

### Phase 4: quest/dialog authoring and selection-effect research

Extend the landed bounded lifecycle table into the synchronized story outline,
transcript, general graph, state table, preview, condition simulator, reusable
condition/effect libraries, journal/reward/item semantics, and complete source
diagnostics/build workflow. Add reusable graph libraries before authors need to
duplicate large quest lines. New `UQuest` class discovery/instantiation is
already narrowly proven for the current game version. The V4 external-field,
predicate-hook, handler, `bSucceedParent`, typed-getter, and guarded-call shapes
are compiler/cache-pipeline-qualified on 1.0.3, but the exact full renderer
output still needs its own compiler gate; qualify actual availability/start/
success/failure behavior, dialog selection effects, rewards, knowledge changes,
and persistence separately on disposable saves. Production lowering remains
blocked for every behavior without a qualified mechanism.
Phase 6 upgrades only the Draft quest templates whose required mechanisms have
passed those gates.

### Phase 5: new NPC vertical slice

Build on the offline-proven class chain: generate a new `UniqueName` plus linked
AngelScript `CharacterDefinition`, `AIAgentConfig`, and
`SpawnAIAgentDefinition` classes/CDOs, leave visual/actor defaults inherited
from Asghan-derived parents, and
qualify one conservative spawn mechanism. The vertical slice is: Draft template
-> one candidate logical NPC/class chain -> one localized name -> one
existing-item inventory ref -> one dialog greeting with voice -> one qualified
spawn -> build -> disposable runtime proof of effective visuals, distinct
identity/dialog/quest state, and
persistence -> clean
undeploy. If the recovered chain requires an additional registry or cooked
asset, the slice blocks and reports that dependency rather than guessing it.
Cooked package/reference/collection creation remains a separate prerequisite
for genuinely new visual/content assets. Partial output must not be labeled a
new NPC.

### Phase 6: new quests, items, world/spawn breadth

Upgrade only qualified Draft quests to production-capable new quests, then
expand quest types/rewards, recover and qualify new-item identity paths,
routines, spawn groups, and safe world anchors. Add cooked
package/reference/collection generation for genuinely new visual/content assets
as its own proved capability rather than as an assumed prerequisite for every
logical entity. Arbitrary level or world-partition editing remains a distinct
later milestone with its own package and runtime proofs.

Reaching this research phase does not override the usability gate above. Broad
World or level authoring begins only after the existing managed workflows meet
their completion criteria. Meeting those criteria is not automatic authority
to continue: first stop at a clean, pushed core-Studio checkpoint, present the
usable workflows for user review, and wait for explicit approval before any
World or level implementation begins.

### Phase 7: production-scale authoring

Add project merge assistance, optional team assignments/work packages,
deterministic CI builds, an extension API, advanced 3D/world previews, and
performance tuning beyond the already-enforced reference budgets. Keep expert
source and CLI interoperability as supported escape hatches. Batch authoring,
coverage dashboards, reusable story structures, keyboard operation, and
accessibility must already be usable before this phase.

## 9. Precise offline acceptance criteria

These criteria run against fixtures and temporary directories. They must not
launch the real game, write under a configured real game root, touch a real
save, activate a real UE4SS mod, or require an existing deployment. Test-profile
lifecycle cases use only a fake launcher, fake game tree, and disposable fixture
saves. Tests install a filesystem-write sentinel around the fake normal game
tree and assert its complete before/after file inventory and hashes are
identical.

### 9.1 V2 project core

1. A canonical V2 fixture containing every entity/ref/origin variant serializes
   to a working-directory snapshot, reopens, validates, and serializes to
   byte-identical canonical manifest/entity shards and an identical AssetStore
   index. Deterministic archive export is byte-identical across directories.
2. Rename and reorder operations leave every `EntityId`, `ProjectTarget`, and
   `CatalogTarget` unchanged. A wrong project, unknown or stale catalog layer,
   seal mismatch, kind mismatch, missing target, duplicate project ID, or
   malformed ID yields the expected stable diagnostic or closed-schema load
   error; no reference searches another layer or resolves by name.
3. Two files with equal bytes import as one blob. Two different files cannot
   claim one digest. Missing, extra, truncated, oversized, hash-mismatched,
   traversal, absolute, link/reparse, case-alias, and duplicate archive entries
   fail before project publication.
4. A ten-operation cross-domain transaction fixture returns to the exact
   original canonical snapshot after ten undos and to the exact edited snapshot
   after ten redos. Injected failure at every delta boundary commits neither
   graph nor asset refs. Journal replay produces the same snapshot once and is
   idempotent.
5. A clone fixture proves `Override`, `Clone as new`, and `Reference` produce
   distinct origin/identity outcomes. Dependency policy choices are reflected
   exactly and no shared entity is accidentally owned or duplicated.
6. Save failure, reopen failure, existing-output refusal, and forced validation
   failure preserve the previous project byte-for-byte and leave no owned
   staging output after ordinary cleanup.
7. Changing one entity rewrites only its stable shard plus required manifest,
   journal, and derived-index facts; it does not rewrite unrelated authored
   entities or copy referenced base-game/dependency entities into the project.
   External shard modification produces a compare/reload conflict and never a
   silent overwrite.
8. Base-game and two dependency index fixtures deliberately reuse one
   catalog-local ID and canonical selector. Catalog-qualified refs and backlinks
   still resolve only through their pinned layer/seal. Updating one layer
   invalidates only its affected catalog targets, origins, and cache records; it
   neither mutates the authored graph nor falls through to an identically named
   entity in another layer.

### 9.2 Graph and diagnostics

1. Golden NPC/quest/dialog fixtures cover valid graphs plus every declared
   blocking diagnostic: missing refs, wrong ref kinds, unreachable quest state,
   missing initial/terminal state, invalid transition, dialog dead end, missing
   locale text, ambiguous voice target, unsupported DataAsset shape, stale
   origin, and unproven runtime mechanism.
2. Diagnostic ordering is deterministic by severity, entity ID, property path,
   and code. Incremental validation and a fresh full validation return identical
   diagnostic sets.
3. Every advertised quick fix produces a previewable transaction, clears only
   its targeted diagnostic on commit, and round-trips through undo/redo.
4. Deleting or cloning an entity produces an exact reference-impact report
   before mutation, including transitive owned children and non-owned users.
5. Draft-only NPC/quest fixtures save, clone, organize, and validate their known
   semantics, but production lowering blocks with stable responsible-entity
   diagnostics. Completing offline support upgrades them to Build-ready without
   inventing Runtime-qualified evidence.
6. Outline, transcript, graph, state-table, and preview adapters project one
   story model: an edit and selection made in each view appears identically in
   every other view, survives undo/reopen, and emits no view-owned duplicate.
   Focus/lazy graph loading returns the same full-validation result as loading
   the entire graph.

### 9.3 Deterministic lowering and bundle handoff

1. Two builds from the same canonical snapshot in different directories emit
   byte-identical generated sources, mini-cache inputs where compilation is not
   required, `BuildSpec` JSON, provenance map, and bundle payloads except for
   fields explicitly specified as nondeterministic (preferably none).
2. Every representable V2 entity lowers to the exact expected existing
   `BuildSpec` field; every emitted row maps back to one or more entity/property
   origins. No V2-only metadata leaks into runtime identity fields.
3. Any entity requiring unsupported cooked DataAsset/world output,
   unresolved refs, stale origins, or unproven production effects blocks before
   bundle staging and names the responsible entities. It is never silently
   omitted.
4. The produced bundle is reopened through gore-mod's parsers. Its manifest,
   component targets, payload hashes, and provenance agree with the build plan.
   Corrupt generated output or a failed reopen publishes no final bundle.
5. Expert-owned source has a golden success case and cases for undeclared
   output, stale input fingerprint, parse/compile failure, target drift, and
   collision; every failure is non-publishing.

### 9.4 Phase-1 voice-at-line slice

The fresh-project prerequisite has its own bounded acceptance gate. From the
guided V1 flow, a managed-R3 project can atomically create one new
`LocalizationEntry` plus `DialogLine` and optional empty locale `VoiceSlot`, or
reuse one exact existing unreferenced managed localization only when its ID,
revision, and localization identity still match. Exact reuse must first expose
only bounded, UTF-8-safe text previews from a read-only double-reopened current
checkpoint, hide technical identities, distinguish duplicate friendly labels,
and allow only locales with non-whitespace text. Publication repeats that exact
read. Tests must reject a referenced entry, stale revision/head/entity identity,
wrong-kind entity, duplicate identity, invalid or empty locale/text, stale
asynchronous preview, and every unsealed global-catalog or caller-supplied
vanilla-adoption shortcut. Candidate preparation, full reopen, guarded managed
publication, and full published reopen must preserve all unrelated entities.
The optional speaker input is metadata only and must never imply an NPC/topic
binding. The result remains explicitly build-blocked, runtime-unqualified,
topic-not-granted, and free of game/save access; this gate proves neither
AngelScript nor playable dialog.

The direct project-text editor has an additional bounded acceptance gate. It
must load complete texts rather than reuse previews, expose no technical
identity in normal UI, survive responsive and asynchronous selection changes,
and publish only after reopening the exact current catalog and seed. Native,
FFI, DTO, session, coordinator, domain, and widget tests must reject stale
head/project/target/entity/LocID state, base-origin or wrong-kind targets,
non-canonical/duplicate/over-budget locale maps, no-op and unrelated deltas,
VoiceSlot removal/blanking, candidate-backed transcript changes, failed CAS,
ambiguous publication, and malformed published reopen. A successful edit may
advance only the project and localization revisions, replace that text map, and
union newly introduced global authoring locales. It still grants no build,
deployment, topic, runtime, game, or save authority.

1. From one line selected by visible speaker/text, the UI/domain test uses the
   automatically derived German and English language slots and resolves them to
   two different exact existing `(archive, member)` targets, both with operation
   `replace`, without asking the author to type paths, select a slot, or choose
   an operation. Exact case-insensitive `${locId}.ogg` fixtures cover one match
   (one reversible proposal), zero matches (`Unresolved`; no invented path), and
   multiple matches (`Ambiguous`; explicit human-readable choice required). It
   rejects duplicate case-folded targets, substring/basename heuristics outside
   that exact rule, invalid locales, invalid source/derived audio, and missing
   `AssetRef`s.
2. Replace requires an exact existing member in a sealed fixture archive. A
   separate Experimental `add` fixture starts from an explicit qualified,
   versioned, sealed new-member namespace/rule that supplies the complete member
   path, then requires exact absence in the archive; a zero-match basename search
   can never create that target. A stale archive/member/rule origin produces a
   blocking diagnostic and no managed Voice build-plan edit. Offline `add`
   success is not part of the first valid build, does not mark runtime lookup
   qualified, and remains forbidden in a production profile without separate
   runtime proof.
3. Save/reopen retains line -> locale -> `VoiceSlot`, its exact
   `Unresolved`/`Ambiguous`/`Resolved` target state and evidence, candidate and
   selected `VoiceTake` refs, authored/derived audio bytes, logical filenames,
   production status, and duration/codec metadata without external or temp
   paths.
4. Preview reads the staged AssetStore Ogg through an exact-current
   line/localization/slot/take/asset binding. Native code fully verifies only the
   selected CAS object. On Windows, native registration pins the Store read-only
   and the temporary parent, atomically creates and retains one fresh
   non-overlapping preview root, rejects identity drift, and returns an opaque
   cleanup token before materializing the fixed `preview.ogg` leaf;
   unsupported desktop platforms fail closed. Studio rehashes the file, plays it
   in-app, unloads before native token-bound cleanup, and
   exposes no CAS or temporary path. A failed release retains the token for
   explicit retry; abrupt termination or an exceptional failed registration may
   leave an isolated temporary root behind until manual or operating-system
   policy cleanup, with no unsafe sweep. Replacing, unlinking,
   undoing, and redoing update the line inspector and diagnostics without
   mutating the source voice ZIP. Desktop playback grants no build, deployment,
   or audible-runtime qualification.
5. Lowering the first valid build consumes verified selected AssetStore bytes
   and emits exactly two ordered existing-member replacements sealed to the
   executable generation, archive, member, and payload. Bundle build produces a
   format-3 Voice manifest and embedded payloads whose byte lengths and hashes
   equal the Store assets; full reopening verifies both edits without a caller-
   controlled or materialized source path. The separate Experimental `add`
   fixture does not enter the managed revision-3 production builder.
6. Invalid second-language input proves all-or-nothing behavior: no partial
   project transaction, build plan, bundle, or archive output is published.
7. Widget tests cover the default author-facing flow: find a line by
   speaker/text, choose language, import/preview/select/unlink a take, see a
   human-readable ambiguity or blocking diagnostic, and navigate to the exact
   problem without opening Expert mode. Exact paths and `Add`/`Replace` remain
   inspectable only in the Advanced section of build details and are not user
   decisions.
8. The default Work list projects only absent authoring locales and intact
   existing Voice slots. Tests cover every next-step precedence branch,
   historical invalid selection, unreviewed alternatives after completion,
   absence of a slot, unavailable Voice evidence, bounded/partial retention,
   exact line/locale actions, same-revision head drift, root/project replacement,
   late callbacks, global single-flight, keyboard/semantics, German copy, 200%
   text scale, and compact/short layouts. “Production decisions complete” must
   never render or behave as Ready/build/runtime authority.
9. The sealed 35,000-entry/1-GiB voice fixture meets the indexing, lookup,
   preview-start, memory, progress, cancellation, and UI-thread budgets in §5.6.
   The test instruments bytes read and proves index/preview do not scan or copy
   every Ogg payload or import the source archive into the AssetStore.

### 9.5 NPC, quest, DataAsset, and spawn offline gates

These gates are necessary before runtime qualification, not a substitute for
it:

1. Each built-in NPC and quest template instantiates a closed graph with unique
   stable IDs, no dangling refs, exactly the documented owned dependencies, and
   deterministic technical identities under collision tests.
2. The Character-plus-first-greeting recipe must retain coordinator and widget
   journeys for successful N -> N+1 -> N+2 publication, cancellation/failure
   before the first mutation, retained NPC-only state after the second form
   stops, exact created-line selection, project/head drift, stale callbacks,
   uncertain publication, single-flight behavior, German copy, and compact
   200%-scale layout. Every result keeps build/runtime/spawn/game/save claims
   closed.
3. The first new-NPC fixture generates a new `UniqueName` and linked AngelScript
   `CharacterDefinition`, `AIAgentConfig`, and `SpawnAIAgentDefinition`
   classes/CDOs plus localization, inventory refs, dialog/voice, and spawn
   artifacts while reusing sealed vanilla visuals. Cache reopen/disassembly and
   independent semantic checks resolve every generated class/default/ref to the
   intended logical NPC and prove that no cooked package was silently assumed.
   A template that declares a genuinely new visual/content asset instead blocks
   until the separate DataAsset gate passes.
4. DataAsset create/edit fixtures round-trip names, imports, exports, object and
   package refs, unversioned headers, arrays, maps, and required collection-shape
   changes byte-semantically. Unsupported schema forms produce typed errors
   before output staging; no size or offset is guessed.
5. The landed bounded V4 fixture round-trips stable objective slots/order, all
   four fixed lifecycle edges, typed DNF conditions, typed cross-node lifecycle
   effects, and parent completion through canonical project JSON and
   deterministic source. Frozen-seed tests reproduce V2/V3 source bytes, and
   the sealed 1.0.3 compiler qualifier reopens/disassembles the exact listed
   external-field, hook, handler, getter, and guarded-call shapes without
   writing a game installation or save. One renderer-produced fixture spanning
   every state-test expression remains an offline gate. The larger
   acceptance fixture must still add localization, dialog links, journal,
   rewards/items, semantic reimport/simulation, and complete build diagnostics.
   Runtime transitions, selection, persistence, and other-version mechanisms
   remain blocking diagnostics for profiles that require them.
6. A spawn fixture resolves an NPC and a qualified world anchor, validates
   transform/policy/condition types, emits one deterministic deployment target,
   and rejects unknown anchors or unsupported persistence without output.
7. Clone-with-dependencies and CSV round trips over these fixtures preserve all
   typed refs and semantic values; collision, duplicate row, unknown column,
   stale base, and partial batch failures commit nothing.

Only after these offline gates pass should disposable runtime qualification be
attempted. New NPC completion additionally requires distinct logical discovery,
spawn, inherited-or-new visual resolution, AI, dialog/quest separation,
persistence, and clean undeploy proof. Quest completion may reuse the recorded
narrow discovery evidence only for its exact sealed game version and class
shape; it additionally requires correct generated transitions/effects, dialog
selection behavior, rewards, persistence, and semantic save comparison. Until
then the corresponding Studio features remain Draft-only, Experimental, or
Unavailable according to their exact operation, regardless of how plausible
their generated files look.

### 9.6 UX, scale, compatibility, test, and release gates

1. A widget/integration journey creates a project through installation/version,
   languages, author-facing project name, dependencies, isolated profile, and
   capability preview; then finds an existing line by visible speaker/text,
   imports and previews a take, validates, builds offline, and opens the exact
   diagnostic. The journey never exposes or requires a technical namespace,
   class, sentinel, archive path, `BuildSpec`, or UE4SS setting. Cancel/restart
   at every onboarding step is lossless.
2. Every primary surface is reachable by keyboard; focus order, shortcuts,
   command palette, screen-reader names/states, scaling, high contrast, and
   resizable-pane persistence have automated coverage. The same actions remain
   available without pointer-only drag gestures. Raw technical evidence is
   reachable but collapsed in the default mode.
3. Before Phase 1 is called usable, a moderated test with at least five people
   who have created a game mod but have not used the GORE CLI, Unreal object
   paths, or AngelScript must give only an author-facing task brief. At least
   four of five must find an existing line, replace its German and English takes,
   resolve one human-readable ambiguity, and produce the first valid offline
   build within 10 minutes without facilitator instruction, Expert mode, or
   needing a backend term. The run records completion, time, wrong turns,
   diagnostics used,
   recovery, and unexpected technical vocabulary; failure causes a UX revision,
   not a relaxed task definition. Repeat the study after material workflow
   changes, and include a keyboard-only/assistive-technology pass before the
   affected milestone release.
4. The scheduled performance suite runs both the
   100,000-catalog/10,000-entity/50,000-edge/5-GB fixture and the
   35,000-entry/1-GiB voice fixture under the exact hardware, cache, warm-up,
   sample-count, responsiveness, memory, progress, and cancellation conditions
   in §5.6. It asserts virtualized lists and focused graphs do not construct every
   row/node, archive work does not block the UI or import the source ZIP, and full
   validation still visits every required entity and edge. The smaller
   structurally equivalent fixtures run on every change; the full suite runs on
   schedule and before each affected milestone release.
5. Multi-select, fill/transform, spreadsheet paste, mapped import, and CSV
   fixtures show a semantic dry-run diff and commit as one transaction. An
   invalid row, stale base revision, reference mismatch, cancellation, or
   injected failure publishes none of the batch and undo restores the exact
   prior snapshot.
6. A fake launcher/profile harness exercises preflight -> isolated profile ->
   optional explicit save copy -> build/deploy -> launch/manual-entry record ->
   bounded log/result capture -> semantic disposable-save comparison ->
   undeploy/cleanup. Success, cancellation, game crash, Studio crash, and
   recovery replay all leave the configured normal game/save/loadout untouched
   and the fake installation inventory/hash-identical after cleanup.
7. Dependency fixtures cover missing/optional dependencies, version ranges,
   layer precedence, exact target collisions, identical catalog-local IDs and
   selectors in different layers, and a supported semantic merge. Unsupported
   collisions remain blocking, and no dependency reference searches another
   layer or falls back to display-name matching.
8. A three-way hotfix fixture compares old base, new base, and authored delta.
   Keep, qualified reapply, accept-new-base, and leave-blocked decisions are
   deterministic, undoable, and scoped to the selected properties. Failure at
   any publication/reopen step preserves the original project and last good
   build byte-for-byte.
9. Two releases of the same immutable revision/profile are byte-identical and
   carry exact mod version, compatibility, dependency/load-order metadata,
   description/icon/license, changelog, hashes, risk summary, and provenance.
   Draft-only content, unqualified production effects, stale origins,
   dependency conflicts, or failed clean install/undeploy checks prevent release
   publication without deleting a previous release.
