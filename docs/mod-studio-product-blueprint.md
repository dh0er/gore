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
[quest authoring](quest-authoring.md). The bounded managed-R3 Voice slice is
recorded separately in [Voice authoring](voice-authoring.md).

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
| Project and content library | The compatibility project remains a legacy format-1 bridge and the separate Story flow remains schema revision 2. The primary Project menu now creates or opens one authoritative managed-R3 project. New asks for friendly metadata and an empty folder, authenticates the configured game, then shows the same searchable content library used by existing R3 projects. Dirty Legacy transitions are confirmed; failed or mismatched candidates never displace the current lease; late published-create failures can recover the exact project; `requiresReopen` and cleanup diagnostics remain visible. R3 hides Legacy Build/Deploy because its NPC/Quest/DataAsset content is not yet generally buildable. | Creation is limited to the exact registered V1 and current Steam-build-`24169431` generation triples, uses a secure project ID and canonical empty revision-3 document, publishes with absent-head CAS, and fully reopens before adoption. Unknown/cross-paired seals, arbitrary nonempty roots, install/project overlap, and candidate-byte drift fail closed. The live current-install proof created, indexed, closed, reopened, and removed a temporary project while EXE/Shipping/Binds length and SHA-256 stayed unchanged. Existing Store safety, semantic indexing, backlinks, and bounded Quest/NPC/Voice/DataAsset transactions are unchanged. General editors, migration/import/clone/Save As, history/recovery UI, global/dependency search, collections, unified transactions, and blob-ownership tools remain. | Project creation itself has no game-runtime claim and writes neither the installation nor a save. End state adds recoverable templates, safe migration/clone/export, global organization, deterministic builds, and isolated test/deploy workflows. |
| Existing game content | Item scalar, localization/dialog text, FMOD, texture, script, change, and build surfaces are **integrated subsets**. They are separate views/providers rather than one semantic graph. | Bounded scalar/default, localization, archive, script, texture, bundle, and fixed-leaf DataAsset paths have separate offline evidence. Evidence for one field/format does not generalize to another. | Only operations named in the normative matrix may be presented as supported. End state adds visible-name search, semantic references, compare/revert, and reviewed schemas. |
| Dialog and narrative | Existing text editing and explicit technical topic registration are integrated. The first bounded Quest lifecycle table and typed condition/action dialog are also integrated, but synchronized transcript/outline/general-graph/state views and dialog semantics are **not integrated**. | Compiler, localization, and guarded registration paths have bounded evidence. The deterministic version-3 Viper candidate passes strengthened preflight/forbidden-operation verification and exact sandbox deploy/undeploy closure. For Quest V4, the four external-trigger fields and predicate-hook shapes, all three handler shapes, `bSucceedParent`, typed getters, and guarded lifecycle-call shapes are compiler/cache-pipeline-qualified on G1R 1.0.3; one exact renderer-produced fixture spanning every state-test expression remains an offline gate. | The retained dialog proof is **render-only**: a naturally registered topic appeared, nothing was selected, and no condition, effect, quest state, save, or persistence behavior was proved. The Quest compiler qualifier likewise proves no gameplay ordering or effect. Dialog selection and Quest runtime behavior remain independently **Research-gated**. |
| Localization and spoken dialog | Localization and the compatibility existing-member replacement editor are integrated. Managed R3 Home now exposes bounded **Add Voice take**, **Manage Voice takes**, **Resolve Voice target**, and **Build Voice bundle** actions. Import searches an exact existing line, locale, and safe local Ogg; target resolution and offline build use a configured installation. Selection management does not: it shows the exact slot's candidates in authored order, labels the current choice, disables non-Approved takes, never picks a default, and lets the author explicitly clear the selection with a build-blocking warning. All surfaces hide technical IDs/paths, recheck a fresh checkpoint, and refresh the visible revision/head after managed publication. This is not yet a complete multilingual Voice production workspace. | Import performs semantic/capacity preflight, double-reads the Ogg before accepted CAS installation, and preserves localization plus alternatives. The separate selection transaction binds the exact head/project/target/line/locale/slot revision/current selection and can change only `VoiceSlot.selected` plus project/slot revisions; its FFI accepts no game/source/build/deploy authority, fully reopens an immutable candidate, and checks the fixed head twice without publishing it. Installed-target resolution seals executable/archive evidence and preserves ambiguity. The all-or-nothing Voice builder derives blockers from the complete graph and lowers only selected Approved Vorbis takes with sealed existing-member targets into a deterministic format-3 bundle in a new offline folder. The managed session alone owns guarded fixed-head CAS, repair, and full published reopen. | Selection, target resolution, and project publication do not write the game or a save. The offline builder performs no deployment and proves no audible runtime behavior. Managed deploy/undeploy, isolated testing, ambiguous-member choice, preview/remove/unlink, recording/normalize/transcode/batch/coverage, qualified Opus output, and new-member runtime proof remain missing. Existing-member and new-member behavior retain separate operation/version gates. End state adds complete multilingual production, history, review, release, and runtime qualification workflows. |
| NPCs | Managed R3 Home now has a bounded Guided NPC Draft wizard. The author enters a display name and chooses a qualified archetype through the searchable picker; technical IDs, namespaces, paths, source, and runtime class names stay hidden. It rebuilds catalog evidence at open and immediately before publication, rejects a stale or reopen-required checkpoint, publishes through the exact managed lease, and reloads the revision/content view. The separate schema-revision-2 Story Draft flow remains. This first R3 wizard creates only a logical-clone shell: no semantic visuals/stats/faction/inventory/routine/dialog/quest/spawn editor, production lowerer, deploy action, or runtime workflow exists. Every result is visibly build-blocked, runtime-unqualified, and not spawned. | The revision-3 transaction consumes freshly sealed Story+NPC catalog selection and a base-game-plus-exact-current collision inventory for modules, paths, symbols, and case-insensitive runtime IDs from the pinned catalog projection. It regenerates the existing NPC/Quest/module closure, preserves valid Quests, and fails closed on drift, residual ownership, or collisions. The FFI route rebuilds those inputs, revalidates game/head state, and prepares/reopens an immutable candidate without publishing the fixed head or writing game files. Strict Dart DTOs validate the exact closure; the managed session independently publishes by guarded fixed-head byte CAS, repair journal, and full reopen. The deterministic three-class generator and retained Asghan-derived chain also compile, compose, reopen, and resolve offline. None of this proves build or spawn behavior. | New-class residence, effective visuals, distinct identity, conservative spawn, AI, dialog/quest separation, streaming, save/reload, persistence, and uninstall behavior are **separate NPC research gates**. Pinned catalog runtime IDs do not claim coverage of unknown game NPCs outside that projection. |
| Quests | Managed R3 has the friendly one-to-eight-objective Draft wizard plus three selected-Quest actions: **Name & objectives**, **Description & connections**, and **States & transitions**. The new behavior table covers the main Quest and objectives across Available/Start/Success/Failure, offers a sequential template, and edits independent direct-engine triggers, typed DNF conditions, bounded cross-node Start/Succeed/Fail actions, and objective-parent completion without exposing technical IDs or source. It does not author objectives/text/connections, dialog, journal, rewards, items, raw AngelScript, or arbitrary effects. Merely reading or deriving the effective behavior plan does not migrate generator-v2/v3 project JSON/source or add `transition_plan`; an otherwise unchanged Quest remains byte-identical, while separate outline/context edits still regenerate their owned fields. V4 retains stable objective slots and a canonical plan whose frozen seed reproduces the legacy source bytes. Outline V1 refuses V4 until a slot-aware successor exists; context V1 preserves V4 exactly. | The native V4 validator closes canonical order/references, required drivers/edges, 8x8 predicate and 8-effect limits, lifecycle contradictions, same-handler terminal conflicts, self effects, and same-kind cycles. The pure transaction binds exact head/project/target/Quest revision and plan seal, proves/regenerates the owned module, and updates only project/Quest/module revisions. Its prepare-only FFI accepts exactly `current_project_json`, `quest_transitions_request_json`, and `root`, no `game_root`; it fully reopens the immutable candidate and never publishes the head. Native status stays `blocked`, `runtime_unqualified`, and publication `not_supported`; only the serialized managed session may publish through exact-head byte CAS, repair, and full reopen. Two isolated 1.0.3 compiler runs covered the four external-trigger fields and predicate-hook shapes, all three handler shapes, `bSucceedParent`, typed getters, and guarded lifecycle-call shapes and produced the same reopened 7,306-module cache. One renderer-produced fixture spanning every state-test expression remains an offline gate, and `default-sites` still cannot enumerate new-class defaults. No Quest operation writes the game installation or a save. | New-class discovery is narrowly runtime-proven on one generation, and the listed lifecycle source shapes are compiler/cache-qualified; neither proves exact full renderer output, generated transition polling/order, or effects in game. Dialog selection, journal, rewards/items/knowledge, persistence, save/reload, uninstall, other versions, complete build/deploy, and general story graphs remain **Research-gated** independently. |
| Items and economy | Existing scalar edits are an integrated subset; semantic clone/new-item/economy workflows are not. | Bounded existing-value paths exist. A general new-item identity/package pipeline is not offline-proven. | New identity, construction, visuals, equip/use behavior, trade/loot integration, and persistence are **Research-gated** independently. |
| World, routines, and spawns | No semantic map/routine/spawn authoring surface is integrated. | A typed Draft model and an optional sealed Unreal handoff are planned, but neither a bridge nor arbitrary level/world-partition output is implemented or proven. | Qualified anchors, schedules, spawns, triggers, navigation, streaming, ownership, and persistence require operation-specific research. A map pin or Unreal handoff must not imply writable or game-compatible world content. |
| DataAssets and cooked content | The read-only Lab, generic fixed-leaf editor, installed-package browser, and managed **Verified DataAsset edits** registry are integrated bounded subsets. The first reviewed form recognizes only exact Human/Scavenger/Wolf Footstep targets after inspection and exposes `FeetTextureSize` X/Y with scale presets, Before/After preview, preserved Z/W, and raw-unit warnings. Expert receipt import/edit and registry-only removal remain. No surface exposes offsets, structural editing, build/pack/deploy, gameplay, or Unreal-bridge authority. | The reviewed request carries only exact head/ordinal/package-source seals plus closed schema/field/X/Y intent. Native code rediscovers the exact `FootstepTag` Vector4 selector, preserves Z/W, and reuses the installed typed-stage executor through two complete drift windows. Separate reviewed, ordinary stage, and installed-proof bindings are independently checked by Dart before guarded exact-head publication. Results remain build-blocked, runtime-unqualified, artifact-not-granted, and native-publication-not-supported. Broader reviewed schemas, gameplay-qualified units, multi-edit/undo, build lowering, post-pack verification, structural package/reference/collection writing, and the sealed Unreal handoff remain missing. | The Footstep property shape and offline lowering are proven; its gameplay units/effect are not. Managed project staging grants no pack, deployment, gameplay, or save authority. Structural creation remains **Research-gated** until complete round-trip and runtime qualification. A stock Unreal Editor is not assumed to open cooked G1R packages or emit compatible output. |
| Visual media | Existing texture replacement is integrated. General visual content and the optional Unreal handoff are not. | Texture bundle output has bounded evidence; general material, mesh, character visual, animation, VFX package creation, and Unreal round-trip do not. | New cooked visual registration/resolution is **Research-gated**. End state includes import validation, thumbnails, lineage, qualified previews, and an optional sealed specialist-tool handoff only for explicitly supported asset types. |
| Audio and music | FMOD sample browsing/preview/replacement is integrated. Semantic cue/event creation is not. | Existing-bank replacement has bounded backend evidence. | New event/cue integration is **Research-gated**. End state adds batch normalize/transcode, loudness/codec checks, ownership, and conflict handling. |
| Cinematics and presentation | No Studio integration. | No current authoring pipeline claim. | Scene timelines, cameras, staging, subtitle/audio sync, animation, and reusable sequences are **future Research-gated** capabilities. |
| Gameplay systems | Only bounded existing scalar/default and Expert script paths are available. | Evidence is selector-, field-, and generator-specific. | General factions, AI, combat, talents, spells, economy, rules, and reusable runtime effects are **Research-gated**; Expert source cannot bypass qualification. |
| UI and player-facing presentation | No semantic UI authoring integration. | Generic texture/script support is not evidence for mod-owned UI. | Journal/menu/icon behavior and new UI remain **future Research-gated** until their actual chains are recovered. |
| Test and debug | Build/deploy controls exist, but named isolated scenarios, managed test profiles, semantic observations/save diff, and recovery orchestration are not integrated. | Offline validators and receipt-driven deployment provide components, not the complete scenario lifecycle. | Each gameplay action needs its own risk profile and runtime evidence. End state includes offline simulation, isolated tests, logs, observations, save diff, and verified cleanup. |
| Build and release | Studio can build/deploy represented legacy domains. Project-wide semantic build roots, dependencies/rebase, immutable releases, and CI are not integrated. | The bundle engine can build/reopen/inspect represented components. | A bundle is not gameplay proof. End state adds deterministic profiles, semantic plans/conflicts, compatibility, rollback, provenance, and closed-world release validation. |
| Collaboration and extension | No semantic collaboration workflow is integrated. | Canonical V2 primitives are groundwork, not merge/sync implementation. | Planned after the single-author managed-project/transaction contract; core authoring must not require a cloud account. |

DataAsset status addendum (2026-07-14): the managed installed-package browser
now supplies a second normal typed fixed-leaf route alongside the retained
ExtractReceipt workflow described in the matrix. It selects only a sealed
original ordinal, binds the exact package/USMAP inspection, independently
reconstructs and compares package bytes, role-bearing sidecars, parsed UTOC
identities, chunk winners, and USMAP name/bytes, then publishes only the same
build-blocked revision-3 stage by exact-head CAS. No extracted receipt or
caller-supplied target/package/output authority crosses this route. Structural
editing, package creation, build/pack/deploy, and runtime qualification remain
missing exactly as stated above. The first schema layered on this route now
recognizes only exact Human/Scavenger/Wolf Footstep presets and one reviewed
`FeetTextureSize` X/Y field. The guided form preserves Z/W and labels the values
as raw asset units; neither gameplay effect nor units are qualified.

"Complete" does not mean exposing every Unreal file type. It means every
advertised operation is semantic, reversible, deterministic, inspectable, and
qualified for the selected game version. Unknown game-source/property
structures remain visible and preserved but read-only; unknown required project
schema content follows the strict block-or-migrate contract in Section 5.

### 3.1 Planned optional Unreal Editor hybrid

For reviewed DataAsset, visual-media, and world-content operations, the intended
end state may use Unreal Editor as an optional specialist surface instead of
recreating its high-fidelity tools inside Mod Studio. No such bridge is
implemented today. A future bridge must export a bounded, versioned handoff
manifest tied to the selected game generation, exact input seals, declared
asset identities/references, adapter version, and required editor/plugin
identity. Re-import is a new validated Mod Studio transaction over declared
outputs; launching Unreal or exporting files alone is not a round trip.

Mod Studio remains the source of truth for semantic IDs, references, project
history, provenance, validation, and Build/Test/Release. The Unreal workspace
is an explicit tool workspace, not a second implicit project state, deployment
path, or authority source. The bridge never writes the game installation.
Authored source and accepted outputs enter the managed project only through its
bounded AssetStore/import rules.

This plan does **not** claim that stock Unreal Editor can open cooked G1R
packages or emit game-compatible cooked content. Matching a nominal engine
version is insufficient: package/reference/cook/registration chains, game
plugins and custom types, output reopening, and runtime behavior must each be
proven for the exact operation and generation. Unsupported outputs remain
Draft-only and build-blocked. The handoff is contextual in DataAsset, visual,
and World workflows or Expert mode; it is not a new permanent top-level tab and
is not required for ordinary supported Studio operations.

### 3.2 Remaining legacy-session limits

The current format-1 Flutter session must not be treated as the durable
foundation for new authoring domains. Strict archive validation, atomic
publication, current-path and workspace ownership, a serialized Open/Save/New
lane, exact saved snapshots, metadata-aware dirty state, true **Save** versus
**Save As**, and `Ctrl+S` make it a substantially safer compatibility/import
bridge.

The separate schema-revision-2 Story workspace already proves an exclusive
lock, serialized derive/save, exact-head CAS publication, repair journal, and
full reopen through the working store. A dedicated revision-3 session API
reuses the same safety core for durable identity, exact-head reads, and bounded
Quest/NPC/Voice/DataAsset mutations without inventing build or runtime
authority.
The typed current-project coordinator is now adopted by Home, the Project menu,
and `Ctrl+S`: a friendly form plus empty-directory picker can create a canonical
generation-bound R3 project, and existing R3 directories can become the visible
current project. Their durable identity and exact-current semantic content
projection are shown, and managed Save verifies the exact head. Creation uses
the same absent-head publication, full reopen, recovery, and single-owner
adoption rules rather than introducing another project format.
Dirty transitions, failed candidate preservation, `requiresReopen`, and terminal
cleanup diagnostics are handled at the shell boundary. Legacy editors,
Build/Deploy, Save As, and Story actions cannot act on hidden compatibility
state while R3 is current. Bounded Quest/NPC Draft, Voice import/selection/
target, and verified DataAsset actions now share that R3 owner; this is a
managed authoring shell, not yet a unified semantic content-authoring flow.

The remaining managed-authoring limits are:

- the bounded format-1 encoder/reader is intentionally in-memory and capped; it
  is not the streaming, sharded, content-addressed storage needed by large mods;
- provider replacement is still not one rollback-capable all-domain
  transaction, even though all represented keyed deployment targets now receive
  one duplicate-validation pass before mutation or publication;
- format-1 embedded source paths are extraction derivatives rather than
  immutable AssetStore refs, so they remain unsuitable as durable entity state;
- autosave recovery, named checkpoints, crash-safe transaction replay, revision
  migration orchestration, and general managed AssetStore blob-ownership tools
  are not implemented.

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
| **Save / checkpoint** | Durably acknowledge the current revision | Target contract: `Ctrl+S` flushes current transaction state and creates/advances a recoverable checkpoint; **Save As** creates a separately validated identity/path. Current R3 shell: semantic transactions publish independently, `Ctrl+S` only fully verifies the exact head, and managed Save As stays disabled until native clone/fork exists. |
| **Export** | Portable backup, interchange, or review copy | A deterministic `.goremod` archive produced from an immutable snapshot; exporting does not silently change the current working path, and opening it imports to a managed working directory |
| **Build** | Produce an inspectable mod artifact | Derived from an immutable project revision and named build root/profile; it does not deploy and cannot become editable source state |
| **Test deployment** | Install one build into an isolated test profile | Receipt-owned, game-closed preflight, explicit disposable save choice, bounded logs/observations, and verified cleanup |
| **Release** | Publish a reproducible user-facing package | References an immutable closed-world validated revision/build plus compatibility, dependency, license, changelog, hashes, and provenance |

The bounded line-centric Voice workflow extends the managed revision-3 session
instead of adding another format-1 list or parallel project state. Exact-head
transactions now import takes, select or clear an existing candidate, and
resolve generation-bound existing archive targets through guarded session
publication, repair, and full published reopen. Separately, the exact-current
offline builder reads verified selected Store bytes, stages and completely
reopens/seals a deterministic format-3 Voice tree, then atomically promotes it
with no-replace semantics without publishing the project head. Production
completion still needs preview/remove/unlink and history, explicit ambiguity
choice, recording/transcode and coverage, managed deploy/undeploy, an isolated
test profile, audible runtime qualification, and a separately proven new-
member path. The landed offline foundation must not be presented as that
complete workflow.

V2 is a closed, versioned contract. Unknown required project formats, schema
revisions, entity kinds, payload variants, or fields are never ignored or
silently round-tripped through a partial model. If a reviewed migration exists,
Studio imports into a new managed project, reports every transformation, saves,
reopens, and leaves the source untouched. Otherwise it blocks editing and build
with a version explanation and offers only operations that preserve the bytes.
Optional forward data is allowed only inside an explicitly versioned extension
envelope with declared preservation semantics; there is no generic catch-all
map. Closed revision-1/2/3 parsers, revision-2-to-3 migration, working-store
persistence, dedicated revision-3 read/prepare FFI/Studio DTOs, and a managed R3
session with exclusive locking, serialized saves, verified fixed-head CAS
publication, repair, and full reopen exist. The typed Legacy/R3 current-project
coordinator is now app-shell adopted for existing-R3 Open, identity display, and
exact-head Save verification, with stale Legacy actions blocked. Bounded
Quest/NPC Draft, Voice import/selection/target, and DataAsset-stage editing now
use the managed session. General semantic editing, migration orchestration into
that session, clone/Save As, autosave/full history, and all-domain transactions
remain integration work; the visible schema-revision-2 Story flow keeps its narrower
scope.

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

Today the separate bounded NPC and Quest Draft wizards exist, but this combined
new-NPC/new-quest slice remains a **planned Draft workflow and not a supported
production promise**. The important product decision is that authors can
eventually scaffold and organize the whole intent without hand-authoring
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

1. **Finish the managed phase-one substrate:** the owned R3 working directory,
   serialized session, strict bounded open/import, Ogg AssetStore I/O, exact
   revision/head publication, repair, full reopen, and first Voice transaction
   are integrated. Complete general recovery/history, cross-domain undo,
   deterministic export, and production lowering without creating a parallel
   project state.
2. **Complete the first non-technical Voice slice:** extend the landed import,
   Approved-take selection/clear, generation-bound target resolution, and
   sealed existing-member offline build with preview/remove/unlink, explicit
   ambiguity choice, recording/transcode, coverage, history, managed deploy/
   undeploy, and an isolated audible test profile without archive terminology.
   Project persistence and offline bundle construction alone do not satisfy the
   production milestone.
3. **Authoring shell:** move represented domains onto the same managed graph;
   add the stable unified Library, references, transactions, history,
   templates, bulk tables/import, coverage, isolated test profiles, and
   deterministic release for already represented domains.
4. **Semantic breadth and safe Drafts:** deepen the landed bounded NPC/Quest
   wizards, Quest outline/context/V4 lifecycle editors, and DataAsset receipt
   registry into complete existing-content views, semantic forms/graphs,
   diagnostics, undo, origin drift, and three-way game-version rebase. Reuse
   the implemented deterministic generators; valid Draft persistence and
   honest build blocking do not satisfy the complete authoring journeys.
5. **Story production:** extend the landed bounded Quest behavior table into
   synchronized transcript/outline/general-graph/state views, reusable
   conditions/effects, journal/reward/item semantics, simulation, complete
   build diagnostics, and separately qualified dialog selection and Quest
   runtime-transition mechanisms.
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

- Do not recreate Unreal Editor as a collection of raw object trees. Use the
  planned optional handoff only for explicitly supported, sealed operations.
- Do not call launching stock Unreal, exporting loose files, or importing an
  unchecked `.uasset` a bridge round trip or game-compatible output.
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
