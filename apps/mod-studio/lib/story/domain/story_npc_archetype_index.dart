import '../../core/mod_ffi.dart';

enum StoryNpcArchetypeQualification {
  offlineCloneQualified,
  sealedLinkageVerifiedExperimental,
}

final class StoryNpcArchetypeIndexException implements Exception {
  const StoryNpcArchetypeIndexException(this.message);

  final String message;

  @override
  String toString() => 'StoryNpcArchetypeIndexException: $message';
}

/// One immutable picker row projected from native static-linkage evidence.
///
/// A non-curated row deliberately uses its exact spawn class as [label]. It does not infer a
/// display name, NPC role, or sex from technical naming conventions. [bodyBlueprintFamilyLabel]
/// describes only the native actor Blueprint family.
final class StoryNpcArchetypeRow {
  StoryNpcArchetypeRow._({
    required this.label,
    required this.spawnClass,
    required this.aiConfigClass,
    required this.characterDefinitionClass,
    required this.actorBlueprint,
    required this.bodyBlueprintFamily,
    required this.qualification,
    required this.curatedCatalogId,
    required this.curatedDisplayName,
    required this.curatedRuntimeUniqueName,
  }) : _searchText = <String>[
         ?curatedDisplayName,
         ?curatedRuntimeUniqueName,
         spawnClass,
         aiConfigClass,
         characterDefinitionClass,
         actorBlueprint,
       ].map(_normalize).join('\u0000');

  final String label;
  final String spawnClass;
  final String aiConfigClass;
  final String characterDefinitionClass;
  final String actorBlueprint;
  final AuthoringNpcCatalogBlueprintFamily bodyBlueprintFamily;
  final StoryNpcArchetypeQualification qualification;
  final String? curatedCatalogId;
  final String? curatedDisplayName;
  final String? curatedRuntimeUniqueName;
  final String _searchText;

  bool get selectable =>
      qualification == StoryNpcArchetypeQualification.offlineCloneQualified;

  bool get experimental =>
      qualification ==
      StoryNpcArchetypeQualification.sealedLinkageVerifiedExperimental;

  String get bodyBlueprintFamilyLabel => switch (bodyBlueprintFamily) {
    AuthoringNpcCatalogBlueprintFamily.humanBase =>
      'Human base body/blueprint family',
    AuthoringNpcCatalogBlueprintFamily.humanWoman =>
      'Human woman body/blueprint family',
    AuthoringNpcCatalogBlueprintFamily.other => 'Other body/blueprint family',
  };

  bool _matches(List<String> terms) =>
      terms.every((term) => _searchText.contains(term));
}

/// Pure, generation-bound picker index over the native NPC archetype catalog.
///
/// It exposes no draft creation or trusted selector construction. Selection maps only to a
/// curated Story catalog ID after an exact class-and-source match.
final class StoryNpcArchetypeIndex {
  StoryNpcArchetypeIndex._({
    required List<StoryNpcArchetypeRow> rows,
    required Map<String, StoryNpcArchetypeRow> selectableByCatalogId,
  }) : rows = List<StoryNpcArchetypeRow>.unmodifiable(rows),
       _selectableByCatalogId = Map<String, StoryNpcArchetypeRow>.unmodifiable(
         selectableByCatalogId,
       );

  factory StoryNpcArchetypeIndex.fromCatalogs({
    required AuthoringStoryCatalogSelections story,
    required AuthoringNpcArchetypeCatalogBuildResult archetypes,
  }) {
    if (!_sameGeneration(story.generation, archetypes.generation)) {
      throw const StoryNpcArchetypeIndexException(
        'NPC archetypes and Story selections target different generations',
      );
    }
    if (!_sameSeal(story.catalogSeal, archetypes.storyCatalogSeal)) {
      throw const StoryNpcArchetypeIndexException(
        'NPC archetypes are not bound to the selected Story catalog',
      );
    }

    final rows = <StoryNpcArchetypeRow>[];
    final selectableByCatalogId = <String, StoryNpcArchetypeRow>{};
    for (final record in archetypes.records) {
      AuthoringStoryCatalogNpcSelection? curated;
      for (final selection in story.npcs) {
        if (!_matchesCuratedRecord(record, selection)) continue;
        if (curated != null) {
          throw const StoryNpcArchetypeIndexException(
            'one NPC archetype ambiguously matches multiple curated selections',
          );
        }
        curated = selection;
      }
      final selectable = curated != null;
      final row = StoryNpcArchetypeRow._(
        label: curated?.displayName ?? record.spawn.className,
        spawnClass: record.spawn.className,
        aiConfigClass: record.aiConfig.className,
        characterDefinitionClass: record.characterDefinition.className,
        actorBlueprint: record.actorBlueprint,
        bodyBlueprintFamily: record.blueprintFamily,
        qualification: selectable
            ? StoryNpcArchetypeQualification.offlineCloneQualified
            : StoryNpcArchetypeQualification.sealedLinkageVerifiedExperimental,
        curatedCatalogId: curated?.catalogId,
        curatedDisplayName: curated?.displayName,
        curatedRuntimeUniqueName: curated?.runtimeUniqueName,
      );
      rows.add(row);
      if (curated != null) {
        if (selectableByCatalogId.putIfAbsent(curated.catalogId, () => row) !=
            row) {
          throw const StoryNpcArchetypeIndexException(
            'multiple NPC archetypes match one curated selection',
          );
        }
      }
    }
    return StoryNpcArchetypeIndex._(
      rows: rows,
      selectableByCatalogId: selectableByCatalogId,
    );
  }

  final List<StoryNpcArchetypeRow> rows;
  final Map<String, StoryNpcArchetypeRow> _selectableByCatalogId;

  StoryNpcArchetypeRow? selectableForCatalogId(String catalogId) =>
      _selectableByCatalogId[catalogId];

  List<StoryNpcArchetypeRow> search(
    String query, {
    bool includeExperimental = false,
    int? limit,
  }) => _searchRows(
    query,
    include: (row) => includeExperimental || row.selectable,
    limit: limit,
  );

  /// Searches only inspect-only linkage evidence with bounded allocation.
  List<StoryNpcArchetypeRow> searchExperimental(String query, {int? limit}) =>
      _searchRows(query, include: (row) => row.experimental, limit: limit);

  List<StoryNpcArchetypeRow> _searchRows(
    String query, {
    required bool Function(StoryNpcArchetypeRow row) include,
    int? limit,
  }) {
    if (limit != null && limit < 0) {
      throw RangeError.range(limit, 0, null, 'limit');
    }
    if (limit == 0) return const <StoryNpcArchetypeRow>[];
    final normalized = _normalize(query).trim();
    final terms = normalized.isEmpty
        ? const <String>[]
        : normalized.split(RegExp(r'\s+'));
    final matches = <StoryNpcArchetypeRow>[];
    for (final row in rows) {
      if (!include(row) || !row._matches(terms)) continue;
      matches.add(row);
      if (matches.length == limit) break;
    }
    return List<StoryNpcArchetypeRow>.unmodifiable(matches);
  }
}

bool _matchesCuratedRecord(
  AuthoringNpcCatalogRecord record,
  AuthoringStoryCatalogNpcSelection curated,
) =>
    record.spawn.className == curated.spawnDefinition.runtimeClass &&
    _sameSeal(record.spawn.sourceSeal, curated.spawnDefinition.sourceSeal) &&
    record.aiConfig.className == curated.aiAgentConfig.runtimeClass &&
    _sameSeal(record.aiConfig.sourceSeal, curated.aiAgentConfig.sourceSeal) &&
    record.characterDefinition.className ==
        curated.characterDefinition.runtimeClass &&
    _sameSeal(
      record.characterDefinition.sourceSeal,
      curated.characterDefinition.sourceSeal,
    );

bool _sameGeneration(
  AuthoringStoryCatalogGeneration left,
  AuthoringStoryCatalogGeneration right,
) =>
    left.edition == right.edition &&
    _sameSeal(left.executable, right.executable) &&
    _sameSeal(left.shippingCache, right.shippingCache) &&
    _sameSeal(left.bindsCache, right.bindsCache);

bool _sameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

String _normalize(String value) => value.toLowerCase();
