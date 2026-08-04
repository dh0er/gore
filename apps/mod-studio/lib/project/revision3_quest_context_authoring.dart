import 'dart:convert';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_quest_authoring.dart';

typedef Revision3QuestContextSeedLoader =
    Future<AuthoringRevision3QuestContextSeed> Function({
      required String questId,
      required int expectedQuestRevision,
      required String expectedModuleId,
      required int expectedModuleRevision,
      required String expectedParentRuntimeClass,
      required String expectedGiverRuntimeUniqueName,
    });

typedef Revision3QuestContextTechnicalPublisher =
    Future<Revision3QuestContextEditPublication> Function({
      required String gameRoot,
      required Revision3QuestContextEditTechnicalPlan plan,
    });

/// Exact visible checkpoint plus a fresh, display-safe Story catalog.
final class Revision3QuestContextEditCheckpoint {
  const Revision3QuestContextEditCheckpoint._({
    required this.index,
    required this.quest,
    required this.seed,
    required this.catalog,
    required this.currentParent,
    required this.currentGiver,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final AuthoringRevision3QuestContextSeed seed;
  final Revision3QuestCatalog catalog;
  final Revision3QuestParentChoice currentParent;
  final Revision3QuestGiverChoice currentGiver;

  Revision3QuestContextEditCheckpoint withCatalogForReview(
    Revision3QuestCatalog freshCatalog,
  ) {
    _requireCatalogSeal(freshCatalog, index);
    final current = _requireExactCurrentCatalogBindings(seed, freshCatalog);
    return Revision3QuestContextEditCheckpoint._(
      index: index,
      quest: quest,
      seed: seed,
      catalog: freshCatalog,
      currentParent: current.parent,
      currentGiver: current.giver,
    );
  }
}

/// Transaction-only context intent. Picker identities remain hidden from UI.
final class Revision3QuestContextEditTechnicalPlan {
  const Revision3QuestContextEditTechnicalPlan._({
    required this.questId,
    required this.expectedQuestRevision,
    required this.moduleId,
    required this.expectedModuleRevision,
    required this.expectedStoryCatalogSeal,
    required this.description,
    required this.parentCatalogId,
    required this.giverCatalogId,
    required this.expectedParentRuntimeClass,
    required this.expectedParentCatalogLayer,
    required this.expectedParentAuthoringSelector,
    required this.expectedParentSourceSeal,
    required this.expectedGiverRuntimeUniqueName,
    required this.expectedGiverCatalogLayer,
    required this.expectedGiverAuthoringSelector,
    required this.expectedGiverSourceSeal,
  });

  final String questId;
  final int expectedQuestRevision;
  final String moduleId;
  final int expectedModuleRevision;
  final AuthoringDraftContentSeal expectedStoryCatalogSeal;
  final String description;
  final String parentCatalogId;
  final String giverCatalogId;
  final String expectedParentRuntimeClass;
  final String expectedParentCatalogLayer;
  final String expectedParentAuthoringSelector;
  final AuthoringDraftContentSeal expectedParentSourceSeal;
  final String expectedGiverRuntimeUniqueName;
  final String expectedGiverCatalogLayer;
  final String expectedGiverAuthoringSelector;
  final AuthoringDraftContentSeal expectedGiverSourceSeal;
}

final class Revision3QuestContextEditPublication {
  const Revision3QuestContextEditPublication({
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
  });

  final String projectId;
  final int projectRevision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
}

final class Revision3QuestContextRequiresReopenException implements Exception {
  const Revision3QuestContextRequiresReopenException();
}

final class Revision3QuestContextStaleCheckpointException implements Exception {
  const Revision3QuestContextStaleCheckpointException();
}

/// The exact stored runtime connections cannot be represented by this catalog.
/// The editor never guesses a replacement because native collision authority
/// requires an explicit catalog selection.
final class Revision3QuestContextUnavailableException implements Exception {
  const Revision3QuestContextUnavailableException();
}

/// The game catalog changed after the author reviewed the visible choices.
final class Revision3QuestContextCatalogDriftException implements Exception {
  const Revision3QuestContextCatalogDriftException(this.freshCatalog);

  final Revision3QuestCatalog freshCatalog;
}

/// Fresh-seed/fresh-catalog orchestration for the separate context editor.
final class Revision3QuestContextAuthoringService {
  const Revision3QuestContextAuthoringService({
    required this._loadSeed,
    required this._loadCatalog,
    required this._publishTechnicalPlan,
  });

  final Revision3QuestContextSeedLoader _loadSeed;
  final Revision3QuestCatalogLoader _loadCatalog;
  final Revision3QuestContextTechnicalPublisher _publishTechnicalPlan;

  Future<Revision3QuestContextEditCheckpoint> load({
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
    required String gameRoot,
  }) async {
    _requireVisibleQuest(index, quest);
    final summary = quest.summary.questDraft!;
    final module = _requireVisibleQuestModule(index, quest);
    final loaded = await Future.wait<Object>([
      _loadSeed(
        questId: quest.id,
        expectedQuestRevision: quest.revision,
        expectedModuleId: module.id,
        expectedModuleRevision: module.revision,
        expectedParentRuntimeClass: summary.parentRuntimeClass,
        expectedGiverRuntimeUniqueName: summary.giverRuntimeUniqueName,
      ),
      _loadCatalog(gameRoot),
    ]);
    final seed = loaded[0] as AuthoringRevision3QuestContextSeed;
    final catalog = loaded[1] as Revision3QuestCatalog;
    _requireSeedBinding(index, quest, seed);
    _requireCatalogSeal(catalog, index);
    final current = _requireExactCurrentCatalogBindings(seed, catalog);
    return Revision3QuestContextEditCheckpoint._(
      index: index,
      quest: quest,
      seed: seed,
      catalog: catalog,
      currentParent: current.parent,
      currentGiver: current.giver,
    );
  }

  Future<Revision3QuestContextEditPublication> publish({
    required Revision3QuestContextEditCheckpoint checkpoint,
    required String gameRoot,
    required String description,
    required Revision3QuestParentChoice parent,
    required Revision3QuestGiverChoice giver,
  }) async {
    final problem = validateDescription(description);
    if (problem != null) throw FormatException(problem);
    _requireCatalogSelection(checkpoint.catalog, parent, giver);
    if (description == checkpoint.seed.description &&
        parent.catalogId == checkpoint.currentParent.catalogId &&
        giver.catalogId == checkpoint.currentGiver.catalogId) {
      throw const FormatException('Change at least one Quest detail.');
    }

    final fresh = await _loadCatalog(gameRoot);
    _requireCatalogSeal(fresh, checkpoint.index);
    _requireExactCurrentCatalogBindings(checkpoint.seed, fresh);
    final freshParent = fresh.parent(parent.catalogId);
    final freshGiver = fresh.giver(giver.catalogId);
    if (!checkpoint.catalog.sameSeal(fresh) ||
        freshParent == null ||
        freshGiver == null ||
        !_sameParentBinding(parent, freshParent) ||
        !_sameGiverBinding(giver, freshGiver)) {
      throw Revision3QuestContextCatalogDriftException(fresh);
    }
    final seal = fresh.catalogSeal!;
    return _publishTechnicalPlan(
      gameRoot: gameRoot,
      plan: Revision3QuestContextEditTechnicalPlan._(
        questId: checkpoint.seed.questId,
        expectedQuestRevision: checkpoint.seed.questRevision,
        moduleId: checkpoint.seed.moduleId,
        expectedModuleRevision: checkpoint.seed.moduleRevision,
        expectedStoryCatalogSeal: seal,
        description: description,
        parentCatalogId: freshParent.catalogId,
        giverCatalogId: freshGiver.catalogId,
        expectedParentRuntimeClass: freshParent.runtimeClass,
        expectedParentCatalogLayer: freshParent.catalogLayer,
        expectedParentAuthoringSelector: freshParent.authoringSelector,
        expectedParentSourceSeal: freshParent.sourceSeal,
        expectedGiverRuntimeUniqueName: freshGiver.runtimeUniqueName,
        expectedGiverCatalogLayer: freshGiver.catalogLayer,
        expectedGiverAuthoringSelector: freshGiver.authoringSelector,
        expectedGiverSourceSeal: freshGiver.sourceSeal,
      ),
    );
  }

  static String? validateDescription(String value) {
    if (value.isEmpty || value.trim() != value) {
      return 'Enter a Quest description without spaces at the beginning or end.';
    }
    if (utf8.encode(value).length > 512) {
      return 'The Quest description must be at most 512 bytes.';
    }
    if (value.runes.any(
      (rune) => rune < 0x20 || rune > 0x7e || rune == 0x22 || rune == 0x5c,
    )) {
      return 'The description currently supports plain ASCII text without line breaks, quotes, or backslashes.';
    }
    return null;
  }
}

({Revision3QuestParentChoice parent, Revision3QuestGiverChoice giver})
_requireExactCurrentCatalogBindings(
  AuthoringRevision3QuestContextSeed seed,
  Revision3QuestCatalog catalog,
) {
  final parent = catalog.parentForRuntimeClass(seed.parentRuntimeClass);
  final giver = catalog.giverForRuntimeUniqueName(seed.giverRuntimeUniqueName);
  if (parent == null ||
      giver == null ||
      parent.catalogLayer != seed.parentCatalogLayer ||
      parent.authoringSelector != seed.parentAuthoringSelector ||
      !_sameSeal(parent.sourceSeal, seed.parentSourceSeal) ||
      giver.catalogLayer != seed.giverCatalogLayer ||
      giver.authoringSelector != seed.giverAuthoringSelector ||
      !_sameSeal(giver.sourceSeal, seed.giverSourceSeal)) {
    throw const Revision3QuestContextUnavailableException();
  }
  return (parent: parent, giver: giver);
}

void _requireVisibleQuest(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
) {
  if (quest.kind != Revision3ContentEntityKind.questDraft ||
      quest.summary.questDraft == null ||
      index.entityById(quest.id) != quest) {
    throw const FormatException(
      'The selected item is not the exact Quest from this project view.',
    );
  }
}

void _requireSeedBinding(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
  AuthoringRevision3QuestContextSeed seed,
) {
  final summary = quest.summary.questDraft!;
  final module = _requireVisibleQuestModule(index, quest);
  if (seed.projectId != index.projectId ||
      seed.projectRevision != index.projectRevision ||
      seed.questId != quest.id ||
      seed.questRevision != quest.revision ||
      seed.moduleId != module.id ||
      seed.moduleRevision != module.revision ||
      seed.parentRuntimeClass != summary.parentRuntimeClass ||
      seed.giverRuntimeUniqueName != summary.giverRuntimeUniqueName) {
    throw const Revision3QuestContextStaleCheckpointException();
  }
}

Revision3ContentEntity _requireVisibleQuestModule(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
) {
  final modules = quest.references
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
  final module = modules.length == 1
      ? index.entityById(modules.single.target.entityId)
      : null;
  if (module == null ||
      module.kind != Revision3ContentEntityKind.scriptModule) {
    throw const FormatException(
      'The selected Quest does not own one exact generated script.',
    );
  }
  return module;
}

void _requireCatalogSeal(
  Revision3QuestCatalog catalog,
  Revision3ContentIndex index,
) {
  final generation = catalog.generationExecutableSeal;
  if (catalog.catalogSeal == null || generation == null) {
    throw const FormatException(
      'The fresh Story catalog has no exact content and generation seals.',
    );
  }
  if (generation.byteLength != index.targetExecutableByteLength ||
      generation.sha256 != index.targetExecutableSha256) {
    throw const Revision3QuestContextUnavailableException();
  }
}

void _requireCatalogSelection(
  Revision3QuestCatalog catalog,
  Revision3QuestParentChoice parent,
  Revision3QuestGiverChoice giver,
) {
  final boundParent = catalog.parent(parent.catalogId);
  final boundGiver = catalog.giver(giver.catalogId);
  if (boundParent == null ||
      boundGiver == null ||
      !_sameParentBinding(parent, boundParent) ||
      !_sameGiverBinding(giver, boundGiver)) {
    throw const FormatException(
      'Choose a Quest family and giver from the current game catalog.',
    );
  }
}

bool _sameParentBinding(
  Revision3QuestParentChoice left,
  Revision3QuestParentChoice right,
) =>
    left.catalogId == right.catalogId &&
    left.runtimeClass == right.runtimeClass &&
    left.catalogLayer == right.catalogLayer &&
    left.authoringSelector == right.authoringSelector &&
    _sameSeal(left.sourceSeal, right.sourceSeal) &&
    left.displayLabel == right.displayLabel;

bool _sameGiverBinding(
  Revision3QuestGiverChoice left,
  Revision3QuestGiverChoice right,
) =>
    left.catalogId == right.catalogId &&
    left.runtimeUniqueName == right.runtimeUniqueName &&
    left.catalogLayer == right.catalogLayer &&
    left.authoringSelector == right.authoringSelector &&
    _sameSeal(left.sourceSeal, right.sourceSeal) &&
    left.displayLabel == right.displayLabel;

bool _sameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;
