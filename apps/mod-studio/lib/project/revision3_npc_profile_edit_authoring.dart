// ignore_for_file: prefer_initializing_formals

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_npc_authoring.dart';

typedef Revision3NpcProfileEditSeedLoader =
    Future<AuthoringRevision3NpcProfileEditSeed> Function({
      required String npcId,
      required int expectedNpcRevision,
      required String expectedScriptModuleId,
      required int expectedScriptModuleRevision,
      required String expectedUniqueName,
      required String expectedModuleNamespace,
      required String expectedParentCharacterDefinition,
      required String expectedParentAiAgentConfig,
      required String expectedParentSpawnDefinition,
    });

typedef Revision3NpcProfileEditTechnicalPublisher =
    Future<Revision3NpcProfileEditPublication> Function({
      required String gameRoot,
      required Revision3NpcProfileEditTechnicalPlan plan,
    });

final class Revision3NpcProfileEditCheckpoint {
  const Revision3NpcProfileEditCheckpoint._({
    required this.index,
    required this.npc,
    required this.module,
    required this.seed,
    required this.catalog,
    required this.currentArchetype,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity npc;
  final Revision3ContentEntity module;
  final AuthoringRevision3NpcProfileEditSeed seed;
  final Revision3NpcCatalog catalog;
  final Revision3NpcCatalogChoice currentArchetype;

  Revision3NpcProfileEditCheckpoint withCatalogForReview(
    Revision3NpcCatalog freshCatalog,
  ) {
    _requireNpcProfileCatalog(freshCatalog, index);
    final current = _requireCurrentNpcArchetype(seed, freshCatalog);
    return Revision3NpcProfileEditCheckpoint._(
      index: index,
      npc: npc,
      module: module,
      seed: seed,
      catalog: freshCatalog,
      currentArchetype: current,
    );
  }
}

/// Transaction-only intent. The session rebinds it to its exact current head
/// before native preparation.
final class Revision3NpcProfileEditTechnicalPlan {
  const Revision3NpcProfileEditTechnicalPlan._({
    required this.projectId,
    required this.projectRevision,
    required this.seed,
    required this.expectedParentCatalogId,
    required this.expectedCurrentParentTriple,
    required this.displayName,
    required this.parentCatalogId,
    required this.expectedParentTriple,
    required this.expectedStoryCatalogSeal,
    required this.expectedNpcCatalogSeal,
    required this.nameChanged,
    required this.archetypeChanged,
    required this.moduleRegenerated,
  });

  final String projectId;
  final int projectRevision;
  final AuthoringRevision3NpcProfileEditSeed seed;
  final String expectedParentCatalogId;
  final AuthoringRevision3NpcProfileParentTripleExpectation
  expectedCurrentParentTriple;
  final String displayName;
  final String parentCatalogId;
  final AuthoringRevision3NpcProfileParentTripleExpectation
  expectedParentTriple;
  final AuthoringDraftContentSeal expectedStoryCatalogSeal;
  final AuthoringDraftContentSeal expectedNpcCatalogSeal;
  final bool nameChanged;
  final bool archetypeChanged;
  final bool moduleRegenerated;

  String get npcId => seed.npcId;
  int get expectedNpcRevision => seed.npcRevision;
  String get scriptModuleId => seed.scriptModuleId;
  int get expectedScriptModuleRevision => seed.scriptModuleRevision;
}

final class Revision3NpcProfileEditPublication {
  const Revision3NpcProfileEditPublication({
    required this.projectId,
    required this.projectRevision,
    required this.npcId,
    required this.npcRevision,
    required this.scriptModuleId,
    required this.scriptModuleRevision,
    required this.displayName,
    required this.previousParentCatalogId,
    required this.parentCatalogId,
    required this.nameChanged,
    required this.archetypeChanged,
    required this.moduleRegenerated,
  });

  final String projectId;
  final int projectRevision;
  final String npcId;
  final int npcRevision;
  final String scriptModuleId;
  final int scriptModuleRevision;
  final String displayName;
  final String previousParentCatalogId;
  final String parentCatalogId;
  final bool nameChanged;
  final bool archetypeChanged;
  final bool moduleRegenerated;
}

final class Revision3NpcProfileCatalogDriftException implements Exception {
  const Revision3NpcProfileCatalogDriftException(this.freshCatalog);

  final Revision3NpcCatalog freshCatalog;
}

final class Revision3NpcProfileEditRequiresReopenException
    implements Exception {
  const Revision3NpcProfileEditRequiresReopenException();
}

final class Revision3NpcProfileEditStaleCheckpointException
    implements Exception {
  const Revision3NpcProfileEditStaleCheckpointException();
}

final class Revision3NpcProfileEditUnavailableException implements Exception {
  const Revision3NpcProfileEditUnavailableException();
}

/// Fresh-seed/fresh-catalog orchestration for one existing NPC profile.
final class Revision3NpcProfileEditAuthoringService {
  const Revision3NpcProfileEditAuthoringService({
    required Revision3NpcProfileEditSeedLoader loadSeed,
    required Revision3NpcCatalogLoader loadCatalog,
    required Revision3NpcProfileEditTechnicalPublisher publishTechnicalPlan,
  }) : _loadSeed = loadSeed,
       _loadCatalog = loadCatalog,
       _publishTechnicalPlan = publishTechnicalPlan;

  final Revision3NpcProfileEditSeedLoader _loadSeed;
  final Revision3NpcCatalogLoader _loadCatalog;
  final Revision3NpcProfileEditTechnicalPublisher _publishTechnicalPlan;

  Future<Revision3NpcProfileEditCheckpoint> load({
    required Revision3ContentIndex index,
    required Revision3ContentEntity npc,
    required String gameRoot,
  }) async {
    final module = _requireVisibleNpcModule(index, npc);
    final summary = npc.summary.npcDraft!;
    final loaded = await Future.wait<Object>(<Future<Object>>[
      _loadSeed(
        npcId: npc.id,
        expectedNpcRevision: npc.revision,
        expectedScriptModuleId: module.id,
        expectedScriptModuleRevision: module.revision,
        expectedUniqueName: summary.uniqueName,
        expectedModuleNamespace: summary.moduleNamespace,
        expectedParentCharacterDefinition: summary.parentCharacterDefinition,
        expectedParentAiAgentConfig: summary.parentAiAgentConfig,
        expectedParentSpawnDefinition: summary.parentSpawnDefinition,
      ),
      _loadCatalog(gameRoot),
    ]);
    final seed = loaded[0] as AuthoringRevision3NpcProfileEditSeed;
    final catalog = loaded[1] as Revision3NpcCatalog;
    _requireNpcProfileSeed(index, npc, module, seed);
    _requireNpcProfileCatalog(catalog, index);
    return Revision3NpcProfileEditCheckpoint._(
      index: index,
      npc: npc,
      module: module,
      seed: seed,
      catalog: catalog,
      currentArchetype: _requireCurrentNpcArchetype(seed, catalog),
    );
  }

  Future<Revision3NpcProfileEditPublication> publish({
    required Revision3NpcProfileEditCheckpoint checkpoint,
    required String gameRoot,
    required String displayName,
    required Revision3NpcCatalogChoice archetype,
  }) async {
    final normalized = Revision3NpcDraftAuthoringInput(
      parentCatalogId: archetype.catalogId,
      displayName: displayName,
    ).displayName;
    _requireNpcProfileSelection(checkpoint.catalog, archetype);
    final nameChanged = normalized != checkpoint.seed.displayName;
    final archetypeChanged = !archetype.parentTriple!.sameBinding(
      checkpoint.currentArchetype.parentTriple!,
    );
    if (!nameChanged && !archetypeChanged) {
      throw const FormatException('Change the NPC name or archetype.');
    }

    final fresh = await _loadCatalog(gameRoot);
    _requireNpcProfileCatalog(fresh, checkpoint.index);
    _requireCurrentNpcArchetype(checkpoint.seed, fresh);
    final freshArchetype = fresh.choice(archetype.catalogId);
    if (!checkpoint.catalog.sameSeal(fresh) ||
        freshArchetype == null ||
        !archetype.sameBinding(freshArchetype)) {
      throw Revision3NpcProfileCatalogDriftException(fresh);
    }
    final storySeal = fresh.storyCatalogSeal!;
    final npcSeal = fresh.npcCatalogSeal!;
    return _publishTechnicalPlan(
      gameRoot: gameRoot,
      plan: Revision3NpcProfileEditTechnicalPlan._(
        projectId: checkpoint.index.projectId,
        projectRevision: checkpoint.index.projectRevision,
        seed: checkpoint.seed,
        expectedParentCatalogId: checkpoint.currentArchetype.catalogId,
        expectedCurrentParentTriple: _npcProfileParentExpectation(
          checkpoint.currentArchetype.parentTriple!,
        ),
        displayName: normalized,
        parentCatalogId: freshArchetype.catalogId,
        expectedParentTriple: _npcProfileParentExpectation(
          freshArchetype.parentTriple!,
        ),
        expectedStoryCatalogSeal: storySeal,
        expectedNpcCatalogSeal: npcSeal,
        nameChanged: nameChanged,
        archetypeChanged: archetypeChanged,
        moduleRegenerated: archetypeChanged,
      ),
    );
  }
}

Revision3ContentEntity _requireVisibleNpcModule(
  Revision3ContentIndex index,
  Revision3ContentEntity npc,
) {
  if (npc.kind != Revision3ContentEntityKind.npcDraft ||
      npc.summary.npcDraft == null ||
      index.entityById(npc.id) != npc) {
    throw const FormatException(
      'The selected item is not the exact NPC from this project view.',
    );
  }
  final references = npc.references
      .where(
        (reference) =>
            reference.role == 'draft_script_module' &&
            reference.qualifier == null &&
            reference.resolution ==
                Revision3ContentReferenceResolution.resolved &&
            reference.target.projectId == index.projectId &&
            reference.target.expectedKind ==
                Revision3ContentEntityKind.scriptModule,
      )
      .toList(growable: false);
  final module = references.length == 1
      ? index.entityById(references.single.target.entityId)
      : null;
  if (module == null ||
      module.kind != Revision3ContentEntityKind.scriptModule) {
    throw const FormatException(
      'The selected NPC does not own one exact generated script.',
    );
  }
  return module;
}

void _requireNpcProfileSeed(
  Revision3ContentIndex index,
  Revision3ContentEntity npc,
  Revision3ContentEntity module,
  AuthoringRevision3NpcProfileEditSeed seed,
) {
  final summary = npc.summary.npcDraft!;
  if (seed.projectId != index.projectId ||
      seed.projectRevision != index.projectRevision ||
      seed.npcId != npc.id ||
      seed.npcRevision != npc.revision ||
      seed.scriptModuleId != module.id ||
      seed.scriptModuleRevision != module.revision ||
      seed.displayName != npc.displayName ||
      seed.uniqueName != summary.uniqueName ||
      seed.moduleNamespace != summary.moduleNamespace ||
      seed.parentCharacterDefinition.runtimeClass !=
          summary.parentCharacterDefinition ||
      seed.parentAiAgentConfig.runtimeClass != summary.parentAiAgentConfig ||
      seed.parentSpawnDefinition.runtimeClass !=
          summary.parentSpawnDefinition) {
    throw const Revision3NpcProfileEditStaleCheckpointException();
  }
}

void _requireNpcProfileCatalog(
  Revision3NpcCatalog catalog,
  Revision3ContentIndex index,
) {
  final generation = catalog.generationExecutableSeal;
  if (generation == null ||
      catalog.storyCatalogSeal == null ||
      catalog.npcCatalogSeal == null ||
      generation.byteLength != index.targetExecutableByteLength ||
      generation.sha256 != index.targetExecutableSha256 ||
      catalog.choices.any((choice) => choice.parentTriple == null)) {
    throw const Revision3NpcProfileEditUnavailableException();
  }
}

Revision3NpcCatalogChoice _requireCurrentNpcArchetype(
  AuthoringRevision3NpcProfileEditSeed seed,
  Revision3NpcCatalog catalog,
) {
  final matches = catalog.choices
      .where((choice) => _sameSeedAndChoice(seed, choice))
      .toList(growable: false);
  if (matches.isEmpty) {
    throw const Revision3NpcProfileEditUnavailableException();
  }
  // Several catalog IDs may intentionally alias the same byte-identical
  // parent triple. Any such ID is a valid exact current-selection witness;
  // the catalog's canonical ordering makes the chosen witness deterministic.
  return matches.first;
}

void _requireNpcProfileSelection(
  Revision3NpcCatalog catalog,
  Revision3NpcCatalogChoice selected,
) {
  final current = catalog.choice(selected.catalogId);
  if (current == null || !current.sameBinding(selected)) {
    throw const FormatException(
      'Choose an archetype from the current NPC catalog.',
    );
  }
}

bool _sameSeedAndChoice(
  AuthoringRevision3NpcProfileEditSeed seed,
  Revision3NpcCatalogChoice choice,
) {
  final triple = choice.parentTriple;
  final generation = seed.parentCharacterDefinition.generation.executable;
  return triple != null &&
      _sameProfileParent(
        seed.parentCharacterDefinition,
        triple.characterDefinition,
      ) &&
      _sameProfileParent(seed.parentAiAgentConfig, triple.aiAgentConfig) &&
      _sameProfileParent(seed.parentSpawnDefinition, triple.spawnDefinition) &&
      seed.parentAiAgentConfig.generation.executable.byteLength ==
          generation.byteLength &&
      seed.parentAiAgentConfig.generation.executable.sha256 ==
          generation.sha256 &&
      seed.parentSpawnDefinition.generation.executable.byteLength ==
          generation.byteLength &&
      seed.parentSpawnDefinition.generation.executable.sha256 ==
          generation.sha256;
}

bool _sameProfileParent(
  AuthoringRevision3NpcInspectionParent stored,
  Revision3NpcCatalogParentBinding catalog,
) =>
    stored.catalogLayer == catalog.catalogLayer &&
    stored.canonicalSelector == catalog.authoringSelector &&
    stored.runtimeClass == catalog.runtimeClass &&
    stored.sourceSeal.byteLength == catalog.sourceSeal.byteLength &&
    stored.sourceSeal.sha256 == catalog.sourceSeal.sha256;

AuthoringRevision3NpcProfileParentTripleExpectation
_npcProfileParentExpectation(Revision3NpcCatalogParentTriple triple) =>
    AuthoringRevision3NpcProfileParentTripleExpectation(
      characterDefinition: _npcProfileSingleParentExpectation(
        triple.characterDefinition,
      ),
      aiAgentConfig: _npcProfileSingleParentExpectation(triple.aiAgentConfig),
      spawnDefinition: _npcProfileSingleParentExpectation(
        triple.spawnDefinition,
      ),
    );

AuthoringRevision3NpcProfileParentExpectation
_npcProfileSingleParentExpectation(Revision3NpcCatalogParentBinding parent) =>
    AuthoringRevision3NpcProfileParentExpectation(
      catalogLayer: parent.catalogLayer,
      authoringSelector: parent.authoringSelector,
      runtimeClass: parent.runtimeClass,
      sourceSeal: parent.sourceSeal,
    );
