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
- [Cooked DataAsset fixed-leaf workflow](dataasset-authoring.md)
- [Offline AngelScript default patching](angelscript-default-patching.md)

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
  Settings / Expert mode

Persistent workspace
  Left: scope, outline, collections, and saved searches
  Center: table, form, graph, transcript, preview, map, or timeline
  Right: Properties | References | Problems
  Bottom drawer: Changes | Diagnostics | Build log | Test log
```

The Content Library has explicit `My Mod`, `Base Game`, and `Dependencies`
scopes. A global **Create** action opens concept templates. DataAssets, archive
members, scripts, textures, and generated artifacts may be searchable content
types or Advanced details, but they do not become an ever-growing row of
top-level format tabs. Editors use resizable panes, document history,
breadcrumbs, pinned entities, recent entities, and a command palette so a large
project does not depend on repeatedly navigating one tree.

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
| Project save/load | **Partial** | `.goremod` format 1 stores overrides, localization, FMOD replacements, textures, scripts, and runtime dialog-topic registrations. It embeds selected source files, but it has no typed cross-domain graph, stable entity IDs, content-addressed store, migration layer, or durable history. |
| Unified content browser | **Missing** | Items, dialog/localization, FMOD audio, textures, and scripts have separate tabs. There is no global entity search, reference graph, NPC/quest browser, or source-aware clone workflow. |
| Existing item scalar edits | **Proven subset** | The categorized item browser and typed scalar field editor stage CDO overrides. The fallback schema is limited and does not imply arbitrary property or item creation support. |
| Existing NPC edits | **Partial backend, missing authoring UI** | Catalog/model generation and generic CDO overrides can describe some existing NPC-class fields, but Mod Studio has no NPC entity browser or semantic NPC editor and no end-to-end NPC authoring proof. |
| New NPC identity | **Missing; script-only hypothesis identified** | ObjectDump/cache evidence represents Asghan's `CharacterDefinition`, `AIAgentConfig`, and `SpawnAIAgentDefinition` as linked `/Script/Angelscript` classes/CDOs. The fastest hypothesis is therefore a new `CharacterDefinition` with a new `UniqueName` plus new linked config/spawn classes while reusing qualified vanilla visuals. Spawning another body for an existing identity is proven, but it shares that identity's dialog/quest state and is not a new NPC. The logical-clone hypothesis is not yet proven end to end: class residence alone does not establish discovery, distinct dialog/quest identity, spawning, or persistence. Cooked-asset creation is required for genuinely new visual/content assets and for any registry or collection change the recovered chain actually requires, but is not currently proven mandatory for a logical NPC identity. |
| Existing localized dialog lines | **Proven** | The Dialogs tab groups `info_`/`dia_`/`gvl_`/`svm_` IDs, edits languages, and can add an explicit missing localization ID. Localization alone does not create a selectable topic. |
| New dialog topic insertion/rendering | **Proven narrow runtime path** | A compiled `UChoice` class plus explicit participant/topic/sentinel registration reached the natural choice UI and was visually confirmed. The current Studio editor exposes those technical identities manually. Automatic discovery remains unproven. See [dialog authoring](dialog-authoring.md). |
| Dialog selection effects | **Unproven** | Topic selection, quest/knowledge changes, `ActedTopics`, and selection-side save effects are outside the render proof. The safe proof intentionally selected nothing. |
| Quest inspection/edit/create | **Missing Studio path; narrow CLI/discovery proofs** | Save-editor quest-marker support is save editing, not mod authoring. Mod Studio has no quest catalog, typed quest graph, quest generator, or validated lowering. The CLI can strictly edit ordinary methods in an exact existing quest module and patch a separately sealed primitive default site, but that is not semantic quest authoring. A retained native crash report for the current game version lists runtime instances of two added `UQuest` subclasses as `Available`, proving narrow automatic class discovery and instantiation on world/save load. It does not prove authored transitions, dialog selection, effects, rewards, or persistence. See [quest authoring](quest-authoring.md). |
| Voice archive editing | **Proven backend, missing Studio integration** | `gore voice` and `gore-mod::BuildSpec.voice` support bounded exact-path Ogg add/replace and transactional bundle deployment. Format-1 Studio projects and `toBuildSpec()` do not carry voice entries, and no voice is linked to a dialog line in the UI. Brand-new voice-path resolution remains runtime-dependent. |
| FMOD sound/music replacement | **Proven** | Studio browses samples, previews originals or staged WAVs, and stages replacements for the bundle engine. This is sound-bank replacement, not spoken-dialog voice authoring. |
| Texture replacement | **Proven subset** | Existing texture assets can be browsed and replaced with additive IoStore output. This is not general cooked-asset creation. |
| Existing cooked DataAsset fixed leaves | **Proven narrow offline path** | Extract, inspect, same-width compare-and-swap patch, re-inspect, and offline pack are receipt-bound. Only structurally proven fixed-width leaves are editable. There is no Studio surface or semantic gameplay validation. See [DataAsset authoring](dataasset-authoring.md). |
| DataAsset creation/reference/collection editing | **Missing** | New exports/packages, `FName`/object/package reference creation, map keys, variable-width values, unversioned-header growth, and array/map shape changes are unsupported. These are hard prerequisites for genuinely new visual/content assets and for any content path that is proven to require new cooked package/reference/collection shapes. Current evidence does not establish them as universal prerequisites for logical NPC, item, or quest identities. |
| AngelScript source authoring | **Experimental** | Studio can stage new/edit modules and use the game compiler, with guarded diagnostics and mini-cache lowering. Existing generated `__InitDefaults` methods are not generally source-editable; new modules are the supported path for authored defaults. |
| Existing native default edits | **Proven narrow CLI path** | Scalar direct assignments and already-present `GameplayTag -> float32` entries can be patched offline under sealed selectors. Keys, maps, code size, and general generated source cannot be added. |
| Items/world/spawns | **Partial/missing** | Existing item scalar overrides are present. New item identity, placed world actors, spawn points, routines, level edits, and world-partition integration have no semantic Studio workflow or production proof. |
| Localization | **Proven** | Multi-language edits and explicit new IDs lower to `BuildSpec.loc_edits`; deploy is backup/restore aware. Referential completeness across quests, dialog, and voices is not yet validated as one graph. |
| Build/deploy/undeploy | **Proven for represented domains** | Studio drives the same bundle engine as the CLI and can restore its deployment. Voice and cooked DataAsset authoring are not represented by the current Studio project. A build dialog is not yet a project-wide dependency/risk review. |
| Validation | **Partial** | Scalar field validation, script freshness gates, bounded codecs, selector proofs, and backend build checks exist. There is no incremental graph validator, reachability analysis, semantic quest validation, or one-click diagnostic system. |
| Undo/redo/history | **Missing as a system** | Individual edits can often be cleared or removed, but there is no shared command log, multi-domain atomic undo/redo, crash journal, or named checkpoints. |
| Templates/clone/batch/CSV | **Missing** | No dependency-aware templates, clone modes, transactional bulk editor, or CSV round trip exists. |
| Expert escape hatch | **Partial** | The CLI and script-source editor expose powerful low-level paths. Studio lacks a unified generated-source/raw-property/BuildSpec inspector and source override contract. |

## 4. Complete authoring surfaces

### 4.1 Project dashboard

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

Each result has a kind, display name, source badge (`Base Game`, `My Mod`, or a
named dependency), readiness/diagnostic badge, and change state. The technical
identity and exact origin are shown in Advanced details. Saved filters,
collections, tags, and modules let authors organize a large mod by chapter,
location, quest line, owner, or production status. Search is global and
accent/case tolerant, and large result sets are virtualized and indexed.
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
build remains blocked until the exact definition/config/spawn links and runtime
discovery, distinct identity, dialog/quest separation, and persistence chain are
proven. The first implementation should test the bounded script-only logical
clone that reuses qualified vanilla visuals. Cooked package/reference/collection
support becomes an additional requirement only when the chosen template needs
new visual/content assets or the recovered identity chain proves it necessary.
The UI must not claim that an unreferenced AngelScript class is a working new
NPC or include it silently in a bundle.

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

### 4.6 Voice at the line

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

### 4.7 DataAsset inspector: semantic and expert layers

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

### 4.9 Localization workspace

The workspace combines source text, every target language, fallback behavior,
speaker/context, references, character limits, voice duration, take status, and
missing coverage. Its virtualized table supports multi-select, fill/transform,
spreadsheet paste, mapped import, and transactional undo. Bulk CSV uses stable
line/entity IDs and locale columns, exports a schema/version header, and rejects
unknown or duplicate IDs on import. New IDs are explicitly project-owned and
collision-checked against the selected game version and other project entities.

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

- project format version, migrated by the Studio;
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
not implicitly owned. The first bounded new-NPC target is a script-only logical
clone with a new `UniqueName`, new linked definition/config/spawn classes, and
qualified inherited vanilla visuals. It is buildable only when those links,
distinct identity, spawn, dialog/quest separation, and persistence are supported
for the chosen archetype. Cooked package, reference-table, or
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
`Available` runtime instances after world/save load. The native scan's internal
algorithm, other versions, vanilla semantic representation, authored
transitions, selection-driven effects, rewards, and persistence remain
unqualified. This evidence is sufficient to move a strict Draft schema and
offline source generator earlier; it is not sufficient to mark generated quest
behavior Supported. Before a quest lowerer or effect is marked Supported, its
generated source and exact target must validate offline and every claimed
transition/effect/persistence mechanism must pass bounded disposable-save proof.
Successful AngelScript compilation or class discovery alone is not sufficient.

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

## 8. Migration from format 1

Migration is an import transaction, never an in-place reinterpretation:

1. Bounded-read and fully validate the format-1 `.goremod` ZIP and
   `project.json` using the existing safe embedded-path rules.
2. Compute a source archive seal and derive a deterministic migration namespace
   and `ProjectId`. Derive each initial `EntityId` from that namespace plus the
   legacy domain, canonical legacy key, and occurrence. These IDs are stable for
   repeated imports of the same file but are thereafter immutable and no longer
   name-derived.
3. Convert metadata, overrides, localization, FMOD audio, textures, scripts,
   and dialog-topic registrations into typed V2 entities. Preserve legacy list
   order only where it has deployment meaning.
   Localization IDs become `LocalizationEntry` entities. Create a semantic
   `DialogLine`/speaker/`VoiceSlot` relationship only when the sealed catalogs
   provide a qualified mapping; otherwise preserve the text and emit an
   actionable linking diagnostic rather than infer identity from an ID prefix.
4. Stream every embedded authored payload into the AssetStore and replace
   temporary/external paths with `AssetRef`s. A missing or unsafe required
   payload becomes a blocking diagnostic; it is not silently discarded.
5. Create `OriginRef::Catalog` only when format-1 data contains a sufficient
   canonical target and the migration can pin its exact base-game or dependency
   layer/seal. Otherwise use `Imported` and emit an origin-resolution diagnostic.
6. Preserve the original `project.json` and archive SHA-256 as a read-only
   migration provenance record.
7. Validate, save, reopen, and byte/hash-check a new format-2 project beside the
   original. Do not overwrite the format-1 project on first migration.

Format 1 has no Studio voice domain, typed relationships, content hashes,
history, game-generation anchor, or distinction between semantic ownership and
deployment rows. Migration must not invent these facts. It should create
actionable diagnostics and a migration report instead.

## 9. Phased roadmap

Each phase requires its offline acceptance gates below. Runtime claims require
additional targeted runtime proof; offline green tests alone never certify
discovery, rendering, selection, persistence, or spawning.

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

Deliver the first usable subset of the Phase-0 contract: stable catalog-qualified
refs, AssetStore, transactions, and format-1 import for the affected
localization/dialog domains, then add a line-centric voice editor. It must let
the author find a line by visible speaker/text, use its automatically derived
language slot without seeing archive paths, import and validate a take, preview
it, survive save/reopen without external paths, lower the internally resolved
exact target to `BuildSpec.voice`, and build a verified bundle. The first valid
build uses two exact existing-member replacements in two languages. New-member
`add` has a separate offline Experimental acceptance case and is not required
for the first valid build or permitted in a production profile until its exact
target rule and runtime lookup are qualified.
This is first because the backend and deployment contract already exist and it
immediately turns three disconnected capabilities—dialog text, audio files, and
bundles—into one non-technical workflow.

### Phase 2: unified browser, references, templates, and history

Move the existing item/localization/dialog/FMOD/texture/script domains behind
the V2 graph. Add global search, source/readiness badges, dependency views,
named transactions, undo/redo, checkpoints, clone preview, a multi-select table
editor, spreadsheet paste/import mapping, and transactional CSV. Complete the
command palette, shortcuts, focus/accessibility audits, saved collections, and
translation/voice coverage dashboard at this stage. Add isolated test profiles,
the receipt-driven test lifecycle, dependency/loadout conflict validation, and
the first deterministic release package. Keep the old domain editors as
adapters during migration; do not maintain two independent project states.

### Phase 3: semantic existing content and early Draft authoring

Add reviewed schemas for existing NPCs, items, known DataAssets, and dialog
relationships. Integrate the fixed-leaf DataAsset workflow through semantic
selectors and receipts. Existing-class and existing-asset overrides come before
production new-identity creation. Recover the vanilla quest catalog and enough
of its exact representation for a strict typed model. Add a **Draft-only NPC**
skeleton/editor around the linked script-class hypothesis and **Draft-only
Quest** templates/schema/offline source generator. The narrow new-`UQuest`
discovery proof is recorded in the capability registry, but unqualified behavior
keeps generated quests out of production builds. Add project-wide
generation/rebase diagnostics, the full three-way rebase workflow, offline
semantic diff/build-plan inspection, and batch edits for compatible semantic
fields.

### Phase 4: quest/dialog authoring and selection-effect research

Build the synchronized story outline, transcript, graph, state table, preview,
condition simulator, and reviewed source generators on the Phase-3 Draft model.
Add reusable graph libraries before authors need to duplicate large quest lines.
New `UQuest` class discovery/instantiation is already narrowly proven for the
current game version; qualify generated availability/start/success/failure
transitions, dialog selection effects, rewards, knowledge changes, and
persistence separately on disposable saves. Production lowering remains blocked
for every behavior without a qualified mechanism. Phase 6 upgrades only the
Draft quest templates whose required mechanisms have passed those gates.

### Phase 5: new NPC vertical slice

Test the fastest evidence-backed hypothesis first: generate a new `UniqueName`
plus linked AngelScript `CharacterDefinition`, `AIAgentConfig`, and
`SpawnAIAgentDefinition` classes/CDOs, reuse qualified vanilla visuals, and add
one conservative spawn mechanism. The vertical slice is: Draft template -> one
logically distinct NPC -> one localized name -> one existing-item inventory ref
-> one dialog greeting with voice -> one qualified spawn -> build -> disposable
runtime proof of distinct identity/dialog/quest state and persistence -> clean
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

### Phase 7: production-scale authoring

Add project merge assistance, optional team assignments/work packages,
deterministic CI builds, an extension API, advanced 3D/world previews, and
performance tuning beyond the already-enforced reference budgets. Keep expert
source and CLI interoperability as supported escape hatches. Batch authoring,
coverage dashboards, reusable story structures, keyboard operation, and
accessibility must already be usable before this phase.

## 10. Precise offline acceptance criteria

These criteria run against fixtures and temporary directories. They must not
launch the real game, write under a configured real game root, touch a real
save, activate a real UE4SS mod, or require an existing deployment. Test-profile
lifecycle cases use only a fake launcher, fake game tree, and disposable fixture
saves. Tests install a filesystem-write sentinel around the fake normal game
tree and assert its complete before/after file inventory and hashes are
identical.

### 10.1 V2 project core

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

### 10.2 Format-1 migration

1. A fixture containing every format-1 domain migrates twice to the same IDs,
   graph, AssetStore hashes, and report. Every legacy staged row appears exactly
   once in V2 or has a blocking diagnostic naming why it cannot migrate.
2. Embedded audio, texture, script source, and mini-cache bytes match their V2
   blobs exactly. No V2 entity references the extraction temp directory or an
   original external source path.
3. Unsafe/missing payloads, duplicate legacy keys, malformed dialog identities,
   and unknown required format versions fail or diagnose according to an
   explicit golden report; none are silently dropped.
4. Migration writes only a new V2 output. The input archive remains byte- and
   timestamp-identical, and an existing output is never overwritten.

### 10.3 Graph and diagnostics

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

### 10.4 Deterministic lowering and bundle handoff

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

### 10.5 Phase-1 voice-at-line slice

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
   blocking diagnostic and no `BuildSpec.voice` row. Offline `add` success is not
   part of the first valid build, does not mark runtime lookup qualified, and
   remains forbidden in a production profile without separate runtime proof.
3. Save/reopen retains line -> locale -> `VoiceSlot`, its exact
   `Unresolved`/`Ambiguous`/`Resolved` target state and evidence, candidate and
   selected `VoiceTake` refs, authored/derived audio bytes, logical filenames,
   production status, and duration/codec metadata without external or temp
   paths.
4. Preview reads the staged AssetStore Ogg. Replacing, unlinking, undoing, and
   redoing update the line inspector and diagnostics without mutating the source
   voice ZIP.
5. Lowering the first valid build emits exactly two ordered `VoiceArchiveEdit`
   rows with the expected archive, `replace` operation, member, and materialized
   Ogg path. Bundle build produces a version-1 voice manifest and payloads whose
   hashes equal the AssetStore blobs; reopening verifies both edits. The
   Experimental `add` fixture lowers and verifies separately.
6. Invalid second-language input proves all-or-nothing behavior: no partial
   project transaction, BuildSpec, bundle, or archive output is published.
7. Widget tests cover the default author-facing flow: find a line by
   speaker/text, choose language, import/preview/select/unlink a take, see a
   human-readable ambiguity or blocking diagnostic, and navigate to the exact
   problem without opening Expert mode. Exact paths and `Add`/`Replace` remain
   inspectable only in the Advanced section of build details and are not user
   decisions.
8. The sealed 35,000-entry/1-GiB voice fixture meets the indexing, lookup,
   preview-start, memory, progress, cancellation, and UI-thread budgets in §5.6.
   The test instruments bytes read and proves index/preview do not scan or copy
   every Ogg payload or import the source archive into the AssetStore.

### 10.6 NPC, quest, DataAsset, and spawn offline gates

These gates are necessary before runtime qualification, not a substitute for
it:

1. Each built-in NPC and quest template instantiates a closed graph with unique
   stable IDs, no dangling refs, exactly the documented owned dependencies, and
   deterministic technical identities under collision tests.
2. The first new-NPC fixture generates a new `UniqueName` and linked AngelScript
   `CharacterDefinition`, `AIAgentConfig`, and `SpawnAIAgentDefinition`
   classes/CDOs plus localization, inventory refs, dialog/voice, and spawn
   artifacts while reusing sealed vanilla visuals. Cache reopen/disassembly and
   independent semantic checks resolve every generated class/default/ref to the
   intended logical NPC and prove that no cooked package was silently assumed.
   A template that declares a genuinely new visual/content asset instead blocks
   until the separate DataAsset gate passes.
3. DataAsset create/edit fixtures round-trip names, imports, exports, object and
   package refs, unversioned headers, arrays, maps, and required collection-shape
   changes byte-semantically. Unsupported schema forms produce typed errors
   before output staging; no size or offset is guessed.
4. A quest fixture compiles every state, transition, condition, effect,
   localization, dialog link, and reward exactly once and reimports generated
   semantic metadata to the same graph. For the sealed current-version fixture,
   the capability registry recognizes the retained narrow class-discovery proof
   instead of reporting discovery as unknown. Unqualified transition, selection,
   reward, persistence, or other-version discovery mechanisms remain blocking
   diagnostics for the profiles that require them.
5. A spawn fixture resolves an NPC and a qualified world anchor, validates
   transform/policy/condition types, emits one deterministic deployment target,
   and rejects unknown anchors or unsupported persistence without output.
6. Clone-with-dependencies and CSV round trips over these fixtures preserve all
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

### 10.7 UX, scale, compatibility, test, and release gates

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
