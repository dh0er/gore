# GORE Mod Studio product blueprint

Status: north-star product and UX blueprint grounded in the repository as of
July 2026. This document describes what a complete Studio should feel like and
how authors reach useful results quickly. It does not widen any implementation
or runtime claim.

Document ownership is deliberate. The normative project model, canonical
navigation shell, proof vocabulary, exact capability matrix, lowering contract,
roadmap, and acceptance gates live in
[Mod Studio authoring architecture](mod-studio-authoring-vision.md). This
blueprint owns product priorities, author journeys, novice-facing behavior, and
success metrics; it does not define a second project state or navigation model.
If the documents differ on a technical contract, the architecture specification
wins. The runtime boundaries for the most important new-content paths are
recorded in [NPC authoring](npc-authoring.md) and
[quest authoring](quest-authoring.md).

## 1. Product promise

The Studio should let an author think in Gothic concepts rather than storage
formats:

> Create a character, give them a daily life, write a conversation and quest,
> add localized voices and rewards, place the playable result in an isolated
> test profile, and publish a reproducible mod without learning archive paths,
> Unreal reflection identities, AngelScript cache internals, or deployment
> layout.

That is the end-state promise, not the current capability claim. Evidence has
three separate layers: a backend can work offline without being integrated into
Studio, Studio can persist an authoring concept without proving it in the game,
and a runtime proof applies only to its exact operation and game generation.
Today Studio integrates useful subsets for existing content, localization,
replacement media, scripts, topic registration, and bundle deployment. The
retained dialog proof is **render-only**: a naturally registered topic became
visible, nothing was selected, and no selection effect or persistence behavior
was proved. NPC and quest backends have useful but different offline/runtime
evidence; neither is a production-capable Studio workflow.

Three outcomes define the product:

1. A first-time author can make one **Ready to build** visible change and
   produce an offline build in under ten minutes, without entering Expert mode.
2. A story author can scaffold a large mod quickly as a coherent Draft, then
   see exactly which parts can build, which can only be drafted, and what still
   needs toolkit or in-game proof work.
3. A team can repeatedly edit, validate, test, rebase, and release thousands of
   related entities without corrupting a project, normal save, or game install.

The managed working project is always the source of truth. UI tabs are views of
that one state, not independent staging stores. Autosave journals, portable
exports, generated source, mini-caches, packages, bundles, releases,
deployments, and test saves each have different purposes and never become a
second editable project by accident.

## 2. Who the Studio serves

The default experience serves people who understand stories and games but do
not necessarily understand programming:

- **First-time tinkerer:** wants to rename, rebalance, revoice, or replace one
  thing and see it in game with minimal setup.
- **Story author:** thinks in characters, motivations, scenes, choices,
  objectives, journal entries, and consequences.
- **Content designer:** creates many NPCs, items, routines, rewards, spawns, and
  balance values and needs fast table and template workflows.
- **Media contributor:** records voice, translates text, or supplies audio,
  textures, models, and animation without needing access to every game system.
- **Technical modder:** needs generated AngelScript, typed raw properties,
  provenance, CLI/CI, and a supported override path without forking the normal
  authoring model.
- **Team lead/release manager:** needs scope, review, compatibility, reproducible
  builds, clean test profiles, and release evidence.

One project model serves all of them. A contributor does not receive a separate
"simple" file format that later has to be migrated into the real project.

## 3. Capability charter for a complete Studio

The end-state scope is broader than the capabilities already recovered. Status
must be reported on three independent axes:

- **Studio integration:** can an author discover, edit, persist, reopen, undo,
  validate, and lower the concept through the app?
- **Offline/backend evidence:** can a bounded library or CLI path generate,
  inspect, reopen, and verify the required artifact without the game?
- **Runtime evidence:** has the exact player-visible behavior been demonstrated
  safely on the exact game generation, including persistence where claimed?

`Implemented backend` never means `available in Studio`, and neither means
`works in game`. **Planned** describes product/integration work.
**Research-gated** means a Draft may be representable, but production output
must wait for a recovered, tested, version-qualified runtime mechanism.

| Area | Studio integration now | Offline/backend evidence | Runtime boundary and complete experience |
|---|---|---|---|
| Project and content library | The Flutter app still has a legacy format-1 archive and separate provider/tab state. Its compatibility session now owns the current path and extracted workspace, serializes Open/Save/New, distinguishes Save from Save As, preserves edits made during I/O as dirty, and uses strict bounded ZIP validation, canonical embedded paths, and crash-recoverable byte swaps. A V2 working project, one transaction log, history, recovery, and unified library are **not integrated**. | `gore-authoring` implements a closed phase-one `ProjectV2` model for localization/dialog-line/voice concepts, strict IDs/refs, canonical JSON, validation, and an AssetStore **index**. It does not yet implement the complete working-directory store, blob lifecycle, transaction journal, migration, or Studio session. | Runtime is not applicable. End state is safe create/open/recover, global search, collections, backlinks, lineage, source layers, and deterministic export from one managed project. |
| Existing game content | Item scalar, localization/dialog text, FMOD, texture, script, change, and build surfaces are **integrated subsets**. They are separate views/providers rather than one semantic graph. | Bounded scalar/default, localization, archive, script, texture, bundle, and fixed-leaf DataAsset paths have separate offline evidence. Evidence for one field/format does not generalize to another. | Only operations named in the normative matrix may be presented as supported. End state adds visible-name search, semantic references, compare/revert, and reviewed schemas. |
| Dialog and narrative | Existing text editing and explicit technical topic registration are integrated; transcript/outline/graph/state views and semantic condition/effect editing are **not integrated**. | Compiler, localization, and guarded registration paths have bounded evidence. | The retained runtime proof is **render-only** on one generation: a naturally registered topic appeared in the choice UI, nothing was selected, and no condition, effect, quest state, save, or persistence behavior was proved. All selection behavior remains **Research-gated**. |
| Localization and spoken dialog | Localization is integrated. A bounded line-first editor can now replace an existing spoken line per locale directly beside its dialog text: it resolves installed language archives, blocks zero matches, requires an explicit choice for ambiguity, stages a sealed Ogg replacement, previews/removes it, and participates in strict portable save/reopen, dirty state, Build, and Deploy. Actor-qualified Asghan/Viper IDs are visible in the browser. The managed V2 line -> locale -> multi-take/status workflow is **not integrated**. | Localization lowering, exact-path bounded Ogg replacement, sealed target resolution, bundle construction, and archive verification have bounded backend evidence. Manifest v2 now enforces the observed pristine archive SHA/size and exact member size/CRC before mutation; legacy unsealed manifest v1 remains readable. A read-only real-data probe uniquely resolved one Asghan and one Viper line on the named German archive seal. The V2 core models slots/takes but its AssetStore is currently an index contract, not complete blob I/O. | Existing-member and new-member behavior retain separate operation/version gates; Studio keeps new members Draft-only and build-blocked, and a zero basename match never proves a new path. Offline rewrite/deploy safety does not by itself prove audible gameplay behavior. End state adds multiple takes, recording/import, approval, duration, coverage, and managed history at the line. |
| NPCs | No semantic NPC browser/wizard or managed-project NPC entity is integrated. | A deterministic backend generator can create the bounded three-class logical-clone Draft; the retained Asghan-derived chain compiles, composes, reopens, and resolves offline. That generator is not a Studio workflow and compile/compose success is not spawning. | New-class residence, effective visuals, distinct identity, conservative spawn, AI, dialog/quest separation, streaming, save/reload, persistence, and uninstall behavior are **separate NPC research gates**. |
| Quests | No semantic quest catalog/editor or managed-project quest entity is integrated. | A backend discovery-only Draft generator produces a fixed root plus one objective with no transitions/effects; it is not a Studio workflow. Separately, a retained probe showed two added quest subclasses instantiated as `Available` on one generation. That probe does not automatically qualify generator output. | Availability/start/success/failure transitions, dialog selection, journal, rewards, knowledge, effects, save/reload, persistence, and uninstall behavior are **separate quest research gates**. They do not become qualified when NPC spawning works. |
| Items and economy | Existing scalar edits are an integrated subset; semantic clone/new-item/economy workflows are not. | Bounded existing-value paths exist. A general new-item identity/package pipeline is not offline-proven. | New identity, construction, visuals, equip/use behavior, trade/loot integration, and persistence are **Research-gated** independently. |
| World, routines, and spawns | No semantic map/routine/spawn authoring surface is integrated. | A typed Draft model is planned; arbitrary level/world-partition output is not proven. | Qualified anchors, schedules, spawns, triggers, navigation, streaming, ownership, and persistence require operation-specific research. A map pin must not imply writable world content. |
| DataAssets and cooked content | No semantic Studio DataAsset surface is integrated. | Receipt-bound existing fixed-width leaf edits are a proven offline subset. General package/export/name/reference/array/map/collection writing is not. | Existing fixed-leaf runtime semantics are only as broad as their reviewed field proof. Structural creation remains **Research-gated** until complete round-trip and runtime qualification. |
| Visual media | Existing texture replacement is integrated. General visual content is not. | Texture bundle output has bounded evidence; general material, mesh, character visual, animation, and VFX package creation does not. | New cooked visual registration/resolution is **Research-gated**. End state includes import validation, thumbnails, lineage, and qualified previews. |
| Audio and music | FMOD sample browsing/preview/replacement is integrated. Semantic cue/event creation is not. | Existing-bank replacement has bounded backend evidence. | New event/cue integration is **Research-gated**. End state adds batch normalize/transcode, loudness/codec checks, ownership, and conflict handling. |
| Cinematics and presentation | No Studio integration. | No current authoring pipeline claim. | Scene timelines, cameras, staging, subtitle/audio sync, animation, and reusable sequences are **future Research-gated** capabilities. |
| Gameplay systems | Only bounded existing scalar/default and Expert script paths are available. | Evidence is selector-, field-, and generator-specific. | General factions, AI, combat, talents, spells, economy, rules, and reusable runtime effects are **Research-gated**; Expert source cannot bypass qualification. |
| UI and player-facing presentation | No semantic UI authoring integration. | Generic texture/script support is not evidence for mod-owned UI. | Journal/menu/icon behavior and new UI remain **future Research-gated** until their actual chains are recovered. |
| Test and debug | Build/deploy controls exist, but named isolated scenarios, managed test profiles, semantic observations/save diff, and recovery orchestration are not integrated. | Offline validators and receipt-driven deployment provide components, not the complete scenario lifecycle. | Each gameplay action needs its own risk profile and runtime evidence. End state includes offline simulation, isolated tests, logs, observations, save diff, and verified cleanup. |
| Build and release | Studio can build/deploy represented legacy domains. Project-wide semantic build roots, dependencies/rebase, immutable releases, and CI are not integrated. | The bundle engine can build/reopen/inspect represented components. | A bundle is not gameplay proof. End state adds deterministic profiles, semantic plans/conflicts, compatibility, rollback, provenance, and closed-world release validation. |
| Collaboration and extension | No semantic collaboration workflow is integrated. | Canonical V2 primitives are groundwork, not merge/sync implementation. | Planned after the single-author managed-project/transaction contract; core authoring must not require a cloud account. |

"Complete" does not mean exposing every Unreal file type. It means every
advertised operation is semantic, reversible, deterministic, inspectable, and
qualified for the selected game version. Unknown game-source/property
structures remain visible and preserved but read-only; unknown required project
schema content follows the strict block-or-migrate contract in Section 5.

### 3.1 Remaining legacy-session limits

The current format-1 Flutter session must not be treated as the durable
foundation for new authoring domains. Strict archive validation, atomic
publication, current-path and workspace ownership, a serialized Open/Save/New
lane, exact saved snapshots, metadata-aware dirty state, true **Save** versus
**Save As**, and `Ctrl+S` make it a substantially safer compatibility/import
bridge. Its remaining managed-authoring limits are:

- the bounded format-1 encoder/reader is intentionally in-memory and capped; it
  is not the streaming, sharded, content-addressed storage needed by large mods;
- provider replacement is still not one rollback-capable all-domain
  transaction, even though all represented keyed deployment targets now receive
  one duplicate-validation pass before mutation or publication;
- format-1 embedded source paths are extraction derivatives rather than
  immutable AssetStore refs, so they remain unsuitable as durable entity state;
- autosave recovery, named checkpoints, crash-safe transaction replay, V2
  migration, and managed AssetStore blob ownership are not implemented.

These are release blockers for the managed authoring substrate, not reasons to
hide user data or add another domain-specific save mechanism. New Voice, NPC,
or Quest UX must not deepen this legacy state model.

## 4. The ideal information architecture

The architecture specification owns the canonical information architecture.
This blueprint repeats only its stable primary destinations so product work
does not invent a competing hierarchy:

```text
Home
Content Library
Story
World
Localization & Voice
Validate & Test
Build & Release
Settings / Expert mode
```

These primary destinations are stable and discoverable; they do not appear and
disappear based on project contents or support level. A section with no authored
content shows what belongs there, a small example, the safest available next
action, and links to relevant content elsewhere. A not-yet-supported creation
path remains discoverable with plain-language explanation and a Draft option
when safe; it is not hidden behind an empty tab or Expert mode. Contextual
subsections may adapt, but breadcrumbs, global search, the command palette, and
the global **Create** action always provide a predictable route back.

Media such as sound, textures, and later visual/cinematic assets are content
types in the Library or context views in Localization & Voice, Story, and World;
they do not require an additional permanent backend-format destination. Items,
NPCs, quests, and other concepts likewise appear in their task workspace and
remain globally searchable in the Library.

Each workspace uses the same shell:

- left: scope, outline, collections, and saved views;
- center: the best view for the task (form, table, transcript, graph, map, or
  timeline);
- right: Properties, References, and Problems for the current selection;
- bottom drawer: Changes, Diagnostics, Build log, and Test log;
- global bar: back/forward, search/commands, undo/redo, save/recovery state,
  game compatibility, Validate, Test, and Build.

Selection, navigation history, pinned entities, recent entities, and filters
survive workspace changes and project reopen. A reference link always opens the
semantic owner, not an opaque generated file. Every editor is a projection of
the one managed project graph; no tab owns a private authoritative copy.

## 5. Managed project, autosave, export, build, and release

These nouns describe different artifacts and must remain visibly distinct:

| Artifact/action | Purpose | Contract |
|---|---|---|
| **Managed working project** | The live editable source of truth | A Studio-owned directory with canonical V2 shards, immutable AssetStore blobs, session/current-path ownership, a serialized operation lane, and one transaction history |
| **Autosave/recovery** | Recover unsaved work after a crash | A bounded journal/recovery snapshot tied to the exact base revision; it is automatic and is not a portable project export or release |
| **Save / checkpoint** | Durably acknowledge the current revision | `Ctrl+S` flushes the current transaction state and creates/advances a recoverable checkpoint at the owned path; **Save As** creates a separately validated managed project identity/path |
| **Export** | Portable backup, interchange, or review copy | A deterministic `.goremod` archive produced from an immutable snapshot; exporting does not silently change the current working path, and opening it imports to a managed working directory |
| **Build** | Produce an inspectable mod artifact | Derived from an immutable project revision and named build root/profile; it does not deploy and cannot become editable source state |
| **Test deployment** | Install one build into an isolated test profile | Receipt-owned, game-closed preflight, explicit disposable save choice, bounded logs/observations, and verified cleanup |
| **Release** | Publish a reproducible user-facing package | References an immutable closed-world validated revision/build plus compatibility, dependency, license, changelog, hashes, and provenance |

The first line-centric Voice slice must implement the minimum real form of this
substrate rather than adding another format-1 list. It needs a managed V2
working project, current-path/session ownership, strict bounded open/import,
AssetStore **blob I/O** (not only the implemented index type), one transaction
boundary across line/text/slot/take, exact saved-revision capture, serialized
save/open, crash recovery, and deterministic export/build materialization. That
foundation is part of the slice, not a large prerequisite postponed indefinitely.

V2 is a closed, versioned contract. Unknown required project formats, schema
revisions, entity kinds, payload variants, or fields are never ignored or
silently round-tripped through a partial model. If a reviewed migration exists,
Studio imports into a new managed project, reports every transformation, saves,
reopens, and leaves the source untouched. Otherwise it blocks editing and build
with a version explanation and offers only operations that preserve the bytes.
Optional forward data is allowed only inside an explicitly versioned extension
envelope with declared preservation semantics; there is no generic catch-all
map. The implemented phase-one V2 parser already rejects unknown fields, but
complete working-project persistence and migrations are still integration work.

## 6. Progressive disclosure without a toy mode

There are three presentation levels over one model:

### Guided (default)

The author chooses an intent such as **Make a character**, **Write a quest**,
**Replace a voice line**, or **Change an item**. Wizards ask only decisions that
affect authored meaning. The Studio creates and collision-checks internal IDs
and names without showing them, selects safe defaults, and explains unavailable
choices before content is created.

### Author

Forms, tables, transcript, graph, dependency views, batch tools, build profiles,
and diagnostics expose the full semantic model. This is the normal mode for a
large project and must still avoid backend paths and raw code.

### Expert

The same entity reveals origin seals, reflected identities, generated source,
typed raw properties, archive/member targets, lowering output, and provenance.
An expert-owned source override is explicit and cannot be overwritten by a
semantic editor. Re-entering semantic ownership requires a checked import.

These are disclosure levels, not global capability switches. A technical panel
can be opened for one entity while the rest of the project remains in its
normal authoring view. Unsupported controls are not merely disabled: they state
whether the reason is missing author input, incompatible game version, missing
toolkit support, or behavior that still needs an in-game proof.

The engineering terms in this document are not novice UI labels. Guided copy
uses outcomes such as **Ready to build**, **Can be drafted, but not installed
yet**, **Needs a different game version**, and **This action is not supported
yet**. A concrete message says, for example, "You can write this quest now, but
the game action that completes it is not supported for your version yet."
`Runtime-qualified`, `source seal`, `capability registry`, `technical
namespace`, and similar evidence terms appear only in explanatory or Expert
details.

Readiness copy still keeps the evidence levels distinct:

- **Ready to build** means Studio can create and verify the offline artifact; it
  does not promise the behavior has run in game.
- **Ready for isolated test** means the named action has an approved test path
  for this game version and risk profile.
- **Ready for release** means the complete selected release profile passed its
  offline and required runtime/cleanup gates.
- **Can be drafted, but not installed yet** means intent is safely saved but an
  output or runtime mechanism is still missing.

## 7. The fastest useful workflows

### 7.1 First visible change

The default onboarding should lead to a result, not a blank project:

1. Detect the game and identify its exact generation.
   Show the author its familiar edition/version; retain the exact executable
   anchor only as compatibility evidence.
2. Offer recipes that clearly say whether the result can be built for that
   version or only drafted.
3. Let the author search by visible in-game name or text.
4. Make one semantic edit or voice/media replacement.
5. Validate continuously; show one plain-language blocker with a direct fix.
6. Build offline, reopen the artifact, and show the semantic change plan.
7. Offer an isolated test only when Studio has a safe test plan for that exact
   action and game version.

The Studio creates the managed project and recovery journal before the first
edit, remembers setup, and never requires an internal namespace, output path,
or manual deployment copy.

### 7.2 One playable story slice

The north-star flow for substantial mods is **Create playable slice**:

1. Choose a **Ready to build** or **Draft now** recipe: for example "new
   archetype-based NPC with greeting and talk quest."
2. Pick an archetype, location/anchor, languages, and high-level quest pattern.
3. Preview the owned/ref dependency plan: NPC identity chain, spawn, quest,
   dialog, text, voice slots, journal, item/reward refs, and test scenario.
4. Create the complete typed graph in one undoable transaction.
5. Open a focused Story workspace with placeholders and a checklist in logical
   production order: identity -> placement -> greeting -> objective -> result ->
   journal/reward -> localization -> voice -> test.
6. Name the slice, choose its authored roots and expected observation, and use
   **Test from here** only when the dependency closure has a qualified test
   path. The named slice remains useful for writing and review when it cannot
   yet run.

Today this combined new-NPC/new-quest slice is a **planned Draft workflow and
not a supported production promise**. The important product decision is that
authors can scaffold and organize the whole intent without hand-authoring
disconnected backend rows. NPC spawning/identity and quest
transitions/effects/persistence are different qualification tracks; success in
one never upgrades the other. A scoped iteration build names every excluded or
blocked mechanism rather than silently omitting it.

### 7.3 Build a large mod quickly

Large-mod authoring is a production pipeline, not repeated use of small forms:

1. Start from a project kit that defines languages, automatically managed
   internal naming rules, default archetypes, reusable conditions/effects,
   release profile, and content-status workflow.
2. Scaffold chapters, locations, quest lines, principal NPC roles, and critical
   path as Draft entities before filling details.
3. Instantiate parameterized templates in bulk. Shared dependencies are
   referenced; owned content is cloned with a preview.
4. Write dialog in transcript view, quest structure in outline/state table, and
   use a graph only for meaningful branching.
5. Edit inventories, stats, routines, localization, voice casting, and
   production status in virtualized tables with multi-select and paste/import.
6. Work from dashboards such as **My tasks**, **Blocking the next test**,
   **Missing translation/voice**, **Changed by hotfix**, and **Ready for review**.
7. Validate and build a named affected dependency closure during iteration;
   run a closed-world release validation before publishing.

The shortest path is therefore not "generate more code." It is: generate a
valid semantic skeleton, reuse reviewed building blocks, edit many compatible
values at once, keep references correct automatically, and run the smallest
relevant test without rebuilding or navigating unrelated content.

### 7.4 Named playable slices and build roots

A **playable slice** is an author-facing named goal such as "Old Mine Viper
intro." Internally it owns one or more semantic root refs, a build profile, an
expected test scenario, and an inclusion explanation. A **build root** is the
deterministic dependency closure used to produce one scoped iteration artifact.
The Studio derives the closure from typed references; an author never maintains
a file list.

Scoped validation checks the complete selected closure plus global invariants
that could make it unsafe, such as identity and deployment-target collisions.
The build plan lists included, blocked, and deliberately out-of-scope authored
content. A scoped artifact is visibly labeled **Test build: <slice>** and can
never be promoted to a release merely because it built successfully.

A release is closed-world for its declared production roots and profile. Every
authored runtime entity is either reachable and validated, or explicitly
excluded with a retained author-facing reason; unreachable or accidentally
excluded content is a diagnostic. Release revalidates all roots, cross-root
collisions, dependencies, compatibility, runtime proof requirements, and the
final bundle. Draft-only content never disappears through an implicit root.

## 8. Recipes, templates, and reusable content

A template is a versioned semantic graph fragment with parameters,
preconditions, owned/reference policy, diagnostics, and an expected test
scenario. It is never an opaque script macro.

Useful built-ins include:

- edit/clone an existing NPC; archetype-based Draft NPC; merchant; guard;
  trainer; ambient citizen; quest giver;
- one-shot conversation; greeting; branching conversation; ambient bark set;
- talk, fetch, delivery, kill, escort, investigation, multi-objective, choice,
  and timed quest patterns;
- item reward, trade inventory, equipment set, loot table, routine, spawn group,
  and location/chapter starter;
- bilingual or multilingual voice/localization production packs;
- **Ready to build** replacement-only first-mod recipes whose complete path is
  available for the selected version.

Before creation, the template shows **What works in your game version**:
buildable results, parts that can only be drafted, required add-ons, what still
needs an in-game proof, and whether a release build is possible. Engineering
evidence is an expandable detail. Changing a parameter recomputes the preview.
Creation is atomic and undoable.

Project-owned template packs capture house style. Updating a pack never mutates
instances silently; the Studio offers a semantic three-way update with preview.
Teams can publish signed template/dependency packs independently from executable
extensions.

## 9. Bulk authoring and automation

Every repetitive operation needs a safe batch form:

- multi-select common fields with **Set**, **Add**, **Scale**, **Clear**, and
  deterministic formula/transform operations;
- spreadsheet paste plus mapped CSV/TSV for tables, XLIFF/CSV for localization,
  and folder/manifest import for media;
- batch create from a reviewed template and input table;
- find/replace on semantic fields and references, never blind generated-source
  text replacement;
- clone a dialog branch, quest subtree, inventory, routine, or spawn group with
  explicit shared-versus-owned dependency choices;
- deterministic renaming and collision preview, with internal-name details only
  in Advanced view;
- "Fix all safe in scope" only for quick fixes whose semantics are lossless and
  independently previewable.

Every batch tool shows scope, row-level validation, reference impact, and a
semantic before/after diff. One invalid row prevents publication unless the
author explicitly creates a new transaction containing only the valid rows.
Cancel, failure, or stale project revision commits nothing. Undo reverts the
entire published batch.

Optional assistive generation may draft names, descriptions, dialog variants,
translations, tags, or test outlines, but it is never a correctness authority.
Generated suggestions carry provenance, require author acceptance, obey the
same project transactions, and cannot invent a runtime identity or bypass a
readiness gate. Core authoring, build, and release remain fully usable offline
without an AI service.

## 10. Validation and testing as author tools

Diagnostics should answer four questions in this order:

1. What authored thing is affected?
2. What player-visible behavior will not work or cannot yet be claimed?
3. Is this missing content, a conflict, a stale game/dependency origin, an
   unavailable output shape, or behavior that still needs an in-game proof?
4. What can the author do next?

The Problems view groups by task and production impact, not by compiler file.
Opening a problem selects the exact entity/property in its natural editor.
Compiler and lowering evidence is expandable. A quick fix is a previewed
transaction, never an eager mutation.

Testing has a confidence ladder:

```text
live semantic lint
  -> full closed-world validation
  -> generated artifact reopen/inspection
  -> offline state/dialog simulation
  -> isolated dialog render-only test (topic visible; do not select)
  -> disposable-save gameplay scenario
  -> save/reload and clean-undeploy verification
  -> release qualification for the exact profile
```

The Studio chooses the lowest-risk level that can answer the author's question.
**Test from here** creates or uses a named playable slice, derives its smallest
affected build-root closure and scenario, but does not fabricate game state
through an unqualified console/loader hook. In particular, the retained dialog
render-only path must never select the topic. Manual setup remains first-class
where automation is unproven. A test run records expected observations, actual
author observations, logs, hashes, project revision, game version, loadout,
disposable save diff, and cleanup.

Build, Test, Deploy, and Release are visibly different actions. A successful
build does not imply runtime success; a successful scoped test does not qualify
unrelated behavior or mutate the normal loadout; and release packages only an
immutable closed-world validated revision.

## 11. Collaboration for people who do not use Git

The single-author safety model comes first: autosave, crash recovery,
transactions, checkpoints, and deterministic export. Collaboration then builds
on semantic changes rather than shared mutable generated files.

The minimum useful team workflow is:

- assign an entity, collection, chapter, locale, or media-production task;
- create a named change set from a known project revision;
- add comments, status, and requested changes to semantic entities/properties;
- export a bounded review/work package with required catalog/dependency seals;
- preview and apply it as one transaction with reference/conflict validation;
- record author, reviewer, decision, and resulting revision.

The working-directory format and canonical text export should be Git-friendly
for technical teams, but Git is optional and generated indexes/artifacts remain
outside semantic history. A semantic diff says "Asghan greeting text changed"
or "quest reward now references item X," not merely that JSON lines moved. A
semantic three-way merge handles only reviewed domain operations; ambiguous
graph, asset, or deployment-target conflicts require a person.

Later collaboration may add local-network or hosted sync, presence, branch
views, and asset locks. It must not make an online account mandatory for
authoring or make the server the only copy of a project. Large binary assets use
content hashes and explicit transfer/availability status rather than being
silently omitted.

## 12. Accessibility, localization, and learning

The Studio itself and the mod's authored languages are independent. Changing
the UI language cannot change project locales or generated content. Search is
case/accent tolerant for display text while identity resolution remains exact.

Every primary action must support keyboard navigation and the command palette.
Tables, graphs, maps, timelines, waveform controls, and drag operations all
need a non-pointer alternative. Focus order, screen-reader labels and live
status, scalable text, high contrast, reduced motion, and non-color status cues
are release gates, not final polish. Layouts work at 125-200% Windows scaling
and persist per workspace without hiding the selected problem or primary action.

Contextual learning is task-based:

- starter projects contain a guided playable/replacement slice, not prose-only
  documentation;
- empty states offer the next useful action and a small example;
- unfamiliar terms have one-sentence inline explanations plus deeper help;
- blocked actions link to a plain-language explanation and safe available
  alternatives;
- tutorials can be reset and never write to the real install or normal saves.

## 13. Scale and responsiveness

Large projects use stable shards, content-addressed assets, lazy catalogs,
incremental indexes, focused graphs, virtualized tables, and cached dependency
closures. Opening the shell does not deserialize every entity or audio payload.
Search, backlinks, diagnostics, and coverage update incrementally; full release
validation still streams the complete canonical snapshot.

Long operations run off the UI thread, report phase, scope, progress, and
estimated remaining work when measurable, and acknowledge cancellation at a
safe boundary. A stopped index/build/import never publishes a partial result.
Background work is prioritized around the current task: visible entity,
blocking diagnostics, selected test closure, then the rest of the project.

The concrete 100,000-catalog-row, 10,000-authored-entity, 50,000-edge, 5-GB
project and 1-GiB voice-archive budgets in the architecture specification are
release gates. Subjective "works on my machine" testing is insufficient.

## 14. Delivery order: useful slices, not a backend big bang

This blueprint maps to the detailed Phase 0-7 roadmap in the architecture
specification:

1. **Managed phase-one substrate:** replace the legacy session blockers for the
   affected domains with an owned V2 working directory, current path and
   serialized session, strict bounded import/open, AssetStore blob I/O,
   cross-domain transaction, exact saved revision, autosave/recovery, and
   deterministic export. Wire the available crash-recoverable replacement
   helper into the publication path. This is the first part of the Voice
   vertical slice, not a detached infrastructure rewrite.
2. **Complete that first non-technical Voice slice:** on the substrate from
   step 1, find a dialog line by visible speaker/text, attach localized voice
   takes, save/reopen/undo/recover, validate, and build exact existing-member
   replacements without archive terminology. Backend voice operations or a V2
   model alone do not satisfy this milestone; the Studio journey and AssetStore
   persistence must pass together.
3. **Authoring shell:** move represented domains onto the same managed graph;
   add the stable unified Library, references, transactions, history,
   templates, bulk tables/import, coverage, isolated test profiles, and
   deterministic release for already represented domains.
4. **Semantic breadth and safe Drafts:** existing NPC/item/DataAsset views,
   recovered quest catalog, Studio-integrated Draft NPC/quest entities and
   wizards, deterministic source generation, source-origin drift, and three-way
   game-version rebase. Reuse the implemented backend generators; do not count
   them as complete until their inputs, outputs, diagnostics, undo, persistence,
   and build blocking are integrated.
5. **Story production:** synchronized transcript/outline/graph/state views,
   reusable conditions/effects, simulation, and separately qualified dialog
   selection and quest-transition mechanisms.
6. **New-NPC runtime slice:** qualify the archetype-based class chain,
   residence, distinct identity, inherited visuals, conservative spawn, AI,
   dialog/quest separation, streaming, persistence, save/reload, and clean
   undeploy on one exact game generation. Quest transition success is neither a
   prerequisite nor a result of this NPC qualification.
7. **New-Quest runtime slice:** independently qualify generated quest discovery,
   availability/start/success/failure transitions, natural dialog selection,
   journal, rewards/knowledge/effects, persistence, save/reload, uninstall, and
   clean undeploy. NPC spawn success does not qualify any of these behaviors.
8. **Content breadth:** qualify new items, routines, spawn groups, and wider
   quest patterns; implement cooked package/reference/collection creation; only
   then add advanced world/visual/cinematic paths whose actual runtime chains
   are known.
9. **Production ecosystem:** semantic collaboration, CI, extensions, advanced
   previews, dependency distribution, and release channels.

At every step, ship one complete author journey before adding another raw
backend surface. Draft representation may arrive before runtime qualification;
production labels and outputs may not.

## 15. Product metrics and definition of done

Each milestone has behavioral metrics in addition to unit tests:

- time to first **Ready to build** offline artifact and first isolated visible
  result;
- percentage completing the primary task without Expert mode or backend terms;
- number of wrong turns, dead ends, and diagnostics that fail to lead to a fix;
- save/recovery success after injected write failure, process kill, and reopen;
- median time to create and revise one complete template-based story slice;
- throughput for 100/1,000-row localization, inventory, routine, and balance
  edits, including review and undo;
- search, selection, edit, validation, build, cancellation, and memory budgets
  on the reference large project;
- translation and voice coverage, blocking-reference count, stale-origin count,
  and runtime-qualification coverage for a release profile;
- deterministic rebuild, clean isolated install, save/reload where required,
  clean undeploy, and byte-identical second release from one immutable revision;
- moderated non-technical usability tests plus keyboard-only and assistive-
  technology passes before the affected milestone ships.

A feature is not done because a form exists or a compiler accepts its output.
It is done when an author can discover it, create valid intent, recover from a
mistake, understand its readiness, build deterministically, run the appropriate
safe test, and return the machine to the recorded clean state.

## 16. Product traps to avoid

- Do not recreate Unreal Editor as a collection of raw object trees.
- Do not make every storage format a permanent top-level tab.
- Do not hide or reorder primary navigation because a project is empty or a
  capability is unavailable; use honest, useful empty states.
- Do not require a graph for linear writing or a form for bulk production.
- Do not call a backend generator, V2 type, compiler result, or offline artifact
  a Studio feature until the complete author journey is integrated.
- Do not equate compile success, class discovery, or rendering with complete
  gameplay behavior.
- Do not expose archive paths, class identities, sentinels, hashes, or loader
  settings as normal author decisions.
- Do not guess references, deployment targets, serializers, world anchors, or
  runtime registration from a similar display name.
- Do not let Draft-only content disappear silently from a build.
- Do not let a scoped build root masquerade as a closed-world release.
- Do not ignore unknown required V2 fields or variants; migrate explicitly or
  block editing/build while preserving the source.
- Do not conflate the managed project, recovery journal, portable export,
  build artifact, deployment, or release.
- Do not mistake a bounded format-1 compatibility bridge for the final
  authoring state. New domain work must have a deterministic migration into the
  managed substrate and must not invent another private save mechanism.
- Do not deploy as a side effect of Build or test against the normal save/loadout
  by default.
- Do not make Git, a terminal, a cloud account, or an AI service prerequisites
  for normal authoring.
- Do not postpone undo, recovery, accessibility, large-project indexing, or
  semantic diagnostics until after content editors are built; all later
  workflows depend on them.
