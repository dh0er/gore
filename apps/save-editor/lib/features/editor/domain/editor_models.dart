import 'dart:convert';

import 'package:goresave/features/editor/domain/progression_models.dart';

/// One property surfaced by the typed property browser search.
class TypedPropertyHit {
  const TypedPropertyHit({
    required this.path,
    required this.display,
    required this.type,
    required this.value,
    required this.editable,
    this.id = '',
    this.source = 'private',
    this.kind = 'scalar',
    this.structType,
    this.editValue,
    this.childCount = 0,
    this.depth = 0,
  });

  factory TypedPropertyHit.fromJson(Map<String, Object?> json) {
    return TypedPropertyHit(
      path:
          (json['path'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
      display: json['display'] as String? ?? '',
      type: json['type'] as String? ?? '',
      value: json['value'] as String? ?? '',
      editable: json['editable'] as bool? ?? false,
      id: json['id'] as String? ?? '',
      source: json['source'] as String? ?? 'private',
      kind: json['kind'] as String? ?? 'scalar',
      structType: json['structType'] as String?,
      editValue: json['editValue'],
      childCount: (json['childCount'] as num?)?.toInt() ?? 0,
      depth: (json['depth'] as num?)?.toInt() ?? 0,
    );
  }

  /// setValue-addressable path segments (name / `{mapKey}` / `[index]`).
  final List<String> path;
  final String display;
  final String type;
  final String value;
  final bool editable;

  /// Stable within one parsed save, independent of the active search/filter.
  final String id;
  final String source;
  final String kind;
  final String? structType;

  /// Lossless JSON shape accepted by `private.typed.setValue`.
  final Object? editValue;
  final int childCount;
  final int depth;

  String get stableId => id.isEmpty ? '$source:${path.join('/')}' : id;

  bool get isNativeStruct => kind == 'nativeStruct';
}

class TypedSearchSummary {
  const TypedSearchSummary({
    this.sources = const {},
    this.kinds = const {},
    this.types = const {},
    this.editable = 0,
    this.readOnly = 0,
    this.typedSources = const [],
  });

  factory TypedSearchSummary.fromJson(Map<String, Object?> json) {
    Map<String, int> counts(Object? raw) {
      if (raw is! Map) return const {};
      return {
        for (final entry in raw.entries)
          if (entry.key is String && entry.value is num)
            entry.key as String: (entry.value as num).toInt(),
      };
    }

    return TypedSearchSummary(
      sources: counts(json['sources']),
      kinds: counts(json['kinds']),
      types: counts(json['types']),
      editable: (json['editable'] as num?)?.toInt() ?? 0,
      readOnly: (json['readOnly'] as num?)?.toInt() ?? 0,
      typedSources:
          (json['typedSources'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
    );
  }

  final Map<String, int> sources;
  final Map<String, int> kinds;
  final Map<String, int> types;
  final int editable;
  final int readOnly;
  final List<String> typedSources;
}

/// Result of a typed property search over the decoded private payload.
class TypedSearchResult {
  const TypedSearchResult({
    this.results = const [],
    this.offset = 0,
    this.limit = 50,
    this.total = 0,
    this.source = 'private',
    this.summary = const TypedSearchSummary(),
    this.warnings = const [],
    this.error,
  });

  factory TypedSearchResult.fromJson(Map<String, Object?> json) {
    return TypedSearchResult(
      results:
          (json['results'] as List?)
              ?.whereType<Map>()
              .map((e) => TypedPropertyHit.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 50,
      total: (json['total'] as num?)?.toInt() ?? 0,
      source: json['source'] as String? ?? 'private',
      summary: json['summary'] is Map
          ? TypedSearchSummary.fromJson(
              (json['summary'] as Map).cast<String, Object?>(),
            )
          : const TypedSearchSummary(),
      warnings:
          (json['warnings'] as List?)?.whereType<String>().toList(
            growable: false,
          ) ??
          const [],
    );
  }

  final List<TypedPropertyHit> results;
  final int offset;
  final int limit;
  final int total;
  final String source;
  final TypedSearchSummary summary;
  final List<String> warnings;
  final String? error;

  /// Zero-based index of the current page.
  int get pageIndex => limit == 0 ? 0 : offset ~/ limit;

  /// Total number of pages (at least 1).
  int get pageCount => total == 0 ? 1 : (total + limit - 1) ~/ limit;

  bool get hasPrevious => offset > 0;
  bool get hasNext => offset + results.length < total;
}

class ScreenshotSummary {
  const ScreenshotSummary({
    required this.mimeType,
    required this.byteLength,
    required this.bytesBase64,
  });

  factory ScreenshotSummary.fromJson(Map<String, Object?> json) {
    return ScreenshotSummary(
      mimeType: json['mimeType'] as String? ?? 'image/jpeg',
      byteLength: (json['byteLength'] as num?)?.toInt() ?? 0,
      bytesBase64: json['bytesBase64'] as String? ?? '',
    );
  }

  static ScreenshotSummary? maybeFromJson(Object? value) {
    if (value is! Map) return null;
    return ScreenshotSummary.fromJson(value.cast<String, Object?>());
  }

  final String mimeType;
  final int byteLength;
  final String bytesBase64;
}

class ProfileSummary {
  const ProfileSummary({
    required this.profileId,
    this.profileName,
    this.quickSaveSlots = const [],
    this.autoSaveSlots = const [],
    this.savedSlots = const [],
    this.difficultyPreset,
    this.customCombatSettings,
    this.customResourcesSettings,
    this.customProgressionSettings,
    this.survival,
    this.permanentDeath,
    this.permanentDeathGameOver,
    this.fakeSloppyCombos,
    this.maxQuick,
    this.maxAuto,
  });

  factory ProfileSummary.fromJson(Map<String, Object?> json) {
    return ProfileSummary(
      profileId: (json['profileId'] as num?)?.toInt() ?? 0,
      profileName: json['profileName'] as String?,
      quickSaveSlots:
          (json['quickSaveSlots'] as List?)?.whereType<String>().toList() ??
          const [],
      autoSaveSlots:
          (json['autoSaveSlots'] as List?)?.whereType<String>().toList() ??
          const [],
      savedSlots:
          (json['savedSlots'] as List?)?.whereType<String>().toList() ??
          const [],
      difficultyPreset: json['difficultyPreset'] as String?,
      customCombatSettings: json['customCombatSettings'] as String?,
      customResourcesSettings: json['customResourcesSettings'] as String?,
      customProgressionSettings: json['customProgressionSettings'] as String?,
      survival: json['survival'] as bool?,
      permanentDeath: json['permanentDeath'] as bool?,
      permanentDeathGameOver: json['permanentDeathGameOver'] as bool?,
      fakeSloppyCombos: json['fakeSloppyCombos'] as bool?,
      maxQuick: (json['maxQuick'] as num?)?.toInt(),
      maxAuto: (json['maxAuto'] as num?)?.toInt(),
    );
  }

  final int profileId;
  final String? profileName;
  final List<String> quickSaveSlots;
  final List<String> autoSaveSlots;
  final List<String> savedSlots;
  final String? difficultyPreset;
  final String? customCombatSettings;
  final String? customResourcesSettings;
  final String? customProgressionSettings;
  final bool? survival;
  final bool? permanentDeath;
  final bool? permanentDeathGameOver;
  final bool? fakeSloppyCombos;
  final int? maxQuick;
  final int? maxAuto;

  String get displayName => 'Profile $displayNumber';

  /// The game-facing slot is stored as a zero-based numeric `m_ProfileName`.
  /// It can differ from the stable internal id used by save mutations.
  int get displayNumber {
    final gameSlot = int.tryParse(profileName?.trim() ?? '');
    if (gameSlot != null && gameSlot >= 0 && gameSlot < 4) {
      return gameSlot + 1;
    }
    return gameProfileNumber(profileId);
  }

  /// The profile's difficulty, mapped into the same [DifficultySettings] shape
  /// the editor uses. This is the authoritative, profile-wide difficulty — the
  /// only difficulty the app reads or writes.
  DifficultySettings get difficulty => DifficultySettings(
    preset: difficultyPreset,
    combat: customCombatSettings,
    resources: customResourcesSettings,
    progression: customProgressionSettings,
    flowHelper: fakeSloppyCombos,
    permadeath: permanentDeath,
  );
}

int gameProfileNumber(int profileId) => profileId + 1;

/// Maps a difficulty class short-name suffix to its UI label.
String _difficultyLevelLabel(String? className) {
  if (className == null) return '-';
  if (className.endsWith('_Easy')) return 'Novice';
  if (className.endsWith('_Standard')) return 'Gothic';
  if (className.endsWith('_Hard')) return 'Hard';
  if (className.endsWith('_Custom')) return 'Custom';
  return className;
}

class DifficultySettings {
  const DifficultySettings({
    this.preset,
    this.combat,
    this.resources,
    this.progression,
    this.flowHelper,
    this.permadeath,
  });

  factory DifficultySettings.fromJson(Map<String, Object?> json) {
    return DifficultySettings(
      preset: json['preset'] as String?,
      combat: json['combat'] as String?,
      resources: json['resources'] as String?,
      progression: json['progression'] as String?,
      flowHelper: json['flowHelper'] as bool?,
      permadeath: json['permadeath'] as bool?,
    );
  }

  static DifficultySettings? maybeFromJson(Object? json) => json is Map
      ? DifficultySettings.fromJson(json.cast<String, Object?>())
      : null;

  final String? preset;
  final String? combat;
  final String? resources;
  final String? progression;
  final bool? flowHelper;
  final bool? permadeath;

  /// True when ANY difficulty field is present. Distinguishes a profile that
  /// carries editable difficulty (even with an unrecognised preset class — the
  /// dialog can still repair it) from one synthesized without difficulty data
  /// (nothing to patch). Drives whether the header chip is interactive.
  bool get hasAnyValue =>
      preset != null ||
      combat != null ||
      resources != null ||
      progression != null ||
      flowHelper != null ||
      permadeath != null;

  String get presetLabel => _difficultyLevelLabel(preset);
  String get combatLabel => _difficultyLevelLabel(combat);
  String get resourcesLabel => _difficultyLevelLabel(resources);
  String get progressionLabel => _difficultyLevelLabel(progression);
}

class SaveSlot {
  const SaveSlot({
    required this.path,
    required this.slot,
    required this.format,
    required this.fileSize,
    required this.sha1,
    required this.status,
    this.playerSaveName,
    this.persistentPlayerSaveName,
    this.slotName,
    this.compressionMethod,
    this.chunkCount,
    this.chapterId,
    this.mapName,
    this.timePlayedSeconds,
    this.timeLoadedSeconds,
    this.quickSave,
    this.autoSave,
    this.persistentProfileId,
    this.screenshot,
    this.difficulty,
    this.isExternal = false,
  });

  factory SaveSlot.fromJson(Map<String, Object?> json) {
    return SaveSlot(
      path: json['path'] as String? ?? '',
      slot: json['slot'] as String? ?? 'unknown',
      format: json['format'] as String? ?? 'UNKNOWN',
      fileSize: (json['fileSize'] as num?)?.toInt() ?? 0,
      sha1: json['sha1'] as String? ?? '',
      status: json['status'] as String? ?? 'unknown',
      playerSaveName: json['playerSaveName'] as String?,
      persistentPlayerSaveName: json['persistentPlayerSaveName'] as String?,
      slotName: json['slotName'] as String?,
      compressionMethod: json['compressionMethod'] as String?,
      chunkCount: (json['chunkCount'] as num?)?.toInt(),
      chapterId: (json['chapterId'] as num?)?.toInt(),
      mapName: json['mapName'] as String?,
      timePlayedSeconds: (json['timePlayedSeconds'] as num?)?.toDouble(),
      timeLoadedSeconds: (json['timeLoadedSeconds'] as num?)?.toDouble(),
      quickSave: json['quickSave'] as bool?,
      autoSave: json['autoSave'] as bool?,
      persistentProfileId: (json['persistentProfileId'] as num?)?.toInt(),
      screenshot: ScreenshotSummary.maybeFromJson(json['screenshot']),
      difficulty: DifficultySettings.maybeFromJson(json['difficulty']),
      isExternal: json['isExternal'] as bool? ?? false,
    );
  }

  /// Build the sidebar entry for a save opened directly from an arbitrary
  /// file. Detached saves deliberately carry no PersistentDataList profile:
  /// an embedded numeric profile id is not proof that the file belongs to a
  /// profile in the currently scanned game folder.
  factory SaveSlot.fromInspection(
    SaveInspection inspection, {
    required bool isExternal,
  }) {
    final path = inspection.path ?? '';
    final normalized = path.replaceAll('\\', '/');
    final fileName = normalized.split('/').last;
    final dot = fileName.lastIndexOf('.');
    final slot = dot > 0 ? fileName.substring(0, dot) : fileName;
    return SaveSlot(
      path: path,
      slot: slot.isEmpty ? 'external' : slot,
      format: inspection.format,
      fileSize: inspection.size,
      sha1: inspection.sha1,
      status: 'ok',
      playerSaveName: inspection.playerSaveName,
      persistentPlayerSaveName: isExternal
          ? null
          : inspection.persistentPlayerSaveName,
      slotName: inspection.slotName,
      compressionMethod: inspection.compressionMethod,
      chunkCount: inspection.chunkCount,
      chapterId: inspection.chapterId,
      mapName: inspection.mapName,
      timePlayedSeconds: inspection.timePlayedSeconds,
      timeLoadedSeconds: inspection.timeLoadedSeconds,
      quickSave: inspection.quickSave,
      autoSave: inspection.autoSave,
      persistentProfileId: isExternal ? null : inspection.persistentProfileId,
      screenshot: inspection.screenshot,
      difficulty: inspection.difficulty,
      isExternal: isExternal,
    );
  }

  final String path;
  final String slot;
  final String format;
  final int fileSize;
  final String sha1;
  final String status;
  final String? playerSaveName;
  final String? persistentPlayerSaveName;
  final String? slotName;
  final String? compressionMethod;
  final int? chunkCount;
  final int? chapterId;
  final String? mapName;
  final double? timePlayedSeconds;
  final double? timeLoadedSeconds;
  final bool? quickSave;
  final bool? autoSave;
  final int? persistentProfileId;
  final ScreenshotSummary? screenshot;
  final DifficultySettings? difficulty;

  /// True when this slot was opened directly and is not registered in the
  /// scanned folder's PersistentDataList.sav.
  final bool isExternal;

  /// PersistentDataList still references this slot, but its expected `.sav`
  /// file is absent. Missing rows remain visible for cleanup and are never
  /// inspectable as save files.
  bool get isMissing => status == 'missing';

  /// The platform-independent basename shown in the sidebar. Save paths can be
  /// Windows-style even when tests or tooling run on another host.
  String get fileName {
    final normalized = path.replaceAll('\\', '/');
    final name = normalized.split('/').last;
    return name.isEmpty ? '$slot.sav' : name;
  }

  String get displayName {
    final name = playerSaveName ?? persistentPlayerSaveName;
    return name == null || name.isEmpty ? slot : name;
  }
}

class SaveInspection {
  const SaveInspection({
    required this.format,
    required this.path,
    required this.size,
    required this.sha1,
    required this.raw,
    this.slot,
    this.playerSaveName,
    this.persistentPlayerSaveName,
    this.slotName,
    this.compressionMethod,
    this.chunkCount,
    this.uncompressedSize,
    this.trailerSize,
    this.chapterId,
    this.mapName,
    this.timePlayedSeconds,
    this.timeLoadedSeconds,
    this.quickSave,
    this.autoSave,
    this.persistentProfileId,
    this.screenshot,
    this.difficulty,
    this.privateStatus,
    this.privateDecoded = false,
    this.privateDecompressedSize,
    this.privateStringCount,
    this.privateStrings = const [],
    this.privatePreview = false,
    this.privateDecodedChunkCount,
    this.privateTotalChunkCount,
    this.privatePlayer = const PrivatePlayerSummary(),
    this.privateInventory = const PrivateInventorySummary(),
    this.privateNpc = const PrivateNpcSummary(),
    this.privateProgression = const ProgressionOverview(),
    this.privateFactions,
    this.privateTypedParseStatus,
    this.privateTypedPropertyCount,
    this.privateTypedMaxDepth,
  });

  factory SaveInspection.fromJson(Map<String, Object?> json) {
    final public = (json['public'] as Map?)?.cast<String, Object?>();
    final persistent = (json['persistent'] as Map?)?.cast<String, Object?>();
    final stream = (json['compressedStream'] as Map?)?.cast<String, Object?>();
    final private = (json['private'] as Map?)?.cast<String, Object?>();
    final privatePlayer = (private?['player'] as Map?)?.cast<String, Object?>();
    final privateInventory = (private?['inventory'] as Map?)
        ?.cast<String, Object?>();
    final privateNpc = (private?['npc'] as Map?)?.cast<String, Object?>();
    final privateProgression = (private?['progression'] as Map?)
        ?.cast<String, Object?>();
    final privateFactions = (private?['factions'] as Map?)
        ?.cast<String, Object?>();
    final privateStatus = private?['status'] as String?;
    final typedParse = (private?['typedParse'] as Map?)
        ?.cast<String, Object?>();
    return SaveInspection(
      format: json['format'] as String? ?? 'UNKNOWN',
      path: json['path'] as String?,
      slot: json['slot'] as String?,
      size: (json['size'] as num?)?.toInt() ?? 0,
      sha1: json['sha1'] as String? ?? '',
      playerSaveName: public?['playerSaveName'] as String?,
      persistentPlayerSaveName: persistent?['playerSaveName'] as String?,
      slotName: public?['slotName'] as String?,
      compressionMethod: stream?['method'] as String?,
      chunkCount: (stream?['chunkCount'] as num?)?.toInt(),
      uncompressedSize: (stream?['uncompressedSize'] as num?)?.toInt(),
      trailerSize: (json['trailerSize'] as num?)?.toInt(),
      chapterId: (persistent?['chapterId'] as num?)?.toInt(),
      mapName: persistent?['mapName'] as String?,
      timePlayedSeconds: (persistent?['timePlayedSeconds'] as num?)?.toDouble(),
      timeLoadedSeconds: (persistent?['timeLoadedSeconds'] as num?)?.toDouble(),
      quickSave: persistent?['quickSave'] as bool?,
      autoSave: persistent?['autoSave'] as bool?,
      persistentProfileId: (persistent?['profileId'] as num?)?.toInt(),
      screenshot: ScreenshotSummary.maybeFromJson(json['screenshot']),
      difficulty: DifficultySettings.maybeFromJson(json['difficulty']),
      privateStatus: privateStatus,
      privateDecoded:
          privateStatus == 'decoded' || privateStatus == 'decoded_preview',
      privateDecompressedSize: (private?['decompressedSize'] as num?)?.toInt(),
      privateStringCount: (private?['stringCount'] as num?)?.toInt(),
      privateStrings:
          (private?['strings'] as List?)?.whereType<String>().toList() ??
          const [],
      // A decoded_preview status is a partial decode even when the explicit
      // `preview` flag is absent, so treat the status as authoritative; private
      // edits must stay disabled for previews.
      privatePreview:
          privateStatus == 'decoded_preview' ||
          (private?['preview'] as bool? ?? false),
      privateDecodedChunkCount: (private?['decodedChunkCount'] as num?)
          ?.toInt(),
      privateTotalChunkCount: (private?['totalChunkCount'] as num?)?.toInt(),
      privatePlayer: PrivatePlayerSummary.fromJson(privatePlayer),
      privateInventory: PrivateInventorySummary.fromJson(privateInventory),
      privateNpc: PrivateNpcSummary.fromJson(privateNpc),
      privateProgression: ProgressionOverview.fromJson(privateProgression),
      privateFactions: privateFactions == null
          ? null
          : FactionsPage.fromJson(privateFactions),
      privateTypedParseStatus: typedParse?['status'] as String?,
      privateTypedPropertyCount: (typedParse?['propertyCount'] as num?)
          ?.toInt(),
      privateTypedMaxDepth: (typedParse?['maxDepth'] as num?)?.toInt(),
      raw: json,
    );
  }

  final String format;
  final String? path;
  final String? slot;
  final int size;
  final String sha1;
  final String? playerSaveName;
  final String? persistentPlayerSaveName;
  final String? slotName;
  final String? compressionMethod;
  final int? chunkCount;
  final int? uncompressedSize;
  final int? trailerSize;
  final int? chapterId;
  final String? mapName;
  final double? timePlayedSeconds;
  final double? timeLoadedSeconds;
  final bool? quickSave;
  final bool? autoSave;
  final int? persistentProfileId;
  final ScreenshotSummary? screenshot;
  final DifficultySettings? difficulty;
  final String? privateStatus;
  final bool privateDecoded;
  final int? privateDecompressedSize;
  final int? privateStringCount;
  final List<String> privateStrings;
  final bool privatePreview;
  final int? privateDecodedChunkCount;
  final int? privateTotalChunkCount;

  /// Private writes are only safe when the full payload is decoded. A preview
  /// (partial) decode shows data read-only, so edit actions stay disabled.
  bool get privateEditable => privateDecoded && !privatePreview;

  final PrivatePlayerSummary privatePlayer;
  final PrivateInventorySummary privateInventory;
  final PrivateNpcSummary privateNpc;
  final ProgressionOverview privateProgression;
  final FactionsPage? privateFactions;

  /// Status of the strict typed property parse of the decoded private payload
  /// ('ok', 'failed', 'skipped_preview'); null when no private decode ran.
  final String? privateTypedParseStatus;
  final int? privateTypedPropertyCount;
  final int? privateTypedMaxDepth;

  /// The byte-exact typed parse succeeded — the save's full property layout is
  /// verified and typed (layout-aware) edits are safe.
  bool get privateTypedVerified => privateTypedParseStatus == 'ok';

  final Map<String, Object?> raw;

  String prettyJson() {
    const encoder = JsonEncoder.withIndent('  ');
    return encoder.convert(raw);
  }
}

/// NPC capabilities emitted by `inspect_save` under `private.npc`.
///
/// Relationship persistence needs a compatible `RelationshipByGlobalId` map;
/// having a typed NPC attribute map alone is not enough. The UI therefore uses
/// this per-save capability instead of assuming every decoded save can accept
/// `private.npc.setRelationship`.
class PrivateNpcSummary {
  const PrivateNpcSummary({this.hasNpcs = false, this.writable = const []});

  factory PrivateNpcSummary.fromJson(Map<String, Object?>? json) {
    return PrivateNpcSummary(
      hasNpcs: json?['hasNpcs'] == true,
      writable:
          (json?['writable'] as List?)?.whereType<String>().toList() ??
          const [],
    );
  }

  final bool hasNpcs;
  final List<String> writable;

  bool get canSetRelationship =>
      writable.contains('private.npc.setRelationship');
}

class PrivatePlayerSummary {
  const PrivatePlayerSummary({
    this.saveVersionNumber,
    this.currentWorld,
    this.playerName,
    this.profileName,
    this.transform,
    this.attributes = const [],
    this.scriptPaths = const [],
    this.properties = const [],
    this.writable = const [],
  });

  factory PrivatePlayerSummary.fromJson(Map<String, Object?>? json) {
    return PrivatePlayerSummary(
      saveVersionNumber: (json?['saveVersionNumber'] as num?)?.toInt(),
      currentWorld: json?['currentWorld'] as String?,
      playerName: json?['playerName'] as String?,
      profileName: json?['profileName'] as String?,
      transform: PrivatePlayerTransform.fromJson(json?['transform']),
      attributes:
          (json?['attributes'] as List?)
              ?.whereType<Map>()
              .map((value) => PrivatePlayerAttribute.fromJson(value))
              .toList() ??
          const [],
      scriptPaths:
          (json?['scriptPaths'] as List?)?.whereType<String>().toList() ??
          const [],
      properties:
          (json?['properties'] as List?)?.whereType<String>().toList() ??
          const [],
      writable:
          (json?['writable'] as List?)?.whereType<String>().toList() ??
          const [],
    );
  }

  final int? saveVersionNumber;
  final String? currentWorld;
  final String? playerName;
  final String? profileName;
  final PrivatePlayerTransform? transform;
  final List<PrivatePlayerAttribute> attributes;
  final List<String> scriptPaths;
  final List<String> properties;
  final List<String> writable;

  bool get hasData =>
      saveVersionNumber != null ||
      currentWorld != null ||
      playerName != null ||
      profileName != null ||
      transform != null ||
      attributes.isNotEmpty ||
      scriptPaths.isNotEmpty ||
      properties.isNotEmpty ||
      writable.isNotEmpty;
}

class PrivatePlayerTransform {
  const PrivatePlayerTransform({
    required this.location,
    required this.rotation,
  });

  static PrivatePlayerTransform? fromJson(Object? value) {
    if (value is! Map) return null;
    final location = PrivateVector3.fromJson(value['location']);
    final rotation = PrivateRotation.fromJson(value['rotation']);
    if (location == null || rotation == null) return null;
    return PrivatePlayerTransform(location: location, rotation: rotation);
  }

  final PrivateVector3 location;
  final PrivateRotation rotation;
}

class PrivateVector3 {
  const PrivateVector3({required this.x, required this.y, required this.z});

  static PrivateVector3? fromJson(Object? value) {
    if (value is! Map) return null;
    final x = (value['x'] as num?)?.toDouble();
    final y = (value['y'] as num?)?.toDouble();
    final z = (value['z'] as num?)?.toDouble();
    if (x == null || y == null || z == null) return null;
    return PrivateVector3(x: x, y: y, z: z);
  }

  final double x;
  final double y;
  final double z;
}

class PrivateRotation {
  const PrivateRotation({
    required this.pitch,
    required this.yaw,
    required this.roll,
  });

  static PrivateRotation? fromJson(Object? value) {
    if (value is! Map) return null;
    final pitch = (value['pitch'] as num?)?.toDouble();
    final yaw = (value['yaw'] as num?)?.toDouble();
    final roll = (value['roll'] as num?)?.toDouble();
    if (pitch == null || yaw == null || roll == null) return null;
    return PrivateRotation(pitch: pitch, yaw: yaw, roll: roll);
  }

  final double pitch;
  final double yaw;
  final double roll;
}

class PrivatePlayerAttribute {
  const PrivatePlayerAttribute({
    required this.id,
    this.baseValue,
    this.currentValue,
  });

  factory PrivatePlayerAttribute.fromJson(Map<Object?, Object?> json) {
    return PrivatePlayerAttribute(
      id: json['id'] as String? ?? '',
      baseValue: (json['baseValue'] as num?)?.toDouble(),
      currentValue: (json['currentValue'] as num?)?.toDouble(),
    );
  }

  final String id;
  final double? baseValue;
  final double? currentValue;
}

class PrivateInventorySummary {
  const PrivateInventorySummary({
    this.candidateCount = 0,
    this.candidates = const [],
    this.itemStackCount = 0,
    this.itemScope,
    this.items = const [],
    this.mainContainerPaths = const [],
    this.equippedArmorPaths = const [],
    this.scriptPaths = const [],
    this.properties = const [],
    this.writable = const [],
    this.misalignedSlots = 0,
  });

  factory PrivateInventorySummary.fromJson(Map<String, Object?>? json) {
    return PrivateInventorySummary(
      candidateCount: (json?['candidateCount'] as num?)?.toInt() ?? 0,
      candidates:
          (json?['candidates'] as List?)?.whereType<String>().toList() ??
          const [],
      itemStackCount: (json?['itemStackCount'] as num?)?.toInt() ?? 0,
      items:
          (json?['items'] as List?)
              ?.whereType<Map>()
              .map((value) => PrivateInventoryItem.fromJson(value))
              .toList() ??
          const [],
      itemScope: json?['itemScope'] as String?,
      mainContainerPaths:
          (json?['mainContainerPaths'] as List?)
              ?.whereType<String>()
              .toList() ??
          const [],
      equippedArmorPaths:
          (json?['equippedArmorPaths'] as List?)
              ?.whereType<String>()
              .toList() ??
          const [],
      scriptPaths:
          (json?['scriptPaths'] as List?)?.whereType<String>().toList() ??
          const [],
      properties:
          (json?['properties'] as List?)?.whereType<String>().toList() ??
          const [],
      writable:
          (json?['writable'] as List?)?.whereType<String>().toList() ??
          const [],
      misalignedSlots:
          ((json?['slotIntegrity'] as Map?)?['misalignedSlots'] as num?)
              ?.toInt() ??
          0,
    );
  }

  final int candidateCount;
  final List<String> candidates;
  final int itemStackCount;
  final String? itemScope;
  final List<PrivateInventoryItem> items;
  // Complete set of MainContainer item paths (uncapped), used to exclude
  // already-owned items from the add picker even when [items] is truncated.
  final List<String> mainContainerPaths;
  // Worn-armor item paths (uncapped, from the typed tree). Excluded from the add
  // picker so the user cannot add a duplicate of the currently-equipped armor —
  // which would make the equipped badge/upgrades ambiguous. Reliable even when
  // [items] is truncated and the worn row falls outside it.
  final List<String> equippedArmorPaths;
  final List<String> scriptPaths;
  final List<String> properties;
  final List<String> writable;

  /// Inventory slots anywhere in the save whose stored id no longer matches the
  /// position they sit in — damage an older version of this editor left behind.
  /// The game resolves a slot by that position, so it acts on the wrong item
  /// until the save is repaired.
  final int misalignedSlots;

  /// Whether the core offers the whole-save slot repair for this save. The
  /// WARNING keys off [misalignedSlots] alone — a save can be damaged without
  /// the repair being on offer, and the reader still needs to know.
  bool get canRepairSlots =>
      misalignedSlots > 0 && writable.contains('private.inventory.repairSlots');

  bool get hasData =>
      candidateCount > 0 ||
      candidates.isNotEmpty ||
      itemStackCount > 0 ||
      items.isNotEmpty ||
      scriptPaths.isNotEmpty ||
      properties.isNotEmpty;
}

class ArmorUpgrade {
  const ArmorUpgrade({required this.key, required this.value});
  final String key;
  final String value;
}

/// Result of loading a single NPC's inventory via `private.npc.inventory`.
/// The payload has the SAME shape as the player inventory summary, so it parses
/// straight into a [PrivateInventorySummary]; an inline [error] is carried
/// instead of throwing, mirroring [NpcAttributesResult].
class NpcInventoryResult {
  const NpcInventoryResult({
    this.inventory = const PrivateInventorySummary(),
    this.error,
  });

  factory NpcInventoryResult.fromJson(Map<String, Object?> json) {
    return NpcInventoryResult(
      inventory: PrivateInventorySummary.fromJson(json),
    );
  }

  final PrivateInventorySummary inventory;
  final String? error;
}

class PrivateInventoryItem {
  const PrivateInventoryItem({
    required this.id,
    required this.path,
    this.count,
    this.removable = false,
    this.equipped = false,
    this.upgrades = const [],
    this.slotId,
    this.containerType,
  });

  factory PrivateInventoryItem.fromJson(Map<Object?, Object?> json) {
    return PrivateInventoryItem(
      id: json['id'] as String? ?? '',
      path: json['path'] as String? ?? '',
      count: (json['count'] as num?)?.toInt(),
      removable: json['removable'] as bool? ?? false,
      equipped: json['equipped'] as bool? ?? false,
      upgrades:
          (json['upgrades'] as List?)
              ?.whereType<Map<Object?, Object?>>()
              .map(
                (u) => ArmorUpgrade(
                  key: u['key'] as String? ?? '',
                  value: u['value'] as String? ?? '',
                ),
              )
              .toList() ??
          const [],
      slotId: (json['slotId'] as num?)?.toInt(),
      containerType: json['containerType'] as String?,
    );
  }

  final String id;
  final String path;
  final int? count;
  // True only for rows in the player's MainContainer, which the core's
  // removeItem op can delete. Rows from other containers are not removable.
  final bool removable;

  /// True for the worn armor (the item in the player's ArmorSlot container).
  final bool equipped;

  /// Armor upgrade slots (part/tier pairs), populated only on the equipped
  /// armor row; empty for all other items.
  final List<ArmorUpgrade> upgrades;

  /// Stable per-slot discriminator (`m_Id`) from the core. Lets a count edit
  /// pin one specific stack when two slots share an item-definition path
  /// (NPC inventories with duplicate stacks). Null when the core did not emit
  /// one (older payloads / no slot id resolvable).
  final int? slotId;

  /// The container this row lives in (e.g. `MainContainer`, `MeleeSlot`,
  /// `Pouch`). NPC inventories surface multiple containers; the frontend MUST
  /// echo this back on count/remove edits so the core addresses the right
  /// container's slot. Player rows are all `MainContainer`; null on older
  /// payloads that did not emit it.
  final String? containerType;
}

class InventoryItemCountChange {
  const InventoryItemCountChange({
    required this.id,
    required this.path,
    required this.count,
    this.actorId,
    this.slotId,
    this.containerType,
  });

  final String id;
  final String path;
  final int count;

  /// GlobalId of the NPC whose inventory this edits, or null for the player's
  /// inventory. When non-null it is forwarded to the core so the edit targets
  /// the NPC's container instead of the player's.
  final String? actorId;

  /// Stable slot `m_Id` of the targeted stack, forwarded so the core can pin a
  /// specific slot when two share a path. Null for the player path (the core's
  /// untyped scan does not use it) and when the row carried no slot id.
  final int? slotId;

  /// The container the targeted slot lives in (e.g. `MeleeSlot`, `Pouch`),
  /// echoed back so the core addresses the right container. Null for the player
  /// (all MainContainer) and older payloads. Without it, non-MainContainer NPC
  /// edits fail to find the slot.
  final String? containerType;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.inventory.setItemCount',
      'value': {
        'id': id,
        'path': path,
        'count': count,
        if (actorId != null) 'actorId': actorId,
        if (slotId != null) 'slotId': slotId,
        if (containerType != null) 'containerType': containerType,
      },
    };
  }
}

class InventoryItemAdd {
  const InventoryItemAdd({
    required this.path,
    required this.count,
    this.actorId,
  });

  final String path;
  final int count;

  /// See [InventoryItemCountChange.actorId].
  final String? actorId;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.inventory.addItem',
      'value': {
        'path': path,
        'count': count,
        if (actorId != null) 'actorId': actorId,
      },
    };
  }
}

/// A queued "reset this actor's inventory to the game-start save" edit. Carries
/// the resolved Resources difficulty level (which start-save to pull from) and
/// the target actor (null = player, else the NPC's GlobalId).
class InventoryReset {
  const InventoryReset({required this.resourcesLevel, this.actorId});

  final String resourcesLevel;

  /// See [InventoryItemCountChange.actorId].
  final String? actorId;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.inventory.reset',
      'value': {
        'resourcesLevel': resourcesLevel,
        if (actorId != null) 'actorId': actorId,
      },
    };
  }
}

class InventoryItemRemove {
  const InventoryItemRemove({
    required this.path,
    this.actorId,
    this.slotId,
    this.containerType,
  });

  final String path;

  /// See [InventoryItemCountChange.actorId].
  final String? actorId;

  /// See [InventoryItemCountChange.slotId]. Forwarded so the core removes the
  /// exact slot when several stacks share a path in the same container.
  final int? slotId;

  /// See [InventoryItemCountChange.containerType]. Forwarded so the core removes
  /// from the right container (e.g. an NPC's Pouch/MeleeSlot, not MainContainer).
  final String? containerType;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.inventory.removeItem',
      'value': {
        'path': path,
        if (actorId != null) 'actorId': actorId,
        if (slotId != null) 'slotId': slotId,
        if (containerType != null) 'containerType': containerType,
      },
    };
  }
}

class BackupEntry {
  const BackupEntry({
    required this.path,
    required this.fileName,
    required this.fileSize,
    required this.sha1,
    required this.status,
    this.scope = 'save',
    this.createdEpoch,
    this.playerSaveName,
    this.slotName,
    this.name,
  });

  factory BackupEntry.fromJson(Map<Object?, Object?> json) {
    return BackupEntry(
      path: json['path'] as String? ?? '',
      fileName: json['fileName'] as String? ?? '',
      fileSize: (json['fileSize'] as num?)?.toInt() ?? 0,
      sha1: json['sha1'] as String? ?? '',
      createdEpoch: (json['createdEpoch'] as num?)?.toInt(),
      status: json['status'] as String? ?? 'unknown',
      scope: json['scope'] as String? ?? 'save',
      playerSaveName: json['playerSaveName'] as String?,
      slotName: json['slotName'] as String?,
      name: json['name'] as String?,
    );
  }

  final String path;
  final String fileName;
  final int fileSize;
  final String sha1;
  final int? createdEpoch;
  final String status;
  final String scope;
  final String? playerSaveName;
  final String? slotName;

  /// User-given label, kept beside the backup files. `null` when the user never
  /// named this backup — the file name is the title then.
  final String? name;

  /// What to head the entry with: the label when there is one, else the file
  /// name (which stays visible either way).
  String get title => name?.isNotEmpty == true ? name! : fileName;

  bool get canRestore =>
      (scope == 'save' || scope == 'persistent_data_list') && status == 'ok';
}

/// Status of the always-on in-process Oodle codec, parsed from the core's
/// `check_codec` response `data`. The old out-of-process codec-host shape
/// (per-backend probe arrays, severity, profile and resolution metadata) is
/// gone — the in-process codec is effectively always ready, so this carries
/// only the simplified backend/capability fields.
class CodecStatus {
  const CodecStatus({
    required this.backend,
    required this.available,
    required this.status,
    this.canDecompress = false,
    this.canCompress = false,
    this.adapter,
  });

  factory CodecStatus.fromJson(Map<String, Object?> json) {
    final details = (json['details'] as Map?)?.cast<String, Object?>();
    return CodecStatus(
      backend: json['backend'] as String? ?? 'unknown',
      available: json['available'] as bool? ?? false,
      status: json['status'] as String? ?? 'unknown',
      canDecompress: json['canDecompress'] as bool? ?? false,
      canCompress: json['canCompress'] as bool? ?? false,
      adapter: details?['adapter'] as String?,
    );
  }

  final String backend;
  final bool available;

  /// One of `ready` (decode + encode), `decode_only`, or `unavailable`.
  final String status;
  final bool canDecompress;
  final bool canCompress;
  final String? adapter;
}
