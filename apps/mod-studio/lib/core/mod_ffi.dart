import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;

import 'core_service.dart';

const _maxNativeErrorCodeLength = 128;
const _maxNativeErrorMessageLength = 64 * 1024;
const _maxAuthoringStorePathBytes = 32 * 1024;
const _maxAuthoringHeadJsonBytes = 64 * 1024;
const _maxAuthoringProjectJsonBytes = 16 * 1024 * 1024;
const _maxAuthoringNpcDraftInputBytes = 16 * 1024;
const _maxAuthoringQuestDraftInputBytes = 20 * 1024 * 1024;
const _maxAuthoringDraftSourceBytes = 1024 * 1024;
const _maxAuthoringDraftMetadataBytes = 4 * 1024;
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

  /// Validate and preview one bounded logical-NPC clone entirely in memory.
  Future<AuthoringLogicalNpcCloneDraftResult>
  authoringLogicalNpcCloneDraftV1Generate({required String inputJson}) async {
    _authoringDraftRequestString(
      inputJson,
      'inputJson',
      _maxAuthoringNpcDraftInputBytes,
    );
    const command = 'authoring_logical_npc_clone_draft_v1_generate';
    _authoringDraftEnvelopePreflight(command, inputJson);
    final response = await _call(command, {'input_json': inputJson});
    return AuthoringLogicalNpcCloneDraftResult.fromJson(response);
  }

  /// Validate and preview one bounded discovery-shaped quest entirely in memory.
  Future<AuthoringDraftQuestSkeletonResult>
  authoringDraftQuestSkeletonV1Generate({required String inputJson}) async {
    _authoringDraftRequestString(
      inputJson,
      'inputJson',
      _maxAuthoringQuestDraftInputBytes,
    );
    const command = 'authoring_draft_quest_skeleton_v1_generate';
    _authoringDraftEnvelopePreflight(command, inputJson);
    final response = await _call(command, {'input_json': inputJson});
    return AuthoringDraftQuestSkeletonResult.fromJson(response);
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

void _authoringDraftRequestString(String value, String field, int maxBytes) {
  if (value.isEmpty) {
    throw ArgumentError.value(value, field, 'must not be empty');
  }
  var bytes = 0;
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    if (unit <= 0x7f) {
      bytes += 1;
    } else if (unit <= 0x7ff) {
      bytes += 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw ArgumentError.value(
          value.length,
          field,
          'contains invalid UTF-16',
        );
      }
      final low = value.codeUnitAt(++index);
      if (low < 0xdc00 || low > 0xdfff) {
        throw ArgumentError.value(
          value.length,
          field,
          'contains invalid UTF-16',
        );
      }
      bytes += 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw ArgumentError.value(value.length, field, 'contains invalid UTF-16');
    } else {
      bytes += 3;
    }
    if (bytes > maxBytes) {
      throw ArgumentError.value(
        '<${value.length} characters>',
        field,
        'must be 1..=$maxBytes UTF-8 bytes',
      );
    }
  }
}

void _authoringDraftEnvelopePreflight(String command, String inputJson) {
  // Conservative allocation-free upper bound for jsonEncode's command envelope. Counting every
  // control scalar as `\u00XX` may reject a short-escape-heavy input early, but can never let an
  // oversized allocation reach the UI isolate.
  final envelopeBytes =
      '{"command":"","payload":{"input_json":""}}'.length + command.length;
  var encodedBytes = envelopeBytes;
  for (var index = 0; index < inputJson.length; index++) {
    final unit = inputJson.codeUnitAt(index);
    final int added;
    if (unit <= 0x1f || unit == 0x2028 || unit == 0x2029) {
      added = 6;
    } else if (unit == 0x22 || unit == 0x5c) {
      added = 2;
    } else if (unit <= 0x7f) {
      added = 1;
    } else if (unit <= 0x7ff) {
      added = 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      // The raw-input preflight already proved this is a paired surrogate.
      index++;
      added = 4;
    } else {
      added = 3;
    }
    if (added > goreCoreTransportMaxRequestBytes - encodedBytes) {
      throw ArgumentError.value(
        '<${inputJson.length} characters>',
        'inputJson',
        'escaped command envelope exceeds the '
            '$goreCoreTransportMaxRequestBytes-byte transport limit',
      );
    }
    encodedBytes += added;
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

Map<String, Object?> _authoringDraftObjectField(
  Map<String, Object?> json,
  String field,
) => _authoringRequiredObject(json[field], 'draft field $field');

String _authoringDraftSha256(Map<String, Object?> json, String field) {
  final value = _authoringRequiredString(json, field, maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(value)) {
    throw FormatException('authoring draft field $field is not a SHA-256');
  }
  return value;
}

String _authoringDraftVerifiedSourceSha256(
  Map<String, Object?> json,
  String source,
) {
  final declared = _authoringDraftSha256(json, 'source_sha256');
  final actual = crypto.sha256.convert(utf8.encode(source)).toString();
  if (declared != actual) {
    throw const FormatException(
      'authoring draft source and source_sha256 disagree',
    );
  }
  return declared;
}

String _authoringDraftFixedString(
  Map<String, Object?> json,
  String field,
  String expected,
) {
  final value = _authoringRequiredString(
    json,
    field,
    maxBytes: _maxAuthoringDraftMetadataBytes,
  );
  if (value != expected) {
    throw FormatException('authoring draft field $field is not supported');
  }
  return value;
}

final _authoringDraftIdentifierPattern = RegExp(r'^[A-Za-z_][A-Za-z0-9_]*$');
final _authoringDraftTechnicalIdPattern = RegExp(
  r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$',
);
final _authoringDraftCatalogLayerPattern = RegExp(
  r'^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$',
);
const _authoringDraftReservedIdentifiers = <String>{
  'abstract',
  'access',
  'and',
  'and_eq',
  'as',
  'auto',
  'bool',
  'break',
  'case',
  'cast',
  'catch',
  'class',
  'const',
  'continue',
  'default',
  'delegate',
  'do',
  'double',
  'else',
  'enum',
  'event',
  'explicit',
  'external',
  'false',
  'final',
  'float',
  'for',
  'from',
  'funcdef',
  'get',
  'if',
  'import',
  'in',
  'inout',
  'int',
  'int8',
  'int16',
  'int32',
  'int64',
  'interface',
  'is',
  'mixin',
  'namespace',
  'not',
  'not_eq',
  'null',
  'or',
  'or_eq',
  'out',
  'override',
  'private',
  'property',
  'protected',
  'return',
  'set',
  'shared',
  'super',
  'switch',
  'struct',
  'this',
  'true',
  'try',
  'typedef',
  'uint',
  'uint8',
  'uint16',
  'uint32',
  'uint64',
  'void',
  'while',
  'xor',
  'xor_eq',
  'staticclass',
  'spawn',
  'getorcreate',
  'create',
  'getg1r',
};

void _authoringDraftValidateIdentifier(
  String value,
  String field, {
  int maxBytes = 96,
}) {
  if (utf8.encode(value).length > maxBytes ||
      !_authoringDraftIdentifierPattern.hasMatch(value) ||
      value.startsWith('__') ||
      _authoringDraftReservedIdentifiers.contains(value.toLowerCase())) {
    throw FormatException('authoring draft field $field is not an identifier');
  }
}

bool _authoringDraftReservedPortableSegment(String value) {
  final upper = value.toUpperCase();
  if (const {'CON', 'PRN', 'AUX', 'NUL'}.contains(upper)) return true;
  if (upper.length == 4 &&
      (upper.startsWith('COM') || upper.startsWith('LPT'))) {
    final suffix = upper.codeUnitAt(3);
    return suffix >= 0x31 && suffix <= 0x39;
  }
  return false;
}

void _authoringDraftValidateModuleNamespace(String value) {
  if (utf8.encode(value).length > 255) {
    throw const FormatException('authoring draft module namespace is too long');
  }
  final segments = value.split('.');
  if (segments.isEmpty || segments.length > 16) {
    throw const FormatException('authoring draft module namespace is invalid');
  }
  for (final segment in segments) {
    _authoringDraftValidateIdentifier(segment, 'module_namespace');
    if (_authoringDraftReservedPortableSegment(segment)) {
      throw const FormatException(
        'authoring draft module namespace is not portable',
      );
    }
  }
}

void _authoringDraftValidateCatalogLayer(String value, String field) {
  if (utf8.encode(value).length > 128 ||
      !_authoringDraftCatalogLayerPattern.hasMatch(value)) {
    throw FormatException(
      'authoring draft field $field is not a canonical catalog layer',
    );
  }
}

String _authoringDraftPascalTechnicalId(String technicalId) => technicalId
    .split('_')
    .map(
      (segment) =>
          '${segment.substring(0, 1)}${segment.substring(1).toLowerCase()}',
    )
    .join();

List<AuthoringDraftDiagnostic> _authoringDraftDiagnostics(
  Map<String, Object?> json, {
  required bool valid,
}) {
  final raw = json['diagnostics'];
  if (raw is! List || raw.length > 1) {
    throw const FormatException(
      'authoring draft diagnostics must be a bounded list',
    );
  }
  final diagnostics = raw
      .map(
        (item) => AuthoringDraftDiagnostic.fromJson(
          _authoringRequiredObject(item, 'draft diagnostic'),
        ),
      )
      .toList(growable: false);
  if ((valid && diagnostics.isNotEmpty) ||
      (!valid && diagnostics.length != 1)) {
    throw const FormatException(
      'authoring draft validity and diagnostics disagree',
    );
  }
  return List.unmodifiable(diagnostics);
}

class AuthoringDraftDiagnostic {
  const AuthoringDraftDiagnostic._({
    required this.code,
    required this.field,
    required this.message,
  });

  static const _codes = <String>{
    'NPC_EMPTY_VALUE',
    'NPC_VALUE_TOO_LONG',
    'NPC_TOO_MANY_MODULE_SEGMENTS',
    'NPC_INVALID_IDENTIFIER_START',
    'NPC_INVALID_IDENTIFIER_CHARACTER',
    'NPC_RESERVED_IDENTIFIER',
    'NPC_RESERVED_MODULE_SEGMENT',
    'NPC_UNEXPECTED_PARENT_CLASS_PREFIX',
    'NPC_CLASS_NAME_COLLISION',
    'QUEST_INVALID_SEAL',
    'QUEST_GENERATION_MISMATCH',
    'QUEST_ZERO_ENTITY_ID',
    'QUEST_EMPTY_VALUE',
    'QUEST_VALUE_TOO_LONG',
    'QUEST_INVALID_CHARACTER',
    'QUEST_RESERVED_IDENTIFIER',
    'QUEST_NON_CANONICAL_IDENTIFIER',
    'QUEST_TOO_MANY_MODULE_SEGMENTS',
    'QUEST_RESERVED_MODULE_SEGMENT',
    'QUEST_INVALID_PARENT_CLASS',
    'QUEST_PARENT_CLASS_COLLISION',
    'QUEST_NON_CANONICAL_TEXT',
    'QUEST_TOO_MANY_COLLISION_ENTRIES',
    'QUEST_COLLISION_CATALOG_TOO_LARGE',
    'QUEST_UNSAFE_COLLISION_ENTRY',
    'QUEST_DUPLICATE_COLLISION_ENTRY',
    'QUEST_GENERATED_NAME_COLLISION',
    'QUEST_GENERATED_SYMBOL_COLLISION',
  };

  final String code;
  final String field;
  final String message;

  factory AuthoringDraftDiagnostic.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'code',
      'field',
      'message',
    }, 'draft diagnostic');
    final code = _authoringRequiredString(json, 'code', maxBytes: 128);
    final field = _authoringRequiredString(
      json,
      'field',
      maxBytes: _maxAuthoringDiagnosticPathBytes,
    );
    final message = _authoringRequiredString(
      json,
      'message',
      maxBytes: _maxAuthoringDiagnosticMessageBytes,
    );
    if (!_codes.contains(code) ||
        !_authoringDiagnosticCodePattern.hasMatch(code)) {
      throw const FormatException('unknown authoring draft diagnostic code');
    }
    return AuthoringDraftDiagnostic._(
      code: code,
      field: field,
      message: message,
    );
  }
}

class AuthoringLogicalNpcCloneDraftResult {
  const AuthoringLogicalNpcCloneDraftResult._({
    required this.valid,
    required this.generated,
    required this.diagnostics,
  });

  final bool valid;
  final AuthoringLogicalNpcCloneGenerated? generated;
  final List<AuthoringDraftDiagnostic> diagnostics;

  factory AuthoringLogicalNpcCloneDraftResult.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'valid',
      'generated',
      'diagnostics',
    }, 'NPC draft response');
    if (json['ok'] != true) {
      throw const FormatException('authoring NPC draft response is not ok');
    }
    final valid = _authoringRequiredBool(json, 'valid');
    final generatedValue = json['generated'];
    final generated = generatedValue == null
        ? null
        : AuthoringLogicalNpcCloneGenerated.fromJson(
            _authoringRequiredObject(generatedValue, 'NPC generated preview'),
          );
    if (valid != (generated != null)) {
      throw const FormatException(
        'authoring NPC draft validity and generated preview disagree',
      );
    }
    return AuthoringLogicalNpcCloneDraftResult._(
      valid: valid,
      generated: generated,
      diagnostics: _authoringDraftDiagnostics(json, valid: valid),
    );
  }
}

class AuthoringLogicalNpcCloneGenerated {
  const AuthoringLogicalNpcCloneGenerated._({
    required this.generatorId,
    required this.generatorVersion,
    required this.moduleNamespace,
    required this.moduleRelativePath,
    required this.uniqueName,
    required this.classes,
    required this.source,
    required this.sourceSha256,
    required this.inputFingerprint,
    required this.status,
  });

  final String generatorId;
  final int generatorVersion;
  final String moduleNamespace;
  final String moduleRelativePath;
  final String uniqueName;
  final AuthoringLogicalNpcCloneClasses classes;
  final String source;
  final String sourceSha256;
  final String inputFingerprint;
  final AuthoringLogicalNpcCloneStatus status;

  factory AuthoringLogicalNpcCloneGenerated.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'generator_id',
      'generator_version',
      'module_namespace',
      'module_relative_path',
      'unique_name',
      'classes',
      'source',
      'source_sha256',
      'input_fingerprint',
      'status',
    }, 'NPC generated preview');
    final generatorId = _authoringDraftFixedString(
      json,
      'generator_id',
      'gore-authoring.logical-npc-clone-draft',
    );
    final generatorVersion = _authoringRequiredInt(
      json,
      'generator_version',
      min: 1,
      max: 1,
    );
    final moduleNamespace = _authoringRequiredString(
      json,
      'module_namespace',
      maxBytes: 255,
    );
    _authoringDraftValidateModuleNamespace(moduleNamespace);
    final moduleRelativePath = _authoringRequiredString(
      json,
      'module_relative_path',
      maxBytes: 258,
    );
    if (moduleRelativePath != '${moduleNamespace.replaceAll('.', '/')}.as') {
      throw const FormatException('NPC generated module path is inconsistent');
    }
    final uniqueName = _authoringRequiredString(
      json,
      'unique_name',
      maxBytes: 64,
    );
    _authoringDraftValidateIdentifier(uniqueName, 'unique_name', maxBytes: 64);
    final classes = AuthoringLogicalNpcCloneClasses.fromJson(
      _authoringDraftObjectField(json, 'classes'),
      uniqueName: uniqueName,
    );
    final status = AuthoringLogicalNpcCloneStatus.fromJson(
      _authoringDraftObjectField(json, 'status'),
    );
    final source = _authoringRequiredString(
      json,
      'source',
      maxBytes: _maxAuthoringDraftSourceBytes,
    );
    return AuthoringLogicalNpcCloneGenerated._(
      generatorId: generatorId,
      generatorVersion: generatorVersion,
      moduleNamespace: moduleNamespace,
      moduleRelativePath: moduleRelativePath,
      uniqueName: uniqueName,
      classes: classes,
      source: source,
      sourceSha256: _authoringDraftVerifiedSourceSha256(json, source),
      inputFingerprint: _authoringDraftSha256(json, 'input_fingerprint'),
      status: status,
    );
  }
}

class AuthoringLogicalNpcCloneClasses {
  const AuthoringLogicalNpcCloneClasses._({
    required this.characterDefinition,
    required this.aiAgentConfig,
    required this.spawnDefinition,
  });

  final String characterDefinition;
  final String aiAgentConfig;
  final String spawnDefinition;

  factory AuthoringLogicalNpcCloneClasses.fromJson(
    Map<String, Object?> json, {
    required String uniqueName,
  }) {
    _authoringExactFields(json, const {
      'character_definition',
      'ai_agent_config',
      'spawn_definition',
    }, 'NPC classes');
    final characterDefinition = _authoringRequiredString(
      json,
      'character_definition',
      maxBytes: 160,
    );
    final aiAgentConfig = _authoringRequiredString(
      json,
      'ai_agent_config',
      maxBytes: 160,
    );
    final spawnDefinition = _authoringRequiredString(
      json,
      'spawn_definition',
      maxBytes: 160,
    );
    if (characterDefinition != 'UCharacterDefinition_Human_$uniqueName' ||
        aiAgentConfig != 'UAIAgentConfig_Human_$uniqueName' ||
        spawnDefinition != 'USpawnAIAgentDefinition_$uniqueName') {
      throw const FormatException('NPC generated classes are inconsistent');
    }
    return AuthoringLogicalNpcCloneClasses._(
      characterDefinition: characterDefinition,
      aiAgentConfig: aiAgentConfig,
      spawnDefinition: spawnDefinition,
    );
  }
}

enum AuthoringDraftAuthoringStatus { offlineDraft }

enum AuthoringDraftRuntimeStatus { runtimeUnqualified }

enum AuthoringDraftQuestTransitionStatus { transitionsRuntimeUnqualified }

class AuthoringLogicalNpcCloneStatus {
  const AuthoringLogicalNpcCloneStatus._({
    required this.authoring,
    required this.runtime,
  });

  final AuthoringDraftAuthoringStatus authoring;
  final AuthoringDraftRuntimeStatus runtime;

  factory AuthoringLogicalNpcCloneStatus.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {'authoring', 'runtime'}, 'NPC status');
    _authoringDraftFixedString(json, 'authoring', 'offline_draft');
    _authoringDraftFixedString(json, 'runtime', 'runtime_unqualified');
    return const AuthoringLogicalNpcCloneStatus._(
      authoring: AuthoringDraftAuthoringStatus.offlineDraft,
      runtime: AuthoringDraftRuntimeStatus.runtimeUnqualified,
    );
  }
}

class AuthoringDraftQuestSkeletonResult {
  const AuthoringDraftQuestSkeletonResult._({
    required this.valid,
    required this.generated,
    required this.diagnostics,
  });

  final bool valid;
  final AuthoringDraftQuestGenerated? generated;
  final List<AuthoringDraftDiagnostic> diagnostics;

  factory AuthoringDraftQuestSkeletonResult.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'valid',
      'generated',
      'diagnostics',
    }, 'quest draft response');
    if (json['ok'] != true) {
      throw const FormatException('authoring quest draft response is not ok');
    }
    final valid = _authoringRequiredBool(json, 'valid');
    final generatedValue = json['generated'];
    final generated = generatedValue == null
        ? null
        : AuthoringDraftQuestGenerated.fromJson(
            _authoringRequiredObject(generatedValue, 'quest generated preview'),
          );
    if (valid != (generated != null)) {
      throw const FormatException(
        'authoring quest draft validity and generated preview disagree',
      );
    }
    return AuthoringDraftQuestSkeletonResult._(
      valid: valid,
      generated: generated,
      diagnostics: _authoringDraftDiagnostics(json, valid: valid),
    );
  }
}

class AuthoringDraftQuestGenerated {
  const AuthoringDraftQuestGenerated._({
    required this.target,
    required this.questId,
    required this.generatorId,
    required this.generatorVersion,
    required this.giver,
    required this.parentQuest,
    required this.collisionCatalog,
    required this.technicalNames,
    required this.fixedShape,
    required this.source,
    required this.sourceSha256,
    required this.inputFingerprint,
    required this.status,
  });

  final AuthoringDraftGameGeneration target;
  final String questId;
  final String generatorId;
  final int generatorVersion;
  final AuthoringDraftQuestGiver giver;
  final AuthoringDraftParentQuest parentQuest;
  final AuthoringDraftCatalogAnchor collisionCatalog;
  final AuthoringDraftQuestTechnicalNames technicalNames;
  final AuthoringDraftQuestFixedShape fixedShape;
  final String source;
  final String sourceSha256;
  final String inputFingerprint;
  final AuthoringDraftQuestStatus status;

  factory AuthoringDraftQuestGenerated.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'target',
      'quest_id',
      'generator_id',
      'generator_version',
      'giver',
      'parent_quest',
      'collision_catalog',
      'technical_names',
      'fixed_shape',
      'source',
      'source_sha256',
      'input_fingerprint',
      'status',
    }, 'quest generated preview');
    final questId = _authoringRequiredString(json, 'quest_id', maxBytes: 32);
    _authoringEntityId(questId, 'quest_id');
    if (questId == '00000000000000000000000000000000') {
      throw const FormatException('authoring quest ID must not be zero');
    }
    final status = AuthoringDraftQuestStatus.fromJson(
      _authoringDraftObjectField(json, 'status'),
    );
    final target = AuthoringDraftGameGeneration.fromJson(
      _authoringDraftObjectField(json, 'target'),
    );
    final giver = AuthoringDraftQuestGiver.fromJson(
      _authoringDraftObjectField(json, 'giver'),
    );
    final parentQuest = AuthoringDraftParentQuest.fromJson(
      _authoringDraftObjectField(json, 'parent_quest'),
    );
    final collisionCatalog = AuthoringDraftCatalogAnchor.fromJson(
      _authoringDraftObjectField(json, 'collision_catalog'),
    );
    if (!target.sameGeneration(giver.generation) ||
        !target.sameGeneration(parentQuest.generation) ||
        !target.sameGeneration(collisionCatalog.generation)) {
      throw const FormatException(
        'quest generated provenance has inconsistent game generations',
      );
    }
    final technicalNames = AuthoringDraftQuestTechnicalNames.fromJson(
      _authoringDraftObjectField(json, 'technical_names'),
    );
    if (parentQuest.runtimeClass.toLowerCase() ==
            technicalNames.rootClass.toLowerCase() ||
        parentQuest.runtimeClass.toLowerCase() ==
            technicalNames.objectiveClass.toLowerCase()) {
      throw const FormatException(
        'quest generated class collides with its parent class',
      );
    }
    final source = _authoringRequiredString(
      json,
      'source',
      maxBytes: _maxAuthoringDraftSourceBytes,
    );
    return AuthoringDraftQuestGenerated._(
      target: target,
      questId: questId,
      generatorId: _authoringDraftFixedString(
        json,
        'generator_id',
        'gore-authoring.draft-quest-skeleton',
      ),
      generatorVersion: _authoringRequiredInt(
        json,
        'generator_version',
        min: 1,
        max: 1,
      ),
      giver: giver,
      parentQuest: parentQuest,
      collisionCatalog: collisionCatalog,
      technicalNames: technicalNames,
      fixedShape: AuthoringDraftQuestFixedShape.fromJson(
        _authoringDraftObjectField(json, 'fixed_shape'),
      ),
      source: source,
      sourceSha256: _authoringDraftVerifiedSourceSha256(json, source),
      inputFingerprint: _authoringDraftSha256(json, 'input_fingerprint'),
      status: status,
    );
  }
}

class AuthoringDraftQuestStatus {
  const AuthoringDraftQuestStatus._({
    required this.authoring,
    required this.discovery,
    required this.transitions,
  });

  final AuthoringDraftAuthoringStatus authoring;
  final AuthoringDraftRuntimeStatus discovery;
  final AuthoringDraftQuestTransitionStatus transitions;

  factory AuthoringDraftQuestStatus.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'authoring',
      'discovery',
      'transitions',
    }, 'quest status');
    _authoringDraftFixedString(json, 'authoring', 'offline_draft');
    _authoringDraftFixedString(json, 'discovery', 'runtime_unqualified');
    _authoringDraftFixedString(
      json,
      'transitions',
      'transitions_runtime_unqualified',
    );
    return const AuthoringDraftQuestStatus._(
      authoring: AuthoringDraftAuthoringStatus.offlineDraft,
      discovery: AuthoringDraftRuntimeStatus.runtimeUnqualified,
      transitions:
          AuthoringDraftQuestTransitionStatus.transitionsRuntimeUnqualified,
    );
  }
}

class AuthoringDraftContentSeal {
  const AuthoringDraftContentSeal._({
    required this.byteLength,
    required this.sha256,
  });

  final int byteLength;
  final String sha256;

  factory AuthoringDraftContentSeal.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'byte_len',
      'sha256',
    }, 'draft content seal');
    return AuthoringDraftContentSeal._(
      byteLength: _authoringRequiredInt(json, 'byte_len', min: 1),
      sha256: _authoringDraftSha256(json, 'sha256'),
    );
  }
}

class AuthoringDraftGameGeneration {
  const AuthoringDraftGameGeneration._({required this.executable});
  final AuthoringDraftContentSeal executable;

  bool sameGeneration(AuthoringDraftGameGeneration other) =>
      executable.byteLength == other.executable.byteLength &&
      executable.sha256 == other.executable.sha256;

  factory AuthoringDraftGameGeneration.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {'executable'}, 'draft game generation');
    return AuthoringDraftGameGeneration._(
      executable: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'executable'),
      ),
    );
  }
}

class AuthoringDraftQuestGiver {
  const AuthoringDraftQuestGiver._({
    required this.generation,
    required this.sourceSeal,
    required this.catalogLayer,
    required this.canonicalSelector,
    required this.runtimeUniqueName,
  });
  final AuthoringDraftGameGeneration generation;
  final AuthoringDraftContentSeal sourceSeal;
  final String catalogLayer;
  final String canonicalSelector;
  final String runtimeUniqueName;

  factory AuthoringDraftQuestGiver.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'generation',
      'source_seal',
      'catalog_layer',
      'canonical_selector',
      'runtime_unique_name',
    }, 'quest giver');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    final canonicalSelector = _authoringRequiredString(
      json,
      'canonical_selector',
      maxBytes: 96,
    );
    final runtimeUniqueName = _authoringRequiredString(
      json,
      'runtime_unique_name',
      maxBytes: 96,
    );
    _authoringDraftValidateCatalogLayer(catalogLayer, 'giver.catalog_layer');
    _authoringDraftValidateIdentifier(
      canonicalSelector,
      'giver.canonical_selector',
    );
    _authoringDraftValidateIdentifier(
      runtimeUniqueName,
      'giver.runtime_unique_name',
    );
    return AuthoringDraftQuestGiver._(
      generation: AuthoringDraftGameGeneration.fromJson(
        _authoringDraftObjectField(json, 'generation'),
      ),
      sourceSeal: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'source_seal'),
      ),
      catalogLayer: catalogLayer,
      canonicalSelector: canonicalSelector,
      runtimeUniqueName: runtimeUniqueName,
    );
  }
}

class AuthoringDraftParentQuest {
  const AuthoringDraftParentQuest._({
    required this.generation,
    required this.sourceSeal,
    required this.catalogLayer,
    required this.canonicalSelector,
    required this.runtimeClass,
  });
  final AuthoringDraftGameGeneration generation;
  final AuthoringDraftContentSeal sourceSeal;
  final String catalogLayer;
  final String canonicalSelector;
  final String runtimeClass;

  factory AuthoringDraftParentQuest.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'generation',
      'source_seal',
      'catalog_layer',
      'canonical_selector',
      'runtime_class',
    }, 'parent quest');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    final canonicalSelector = _authoringRequiredString(
      json,
      'canonical_selector',
      maxBytes: 96,
    );
    final runtimeClass = _authoringRequiredString(
      json,
      'runtime_class',
      maxBytes: 96,
    );
    _authoringDraftValidateCatalogLayer(
      catalogLayer,
      'parent_quest.catalog_layer',
    );
    _authoringDraftValidateIdentifier(
      canonicalSelector,
      'parent_quest.canonical_selector',
    );
    _authoringDraftValidateIdentifier(
      runtimeClass,
      'parent_quest.runtime_class',
    );
    if (!runtimeClass.startsWith('UQuest_')) {
      throw const FormatException(
        'authoring draft parent runtime class has an invalid prefix',
      );
    }
    return AuthoringDraftParentQuest._(
      generation: AuthoringDraftGameGeneration.fromJson(
        _authoringDraftObjectField(json, 'generation'),
      ),
      sourceSeal: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'source_seal'),
      ),
      catalogLayer: catalogLayer,
      canonicalSelector: canonicalSelector,
      runtimeClass: runtimeClass,
    );
  }
}

class AuthoringDraftCatalogAnchor {
  const AuthoringDraftCatalogAnchor._({
    required this.generation,
    required this.sourceSeal,
    required this.catalogLayer,
  });
  final AuthoringDraftGameGeneration generation;
  final AuthoringDraftContentSeal sourceSeal;
  final String catalogLayer;

  factory AuthoringDraftCatalogAnchor.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'generation',
      'source_seal',
      'catalog_layer',
    }, 'collision catalog anchor');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    _authoringDraftValidateCatalogLayer(
      catalogLayer,
      'collision_catalog.catalog_layer',
    );
    return AuthoringDraftCatalogAnchor._(
      generation: AuthoringDraftGameGeneration.fromJson(
        _authoringDraftObjectField(json, 'generation'),
      ),
      sourceSeal: AuthoringDraftContentSeal.fromJson(
        _authoringDraftObjectField(json, 'source_seal'),
      ),
      catalogLayer: catalogLayer,
    );
  }
}

class AuthoringDraftQuestTechnicalNames {
  const AuthoringDraftQuestTechnicalNames._({
    required this.moduleNamespace,
    required this.moduleRelativePath,
    required this.rootClass,
    required this.objectiveClass,
    required this.textHelper,
    required this.rootGetter,
    required this.objectiveGetter,
  });
  final String moduleNamespace;
  final String moduleRelativePath;
  final String rootClass;
  final String objectiveClass;
  final String textHelper;
  final String rootGetter;
  final String objectiveGetter;

  factory AuthoringDraftQuestTechnicalNames.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'module_namespace',
      'module_relative_path',
      'root_class',
      'objective_class',
      'text_helper',
      'root_getter',
      'objective_getter',
    }, 'quest technical names');
    final moduleNamespace = _authoringRequiredString(
      json,
      'module_namespace',
      maxBytes: 255,
    );
    _authoringDraftValidateModuleNamespace(moduleNamespace);
    final moduleRelativePath = _authoringRequiredString(
      json,
      'module_relative_path',
      maxBytes: 258,
    );
    if (moduleRelativePath != '${moduleNamespace.replaceAll('.', '/')}.as') {
      throw const FormatException(
        'quest generated module path is inconsistent',
      );
    }
    final rootClass = _authoringRequiredString(
      json,
      'root_class',
      maxBytes: 96,
    );
    final objectiveClass = _authoringRequiredString(
      json,
      'objective_class',
      maxBytes: 96,
    );
    final textHelper = _authoringRequiredString(
      json,
      'text_helper',
      maxBytes: 96,
    );
    final rootGetter = _authoringRequiredString(
      json,
      'root_getter',
      maxBytes: 96,
    );
    final objectiveGetter = _authoringRequiredString(
      json,
      'objective_getter',
      maxBytes: 96,
    );
    for (final entry in <String, String>{
      'root_class': rootClass,
      'objective_class': objectiveClass,
      'text_helper': textHelper,
      'root_getter': rootGetter,
      'objective_getter': objectiveGetter,
    }.entries) {
      _authoringDraftValidateIdentifier(entry.value, entry.key);
    }
    if (!rootClass.startsWith('UQuest_')) {
      throw const FormatException('quest root class has an invalid prefix');
    }
    final technicalId = rootClass.substring('UQuest_'.length);
    if (!_authoringDraftTechnicalIdPattern.hasMatch(technicalId)) {
      throw const FormatException('quest technical ID is not canonical');
    }
    final pascal = _authoringDraftPascalTechnicalId(technicalId);
    if (objectiveClass != '${rootClass}_OBJ_DONE' ||
        rootGetter != 'Get$pascal' ||
        objectiveGetter != 'Get${pascal}Objective') {
      throw const FormatException('quest generated technical names disagree');
    }
    final foldedSymbols = <String>{
      rootClass.toLowerCase(),
      objectiveClass.toLowerCase(),
      textHelper.toLowerCase(),
      rootGetter.toLowerCase(),
      objectiveGetter.toLowerCase(),
    };
    if (foldedSymbols.length != 5) {
      throw const FormatException('quest generated symbols collide');
    }
    return AuthoringDraftQuestTechnicalNames._(
      moduleNamespace: moduleNamespace,
      moduleRelativePath: moduleRelativePath,
      rootClass: rootClass,
      objectiveClass: objectiveClass,
      textHelper: textHelper,
      rootGetter: rootGetter,
      objectiveGetter: objectiveGetter,
    );
  }
}

class AuthoringDraftQuestFixedShape {
  const AuthoringDraftQuestFixedShape._({
    required this.questBaseClass,
    required this.rootKind,
    required this.objectiveKind,
    required this.rootExternalStart,
    required this.objectiveExternalStart,
    required this.objectiveExternalSuccess,
    required this.objectiveSucceedsParent,
  });
  final String questBaseClass;
  final String rootKind;
  final String objectiveKind;
  final bool rootExternalStart;
  final bool objectiveExternalStart;
  final bool objectiveExternalSuccess;
  final bool objectiveSucceedsParent;

  factory AuthoringDraftQuestFixedShape.fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'quest_base_class',
      'root_kind',
      'objective_kind',
      'root_external_start',
      'objective_external_start',
      'objective_external_success',
      'objective_succeeds_parent',
    }, 'quest fixed shape');
    final questBaseClass = _authoringDraftFixedString(
      json,
      'quest_base_class',
      'UG1RQuest',
    );
    final rootKind = _authoringDraftFixedString(
      json,
      'root_kind',
      'EQuestKind::Side',
    );
    final objectiveKind = _authoringDraftFixedString(
      json,
      'objective_kind',
      'EQuestKind::Subobjective',
    );
    final rootExternalStart = _authoringRequiredBool(
      json,
      'root_external_start',
    );
    final objectiveExternalStart = _authoringRequiredBool(
      json,
      'objective_external_start',
    );
    final objectiveExternalSuccess = _authoringRequiredBool(
      json,
      'objective_external_success',
    );
    final objectiveSucceedsParent = _authoringRequiredBool(
      json,
      'objective_succeeds_parent',
    );
    if (!rootExternalStart ||
        !objectiveExternalStart ||
        !objectiveExternalSuccess ||
        !objectiveSucceedsParent) {
      throw const FormatException('quest fixed shape is not supported');
    }
    return AuthoringDraftQuestFixedShape._(
      questBaseClass: questBaseClass,
      rootKind: rootKind,
      objectiveKind: objectiveKind,
      rootExternalStart: rootExternalStart,
      objectiveExternalStart: objectiveExternalStart,
      objectiveExternalSuccess: objectiveExternalSuccess,
      objectiveSucceedsParent: objectiveSucceedsParent,
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
