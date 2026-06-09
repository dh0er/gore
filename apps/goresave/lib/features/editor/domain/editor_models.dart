import 'dart:convert';

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

  String get displayName {
    final name = profileName?.trim();
    if (name == null || name.isEmpty) return 'Profile $profileId';
    return name == profileId.toString() ? 'Profile $name' : name;
  }
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
    this.privateProgression = const PrivateProgressionSummary(),
  });

  factory SaveInspection.fromJson(Map<String, Object?> json) {
    final public = (json['public'] as Map?)?.cast<String, Object?>();
    final persistent = (json['persistent'] as Map?)?.cast<String, Object?>();
    final stream = (json['compressedStream'] as Map?)?.cast<String, Object?>();
    final private = (json['private'] as Map?)?.cast<String, Object?>();
    final privatePlayer = (private?['player'] as Map?)?.cast<String, Object?>();
    final privateInventory = (private?['inventory'] as Map?)
        ?.cast<String, Object?>();
    final privateProgression = (private?['progression'] as Map?)
        ?.cast<String, Object?>();
    final privateStatus = private?['status'] as String?;
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
      privateStatus: privateStatus,
      privateDecoded:
          privateStatus == 'decoded' || privateStatus == 'decoded_preview',
      privateDecompressedSize: (private?['decompressedSize'] as num?)?.toInt(),
      privateStringCount: (private?['stringCount'] as num?)?.toInt(),
      privateStrings:
          (private?['strings'] as List?)?.whereType<String>().toList() ??
          const [],
      privatePreview: private?['preview'] as bool? ?? false,
      privateDecodedChunkCount: (private?['decodedChunkCount'] as num?)
          ?.toInt(),
      privateTotalChunkCount: (private?['totalChunkCount'] as num?)?.toInt(),
      privatePlayer: PrivatePlayerSummary.fromJson(privatePlayer),
      privateInventory: PrivateInventorySummary.fromJson(privateInventory),
      privateProgression: PrivateProgressionSummary.fromJson(
        privateProgression,
      ),
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
  final PrivateProgressionSummary privateProgression;
  final Map<String, Object?> raw;

  String prettyJson() {
    const encoder = JsonEncoder.withIndent('  ');
    return encoder.convert(raw);
  }
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
    this.scriptPaths = const [],
    this.properties = const [],
    this.writable = const [],
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

  final int candidateCount;
  final List<String> candidates;
  final int itemStackCount;
  final String? itemScope;
  final List<PrivateInventoryItem> items;
  final List<String> scriptPaths;
  final List<String> properties;
  final List<String> writable;

  bool get hasData =>
      candidateCount > 0 ||
      candidates.isNotEmpty ||
      itemStackCount > 0 ||
      items.isNotEmpty ||
      scriptPaths.isNotEmpty ||
      properties.isNotEmpty;
}

class PrivateInventoryItem {
  const PrivateInventoryItem({
    required this.id,
    required this.path,
    this.count,
  });

  factory PrivateInventoryItem.fromJson(Map<Object?, Object?> json) {
    return PrivateInventoryItem(
      id: json['id'] as String? ?? '',
      path: json['path'] as String? ?? '',
      count: (json['count'] as num?)?.toInt(),
    );
  }

  final String id;
  final String path;
  final int? count;
}

class PrivateProgressionSummary {
  const PrivateProgressionSummary({
    this.candidateCount = 0,
    this.candidates = const [],
    this.gameplayTags = const [],
    this.sections = const [],
    this.scriptPaths = const [],
    this.properties = const [],
    this.writable = const [],
  });

  factory PrivateProgressionSummary.fromJson(Map<String, Object?>? json) {
    return PrivateProgressionSummary(
      candidateCount: (json?['candidateCount'] as num?)?.toInt() ?? 0,
      candidates:
          (json?['candidates'] as List?)?.whereType<String>().toList() ??
          const [],
      gameplayTags:
          (json?['gameplayTags'] as List?)?.whereType<String>().toList() ??
          const [],
      sections:
          (json?['sections'] as List?)?.whereType<String>().toList() ??
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

  final int candidateCount;
  final List<String> candidates;
  final List<String> gameplayTags;
  final List<String> sections;
  final List<String> scriptPaths;
  final List<String> properties;
  final List<String> writable;

  bool get hasData =>
      candidateCount > 0 ||
      candidates.isNotEmpty ||
      gameplayTags.isNotEmpty ||
      sections.isNotEmpty ||
      scriptPaths.isNotEmpty ||
      properties.isNotEmpty;
}

class InventoryItemCountChange {
  const InventoryItemCountChange({
    required this.id,
    required this.path,
    required this.count,
  });

  final String id;
  final String path;
  final int count;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.inventory.setItemCount',
      'value': {'id': id, 'path': path, 'count': count},
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

  bool get canRestore => scope == 'save' && status == 'ok';
}

class CodecStatus {
  const CodecStatus({
    required this.available,
    required this.status,
    required this.message,
    this.canDecompress = false,
    this.canCompress = false,
    this.adapter,
    this.selectedBackend,
    this.profile,
    this.resolutionMode,
  });

  factory CodecStatus.fromJson(Map<String, Object?> json) {
    return CodecStatus(
      available: json['available'] as bool? ?? false,
      status: json['status'] as String? ?? 'unknown',
      message: json['message'] as String? ?? '',
      canDecompress: json['canDecompress'] as bool? ?? false,
      canCompress: json['canCompress'] as bool? ?? false,
      adapter: json['adapter'] as String?,
      selectedBackend: json['selectedBackend'] as String?,
      profile: json['profile'] as String?,
      resolutionMode: json['resolutionMode'] as String?,
    );
  }

  final bool available;
  final String status;
  final String message;
  final bool canDecompress;
  final bool canCompress;
  final String? adapter;
  final String? selectedBackend;
  final String? profile;
  final String? resolutionMode;
}
