import 'dart:convert';

const _maxProjectJsonBytes = 16 * 1024 * 1024;
const _maxEntities = 100000;
const _maxAssets = 100000;
const _maxReferences = 1000000;
const _signedWireMax = 0x7fffffffffffffff;
const _signedWireMin = -0x7fffffffffffffff - 1;
const _maxItemPatchFields = 256;
const _maxItemFieldNameBytes = 128;
const _maxItemClassBytes = 256;
const _maxItemEnumTypeBytes = 256;
const _maxItemStringBytes = 16 * 1024;
const _maxItemStringTotalBytes = 64 * 1024;
const _npcGeneratorId = 'gore-authoring.logical-npc-clone-draft';
const _npcGeneratorVersion = 1;
const _questGeneratorId = 'gore-authoring.draft-quest-skeleton';
const _questGeneratorVersion = 4;
const _questCollisionMediaType =
    'application/vnd.gore.quest-collision-capability+json;version=2';

final _idPattern = RegExp(r'^[0-9a-f]{32}$');
final _shaPattern = RegExp(r'^[0-9a-f]{64}$');
final _itemIdentifierPattern = RegExp(r'^[A-Za-z_][A-Za-z0-9_]*$');

/// The managed content projection lost exact-current authority and must be
/// reopened before another catalog read or publication attempt.
final class Revision3ContentRequiresReopenException implements Exception {
  const Revision3ContentRequiresReopenException();
}

enum Revision3ContentEntityKind {
  localizationEntry('localization_entry', 'Localization'),
  dialogLine('dialog_line', 'Dialog line'),
  voiceSlot('voice_slot', 'Voice slot'),
  voiceTake('voice_take', 'Voice take'),
  npcDraft('npc_draft', 'NPC Draft'),
  questDraft('quest_draft', 'Quest Draft'),
  scriptModule('script_module', 'Generated script'),
  itemPatch('item_patch', 'Item edit');

  const Revision3ContentEntityKind(this.wireName, this.displayName);

  final String wireName;
  final String displayName;

  static Revision3ContentEntityKind parse(Object? value, String context) =>
      values.firstWhere(
        (kind) => kind.wireName == value,
        orElse: () => throw FormatException('$context has an unknown kind'),
      );
}

enum Revision3ContentReferenceResolution {
  resolved('resolved'),
  foreignProject('foreign_project'),
  missingEntity('missing_entity'),
  kindMismatch('kind_mismatch');

  const Revision3ContentReferenceResolution(this.wireName);
  final String wireName;

  static Revision3ContentReferenceResolution parse(
    Object? value,
    String context,
  ) => values.firstWhere(
    (item) => item.wireName == value,
    orElse: () => throw FormatException('$context has an invalid resolution'),
  );
}

enum Revision3ContentAssetReferenceResolution {
  resolved('resolved'),
  missingAsset('missing_asset'),
  byteLengthMismatch('byte_length_mismatch'),
  mediaTypeMismatch('media_type_mismatch');

  const Revision3ContentAssetReferenceResolution(this.wireName);
  final String wireName;

  static Revision3ContentAssetReferenceResolution parse(
    Object? value,
    String context,
  ) => values.firstWhere(
    (item) => item.wireName == value,
    orElse: () => throw FormatException('$context has an invalid resolution'),
  );
}

enum Revision3ContentAssetClass {
  voiceAudio('voice_audio', 'Voice audio'),
  questCollisionArtifact('quest_collision_artifact', 'Quest evidence'),
  dataAssetStageManifest('data_asset_stage_manifest', 'DataAsset stage'),
  dataAssetStageComponent('data_asset_stage_component', 'DataAsset component'),
  other('other', 'Other asset');

  const Revision3ContentAssetClass(this.wireName, this.displayName);
  final String wireName;
  final String displayName;

  static Revision3ContentAssetClass parse(Object? value, String context) =>
      values.firstWhere(
        (item) => item.wireName == value,
        orElse: () => throw FormatException('$context has an invalid class'),
      );
}

final class Revision3ContentIndex {
  Revision3ContentIndex._({
    required this.projectId,
    required this.projectRevision,
    required this.projectName,
    required this.projectVersion,
    required this.projectAuthor,
    required this.targetExecutableSha256,
    required this.targetExecutableByteLength,
    required this.authoringLocales,
    required this.entities,
    required this.assets,
  });

  final String projectId;
  final int projectRevision;
  final String projectName;
  final String projectVersion;
  final String projectAuthor;
  final String targetExecutableSha256;
  final int targetExecutableByteLength;
  final List<String> authoringLocales;
  final List<Revision3ContentEntity> entities;
  final List<Revision3ContentAsset> assets;

  int get problemCount =>
      entities.fold<int>(0, (total, entity) => total + entity.problemCount);

  late final Map<String, Revision3ContentEntity> _entitiesById =
      Map<String, Revision3ContentEntity>.unmodifiable({
        for (final entity in entities) entity.id: entity,
      });
  late final Map<String, Revision3ContentAsset> _assetsBySha256 =
      Map<String, Revision3ContentAsset>.unmodifiable({
        for (final asset in assets) asset.sha256: asset,
      });
  late final Map<String, List<Revision3ContentEntityBacklink>>
  _entityBacklinks = _buildEntityBacklinks();
  late final Map<String, List<Revision3ContentAssetBacklink>> _assetBacklinks =
      _buildAssetBacklinks();

  Revision3ContentEntity? entityById(String entityId) =>
      _entitiesById[entityId];

  Revision3ContentAsset? assetBySha256(String sha256) =>
      _assetsBySha256[sha256];

  List<Revision3ContentEntityBacklink> backlinksToEntity(String entityId) =>
      _entityBacklinks[entityId] ?? const <Revision3ContentEntityBacklink>[];

  List<Revision3ContentAssetBacklink> backlinksToAsset(String sha256) =>
      _assetBacklinks[sha256] ?? const <Revision3ContentAssetBacklink>[];

  Map<String, List<Revision3ContentEntityBacklink>> _buildEntityBacklinks() {
    final backlinks = <String, List<Revision3ContentEntityBacklink>>{};
    for (final source in entities) {
      for (final reference in source.references) {
        if (reference.target.projectId != projectId ||
            !_entitiesById.containsKey(reference.target.entityId)) {
          continue;
        }
        backlinks
            .putIfAbsent(
              reference.target.entityId,
              () => <Revision3ContentEntityBacklink>[],
            )
            .add(
              Revision3ContentEntityBacklink(
                source: source,
                reference: reference,
              ),
            );
      }
    }
    return Map<String, List<Revision3ContentEntityBacklink>>.unmodifiable({
      for (final entry in backlinks.entries)
        entry.key: List<Revision3ContentEntityBacklink>.unmodifiable(
          entry.value,
        ),
    });
  }

  Map<String, List<Revision3ContentAssetBacklink>> _buildAssetBacklinks() {
    final backlinks = <String, List<Revision3ContentAssetBacklink>>{};
    for (final source in entities) {
      for (final reference in source.assetReferences) {
        if (!_assetsBySha256.containsKey(reference.sha256)) continue;
        backlinks
            .putIfAbsent(
              reference.sha256,
              () => <Revision3ContentAssetBacklink>[],
            )
            .add(
              Revision3ContentAssetBacklink(
                source: source,
                reference: reference,
              ),
            );
      }
    }
    return Map<String, List<Revision3ContentAssetBacklink>>.unmodifiable({
      for (final entry in backlinks.entries)
        entry.key: List<Revision3ContentAssetBacklink>.unmodifiable(
          entry.value,
        ),
    });
  }

  factory Revision3ContentIndex.fromJsonObject(Map<String, Object?> json) {
    _requireKeys(json, const <String>[
      'schema_revision',
      'project_id',
      'project_revision',
      'project_name',
      'project_version',
      'project_author',
      'target',
      'authoring_locales',
      'entity_counts',
      'entities',
      'assets',
    ], 'revision-3 content index');
    if (json['schema_revision'] != 1) {
      throw const FormatException(
        'revision-3 content index has an unsupported schema',
      );
    }
    final projectId = _id(json['project_id'], 'content project_id');
    if (projectId == '00000000000000000000000000000000') {
      throw const FormatException('content project_id must not be zero');
    }
    final projectRevision = _integer(
      json['project_revision'],
      'content project_revision',
    );
    final target = _object(json['target'], 'content target');
    _requireKeys(target, const <String>['executable'], 'content target');
    final executable = _seal(target['executable'], 'content target executable');
    if (executable.byteLength == 0) {
      throw const FormatException(
        'content target executable must not be empty',
      );
    }

    final authoringLocales = _stringList(
      json['authoring_locales'],
      'content authoring_locales',
      maxItems: 1000,
      maxStringBytes: 64,
    );
    _requireSortedUnique(authoringLocales, 'content authoring_locales');

    final rawEntities = _list(
      json['entities'],
      'content entities',
      maxItems: _maxEntities,
    );
    final entities = rawEntities
        .mapIndexed(
          (index, value) => Revision3ContentEntity._fromJson(
            _object(value, 'content entity $index'),
            index,
          ),
        )
        .toList(growable: false);
    _requireSortedUnique(
      entities.map((entity) => entity.id).toList(growable: false),
      'content entity IDs',
    );

    final rawAssets = _list(
      json['assets'],
      'content assets',
      maxItems: _maxAssets,
    );
    final assets = rawAssets
        .mapIndexed(
          (index, value) => Revision3ContentAsset._fromJson(
            _object(value, 'content asset $index'),
            index,
          ),
        )
        .toList(growable: false);
    _requireSortedUnique(
      assets.map((asset) => asset.sha256).toList(growable: false),
      'content asset hashes',
    );

    final rawCounts = _object(json['entity_counts'], 'content entity_counts');
    var previousKindIndex = -1;
    final declaredCounts = <Revision3ContentEntityKind, int>{};
    for (final entry in rawCounts.entries) {
      final kind = Revision3ContentEntityKind.parse(
        entry.key,
        'content entity_counts',
      );
      if (kind.index <= previousKindIndex || declaredCounts.containsKey(kind)) {
        throw const FormatException(
          'content entity_counts are not unique canonical kind order',
        );
      }
      previousKindIndex = kind.index;
      final count = _integer(entry.value, 'content count ${entry.key}');
      if (count == 0) {
        throw const FormatException('content entity_counts must omit zeroes');
      }
      declaredCounts[kind] = count;
    }
    final actualCounts = <Revision3ContentEntityKind, int>{};
    for (final entity in entities) {
      actualCounts.update(entity.kind, (value) => value + 1, ifAbsent: () => 1);
    }
    if (!_equalCounts(declaredCounts, actualCounts)) {
      throw const FormatException(
        'content entity_counts disagree with projected entities',
      );
    }

    final byEntityId = {for (final entity in entities) entity.id: entity};
    final byAssetHash = {for (final asset in assets) asset.sha256: asset};
    final itemPatchTargets = <String>{};
    var referenceCount = 0;
    for (final entity in entities) {
      referenceCount += entity.references.length;
      if (referenceCount > _maxReferences) {
        throw const FormatException('content reference graph is too large');
      }
      for (final reference in entity.references) {
        final expected = _resolveReference(projectId, byEntityId, reference);
        if (reference.resolution != expected) {
          throw FormatException(
            'content reference ${entity.id}/${reference.role} has false resolution',
          );
        }
      }
      for (final reference in entity.assetReferences) {
        final expected = _resolveAssetReference(byAssetHash, reference);
        if (reference.resolution != expected) {
          throw FormatException(
            'content asset reference ${entity.id}/${reference.role} has false resolution',
          );
        }
      }
      final owner = entity.origin.generatedOwner;
      if (owner != null &&
          !entity.references.any(
            (reference) =>
                reference.role == 'origin_owner' && reference.target == owner,
          )) {
        throw FormatException(
          'generated content entity ${entity.id} has no exact origin_owner reference',
        );
      }
      _validateDialogLineProjectionFacts(entity);
      _validateVoiceSlotProjectionFacts(entity);
      _validateScriptModuleProjectionFacts(
        entity,
        projectId: projectId,
        entitiesById: byEntityId,
      );
      _validateNpcProjectionFacts(
        entity,
        projectId: projectId,
        entitiesById: byEntityId,
      );
      _validateQuestProjectionFacts(
        entity,
        projectId: projectId,
        entitiesById: byEntityId,
      );
      _validateItemPatchProjectionFacts(
        entity,
        targetExecutable: Revision3ContentSeal(
          sha256: executable.sha256,
          byteLength: executable.byteLength,
        ),
      );
      if (entity.kind == Revision3ContentEntityKind.itemPatch &&
          !itemPatchTargets.add(entity.summary.itemPatch!.vanillaClass)) {
        throw FormatException(
          'content ItemPatch ${entity.id} duplicates a vanilla target',
        );
      }
    }

    return Revision3ContentIndex._(
      projectId: projectId,
      projectRevision: projectRevision,
      projectName: _string(
        json['project_name'],
        'content project_name',
        allowEmpty: true,
      ),
      projectVersion: _string(
        json['project_version'],
        'content project_version',
        allowEmpty: true,
      ),
      projectAuthor: _string(
        json['project_author'],
        'content project_author',
        allowEmpty: true,
      ),
      targetExecutableSha256: executable.sha256,
      targetExecutableByteLength: executable.byteLength,
      authoringLocales: List<String>.unmodifiable(authoringLocales),
      entities: List<Revision3ContentEntity>.unmodifiable(entities),
      assets: List<Revision3ContentAsset>.unmodifiable(assets),
    );
  }
}

/// One reverse edge derived from a validated typed entity reference.
final class Revision3ContentEntityBacklink {
  const Revision3ContentEntityBacklink({
    required this.source,
    required this.reference,
  });

  final Revision3ContentEntity source;
  final Revision3ContentReference reference;
}

/// One reverse edge derived from a validated AssetStore reference.
final class Revision3ContentAssetBacklink {
  const Revision3ContentAssetBacklink({
    required this.source,
    required this.reference,
  });

  final Revision3ContentEntity source;
  final Revision3ContentAssetReference reference;
}

final class Revision3ContentEntity {
  const Revision3ContentEntity._({
    required this.id,
    required this.kind,
    required this.displayName,
    required this.revision,
    required this.origin,
    required this.summary,
    required this.references,
    required this.assetReferences,
  });

  final String id;
  final Revision3ContentEntityKind kind;
  final String displayName;
  final int revision;
  final Revision3ContentOrigin origin;
  final Revision3ContentSummary summary;
  final List<Revision3ContentReference> references;
  final List<Revision3ContentAssetReference> assetReferences;

  int get problemCount =>
      references
          .where(
            (item) =>
                item.resolution != Revision3ContentReferenceResolution.resolved,
          )
          .length +
      assetReferences
          .where(
            (item) =>
                item.resolution !=
                Revision3ContentAssetReferenceResolution.resolved,
          )
          .length;

  bool matches(String foldedQuery) {
    if (foldedQuery.isEmpty) return true;
    return <String>[
      displayName,
      id,
      kind.displayName,
      origin.label,
      ...summary.searchTerms,
    ].any((value) => value.toLowerCase().contains(foldedQuery));
  }

  factory Revision3ContentEntity._fromJson(
    Map<String, Object?> json,
    int index,
  ) {
    final context = 'content entity $index';
    _requireKeys(json, const <String>[
      'id',
      'kind',
      'display_name',
      'revision',
      'origin',
      'summary',
      'references',
      'asset_references',
    ], context);
    final kind = Revision3ContentEntityKind.parse(json['kind'], context);
    final rawReferences = _list(
      json['references'],
      '$context references',
      maxItems: _maxReferences,
    );
    final references = rawReferences
        .mapIndexed(
          (refIndex, value) => Revision3ContentReference._fromJson(
            _object(value, '$context reference $refIndex'),
            '$context reference $refIndex',
          ),
        )
        .toList(growable: false);
    final rawAssetReferences = _list(
      json['asset_references'],
      '$context asset_references',
      maxItems: _maxAssets,
    );
    final assetReferences = rawAssetReferences
        .mapIndexed(
          (refIndex, value) => Revision3ContentAssetReference._fromJson(
            _object(value, '$context asset reference $refIndex'),
            '$context asset reference $refIndex',
          ),
        )
        .toList(growable: false);
    return Revision3ContentEntity._(
      id: _id(json['id'], '$context id'),
      kind: kind,
      displayName: _string(
        json['display_name'],
        '$context display_name',
        allowEmpty: true,
      ),
      revision: _integer(json['revision'], '$context revision'),
      origin: Revision3ContentOrigin._fromJson(
        _object(json['origin'], '$context origin'),
        '$context origin',
      ),
      summary: Revision3ContentSummary._fromJson(
        _object(json['summary'], '$context summary'),
        kind,
        '$context summary',
      ),
      references: List<Revision3ContentReference>.unmodifiable(references),
      assetReferences: List<Revision3ContentAssetReference>.unmodifiable(
        assetReferences,
      ),
    );
  }
}

final class Revision3ContentSeal {
  const Revision3ContentSeal({required this.sha256, required this.byteLength});

  final String sha256;
  final int byteLength;
}

final class Revision3ContentOrigin {
  const Revision3ContentOrigin._({
    required this.type,
    required this.label,
    this.generatedOwner,
    this.generatorVersion,
    this.generationExecutable,
    this.catalogLayer,
    this.sourceSeal,
  });

  final String type;
  final String label;
  final Revision3ContentReferenceTarget? generatedOwner;
  final int? generatorVersion;
  final Revision3ContentSeal? generationExecutable;
  final String? catalogLayer;
  final Revision3ContentSeal? sourceSeal;

  factory Revision3ContentOrigin._fromJson(
    Map<String, Object?> json,
    String context,
  ) {
    final type = _string(json['type'], '$context type');
    switch (type) {
      case 'new':
        _requireKeys(json, const ['type', 'authored_runtime_id'], context);
        return Revision3ContentOrigin._(
          type: type,
          label: _string(
            json['authored_runtime_id'],
            '$context authored_runtime_id',
            allowEmpty: true,
          ),
        );
      case 'vanilla':
        _requireKeys(json, const [
          'type',
          'generation',
          'catalog_layer',
          'canonical_selector',
          'source_seal',
        ], context);
        final generation = _generation(
          json['generation'],
          '$context generation',
        );
        final sourceSeal = _seal(json['source_seal'], '$context source_seal');
        final catalogLayer = _string(
          json['catalog_layer'],
          '$context catalog_layer',
        );
        return Revision3ContentOrigin._(
          type: type,
          label: _string(
            json['canonical_selector'],
            '$context canonical_selector',
          ),
          generationExecutable: generation,
          catalogLayer: catalogLayer,
          sourceSeal: sourceSeal,
        );
      case 'imported':
        _requireKeys(json, const [
          'type',
          'importer',
          'source_seal',
          'external_identity',
        ], context);
        final sourceSeal = _seal(json['source_seal'], '$context source_seal');
        final importer = _string(json['importer'], '$context importer');
        final external = _nullableString(
          json['external_identity'],
          '$context external_identity',
        );
        return Revision3ContentOrigin._(
          type: type,
          label: external ?? importer,
          sourceSeal: sourceSeal,
        );
      case 'generated':
        _requireKeys(json, const [
          'type',
          'generator_id',
          'generator_version',
          'owner',
        ], context);
        final generator = _string(
          json['generator_id'],
          '$context generator_id',
        );
        final generatorVersion = _integer(
          json['generator_version'],
          '$context generator_version',
        );
        final owner = Revision3ContentReferenceTarget._fromJson(
          _object(json['owner'], '$context owner'),
          '$context owner',
        );
        return Revision3ContentOrigin._(
          type: type,
          label: generator,
          generatedOwner: owner,
          generatorVersion: generatorVersion,
        );
      default:
        throw FormatException('$context has an unsupported type');
    }
  }
}

enum Revision3ContentVoiceTargetResolution {
  unresolved,
  ambiguous,
  resolved;

  static Revision3ContentVoiceTargetResolution parse(
    Object? value,
    String context,
  ) => Revision3ContentVoiceTargetResolution.values.byName(
    _enumString(value, const {'unresolved', 'ambiguous', 'resolved'}, context),
  );
}

/// Structured VoiceSlot facts retained from the validated wire projection.
/// UI and authoring logic must not recover these facts from presentation text.
final class Revision3ContentVoiceSlotSummary {
  const Revision3ContentVoiceSlotSummary({
    required this.targetResolution,
    required this.candidateCount,
    required this.hasSelectedTake,
  });

  final Revision3ContentVoiceTargetResolution targetResolution;
  final int candidateCount;
  final bool hasSelectedTake;
}

enum Revision3ContentVoiceTakeStatus {
  draft,
  recorded,
  reviewed,
  approved;

  static Revision3ContentVoiceTakeStatus parse(Object? value, String context) =>
      Revision3ContentVoiceTakeStatus.values.byName(
        _enumString(value, const {
          'draft',
          'recorded',
          'reviewed',
          'approved',
        }, context),
      );
}

enum Revision3ContentVoiceOggCodec {
  vorbis,
  opus;

  static Revision3ContentVoiceOggCodec parse(Object? value, String context) =>
      Revision3ContentVoiceOggCodec.values.byName(
        _enumString(value, const {'vorbis', 'opus'}, context),
      );
}

/// Structured VoiceTake facts used for fail-closed authoring decisions.
final class Revision3ContentVoiceTakeSummary {
  const Revision3ContentVoiceTakeSummary({
    required this.locale,
    required this.status,
    required this.codec,
    required this.channels,
    required this.sampleRate,
  });

  final String locale;
  final Revision3ContentVoiceTakeStatus status;
  final Revision3ContentVoiceOggCodec codec;
  final int channels;
  final int sampleRate;
}

/// Structured DialogLine facts bound to its exact projected slot references.
final class Revision3ContentDialogLineSummary {
  const Revision3ContentDialogLineSummary({
    required this.speaker,
    required this.voiceSlotLocales,
  });

  final String? speaker;
  final List<String> voiceSlotLocales;
}

/// Structured LocalizationEntry facts retained from the validated wire
/// projection. Authoring code must not recover locales from presentation text.
final class Revision3ContentLocalizationEntrySummary {
  Revision3ContentLocalizationEntrySummary({required List<String> locales})
    : locales = List<String>.unmodifiable(locales);

  final List<String> locales;
}

/// Structured QuestDraft facts retained from the exact-current projection.
///
/// These values are the safe authoring seed for the outline editor. In
/// particular, objective order is semantic and must never be reconstructed
/// from presentation strings or reference ordering.
final class Revision3ContentQuestDraftSummary {
  Revision3ContentQuestDraftSummary({
    required this.technicalId,
    required this.title,
    required List<String> objectiveTitles,
    required List<int> objectiveSlots,
    required this.transcriptCount,
    required this.moduleNamespace,
    required this.parentRuntimeClass,
    required this.giverRuntimeUniqueName,
  }) : objectiveTitles = List<String>.unmodifiable(objectiveTitles),
       objectiveSlots = List<int>.unmodifiable(objectiveSlots);

  final String technicalId;
  final String title;
  final List<String> objectiveTitles;
  final List<int> objectiveSlots;
  final int transcriptCount;
  final String moduleNamespace;
  final String parentRuntimeClass;
  final String giverRuntimeUniqueName;
}

/// Exact display-safe projection of the persisted NPC Draft input.
///
/// Parent order is semantic and is retained explicitly so profile authoring
/// never reconstructs the three-class chain from presentation/search text.
final class Revision3ContentNpcDraftSummary {
  const Revision3ContentNpcDraftSummary({
    required this.uniqueName,
    required this.moduleNamespace,
    required this.parentCharacterDefinition,
    required this.parentAiAgentConfig,
    required this.parentSpawnDefinition,
    required this.greetingCount,
  });

  final String uniqueName;
  final String moduleNamespace;
  final String parentCharacterDefinition;
  final String parentAiAgentConfig;
  final String parentSpawnDefinition;
  final int greetingCount;
}

enum Revision3ContentItemScalarType {
  integer('integer'),
  float_('float'),
  boolean('boolean'),
  string('string'),
  enum_('enum');

  const Revision3ContentItemScalarType(this.wireName);

  final String wireName;

  static Revision3ContentItemScalarType parse(Object? value, String context) =>
      values.firstWhere(
        (type) => type.wireName == value,
        orElse: () =>
            throw FormatException('$context has an invalid scalar type'),
      );
}

final class Revision3ContentItemEnumValue {
  const Revision3ContentItemEnumValue({
    required this.enumType,
    required this.backing,
  });

  final String enumType;
  final int backing;
}

/// One closed, independently validated scalar from a managed ItemPatch.
final class Revision3ContentItemScalarValue {
  const Revision3ContentItemScalarValue._({
    required this.type,
    this.integerValue,
    this.floatValue,
    this.booleanValue,
    this.stringValue,
    this.enumValue,
  });

  final Revision3ContentItemScalarType type;
  final int? integerValue;
  final double? floatValue;
  final bool? booleanValue;
  final String? stringValue;
  final Revision3ContentItemEnumValue? enumValue;

  Object get value => switch (type) {
    Revision3ContentItemScalarType.integer => integerValue!,
    Revision3ContentItemScalarType.float_ => floatValue!,
    Revision3ContentItemScalarType.boolean => booleanValue!,
    Revision3ContentItemScalarType.string => stringValue!,
    Revision3ContentItemScalarType.enum_ => enumValue!,
  };

  factory Revision3ContentItemScalarValue._fromJson(
    Map<String, Object?> json,
    String context,
  ) {
    _requireKeys(json, const ['type', 'data'], context);
    final type = Revision3ContentItemScalarType.parse(
      json['type'],
      '$context type',
    );
    final data = json['data'];
    return switch (type) {
      Revision3ContentItemScalarType.integer =>
        Revision3ContentItemScalarValue._(
          type: type,
          integerValue: _signedInteger(data, '$context data'),
        ),
      Revision3ContentItemScalarType.float_ =>
        Revision3ContentItemScalarValue._(
          type: type,
          floatValue: _finiteCanonicalFloat(data, '$context data'),
        ),
      Revision3ContentItemScalarType.boolean =>
        Revision3ContentItemScalarValue._(
          type: type,
          booleanValue: _boolean(data, '$context data'),
        ),
      Revision3ContentItemScalarType.string =>
        Revision3ContentItemScalarValue._(
          type: type,
          stringValue: _itemString(data, '$context data'),
        ),
      Revision3ContentItemScalarType.enum_ => Revision3ContentItemScalarValue._(
        type: type,
        enumValue: _itemEnumValue(data, '$context data'),
      ),
    };
  }
}

/// Structured ItemPatch facts retained from the native exact-current index.
final class Revision3ContentItemPatchSummary {
  Revision3ContentItemPatchSummary({
    required this.vanillaClass,
    required Map<String, Revision3ContentItemScalarValue> fields,
  }) : fields = Map<String, Revision3ContentItemScalarValue>.unmodifiable(
         fields,
       );

  final String vanillaClass;
  final Map<String, Revision3ContentItemScalarValue> fields;
}

/// Closed generator identity retained for generated script ownership checks.
final class Revision3ContentScriptModuleSummary {
  const Revision3ContentScriptModuleSummary({
    required this.generatorId,
    required this.generatorVersion,
  });

  final String generatorId;
  final int generatorVersion;
}

final class Revision3ContentSummary {
  const Revision3ContentSummary._({
    required this.primaryIdentity,
    required this.secondaryText,
    required this.searchTerms,
    required this.localizationEntry,
    required this.dialogLine,
    required this.voiceSlot,
    required this.voiceTake,
    required this.npcDraft,
    required this.questDraft,
    required this.scriptModule,
    required this.itemPatch,
  });

  final String primaryIdentity;
  final String secondaryText;
  final List<String> searchTerms;
  final Revision3ContentLocalizationEntrySummary? localizationEntry;
  final Revision3ContentDialogLineSummary? dialogLine;
  final Revision3ContentVoiceSlotSummary? voiceSlot;
  final Revision3ContentVoiceTakeSummary? voiceTake;
  final Revision3ContentNpcDraftSummary? npcDraft;
  final Revision3ContentQuestDraftSummary? questDraft;
  final Revision3ContentScriptModuleSummary? scriptModule;
  final Revision3ContentItemPatchSummary? itemPatch;

  factory Revision3ContentSummary._fromJson(
    Map<String, Object?> json,
    Revision3ContentEntityKind entityKind,
    String context,
  ) {
    _requireKeys(json, const ['kind', 'data'], context);
    final summaryKind = Revision3ContentEntityKind.parse(json['kind'], context);
    if (summaryKind != entityKind) {
      throw FormatException('$context kind disagrees with its entity');
    }
    final data = _object(json['data'], '$context data');
    String primary;
    String secondary;
    final terms = <String>[];
    Revision3ContentLocalizationEntrySummary? localizationEntry;
    Revision3ContentDialogLineSummary? dialogLine;
    Revision3ContentVoiceSlotSummary? voiceSlot;
    Revision3ContentVoiceTakeSummary? voiceTake;
    Revision3ContentNpcDraftSummary? npcDraft;
    Revision3ContentQuestDraftSummary? questDraft;
    Revision3ContentScriptModuleSummary? scriptModule;
    Revision3ContentItemPatchSummary? itemPatch;
    switch (entityKind) {
      case Revision3ContentEntityKind.localizationEntry:
        _requireKeys(data, const ['loc_id', 'locales'], '$context data');
        primary = _string(data['loc_id'], '$context loc_id');
        final locales = _stringList(
          data['locales'],
          '$context locales',
          maxItems: 1000,
          maxStringBytes: 64,
        );
        _requireSortedUnique(locales, '$context locales');
        if (locales.any((locale) => !_contentLocaleIsCanonical(locale))) {
          throw FormatException(
            '$context locales contains a non-canonical locale',
          );
        }
        localizationEntry = Revision3ContentLocalizationEntrySummary(
          locales: locales,
        );
        secondary = locales.isEmpty ? 'No authored locale' : locales.join(', ');
        terms.addAll(locales);
      case Revision3ContentEntityKind.dialogLine:
        _requireKeys(data, const [
          'speaker_hint',
          'voice_slot_locales',
        ], '$context data');
        final speaker = _nullableString(
          data['speaker_hint'],
          '$context speaker_hint',
        );
        final locales = _stringList(
          data['voice_slot_locales'],
          '$context voice_slot_locales',
          maxItems: 1000,
          maxStringBytes: 64,
        );
        _requireSortedUnique(locales, '$context voice_slot_locales');
        if (locales.any((locale) => !_contentLocaleIsCanonical(locale))) {
          throw FormatException(
            '$context voice_slot_locales contains a non-canonical locale',
          );
        }
        dialogLine = Revision3ContentDialogLineSummary(
          speaker: speaker,
          voiceSlotLocales: List<String>.unmodifiable(locales),
        );
        primary = speaker ?? 'Dialog line';
        secondary = locales.isEmpty
            ? 'No voice slots'
            : 'Voice: ${locales.join(', ')}';
        if (speaker != null) terms.add(speaker);
        terms.addAll(locales);
      case Revision3ContentEntityKind.voiceSlot:
        _requireKeys(data, const [
          'locale',
          'target_resolution',
          'candidate_count',
          'has_selected_take',
        ], '$context data');
        primary = _locale(data['locale'], '$context locale');
        final resolution = Revision3ContentVoiceTargetResolution.parse(
          data['target_resolution'],
          '$context target_resolution',
        );
        final candidates = _integer(
          data['candidate_count'],
          '$context candidate_count',
        );
        final selected = _boolean(
          data['has_selected_take'],
          '$context has_selected_take',
        );
        voiceSlot = Revision3ContentVoiceSlotSummary(
          targetResolution: resolution,
          candidateCount: candidates,
          hasSelectedTake: selected,
        );
        secondary =
            '${resolution.name} · $candidates take${candidates == 1 ? '' : 's'}${selected ? ' · selected' : ''}';
      case Revision3ContentEntityKind.voiceTake:
        _requireKeys(data, const [
          'locale',
          'status',
          'codec',
          'channels',
          'sample_rate',
        ], '$context data');
        primary = _locale(data['locale'], '$context locale');
        final status = Revision3ContentVoiceTakeStatus.parse(
          data['status'],
          '$context status',
        );
        final codec = Revision3ContentVoiceOggCodec.parse(
          data['codec'],
          '$context codec',
        );
        final channels = _integer(data['channels'], '$context channels');
        final sampleRate = _integer(
          data['sample_rate'],
          '$context sample_rate',
        );
        if (channels < 1 || channels > 0xff) {
          throw FormatException('$context channels is outside its domain');
        }
        if (sampleRate < 1 || sampleRate > 0xffffffff) {
          throw FormatException('$context sample_rate is outside its domain');
        }
        voiceTake = Revision3ContentVoiceTakeSummary(
          locale: primary,
          status: status,
          codec: codec,
          channels: channels,
          sampleRate: sampleRate,
        );
        secondary =
            '${status.name} · ${codec.name} · ${channels}ch · $sampleRate Hz';
        terms.addAll([status.name, codec.name]);
      case Revision3ContentEntityKind.npcDraft:
        _requireKeys(data, <String>[
          'unique_name',
          'module_namespace',
          'parent_character_definition',
          'parent_ai_agent_config',
          'parent_spawn_definition',
          'greeting_count',
        ], '$context data');
        primary = _string(data['unique_name'], '$context unique_name');
        final namespace = _string(
          data['module_namespace'],
          '$context module_namespace',
        );
        final parentCharacterDefinition = _string(
          data['parent_character_definition'],
          '$context parent_character_definition',
        );
        final parentAiAgentConfig = _string(
          data['parent_ai_agent_config'],
          '$context parent_ai_agent_config',
        );
        final parentSpawnDefinition = _string(
          data['parent_spawn_definition'],
          '$context parent_spawn_definition',
        );
        final parents = <String>[
          parentCharacterDefinition,
          parentAiAgentConfig,
          parentSpawnDefinition,
        ];
        final greetingCount = _integer(
          data['greeting_count'],
          '$context greeting_count',
        );
        if (greetingCount > 256) {
          throw FormatException('$context greeting list exceeds 256 lines');
        }
        npcDraft = Revision3ContentNpcDraftSummary(
          uniqueName: primary,
          moduleNamespace: namespace,
          parentCharacterDefinition: parentCharacterDefinition,
          parentAiAgentConfig: parentAiAgentConfig,
          parentSpawnDefinition: parentSpawnDefinition,
          greetingCount: greetingCount,
        );
        secondary = namespace;
        terms.addAll([namespace, ...parents]);
      case Revision3ContentEntityKind.questDraft:
        final hasAdditionalObjectives = data.containsKey(
          'additional_objective_titles',
        );
        _requireKeys(data, <String>[
          'technical_id',
          'title',
          'objective_title',
          if (hasAdditionalObjectives) 'additional_objective_titles',
          'objective_slots',
          'transcript_count',
          'module_namespace',
          'parent_runtime_class',
          'giver_runtime_unique_name',
        ], '$context data');
        primary = _string(data['technical_id'], '$context technical_id');
        final title = _string(
          data['title'],
          '$context title',
          allowEmpty: true,
        );
        final objective = _string(
          data['objective_title'],
          '$context objective_title',
          allowEmpty: true,
        );
        final additionalObjectives = hasAdditionalObjectives
            ? _stringList(
                data['additional_objective_titles'],
                '$context additional_objective_titles',
                maxItems: 7,
                maxStringBytes: 128,
              )
            : const <String>[];
        if (hasAdditionalObjectives && additionalObjectives.isEmpty) {
          throw FormatException('$context has an empty objective extension');
        }
        final foldedObjectives = <String>{objective.toLowerCase()};
        var objectiveBytes = utf8.encode(objective).length;
        for (final additional in additionalObjectives) {
          objectiveBytes += utf8.encode(additional).length;
          if (additional.trim() != additional ||
              objectiveBytes > 1024 ||
              !foldedObjectives.add(additional.toLowerCase())) {
            throw FormatException(
              '$context has duplicate or non-canonical objectives',
            );
          }
        }
        final objectiveSlots = _integerList(
          data['objective_slots'],
          '$context objective_slots',
          maxItems: 8,
          min: 1,
          max: 0xffff,
        );
        if (objectiveSlots.isEmpty ||
            objectiveSlots.length != 1 + additionalObjectives.length ||
            objectiveSlots.toSet().length != objectiveSlots.length) {
          throw FormatException(
            '$context has invalid semantic objective slots',
          );
        }
        final transcriptCount = _integer(
          data['transcript_count'],
          '$context transcript_count',
        );
        if (transcriptCount > 256) {
          throw FormatException('$context transcript exceeds 256 lines');
        }
        final namespace = _string(
          data['module_namespace'],
          '$context module_namespace',
        );
        final parent = _string(
          data['parent_runtime_class'],
          '$context parent_runtime_class',
        );
        final giver = _string(
          data['giver_runtime_unique_name'],
          '$context giver_runtime_unique_name',
        );
        questDraft = Revision3ContentQuestDraftSummary(
          technicalId: primary,
          title: title,
          objectiveTitles: <String>[objective, ...additionalObjectives],
          objectiveSlots: objectiveSlots,
          transcriptCount: transcriptCount,
          moduleNamespace: namespace,
          parentRuntimeClass: parent,
          giverRuntimeUniqueName: giver,
        );
        secondary = title.isEmpty ? objective : title;
        terms.addAll([
          title,
          objective,
          ...additionalObjectives,
          namespace,
          parent,
          giver,
        ]);
      case Revision3ContentEntityKind.scriptModule:
        _requireKeys(data, const [
          'generator_id',
          'generator_version',
          'module_namespace',
          'module_relative_path',
          'status',
        ], '$context data');
        final generator = _string(
          data['generator_id'],
          '$context generator_id',
        );
        final generatorVersion = _integer(
          data['generator_version'],
          '$context generator_version',
        );
        scriptModule = Revision3ContentScriptModuleSummary(
          generatorId: generator,
          generatorVersion: generatorVersion,
        );
        primary = _string(
          data['module_namespace'],
          '$context module_namespace',
        );
        secondary = _string(
          data['module_relative_path'],
          '$context module_relative_path',
        );
        final status = _object(data['status'], '$context status');
        _requireKeys(status, const ['authoring', 'runtime'], '$context status');
        if (status['authoring'] != 'offline_draft' ||
            status['runtime'] != 'runtime_unqualified') {
          throw FormatException('$context has an unsupported script status');
        }
        terms.add(generator);
      case Revision3ContentEntityKind.itemPatch:
        itemPatch = _parseItemPatchSummary(data, '$context data');
        primary = itemPatch.vanillaClass;
        final fieldCount = itemPatch.fields.length;
        secondary = '$fieldCount edited field${fieldCount == 1 ? '' : 's'}';
        terms.addAll(itemPatch.fields.keys);
        for (final value in itemPatch.fields.values) {
          final enumValue = value.enumValue;
          if (enumValue != null) terms.add(enumValue.enumType);
          final stringValue = value.stringValue;
          if (stringValue != null) terms.add(stringValue);
        }
    }
    return Revision3ContentSummary._(
      primaryIdentity: primary,
      secondaryText: secondary,
      searchTerms: List<String>.unmodifiable([primary, secondary, ...terms]),
      localizationEntry: localizationEntry,
      dialogLine: dialogLine,
      voiceSlot: voiceSlot,
      voiceTake: voiceTake,
      npcDraft: npcDraft,
      questDraft: questDraft,
      scriptModule: scriptModule,
      itemPatch: itemPatch,
    );
  }
}

Revision3ContentItemPatchSummary _parseItemPatchSummary(
  Map<String, Object?> data,
  String context,
) {
  _requireKeys(data, const [
    'vanilla_class',
    'field_count',
    'field_types',
    'fields',
  ], context);
  final vanillaClass = _itemIdentifier(
    data['vanilla_class'],
    '$context vanilla_class',
    maxBytes: _maxItemClassBytes,
  );
  final fieldCount = _integer(data['field_count'], '$context field_count');
  if (fieldCount < 1 || fieldCount > _maxItemPatchFields) {
    throw FormatException('$context field_count is outside its domain');
  }

  final rawTypes = _object(data['field_types'], '$context field_types');
  final rawFields = _object(data['fields'], '$context fields');
  if (rawTypes.length != fieldCount || rawFields.length != fieldCount) {
    throw FormatException('$context field maps disagree with field_count');
  }
  _requireSortedUnique(rawTypes.keys.toList(), '$context field_types');
  _requireSortedUnique(rawFields.keys.toList(), '$context fields');
  if (!_equalStringLists(rawTypes.keys.toList(), rawFields.keys.toList())) {
    throw FormatException('$context field maps have different keys');
  }

  final fields = <String, Revision3ContentItemScalarValue>{};
  var totalStringBytes = 0;
  for (final entry in rawFields.entries) {
    final name = _itemIdentifier(
      entry.key,
      '$context field name',
      maxBytes: _maxItemFieldNameBytes,
    );
    final declaredType = Revision3ContentItemScalarType.parse(
      rawTypes[name],
      '$context field_types $name',
    );
    final value = Revision3ContentItemScalarValue._fromJson(
      _object(entry.value, '$context field $name'),
      '$context field $name',
    );
    if (value.type != declaredType) {
      throw FormatException('$context field $name has a false declared type');
    }
    final stringValue = value.stringValue;
    if (stringValue != null) {
      totalStringBytes += utf8.encode(stringValue).length;
      if (totalStringBytes > _maxItemStringTotalBytes) {
        throw FormatException('$context string fields exceed their budget');
      }
    }
    fields[name] = value;
  }
  return Revision3ContentItemPatchSummary(
    vanillaClass: vanillaClass,
    fields: fields,
  );
}

void _validateItemPatchProjectionFacts(
  Revision3ContentEntity entity, {
  required Revision3ContentSeal targetExecutable,
}) {
  if (entity.kind != Revision3ContentEntityKind.itemPatch) return;
  final facts = entity.summary.itemPatch;
  final generation = entity.origin.generationExecutable;
  final sourceSeal = entity.origin.sourceSeal;
  final catalogLayer = entity.origin.catalogLayer;
  final displayName = entity.displayName;
  if (facts == null ||
      entity.origin.type != 'vanilla' ||
      entity.origin.label != facts.vanillaClass ||
      generation == null ||
      generation.sha256 != targetExecutable.sha256 ||
      generation.byteLength != targetExecutable.byteLength ||
      catalogLayer == null ||
      catalogLayer.isEmpty ||
      catalogLayer.trim() != catalogLayer ||
      utf8.encode(catalogLayer).length > 128 ||
      _containsControlCharacter(catalogLayer) ||
      sourceSeal == null ||
      sourceSeal.byteLength == 0 ||
      displayName.trim().isEmpty ||
      utf8.encode(displayName).length > _maxItemClassBytes ||
      _containsControlCharacter(displayName) ||
      entity.references.isNotEmpty ||
      entity.assetReferences.isNotEmpty) {
    throw FormatException(
      'content ItemPatch ${entity.id} has malformed exact vanilla facts',
    );
  }
}

bool _containsControlCharacter(String value) =>
    value.runes.any((rune) => rune <= 0x1f || (rune >= 0x7f && rune <= 0x9f));

void _validateScriptModuleProjectionFacts(
  Revision3ContentEntity entity, {
  required String projectId,
  required Map<String, Revision3ContentEntity> entitiesById,
}) {
  if (entity.kind != Revision3ContentEntityKind.scriptModule) return;
  final facts = entity.summary.scriptModule;
  final owner = entity.origin.generatedOwner;
  final generator = owner == null
      ? null
      : _storyGeneratorForOwnerKind(owner.expectedKind);
  if (facts == null ||
      generator == null ||
      entity.origin.type != 'generated' ||
      entity.origin.label != generator.id ||
      entity.origin.generatorVersion != generator.version ||
      facts.generatorId != generator.id ||
      facts.generatorVersion != generator.version ||
      owner!.projectId != projectId ||
      entity.assetReferences.isNotEmpty) {
    throw FormatException(
      'content ScriptModule ${entity.id} is not a current native Story module',
    );
  }

  final ownerEntity = entitiesById[owner.entityId];
  if (ownerEntity == null || ownerEntity.kind != owner.expectedKind) {
    throw FormatException(
      'content ScriptModule ${entity.id} has no exact local Story owner',
    );
  }

  final originOwners = entity.references
      .where((reference) => reference.role == 'origin_owner')
      .toList(growable: false);
  final scriptOwners = entity.references
      .where((reference) => reference.role == 'script_owner')
      .toList(growable: false);
  if (entity.references.length != 2 ||
      originOwners.length != 1 ||
      scriptOwners.length != 1 ||
      !_isExactResolvedOwnerReference(originOwners.single, owner) ||
      !_isExactResolvedOwnerReference(scriptOwners.single, owner)) {
    throw FormatException(
      'content ScriptModule ${entity.id} has malformed owner facts',
    );
  }

  final moduleEdges = ownerEntity.references
      .where((reference) => reference.role == 'draft_script_module')
      .toList(growable: false);
  if (moduleEdges.length != 1 ||
      !_isExactResolvedModuleReference(
        moduleEdges.single,
        projectId: projectId,
        moduleId: entity.id,
      )) {
    throw FormatException(
      'content ScriptModule ${entity.id} is not the exact backing of its owner',
    );
  }

  final ownerNamespace = switch (ownerEntity.kind) {
    Revision3ContentEntityKind.npcDraft =>
      ownerEntity.summary.npcDraft?.moduleNamespace,
    Revision3ContentEntityKind.questDraft =>
      ownerEntity.summary.questDraft?.moduleNamespace,
    _ => null,
  };
  if (ownerNamespace == null ||
      entity.summary.primaryIdentity != ownerNamespace) {
    throw FormatException(
      'content ScriptModule ${entity.id} namespace disagrees with its owner',
    );
  }
}

({String id, int version})? _storyGeneratorForOwnerKind(
  Revision3ContentEntityKind kind,
) => switch (kind) {
  Revision3ContentEntityKind.npcDraft => (
    id: _npcGeneratorId,
    version: _npcGeneratorVersion,
  ),
  Revision3ContentEntityKind.questDraft => (
    id: _questGeneratorId,
    version: _questGeneratorVersion,
  ),
  _ => null,
};

bool _isExactResolvedOwnerReference(
  Revision3ContentReference reference,
  Revision3ContentReferenceTarget owner,
) =>
    reference.qualifier == null &&
    reference.target == owner &&
    reference.resolution == Revision3ContentReferenceResolution.resolved;

bool _isExactResolvedModuleReference(
  Revision3ContentReference reference, {
  required String projectId,
  required String moduleId,
}) =>
    reference.qualifier == null &&
    reference.target.projectId == projectId &&
    reference.target.entityId == moduleId &&
    reference.target.expectedKind == Revision3ContentEntityKind.scriptModule &&
    reference.resolution == Revision3ContentReferenceResolution.resolved;

Revision3ContentEntity _requireCurrentStoryModule(
  Revision3ContentEntity draft,
  Revision3ContentReference reference, {
  required String projectId,
  required Map<String, Revision3ContentEntity> entitiesById,
}) {
  if (!_isExactResolvedModuleReference(
    reference,
    projectId: projectId,
    moduleId: reference.target.entityId,
  )) {
    throw FormatException(
      'content ${draft.kind.displayName} ${draft.id} has a malformed generated module reference',
    );
  }
  final module = entitiesById[reference.target.entityId];
  final generator = _storyGeneratorForOwnerKind(draft.kind);
  final owner = module?.origin.generatedOwner;
  final moduleFacts = module?.summary.scriptModule;
  if (module == null ||
      module.kind != Revision3ContentEntityKind.scriptModule ||
      generator == null ||
      module.origin.type != 'generated' ||
      module.origin.label != generator.id ||
      module.origin.generatorVersion != generator.version ||
      owner?.projectId != projectId ||
      owner?.entityId != draft.id ||
      owner?.expectedKind != draft.kind ||
      moduleFacts?.generatorId != generator.id ||
      moduleFacts?.generatorVersion != generator.version) {
    throw FormatException(
      'content ${draft.kind.displayName} ${draft.id} is not owned by its current native generator',
    );
  }
  return module;
}

void _validateNpcProjectionFacts(
  Revision3ContentEntity entity, {
  required String projectId,
  required Map<String, Revision3ContentEntity> entitiesById,
}) {
  if (entity.kind != Revision3ContentEntityKind.npcDraft) return;
  final facts = entity.summary.npcDraft;
  if (facts == null ||
      entity.origin.type != 'new' ||
      entity.origin.label != facts.uniqueName ||
      entity.origin.generatedOwner != null ||
      entity.assetReferences.isNotEmpty) {
    throw FormatException('content NPC ${entity.id} has no structured summary');
  }
  final modules = <Revision3ContentReference>[];
  final greetings = <Revision3ContentReference>[];
  for (final reference in entity.references) {
    switch (reference.role) {
      case 'draft_script_module':
        modules.add(reference);
      case 'npc_greeting_line':
        greetings.add(reference);
      default:
        throw FormatException(
          'content NPC ${entity.id} has a role from another entity kind',
        );
    }
  }
  if (modules.length != 1) {
    throw FormatException(
      'content NPC ${entity.id} has an invalid module count',
    );
  }
  final module = _requireCurrentStoryModule(
    entity,
    modules.single,
    projectId: projectId,
    entitiesById: entitiesById,
  );
  if (module.summary.primaryIdentity != facts.moduleNamespace) {
    throw FormatException(
      'content NPC ${entity.id} module namespace disagrees with its generator',
    );
  }
  if (greetings.length != facts.greetingCount) {
    throw FormatException(
      'content NPC ${entity.id} greeting count disagrees with its references',
    );
  }
  final lineTargets = <String>{};
  for (final reference in greetings) {
    if (reference.qualifier != null ||
        reference.target.projectId != projectId ||
        reference.target.expectedKind !=
            Revision3ContentEntityKind.dialogLine ||
        reference.resolution != Revision3ContentReferenceResolution.resolved ||
        !lineTargets.add(
          '${reference.target.projectId}\u0000${reference.target.entityId}',
        )) {
      throw FormatException(
        'content NPC ${entity.id} has a malformed or duplicate greeting line',
      );
    }
  }
}

void _validateDialogLineProjectionFacts(Revision3ContentEntity entity) {
  if (entity.kind != Revision3ContentEntityKind.dialogLine) return;
  final facts = entity.summary.dialogLine;
  if (facts == null) {
    throw FormatException(
      'content DialogLine ${entity.id} has no structured summary',
    );
  }

  final localizationReferences = <Revision3ContentReference>[];
  final slotReferences = <Revision3ContentReference>[];
  final owner = entity.origin.generatedOwner;
  var ownerReferences = 0;
  for (final reference in entity.references) {
    switch (reference.role) {
      case 'origin_owner':
        ownerReferences++;
        if (owner == null ||
            reference.qualifier != null ||
            reference.target != owner) {
          throw FormatException(
            'content DialogLine ${entity.id} has a malformed origin owner',
          );
        }
        break;
      case 'dialog_localization':
        localizationReferences.add(reference);
        break;
      case 'dialog_voice_slot':
        slotReferences.add(reference);
        break;
      default:
        throw FormatException(
          'content DialogLine ${entity.id} has a role from another entity kind',
        );
    }
  }
  if (ownerReferences != (owner == null ? 0 : 1)) {
    throw FormatException(
      'content DialogLine ${entity.id} has an invalid origin owner count',
    );
  }
  if (localizationReferences.length != 1) {
    throw FormatException(
      'content DialogLine ${entity.id} must have one localization reference',
    );
  }
  final localization = localizationReferences.single;
  if (localization.qualifier != null ||
      localization.target.expectedKind !=
          Revision3ContentEntityKind.localizationEntry) {
    throw FormatException(
      'content DialogLine ${entity.id} has a malformed localization reference',
    );
  }

  final projectedLocales = <String>[];
  final uniqueLocales = <String>{};
  for (final reference in slotReferences) {
    final locale = reference.qualifier;
    if (locale == null ||
        !_contentLocaleIsCanonical(locale) ||
        reference.target.expectedKind != Revision3ContentEntityKind.voiceSlot ||
        !uniqueLocales.add(locale)) {
      throw FormatException(
        'content DialogLine ${entity.id} has a malformed or duplicate VoiceSlot reference',
      );
    }
    projectedLocales.add(locale);
  }
  if (!_equalStringLists(facts.voiceSlotLocales, projectedLocales)) {
    throw FormatException(
      'content DialogLine ${entity.id} VoiceSlot locales disagree with its references',
    );
  }
}

void _validateVoiceSlotProjectionFacts(Revision3ContentEntity entity) {
  if (entity.kind != Revision3ContentEntityKind.voiceSlot) return;
  final facts = entity.summary.voiceSlot;
  if (facts == null) {
    throw FormatException(
      'content VoiceSlot ${entity.id} has no structured summary',
    );
  }
  final candidates = <Revision3ContentReference>[];
  final selected = <Revision3ContentReference>[];
  final owner = entity.origin.generatedOwner;
  var ownerReferences = 0;
  for (final reference in entity.references) {
    switch (reference.role) {
      case 'origin_owner':
        ownerReferences++;
        if (owner == null ||
            reference.qualifier != null ||
            reference.target != owner) {
          throw FormatException(
            'content VoiceSlot ${entity.id} has a malformed origin owner',
          );
        }
        break;
      case 'voice_candidate':
        candidates.add(reference);
        break;
      case 'voice_selected':
        selected.add(reference);
        break;
      default:
        throw FormatException(
          'content VoiceSlot ${entity.id} has a role from another entity kind',
        );
    }
  }
  if (ownerReferences != (owner == null ? 0 : 1)) {
    throw FormatException(
      'content VoiceSlot ${entity.id} has an invalid origin owner count',
    );
  }
  if (facts.candidateCount != candidates.length) {
    throw FormatException(
      'content VoiceSlot ${entity.id} candidate_count disagrees with its references',
    );
  }
  final candidateTargets = <String>{};
  for (final candidate in candidates) {
    if (candidate.qualifier != null ||
        candidate.target.expectedKind != Revision3ContentEntityKind.voiceTake) {
      throw FormatException(
        'content VoiceSlot ${entity.id} has a malformed candidate reference',
      );
    }
    final key =
        '${candidate.target.projectId}\u0000${candidate.target.entityId}';
    if (!candidateTargets.add(key)) {
      throw FormatException(
        'content VoiceSlot ${entity.id} has duplicate candidate references',
      );
    }
  }
  if (selected.length > 1 || facts.hasSelectedTake != (selected.length == 1)) {
    throw FormatException(
      'content VoiceSlot ${entity.id} selected-take facts disagree with its references',
    );
  }
  if (selected case [final chosen]) {
    final key = '${chosen.target.projectId}\u0000${chosen.target.entityId}';
    if (chosen.qualifier != null ||
        chosen.target.expectedKind != Revision3ContentEntityKind.voiceTake ||
        !candidateTargets.contains(key)) {
      throw FormatException(
        'content VoiceSlot ${entity.id} selected take is not an exact candidate',
      );
    }
  }
}

void _validateQuestProjectionFacts(
  Revision3ContentEntity entity, {
  required String projectId,
  required Map<String, Revision3ContentEntity> entitiesById,
}) {
  if (entity.kind != Revision3ContentEntityKind.questDraft) return;
  final facts = entity.summary.questDraft;
  if (facts == null ||
      facts.objectiveSlots.isEmpty ||
      facts.objectiveSlots.length != facts.objectiveTitles.length ||
      entity.origin.type != 'new' ||
      entity.origin.label != facts.technicalId ||
      entity.origin.generatedOwner != null) {
    throw FormatException(
      'content Quest ${entity.id} has no structured summary',
    );
  }
  final modules = <Revision3ContentReference>[];
  final transcript = <Revision3ContentReference>[];
  for (final reference in entity.references) {
    switch (reference.role) {
      case 'draft_script_module':
        modules.add(reference);
      case 'quest_transcript_line':
        transcript.add(reference);
      default:
        throw FormatException(
          'content Quest ${entity.id} has a role from another entity kind',
        );
    }
  }
  if (modules.length != 1) {
    throw FormatException(
      'content Quest ${entity.id} has an invalid module count',
    );
  }
  final module = _requireCurrentStoryModule(
    entity,
    modules.single,
    projectId: projectId,
    entitiesById: entitiesById,
  );
  if (module.summary.primaryIdentity != facts.moduleNamespace) {
    throw FormatException(
      'content Quest ${entity.id} module namespace disagrees with its generator',
    );
  }

  if (entity.assetReferences.length != 1) {
    throw FormatException(
      'content Quest ${entity.id} has no exact current collision artifact',
    );
  }
  final collision = entity.assetReferences.single;
  if (collision.role != 'quest_collision_artifact' ||
      collision.logicalName != null ||
      collision.expectedMediaType != _questCollisionMediaType ||
      collision.resolution !=
          Revision3ContentAssetReferenceResolution.resolved) {
    throw FormatException(
      'content Quest ${entity.id} has a non-current collision artifact',
    );
  }
  if (transcript.length != facts.transcriptCount) {
    throw FormatException(
      'content Quest ${entity.id} transcript count disagrees with its references',
    );
  }
  final activeSlots = facts.objectiveSlots.toSet();
  final lineTargets = <String>{};
  final canonicalSlot = RegExp(r'^[1-9][0-9]*$');
  for (final reference in transcript) {
    final qualifier = reference.qualifier;
    final slot = qualifier == null || !canonicalSlot.hasMatch(qualifier)
        ? null
        : int.tryParse(qualifier);
    if (qualifier == null ||
        slot == null ||
        slot > 0xffff ||
        !activeSlots.contains(slot) ||
        reference.target.projectId != projectId ||
        reference.target.expectedKind !=
            Revision3ContentEntityKind.dialogLine ||
        reference.resolution != Revision3ContentReferenceResolution.resolved ||
        !lineTargets.add(
          '${reference.target.projectId}\u0000${reference.target.entityId}',
        )) {
      throw FormatException(
        'content Quest ${entity.id} has a malformed or duplicate transcript line',
      );
    }
  }
}

final class Revision3ContentReferenceTarget {
  const Revision3ContentReferenceTarget._({
    required this.projectId,
    required this.entityId,
    required this.expectedKind,
  });

  final String projectId;
  final String entityId;
  final Revision3ContentEntityKind expectedKind;

  factory Revision3ContentReferenceTarget._fromJson(
    Map<String, Object?> json,
    String context,
  ) {
    _requireKeys(json, const [
      'project_id',
      'entity_id',
      'expected_kind',
    ], context);
    return Revision3ContentReferenceTarget._(
      projectId: _id(json['project_id'], '$context project_id'),
      entityId: _id(json['entity_id'], '$context entity_id'),
      expectedKind: Revision3ContentEntityKind.parse(
        json['expected_kind'],
        '$context expected_kind',
      ),
    );
  }

  @override
  bool operator ==(Object other) =>
      other is Revision3ContentReferenceTarget &&
      projectId == other.projectId &&
      entityId == other.entityId &&
      expectedKind == other.expectedKind;

  @override
  int get hashCode => Object.hash(projectId, entityId, expectedKind);
}

final class Revision3ContentReference {
  const Revision3ContentReference._({
    required this.role,
    required this.qualifier,
    required this.target,
    required this.resolution,
  });

  final String role;
  final String? qualifier;
  final Revision3ContentReferenceTarget target;
  final Revision3ContentReferenceResolution resolution;

  factory Revision3ContentReference._fromJson(
    Map<String, Object?> json,
    String context,
  ) {
    _requireKeys(json, const [
      'role',
      'qualifier',
      'target',
      'resolution',
    ], context);
    return Revision3ContentReference._(
      role: _enumString(json['role'], const {
        'origin_owner',
        'dialog_localization',
        'dialog_voice_slot',
        'voice_candidate',
        'voice_selected',
        'npc_greeting_line',
        'quest_transcript_line',
        'draft_script_module',
        'script_owner',
      }, '$context role'),
      qualifier: _nullableString(json['qualifier'], '$context qualifier'),
      target: Revision3ContentReferenceTarget._fromJson(
        _object(json['target'], '$context target'),
        '$context target',
      ),
      resolution: Revision3ContentReferenceResolution.parse(
        json['resolution'],
        context,
      ),
    );
  }
}

final class Revision3ContentAssetReference {
  const Revision3ContentAssetReference._({
    required this.role,
    required this.sha256,
    required this.byteLength,
    required this.logicalName,
    required this.expectedMediaType,
    required this.resolution,
  });

  final String role;
  final String sha256;
  final int byteLength;
  final String? logicalName;
  final String expectedMediaType;
  final Revision3ContentAssetReferenceResolution resolution;

  factory Revision3ContentAssetReference._fromJson(
    Map<String, Object?> json,
    String context,
  ) {
    _requireKeys(json, const [
      'role',
      'sha256',
      'byte_len',
      'logical_name',
      'expected_media_type',
      'resolution',
    ], context);
    return Revision3ContentAssetReference._(
      role: _enumString(json['role'], const {
        'voice_audio',
        'quest_collision_artifact',
      }, '$context role'),
      sha256: _sha(json['sha256'], '$context sha256'),
      byteLength: _integer(json['byte_len'], '$context byte_len'),
      logicalName: _nullableString(
        json['logical_name'],
        '$context logical_name',
      ),
      expectedMediaType: _string(
        json['expected_media_type'],
        '$context expected_media_type',
        maxBytes: 256,
      ),
      resolution: Revision3ContentAssetReferenceResolution.parse(
        json['resolution'],
        context,
      ),
    );
  }
}

final class Revision3ContentAsset {
  const Revision3ContentAsset._({
    required this.sha256,
    required this.byteLength,
    required this.mediaType,
    required this.assetClass,
  });

  final String sha256;
  final int byteLength;
  final String mediaType;
  final Revision3ContentAssetClass assetClass;

  factory Revision3ContentAsset._fromJson(
    Map<String, Object?> json,
    int index,
  ) {
    final context = 'content asset $index';
    _requireKeys(json, const [
      'sha256',
      'byte_len',
      'media_type',
      'class',
    ], context);
    final mediaType = _string(
      json['media_type'],
      '$context media_type',
      maxBytes: 256,
    );
    final assetClass = Revision3ContentAssetClass.parse(json['class'], context);
    if (assetClass != _classifyAsset(mediaType)) {
      throw FormatException('$context class disagrees with its media type');
    }
    return Revision3ContentAsset._(
      sha256: _sha(json['sha256'], '$context sha256'),
      byteLength: _integer(json['byte_len'], '$context byte_len'),
      mediaType: mediaType,
      assetClass: assetClass,
    );
  }
}

Revision3ContentReferenceResolution _resolveReference(
  String projectId,
  Map<String, Revision3ContentEntity> entities,
  Revision3ContentReference reference,
) {
  if (reference.target.projectId != projectId) {
    return Revision3ContentReferenceResolution.foreignProject;
  }
  final target = entities[reference.target.entityId];
  if (target == null) return Revision3ContentReferenceResolution.missingEntity;
  if (target.kind != reference.target.expectedKind) {
    return Revision3ContentReferenceResolution.kindMismatch;
  }
  return Revision3ContentReferenceResolution.resolved;
}

Revision3ContentAssetReferenceResolution _resolveAssetReference(
  Map<String, Revision3ContentAsset> assets,
  Revision3ContentAssetReference reference,
) {
  final target = assets[reference.sha256];
  if (target == null) {
    return Revision3ContentAssetReferenceResolution.missingAsset;
  }
  if (target.byteLength != reference.byteLength) {
    return Revision3ContentAssetReferenceResolution.byteLengthMismatch;
  }
  if (target.mediaType != reference.expectedMediaType) {
    return Revision3ContentAssetReferenceResolution.mediaTypeMismatch;
  }
  return Revision3ContentAssetReferenceResolution.resolved;
}

Revision3ContentAssetClass _classifyAsset(String mediaType) =>
    switch (mediaType) {
      'audio/ogg' => Revision3ContentAssetClass.voiceAudio,
      _questCollisionMediaType =>
        Revision3ContentAssetClass.questCollisionArtifact,
      'application/vnd.gore.dataasset-fixed-leaf-stage+json;version=1' =>
        Revision3ContentAssetClass.dataAssetStageManifest,
      'application/vnd.gore.dataasset-fixed-leaf-component;version=1' =>
        Revision3ContentAssetClass.dataAssetStageComponent,
      _ => Revision3ContentAssetClass.other,
    };

bool _equalCounts(
  Map<Revision3ContentEntityKind, int> left,
  Map<Revision3ContentEntityKind, int> right,
) {
  if (left.length != right.length) return false;
  for (final entry in left.entries) {
    if (right[entry.key] != entry.value) return false;
  }
  return true;
}

Revision3ContentSeal _seal(Object? value, String context) {
  final object = _object(value, context);
  _requireKeys(object, const ['byte_len', 'sha256'], context);
  return Revision3ContentSeal(
    sha256: _sha(object['sha256'], '$context sha256'),
    byteLength: _integer(object['byte_len'], '$context byte_len'),
  );
}

Revision3ContentSeal _generation(Object? value, String context) {
  final object = _object(value, context);
  _requireKeys(object, const ['executable'], context);
  final executable = _seal(object['executable'], '$context executable');
  if (executable.byteLength == 0) {
    throw FormatException('$context executable is empty');
  }
  return executable;
}

Map<String, Object?> _object(Object? value, String context) {
  if (value is! Map) throw FormatException('$context is not an object');
  final result = <String, Object?>{};
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw FormatException('$context has a non-string key');
    }
    result[entry.key as String] = entry.value;
  }
  return result;
}

List<Object?> _list(Object? value, String context, {required int maxItems}) {
  if (value is! List || value.length > maxItems) {
    throw FormatException('$context is not a bounded list');
  }
  return value.cast<Object?>();
}

void _requireKeys(
  Map<String, Object?> object,
  List<String> expected,
  String context,
) {
  final actual = object.keys.toList(growable: false);
  if (actual.length != expected.length) {
    throw FormatException('$context has an invalid closed shape');
  }
  for (var index = 0; index < expected.length; index++) {
    if (actual[index] != expected[index]) {
      throw FormatException('$context has non-canonical or unknown fields');
    }
  }
}

String _string(
  Object? value,
  String context, {
  bool allowEmpty = false,
  int maxBytes = _maxProjectJsonBytes,
}) {
  if (value is! String ||
      (!allowEmpty && value.isEmpty) ||
      utf8.encode(value).length > maxBytes) {
    throw FormatException('$context is not bounded UTF-8 text');
  }
  return value;
}

String? _nullableString(Object? value, String context) {
  if (value == null) return null;
  return _string(value, context, allowEmpty: true);
}

String _id(Object? value, String context) {
  final text = _string(value, context, maxBytes: 32);
  if (!_idPattern.hasMatch(text)) {
    throw FormatException('$context is not an ID');
  }
  return text;
}

String _sha(Object? value, String context) {
  final text = _string(value, context, maxBytes: 64);
  if (!_shaPattern.hasMatch(text)) {
    throw FormatException('$context is not a SHA-256');
  }
  return text;
}

String _locale(Object? value, String context) =>
    _string(value, context, maxBytes: 64);

bool _contentLocaleIsCanonical(String value) {
  if (value.isEmpty ||
      value.length > 35 ||
      value.codeUnits.any((unit) => unit > 0x7f)) {
    return false;
  }
  final segments = value.split('-');
  if (!RegExp(r'^[a-z]{2,8}$').hasMatch(segments.first)) return false;
  final canonical = StringBuffer(segments.first);
  for (final segment in segments.skip(1)) {
    if (!RegExp(r'^[A-Za-z0-9]{1,8}$').hasMatch(segment)) return false;
    canonical.write('-');
    if (segment.length == 4 && RegExp(r'^[A-Za-z]+$').hasMatch(segment)) {
      canonical.write(
        '${segment[0].toUpperCase()}${segment.substring(1).toLowerCase()}',
      );
    } else if (segment.length == 2 &&
        RegExp(r'^[A-Za-z]+$').hasMatch(segment)) {
      canonical.write(segment.toUpperCase());
    } else {
      canonical.write(segment.toLowerCase());
    }
  }
  return canonical.toString() == value;
}

int _integer(Object? value, String context) {
  if (value is! int || value < 0 || value > _signedWireMax) {
    throw FormatException('$context is not a signed-wire-safe integer');
  }
  return value;
}

int _signedInteger(Object? value, String context) {
  if (value is! int || value < _signedWireMin || value > _signedWireMax) {
    throw FormatException('$context is not a signed 64-bit integer');
  }
  return value;
}

double _finiteCanonicalFloat(Object? value, String context) {
  if (value is! double ||
      !value.isFinite ||
      (value == 0.0 && value.isNegative)) {
    throw FormatException('$context is not a canonical finite float');
  }
  return value;
}

String _itemString(Object? value, String context) {
  final result = _string(
    value,
    context,
    allowEmpty: true,
    maxBytes: _maxItemStringBytes,
  );
  if (result.contains('\u0000')) {
    throw FormatException('$context contains a null character');
  }
  return result;
}

String _itemIdentifier(Object? value, String context, {required int maxBytes}) {
  final result = _string(value, context, maxBytes: maxBytes);
  if (!_itemIdentifierPattern.hasMatch(result)) {
    throw FormatException('$context is not a canonical item identifier');
  }
  return result;
}

Revision3ContentItemEnumValue _itemEnumValue(Object? value, String context) {
  final object = _object(value, context);
  _requireKeys(object, const ['enum_type', 'backing'], context);
  final enumType = _string(
    object['enum_type'],
    '$context enum_type',
    maxBytes: _maxItemEnumTypeBytes,
  );
  if (enumType
      .split('::')
      .any((segment) => !_itemIdentifierPattern.hasMatch(segment))) {
    throw FormatException('$context enum_type is not canonical');
  }
  return Revision3ContentItemEnumValue(
    enumType: enumType,
    backing: _signedInteger(object['backing'], '$context backing'),
  );
}

bool _boolean(Object? value, String context) {
  if (value is! bool) throw FormatException('$context is not a bool');
  return value;
}

String _enumString(Object? value, Set<String> allowed, String context) {
  if (value is! String || !allowed.contains(value)) {
    throw FormatException('$context is not a supported enum value');
  }
  return value;
}

List<String> _stringList(
  Object? value,
  String context, {
  required int maxItems,
  required int maxStringBytes,
}) => _list(value, context, maxItems: maxItems)
    .mapIndexed(
      (index, item) =>
          _string(item, '$context item $index', maxBytes: maxStringBytes),
    )
    .toList(growable: false);

List<int> _integerList(
  Object? value,
  String context, {
  required int maxItems,
  required int min,
  required int max,
}) => _list(value, context, maxItems: maxItems)
    .mapIndexed((index, item) {
      final parsed = _integer(item, '$context item $index');
      if (parsed < min || parsed > max) {
        throw FormatException('$context item $index is outside its domain');
      }
      return parsed;
    })
    .toList(growable: false);

void _requireSortedUnique(List<String> values, String context) {
  for (var index = 1; index < values.length; index++) {
    if (values[index - 1].compareTo(values[index]) >= 0) {
      throw FormatException('$context is not unique canonical order');
    }
  }
}

bool _equalStringLists(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

extension _IndexedIterable<E> on Iterable<E> {
  Iterable<T> mapIndexed<T>(T Function(int index, E value) convert) sync* {
    var index = 0;
    for (final value in this) {
      yield convert(index++, value);
    }
  }
}
