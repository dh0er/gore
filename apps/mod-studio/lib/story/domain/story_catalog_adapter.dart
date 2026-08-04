import '../../core/mod_ffi.dart';
import 'story_npc_archetype_index.dart';

/// Fail-closed boundary for projecting a verified native Story catalog.
final class StoryCatalogAdapterException implements Exception {
  const StoryCatalogAdapterException(this.message);

  final String message;

  @override
  String toString() => 'StoryCatalogAdapterException: $message';
}

final class StoryCatalogContentSeal {
  const StoryCatalogContentSeal._({
    required this.byteLength,
    required this.sha256,
  });

  final int byteLength;
  final String sha256;
}

/// Safe chooser row. Deliberately excludes the source catalog selector, which
/// may contain implementation paths and is not an authoring identity.
final class StoryCatalogNpcChoice {
  const StoryCatalogNpcChoice._({
    required this.catalogId,
    required this.displayName,
    required this.runtimeUniqueName,
    required this.authoringQualification,
    required this.runtimeQualification,
    required this.blocksBuild,
  });

  final String catalogId;
  final String displayName;
  final String runtimeUniqueName;
  final AuthoringStoryCatalogNpcAuthoringQualification authoringQualification;
  final AuthoringStoryCatalogRuntimeQualification runtimeQualification;
  final bool blocksBuild;
}

final class StoryCatalogQuestParentChoice {
  const StoryCatalogQuestParentChoice._({
    required this.catalogId,
    required this.displayName,
    required this.catalogLayer,
    required this.authoringSelector,
    required this.runtimeClass,
    required this.parentClassName,
    required this.sourceSeal,
  });

  final String catalogId;
  final String displayName;
  final String catalogLayer;
  final String authoringSelector;
  final String runtimeClass;
  final String parentClassName;
  final StoryCatalogContentSeal sourceSeal;
}

final class StoryCatalogQuestGiverChoice {
  const StoryCatalogQuestGiverChoice._({
    required this.catalogId,
    required this.displayName,
    required this.catalogLayer,
    required this.authoringSelector,
    required this.runtimeUniqueName,
    required this.sourceSeal,
  });

  final String catalogId;
  final String displayName;
  final String catalogLayer;
  final String authoringSelector;
  final String runtimeUniqueName;
  final StoryCatalogContentSeal sourceSeal;
}

/// Deterministic projection of the pinned native Story catalog for non-UI
/// authoring flows.
final class StoryCatalogAdapter {
  StoryCatalogAdapter._({
    required this.npcChoices,
    required this.questParents,
    required this.questGivers,
    required this.npcArchetypeIndex,
  });

  factory StoryCatalogAdapter.fromSelections(
    AuthoringStoryCatalogSelections selections,
  ) => _fromSelections(selections, null);

  /// Join the broad native archetype catalog only through its exact Story
  /// generation/catalog binding. Callers cannot attach an unrelated picker
  /// index to this adapter.
  factory StoryCatalogAdapter.fromSelectionsAndArchetypes(
    AuthoringStoryCatalogSelections selections,
    AuthoringNpcArchetypeCatalogBuildResult archetypes,
  ) => _fromSelections(
    selections,
    StoryNpcArchetypeIndex.fromCatalogs(
      story: selections,
      archetypes: archetypes,
    ),
  );

  static StoryCatalogAdapter _fromSelections(
    AuthoringStoryCatalogSelections selections,
    StoryNpcArchetypeIndex? npcArchetypeIndex,
  ) {
    _requireSupportedReadiness(selections);
    final npcIds = <String>{};
    final npcChoices = <StoryCatalogNpcChoice>[];
    final giverChoices = <StoryCatalogQuestGiverChoice>[];
    for (final npc in selections.npcs) {
      if (!npcIds.add(npc.catalogId)) {
        throw const StoryCatalogAdapterException(
          'Story catalog contains a duplicate NPC choice',
        );
      }
      npcChoices.add(
        StoryCatalogNpcChoice._(
          catalogId: npc.catalogId,
          displayName: npc.displayName,
          runtimeUniqueName: npc.runtimeUniqueName,
          authoringQualification: npc.authoringQualification,
          runtimeQualification: npc.runtimeQualification,
          blocksBuild: npc.blocksBuild,
        ),
      );
      giverChoices.add(
        StoryCatalogQuestGiverChoice._(
          catalogId: npc.catalogId,
          displayName: npc.displayName,
          catalogLayer: npc.questGiver.catalogLayer,
          authoringSelector: npc.questGiver.authoringSelector,
          runtimeUniqueName: npc.questGiver.runtimeUniqueName,
          sourceSeal: _seal(npc.questGiver.sourceSeal),
        ),
      );
    }
    final parentChoices = <StoryCatalogQuestParentChoice>[
      for (final parent in selections.questParents)
        StoryCatalogQuestParentChoice._(
          catalogId: parent.catalogId,
          displayName: parent.displayName,
          catalogLayer: parent.questClass.catalogLayer,
          authoringSelector: parent.questClass.authoringSelector,
          runtimeClass: parent.questClass.runtimeClass,
          parentClassName: parent.parentClassName,
          sourceSeal: _seal(parent.questClass.sourceSeal),
        ),
    ];
    return StoryCatalogAdapter._(
      npcChoices: List<StoryCatalogNpcChoice>.unmodifiable(npcChoices),
      questParents: List<StoryCatalogQuestParentChoice>.unmodifiable(
        parentChoices,
      ),
      questGivers: List<StoryCatalogQuestGiverChoice>.unmodifiable(
        giverChoices,
      ),
      npcArchetypeIndex: npcArchetypeIndex,
    );
  }

  final List<StoryCatalogNpcChoice> npcChoices;
  final List<StoryCatalogQuestParentChoice> questParents;
  final List<StoryCatalogQuestGiverChoice> questGivers;

  /// Full native archetype picker index when both native catalogs were loaded.
  ///
  /// Callers that only need the curated Story catalog may omit it.
  final StoryNpcArchetypeIndex? npcArchetypeIndex;
}

void _requireSupportedReadiness(AuthoringStoryCatalogSelections selections) {
  if (selections.schemaRevision != 1 || !selections.blocksBuild) {
    throw const StoryCatalogAdapterException(
      'Story catalog top-level readiness is unsupported',
    );
  }
  if (selections.npcs.isEmpty || selections.questParents.isEmpty) {
    throw const StoryCatalogAdapterException(
      'Story catalog chooser sets must not be empty',
    );
  }
  for (final npc in selections.npcs) {
    if (npc.discoveryStatus !=
            AuthoringStoryCatalogNpcDiscoveryStatus
                .sealedCacheDefaultsVerified ||
        npc.authoringQualification !=
            AuthoringStoryCatalogNpcAuthoringQualification.offlineQualified ||
        npc.runtimeQualification !=
            AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified ||
        !npc.blocksBuild) {
      throw StoryCatalogAdapterException(
        'Story catalog NPC ${npc.catalogId} has inconsistent readiness',
      );
    }
  }
  for (final parent in selections.questParents) {
    if (parent.role != AuthoringStoryCatalogQuestParentRole.chapter ||
        parent.qualification !=
            AuthoringStoryCatalogQuestParentQualification
                .curatedDefaultsVerified ||
        parent.transitionQualification !=
            AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified ||
        !parent.blocksBuild) {
      throw StoryCatalogAdapterException(
        'Story catalog Quest parent ${parent.catalogId} has inconsistent readiness',
      );
    }
  }
}

StoryCatalogContentSeal _seal(AuthoringDraftContentSeal seal) =>
    StoryCatalogContentSeal._(byteLength: seal.byteLength, sha256: seal.sha256);
