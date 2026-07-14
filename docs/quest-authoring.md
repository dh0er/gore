# AngelScript quest authoring

GORE can compile new `UQuest` subclasses, carry their generated defaults and
new symbols in an additive mini-cache, and compose them into the game's script
cache. Automatic discovery of new quest classes is narrowly runtime-proven on
the current Gothic 1 Remake generation. Quest transitions, authored effects,
dialog selection, and persistence require separate qualification.

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

The current Asghan candidate resolves and disassembles all 23 authored
functions offline. Its discovery proof upgrades new quest **class discovery**
from hypothesis to a supported narrow mechanism, but it does not upgrade the
candidate's transition/effect behavior to production-ready.

## Discovery-only Draft generators

`gore-authoring` retains the byte-frozen `DraftQuestSkeletonV1` for the smallest
useful Quest Draft: exactly one `UG1RQuest` root and one objective. The additive
`DraftQuestSkeletonV2` keeps that first objective's technical identities, then
emits two through eight ordered objective classes with deterministic
class/getter names. It regenerates the complete multi-objective source so only
the final generated objective has `bSucceedParent = true`; the separate V1
single-objective output remains byte-frozen. This represents author order and
completion shape; it does not claim that runtime transitions enforce the order.

Both generators are bounded and offline-only. They deliberately contain no
transition predicate or action, dialog selection, effect, reward, journal
operation, failure path, filesystem write, compiler invocation, game launch,
or save operation. Objective titles are canonical, byte-bounded, and unique
case-insensitively. Every generated class and getter is checked against the
sealed collision catalog, including all additional objectives.

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

The generator always reports `OfflineDraft`, `RuntimeUnqualified`, and
`TransitionsRuntimeUnqualified`. Caller-supplied seals cannot upgrade those
statuses. Runtime discovery evidence belongs to the versioned capability
registry and must match this exact generated operation independently.

### Native Quest-intent transaction

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

### Managed revision-3 existing-Quest outline edit V1

The managed R3 Content Library now exposes **Edit quest outline** for one
selected, exact-current `QuestDraft`. This count-preserving editor may change
only the Quest's name in the project library, its player-facing title, and the
text/order of its existing one through eight objectives. The objective count
cannot change in this operation. Description, Quest family/parent, giver,
technical identities, stable Quest and ScriptModule IDs, ownership, provenance,
and the retained `QuestCollisionArtifactRef` remain byte-for-byte unchanged.
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
read/write.

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

Never manufacture a conversation with a console command, ability grant, or
direct activation in order to reach a quest callsite. Dialog selection and
quest effects must be exercised through a natural conversation on a disposable
save.

## Mod Studio boundary

Mod Studio can safely provide a typed Draft quest wizard, outline/transcript/
graph views, localization, deterministic source generation, and dependency
checks on top of this generator. The generator itself does not compile or
compose anything. Offline compile/compose/reopen evidence exists through the
sealed retained candidate workflow, but a general one-click Studio compile path
is not qualified until that complete diagnostic and artifact chain is integrated
for Draft projects.

Studio may report new-class discovery as qualified only when the versioned
capability registry matches the exact proven game generation and class shape;
that does not change the generator's own runtime-unqualified result. Production
build remains blocked for every required transition, dialog effect, reward,
journal action, or persistence behavior that has not passed its separate
qualification gate.
