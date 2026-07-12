import 'core_service.dart';

/// Typed wrappers over the gore-ffi commands for audio, read-only voice inspection, stateless
/// authoring checks, and unified mod build/deploy.
class ModFfi {
  ModFfi(this._core);
  final GoreCoreFfiService _core;

  Future<Map<String, Object?>> _call(
    String cmd,
    Map<String, Object?> payload,
  ) async {
    final r = await _core.execute(cmd, payload: payload);
    if (r['ok'] != true) {
      final e = r['error'];
      final msg = e is Map ? (e['message'] ?? e.toString()) : 'unknown error';
      throw ModFfiException('$cmd: $msg');
    }
    return r;
  }

  Future<List<AudioSampleInfo>> audioList(String bank, {String? key}) async {
    final payload = <String, Object?>{'bank': bank};
    if (key != null) payload['key'] = key;
    final r = await _call('audio_list', payload);
    final list = (r['samples'] as List?) ?? const [];
    return list
        .whereType<Map>()
        .map((m) => AudioSampleInfo.fromJson(m.cast<String, Object?>()))
        .toList();
  }

  /// Extract one sample to a temp .ogg; returns its path.
  Future<String> audioExtract(String bank, String sample, {String? key}) async {
    final payload = <String, Object?>{'bank': bank, 'sample': sample};
    if (key != null) payload['key'] = key;
    final r = await _call('audio_extract', payload);
    return r['ogg_path'] as String;
  }

  /// Resolve every eligible exact ASCII case-insensitive `${locId}.ogg` basename in one read-only
  /// voice snapshot.
  /// Ambiguous matches are returned in full and are never selected by the backend.
  Future<VoiceArchiveMatchLineResult> voiceArchiveMatchLine({
    required String archive,
    required String locId,
  }) async {
    final r = await _call('voice_archive_match_line', {
      'archive': archive,
      'loc_id': locId,
    });
    return VoiceArchiveMatchLineResult.fromJson(r);
  }

  /// Parse, canonicalize, and validate one format-2 authoring snapshot without retaining state.
  Future<AuthoringProjectCheckResult> authoringProjectCheck({
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    final r = await _call('authoring_project_check', {
      'project_json': projectJson,
      'profile': profile.wireName,
    });
    return AuthoringProjectCheckResult.fromJson(r);
  }

  /// Build the unified bundle into `outDir`; returns the bundle dir.
  Future<String> modBuild(Map<String, Object?> spec, String outDir) async {
    final r = await _call('mod_build', {'out_dir': outDir, 'spec': spec});
    return r['bundle_dir'] as String;
  }

  Future<void> modDeploy(String bundleDir, String gameRoot) =>
      _call('mod_deploy', {'bundle_dir': bundleDir, 'game_root': gameRoot});

  /// Undeploy the active mod. Returns true if a deployment was actually undone, false if nothing
  /// was deployed (the FFI returns a null record in that case).
  Future<bool> modUndeploy(String gameRoot) async {
    final r = await _call('mod_undeploy', {'game_root': gameRoot});
    return r['record'] != null;
  }

  /// Load (or build, if absent/`rebuild`) the texture index. Returns {assetPath: packageIdString}.
  Future<Map<String, String>> textureIndex(
    String game, {
    bool rebuild = false,
  }) async {
    final r = await _call('texture_index', {'game': game, 'rebuild': rebuild});
    final entries = (r['entries'] as Map).cast<String, Object?>();
    return entries.map((k, v) => MapEntry(k, v as String));
  }

  /// Extract a texture to a temp PNG; returns the FFI result map (png_path, width, height, format).
  Future<Map<String, Object?>> textureExtract(
    String game, {
    String? asset,
    String? packageId,
  }) async {
    final payload = <String, Object?>{'game': game};
    if (asset != null) payload['asset'] = asset;
    if (packageId != null) payload['package_id'] = packageId;
    return _call('texture_extract', payload);
  }

  /// Auto-detect the game install via Steam; returns the exe path hint, or null.
  Future<String?> findGameExe() async {
    final r = await _call('find_game', const {});
    if (r['found'] != true) return null;
    return r['exe'] as String?;
  }

  /// List modules in a precompiled script cache: [{name, file}].
  Future<List<ScriptModuleInfo>> scriptListModules(String cache) async {
    final r = await _call('script_list_modules', {'cache': cache});
    final list = (r['modules'] as List?) ?? const [];
    return list
        .whereType<Map>()
        .map((m) => ScriptModuleInfo.fromJson(m.cast<String, Object?>()))
        .toList();
  }

  /// Emit recompilable .as source for one module.
  Future<String> scriptEmitModule(String cache, String module) async {
    final r = await _call('script_emit_module', {
      'cache': cache,
      'module': module,
    });
    return r['source'] as String;
  }

  /// Compile a staged .as into a 1-module mini-cache via the game; returns {mini_path, module}.
  Future<Map<String, Object?>> scriptCompile({
    required String gameDir,
    required String op,
    required String moduleName,
    required String relPath,
    required String asPath,
    required String workDir,
    bool allowNewSymbols = false,
  }) => _call('script_compile', {
    'game_dir': gameDir,
    'op': op,
    'module_name': moduleName,
    'rel_path': relPath,
    'as_path': asPath,
    'work_dir': workDir,
    'allow_new_symbols': allowNewSymbols,
  });
}

class ModFfiException implements Exception {
  ModFfiException(this.message);
  final String message;
  @override
  String toString() => message;
}

class AudioSampleInfo {
  AudioSampleInfo({
    required this.index,
    required this.name,
    required this.freq,
    required this.channels,
    required this.seconds,
  });
  final int index;
  final String name;
  final int freq;
  final int channels;
  final double seconds;

  factory AudioSampleInfo.fromJson(Map<String, Object?> j) => AudioSampleInfo(
    index: (j['index'] as num).toInt(),
    name: j['name'] as String,
    freq: (j['freq'] as num).toInt(),
    channels: (j['channels'] as num).toInt(),
    seconds: (j['seconds'] as num).toDouble(),
  );
}

enum AuthoringValidationProfile {
  production('production'),
  experimental('experimental');

  const AuthoringValidationProfile(this.wireName);
  final String wireName;
}

enum AuthoringDiagnosticSeverity { error, warning, info }

final _authoringDiagnosticCodePattern = RegExp(
  r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$',
);
final _authoringEntityIdPattern = RegExp(r'^[0-9a-f]{32}$');

String _authoringRequiredString(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! String || value.isEmpty) {
    throw FormatException(
      'authoring response field $field is not a valid string',
    );
  }
  return value;
}

bool _authoringRequiredBool(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! bool) {
    throw FormatException('authoring response field $field is not a bool');
  }
  return value;
}

String? _authoringRequiredNullableString(
  Map<String, Object?> json,
  String field,
) {
  if (!json.containsKey(field)) {
    throw FormatException('authoring response is missing field $field');
  }
  final value = json[field];
  if (value == null) return null;
  if (value is! String || value.isEmpty) {
    throw FormatException(
      'authoring response field $field is not a valid nullable string',
    );
  }
  return value;
}

Map<String, Object?> _authoringRequiredObject(Object? value, String context) {
  if (value is! Map) {
    throw FormatException('authoring response $context is not an object');
  }
  final object = <String, Object?>{};
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw FormatException(
        'authoring response $context contains a non-string key',
      );
    }
    object[entry.key as String] = entry.value;
  }
  return object;
}

String _authoringEntityId(String value, String field) {
  if (!_authoringEntityIdPattern.hasMatch(value)) {
    throw FormatException(
      'authoring response field $field is not a canonical entity ID',
    );
  }
  return value;
}

class AuthoringProjectCheckResult {
  const AuthoringProjectCheckResult._({
    required this.canonicalProjectJson,
    required this.diagnostics,
    required this.blocksBuild,
  });

  final String canonicalProjectJson;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringProjectCheckResult.fromJson(Map<String, Object?> json) {
    final canonicalProjectJson = _authoringRequiredString(
      json,
      'canonical_project_json',
    );
    final rawDiagnostics = json['diagnostics'];
    if (rawDiagnostics is! List) {
      throw const FormatException(
        'authoring response diagnostics is not an array',
      );
    }
    final diagnostics = <AuthoringDiagnostic>[];
    for (var index = 0; index < rawDiagnostics.length; index++) {
      diagnostics.add(
        AuthoringDiagnostic.fromJson(
          _authoringRequiredObject(
            rawDiagnostics[index],
            'diagnostic at index $index',
          ),
        ),
      );
    }

    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    if (blocksBuild !=
        diagnostics.any((diagnostic) => diagnostic.blocksBuild)) {
      throw const FormatException(
        'authoring response blocks_build is inconsistent with diagnostics',
      );
    }
    return AuthoringProjectCheckResult._(
      canonicalProjectJson: canonicalProjectJson,
      diagnostics: List.unmodifiable(diagnostics),
      blocksBuild: blocksBuild,
    );
  }
}

class AuthoringDiagnostic {
  const AuthoringDiagnostic._({
    required this.code,
    required this.severity,
    required this.entity,
    required this.propertyPath,
    required this.message,
    required this.relatedEntities,
    required this.blocksBuild,
  });

  final String code;
  final AuthoringDiagnosticSeverity severity;
  final String? entity;
  final String? propertyPath;
  final String message;
  final List<String> relatedEntities;
  final bool blocksBuild;

  factory AuthoringDiagnostic.fromJson(Map<String, Object?> json) {
    final code = _authoringRequiredString(json, 'code');
    if (!_authoringDiagnosticCodePattern.hasMatch(code)) {
      throw const FormatException('authoring diagnostic code is not canonical');
    }
    final severity = switch (json['severity']) {
      'error' => AuthoringDiagnosticSeverity.error,
      'warning' => AuthoringDiagnosticSeverity.warning,
      'info' => AuthoringDiagnosticSeverity.info,
      final value => throw FormatException(
        'unknown authoring diagnostic severity: $value',
      ),
    };
    final entityValue = _authoringRequiredNullableString(json, 'entity');
    final entity = entityValue == null
        ? null
        : _authoringEntityId(entityValue, 'entity');
    final propertyPath = _authoringRequiredNullableString(
      json,
      'property_path',
    );
    final message = _authoringRequiredString(json, 'message');

    final rawRelatedEntities = json['related_entities'];
    if (rawRelatedEntities is! List) {
      throw const FormatException(
        'authoring diagnostic related_entities is not an array',
      );
    }
    final relatedEntities = <String>[];
    for (var index = 0; index < rawRelatedEntities.length; index++) {
      final value = rawRelatedEntities[index];
      if (value is! String) {
        throw FormatException(
          'authoring diagnostic related entity at index $index is not a string',
        );
      }
      final id = _authoringEntityId(value, 'related_entities[$index]');
      if (relatedEntities.isNotEmpty &&
          relatedEntities.last.compareTo(id) >= 0) {
        throw const FormatException(
          'authoring diagnostic related entities are not canonical and unique',
        );
      }
      relatedEntities.add(id);
    }

    return AuthoringDiagnostic._(
      code: code,
      severity: severity,
      entity: entity,
      propertyPath: propertyPath,
      message: message,
      relatedEntities: List.unmodifiable(relatedEntities),
      blocksBuild: _authoringRequiredBool(json, 'blocks_build'),
    );
  }
}

enum VoiceArchiveLineResolution { unresolved, unique, ambiguous }

String _voiceRequiredString(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! String) {
    throw FormatException('voice response field $field is not a string');
  }
  return value;
}

int _voiceRequiredInt(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int? max,
}) {
  final value = json[field];
  if (value is! int || value < min || (max != null && value > max)) {
    throw FormatException(
      'voice response field $field is not an integer in range '
      '$min..${max ?? 'unbounded'}',
    );
  }
  return value;
}

int? _voiceOptionalInt(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int? max,
}) {
  if (json[field] == null) return null;
  return _voiceRequiredInt(json, field, min: min, max: max);
}

bool _voiceRequiredBool(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! bool) {
    throw FormatException('voice response field $field is not a bool');
  }
  return value;
}

bool _isAscii(String value) => value.codeUnits.every((unit) => unit <= 0x7f);

bool _asciiCaseEquals(String left, String right) =>
    _isAscii(left) &&
    _isAscii(right) &&
    left.toLowerCase() == right.toLowerCase();

class VoiceArchiveMatchLineResult {
  const VoiceArchiveMatchLineResult({
    required this.archive,
    required this.archiveSize,
    required this.archiveSha256,
    required this.locId,
    required this.expectedBasename,
    required this.resolution,
    required this.matches,
  });

  final String archive;
  final int archiveSize;
  final String archiveSha256;
  final String locId;
  final String expectedBasename;
  final VoiceArchiveLineResolution resolution;
  final List<VoiceArchiveEntryInfo> matches;

  factory VoiceArchiveMatchLineResult.fromJson(Map<String, Object?> j) {
    final archive = _voiceRequiredString(j, 'archive');
    final archiveSize = _voiceRequiredInt(j, 'archive_size');
    final locId = _voiceRequiredString(j, 'loc_id');
    if (!_isAscii(locId)) {
      throw const FormatException(
        'voice match response has a non-ASCII loc_id',
      );
    }
    final expectedBasename = _voiceRequiredString(j, 'expected_basename');
    if (expectedBasename != '$locId.ogg') {
      throw const FormatException(
        'voice match expected_basename does not equal loc_id + .ogg',
      );
    }

    final rawMatches = j['matches'];
    if (rawMatches is! List) {
      throw const FormatException('voice match response has no matches array');
    }
    final matches = <VoiceArchiveEntryInfo>[];
    for (final rawMatch in rawMatches) {
      if (rawMatch is! Map) {
        throw const FormatException(
          'voice match response contains a non-object match',
        );
      }
      final match = VoiceArchiveEntryInfo.fromJson(
        rawMatch.cast<String, Object?>(),
      );
      if (!_asciiCaseEquals(match.basename, expectedBasename)) {
        throw const FormatException(
          'voice match basename does not match expected_basename',
        );
      }
      final pathParts = match.path.split('/');
      if (match.path.isEmpty ||
          match.path.contains('\\') ||
          pathParts.isEmpty ||
          pathParts.last != match.basename) {
        throw const FormatException(
          'voice match basename is inconsistent with its entry path',
        );
      }
      if (match.isDirectory || match.isSymlink || match.encrypted) {
        throw const FormatException(
          'voice match response contains an ineligible entry',
        );
      }
      final expectedCompression = switch (match.compressionCode) {
        0 => 'stored',
        8 => 'deflated',
        _ => throw const FormatException(
          'voice match response contains unsupported compression',
        ),
      };
      if (match.compression != expectedCompression) {
        throw const FormatException(
          'voice match compression label/code mismatch',
        );
      }
      matches.add(match);
    }

    final resolution = switch (j['resolution']) {
      'unresolved' => VoiceArchiveLineResolution.unresolved,
      'unique' => VoiceArchiveLineResolution.unique,
      'ambiguous' => VoiceArchiveLineResolution.ambiguous,
      final value => throw FormatException(
        'unknown voice line resolution: $value',
      ),
    };
    final matchCount = _voiceRequiredInt(j, 'match_count');
    final countMatchesResolution = switch (resolution) {
      VoiceArchiveLineResolution.unresolved => matchCount == 0,
      VoiceArchiveLineResolution.unique => matchCount == 1,
      VoiceArchiveLineResolution.ambiguous => matchCount > 1,
    };
    if (matchCount != matches.length || !countMatchesResolution) {
      throw FormatException(
        'voice match count/resolution mismatch: '
        '$matchCount/${matches.length}/$resolution',
      );
    }

    final archiveSha256 = _voiceRequiredString(j, 'archive_sha256');
    if (!RegExp(r'^[0-9a-f]{64}$').hasMatch(archiveSha256)) {
      throw const FormatException(
        'voice match response has an invalid archive SHA-256',
      );
    }
    return VoiceArchiveMatchLineResult(
      archive: archive,
      archiveSize: archiveSize,
      archiveSha256: archiveSha256,
      locId: locId,
      expectedBasename: expectedBasename,
      resolution: resolution,
      matches: List.unmodifiable(matches),
    );
  }
}

class VoiceArchiveEntryInfo {
  const VoiceArchiveEntryInfo({
    required this.index,
    required this.path,
    required this.basename,
    required this.compressedSize,
    required this.uncompressedSize,
    required this.crc32,
    required this.compression,
    required this.compressionCode,
    required this.lastModified,
    required this.unixMode,
    required this.isDirectory,
    required this.isSymlink,
    required this.encrypted,
  });

  final int index;
  final String path;
  final String basename;
  final int compressedSize;
  final int uncompressedSize;
  final int crc32;
  final String compression;
  final int compressionCode;
  final VoiceArchiveEntryTimestamp? lastModified;
  final int? unixMode;
  final bool isDirectory;
  final bool isSymlink;
  final bool encrypted;

  factory VoiceArchiveEntryInfo.fromJson(Map<String, Object?> j) {
    final rawLastModified = j['last_modified'];
    final lastModified = switch (rawLastModified) {
      null => null,
      final Map value => VoiceArchiveEntryTimestamp.fromJson(
        value.cast<String, Object?>(),
      ),
      _ => throw const FormatException(
        'voice archive entry has invalid last_modified metadata',
      ),
    };
    return VoiceArchiveEntryInfo(
      index: _voiceRequiredInt(j, 'index'),
      path: _voiceRequiredString(j, 'path'),
      basename: _voiceRequiredString(j, 'basename'),
      compressedSize: _voiceRequiredInt(j, 'compressed_size'),
      uncompressedSize: _voiceRequiredInt(j, 'uncompressed_size'),
      crc32: _voiceRequiredInt(j, 'crc32', max: 0xffffffff),
      compression: _voiceRequiredString(j, 'compression'),
      compressionCode: _voiceRequiredInt(j, 'compression_code', max: 0xffff),
      lastModified: lastModified,
      unixMode: _voiceOptionalInt(j, 'unix_mode', max: 0xffffffff),
      isDirectory: _voiceRequiredBool(j, 'is_directory'),
      isSymlink: _voiceRequiredBool(j, 'is_symlink'),
      encrypted: _voiceRequiredBool(j, 'encrypted'),
    );
  }
}

class VoiceArchiveEntryTimestamp {
  const VoiceArchiveEntryTimestamp({
    required this.year,
    required this.month,
    required this.day,
    required this.hour,
    required this.minute,
    required this.second,
  });

  final int year;
  final int month;
  final int day;
  final int hour;
  final int minute;
  final int second;

  factory VoiceArchiveEntryTimestamp.fromJson(Map<String, Object?> j) {
    final year = _voiceRequiredInt(j, 'year', min: 1980, max: 2107);
    final month = _voiceRequiredInt(j, 'month', min: 1, max: 12);
    final day = _voiceRequiredInt(j, 'day', min: 1, max: 31);
    final hour = _voiceRequiredInt(j, 'hour', max: 23);
    final minute = _voiceRequiredInt(j, 'minute', max: 59);
    final second = _voiceRequiredInt(j, 'second', max: 59);
    final timestamp = DateTime.utc(year, month, day, hour, minute, second);
    if (timestamp.year != year ||
        timestamp.month != month ||
        timestamp.day != day ||
        timestamp.hour != hour ||
        timestamp.minute != minute ||
        timestamp.second != second) {
      throw const FormatException(
        'voice archive entry has an invalid calendar timestamp',
      );
    }
    return VoiceArchiveEntryTimestamp(
      year: year,
      month: month,
      day: day,
      hour: hour,
      minute: minute,
      second: second,
    );
  }
}

class ScriptModuleInfo {
  ScriptModuleInfo({required this.name, required this.file});
  final String name;
  final String file;
  factory ScriptModuleInfo.fromJson(Map<String, Object?> j) => ScriptModuleInfo(
    name: j['name'] as String,
    file: (j['file'] as String?) ?? '',
  );
}
