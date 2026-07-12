import 'dart:convert';

import 'core_service.dart';

const _maxNativeErrorCodeLength = 128;
const _maxNativeErrorMessageLength = 64 * 1024;
const _maxAuthoringStorePathBytes = 32 * 1024;
const _maxAuthoringHeadJsonBytes = 64 * 1024;
const _maxAuthoringProjectJsonBytes = 16 * 1024 * 1024;
const _maxAuthoringLogicalNameBytes = 1024;
const _maxAuthoringReferencedAssetBytes = 64 * 1024 * 1024;
const _maxAuthoringDiagnostics = 262144;
const _maxAuthoringDiagnosticMessageBytes = 4096;
const _maxAuthoringDiagnosticPathBytes = 4096;
const _maxAuthoringRelatedEntities = 100000;
final _nativeErrorCodePattern = RegExp(r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$');

/// Typed wrappers over the gore-ffi commands for audio, read-only voice inspection, stateless
/// authoring checks, and unified mod build/deploy.
class ModFfi {
  ModFfi(this._core);
  final GoreCoreFfiService _core;

  Future<Map<String, Object?>> _call(
    String cmd,
    Map<String, Object?> payload,
  ) async {
    final Map<String, Object?> r;
    try {
      r = await _core.execute(cmd, payload: payload);
    } on FormatException {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'response could not be decoded',
      );
    }

    final ok = r['ok'];
    if (ok == true) return r;
    if (ok != false) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field ok must be a bool',
      );
    }
    if (r.length != 2 || !r.containsKey('error')) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'error response has an invalid schema',
      );
    }

    final error = r['error'];
    if (error is! Map ||
        error.length != 2 ||
        !error.containsKey('code') ||
        !error.containsKey('message')) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field error has an invalid schema',
      );
    }
    final code = error['code'];
    if (code is! String ||
        code.isEmpty ||
        utf8.encode(code).length > _maxNativeErrorCodeLength ||
        !_nativeErrorCodePattern.hasMatch(code)) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field error.code is invalid',
      );
    }
    final message = error['message'];
    if (message is! String ||
        utf8.encode(message).length > _maxNativeErrorMessageLength ||
        message.trim().isEmpty) {
      throw ModFfiException._malformed(
        command: cmd,
        reason: 'field error.message is invalid',
      );
    }
    throw ModFfiException(command: cmd, code: code, message: message);
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

  /// Open the fixed canonical head and reconstruct one immutable format-2 checkpoint.
  Future<AuthoringStoreOpenedResult> authoringStoreOpen({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open', {
      'root': root,
      'verification': verification.wireName,
      'profile': profile.wireName,
    });
    return AuthoringStoreOpenedResult.fromJson(response);
  }

  /// Prepare immutable objects without publishing `gore-project.json`.
  ///
  /// `expectedHead == null` is a strict CAS assertion that the fixed head is absent. The raw
  /// project string is passed through unchanged so Rust can reject duplicate JSON keys.
  Future<AuthoringCheckpointPreparation> authoringStorePrepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    _authoringStoreRequestString(
      projectJson,
      'projectJson',
      _maxAuthoringProjectJsonBytes,
    );
    final response = await _call('authoring_store_prepare_checkpoint', {
      'root': root,
      'expected_head_json': expectedHead?.canonicalJson,
      'project_json': projectJson,
      'profile': profile.wireName,
    });
    return AuthoringCheckpointPreparation.fromJson(response);
  }

  /// Reopen one candidate head from its exact canonical UTF-8 JSON bytes without publishing it.
  Future<AuthoringStoreOpenedResult> authoringStoreOpenHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open_head_bytes', {
      'root': root,
      'head_json': head.canonicalJson,
      'verification': verification.wireName,
      'profile': profile.wireName,
    });
    return AuthoringStoreOpenedResult.fromJson(response);
  }

  /// Stream one bounded Ogg into the content-addressed store under a strict head CAS token.
  Future<AuthoringImportedOgg> authoringStoreImportOgg({
    required String root,
    required String source,
    required String logicalName,
    required AuthoringWorkingHead? expectedHead,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    _authoringStoreRequestString(source, 'source', _maxAuthoringStorePathBytes);
    _authoringStoreRequestString(
      logicalName,
      'logicalName',
      _maxAuthoringLogicalNameBytes,
    );
    final response = await _call('authoring_store_import_ogg', {
      'root': root,
      'source': source,
      'logical_name': logicalName,
      'expected_head_json': expectedHead?.canonicalJson,
    });
    return AuthoringImportedOgg.fromJson(response);
  }

  /// Verify that one typed logical asset reference resolves to its sealed immutable object.
  Future<void> authoringStoreVerifyAsset({
    required String root,
    required AuthoringAssetRef asset,
    required AuthoringAssetVerification verification,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_verify_asset', {
      'root': root,
      'asset': asset.toJson(),
      'verification': verification.wireName,
    });
    _authoringExactFields(response, const {'ok'}, 'verify response');
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
  const ModFfiException({
    required this.command,
    required this.code,
    required this.message,
  });

  const ModFfiException._malformed({
    required this.command,
    required String reason,
  }) : code = malformedNativeResponseCode,
       message = 'malformed native response: $reason';

  /// Local code used when gore_ffi does not return its documented error shape.
  static const malformedNativeResponseCode = 'MALFORMED_NATIVE_RESPONSE';

  final String command;
  final String code;
  final String message;

  @override
  String toString() => '$command: $message [$code]';
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

enum AuthoringAssetVerification {
  structural('structural'),
  full('full');

  const AuthoringAssetVerification(this.wireName);
  final String wireName;
}

enum AuthoringDiagnosticSeverity { error, warning, info }

final _authoringDiagnosticCodePattern = RegExp(
  r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$',
);
final _authoringEntityIdPattern = RegExp(r'^[0-9a-f]{32}$');
final _authoringSha256Pattern = RegExp(r'^[0-9a-f]{64}$');
const _authoringProjectTopLevelFields = <String>[
  'format',
  'schema_revision',
  'project_id',
  'revision',
  'meta',
  'target',
  'authoring_locales',
  'entities',
  'asset_store',
];

void _authoringStoreRequestString(String value, String field, int maxBytes) {
  if (value.isEmpty || utf8.encode(value).length > maxBytes) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'must be 1..=$maxBytes UTF-8 bytes',
    );
  }
}

void _authoringExactFields(
  Map<String, Object?> json,
  Set<String> expected,
  String context,
) {
  if (json.length != expected.length || !expected.every(json.containsKey)) {
    throw FormatException('authoring $context has an invalid schema');
  }
}

String _authoringRequiredString(
  Map<String, Object?> json,
  String field, {
  int maxBytes = _maxAuthoringProjectJsonBytes,
}) {
  final value = json[field];
  if (value is! String ||
      value.isEmpty ||
      utf8.encode(value).length > maxBytes) {
    throw FormatException(
      'authoring response field $field is not a bounded string',
    );
  }
  return value;
}

int _authoringRequiredInt(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int? max,
}) {
  final value = json[field];
  if (value is! int || value < min || (max != null && value > max)) {
    throw FormatException(
      'authoring response field $field is not an integer in range',
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
  String field, {
  int maxBytes = _maxAuthoringDiagnosticPathBytes,
}) {
  if (!json.containsKey(field)) {
    throw FormatException('authoring response is missing field $field');
  }
  final value = json[field];
  if (value == null) return null;
  if (value is! String ||
      value.isEmpty ||
      utf8.encode(value).length > maxBytes) {
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

void _authoringRequireCanonicalProjectJson(String projectJson) {
  final Object? decoded;
  try {
    decoded = jsonDecode(projectJson);
  } on FormatException {
    throw const FormatException('authoring store project JSON is invalid');
  }
  final project = _authoringRequiredObject(decoded, 'store project');
  final fields = project.keys.toList(growable: false);
  if (fields.length != _authoringProjectTopLevelFields.length) {
    throw const FormatException(
      'authoring store project JSON has an invalid top-level schema',
    );
  }
  for (var index = 0; index < fields.length; index++) {
    if (fields[index] != _authoringProjectTopLevelFields[index]) {
      throw const FormatException(
        'authoring store project JSON has non-canonical field order',
      );
    }
  }
  if (jsonEncode(decoded) != projectJson) {
    throw const FormatException(
      'authoring store project JSON is not canonical',
    );
  }
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
    _authoringExactFields(json, const {
      'ok',
      'canonical_project_json',
      'diagnostics',
      'blocks_build',
    }, 'project-check response');
    if (json['ok'] != true) {
      throw const FormatException('authoring project-check response is not ok');
    }
    final canonicalProjectJson = _authoringRequiredString(
      json,
      'canonical_project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
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
    _authoringExactFields(json, const {
      'code',
      'severity',
      'entity',
      'property_path',
      'message',
      'related_entities',
      'blocks_build',
    }, 'diagnostic');
    final code = _authoringRequiredString(json, 'code', maxBytes: 128);
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
      maxBytes: _maxAuthoringDiagnosticPathBytes,
    );
    final message = _authoringRequiredString(
      json,
      'message',
      maxBytes: _maxAuthoringDiagnosticMessageBytes,
    );

    final rawRelatedEntities = json['related_entities'];
    if (rawRelatedEntities is! List ||
        rawRelatedEntities.length > _maxAuthoringRelatedEntities) {
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

List<AuthoringDiagnostic> _authoringDiagnostics(Map<String, Object?> json) {
  final rawDiagnostics = json['diagnostics'];
  if (rawDiagnostics is! List ||
      rawDiagnostics.length > _maxAuthoringDiagnostics) {
    throw const FormatException(
      'authoring response diagnostics is not a bounded array',
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
  return List.unmodifiable(diagnostics);
}

void _authoringValidateBlocksBuild(
  bool blocksBuild,
  List<AuthoringDiagnostic> diagnostics,
) {
  if (blocksBuild != diagnostics.any((diagnostic) => diagnostic.blocksBuild)) {
    throw const FormatException(
      'authoring response blocks_build is inconsistent with diagnostics',
    );
  }
}

class AuthoringWorkingHead {
  const AuthoringWorkingHead._({
    required this.canonicalJson,
    required this.snapshotByteLength,
    required this.snapshotSha256,
  });

  /// Exact canonical UTF-8 bytes represented as a Dart string. Do not re-encode this from fields
  /// when using it as a compare-and-swap token.
  final String canonicalJson;
  final int snapshotByteLength;
  final String snapshotSha256;

  factory AuthoringWorkingHead.fromCanonicalJson(String value) {
    if (value.isEmpty ||
        utf8.encode(value).length > _maxAuthoringHeadJsonBytes) {
      throw const FormatException(
        'authoring head JSON is empty or exceeds its size limit',
      );
    }
    final Object? decoded;
    try {
      decoded = jsonDecode(value);
    } on FormatException {
      throw const FormatException('authoring head JSON is invalid');
    }
    final object = _authoringRequiredObject(decoded, 'head');
    _authoringExactFields(object, const {'store_format', 'snapshot'}, 'head');
    if (object['store_format'] != 1) {
      throw const FormatException('authoring head store_format is not 1');
    }
    final snapshot = _authoringRequiredObject(
      object['snapshot'],
      'head snapshot',
    );
    _authoringExactFields(snapshot, const {
      'byte_len',
      'sha256',
    }, 'head snapshot');
    final byteLength = _authoringRequiredInt(
      snapshot,
      'byte_len',
      min: 1,
      max: _maxAuthoringProjectJsonBytes,
    );
    final sha256 = _authoringRequiredString(snapshot, 'sha256', maxBytes: 64);
    if (!_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException('authoring head SHA-256 is not canonical');
    }
    if (jsonEncode(object) != value) {
      throw const FormatException('authoring head JSON is not canonical');
    }
    return AuthoringWorkingHead._(
      canonicalJson: value,
      snapshotByteLength: byteLength,
      snapshotSha256: sha256,
    );
  }
}

class AuthoringStoreOpenedResult {
  const AuthoringStoreOpenedResult._({
    required this.head,
    required this.projectJson,
    required this.diagnostics,
    required this.blocksBuild,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringStoreOpenedResult.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'ok',
      'head_json',
      'project_json',
      'diagnostics',
      'blocks_build',
    }, 'store-open response');
    if (json['ok'] != true) {
      throw const FormatException('authoring store-open response is not ok');
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectJson = _authoringRequiredString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    _authoringRequireCanonicalProjectJson(projectJson);
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
    return AuthoringStoreOpenedResult._(
      head: head,
      projectJson: projectJson,
      diagnostics: diagnostics,
      blocksBuild: blocksBuild,
    );
  }
}

class AuthoringCheckpointPreparation {
  const AuthoringCheckpointPreparation._({
    required this.head,
    required this.diagnostics,
    required this.blocksBuild,
  });

  final AuthoringWorkingHead head;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringCheckpointPreparation.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'ok',
      'head_json',
      'diagnostics',
      'blocks_build',
    }, 'checkpoint-preparation response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring checkpoint-preparation response is not ok',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
    return AuthoringCheckpointPreparation._(
      head: head,
      diagnostics: diagnostics,
      blocksBuild: blocksBuild,
    );
  }
}

class AuthoringAssetRef {
  const AuthoringAssetRef._({
    required this.sha256,
    required this.byteLength,
    required this.logicalName,
  });

  factory AuthoringAssetRef({
    required String sha256,
    required int byteLength,
    required String logicalName,
  }) => AuthoringAssetRef.fromJson({
    'sha256': sha256,
    'byte_len': byteLength,
    'logical_name': logicalName,
  });

  final String sha256;
  final int byteLength;
  final String logicalName;

  factory AuthoringAssetRef.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'sha256',
      'byte_len',
      'logical_name',
    }, 'asset reference');
    final sha256 = _authoringRequiredString(json, 'sha256', maxBytes: 64);
    if (!_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException('authoring asset SHA-256 is not canonical');
    }
    return AuthoringAssetRef._(
      sha256: sha256,
      byteLength: _authoringRequiredInt(
        json,
        'byte_len',
        min: 1,
        max: _maxAuthoringReferencedAssetBytes,
      ),
      logicalName: _authoringRequiredString(
        json,
        'logical_name',
        maxBytes: _maxAuthoringLogicalNameBytes,
      ),
    );
  }

  Map<String, Object?> toJson() => {
    'sha256': sha256,
    'byte_len': byteLength,
    'logical_name': logicalName,
  };
}

enum AuthoringOggCodec { vorbis, opus }

class AuthoringOggMetadata {
  const AuthoringOggMetadata._({
    required this.codec,
    required this.channels,
    required this.sampleRate,
    required this.pages,
    required this.logicalStreams,
  });

  final AuthoringOggCodec codec;
  final int channels;
  final int sampleRate;
  final int pages;
  final int logicalStreams;

  factory AuthoringOggMetadata.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'codec',
      'channels',
      'sample_rate',
      'pages',
      'logical_streams',
    }, 'Ogg metadata');
    final codec = switch (json['codec']) {
      'vorbis' => AuthoringOggCodec.vorbis,
      'opus' => AuthoringOggCodec.opus,
      _ => throw const FormatException('unknown authoring Ogg codec'),
    };
    return AuthoringOggMetadata._(
      codec: codec,
      channels: _authoringRequiredInt(json, 'channels', min: 1, max: 255),
      sampleRate: _authoringRequiredInt(
        json,
        'sample_rate',
        min: 1,
        max: 0xffffffff,
      ),
      pages: _authoringRequiredInt(json, 'pages', min: 1, max: 0xffffffff),
      logicalStreams: _authoringRequiredInt(
        json,
        'logical_streams',
        min: 1,
        max: 0xffffffff,
      ),
    );
  }
}

class AuthoringImportedOgg {
  const AuthoringImportedOgg._({
    required this.asset,
    required this.ogg,
    required this.deduplicated,
  });

  final AuthoringAssetRef asset;
  final AuthoringOggMetadata ogg;
  final bool deduplicated;

  factory AuthoringImportedOgg.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'ok',
      'asset',
      'ogg',
      'deduplicated',
    }, 'Ogg-import response');
    if (json['ok'] != true) {
      throw const FormatException('authoring Ogg-import response is not ok');
    }
    return AuthoringImportedOgg._(
      asset: AuthoringAssetRef.fromJson(
        _authoringRequiredObject(json['asset'], 'imported asset'),
      ),
      ogg: AuthoringOggMetadata.fromJson(
        _authoringRequiredObject(json['ogg'], 'imported Ogg metadata'),
      ),
      deduplicated: _authoringRequiredBool(json, 'deduplicated'),
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
