# AngelScript quest authoring

GORE can compile new `UQuest` subclasses, carry their generated defaults and
new symbols in an additive mini-cache, and compose them into the game's script
cache. Automatic discovery of new quest classes is narrowly runtime-proven on
the current Gothic 1 Remake generation. The managed revision-3 project can now
persist and edit a bounded semantic lifecycle plan. A separate G1R 1.0.3
qualifier gives compiler/cache-pipeline evidence for the four external-trigger
fields and predicate-hook shapes, all three handler shapes, `bSucceedParent`,
typed cross-node getters, and guarded `StartQuest`/`SucceedQuest`/`FailQuest`
calls used by the lowerer. It did not compile one exact renderer-produced
fixture covering every state-test expression. Generated transition behavior,
dialog selection, and persistence remain runtime-unqualified and production
build stays blocked.

## Proven discovery boundary

The standalone Asghan probe added two classes in a new AngelScript module:

- `UQuest_GORE_PROBE_ASGHAN_MINI`
- `UQuest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE`

After an existing Old Mine save was loaded, the native crash report's quest
table contained both runtime instances as `EQuestState::Available`:

```text
"Instance_Quest_GORE_PROBE_ASGHAN_MINI": "EQuestState::Available"
"Instance_Quest_GORE_PROBE_ASGHAN_MINI_OBJ_DONE": "EQuestState::Available"
```

The retained report is
`C:\Users\Daniel\AppData\Local\G1R\Saved\Crashes\UECC-Windows-5605EA0F42879E207C3A7F89F291944F_0000\gothic_crash_info.log`,
lines 923–924, SHA-256
`1EAC2D1D12995BFF688E258D4DA1B4653BF6C70562DA47469DC6B6388D7B10B4`.
The candidate source and offline artifacts are retained under
`work/probe/asghan-miniquest/public-v1/`.

This proves that the native quest subsystem discovered and instantiated the two
added subclasses during world/save load. It does not prove:

- availability/start/success/failure predicate ordering;
- dialog-driven `StartQuest`, `SucceedQuest`, or `FailQuest` behavior;
- journal unlocks, rewards, knowledge, or `ActedTopics` effects;
- save/reload persistence or safe behavior after uninstalling a quest mod;
- compatibility with another game executable or future hotfix.

The later crash was caused by a retired direct conversation-ability activation
bypass, after both quest instances already existed. It is not evidence that the
quest discovery path failed. That bypass must not be reused.

## Existing quest edits

Ordinary methods in an existing quest module can use the strict
`compile-module --op edit` path. Generated `__InitDefaults` records remain
carried from the selected base module and are not generally source-editable.
Any edit must preserve the exact module/class/layout and metadata constraints
documented by the AngelScript compiler workflow.

Some existing primitive defaults have a narrower offline patch path. One
current sealed site is:

```text
module: Story.G1R.Quest.Quest_BanditsCamp_BANDITSTRUST
class:  UQuest_BanditsCamp_BANDITSTRUST
owner:  UQuest
field:  bExternalStartTrigger
type:   bool
value:  true
raw:    01000000
```

`gore as default-sites` can rediscover the exact selector and
`gore as patch-default` can perform a copy-on-write compare-and-swap edit on a
cache copy. This does not generalize to parent references, localized text,
arrays, journal structures, or arbitrary generated defaults.

## New quest authoring model

The game represents a quest and its objectives as `UG1RQuest` subclasses.
Generated defaults carry parent links, `EQuestKind`, external-transition flags,
involved characters, quest giver, localization identities, journal-document
links, and `bSucceedParent`. Ordinary methods implement predicates and
transition effects. A practical authoring graph therefore needs at least:

- one root quest and typed objective children;
- explicit initial, running, success, and failure ownership;
- external versus predicate-driven transition choice per edge;
- localization and journal-segment references;
- idempotent reward/effect handlers;
- typed dialog, NPC, item, and world-condition references.

The retained lifecycle compiler qualifier resolves and disassembles 23 authored
functions offline. The separate Asghan discovery proof upgrades new quest
**class discovery** from hypothesis to a supported narrow mechanism. Neither
result proves lifecycle polling order, handler order, transition gameplay,
dialog selection, save/reload, or clean uninstall.

## Draft generators and semantic V4

`gore-authoring` retains the byte-frozen `DraftQuestSkeletonV1` for the smallest
useful Quest Draft: exactly one `UG1RQuest` root and one objective. The additive
`DraftQuestSkeletonV2` keeps that first objective's technical identities, then
emits two through eight ordered objective classes with deterministic
class/getter names. It regenerates the complete multi-objective source so only
the final generated objective has `bSucceedParent = true`; the separate V1
single-objective output remains byte-frozen. This represents author order and
completion shape; it does not claim that runtime transitions enforce the order.

Project schema revision remains 3. Existing one-objective Quest entities use
project generator version 2 and omit both `additional_objective_titles` and
`transition_plan`; existing multi-objective entities use generator version 3
and also omit `transition_plan`. Merely reading or deriving their effective
behavior plan performs no migration: an otherwise unchanged Quest keeps its
canonical project JSON and generated source byte-for-byte, and no
`transition_plan` appears until an explicit behavior edit. Separate outline or
context edits still regenerate the fields they own. Generator version 4
requires a closed `transition_plan`. Its deterministic legacy seed reproduces
the complete frozen version-2 and version-3 AngelScript source and source hash
byte-for-byte, including the first objective's original class/getter
identities.

V4 separates stable technical objective slots from presentation order. Slot 1
is the frozen legacy identity, active slots are unique non-zero ascending
ordinals, `objective_order` is their full permutation, and
`next_slot_ordinal` is strictly greater than every active slot and never
regresses during a transition-only edit. Reordering presentation therefore
does not rename a class or getter.

All three generators are bounded and offline-only. V1/V2 deliberately contain
no authored predicate, effect, or failure path. V4 can lower the bounded
lifecycle plan described below, but still contains no dialog selection,
journal operation, reward, item mutation, arbitrary AngelScript, filesystem
write, compiler invocation, game launch, or save operation. Objective titles
are canonical, byte-bounded, and unique case-insensitively. Every generated
class and getter is checked against the sealed collision catalog.

Generation requires the target game generation plus catalog-layer, generation,
and source-seal anchors for the giver, parent quest, and collision inventory.
The giver keeps its catalog selector separate from its runtime unique name; only
the latter reaches generated source. Generated modules, paths, classes, helper,
and getter symbols are pairwise collision-checked and checked against the
sealed catalog using portable case-insensitive identity rules.

The retained golden fixture is 2,008 UTF-8/LF bytes with source SHA-256
`eb38bf814685485977113cf67a679d4b4cb309a2dbcd229fae3a6d57f2a4ae82`.
Its canonical input fingerprint is
`5987a4b5147fb76f34af3cf0f926f0c7de2450d4e370c1aee3d88bcf8121de93`.
These hashes have different jobs: the source hash binds emitted bytes, while
the tagged, length-prefixed input fingerprint also binds provenance, collision
inventories, fixed generator semantics, and inputs that may not appear in the
source text.

The generators always report `OfflineDraft`, `RuntimeUnqualified`, and
`TransitionsRuntimeUnqualified`. Caller-supplied seals and successful offline
compilation cannot upgrade those statuses. Runtime discovery evidence belongs
to the versioned capability registry and must match the exact generated
operation independently.

### Native Quest-intent transaction

The closed Story/inventory authoring registry currently accepts exactly two
reviewed Steam generation triples: the retained V1 seal set and Steam build
`24169431`. Executable, deployment-aware pristine Shipping cache, and
`Binds.Cache` must match one complete registered row; nearby or cross-paired
seals fail closed. Supporting these two inputs does not generalize compiler or
runtime qualification to future or non-Steam builds.

The native command `authoring_project_story_quest_draft_insert_v1` closes the
provenance boundary for the Studio-facing MVP. Its payload contains exactly
`project_json`, `intent_json`, `profile`, and `game_root`. The bounded
`intent_json` contains only transaction/CAS IDs, display and Quest text fields,
module/technical identities, and `parent_catalog_id`/`giver_catalog_id`. It
cannot contain source seals, an inventory artifact, or collision arrays.

Native code selects the deployment-aware pristine Shipping cache, derives the
fixed executable and Binds paths, rebuilds the closed Story catalog and
base-game collision inventory, binds both to the exact canonical revision-2
project, resolves the two catalog IDs, and invokes the existing atomic Story
Draft transaction. Responses bind the four untouched raw inputs and retain the
fixed claims `base_game_and_exact_project_only`, `runtime_unqualified`,
`blocked`, and `not_supported`. Generation inputs and the pristine selection
are revalidated around the transaction and response serialization. No compile,
write, deploy, publish, or game launch occurs.

On the current pinned generation the complete combined collision set makes a
single Quest entity about 3.52 MiB, above the working store's unchanged 1 MiB
per-entity limit. The command therefore returns the explicit bounded error
`AUTHORING_STORY_QUEST_PROJECT_LIMIT`; it does not raise the limit, omit
collision evidence, or return a partial candidate. A more compact committed
collision-capability representation is required before this exact real-game
Quest can be inserted through the managed store.

### Revision-3 offline Draft transaction

Schema revision 3 replaces the oversized embedded collision inventory with a
content-addressed `QuestCollisionArtifactRef`. The historical
`apply_revision3_quest_draft_transaction_v2` established the first-Quest path
with caller-verified collision input; it is not the transaction used by the
current native FFI route.

Existing single-objective revision-3 projects retain project-level Quest
generator version 2 and omit `additional_objective_titles`, so their canonical
JSON and generated source remain byte-identical. New ordered multi-objective
Drafts use project-level Quest generator version 3, persist that optional list,
and regenerate the complete ordered source. The request parser validates the
objective-list shape and the transaction derives the generator version from
whether that list is empty. Persisted version/list mismatches fail closed during
project validation, regeneration, build inspection, Store persistence, and Dart
candidate validation.

The current filesystem-free
`apply_revision3_quest_draft_transaction_v3` consumes a fresh prepared collision
capability bound to the Base Game, trusted catalog, exact current project and
head, non-Quest basis, prior-Quest evidence, and all collision domains. Its two
JSON transports must be exact and canonical. Parent and giver are resolved only
from bounded catalog IDs; generated identities are derived inside the
transaction. It inserts one more Quest Draft plus deterministic ScriptModule,
increments the revision once, and returns an externally opaque result only after
canonical reopen equality. Every rejection leaves the input project unchanged.

The outcome remains permanently build-blocked and runtime-unqualified, grants
no artifact authority, and requires a fresh capability for source inspection.
The pure transaction performs no filesystem write, compile, package, deploy,
launch, or fixed-head publication. The strict native prepare-only FFI route
below orchestrates this v3 transaction with Store persistence. A strict Dart
wrapper and managed-session transaction consume that candidate and publish it
through the guarded fixed-head lane; the managed content library and bounded
Quest wizard expose the resulting semantic Draft.

### Native revision-3 prepare-only FFI route

`authoring_store_prepare_revision3_quest_draft_v3` closes the native boundary
around the repeated-Quest transaction and Store persistence path. Its exact raw
request has the normal `command`/`payload` envelope; that payload accepts exactly
`root`, `game_root`, `current_project_json`, and `quest_request_json`. Unknown,
duplicate, wrong-typed, or oversized fields are rejected, and both embedded
JSON transports must be exact and canonical.

Native code fully opens the exact published R3 project, rebuilds the trusted
Story catalog and base-game collision inventory from the deployment-aware
pristine Shipping cache and current executable/Binds inputs, and binds a fresh
linear collision capability. It then runs the committed C1 transaction, rechecks
the game sources, imports the artifact through the C2 no-clobber Store path, and
prepares and fully reopens the immutable candidate checkpoint. Game inputs are
checked again before a response is released. The working-store root and semantic
game installation must remain disjoint in either direction, including at the
write boundary, and the basis revision must be safe for the signed 64-bit Studio
wire after its single increment.

The route is deliberately prepare-only. It returns basis/candidate heads,
canonical candidate project identity, inserted Quest/module IDs, deduplication,
and only these readiness/authority claims:

- `build_status: blocked`;
- `runtime_status: runtime_unqualified`;
- `artifact_authority: not_granted`;
- `source_inspection: fresh_capability_required`;
- `publication_status: not_supported`.

It never replaces `gore-project.json`. A late source race can leave only verified
immutable CAS orphans and returns no stale candidate response. The strict Dart
DTO/wrapper and managed-session semantic Quest operation now validate the exact
basis, complete Quest/ScriptModule/artifact candidate closure, status contract,
and every signed-wire number before publishing by exact fixed-head byte CAS.
They fully reopen the published checkpoint and poison the session if publication
becomes uncertain.

The managed revision-3 Home surface exposes a bounded friendly Quest wizard
over this operation. It accepts a name, description, one through eight ordered
objectives, Quest family, and giver; objectives can be added, removed, and moved
without entering entity IDs, namespaces, symbols, or paths. Family/giver
choices come from a freshly rebuilt Story catalog when the dialog opens and are
refreshed again immediately before the native transaction. Technical identities
are derived deterministically from the managed project ID, current revision,
and authored intent. Separately, the coordinator rejects a different root or
an advanced canonical head as a stale wizard that must be closed and reopened.
A successful publication
refreshes the visible project revision and exact-current content library, where
every objective participates in search.

This is an ordered objective-outline Draft workflow, not a state graph,
conditions/effects editor, journal/reward workflow, compiler, or runtime
qualification. The generator's plain-text subset is validated in the form, and
the UI continues to show `blocked` and `runtime_unqualified` without compile,
deploy, game-install, or save-file claims.

### Managed revision-3 Studio transaction boundary

`ManagedRevision3AuthoringProjectSession` builds the native request from its own
exact opened working-store root and checkpoint inside the serialized session
lane. Callers provide only friendly Quest intent plus the game root; they cannot
substitute a project basis, candidate head, collision evidence, generated
module, or artifact seal.
The Dart response parser is closed and duplicate-safe, canonicalizes every
embedded transport, recursively rejects numbers outside the signed 64-bit wire,
and checks the complete deterministic Quest/module relationship rather than
trusting a successful native status.

After a full candidate reopen, the session publishes only if the fixed-head
bytes still equal the basis bytes, records the normal crash-repair journal, and
fully reopens the published result. The result remains `blocked`,
`runtime_unqualified`, `not_granted`, `fresh_capability_required`, and
`not_supported`. No compile, pack, deploy, game write, or runtime claim is added.

### Mod Studio Story Workbench V1

Selecting an exact-current managed `QuestDraft` in **Content → This mod** now
opens one responsive workbench. Entity areas at least 900 logical pixels wide
and 430 logical pixels high keep the entity list and detail side by side;
falling below either bound opens the same tabbed detail in a scrollable
78%-height sheet. The Quest tabs are **Overview**, **Story**, **Logic**,
**Dialog & Voice**,
**References**, and **Problems & Checks**.

The workbench does not introduce a parallel editor model. **Overview** invokes
the existing atomic **Edit name & objectives** operation, **Story** invokes
**Edit description & connections**, **Logic** invokes **Edit states &
transitions**, and **Problems & Checks** can open the existing exact-current
source/compiler inspection. **Dialog & Voice** remains visibly unavailable
because the current Quest Draft schema does not model those relationships.
References shows only outgoing entity/asset links and derived incoming links in
the exact current index. Its unresolved count is reference status, not a
project-wide validator or evidence of build/runtime readiness.

The workbench keeps **Draft only**, **Build blocked**, and **Runtime not
verified** separate. Entity/tab selection survives an exact revision refresh of
the same project only while the selected Quest still exists; a project switch
clears it, and a removed Quest cannot retain stale tab state. This UI adds no
new mutation beyond the reused atomic callbacks and grants no game/save, build,
deploy, runtime, or publication authority.

### Managed revision-3 existing-Quest outline edit V1

The managed R3 Story Workbench exposes **Edit name & objectives** from the
**Overview** tab for one selected, exact-current `QuestDraft`. This
count-preserving editor may change
only the Quest's name in the project library, its player-facing title, and the
text/order of its existing one through eight objectives. In this outline
operation the objective count, description, Quest family/parent, giver,
technical identities, stable Quest and ScriptModule IDs, ownership, provenance,
and the retained `QuestCollisionArtifactRef` remain byte-for-byte unchanged;
the separate context action below owns description and connection changes.
The project, Quest entity, and owned ScriptModule revisions each advance exactly
once only when at least one editable value changes.

The pure
`apply_revision3_quest_outline_edit_transaction_v1` first proves the existing
Quest/owned-module closure by deterministic regeneration, preserves its
technical module identity, regenerates source from the edited outline, and
requires exact canonical candidate reopen. The strict native prepare-only
`authoring_store_prepare_revision3_quest_outline_edit_v1` payload contains
exactly `root`, `current_project_json`, and
`quest_outline_request_json`; there is no `game_root`. Native code fully opens
and binds the fixed head/project/target/Quest, performs the exact-current Quest
source/asset preflight without accepting client collision authority, prepares
immutable Store objects, fully reopens the candidate, and rechecks the fixed
head before returning. It never replaces `gore-project.json`.

The Studio validates the complete candidate and publishes it only through the
managed session's guarded exact-head byte-CAS, repair journal, and full
published reopen. A stale project, changed Quest revision, no-op, unsafe Store
alias/link, preparation failure, or late head race returns no publishable
success. The operation remains `build_status: blocked`,
`runtime_status: runtime_unqualified`, and
`publication_status: not_supported` at the native boundary. It performs no
compile, package, deployment, game-installation write, game launch, or save-file
read/write. Outline V1 deliberately rejects a generator-V4 Quest rather than
reordering titles while losing stable slot semantics.

### Managed revision-3 stable-slot outline edit V2

Generator-V4 Quests use the adjacent stable-slot-aware Outline V2 transaction.
The same **Name & objectives** dialog loads the exact-current transition seed,
keeps every active objective slot exactly once, and moves each slot together
with its edited title. Reordering presentation therefore preserves every
condition/effect reference and the complete transition graph. The editor still
cannot add/remove objectives or edit technical IDs, parent, giver, provenance,
or runtime behavior.

`apply_revision3_quest_outline_edit_transaction_v2` binds the project, Quest,
owned ScriptModule, active slot set, `next_slot_ordinal`, and a domain-separated
transition-plan seal. The prepare-only
`authoring_store_prepare_revision3_quest_outline_edit_v2` repeats the full Store
and exact-current closure checks, regenerates deterministically, fully reopens
the immutable candidate, and verifies that only the display name, Quest title,
objective titles/order, generated module bytes/seals, and the three revisions
changed. Conditions, effects, active slots, next ordinal, and all unrelated
entities must remain exact. Native code still returns only `blocked`,
`runtime_unqualified`, `not_supported`, and never publishes the fixed head;
the managed session uses the same repair-journal and exact-head byte-CAS lane as
V1. No game root, compiler, build, deploy, game launch, or save access is part
of V2.

### Managed revision-3 existing-Quest context edit V1

The same selected exact-current `QuestDraft` has a separate **Edit description
& connections** action in the Workbench's **Story** tab. This transaction may
change only the player-facing description, Quest family/parent, and giver. It
preserves the library name, title, objective count/text/order, stable Quest and
ScriptModule IDs, technical module identity, ownership, origin/provenance, and
the retained `QuestCollisionArtifactRef`. The project, Quest, and owned module
revisions advance exactly once only when at least one of the three editable
values changes.

Opening the context editor reads the exact description from the managed
project and rebuilds a fresh Story catalog from the configured game
installation. The Quest's current parent runtime class and giver runtime name
must each resolve to exactly one current catalog choice. Missing or ambiguous
current mappings make the operation unavailable; V1 does not guess a default,
silently replace either connection, or act as a hotfix migration tool. The UI
shows friendly family/giver labels while keeping catalog and runtime identities
out of the normal authoring surface.

Immediately before Save, Studio rebuilds the catalog again. It revalidates the
exact current mappings and selected choices and requires the same catalog seal
the author reviewed. The canonical request carries that seal as
`expected_story_catalog_seal`; any changed seal, missing choice, ambiguous
mapping, project/head drift, or no-op fails closed and publishes nothing. After
a catalog change the author must review the fresh choices again rather than
having a replacement selected automatically.

The pure `apply_revision3_quest_context_edit_transaction_v1` consumes fresh
prepared collision authority, proves the existing Quest/module closure by
deterministic regeneration, preserves its technical identity, applies only the
three context fields, and requires canonical candidate reopen. The strict
prepare-only `authoring_store_prepare_revision3_quest_context_edit_v1` payload
contains exactly `root`, `game_root`, `current_project_json`, and
`quest_context_request_json`. Native code fully opens the exact fixed head,
rebuilds pristine game/catalog/collision evidence, binds the requested catalog
seal and Quest revision, revalidates game inputs around preparation, and fully
reopens only an immutable unpublished candidate. It never replaces the fixed
head; the managed session alone may publish by guarded exact-head byte CAS,
repair journal, and full published reopen.

The operation remains `build_status: blocked`,
`runtime_status: runtime_unqualified`, and
`publication_status: not_supported`. Native preparation may write only immutable
candidate objects in the managed working Store; only guarded managed-session
publication changes the current project head. It performs no game-installation,
save-file, deployment, package, launch, or runtime mutation. Context V1 accepts
a generator-V4 Quest and preserves its generator version and complete retained
transition plan exactly. It never silently downgrades the Quest.

### Managed revision-3 existing-Quest states and transitions V1

The selected exact-current `QuestDraft` exposes **Edit states & transitions**
in the Workbench's **Logic** tab. The dialog shows one behavior table with the
main Quest and each existing objective as rows and **Available**, **Start**,
**Success**, and **Failure** as columns. Authors can apply a sequential template
or open one cell to:

- allow the game to trigger that lifecycle edge directly;
- add optional typed conditions over `Available`, `Running`, `Started`,
  `Succeeded`, `Failed`, or `Completed`, including explicit negation;
- add follow-up `Start`, `Succeed`, or `Fail` actions targeting another Quest
  part on start/success/failure edges; and
- mark an objective success as also completing its parent Quest.

External triggering and an automatic predicate are independent and may coexist.
Availability has no handler and therefore cannot carry follow-up actions.
Success and failure cells are optional and may be removed; availability and
start are required for every node. The editor does not add/remove objectives,
edit Quest text or connections, expose IDs/seals/source, accept raw
AngelScript, or author dialog, journal, rewards, items, or arbitrary gameplay
effects. Its visible boundary says that Save creates only an offline project
checkpoint and does not build, run, deploy, or qualify the Quest in game.

The same dialog also offers **Preview project logic**. This is a pure,
resettable offline model over the currently visible, including unsaved, plan.
It uses five conservative, mutually exclusive phases: `Unavailable`,
`Available`, `Running`, `Succeeded`, and `Failed`. The displayed `Started` and
`Completed` observations are derived from those phases. The Rust renderer emits
the corresponding engine state calls independently; this offline truth table
does not represent or prove simultaneous engine-state combinations outside the
five phases. A predicate conjunction with no satisfying exclusive phase is
counted and visibly marked as outside the preview model. It always evaluates
false in the preview, while its independent calls remain in generated source;
that warning is not a runtime-validity judgment.

The preview exposes buttons only for edges whose `external_allowed` flag is
true, evaluates representable typed predicates to a bounded deterministic fixed
point, and records the resulting model condition/action timeline. Follow-up
Start is guarded by `!HasBeenStarted()`; Succeed and Fail are guarded by
`IsRunning()`, matching the calls emitted by the Rust renderer. Objective parent
completion is modeled as the validated implicit root-success action. A repeated
or over-budget cascade restores the pre-action lifecycle state. If Reset is
refused, it also restores the exact prior retained timeline, sequence, and trim
marker; a successful Reset returns every node to the initial offline state
before re-evaluating automatic conditions. The preview never opens native code
or writes a project, game installation, deployment, process, or save. Its
visible boundary explicitly says this is project-logic feedback, not proof of
engine predicate polling/handler order or runtime behavior.

The closed plan supports one through eight objectives. Predicates are bounded
disjunctive normal form: one through eight alternatives, each containing one
through eight required atoms. Every effect list is bounded to eight entries.
The validator requires canonical ordering, active-node references, an external
or predicate driver for every retained transition, availability and start for
every node, and success or failure for every objective. It rejects direct and
lifecycle-state contradictions, duplicate terminal effects and conflicting
success/failure effects on the same target within one handler, self-target
effects, same-node automatic success/failure predicates that are not provably
disjoint, misplaced `succeeds_parent`, and same-kind effect cycles. Within its
objective-success handler, parent completion is treated as an implicit
root-success effect for duplicate/conflict checks; that implicit edge also
participates in the plan-wide same-kind cycle graph. The canonical transition-plan
JSON limit is 384 KiB and the exact edit-request JSON limit is 512 KiB.

The pure
`apply_revision3_quest_transition_plan_transaction_v1` is bound to the exact
head, project ID/revision/target, Quest ID/revision, and a domain-separated
seal of the exact effective transition plan. It first regenerates and proves
the owned ScriptModule, preserves the active objective slots and technical
module identity, increments only the project, Quest, and owned module revisions
once, regenerates source, and requires canonical reopen equality. A retained
V4 plan that is byte-for-byte unchanged is a no-op. For a version-2 or
version-3 Quest the effective plan is synthesized from the frozen source shape;
the first accepted plan is an explicit upgrade to generator version 4. The pure
native contract even permits a seed-identical explicit upgrade, while the
current friendly dialog requires at least one visible behavior change before
Save.

The canonical transaction request contains exactly `expected_head`,
`expected_project_id`, `expected_revision`, `expected_target`, `quest_id`,
`expected_quest_revision`, `expected_transition_plan_seal`, and
`transition_plan`. The plan seal hashes the ASCII domain
`gore-authoring.revision3-quest-transition-plan-v1\0`, the canonical plan JSON
length as unsigned 64-bit big-endian, and the canonical plan bytes; its
`byte_len` is the plan JSON length.

The strict prepare-only FFI command is
`authoring_store_prepare_revision3_quest_transitions_edit_v1`. Its payload
contains exactly `current_project_json`, `quest_transitions_request_json`, and
`root`; there is no `game_root`, compiler, artifact, collision inventory, or
publication authority. Both nested transports and the outer wire must be exact,
bounded, duplicate-free canonical JSON. Native code fully opens the published
Store with asset verification, binds the request, runs the filesystem-free
transaction, prepares immutable candidate objects, fully reopens the candidate,
and rechecks the fixed head before and after response construction. A
successful response returns the two heads, canonical candidate project, exact
project/Quest/module identities and revisions, prior generator version,
legacy-upgrade flag, new plan seal, and only these outcome/status claims:

- `outcome: prepared_unpublished`;
- `build_status: blocked`;
- `runtime_status: runtime_unqualified`; and
- `publication_status: not_supported`.

`not_supported` describes the native route's publication authority. The managed
Studio session may persist the candidate only through the common serialized
fixed-head byte compare-and-swap, repair journal, and full published reopen. It
validates the exact Quest/module revisions, legacy-upgrade flag, retained plan
seal, statuses, and candidate closure before that CAS. Stale checkpoints fail
without publication; any uncertain publication poisons the session and requires
a reopen. Neither native preparation nor managed publication writes the game
installation or a save file.

### V4 source lowering and compiler/cache evidence

The V4 renderer emits only reviewed Quest lifecycle constructs. External flags
map to `bExternalAvailabilityTrigger`, `bExternalStartTrigger`,
`bExternalSuccessTrigger`, and `bExternalFailTrigger`; opting out of the game's
default external availability emits an explicit false override. Predicates map
to `ShouldBeAvailable_Implementation`, `ShouldStart_Implementation`,
`ShouldSucceed_Implementation`, and `ShouldFail_Implementation`. Follow-up
actions map to the three `HandleQuest*` hooks and guarded `StartQuest`,
`SucceedQuest`, or `FailQuest` calls through typed generated getters.

An isolated G1R 1.0.3 compiler qualifier covered three new `UG1RQuest` classes,
all four external flags coexisting with all four predicate hooks, all three
handlers, `bSucceedParent`, typed cross-node getters, and the guarded lifecycle
calls. Two independent compiler runs against a complete temporary game copy
were extracted/remapped and spliced into the pristine 7,305-module Shipping
cache. Both final 7,306-module caches reopened, decompiled, and disassembled to
the same 23-function construct set and were byte-identical: 123,406,626 bytes,
SHA-256
`FB041B3DF1CBD5A0AFC1D87F47BFCA6392AA19CE6475CE9DBD61A6D099D9C41A`.

This qualifies the exact hook, flag, handler, getter, and guarded-call shapes
listed above in the offline compiler/cache pipeline. Generator tests freeze the
exact V2/V3-compatible source and cover representative V4 lowering, but no
single renderer-produced fixture containing every supported state-test
expression has yet passed through that qualifier. It is not an in-game behavior
proof. No qualifier module was installed or launched, the real game Script tree
and all 106 save files retained their preflight seals, and the 36.36-GiB
sandbox was removed.
`gore as default-sites` still reports zero sites for the new classes, so generic
new-class scalar-default editing through that command remains a tooling gap.
Runtime registration, predicate polling/order, handler order, parent completion,
dialog selection, journal/reward/item effects, persistence, save/reload, and
clean uninstall all remain unqualified.

## Safe qualification order

1. Build the new module offline and reopen the mini-cache.
2. Verify the exact new class inventory and directly inspect every generated
   `__InitDefaults` record for parent, kind, flags, text, and journal links.
3. Require all authored functions and references to resolve and disassemble.
4. Build a bundle without deploying and verify its manifest and payload hashes.
5. On a disposable save, prove discovery/read-only state first.
6. Separately test acceptance/start, then one objective transition, then parent
   completion, journal/reward behavior, and save/reload persistence.
7. Compare the disposable save semantically and verify clean undeploy before
   widening the qualified capability.

The retained lifecycle qualifier closes the compiler/cache check only for its
exact listed construct shapes and G1R 1.0.3 fixture. It does not skip a future
exact generated-source check or the per-project artifact, deployment,
disposable-save, observation, persistence, or cleanup gates above.

Never manufacture a conversation with a console command, ability grant, or
direct activation in order to reach a quest callsite. Dialog selection and
quest effects must be exercised through a natural conversation on a disposable
save.

## Exact-current source inspection

The read-only FFI command
`authoring_store_inspect_revision3_quest_source_v1` accepts only a managed Store
root, a game root, the caller's exact canonical head, and one Quest ID. Native
code fully reopens that head, reads the persisted version-2 collision artifact,
reconstructs its immutable historical basis, rebuilds fresh executable,
Shipping-cache, Binds, catalog, and base-inventory inputs, and consumes a
dedicated one-shot inspection capability. Clients cannot supply project JSON,
artifact bytes, catalog data, collision selections, or a reusable capability.

The resulting schema-3 plan regenerates the selected version-2, version-3, or
version-4 Quest module and requires byte-exact equality with the persisted
ScriptModule. Its provenance binds the exact current project, historical
collision head and project, non-Quest basis, prior-Quest evidence, raw artifact,
and semantic collision source. Native code then revalidates the game inputs and
fully reopens the unchanged current Store head before returning the sealed plan.
The route performs no Store, game, or save write. A retained real-install test
uses a temporary Store and proves that the Store tree is byte-identical before
and after inspection.

Mod Studio opens this from the selected Quest Workbench's **Problems & Checks**
tab through **Open source & compiler checks**. The resulting **Source & checks**
view explains the successful source/input/head checks and keeps the negative
boundaries visible. Advanced disclosure shows the generated
AngelScript, module path, IDs, and seals. This is not a compiler invocation:
`build_status` remains `blocked`, runtime remains `runtime_unqualified`, and
publication remains `not_supported`. The inspection grants no artifact,
authoring, compile, build, deploy, runtime, or fixed-head authority.

### Separate exact-current compiler evidence

After the read-only inspection succeeds, **Source & checks** now offers an
explicit **Check with game compiler** action. It does not turn the inspection
result into compiler input. The app sends only the Store root, configured game
root, exact working head, and selected Quest ID. Native code independently
reopens the Store, derives the Quest and ScriptModule revisions, namespace,
relative path, persisted source, and source SHA-256, then acquires the shared
install-mutation guard. No caller can select the compiler work directory.

Under that guard native code regenerates the exact Quest source with fresh
installed-game/catalog/collision evidence, requires equality with the persisted
module, stages it with fixed additive/new-symbol policy in an unreported native-
private workspace, invokes the game compiler, restores every touched
installation path, and neutralizes the mini-cache through its retained create-
new/no-follow file handle. The closed response exposes bounded structured
diagnostics and exact project/entity/module bindings, but no source, cache,
staging, or reusable artifact path. A compiled result is accepted only while
both native and the managed session retain the same exact head, restoration is
exact, no recovery is required, and output disposal is proven.

Compiler rejection is ordinary evidence. Post-attempt Store drift retains the
diagnostics but revokes exact-current acceptance and requires reopening. Restore
uncertainty enters the shared app-wide install safety gate and blocks later
compiler/deploy mutation until a fresh native probe proves safety. Native
staging is neither selected nor cleaned through an app-provided path.

This is compiler evidence for one exact generated source only. No compiled
artifact is adopted, the project is not published, and build, deploy, runtime,
quest behavior, dialog selection, persistence, save safety, and publication
remain unqualified.

## Mod Studio boundary

Mod Studio now presents the bounded outline, context, logic, and inspection
actions as atomic projections of the selected Quest's Story Workbench. It also
provides the Draft wizard, count-preserving legacy outline edit, stable-slot-
aware V4 outline edit, catalog-bound context edit, V4 behavior table, and read-
only generated-source inspection described above. A synchronized
transcript/general graph, journal/reward/item authoring, arbitrary source,
complete diagnostics, build lowering, deployment, and runtime test workflow are
not part of this slice. The deterministic generator itself does not invoke the
compiler or compose a cache.

The native compiler now has a bounded structured-report API that
retains diagnostics-capture disposition and file/line/column/severity/message
records without reparsing formatted compiler errors. True hook/signature/
preflight absence uses the normal compiler fallback exactly once. If the first
generator already completed, `UnavailableWithoutFallback` uses that result and
does not start a second process. Invalid capture becomes `CaptureInvalid` and
rejects an otherwise usable cache; an unconfirmed process exit remains
fail-closed, preserves recovery state, and exposes no possibly live capture.
The generic Scripts workspace consumes that structured report: it requires
an explicit close-game confirmation, shows compiler diagnostics and fallback
status, accepts output only after an exact install restore, and surfaces retained
recovery as the dominant failure state. Managed Quest/NPC checks reuse the same
guard and diagnostic model through their stricter Store-derived commands, but
discard output instead of adopting the generic mini-cache. They change only the
selected source's compiler-evidence status; all build/runtime readiness gates in
this document remain closed.

Offline compiler/compose/reopen evidence now covers the listed lifecycle
field/hook/handler/getter/call shapes, but not one exact renderer-produced
fixture spanning every state-test expression. The managed Quest project also
still has no complete one-click diagnostic, artifact, build, deploy, or cleanup
chain. Consequently every V4 Quest result remains build-blocked and
runtime-unqualified even when its plan is valid.

Studio may report new-class discovery as qualified only when the versioned
capability registry matches the exact proven game generation and class shape;
that does not change the generator's own runtime-unqualified result. Production
build remains blocked for every required transition, dialog effect, reward,
journal action, or persistence behavior that has not passed its separate
qualification gate.
