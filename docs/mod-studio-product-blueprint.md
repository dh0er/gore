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

### Legacy usability is the migration baseline

The managed-R3 project is the long-term data and transaction foundation, but
the current R3 shell is not yet a usability replacement for the compact Legacy
tabs. Legacy already gives authors direct access to useful item, dialog, audio,
texture, script, change, settings, and DataAsset workflows. A safer backend does
not justify making those jobs harder to find or spreading a small number of
actions across status cards and disconnected modal dialogs.

Migration therefore follows a strict parity rule: keep a useful Legacy workflow
available until its managed replacement is at least as clear, direct, and
capable. R3 should converge on coherent workspaces where related browsing,
editing, validation, and next actions happen in place. The canonical information
architecture names product responsibilities; it does not require every
responsibility to become a separate dashboard-like landing page. Prefer
progressive disclosure inside a small number of productive surfaces over more
navigation, cards, and modal launchers. Technical IDs, evidence, and diagnostics
remain available through details or Expert mode without dominating normal work.

The implemented managed-project Home follows that rule: it is one compact
five-task router for **Story**, **Dialog & Voice**, **Problems**, **Content**,
and **Build & Release**. It does not duplicate low-level create/import/build
dialogs. NPC and Quest creation—including the recommended Quest plus opening
line—belongs to Story; dialog and Voice work belongs to Localization & Voice;
DataAssets belong to Content; export remains in the Project menu. Normal chrome
uses “Mod Studio project” rather than schema-revision terminology. The nine
stable shell destinations remain intact and Legacy remains the usability floor.
The persistent **Project Work Bar V1** now keeps the friendly project name and
current area visible above every destination and exposes the three common
continuations **Search**, **Create**, and **Problems**. It reuses the owning
workspaces and their existing authority instead of creating another dashboard
or mutation path.

World and level authoring has two gates. First, the existing project, content,
Story, dialog/localization/Voice, DataAsset, validation, build/release, and
recovery workflows must be genuinely usable end to end. Second, once that core
checkpoint is clean and pushed, implementation stops for user review. World or
level work begins only after explicit user approval; satisfying the first gate
must never start it automatically.

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
| Project and content library | The compatibility project remains a legacy format-1 bridge and the separate Story flow remains schema revision 2. The primary Project menu creates or opens one authoritative managed-R3 project. The canonical shell exposes Home, Content, Story, World, Localization & Voice, Validate & Test, Build & Release, History, and Settings & Expert. Its persistent Project Work Bar shows the friendly project name and current area everywhere, with global **Search**, **Create**, and **Problems** continuations; compact/high-scale layouts keep Search direct and move Create/Problems into an accessible overflow. Content contains a responsive `This mod` / `Base game` / `Installed` / `Search all` browser plus the separate verified-DataAsset-edits route. Scope pages mount lazily, retain state for the same project, and reset on project identity change. Project creation offers Empty, NPC Draft, and Quest Draft starters with the two-stage boundary visible before creation. Unsupported domains stay visible and honestly labelled. | Home and `This mod` read the exact current index. Work-Bar Search opens the existing `Search all` scope, focuses its query field, and performs zero source loads until the author submits a nonempty query. `Base game` derives curated NPC/Quest starting points from one generation-consistent evidence load and exposes broader NPC linkage only as search-gated inspect-only metadata. `Installed` is a query-first view over the exact managed package snapshot, capped at 100 rendered matches, and hands the selected canonical path to the existing exact inspector. `Search all` scans all three source projections independently in memory, retains at most 100 results per source, and reports each source's loading/complete/partial/error state separately. Its exact actions stay within the selected source: open a current mod identity, create a Draft from an exact Base catalog identity, or inspect an exact installed target. Work-Bar Create reuses the existing NPC Draft, recommended Quest plus opening line, and new dialog-line flows; NPC/Quest remain visibly gated when no game is configured, while every creation action remains gated by exact checkpoint, clean managed workspace, and recovery state. Problems opens the existing Validate & Test owner. Every new-project starter first creates/adopts canonical empty revision 0 through absent-head CAS; NPC/Quest then uses the existing guided exact-head publication as a separate revision. Cancel before publication retains the exact empty project, while an uncertain publication enters `requiresReopen` and permits bounded same-lock recovery or close/open. Story, Voice, Validate, the Voice-only offline builder, bounded History/Undo, and Settings remain as previously bounded. | The Work Bar is orientation and routing only: it grants no new read, mutation, publication, game/save-write, deployment, build, or runtime authority. Browsing, searching, and starter selection write neither the game installation nor a save and grant no deployment or runtime authority. The three search sources do not form one atomic combined snapshot; results grant no dependency, reference, backlink, or cross-source mutation authority. Installed rows are discovery metadata, not edit authority; Base-game Draft actions remain build-blocked/runtime-unqualified. General editors, migration/import/clone/Save As, named checkpoints, semantic history diffs, and recovery beyond the bounded timeline/`requiresReopen` repair, dependencies, references, indexed/virtualized large-scale search, collections, reusable/multi-domain templates, unified transactions, blob ownership, full command palette/undo/build/test chrome, full build/deploy, and runtime qualification remain missing. |
| Existing game content | Item scalar, localization/dialog text, FMOD, texture, script, change, and build surfaces are **integrated subsets**. They are separate views/providers rather than one semantic graph. | Bounded scalar/default, localization, archive, script, texture, bundle, and fixed-leaf DataAsset paths have separate offline evidence. Evidence for one field/format does not generalize to another. | Only operations named in the normative matrix may be presented as supported. End state adds visible-name search, semantic references, compare/revert, and reviewed schemas. |
| Dialog and narrative | Existing text editing and explicit technical topic registration are integrated. Selected managed Quests now also have an ordered **Dialog & Voice** transcript: authors see friendly speaker/text/locale/Voice coverage, attach existing project lines, reorder or detach them, group semantic-V4 lines by stable objective, create a line atomically, and open its exact text/Voice editor. Outline, lifecycle, and transcript remain separate bounded projections rather than one synchronized general conversation/Quest graph, and runtime dialog semantics are **not integrated**. | Compiler, localization, and guarded registration paths have bounded evidence. The deterministic version-3 Viper candidate passes strengthened preflight/forbidden-operation verification and exact sandbox deploy/undeploy closure; the current builder reproduced its five-file bundle byte-for-byte and the current deploy path restored a fresh copied sandbox exactly with no lock, backup, or record residue. For Quest V4, the four external-trigger fields and predicate-hook shapes, all three handler shapes, `bSucceedParent`, typed getters, and guarded lifecycle-call shapes are compiler/cache-pipeline-qualified on G1R 1.0.3; one exact renderer-produced fixture spanning every state-test expression remains an offline gate. | The retained dialog proof is **render-only**: a naturally registered topic appeared, nothing was selected, and no condition, effect, quest state, save, or persistence behavior was proved. Quest transcript metadata grants no topic, selection-effect, build, deployment, or runtime authority. The Quest compiler qualifier likewise proves no gameplay ordering or effect. Dialog selection and Quest runtime behavior remain independently **Research-gated**. |
| Localization and spoken dialog | The compatibility existing-member replacement editor remains integrated as a separate path. Managed R3 **Localization & Voice** now opens on a responsive **Work list**, with **Project texts** as the adjacent full-text editor. The list shows only two evidence-backed item types: an authoring locale absent from a safely editable project text, and an intact already-existing `VoiceSlot` for one line/locale. It never invents missing-recording intent when no slot exists. Each slot receives one exact next step—add recording, review/approve, select/repair, resolve target, or production decisions complete—and reuses the existing exact line/locale workflow. “Production decisions complete” links to checks and is not a Ready/build/runtime claim. Project texts still supports friendly search, complete multilingual maps, safe locale edits, shared line/speaker backlinks, and the guided **New dialog line**, **Add Voice take**, **Manage Voice takes**, and **Resolve Voice target** flows. A selected Quest transcript row can jump into this same workspace and exact line/language. Technical IDs/LocIDs stay hidden. Manage previews exact managed Ogg media, edits status/selection, and safely removes one take; VoiceSlot locales cannot be removed or blanked and candidate-backed transcripts remain locked. A fresh project can create one project-owned localization and line plus an optional empty locale slot, or reuse one exact unused managed localization. The speaker remains only a label, and the Voice-only offline builder remains separate. | Queue derivation is a pure bounded projection over matching localization/Voice catalog checkpoints. It retains at most 500 normal rows, prioritizes actionable work, reports omitted rows, preserves known language work when Voice evidence is unavailable, and fails closed on checkpoint mismatch. The managed host and every callback retain exact root/project/revision/canonical-head lifecycle checks; same-revision head drift, late reads, changed next-step evidence, dirty text, and `requiresReopen` cannot authorize an old row. Full-text editing remains bound to one exact project-owned `origin=new` LocalizationEntry and may change only the project/localization revisions, text map, and union of authoring locales. Dialog-line create/reuse, take import, status, selection, slot-scoped removal, target resolution, exact-current managed-CAS preview, and the deterministic format-3 Voice bundle keep their independently bounded prepare/full-reopen/publication evidence. The queue creates no second mutation format and all actions are globally single-flight. | Project-text editing and line/Voice project publication write neither the game nor a save and grant no topic, AngelScript, NPC/runtime binding, build, deployment, or gameplay authority. The new line still has no playable dialog path and remains build-blocked/runtime-unqualified; the offline Voice builder performs no deployment and proves no audible runtime behavior. Vanilla adoption needs a sealed generation-bound catalog. Localization delete/clone, broader line/slot relationship editing, bulk translation/CSV/XLIFF, broader coverage and batch/team review queues, history/rebase, complete conversation graphs, managed deploy/undeploy, isolated testing, recording/normalize/transcode, Opus build/runtime qualification, and new-member runtime proof remain missing. |
| NPCs | Managed R3 has a bounded Guided NPC Draft wizard and a direct existing-Draft **Edit name & archetype** action in the selected NPC's Profile. Both normal forms use a friendly display name and verified archetype choices while hiding technical IDs, `UniqueName`, namespace/path, origins, source, and runtime class names. The profile editor leaves all of that stable technical identity unchanged. It refreshes Story+NPC catalogs at open and immediately before save, rejects stale/reopen-required state, publishes through the exact managed lease, and reloads the revision/content view. The separate schema-revision-2 Story Draft flow remains. These R3 forms still cover only a logical-clone shell: no semantic visuals/stats/faction/inventory/routine/dialog/quest/spawn editor, production lowerer, deploy action, or runtime workflow exists. Every result remains visibly build-blocked, runtime-unqualified, and not spawned. | Creation consumes freshly sealed Story+NPC catalog selection plus a base-game/exact-current collision inventory. Existing-profile preparation instead resolves both current and desired records from fresh catalogs and compares the complete three-parent provenance—generation, source seal, catalog layer, selector, and runtime class for `CharacterDefinition`, `AIAgentConfig`, and `SpawnAIAgentDefinition`. A catalog alias with the same full triple is not a structural change. Name-only edits increment the project/NPC while preserving the complete owned ScriptModule and its revision; a changed triple atomically replaces all three provenances and deterministically regenerates/revisions only that module. Both prepare-only FFI paths revalidate game/head evidence and fully reopen immutable candidates without publishing the fixed head or writing game files. Strict Dart DTOs validate the bounded results; the managed session independently publishes by guarded fixed-head byte CAS, repair journal, and full reopen. The deterministic three-class generator and retained Asghan-derived chain compile, compose, reopen, and resolve offline, but none of this proves production build or spawn behavior. | Profile editing is project-only; the configured installation is read for catalog evidence, while game files and saves remain unchanged. Build, deployment, class residence, effective visuals, distinct identity, conservative spawn, AI, dialog/quest separation, streaming, save/reload, persistence, and uninstall behavior are **separate NPC research gates**. Pinned catalog runtime IDs do not claim coverage of unknown game NPCs outside that projection. |
| Quests | Managed R3 has the friendly one-to-eight-objective Draft wizard plus four canonical selected-Quest tabs: **Journey**, **Dialog & Voice**, **References**, and **Problems & Checks**. Journey presents main-Quest and objective behavior and retains the contextual **Name & objectives**, **Description & connections**, and **States & transitions** handoffs; legacy Story/Logic requests normalize to Journey. The behavior table covers the main Quest and objectives across Available/Start/Success/Failure, offers a sequential template, and edits independent direct-engine triggers, typed DNF conditions, bounded cross-node Start/Succeed/Fail actions, and objective-parent completion without exposing technical IDs or source. The new responsive transcript orders project DialogLines, previews localized text and Voice coverage lazily, attaches/reorders/detaches existing lines, groups V4 lines by stable objective, atomically creates a line/localization/optional empty Voice slot, and hands one exact line/language to Localization & Voice. It does not author a runtime topic, selection effect, journal, rewards, items, raw AngelScript, or arbitrary effects. Merely reading or deriving behavior or an empty transcript does not migrate generator-v2/v3 project JSON/source; an otherwise unchanged Quest remains byte-identical. V4 retains stable objective slots and a canonical plan whose frozen seed reproduces the legacy source bytes. Outline V2 changes V4 display/title text and presentation order while carrying those slots; legacy V2/V3 Quests retain Outline V1. Context, Outline, Transitions, and source inspection preserve transcript metadata exactly. | The native V4 validator closes canonical order/references, required drivers/edges, 8x8 predicate and 8-effect limits, lifecycle contradictions, same-handler terminal conflicts, self effects, and same-kind cycles. The transcript model adds at most 256 unique exact same-project DialogLine refs, with objective grouping only for active V4 slots. Its pure exact-basis transaction either replaces the complete reviewed order or embeds the existing DialogLine insertion and commits the compound result in exactly one project/Quest revision while leaving the owned ScriptModule revision/source, transition plan, assets, and unrelated entities unchanged. Empty transcript stays omitted. The prepare-only FFI has no game root, fully reopens its immutable candidate, repeats fixed-head guards, and never publishes; only the serialized managed session may publish through exact-head byte CAS, repair, and full reopen. Detach never deletes a line, and Quest removal drops only outgoing transcript refs while retaining shared line/localization/Voice content. Native status stays `blocked`, `runtime_unqualified`, topic authority `not_granted`, and publication `not_supported`. Two isolated 1.0.3 compiler runs covered the four external-trigger fields and predicate-hook shapes, all three handler shapes, `bSucceedParent`, typed getters, and guarded lifecycle-call shapes and produced the same reopened 7,306-module cache. One renderer-produced fixture spanning every state-test expression remains an offline gate, and `default-sites` still cannot enumerate new-class defaults. No Quest operation writes the game installation or a save. | New-class discovery is narrowly runtime-proven on one generation, and the listed lifecycle source shapes are compiler/cache-qualified; neither proves exact full renderer output, generated transition polling/order, effects in game, or any transcript-driven dialog behavior. Dialog selection, journal, rewards/items/knowledge, persistence, save/reload, uninstall, other versions, complete build/deploy, and a synchronized general story graph remain **Research-gated** independently. |
| Items and economy | Existing scalar edits are an integrated subset; semantic clone/new-item/economy workflows are not. | Bounded existing-value paths exist. A general new-item identity/package pipeline is not offline-proven. | New identity, construction, visuals, equip/use behavior, trade/loot integration, and persistence are **Research-gated** independently. |
| World, routines, and spawns | The canonical World destination is integrated as an honest unavailable/status surface; no semantic map/routine/spawn authoring surface exists. | A typed Draft model and an optional sealed Unreal handoff are planned, but neither a bridge nor arbitrary level/world-partition output is implemented or proven. | Qualified anchors, schedules, spawns, triggers, navigation, streaming, ownership, and persistence require operation-specific research. A visible World page, map pin, or Unreal handoff must not imply writable or game-compatible world content. |
| DataAssets and cooked content | The read-only Lab, generic fixed-leaf editor, installed-package browser, and managed **Verified DataAsset edits** registry are integrated bounded subsets. A normal quick start names the reviewed Human/Scavenger/Wolf Footstep presets, opens installed data as the primary action, and keeps receipt-based generic paths under **Expert tools**. The reviewed form exposes `FeetTextureSize` X/Y with scale presets, Before/After preview, preserved Z/W, and raw-unit warnings. A successful publication returns to the exact advanced checkpoint, reloads, and expands only the matching target and staged revision; **Build files...** is then the conditional next step. Its focused dialog asks only for a portable pack name and destination parent and creates a new offline mod-file folder. No surface exposes offsets, structural editing, deploy/gameplay, or Unreal-bridge authority. | The reviewed edit request still carries only exact head/ordinal/package-source seals plus closed schema/field/X/Y intent. The managed build runs through the serialized `readBasisSnapshot` lane and a native exact-current final Store gate, independently replays the fixed-leaf edit from the live generation, strictly reopens and semantically re-inspects the generated triplet, and emits a path-free canonical basis/post-pack receipt plus three relative-name/length/hash seals. Protected-root and recognizable-game-install checks precede an atomic absent-output no-clobber rename. Published, cleanup-warning, and publication-uncertain are distinct sealed terminals and none is automatically retryable. Broader reviewed schemas, gameplay-qualified units, multi-edit/undo, structural package/reference/collection writing, and the sealed Unreal handoff remain missing. | The Footstep property shape and one exact offline triplet build are proven; its gameplay units/effect are not. Build neither deploys nor mutates the project, Store head, game installation, or save, and it grants no runtime authority. Structural/new DataAsset creation remains **Research-gated** until complete round-trip and runtime qualification. A stock Unreal Editor is not assumed to open cooked G1R packages or emit compatible output. |
| Visual media | Existing texture replacement is integrated. General visual content and the optional Unreal handoff are not. | Texture bundle output has bounded evidence; general material, mesh, character visual, animation, VFX package creation, and Unreal round-trip do not. | New cooked visual registration/resolution is **Research-gated**. End state includes import validation, thumbnails, lineage, qualified previews, and an optional sealed specialist-tool handoff only for explicitly supported asset types. |
| Audio and music | FMOD sample browsing/preview/replacement is integrated. Semantic cue/event creation is not. | Existing-bank replacement has bounded backend evidence. | New event/cue integration is **Research-gated**. End state adds batch normalize/transcode, loudness/codec checks, ownership, and conflict handling. |
| Cinematics and presentation | No Studio integration. | No current authoring pipeline claim. | Scene timelines, cameras, staging, subtitle/audio sync, animation, and reusable sequences are **future Research-gated** capabilities. |
| Gameplay systems | Only bounded existing scalar/default and Expert script paths are available. | Evidence is selector-, field-, and generator-specific. | General factions, AI, combat, talents, spells, economy, rules, and reusable runtime effects are **Research-gated**; Expert source cannot bypass qualification. |
| UI and player-facing presentation | No semantic UI authoring integration. | Generic texture/script support is not evidence for mod-owned UI. | Journal/menu/icon behavior and new UI remain **future Research-gated** until their actual chains are recovered. |
| Test and debug | Build/deploy controls exist, but named isolated scenarios, managed test profiles, semantic observations/save diff, and recovery orchestration are not integrated. | Offline validators and receipt-driven deployment provide components, not the complete scenario lifecycle. | Each gameplay action needs its own risk profile and runtime evidence. End state includes offline simulation, isolated tests, logs, observations, save diff, and verified cleanup. |
| Build and release | Studio can build/deploy represented legacy domains. The compatibility Scripts workspace now consumes a bounded structured compiler report and an app-scoped read-only install-safety state: Compile and Deploy stay disabled while the game runs, inspection is uncertain, or compiler/deploy recovery evidence exists; a fresh native Recheck is required to clear the dominant recovery banner. Offline Build remains available because it does not mutate the installation, and Undeploy remains reachable so the native recovery path can decide safely. Managed Quest/NPC compile, diagnostics, build, and deploy flows are still not integrated. Project-wide semantic build roots, dependencies/rebase, immutable releases, and CI are also missing. | The bundle engine can build/reopen/inspect represented components. `gore-as` exposes `InstallRestoreDisposition` independently from compile outcome and captured diagnostics, while the additive FFI route reports typed coordinates/severity, explicit normal-generator fallback, exact restoration, and pre-existing recovery without parsing formatted error text. Native code creates only a uniquely owned, marker-bound compile child, holds one shared install-mutation guard across deployment-aware pristine selection and compiler use, rechecks the shipping process immediately before the first live-content or recovery write, and retains a durable blocker when cleanup cannot be proved. Compile, deploy, manager apply, and undeploy share that guard; the read-only probe also projects persistent `gore-mod` recovery. | The process recheck narrows but cannot eliminate a launch race because the game does not honor the toolkit lock. Typed restore/install state is a compatibility safety integration, not managed Quest/NPC compiler support, future-signature compatibility, or gameplay proof; unsupported diagnostics hooks still use the normal compiler fallback. End state adds deterministic profiles, semantic plans/conflicts, compatibility, rollback, provenance, closed-world release validation, and guided recovery/isolated-test workflows. |
| Collaboration and extension | No semantic collaboration workflow is integrated. | Canonical V2 primitives are groundwork, not merge/sync implementation. | Planned after the single-author managed-project/transaction contract; core authoring must not require a cloud account. |

NPC Dialog & Voice addendum (2026-07-17): this supersedes the NPC table row's
older statement that NPC dialog is wholly unmodeled. An exact-current managed
NPC now has a productive **Dialog & Voice** greeting editor. It presents an
ordered friendly transcript with language/text/Voice coverage, supports
attach/reorder/detach, creates a DialogLine plus localization and optional empty
Voice slot atomically at one position, and hands one exact line/language to the
existing Localization & Voice workspace. Technical identities remain hidden in
the normal UI, existing profile/source/removal paths preserve the metadata, and
compact 360-pixel/200%-text layouts remain scrollable. The optional
`NpcDraft.greetings` field is same-project, unique, ordered, capped at 256, and
omitted when empty. Its pure replace/create-and-insert transaction advances
only the project and NPC while preserving the generated ScriptModule bytes and
revision; prepare-only FFI and managed fixed-head CAS have no game/install/save
parameter. This is authoring metadata only: no AngelScript topic, condition,
choice/effect, Quest relationship, lowering, build, deploy, runtime, or
playability authority is granted. Nonempty R3 snapshots require a Studio/Core
version that understands this additive field; older closed-schema builds may
reject them rather than silently discard it.

Character Draft continuation addendum (2026-07-17): normal managed-R3 NPC
creation no longer ends on the launching Home, Story, Base-game, or cross-source
search surface. After publication Studio verifies the fully reopened root,
project ID, revision, and canonical head, mounts **Story**, selects the exact
new NPC, and opens its existing **Dialog & Voice** greeting editor. Cancellation
publishes nothing; project switches, head drift, reopen-required state, or a
disposed owner stop the handoff rather than guessing. The wizard's complete
visible copy is injectable, has English and German production variants, hides
technical identities, remains usable at 360 logical pixels and 200% text
scaling, and keeps publication single-flight. The NPC Profile offers a direct
next-step card into the same owned dialog editor. This connects proven
authoring surfaces only; it adds no automatic greeting mutation, second editor,
compiler, build, deployment, spawn, game/save write, runtime, or playable-NPC
claim.

Character + first greeting addendum (2026-07-17): the recommended managed-R3
NPC route now composes the existing Character Draft and greeting-line forms as
an honest two-checkpoint recipe. Step 1 publishes the project-only NPC and its
owned ScriptModule as N+1. Studio fully reopens and binds that exact root,
project, revision, and canonical head before step 2 creates one localized
DialogLine and inserts it at greeting index zero as N+2. Completion opens the
exact NPC in **Story -> Dialog & Voice** with the new line selected. If step 2
is safely cancelled or fails, the useful NPC-only N+1 Draft remains and opens
at the same empty greeting surface; there is no implicit rollback. Project/head
drift locks the single-flight recipe, while publication uncertainty requires a
reopen. The advanced Character-only Draft remains available separately.
Base-game and cross-source search starters stay one-step Draft routes instead
of silently forcing this recipe. Neither result creates a topic, choice,
condition, effect, Quest relationship, playable conversation, runtime binding,
spawn, build, deploy, game write, or save write.

Story status addendum (2026-07-16): the managed-R3 **Story** destination is now
a direct workspace rather than a card page that sends authors to Content. It
loads the exact-current project index, projects only NPC and Quest Drafts, and
combines friendly search, All/NPC/Quest filters, creation, selection, and the
existing tabbed Story Workbench. A wide and sufficiently tall host uses an
inline list/workbench split; compact or short hosts use the same workbench in a
details sheet. Same-project revisions retain only a still-existing selection
and supported tab, a newly published Draft is selected only at its exact
revision, and non-Story references open their exact Content owner. This adds no
new authority merely by projecting the workspace. It
does not start World work, and the unchanged Legacy tabs remain the usability
baseline until their managed replacements are comparably direct and capable.

Story removal addendum (2026-07-16): a selected managed NPC or Quest Draft now
has a direct **Remove Draft...** action in the same wide workbench and compact
details sheet. Before confirmation, Studio derives the exact Draft/generated-
module pair from the current index and presents any local backlink, mistyped
reference, or second ownership claim as a navigable blocker. The confirmation
uses both friendly names and explicitly says there is no undo in V1 while game
files and saves stay untouched. Native code independently accepts only the
exact current two-entity ownership closure, removes only that pair, preserves
all other entities and the complete AssetStore, and returns an unpublished
fully reopened candidate. The managed session performs the fixed-head publish
and full reopen; the workspace then refreshes and selects a deterministic
survivor or its empty state without automatic retry. This is a first safe
semantic deletion primitive, not general undo/history, project deletion, blob
garbage collection, build/deploy, or runtime behavior.

Quest continuation addendum (2026-07-17): Home now offers a recommended
**Quest + opening line** Draft recipe. It publishes Quest and ScriptModule as
N+1, rebinds to that exact managed checkpoint, and may then use the existing
transcript insertion transaction to publish the first line as N+2. Cancelling
the line form honestly retains and opens the Quest-only N+1 checkpoint. This is
two verified publications, not one atomic multi-domain transaction, and the
line is not a playable conversation, topic, NPC binding, or Quest-start link.

Quest Journey addendum (2026-07-17): a selected Quest's default Story tab is
now the responsive, objective-centered **Journey** rather than the former
technical Overview. It composes the exact current content index, validated
transition seed, and complete ordered transcript only when their project/head,
Quest/module, target/plan seal, objective order/slots, and dialog references
agree. Authors see the main Quest and objectives across Available/Start/
Success/Failure, linked dialog beside stable V4 objectives, a separate General
dialog area, and direct handoffs to the existing name/objective, context,
behavior, and full transcript/Voice editors. Clicking a Journey dialog row
selects that exact row in **Dialog & Voice**; the guided opening-line recipe
also preserves its newly created row through the Story deep link. Older
generator-v2/v3 Quests keep their effective fixed behavior but never receive
invented objective-to-dialog grouping. Loading is read-only, bounded to 256
lines, discards late/stale results, treats lost authority as reopen-required,
and exposes neither technical IDs nor any build, deployment, runtime, game, or
save authority. Quest now has four canonical tabs: **Journey**, **Dialog &
Voice**, **References**, and **Problems & Checks**. Legacy Story/Logic requests
normalize to Journey; direct fallback Workbench callers retain contextual
handoffs so the public edit capabilities remain reachable.

Content-to-Story addendum (2026-07-17): Content remains the cross-source
discovery surface. A selected exact-current Quest/NPC Draft now shows a friendly
summary and one responsive **Open in Story** continuation, with no duplicate
Workbench, edit, or source-check actions in Content. The handoff revalidates
project, revision, head, index, selection,
entity identity, and supported kind, closes compact details first, and selects
that same Draft in canonical Story **Journey**. It is single-flight and
fails with safe author-facing copy; stale/reloaded/disposed callbacks cannot
navigate or mutate. A reopen-required checkpoint keeps the canonical action
visible but disabled with an explicit recovery reason.

Problems routing addendum (2026-07-17): every target-bearing continuation now
keeps its exact stable identity and checkpoint. Quest/NPC findings open the
canonical Story **Problems & Checks** tab; other entities and project assets
use revision-and-head-bound Content navigation; DataAsset findings filter,
select, and expand the exact staged edit. Pre-mount requests are bounded, and
project/revision/same-revision-head drift resolves inertly with sanitized,
localized failure copy.

Project Work Bar addendum (2026-07-17): the managed shell now keeps one
persistent orientation surface above all nine destinations. It shows the exact
index-derived friendly project name when available (with only a folder-name
fallback while that read is pending), follows the selected primary area, and
offers **Search**, **Create**, and **Problems** in English and German. Search
switches to Content's existing `Search all` scope and focuses its existing query
field; mounting/focusing it starts no This-mod, Base-game, or Installed source
load before a nonempty submitted query. Create is a compact chooser over the
already implemented NPC Draft, recommended Quest plus opening-line, and new
dialog-line flows. The first two preserve their configured-game requirement;
all three preserve the clean exact-checkpoint/reopen/recovery gates and are
rechecked after the chooser closes. Problems only routes to the existing
Validate & Test workspace. At compact width or 200% text, Search stays direct
while Create and Problems move into an accessible overflow with their disabled
reasons. Commands are single-flight, expose busy state without animation, and
late search/focus or create continuations fail closed after checkpoint drift,
project switch, detach, or disposal. Same-project revision refreshes update the
orientation; a different project resets both workspace chrome and its bound
Search-all handoff. This is reusable project navigation over existing flows,
not new content, transaction, publication, build, deployment, game/save-write,
or runtime authority.

NPC profile addendum (2026-07-16): a selected managed NPC's **Profile** now
offers one compact **Edit name & archetype** form in both the wide Story
workspace and compact details sheet. It edits the friendly display name and a
curated verified archetype only; stable technical identity is neither shown nor
rewritten. The operation binds fresh sealed Story+NPC catalogs and the exact
current head. A name-only change preserves the owned generated module; only a
different complete three-parent provenance triggers deterministic regeneration
of that module. Publication remains in the serialized managed session. This is
an offline project edit, not a build, deploy, game/save mutation, or gameplay
qualification, and it does not widen the still-missing NPC fields.

DataAsset status addendum (updated 2026-07-16): the managed installed-package
browser supplies a normal typed fixed-leaf route alongside the retained
ExtractReceipt workflow described in the matrix. It selects only a sealed
original ordinal, binds the exact package/USMAP inspection, independently
reconstructs and compares package bytes, role-bearing sidecars, parsed UTOC
identities, chunk winners, and USMAP name/bytes, then publishes the same
revision-3 stage by exact-head CAS. No extracted receipt or caller-supplied
target/package/output authority crosses that edit route. The first reviewed
schema recognizes only exact Human/Scavenger/Wolf Footstep presets and one
`FeetTextureSize` X/Y field; the guided form preserves Z/W and labels all
values as raw asset units.

That exact-current reviewed stage can now produce one offline package triplet
through the direct expanded-stage **Build files...** action. The managed session
uses `readBasisSnapshot`; native code performs live generation replay, strict
post-pack readback/reinspection, a final exact Store source gate, protected-root
checks, recognizable-game-install rejection, and atomic absent-output
publication. Its fixed receipt and returned triplet seals are path-free. The UI
distinguishes complete publication, completed-with-cleanup-warning, and an
uncertain rename that must not be retried automatically. It performs no deploy,
runtime test, or project/game/save mutation. Structural/new DataAssets, other
reviewed schemas, a sealed Unreal bridge, and gameplay qualification remain
missing; neither gameplay effect nor units are qualified.

Usability checkpoint addendum (2026-07-15): **Validate & Test** now contains
bounded **Problems & Readiness V1**. It searches and filters exact-current
reference problems, game-configuration state, and DataAsset-registry/stage
limits. Quest/NPC targets open the exact Draft in Story **Problems & Checks**;
other entities and project assets open through checkpoint-bound Content
navigation; DataAsset findings filter, select, and expand the exact staged edit;
configuration findings open Settings. Compiler evidence remains **Not
evaluated**, general project-wide managed build remains **Blocked**, and runtime
remains **Unqualified**; the
bounded reviewed-DataAsset action is a separate offline build and does not turn
this into an aggregate Ready verdict. The view performs no compiler check,
general lowering, deployment, or gameplay proof. **Project > Close** releases
the coordinator-owned current
session without deleting project data. Add, Manage, and Resolve Voice actions
also fail closed until the current projection contains an intact
Voice-authorable `DialogLine` with its resolved same-project
`LocalizationEntry`; Add and Resolve separately require a configured game. The
guided V1 prerequisite now closes the fresh-project project-local dead end: it
creates a new managed localization and line plus an optional empty slot, or
reuses one exact currently unused managed localization under revision binding.
It is not vanilla adoption, does not trust the unsealed global localization
catalog, accesses no game or save, and creates no topic, AngelScript, buildable
output, or playable dialog. Its transaction remains build-blocked/runtime-
unqualified and reaches the fixed head only through guarded managed-session
publication and full reopen.

Recovery checkpoint addendum (2026-07-16): a managed project that enters the
visible `requiresReopen` state now offers **Try recovery** in the same shell.
The serialized session keeps its exclusive project lock, repairs only its fixed
head publication journal, and fully reopens the resulting Store generation
with asset verification. Recovery succeeds only when format/schema, project
identity, and game target remain exact and the result is either the previously
opened revision or its single publication successor. A same-revision fork,
larger drift, foreign identity, target change, malformed journal, or failed
reopen leaves the old in-memory checkpoint untouched and every authoring action
locked. The normal UI reports only whether the project was safely reopened; it
does not expose journal, head, CAS, or repair-outcome terminology. This path
does not release/reacquire the lock, access the game or a save, undo content,
or provide general history. Closing and opening the project remains the safe
fallback when bounded recovery cannot prove the result.

Voice preview checkpoint addendum (2026-07-17): **Manage Voice takes** now
previews one exact-current managed CAS take directly inside Studio. The UI keeps
technical identities and paths hidden and offers Play/Pause, Replay, seek,
progress, and Stop. Native binding covers the complete line -> localization ->
locale slot -> take -> asset chain and fully verifies only the selected Ogg. On
Windows, native registration first pins the managed Store read-only and the
system-temporary parent, atomically creates and retains one fresh
non-overlapping preview root, rejects identity drift, and returns an opaque
cleanup token.
Unsupported desktop platforms fail closed. The FFI can then create only one
fixed `preview.ogg` through that retained capability. Dart rehashes the file and
unloads the audio decoder before native,
token-bound, non-recursive release. A failed release retains the token for an
explicit retry; abrupt Studio termination or an exceptional failed registration
may leave an isolated temporary root behind until manual or
operating-system-policy cleanup, with no unsafe startup sweep. Stale graph
leaves refresh safely; Store/head/receipt uncertainty requires reopen. Preview
writes no project, Store, game, save, build, deployment, or runtime state and
does not qualify in-game audio. Recording, editing/transcoding, complete
coverage dashboards, and an isolated audible game profile remain separate work.

Voice media-QA addendum (2026-07-17): **Manage Voice takes** now offers a
separate on-demand **Check media** action for one exact-current managed take.
The pathless native/FFI route double-reopens the complete line/localization/
slot/take/asset binding, rereads and rechecks the sealed CAS object, and reports
rational sample-frame duration plus an explicit assurance level. Vorbis uses a
complete packet-by-packet PCM decode with validated initial origin and EOS trim;
the Voice profile accepts mono/stereo only. Opus is labelled structure-and-
timing-only and uses its normative 48 kHz clock with origin, pre-skip, and EOS
trim. Dart/session/controller boundaries revalidate the exact checkpoint, and
the dialog caches only that project/line/locale/take revision. Reloads,
mutations, and context drift discard the result; stale state can reload while
uncertain authority requires reopen. The UI shows no path, digest, or entity ID
and explicitly grants no loudness, clipping, subtitle-fit, audibility, build,
deployment, runtime, project-write, game-write, or save-write authority.

Voice production-queue addendum (2026-07-17): **Localization & Voice** now
defaults to a responsive **Work list** with a **Project texts** switch. V1
projects only missing authoring-locale membership and intact existing Voice
slots; a line without a slot is not labelled as a missing recording. Slot
precedence is exact: zero takes → add recording; no Approved take → review and
approve; no valid Approved selection → select or repair; unresolved/ambiguous
target → resolve; otherwise → production decisions complete. Unreviewed
alternatives do not regress a complete selected/targeted slot. Every action
reuses the existing exact editor or line/locale modal, while completion opens
Validate & Test. It is explicitly not Ready, build, deployment, audibility, or
runtime evidence. The list retains at most 500 rows, reports omitted work,
keeps known language work visible if the Voice catalog is unavailable, and
fails closed on catalog mismatch, root/project/revision/head drift, stale
callbacks, or `requiresReopen`. Broader coverage dashboards, recursive/partial
or multi-locale production, CSV/XLIFF, and batch/team review queues remain
missing. No World or level work is included.

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
current project. Normal Legacy startup now shows a localized managed-project
entry banner with direct Create/Open actions, labels the unchanged tabs as
Legacy compatibility tools, and reuses the existing guarded transition/adoption
flows; the canonical nine-destination shell appears after managed adoption.
Managed Studio Shell v1 exposes the canonical localized,
responsive Home, Content, Story, World, Localization & Voice, Validate & Test,
Build & Release, History, and Settings & Expert destinations. DataAssets is a Content
secondary view. Pages lazy-mount and retain primary selection, each section's
secondary route, and mounted state across same-project revision refreshes; a
different project identity resets to Home. Home reads the exact-current
`Revision3ContentIndex` and leads with five task routes—**Story**, **Dialog &
Voice**, **Problems**, **Content**, and **Build & Release**—before compact counts
and readiness. It no longer hosts duplicate low-level authoring, import, build,
or export actions: Story owns creation, domain workspaces own their operations,
and Project owns export. Durable root/ID/revision/head identity remains
available under collapsed technical details, and managed Save verifies the
exact head. The persistent Project Work Bar now keeps the friendly project name
and selected area visible above those lazy pages. Its Search/Create/Problems
continuations route into the existing Content, Story/Localization, and Validate
owners; they do not duplicate their state or permissions. Creation
uses the same absent-head publication, full reopen, recovery, and single-owner
adoption rules rather than introducing another project format.
Dirty transitions, failed candidate preservation, `requiresReopen`, and terminal
cleanup diagnostics are handled outside the workspace at the shell boundary.
Legacy editors, Build/Deploy, Save As, and Story actions cannot act on hidden
compatibility state while R3 is current. Bounded Quest/NPC Draft, Voice import/
selection/target, exact-head/reference verification, the Voice-only offline
build, Settings, verified DataAsset actions, and the bounded `Search all` view
now share that R3 owner. World, runtime test, full managed build/deploy, and
Expert tools remain visible but unavailable. This is the complete primary shell
with a first cross-source discovery view, not yet a unified cross-source
semantic content-authoring flow. The workspace grants no game/save writes,
deployment, general managed build, or runtime qualification.

The remaining managed-authoring limits are:

- the bounded format-1 encoder/reader is intentionally in-memory and capped; it
  is not the streaming, sharded, content-addressed storage needed by large mods;
- provider replacement is still not one rollback-capable all-domain
  transaction, even though all represented keyed deployment targets now receive
  one duplicate-validation pass before mutation or publication;
- format-1 embedded source paths are extraction derivatives rather than
  immutable AssetStore refs, so they remain unsuitable as durable entity state;
- autosave recovery, named checkpoints, semantic history diffs, revision
  migration orchestration, and general managed AssetStore blob-ownership tools
  are not implemented. The direct History timeline instead retains a sealed,
  newest-first window of at most 256 exact project checkpoints and restores an
  older member append-only as a fresh revision; it is not an unlimited archive.

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
History
Settings / Expert mode
```

Current implementation note: the managed shell implements all nine primary
destinations above. Content now hosts honest `This mod`, `Base game`,
`Installed`, and `Search all` scopes without a fake `Dependencies` scope, while
DataAssets remains a separate verified-edits secondary view. The scope host is
lazy and project-identity aware. Base game is a bounded NPC/Quest starting-point
catalog, not a complete vanilla catalog; Installed is exact DataAsset discovery
metadata, not package or build authority. `Search all` is an explicit-query,
in-memory v1 over the three projections, with at most 100 retained rows per
source and an independent state/error boundary per source. It offers exact
same-source open/create-Draft/inspect actions but no atomic combined snapshot,
dependencies, references, or backlinks. Indexed/virtualized large-scale search,
the full command palette, undo/redo and build/test controls, and broader
workspace chrome remain missing. The landed Work Bar is deliberately the
smaller orientation plus Search/Create/Problems subset.

New-project creation now offers **Empty**, **NPC Draft**, and **Quest Draft**.
All three reuse absent-head atomic empty-project creation. NPC/Quest then open
the existing guided wizard as a second exact-head transaction. Cancel before
publication leaves revision 0; an uncertain result requires reopening and is
never described as empty. Multi-domain templates still require a native compound
prepare/publication transaction.

The primary **Story** destination now follows the productive-workspace rule
directly: it searches and filters current NPC/Quest Drafts, creates another
bounded Draft, and opens the selected exact entity in the existing Workbench in
place. Content remains the cross-source discovery owner, not a mandatory detour
for Story editing. Both views project the same managed graph and route every
mutation through the same session; neither owns a private editable copy.

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
| **Export** | Portable snapshot or review copy | Managed R3 V1 writes a deterministic `.goremod` from one exact immutable snapshot without changing the current working path, project head, game, or saves. Its importer is not implemented, so Studio calls it a **project copy**, explicitly says it is not a restorable backup or playable mod, and tells the author to keep the original working directory. A future separately reviewed importer may create a new managed working directory; it must never edit ZIP members in place. See [Managed project snapshot export](managed-project-export.md). |
| **Build** | Produce an inspectable mod artifact | Derived from an immutable project revision and named build root/profile; it does not deploy and cannot become editable source state |
| **Test deployment** | Install one build into an isolated test profile | Receipt-owned, game-closed preflight, explicit disposable save choice, bounded logs/observations, and verified cleanup |
| **Release** | Publish a reproducible user-facing package | References an immutable closed-world validated revision/build plus compatibility, dependency, license, changelog, hashes, and provenance |

The bounded line-centric Voice workflow extends the managed revision-3 session
instead of adding another format-1 list or parallel project state. Exact-head
transactions now import takes, change one retained take's workflow status,
select or clear an existing Approved candidate, remove one take from one exact
line/language, and resolve generation-bound existing archive targets through
guarded session publication, repair, and full published reopen. Removal clears
the selected target atomically, retains shared takes, and preserves the entire
AssetStore even after final project use. Status changes preserve the slot,
selection, media asset, and all unrelated content; author approval remains
distinct from audio or runtime qualification. The landed Work list now derives
one truthful next action for absent authoring locales and existing Voice slots,
then reuses those same exact transactions. It introduces no parallel project
state and “production decisions complete” is not build or runtime readiness.
Separately, the exact-current
offline builder reads verified selected Store bytes, stages and completely
reopens/seals a deterministic format-3 Voice tree, then atomically promotes it
with no-replace semantics without publishing the project head. Production
completion still needs broader history, explicit ambiguity choice,
recording/transcode, full coverage and batch/team review tooling, managed
deploy/undeploy, an isolated test profile, audible runtime qualification, and a
separately proven new-member path. The landed queue and offline foundation must
not be presented as that complete workflow.

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

Today the separate bounded NPC and Quest Draft wizards exist. Home also composes
the Quest wizard and existing transcript insertion as an honest two-checkpoint
**Quest + opening line** Draft recipe, including a Quest-only continuation when
the second form is cancelled. The broader combined new-NPC/new-quest playable
slice remains a **planned Draft workflow and not a supported production
promise**. The important product decision is that authors can eventually
scaffold and organize the whole intent without hand-authoring disconnected
backend rows. NPC spawning/identity and quest
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
Current V1 opens Quest/NPC Drafts in Story **Problems & Checks**, other entities
and project assets through checkpoint-bound Content navigation, and DataAsset
findings at the exact staged edit. Exact property-level focus remains an
end-state requirement. Compiler and lowering evidence is expandable. A quick
fix is a previewed transaction, never an eager mutation.

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

**Immediate checkpoint order (updated 2026-07-17): usability and completion take
precedence over new breadth.** Problems & Readiness V1, honest action gates,
compact project chrome, Project Close, and the guided project-local dialog-line
create/exact-managed-reuse V1 form the landed baseline. Project Work Bar V1 now
adds persistent project/area orientation plus direct Search/Create/Problems
continuations into those existing owners, including exact Search-all focus with
no pre-query source loads and responsive accessible overflow. It adds no new
authority. The recommended
two-checkpoint **Quest + opening line** recipe now removes the creation-to-writing
dead end while preserving an honest Quest-only result on cancellation; it does
not create a playable conversation. The recommended
two-checkpoint **Character + first greeting** recipe now applies the same honest
continuation to NPC authoring: the NPC-only checkpoint survives a stopped second
step, while successful completion inserts one localized greeting and selects it
in the existing Story surface. It grants no runtime, spawn, build, game, or save
authority. Direct project-text editing and the direct
NPC/Quest Story workspace now replace two former card/modal detours without
changing their authority boundaries. The first
reviewed non-World offline build is now integrated for one selected reviewed
managed DataAsset stage: it derives from the exact-current project, creates only
a new receipt-owned output, then reopens and re-inspects that output. The first
managed project-copy export is also integrated as a deterministic exact-snapshot
review artifact with no importer or restore claim. Exact-current managed-CAS
Voice take preview is integrated as a read-only in-app capability. The first
Voice production Work list is also integrated: it defaults Localization & Voice
to evidence-backed missing-language and existing-slot next steps while keeping
Project texts one switch away. Next, complete the broader
line/localization/Voice production journey, safe project fundamentals such as
semantic/project deletion, undo/history, and broader recovery, and deeper
NPC/Quest semantics before general managed build/release and qualified test
paths. The bounded existing-NPC friendly-name/full-archetype edit is now one
landed part of that semantic depth; authenticated History/Undo is the next
project-foundation slice. Broad World or level authoring does not
start while these primary workflows still strand authors or lack an honest
end-to-end result. Until that gate is met, World remains a visible, truthful
unavailable surface. Even after the gate is met, implementation pauses at a
clean pushed checkpoint and requires explicit user approval before World work.

1. **Finish the managed phase-one substrate:** the owned R3 working directory,
   serialized session, strict bounded open/import, Ogg AssetStore I/O, exact
   revision/head publication, repair, full reopen, first Voice transaction, and
   deterministic exact-snapshot export are integrated. Complete general
   recovery/history, cross-domain undo, and production lowering without
   creating a parallel project state.
2. **Complete the first non-technical Voice slice:** extend the landed Work
   list, import, retained-take review status, Approved-take selection/clear,
   generation-bound target resolution, preview, and sealed existing-member
   offline build with explicit ambiguity choice, recording/transcode, complete
   coverage and broader review queues, history, managed deploy/undeploy, and an
   isolated audible test profile without archive terminology. Per-item
   production decisions and offline bundle construction do not satisfy the
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
8. **Content breadth, only after the usability gate above:** qualify new items,
   routines, spawn groups, and wider
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
