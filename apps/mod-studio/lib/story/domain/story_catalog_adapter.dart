import 'dart:convert';

import '../../core/mod_ffi.dart';
import 'story_draft_requests.dart';

const int _maxNpcDisplayNameBytes = 256;
const int _maxNpcModuleNamespaceBytes = 255;
const int _maxNpcUniqueNameBytes = 64;

/// Fail-closed boundary between a verified native catalog DTO and Story input.
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

enum StoryQuestDraftDisabledReason { collisionInventoryUnavailable }

/// Typed Quest chooser state. It intentionally has no creation method. Until a
/// sealed collision-inventory capability exists, the public Story domain also
/// exposes no Quest input, mutation-builder, or managed-controller API.
final class StoryCatalogQuestDraftAvailability {
  const StoryCatalogQuestDraftAvailability._({
    required this.disabledReason,
    required this.parents,
    required this.givers,
    required this.collisionCatalogLayer,
    required this.collisionSourceSeal,
  });

  bool get canCreate => false;
  final StoryQuestDraftDisabledReason disabledReason;
  final List<StoryCatalogQuestParentChoice> parents;
  final List<StoryCatalogQuestGiverChoice> givers;
  final String collisionCatalogLayer;
  final StoryCatalogContentSeal collisionSourceSeal;
}

/// Deterministic projection of the pinned native Story catalog for non-UI
/// authoring flows.
final class StoryCatalogAdapter {
  StoryCatalogAdapter._({
    required this.npcChoices,
    required this.questAvailability,
    required Map<String, AuthoringStoryCatalogNpcSelection> npcsById,
    required this._generation,
  }) : _npcsById = Map<String, AuthoringStoryCatalogNpcSelection>.unmodifiable(
         npcsById,
       );

  factory StoryCatalogAdapter.fromSelections(
    AuthoringStoryCatalogSelections selections,
  ) {
    _requireSupportedReadiness(selections);
    final npcsById = <String, AuthoringStoryCatalogNpcSelection>{};
    final npcChoices = <StoryCatalogNpcChoice>[];
    final giverChoices = <StoryCatalogQuestGiverChoice>[];
    for (final npc in selections.npcs) {
      if (npcsById.containsKey(npc.catalogId)) {
        throw const StoryCatalogAdapterException(
          'Story catalog contains a duplicate NPC choice',
        );
      }
      npcsById[npc.catalogId] = npc;
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
      questAvailability: StoryCatalogQuestDraftAvailability._(
        disabledReason:
            StoryQuestDraftDisabledReason.collisionInventoryUnavailable,
        parents: List<StoryCatalogQuestParentChoice>.unmodifiable(
          parentChoices,
        ),
        givers: List<StoryCatalogQuestGiverChoice>.unmodifiable(giverChoices),
        collisionCatalogLayer: selections.questCollisionCatalog.catalogLayer,
        collisionSourceSeal: _seal(selections.questCollisionCatalog.sourceSeal),
      ),
      npcsById: npcsById,
      generation: selections.generation,
    );
  }

  final List<StoryCatalogNpcChoice> npcChoices;
  final StoryCatalogQuestDraftAvailability questAvailability;
  final Map<String, AuthoringStoryCatalogNpcSelection> _npcsById;
  final AuthoringStoryCatalogGeneration _generation;

  StoryNpcDraftInput createNpcDraftInput({
    required String catalogId,
    required String displayName,
    required String moduleNamespace,
    required String uniqueName,
  }) {
    final selected = _npcsById[catalogId];
    if (selected == null) {
      throw StoryCatalogAdapterException(
        'unknown Story catalog NPC choice: $catalogId',
      );
    }
    _boundedUtf8(displayName, _maxNpcDisplayNameBytes, 'displayName');
    _boundedUtf8(
      moduleNamespace,
      _maxNpcModuleNamespaceBytes,
      'moduleNamespace',
    );
    _boundedUtf8(uniqueName, _maxNpcUniqueNameBytes, 'uniqueName');
    return StoryNpcDraftInput(
      displayName: displayName,
      moduleNamespace: moduleNamespace,
      uniqueName: uniqueName,
      parentCharacterDefinition: _npcParent(selected.characterDefinition),
      parentAiAgentConfig: _npcParent(selected.aiAgentConfig),
      parentSpawnDefinition: _npcParent(selected.spawnDefinition),
    );
  }

  CanonicalUnverifiedStoryJsonObject _npcParent(
    AuthoringStoryCatalogClassSelection selected,
  ) => CanonicalUnverifiedStoryJsonObject.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'generation': <String, Object?>{
        'executable': _sealJson(_generation.executable),
      },
      'source_seal': _sealJson(selected.sourceSeal),
      'catalog_layer': selected.catalogLayer,
      'canonical_selector': selected.authoringSelector,
      'runtime_class': selected.runtimeClass,
    }),
  );
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
  final collision = selections.questCollisionCatalog;
  if (collision.status !=
          AuthoringStoryCatalogCollisionStatus.inventoryUnavailable ||
      !collision.blocksDraftCreation) {
    throw const StoryCatalogAdapterException(
      'Quest creation must remain disabled without an exact collision inventory',
    );
  }
}

StoryCatalogContentSeal _seal(AuthoringDraftContentSeal seal) =>
    StoryCatalogContentSeal._(byteLength: seal.byteLength, sha256: seal.sha256);

Map<String, Object?> _sealJson(AuthoringDraftContentSeal seal) =>
    <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};

void _boundedUtf8(String value, int maxBytes, String context) {
  if (value.isEmpty) {
    throw StoryCatalogAdapterException('$context must not be empty');
  }
  var bytes = 0;
  for (var index = 0; index < value.length; index++) {
    final codeUnit = value.codeUnitAt(index);
    final int width;
    if (codeUnit <= 0x7f) {
      width = 1;
    } else if (codeUnit <= 0x7ff) {
      width = 2;
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw StoryCatalogAdapterException('$context has malformed UTF-16');
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw StoryCatalogAdapterException('$context has malformed UTF-16');
      }
      index++;
      width = 4;
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      throw StoryCatalogAdapterException('$context has malformed UTF-16');
    } else {
      width = 3;
    }
    bytes += width;
    if (bytes > maxBytes) {
      throw StoryCatalogAdapterException(
        '$context exceeds its $maxBytes-byte limit',
      );
    }
  }
}
