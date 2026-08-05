# Mod Studio NPC and quest authoring internals

This page records implementation contracts, invariants, retained proof, and
native transaction behavior for Mod Studio's NPC and quest authoring. It is
not instructions: the user-facing workflows live in
[Mod Studio](../guide/mod-studio.md).

## NPC authoring

A candidate logical NPC identity can be expressed as a linked AngelScript class
chain that leaves archetype and visual/actor defaults inherited from an existing
human parent. Compilation and cache composition of that chain are now proven
offline. Runtime class residence, effective visuals, spawning, independent
dialog/quest state, persistence, and save behavior remain separate
qualification steps.

### Offline-proven logical clone

The bounded `NpcLogicalCloneV1` probe adds exactly one module with three new
Asghan-derived classes:

```text
new CharacterDefinition
  m_UniqueName = GORE_LOGICAL_ASGHAN_CLONE_V1
        ↓
new AIAgentConfig
  m_CharacterDefinition = new CharacterDefinition
        ↓
new SpawnAIAgentDefinition
  AIAgentConfigClass = new AIAgentConfig
```

The authored source is:

```angelscript
class UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1
    : UCharacterDefinition_Human_OM_GRD_Asghan_263
{
    default m_UniqueName = n"GORE_LOGICAL_ASGHAN_CLONE_V1";
}

class UAIAgentConfig_Human_GORE_LOGICAL_ASGHAN_CLONE_V1
    : UAIAgentConfig_Human_OM_GRD_Asghan_263
{
    default m_CharacterDefinition =
        UCharacterDefinition_Human_GORE_LOGICAL_ASGHAN_CLONE_V1::StaticClass();
}

class USpawnAIAgentDefinition_GORE_LOGICAL_ASGHAN_CLONE_V1
    : USpawnAIAgentDefinition_OM_GRD_Asghan_263
{
    default AIAgentConfigClass =
        UAIAgentConfig_Human_GORE_LOGICAL_ASGHAN_CLONE_V1::StaticClass();
}
```

The guarded compiler produced a 6,837-byte additive mini-cache in 124.2
seconds. Composing it with the current pristine cache produced a parseable
123,401,035-byte candidate with 7,306 modules: exactly one more than the 7,305
module base. The emitted new module contains exactly those three classes.

Direct decompilation of the composed candidate resolves all three generated
defaults and their class links. It contains no unresolved `func?`, `syscall?`,
`field_0x`, or static-name placeholder. The only appended StaticName is
`GORE_LOGICAL_ASGHAN_CLONE_V1`.

Retained evidence:

- source SHA-256:
  `600C5B7C661748EB585D0785D83F74B928D74C1691F6302E6880DE4F3CFEFA25`;
- mini-cache SHA-256:
  `41D88E192D69065B4743370B0AA782056E1015F63A2E926A3C7246AD08820B4E`;
- composed candidate SHA-256:
  `151E8D8FA8C2B0DAFA8834B3D7B6CD80BE690DD0CE701F9DB3749CB86A52C22D`;
- full local proof: `work/probe/npc-logical-clone-v1/README.md` and
  `proof.json`;
- reproducible read-only verifier:
  `work/probe/npc-logical-clone-v1/verify-offline.ps1`.

The verifier also reconfirmed that the game executable, shipping script cache,
`Binds.Cache`, and `Music.bank` retained their pristine hashes. No deploy
record, compiler lock/journal/backup, loose script, development cache, probe
UE4SS component, game/compiler process, or spawn edit remained. The probe did
not start a playable session or perform a save operation.

### What this changes

Cooked DataAsset creation is not proven to be a universal prerequisite for a
new **logical** NPC identity. At the class/default level, the current chain
inherits an existing archetype, actor Blueprint, visuals, AI, routine defaults,
and other cooked content while changing the identity and class links in
AngelScript. Effective runtime resolution remains unproven.

Cooked asset/package tooling is still required when a mod needs genuinely new
meshes, materials, animations, character Blueprints, modular visual tokens,
placed world actors, or another registry/package shape that the chosen content
path actually uses. The offline class proof does not show that the game accepts
or spawns the logical clone.

### Remaining runtime gates

The proof does not yet establish:

- that all three new classes are resident and accepted by native spawn code;
- a safe placement or spawn hook for a new body;
- a distinct GlobalId and save record with no vanilla identity collision;
- independent dialog, quest, knowledge, inventory, or routine state;
- visuals, AI, streaming-boundary behavior, save/reload, or uninstall behavior;
- compatibility with another executable or future hotfix.

The next qualification order is:

1. Rebuild and verify the three-class mini-cache offline.
2. Deploy only the class module and read back the exact class/CDO identities
   without spawning or mutating gameplay.
3. Separately compose one conservative, reviewed spawn-site edit.
4. Spawn only on a disposable save/profile and verify unique identity, visuals,
   AI, interaction, dialog/quest separation, streaming, and save/reload.
5. Compare the disposable save semantically, undeploy, and verify the pristine
   installation before widening the capability claim.

A second call to a vanilla spawn definition only creates another body sharing
the vanilla identity; it is not a substitute for this new linked class chain.

### Mod Studio boundary

The central closed generation registry currently contains exactly three
reviewed Steam generation triples: the retained Steam 1.0.3 Hotfix 1 seal set,
Steam build `24169431`, and Steam build `24340829` from the 2026-07-31 update.
For Story/NPC/Quest, executable, deployment-aware pristine Shipping cache, and
`Binds.Cache` must all match the same registered row; nearby hashes and cross-
generation mixtures fail closed. Item authoring has a separate narrower gate:
the exact executable seal selects the row and its audited Item field matrix,
without treating Shipping or Binds as Item evidence. The `24340829` row retains
offline qualification with no class reparenting or property-owner moves,
byte-identical curated modules, and an unchanged audited Item field matrix. It
therefore admits the existing bounded project-only Story/NPC/Quest routes for
the exact triple and the Item route for the exact executable seal. It grants no
dialog-runtime, production-build, deployment, live-game, or DataAsset authority,
and it is not a promise that future or non-Steam builds are compatible without
their own reviewed row.

Native Story/NPC catalog refusals for an unsupported generation may include the
optional, bounded `error.details` object
`{"kind":"unsupported_generation","actual":{...},"supported":[...]}`. This
is a closed diagnostic contract: `actual` is the observed executable/Shipping/
Binds seal triple and `supported` contains at most 16 complete registered
triples. Some refusal paths cannot retain that observed triple and therefore
omit `details`; absence remains valid. Dart accepts only the exact schema on the
matching unsupported-generation command/code pairs and treats malformed or
inconsistent details as `MALFORMED_NATIVE_RESPONSE`. The object contains no
native paths and grants no build, deployment, publication, runtime, or broader
generation authority.

Publication is bound to the exact project root, project ID, revision, and head.
A changed checkpoint or a session that requires reopen locks the wizard rather
than applying stale intent. On success the managed session publishes the
NPC/`ScriptModule` pair through guarded fixed-head byte CAS, fully reopens the
published checkpoint, and refreshes the visible project revision and content
library.

#### Guided Character + first greeting Draft V1

Both successful handoffs bind the returned publication to the exact reopened
checkpoint and require one-revision, new-head progression. Completion opens the
exact NPC's **Story -> Dialog & Voice** surface at N+2 with the created line
selected. The recipe is single-flight. Head or project drift locks it, and an
uncertain publication requires reopening instead of an automatic retry.

### Native read-only archetype catalog

The native command
`authoring_npc_archetype_catalog_v1_build_for_game_root` accepts exactly
`{"game_root":"..."}`. It derives the executable and `Binds.Cache` paths and
selects the Shipping cache only through the deployment-aware pristine selector;
clients cannot submit cache paths, backups, catalog provenance, or claims.

On success it returns one bounded canonical `npc_archetype_catalog.v1` JSON
string, a domain-separated binding to the exact game-root request, generation,
catalog/source/payload seals, record/rejection counts, and these fixed claims:

- linkage: `sealed_linkage_verified`;
- runtime: `runtime_unqualified`;
- build, deploy, publication: `not_supported`.

Executable and Binds guards plus the pristine Shipping selection are
revalidated around catalog serialization and bounded response construction.
Errors never include native paths. The command performs no filesystem writes,
game launch, build, deploy, publication, or runtime qualification.

### Native revision-3 Draft transaction

The revision-3 core now has a filesystem-free atomic transaction for one NPC
Draft plus its owned deterministic `ScriptModule`. The closed request binds the
exact working head, project ID, revision, game-generation target, distinct
entity IDs, display name, module namespace, unique runtime name, and one native
archetype catalog ID. Parent class provenance is not accepted from serialized
caller input.

Instead, the transaction consumes two fresh native contexts: a Story/NPC
catalog selection sealed to the exact generation, and a base-game plus
exact-current-project collision inventory. The collision domains cover module
namespaces, relative source paths, generated symbols, and case-insensitive
runtime IDs projected by the pinned Story catalog. Existing revision-3 NPC and
Quest/module pairs are regenerated and checked as a complete closure; valid
existing Quests remain unchanged, while residual modules, orphan ownership,
source drift, entity/runtime/symbol/path collisions, or a mismatched
head/project/revision/target fail before a candidate is returned. The runtime-ID
claim is deliberately limited to the catalog's curated projection, not unknown
game NPCs outside it.

The native FFI command `authoring_store_prepare_revision3_npc_draft_v1`
reopens the exact published project, rebuilds the Story and NPC catalogs and
current-project collision source, consumes those contexts in the transaction,
prepares one immutable candidate checkpoint, and fully reopens it. It
revalidates the game inputs and fixed head around preparation and never replaces
the fixed head or writes into the game. Its only status claims are:

- `build_status: blocked`;
- `runtime_status: runtime_unqualified`;
- `catalog_authority: not_granted`;
- `collision_authority: not_granted`;
- `source_inspection: fresh_native_context_required`;
- `publication_status: not_supported`.

Strict Dart request/result DTOs validate the exact basis, statuses, generated
NPC/module closure, catalog-resolved parent evidence, AssetStore bindings, and
candidate checkpoint returned by that command. The managed revision-3 session
then independently reopens and publishes the candidate through its serialized
repair-journal and fixed-head byte-CAS lane, followed by a full published
reopen. Retryable catalog/input failures remain distinct from integrity or
publication uncertainty; uncertain state requires reopening the session.

Together with the Guided wizard above, this lands managed-session and visible
Studio publication of the bounded NPC Draft. It does **not** land a complete
semantic NPC editor, generated production build output, class residence,
discovery, spawn support, or runtime qualification. The retained live FFI test
remains environment-gated by `GORE_STORY_GAME_ROOT`; offline core and FFI
fixtures do not substitute for that pinned-game proof.

### Managed revision-3 existing-NPC profile edit V1

Studio loads an exact NPC/module seed and a fresh sealed Story+NPC catalog when
the dialog opens, and refreshes that catalog immediately before save. Native
`authoring_store_prepare_revision3_npc_profile_edit_v1` then reopens the exact
Store basis and independently rebuilds the fresh catalogs. It resolves both the
current catalog witness chosen by matching the persisted triple and the
requested catalog ID, then verifies every parent binding's generation, source
seal, catalog layer, canonical selector, and runtime class. Catalog IDs are not
treated as archetype identity: two records with the same complete triple are
structural aliases. Drift, an unavailable current triple, a stale entity/module/
checkpoint, a changed fixed head, or publication uncertainty fails closed;
uncertainty requires reopen and is never automatically retried.

The pure
`apply_revision3_npc_profile_edit_transaction_v1` transaction always preserves
technical identity, unrelated entities, and the complete AssetStore. Every real
edit advances the project and selected NPC revisions exactly once. A name-only
edit preserves the entire owned ScriptModule byte-for-byte, including its
revision, even when the selected catalog ID is merely an alias of the current
full triple. When and only when the full parent triple differs, the transaction
atomically replaces all three provenances, deterministically regenerates the
owned ScriptModule, and advances that module revision exactly once. The route
fully reopens the immutable candidate and returns only
`build_status: blocked`, `runtime_status: runtime_unqualified`, catalog and
collision authority `not_granted`, and native publication `not_supported`.

Only the serialized managed session may publish the candidate through guarded
fixed-head byte CAS, repair journaling, and a full published reopen. The game
installation supplies read-only catalog evidence; the editor writes only the
managed project. It performs no compiler or production build, deploy/undeploy,
game or save mutation, class residence/discovery, spawn, runtime behavior, or
qualification. A successful project edit therefore needs no game test and does
not make the NPC playable.

### Managed revision-3 NPC greeting lines V1

`NpcDraft.greetings` is optional authoring metadata outside the deterministic
NPC generator input. Empty greeting lists omit it. Nonempty lists contain at
most 256 unique, ordered,
same-project DialogLine references. The content index emits the generated
ScriptModule relationship first, then `npc_greeting_line` references in authored
order plus an exact `greeting_count`. Detaching a binding never deletes the
shared line, localization, Voice slots, takes, or assets. Removing the NPC drops
only its outgoing greeting relationships and retains that shared content.

The pure `apply_revision3_npc_greeting_edit_transaction_v1` operation either
replaces the complete reviewed order or embeds one existing DialogLine creation
and inserts it atomically. Both modes advance the project and selected NPC
revisions exactly once while preserving the owned ScriptModule revision,
source bytes, generator input/fingerprint, unrelated entities, assets, and
target. The prepare-only FFI route has no game-root/install/save parameter,
fully reopens the immutable candidate, repeats fixed-head guards, and returns
only `blocked`, `runtime_unqualified`, `not_granted`, and `not_supported`
status. Only the serialized managed session publishes through guarded
fixed-head byte CAS, repair journaling, and a full published reopen.

This metadata deliberately does **not** create an AngelScript topic, greeting
condition, player choice, selection effect, Quest relationship, NPC runtime
registration, build output, deployment, or playable conversation. It writes
only the managed project and does not touch the game installation or a save.
Those runtime and lowering mechanisms remain separate research gates.

### Managed revision-3 NPC Draft removal V1

The pure `apply_revision3_story_draft_removal_transaction_v1` independently regenerates
the NPC module, proves its generator/origin/payload owner and exact three-edge
closure, advances the project once, removes exactly those two entities, and
preserves every other entity plus the complete AssetStore. The strict
`authoring_store_prepare_remove_revision3_story_draft_v1` FFI route fully opens
the fixed basis, prepares and fully reopens an immutable candidate, and repeats
fixed-head guards without replacing `gore-project.json`. Only the serialized
managed session may publish by exact-head byte CAS and full published reopen.

The successful workspace refresh cannot retain the removed NPC; it selects a
deterministic remaining Story Draft or shows the empty state. There is no
automatic retry. This operation performs no physical CAS/blob deletion,
compiler/build/deploy work, game or save access, spawn change, or runtime
qualification. The removal route itself has no local rollback, but its
published checkpoint participates in bounded authenticated project History and
global Undo: a retained earlier version is restored as a new revision without
erasing later history. Whole-project backup and restore likewise exist only as
[Snapshot V2](studio-project-archive.md). General project deletion remains a
separate missing fundamental.

### Managed source/readiness profile

`build_revision3_npc_source_inspection_plan_v1` is the pure, read-only native
foundation for the existing-NPC profile. It
accepts exact canonical revision-3 project JSON plus one NPC entity ID, verifies
the selected NPC/ScriptModule closure and persisted parent provenance triple,
and requires exact regeneration of generator, owner, origin, module path,
source SHA-256, and input fingerprint. Other project content, including
generator-V4 Quests, remains present in the exact input but is neither widened
nor copied into the bounded NPC plan.

The exact Store-bound FFI command
`authoring_store_inspect_revision3_npc_source_v1` accepts only the Store root,
canonical expected head, and selected NPC ID. It fully opens the current Store
before and after plan construction, binds the canonical project and plan seals,
and accepts no game-root or compiler request. It returns no canonical project
JSON, Store/absolute host path, or reusable authority; the bounded plan
intentionally includes the canonical module-relative `.as` path and generated
source for read-only inspection. The managed session runs it through the
serialized exact-read lane and checks the published head on both sides.
Local/content errors are retryable only while that head remains exact; Store or
response integrity uncertainty requires reopen.

The plan reports `compiler_status: not_run`, `build_status: blocked`,
`runtime_qualification: runtime_unqualified`, and unsupported spawn/publication
with four fixed readiness-blocker diagnostics. The outer result is likewise
closed to `inspection_only`, `source_readiness_inspection_only`, and
`persisted_and_regenerated_exact`. Its serializer enforces the entity and
four-MiB plan envelopes before allocating the canonical result. The exposed
route has no compiler, build, spawn, deployment, mutation, or publication entry
point. Successful source regeneration is therefore not evidence of class
residence, discovery, spawning, distinct runtime state, or save safety.

### Exact-current compiler check

When a configured installation is available, the opened **Profile & checks**
dialog offers a separate evidence-only compiler action. The app submits
only the managed Store root, configured game root, exact working head, selected
NPC ID. It cannot submit source text, module identity, compiler policy, a work
directory, or an output path.

Native code reopens the exact project and derives the NPC revision, owned
ScriptModule revision, namespace, relative path, persisted source, and source
SHA-256 before acquiring the shared install-mutation guard. This explicitly
invoked check is the bounded exception to Studio's normal read-only use of the
installation. Under that guard it re-runs the closed NPC source inspection,
verifies fresh game inputs, creates the complete validated base source tree in
an unreported native-private workspace, overlays the derived managed module
with fixed additive/new-symbol policy, stages that complete tree under
`Script/`, runs the game compiler, restores every touched install path, and
neutralizes the mini-cache through the exact file handle retained from its
create-new/no-follow write. The response contains bounded
file/line/column/severity diagnostics and exact project/entity/module evidence,
but no source, mini-cache, staging, or reusable artifact path.

Studio accepts the result as “exact source accepted” only when native and the
managed session both retain the same exact head, the compiler accepted the
source, installation restoration is exact, no recovery is required, and output
disposal is proven. A post-attempt Store drift keeps the diagnostics but makes
them stale and requires reopening. Restore uncertainty is retained in the
app-wide install safety gate and blocks every later compiler or deploy mutation
until a fresh native probe proves the installation safe. Native staging is an
internal implementation detail and is never caller-selected or returned.

This closes only the selected generated-source compiler check. It still grants
no production build, cache adoption, deployment, class residence, spawn,
runtime, publication, or save authority. The remaining production, residence,
and spawn blockers stay visible after compiler acceptance.

## Quest authoring

Quest authoring is a greenfield Revision-3 workflow. The only persisted
generator contract is version 4.

### Current model

A Quest consists of one `quest_draft` entity and one generated
`script_module` entity. Both entities use:

- generator ID `gore-authoring.draft-quest-skeleton`
- generator version `4`
- an exact owner/reference pair
- revisions that advance together for Quest edits

The Quest input contains the runtime identity, resolved parent and giver,
localized text literals, objective titles, collision-catalog evidence, and a
required `transition_plan`.

The transition plan owns stable positive objective slots. Slot `1` is always
present, active slots are strictly ascending, the visible objective order is a
complete permutation of those slots, and `next_slot_ordinal` never reuses a
slot. New Quests receive the canonical default plan for their objective count.

### Publication contract

All edits use the managed project transaction lane:

1. Read an exact-current private seed where the editor needs one.
2. Build a bounded request carrying the expected head and entity revisions.
3. Prepare one unpublished native candidate.
4. Verify that only the permitted Quest/module delta occurred.
5. Publish with compare-and-swap and fully reopen the resulting checkpoint.

Head conflicts never overwrite a winner. Integrity uncertainty marks the
session as requiring reopen. Correctable semantic conflicts remain retryable.

### Runtime boundary

Mod Studio can author, persist, inspect, and prepare generator-version-4 Quest
content. Runtime qualification and game installation remain explicit status
claims; the editor never presents an offline draft as proven playable. The
managed compiler and runtime validation work must succeed before publication to
the game can be claimed.

### Invariants for new work

- Add functionality to the single Revision-3/version-4 path.
- Keep the transition plan required in every persisted Quest.
- Preserve stable objective slots across outline, behavior, and transcript
  edits.
- Keep generated module source and seals derived from the exact Quest input.
- Do not add alternate readers or duplicate editor APIs.
