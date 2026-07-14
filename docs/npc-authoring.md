# AngelScript NPC authoring

A candidate logical NPC identity can be expressed as a linked AngelScript class
chain that leaves archetype and visual/actor defaults inherited from an existing
human parent. Compilation and cache composition of that chain are now proven
offline. Runtime class residence, effective visuals, spawning, independent
dialog/quest state, persistence, and save behavior remain separate
qualification steps.

## Offline-proven logical clone

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

## What this changes

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

## Remaining runtime gates

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

## Mod Studio boundary

Mod Studio can safely provide a Draft NPC wizard that selects a qualified
vanilla archetype, generates the definition/config/spawn classes and unique
namespace, previews the exact source, and runs the offline verifier. The UI can
mark that class chain as offline-qualified while keeping runtime spawn,
persistence, dialog, and quest capabilities explicitly blocked or
Experimental. It must not label a compiled but unspawned class chain as a
working new NPC.

## Native read-only archetype catalog

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

## Native revision-3 Draft transaction

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

This lands the native prepare-only revision-3 boundary, not managed-session or
Studio publication, a semantic NPC editor, generated production build output,
spawn support, or runtime qualification. The retained live FFI test remains
environment-gated by `GORE_STORY_GAME_ROOT`; offline core and FFI fixtures do
not substitute for that pinned-game proof.
