import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;

import 'core_service.dart';

const _maxNativeErrorCodeLength = 128;
const _maxNativeErrorMessageLength = 64 * 1024;
const _maxAuthoringStorePathBytes = 32 * 1024;
const _maxAuthoringHeadJsonBytes = 64 * 1024;
const _maxAuthoringProjectJsonBytes = 16 * 1024 * 1024;
const _maxAuthoringStoryMutationJsonBytes = 20 * 1024 * 1024;
const _maxAuthoringStoryCatalogJsonBytes = 16 * 1024 * 1024;
const _maxAuthoringStoryCatalogNpcs = 2;
const _maxAuthoringStoryCatalogQuestParents = 1;
// This one FFI command deliberately stays within signed 64-bit JSON integers. A base at this
// maximum can advance once to `_maxAuthoringStoryAppliedRevision` without becoming a double.
const _maxAuthoringStoryBaseRevision = 0x7ffffffffffffffe;
const _maxAuthoringStoryAppliedRevision = 0x7fffffffffffffff;
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

  /// Atomically evaluate one Story Draft insert against exact canonical revision-2 project bytes.
  Future<AuthoringStoryDraftInsertResult> authoringProjectStoryDraftInsertV1({
    required String projectJson,
    required String mutationJson,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringDraftRequestString(
      projectJson,
      'projectJson',
      _maxAuthoringProjectJsonBytes,
    );
    _authoringDraftRequestString(
      mutationJson,
      'mutationJson',
      _maxAuthoringStoryMutationJsonBytes,
    );
    const command = 'authoring_project_story_draft_insert_v1';
    _authoringStoryMutationEnvelopePreflight(
      command,
      projectJson,
      mutationJson,
      profile.wireName,
    );
    final response = await _call(command, {
      'project_json': projectJson,
      'mutation_json': mutationJson,
      'profile': profile.wireName,
    });
    return AuthoringStoryDraftInsertResult.fromJson(
      response,
      projectJson: projectJson,
      mutationJson: mutationJson,
      profile: profile,
    );
  }

  /// Verify and project one exact pinned `story_catalog.v1` document without filesystem access.
  Future<AuthoringStoryCatalogSelections> authoringStoryCatalogV1Read({
    required String catalogJson,
  }) async {
    _authoringDraftRequestString(
      catalogJson,
      'catalogJson',
      _maxAuthoringStoryCatalogJsonBytes,
    );
    const command = 'authoring_story_catalog_v1_read';
    _authoringSingleRawJsonEnvelopePreflight(
      command,
      'catalog_json',
      'catalogJson',
      catalogJson,
    );
    final response = await _call(command, {'catalog_json': catalogJson});
    return AuthoringStoryCatalogSelections._fromJson(
      response,
      catalogJson: catalogJson,
    );
  }

  /// Build the pinned Story catalog from one installed generation entirely in memory.
  Future<AuthoringStoryCatalogBuildResult> authoringStoryCatalogV1Build({
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) async {
    _authoringStoryCatalogPath(executable, 'executable');
    _authoringStoryCatalogPath(shippingCache, 'shippingCache');
    _authoringStoryCatalogPath(bindsCache, 'bindsCache');
    const command = 'authoring_story_catalog_v1_build';
    _authoringStoryCatalogBuildEnvelopePreflight(
      command,
      executable,
      shippingCache,
      bindsCache,
    );
    final response = await _call(command, {
      'executable': executable,
      'shipping_cache': shippingCache,
      'binds_cache': bindsCache,
    });
    return AuthoringStoryCatalogBuildResult._fromJson(
      response,
      executable: executable,
      shippingCache: shippingCache,
      bindsCache: bindsCache,
    );
  }

  /// Build and then pass the exact raw catalog through the existing pinned chooser reader.
  Future<AuthoringStoryCatalogSelections> authoringStoryCatalogV1BuildAndRead({
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) async {
    final built = await authoringStoryCatalogV1Build(
      executable: executable,
      shippingCache: shippingCache,
      bindsCache: bindsCache,
    );
    return authoringStoryCatalogV1Read(catalogJson: built.catalogJson);
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

  /// Open the fixed canonical head and dispatch one closed schema-revision-1/2 document.
  Future<AuthoringStoreOpenedResult> authoringStoreOpenDocument({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open_document', {
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

  /// Prepare immutable objects for a closed schema-revision-1/2 document without publishing it.
  ///
  /// The raw document string and exact optional head CAS token are preserved byte-for-byte.
  Future<AuthoringCheckpointPreparation>
  authoringStorePrepareDocumentCheckpoint({
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
    final response =
        await _call('authoring_store_prepare_document_checkpoint', {
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

  /// Reopen one revision-dispatched candidate from its exact canonical head bytes.
  Future<AuthoringStoreOpenedResult> authoringStoreOpenHeadBytesDocument({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    _authoringStoreRequestString(root, 'root', _maxAuthoringStorePathBytes);
    final response = await _call('authoring_store_open_head_bytes_document', {
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

void _authoringStoryMutationEnvelopePreflight(
  String command,
  String projectJson,
  String mutationJson,
  String profile,
) {
  var encodedBytes =
      '{"command":"","payload":{"project_json":"","mutation_json":"","profile":""}}'
          .length +
      command.length +
      profile.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    projectJson,
    'projectJson',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(
    mutationJson,
    'mutationJson',
    encodedBytes,
  );
}

void _authoringSingleRawJsonEnvelopePreflight(
  String command,
  String wireField,
  String dartField,
  String value,
) {
  final encodedBytes =
      '{"command":"","payload":{"":""}}'.length +
      command.length +
      wireField.length;
  _authoringAddEscapedJsonStringBytes(value, dartField, encodedBytes);
}

void _authoringStoryCatalogPath(String value, String field) {
  _authoringDraftRequestString(value, field, _maxAuthoringStorePathBytes);
  if (value.contains('\u0000')) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'must not contain NUL',
    );
  }
}

void _authoringStoryCatalogBuildEnvelopePreflight(
  String command,
  String executable,
  String shippingCache,
  String bindsCache,
) {
  var encodedBytes =
      '{"command":"","payload":{"executable":"","shipping_cache":"","binds_cache":""}}'
          .length +
      command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    executable,
    'executable',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    shippingCache,
    'shippingCache',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(bindsCache, 'bindsCache', encodedBytes);
}

int _authoringAddEscapedJsonStringBytes(
  String value,
  String field,
  int encodedBytes,
) {
  if (encodedBytes > goreCoreTransportMaxRequestBytes) {
    throw ArgumentError.value(
      '<${value.length} characters>',
      field,
      'escaped command envelope exceeds the '
          '$goreCoreTransportMaxRequestBytes-byte transport limit',
    );
  }
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
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
        '<${value.length} characters>',
        field,
        'escaped command envelope exceeds the '
            '$goreCoreTransportMaxRequestBytes-byte transport limit',
      );
    }
    encodedBytes += added;
  }
  return encodedBytes;
}

const _authoringStoryRequestBindingDomain =
    'gore-authoring.story-draft-insert-v1.request-binding\u0000';

String _authoringStoryRequestBindingSha256(
  String projectJson,
  String mutationJson,
  AuthoringValidationProfile profile,
) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryRequestBindingDomain));
  for (final bytes in <List<int>>[
    utf8.encode(projectJson),
    utf8.encode(mutationJson),
    utf8.encode(profile.wireName),
  ]) {
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
    input
      ..add(length)
      ..add(bytes);
  }
  input.close();
  return output.value.toString();
}

final class _AuthoringDigestCollector implements Sink<crypto.Digest> {
  crypto.Digest? _value;

  crypto.Digest get value =>
      _value ??
      (throw const FormatException('authoring SHA-256 digest was not emitted'));

  @override
  void add(crypto.Digest data) {
    if (_value != null) {
      throw const FormatException('authoring SHA-256 emitted more than once');
    }
    _value = data;
  }

  @override
  void close() {}
}

Map<String, Object?> _authoringDecodeDuplicateSafeObject(
  String raw,
  String context,
) {
  try {
    _AuthoringDuplicateKeyJsonScanner(raw).validate();
    return _authoringRequiredObject(jsonDecode(raw), context);
  } on FormatException {
    throw FormatException(
      'authoring $context JSON is invalid or has duplicate keys',
    );
  }
}

final class _AuthoringDuplicateKeyJsonScanner {
  _AuthoringDuplicateKeyJsonScanner(this.source);

  static const _maxDepth = 128;
  final String source;
  int _index = 0;

  void validate() {
    _value(0);
    _whitespace();
    if (_index != source.length) throw const FormatException('trailing JSON');
  }

  void _value(int depth) {
    if (depth > _maxDepth) throw const FormatException('JSON too deep');
    _whitespace();
    if (_index >= source.length) throw const FormatException('missing JSON');
    switch (source.codeUnitAt(_index)) {
      case 0x7b:
        _object(depth + 1);
      case 0x5b:
        _array(depth + 1);
      case 0x22:
        _string();
      default:
        final start = _index;
        while (_index < source.length &&
            !_isValueDelimiter(source.codeUnitAt(_index))) {
          _index++;
        }
        if (_index == start) throw const FormatException('invalid JSON value');
    }
  }

  void _object(int depth) {
    _index++;
    _whitespace();
    if (_take(0x7d)) return;
    final keys = <String>{};
    while (true) {
      _whitespace();
      if (_index >= source.length || source.codeUnitAt(_index) != 0x22) {
        throw const FormatException('object key required');
      }
      final rawKey = _string();
      final key = jsonDecode(rawKey);
      if (key is! String || !keys.add(key)) {
        throw const FormatException('duplicate object key');
      }
      _whitespace();
      if (!_take(0x3a)) throw const FormatException('colon required');
      _value(depth);
      _whitespace();
      if (_take(0x7d)) return;
      if (!_take(0x2c)) throw const FormatException('comma required');
    }
  }

  void _array(int depth) {
    _index++;
    _whitespace();
    if (_take(0x5d)) return;
    while (true) {
      _value(depth);
      _whitespace();
      if (_take(0x5d)) return;
      if (!_take(0x2c)) throw const FormatException('comma required');
    }
  }

  String _string() {
    final start = _index++;
    while (_index < source.length) {
      final unit = source.codeUnitAt(_index++);
      if (unit == 0x22) return source.substring(start, _index);
      if (unit == 0x5c) {
        if (_index >= source.length) {
          throw const FormatException('unterminated escape');
        }
        _index++;
      } else if (unit <= 0x1f) {
        throw const FormatException('control character in string');
      }
    }
    throw const FormatException('unterminated string');
  }

  void _whitespace() {
    while (_index < source.length) {
      final unit = source.codeUnitAt(_index);
      if (unit != 0x20 && unit != 0x09 && unit != 0x0a && unit != 0x0d) return;
      _index++;
    }
  }

  bool _take(int unit) {
    if (_index < source.length && source.codeUnitAt(_index) == unit) {
      _index++;
      return true;
    }
    return false;
  }

  static bool _isValueDelimiter(int unit) =>
      unit == 0x20 ||
      unit == 0x09 ||
      unit == 0x0a ||
      unit == 0x0d ||
      unit == 0x2c ||
      unit == 0x5d ||
      unit == 0x7d;
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

({
  Map<String, Object?> project,
  String projectId,
  int schemaRevision,
  int revision,
})
_authoringRequireCanonicalProjectJson(String projectJson) {
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
  if (project['format'] != 2) {
    throw const FormatException(
      'authoring store project JSON has an unsupported format',
    );
  }
  final schemaRevision = project['schema_revision'];
  if (schemaRevision is! int || (schemaRevision != 1 && schemaRevision != 2)) {
    throw const FormatException(
      'authoring store project JSON has an unsupported schema revision',
    );
  }
  final projectId = _authoringEntityId(
    _authoringRequiredString(project, 'project_id', maxBytes: 32),
    'project_id',
  );
  if (projectId == '00000000000000000000000000000000') {
    throw const FormatException('authoring store project ID must not be zero');
  }
  final revision = _authoringRequiredInt(project, 'revision');
  if (jsonEncode(decoded) != projectJson) {
    throw const FormatException(
      'authoring store project JSON is not canonical',
    );
  }
  return (
    project: project,
    projectId: projectId,
    schemaRevision: schemaRevision,
    revision: revision,
  );
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

final class AuthoringDraftContentSeal {
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

enum AuthoringStoryCatalogNpcDiscoveryStatus { sealedCacheDefaultsVerified }

enum AuthoringStoryCatalogNpcAuthoringQualification { offlineQualified }

enum AuthoringStoryCatalogRuntimeQualification { runtimeUnqualified }

enum AuthoringStoryCatalogQuestParentRole { chapter }

enum AuthoringStoryCatalogQuestParentQualification { curatedDefaultsVerified }

enum AuthoringStoryCatalogCollisionStatus { inventoryUnavailable }

final _authoringStoryCatalogIdPattern = RegExp(r'^[a-z0-9][a-z0-9._:-]*$');
final _authoringStoryCatalogAliasPattern = RegExp(r'^Catalog_[0-9a-f]{64}$');
const _authoringStoryCatalogSelectorDomain =
    'gore-story-catalog.authoring-selector-v1\u0000';
const _authoringStoryCatalogBuildBindingDomain =
    'gore-story-catalog.authoring-build-v1.request-binding\u0000';

final class AuthoringStoryCatalogBuildResult {
  const AuthoringStoryCatalogBuildResult._({
    required this.requestBindingSha256,
    required this.catalogJson,
    required this.generation,
    required this.catalogSeal,
  });

  final String requestBindingSha256;
  final String catalogJson;
  final AuthoringStoryCatalogGeneration generation;
  final AuthoringDraftContentSeal catalogSeal;

  factory AuthoringStoryCatalogBuildResult._fromJson(
    Map<String, Object?> json, {
    required String executable,
    required String shippingCache,
    required String bindsCache,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'request_binding_sha256',
      'catalog_json',
      'generation',
      'catalog_seal',
    }, 'Story catalog build response');
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring Story catalog build response is not ok',
      );
    }
    final requestBindingSha256 = _authoringRequiredString(
      json,
      'request_binding_sha256',
      maxBytes: 64,
    );
    final expectedBinding = _authoringStoryCatalogBuildBindingSha256(
      executable,
      shippingCache,
      bindsCache,
    );
    if (!_authoringSha256Pattern.hasMatch(requestBindingSha256) ||
        requestBindingSha256 != expectedBinding) {
      throw const FormatException(
        'authoring Story catalog build response is not bound to its exact paths',
      );
    }
    final catalogJson = _authoringRequiredString(
      json,
      'catalog_json',
      maxBytes: _maxAuthoringStoryCatalogJsonBytes,
    );
    final rawCatalog = _authoringDecodeDuplicateSafeObject(
      catalogJson,
      'Story catalog build result',
    );
    if (jsonEncode(rawCatalog) != catalogJson) {
      throw const FormatException(
        'authoring Story catalog build result is not canonical JSON',
      );
    }
    _authoringExactFields(rawCatalog, const {
      'format',
      'schema_revision',
      'catalog',
      'catalog_seal',
    }, 'Story catalog build result');
    if (rawCatalog['format'] != 'story_catalog' ||
        rawCatalog['schema_revision'] != 1) {
      throw const FormatException(
        'authoring Story catalog build result has an unsupported format',
      );
    }
    final rawPayload = _authoringRequiredObject(
      rawCatalog['catalog'],
      'Story catalog build payload',
    );
    _authoringExactFields(rawPayload, const {
      'generation',
      'record_set_id',
      'record_set_seal',
      'npcs',
      'quest_parents',
    }, 'Story catalog build payload');
    final generation = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        json['generation'],
        'Story catalog build generation',
      ),
    );
    final rawGeneration = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        rawPayload['generation'],
        'Story catalog build raw generation',
      ),
    );
    final catalogSeal = _authoringStoryCatalogSeal(
      json['catalog_seal'],
      'build catalog_seal',
    );
    final rawCatalogSeal = _authoringStoryCatalogSeal(
      rawCatalog['catalog_seal'],
      'build raw catalog_seal',
    );
    if (!_authoringStoryCatalogSameGeneration(generation, rawGeneration) ||
        !_authoringStoryCatalogSameSeal(catalogSeal, rawCatalogSeal)) {
      throw const FormatException(
        'authoring Story catalog build response disagrees with its raw catalog',
      );
    }
    return AuthoringStoryCatalogBuildResult._(
      requestBindingSha256: requestBindingSha256,
      catalogJson: catalogJson,
      generation: generation,
      catalogSeal: catalogSeal,
    );
  }
}

final class AuthoringStoryCatalogSelections {
  const AuthoringStoryCatalogSelections._({
    required this.requestCatalogSha256,
    required this.schemaRevision,
    required this.generation,
    required this.catalogSeal,
    required this.npcs,
    required this.questParents,
    required this.questCollisionCatalog,
    required this.blocksBuild,
  });

  final String requestCatalogSha256;
  final int schemaRevision;
  final AuthoringStoryCatalogGeneration generation;
  final AuthoringDraftContentSeal catalogSeal;
  final List<AuthoringStoryCatalogNpcSelection> npcs;
  final List<AuthoringStoryCatalogQuestParentSelection> questParents;
  final AuthoringStoryCatalogCollisionAvailability questCollisionCatalog;
  final bool blocksBuild;

  factory AuthoringStoryCatalogSelections._fromJson(
    Map<String, Object?> json, {
    required String catalogJson,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'request_catalog_sha256',
      'selections',
    }, 'Story catalog response');
    if (json['ok'] != true) {
      throw const FormatException('authoring Story catalog response is not ok');
    }
    final requestCatalogSha256 = _authoringRequiredString(
      json,
      'request_catalog_sha256',
      maxBytes: 64,
    );
    final expectedBinding = crypto.sha256
        .convert(utf8.encode(catalogJson))
        .toString();
    if (!_authoringSha256Pattern.hasMatch(requestCatalogSha256) ||
        requestCatalogSha256 != expectedBinding) {
      throw const FormatException(
        'authoring Story catalog response is not bound to its exact request',
      );
    }

    final selections = _authoringRequiredObject(
      json['selections'],
      'Story catalog selections',
    );
    _authoringExactFields(selections, const {
      'schema_revision',
      'generation',
      'catalog_seal',
      'npcs',
      'quest_parents',
      'quest_collision_catalog',
      'blocks_build',
    }, 'Story catalog selections');
    final schemaRevision = _authoringRequiredInt(
      selections,
      'schema_revision',
      min: 1,
      max: 1,
    );
    final generation = AuthoringStoryCatalogGeneration._fromJson(
      _authoringRequiredObject(
        selections['generation'],
        'Story catalog generation',
      ),
    );
    final catalogSeal = _authoringStoryCatalogSeal(
      selections['catalog_seal'],
      'catalog_seal',
    );
    final npcs = _authoringStoryCatalogList(
      selections['npcs'],
      expectedLength: _maxAuthoringStoryCatalogNpcs,
      context: 'NPC selections',
      decode: AuthoringStoryCatalogNpcSelection._fromJson,
    );
    final questParents = _authoringStoryCatalogList(
      selections['quest_parents'],
      expectedLength: _maxAuthoringStoryCatalogQuestParents,
      context: 'Quest-parent selections',
      decode: AuthoringStoryCatalogQuestParentSelection._fromJson,
    );
    _authoringStoryCatalogRequireSortedIds(
      npcs.map((entry) => entry.catalogId),
      'NPC selections',
    );
    _authoringStoryCatalogRequireSortedIds(
      questParents.map((entry) => entry.catalogId),
      'Quest-parent selections',
    );
    final questCollisionCatalog =
        AuthoringStoryCatalogCollisionAvailability._fromJson(
          _authoringRequiredObject(
            selections['quest_collision_catalog'],
            'Story catalog collision availability',
          ),
        );
    if (!_authoringStoryCatalogSameSeal(
      questCollisionCatalog.sourceSeal,
      generation.shippingCache,
    )) {
      throw const FormatException(
        'authoring Story catalog collision seal is not the generation Shipping cache',
      );
    }

    final aliases = <String>{};
    for (final npc in npcs) {
      for (final alias in <String>[
        npc.characterDefinition.authoringSelector,
        npc.aiAgentConfig.authoringSelector,
        npc.spawnDefinition.authoringSelector,
        npc.questGiver.authoringSelector,
      ]) {
        if (!aliases.add(alias)) {
          throw const FormatException(
            'authoring Story catalog selector aliases are not unique',
          );
        }
      }
    }
    for (final parent in questParents) {
      if (!aliases.add(parent.questClass.authoringSelector)) {
        throw const FormatException(
          'authoring Story catalog selector aliases are not unique',
        );
      }
    }
    final blocksBuild = _authoringRequiredBool(selections, 'blocks_build');
    if (!blocksBuild ||
        npcs.any((entry) => !entry.blocksBuild) ||
        questParents.any((entry) => !entry.blocksBuild) ||
        !questCollisionCatalog.blocksDraftCreation) {
      throw const FormatException(
        'authoring Story catalog readiness gates are inconsistent',
      );
    }
    return AuthoringStoryCatalogSelections._(
      requestCatalogSha256: requestCatalogSha256,
      schemaRevision: schemaRevision,
      generation: generation,
      catalogSeal: catalogSeal,
      npcs: List.unmodifiable(npcs),
      questParents: List.unmodifiable(questParents),
      questCollisionCatalog: questCollisionCatalog,
      blocksBuild: blocksBuild,
    );
  }
}

final class AuthoringStoryCatalogGeneration {
  const AuthoringStoryCatalogGeneration._({
    required this.edition,
    required this.executable,
    required this.shippingCache,
    required this.bindsCache,
  });

  final String edition;
  final AuthoringDraftContentSeal executable;
  final AuthoringDraftContentSeal shippingCache;
  final AuthoringDraftContentSeal bindsCache;

  factory AuthoringStoryCatalogGeneration._fromJson(Map<String, Object?> json) {
    _authoringExactFields(json, const {
      'edition',
      'executable',
      'shipping_cache',
      'binds_cache',
    }, 'Story catalog generation');
    final edition = _authoringRequiredString(json, 'edition', maxBytes: 64);
    if (edition != 'g1r-steam') {
      throw const FormatException(
        'authoring Story catalog edition is not supported',
      );
    }
    return AuthoringStoryCatalogGeneration._(
      edition: edition,
      executable: _authoringStoryCatalogSeal(
        json['executable'],
        'generation.executable',
      ),
      shippingCache: _authoringStoryCatalogSeal(
        json['shipping_cache'],
        'generation.shipping_cache',
      ),
      bindsCache: _authoringStoryCatalogSeal(
        json['binds_cache'],
        'generation.binds_cache',
      ),
    );
  }
}

final class AuthoringStoryCatalogClassSelection {
  const AuthoringStoryCatalogClassSelection._({
    required this.catalogLayer,
    required this.authoringSelector,
    required this.sourceCatalogSelector,
    required this.runtimeClass,
    required this.sourceSeal,
  });

  final String catalogLayer;
  final String authoringSelector;
  final String sourceCatalogSelector;
  final String runtimeClass;
  final AuthoringDraftContentSeal sourceSeal;

  factory AuthoringStoryCatalogClassSelection._fromJson(
    Map<String, Object?> json, {
    required String catalogId,
    required String role,
    required String expectedCatalogLayer,
  }) {
    _authoringExactFields(json, const {
      'catalog_layer',
      'authoring_selector',
      'source_catalog_selector',
      'runtime_class',
      'source_seal',
    }, 'Story catalog class selection');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    _authoringDraftValidateCatalogLayer(catalogLayer, 'catalog_layer');
    if (catalogLayer != expectedCatalogLayer) {
      throw const FormatException(
        'authoring Story catalog class layer is not the pinned base-game layer',
      );
    }
    final authoringSelector = _authoringStoryCatalogSelectorAlias(
      json,
      'authoring_selector',
    );
    if (authoringSelector !=
        _authoringStoryCatalogExpectedSelector(catalogId, role)) {
      throw const FormatException(
        'authoring Story catalog class selector alias disagrees with its record and role',
      );
    }
    final sourceCatalogSelector = _authoringRequiredString(
      json,
      'source_catalog_selector',
      maxBytes: 4096,
    );
    final runtimeClass = _authoringRequiredString(
      json,
      'runtime_class',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(runtimeClass, 'runtime_class');
    _authoringStoryCatalogSourceSelector(sourceCatalogSelector, runtimeClass);
    return AuthoringStoryCatalogClassSelection._(
      catalogLayer: catalogLayer,
      authoringSelector: authoringSelector,
      sourceCatalogSelector: sourceCatalogSelector,
      runtimeClass: runtimeClass,
      sourceSeal: _authoringStoryCatalogSeal(
        json['source_seal'],
        'class.source_seal',
      ),
    );
  }
}

final class AuthoringStoryCatalogQuestGiverSelection {
  const AuthoringStoryCatalogQuestGiverSelection._({
    required this.catalogLayer,
    required this.authoringSelector,
    required this.sourceCatalogSelector,
    required this.runtimeUniqueName,
    required this.sourceSeal,
  });

  final String catalogLayer;
  final String authoringSelector;
  final String sourceCatalogSelector;
  final String runtimeUniqueName;
  final AuthoringDraftContentSeal sourceSeal;

  factory AuthoringStoryCatalogQuestGiverSelection._fromJson(
    Map<String, Object?> json, {
    required String catalogId,
    required String expectedCatalogLayer,
  }) {
    _authoringExactFields(json, const {
      'catalog_layer',
      'authoring_selector',
      'source_catalog_selector',
      'runtime_unique_name',
      'source_seal',
    }, 'Story catalog Quest-giver selection');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    _authoringDraftValidateCatalogLayer(
      catalogLayer,
      'quest_giver.catalog_layer',
    );
    if (catalogLayer != expectedCatalogLayer) {
      throw const FormatException(
        'authoring Story catalog Quest-giver layer is not the pinned base-game layer',
      );
    }
    final sourceCatalogSelector = _authoringRequiredString(
      json,
      'source_catalog_selector',
      maxBytes: 4096,
    );
    final runtimeUniqueName = _authoringRequiredString(
      json,
      'runtime_unique_name',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(
      runtimeUniqueName,
      'quest_giver.runtime_unique_name',
    );
    _authoringStoryCatalogSourceSelector(
      sourceCatalogSelector,
      'UCharacterDefinition_Human_$runtimeUniqueName',
    );
    final authoringSelector = _authoringStoryCatalogSelectorAlias(
      json,
      'authoring_selector',
    );
    if (authoringSelector !=
        _authoringStoryCatalogExpectedSelector(catalogId, 'quest_giver')) {
      throw const FormatException(
        'authoring Story catalog Quest-giver selector alias disagrees with its record and role',
      );
    }
    return AuthoringStoryCatalogQuestGiverSelection._(
      catalogLayer: catalogLayer,
      authoringSelector: authoringSelector,
      sourceCatalogSelector: sourceCatalogSelector,
      runtimeUniqueName: runtimeUniqueName,
      sourceSeal: _authoringStoryCatalogSeal(
        json['source_seal'],
        'quest_giver.source_seal',
      ),
    );
  }
}

final class AuthoringStoryCatalogNpcSelection {
  const AuthoringStoryCatalogNpcSelection._({
    required this.catalogId,
    required this.displayName,
    required this.runtimeUniqueName,
    required this.characterDefinition,
    required this.aiAgentConfig,
    required this.spawnDefinition,
    required this.questGiver,
    required this.discoveryStatus,
    required this.authoringQualification,
    required this.runtimeQualification,
    required this.evidenceId,
    required this.blocksBuild,
  });

  final String catalogId;
  final String displayName;
  final String runtimeUniqueName;
  final AuthoringStoryCatalogClassSelection characterDefinition;
  final AuthoringStoryCatalogClassSelection aiAgentConfig;
  final AuthoringStoryCatalogClassSelection spawnDefinition;
  final AuthoringStoryCatalogQuestGiverSelection questGiver;
  final AuthoringStoryCatalogNpcDiscoveryStatus discoveryStatus;
  final AuthoringStoryCatalogNpcAuthoringQualification authoringQualification;
  final AuthoringStoryCatalogRuntimeQualification runtimeQualification;
  final String evidenceId;
  final bool blocksBuild;

  factory AuthoringStoryCatalogNpcSelection._fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'catalog_id',
      'display_name',
      'runtime_unique_name',
      'character_definition',
      'ai_agent_config',
      'spawn_definition',
      'quest_giver',
      'discovery_status',
      'authoring_qualification',
      'runtime_qualification',
      'evidence_id',
      'blocks_build',
    }, 'Story catalog NPC selection');
    final catalogId = _authoringStoryCatalogId(json, 'catalog_id', 'g1r:npc:');
    final runtimeUniqueName = _authoringRequiredString(
      json,
      'runtime_unique_name',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(runtimeUniqueName, 'runtime_unique_name');
    final characterDefinition = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['character_definition'],
        'Story catalog character definition',
      ),
      catalogId: catalogId,
      role: 'character_definition',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    final aiAgentConfig = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['ai_agent_config'],
        'Story catalog AI-agent config',
      ),
      catalogId: catalogId,
      role: 'ai_agent_config',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    final spawnDefinition = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['spawn_definition'],
        'Story catalog spawn definition',
      ),
      catalogId: catalogId,
      role: 'spawn_definition',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    if (!characterDefinition.runtimeClass.startsWith('UCharacterDefinition_') ||
        !aiAgentConfig.runtimeClass.startsWith('UAIAgentConfig_') ||
        !spawnDefinition.runtimeClass.startsWith('USpawnAIAgentDefinition_')) {
      throw const FormatException(
        'authoring Story catalog NPC class roles are inconsistent',
      );
    }
    final questGiver = AuthoringStoryCatalogQuestGiverSelection._fromJson(
      _authoringRequiredObject(
        json['quest_giver'],
        'Story catalog Quest giver',
      ),
      catalogId: catalogId,
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    if (questGiver.runtimeUniqueName != runtimeUniqueName ||
        questGiver.catalogLayer != characterDefinition.catalogLayer ||
        questGiver.sourceCatalogSelector !=
            characterDefinition.sourceCatalogSelector ||
        !_authoringStoryCatalogSameSeal(
          questGiver.sourceSeal,
          characterDefinition.sourceSeal,
        )) {
      throw const FormatException(
        'authoring Story catalog Quest giver disagrees with its NPC provenance',
      );
    }
    return AuthoringStoryCatalogNpcSelection._(
      catalogId: catalogId,
      displayName: _authoringStoryCatalogFriendlyText(json, 'display_name'),
      runtimeUniqueName: runtimeUniqueName,
      characterDefinition: characterDefinition,
      aiAgentConfig: aiAgentConfig,
      spawnDefinition: spawnDefinition,
      questGiver: questGiver,
      discoveryStatus: switch (json['discovery_status']) {
        'sealed_cache_defaults_verified' =>
          AuthoringStoryCatalogNpcDiscoveryStatus.sealedCacheDefaultsVerified,
        _ => throw const FormatException(
          'authoring Story catalog NPC discovery status is unsupported',
        ),
      },
      authoringQualification: switch (json['authoring_qualification']) {
        'offline_qualified' =>
          AuthoringStoryCatalogNpcAuthoringQualification.offlineQualified,
        _ => throw const FormatException(
          'authoring Story catalog NPC authoring qualification is unsupported',
        ),
      },
      runtimeQualification: _authoringStoryCatalogRuntimeQualification(
        json['runtime_qualification'],
      ),
      evidenceId: _authoringStoryCatalogEvidenceId(json, 'evidence_id'),
      blocksBuild: _authoringRequiredBool(json, 'blocks_build'),
    );
  }
}

final class AuthoringStoryCatalogQuestParentSelection {
  const AuthoringStoryCatalogQuestParentSelection._({
    required this.catalogId,
    required this.displayName,
    required this.questClass,
    required this.parentClassName,
    required this.role,
    required this.qualification,
    required this.transitionQualification,
    required this.evidenceId,
    required this.blocksBuild,
  });

  final String catalogId;
  final String displayName;
  final AuthoringStoryCatalogClassSelection questClass;
  final String parentClassName;
  final AuthoringStoryCatalogQuestParentRole role;
  final AuthoringStoryCatalogQuestParentQualification qualification;
  final AuthoringStoryCatalogRuntimeQualification transitionQualification;
  final String evidenceId;
  final bool blocksBuild;

  factory AuthoringStoryCatalogQuestParentSelection._fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'catalog_id',
      'display_name',
      'quest_class',
      'parent_class_name',
      'role',
      'qualification',
      'transition_qualification',
      'evidence_id',
      'blocks_build',
    }, 'Story catalog Quest-parent selection');
    final catalogId = _authoringStoryCatalogId(
      json,
      'catalog_id',
      'g1r:quest-parent:',
    );
    final questClass = AuthoringStoryCatalogClassSelection._fromJson(
      _authoringRequiredObject(
        json['quest_class'],
        'Story catalog Quest class',
      ),
      catalogId: catalogId,
      role: 'quest_parent',
      expectedCatalogLayer: 'base-game.g1r.scripts',
    );
    final parentClassName = _authoringRequiredString(
      json,
      'parent_class_name',
      maxBytes: 128,
    );
    _authoringDraftValidateIdentifier(parentClassName, 'parent_class_name');
    if (!questClass.runtimeClass.startsWith('UQuest_') ||
        !parentClassName.startsWith('UQuest_') ||
        questClass.runtimeClass == parentClassName) {
      throw const FormatException(
        'authoring Story catalog Quest parent classes are inconsistent',
      );
    }
    return AuthoringStoryCatalogQuestParentSelection._(
      catalogId: catalogId,
      displayName: _authoringStoryCatalogFriendlyText(json, 'display_name'),
      questClass: questClass,
      parentClassName: parentClassName,
      role: switch (json['role']) {
        'chapter' => AuthoringStoryCatalogQuestParentRole.chapter,
        _ => throw const FormatException(
          'authoring Story catalog Quest-parent role is unsupported',
        ),
      },
      qualification: switch (json['qualification']) {
        'curated_defaults_verified' =>
          AuthoringStoryCatalogQuestParentQualification.curatedDefaultsVerified,
        _ => throw const FormatException(
          'authoring Story catalog Quest-parent qualification is unsupported',
        ),
      },
      transitionQualification: _authoringStoryCatalogRuntimeQualification(
        json['transition_qualification'],
      ),
      evidenceId: _authoringStoryCatalogEvidenceId(json, 'evidence_id'),
      blocksBuild: _authoringRequiredBool(json, 'blocks_build'),
    );
  }
}

final class AuthoringStoryCatalogCollisionAvailability {
  const AuthoringStoryCatalogCollisionAvailability._({
    required this.status,
    required this.catalogLayer,
    required this.sourceSeal,
    required this.blocksDraftCreation,
  });

  final AuthoringStoryCatalogCollisionStatus status;
  final String catalogLayer;
  final AuthoringDraftContentSeal sourceSeal;
  final bool blocksDraftCreation;

  factory AuthoringStoryCatalogCollisionAvailability._fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const {
      'status',
      'catalog_layer',
      'source_seal',
      'blocks_draft_creation',
    }, 'Story catalog collision availability');
    final catalogLayer = _authoringRequiredString(
      json,
      'catalog_layer',
      maxBytes: 128,
    );
    if (catalogLayer != 'resolved-loadout.scripts.v1') {
      throw const FormatException(
        'authoring Story catalog collision layer is unsupported',
      );
    }
    final blocksDraftCreation = _authoringRequiredBool(
      json,
      'blocks_draft_creation',
    );
    if (!blocksDraftCreation) {
      throw const FormatException(
        'authoring Story catalog must block Quest Draft creation without an inventory',
      );
    }
    return AuthoringStoryCatalogCollisionAvailability._(
      status: switch (json['status']) {
        'inventory_unavailable' =>
          AuthoringStoryCatalogCollisionStatus.inventoryUnavailable,
        _ => throw const FormatException(
          'authoring Story catalog collision status is unsupported',
        ),
      },
      catalogLayer: catalogLayer,
      sourceSeal: _authoringStoryCatalogSeal(
        json['source_seal'],
        'quest_collision_catalog.source_seal',
      ),
      blocksDraftCreation: blocksDraftCreation,
    );
  }
}

List<T> _authoringStoryCatalogList<T>(
  Object? raw, {
  required int expectedLength,
  required String context,
  required T Function(Map<String, Object?>) decode,
}) {
  if (raw is! List || raw.length != expectedLength) {
    throw FormatException('authoring Story catalog $context has invalid size');
  }
  return [
    for (var index = 0; index < raw.length; index++)
      decode(
        _authoringRequiredObject(
          raw[index],
          'Story catalog $context entry $index',
        ),
      ),
  ];
}

void _authoringStoryCatalogRequireSortedIds(
  Iterable<String> values,
  String context,
) {
  String? previous;
  for (final value in values) {
    if (previous != null && previous.compareTo(value) >= 0) {
      throw FormatException(
        'authoring Story catalog $context is not sorted and unique',
      );
    }
    previous = value;
  }
}

AuthoringDraftContentSeal _authoringStoryCatalogSeal(
  Object? raw,
  String context,
) {
  final seal = AuthoringDraftContentSeal.fromJson(
    _authoringRequiredObject(raw, 'Story catalog $context'),
  );
  if (seal.byteLength > 0x7fffffffffffffff) {
    throw FormatException(
      'authoring Story catalog $context exceeds wire range',
    );
  }
  return seal;
}

bool _authoringStoryCatalogSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

bool _authoringStoryCatalogSameGeneration(
  AuthoringStoryCatalogGeneration left,
  AuthoringStoryCatalogGeneration right,
) =>
    left.edition == right.edition &&
    _authoringStoryCatalogSameSeal(left.executable, right.executable) &&
    _authoringStoryCatalogSameSeal(left.shippingCache, right.shippingCache) &&
    _authoringStoryCatalogSameSeal(left.bindsCache, right.bindsCache);

String _authoringStoryCatalogSelectorAlias(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 72);
  if (!_authoringStoryCatalogAliasPattern.hasMatch(value)) {
    throw const FormatException(
      'authoring Story catalog selector alias is invalid',
    );
  }
  return value;
}

String _authoringStoryCatalogExpectedSelector(String catalogId, String role) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryCatalogSelectorDomain));
  for (final value in <String>[catalogId, role]) {
    final bytes = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
    input
      ..add(length)
      ..add(bytes);
  }
  input.close();
  return 'Catalog_${output.value}';
}

String _authoringStoryCatalogBuildBindingSha256(
  String executable,
  String shippingCache,
  String bindsCache,
) {
  final output = _AuthoringDigestCollector();
  final input = crypto.sha256.startChunkedConversion(output);
  input.add(utf8.encode(_authoringStoryCatalogBuildBindingDomain));
  for (final value in <String>[executable, shippingCache, bindsCache]) {
    final bytes = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
    input
      ..add(length)
      ..add(bytes);
  }
  input.close();
  return output.value.toString();
}

void _authoringStoryCatalogSourceSelector(String value, String runtimeClass) {
  if (!value.startsWith('script-class:') ||
      !value.endsWith('/$runtimeClass') ||
      value.codeUnits.any(
        (unit) => unit < 0x21 || unit > 0x7e || unit == 0x22 || unit == 0x5c,
      )) {
    throw const FormatException(
      'authoring Story catalog source selector is invalid',
    );
  }
}

String _authoringStoryCatalogId(
  Map<String, Object?> json,
  String field,
  String prefix,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 256);
  if (!value.startsWith(prefix) ||
      !_authoringStoryCatalogIdPattern.hasMatch(value)) {
    throw const FormatException('authoring Story catalog ID is invalid');
  }
  return value;
}

String _authoringStoryCatalogFriendlyText(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 256);
  if (value.trim() != value ||
      value.runes.any((rune) => rune < 0x20 || rune == 0x7f)) {
    throw const FormatException(
      'authoring Story catalog display text is invalid',
    );
  }
  return value;
}

String _authoringStoryCatalogEvidenceId(
  Map<String, Object?> json,
  String field,
) {
  final value = _authoringRequiredString(json, field, maxBytes: 512);
  if (!_authoringStoryCatalogIdPattern.hasMatch(value)) {
    throw const FormatException(
      'authoring Story catalog evidence ID is invalid',
    );
  }
  return value;
}

AuthoringStoryCatalogRuntimeQualification
_authoringStoryCatalogRuntimeQualification(Object? value) => switch (value) {
  'runtime_unqualified' =>
    AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified,
  _ => throw const FormatException(
    'authoring Story catalog runtime qualification is unsupported',
  ),
};

enum AuthoringStoryDraftKind {
  npcDraft('npc_draft'),
  questDraft('quest_draft');

  const AuthoringStoryDraftKind(this.wireName);
  final String wireName;
}

sealed class AuthoringStoryDraftInsertResult {
  const AuthoringStoryDraftInsertResult();

  String get requestBindingSha256;

  factory AuthoringStoryDraftInsertResult.fromJson(
    Map<String, Object?> json, {
    required String projectJson,
    required String mutationJson,
    required AuthoringValidationProfile profile,
  }) {
    if (json['ok'] != true) {
      throw const FormatException(
        'authoring Story Draft insert response is not ok',
      );
    }
    final expectedBinding = _authoringStoryRequestBindingSha256(
      projectJson,
      mutationJson,
      profile,
    );
    final actualBinding = _authoringRequiredString(
      json,
      'request_binding_sha256',
      maxBytes: 64,
    );
    if (!_authoringSha256Pattern.hasMatch(actualBinding) ||
        actualBinding != expectedBinding) {
      throw const FormatException(
        'authoring Story Draft response is not bound to its exact request',
      );
    }
    return switch (json['outcome']) {
      'applied' => AuthoringStoryDraftInsertApplied._fromJson(
        json,
        _authoringStoryRequestContext(projectJson, mutationJson),
        actualBinding,
      ),
      'rejected' => AuthoringStoryDraftInsertRejected._fromJson(
        json,
        actualBinding,
      ),
      _ => throw const FormatException(
        'authoring Story Draft insert outcome is not supported',
      ),
    };
  }
}

final class AuthoringStoryDraftInsertApplied
    extends AuthoringStoryDraftInsertResult {
  const AuthoringStoryDraftInsertApplied._({
    required this.requestBindingSha256,
    required this.projectJson,
    required this.revision,
    required this.draftId,
    required this.draftKind,
    required this.scriptModuleId,
    required this.diagnostics,
    required this.blocksBuild,
  });

  @override
  final String requestBindingSha256;
  final String projectJson;
  final int revision;
  final String draftId;
  final AuthoringStoryDraftKind draftKind;
  final String scriptModuleId;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  factory AuthoringStoryDraftInsertApplied._fromJson(
    Map<String, Object?> json,
    _AuthoringStoryRequestContext context,
    String requestBindingSha256,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'request_binding_sha256',
      'project_json',
      'revision',
      'draft_id',
      'draft_kind',
      'script_module_id',
      'diagnostics',
      'blocks_build',
    }, 'Story Draft applied response');
    if (json['ok'] != true || json['outcome'] != 'applied') {
      throw const FormatException(
        'authoring Story Draft applied response has an invalid discriminator',
      );
    }
    final projectJson = _authoringRequiredString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final project = _authoringRequireCanonicalProjectJson(projectJson);
    if (project.schemaRevision != 2) {
      throw const FormatException(
        'authoring Story Draft candidate is not schema revision 2',
      );
    }
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    if (revision != context.baseRevision + 1 ||
        revision != project.revision ||
        project.projectId != context.projectId) {
      throw const FormatException(
        'authoring Story Draft candidate identity or revision disagrees with its base',
      );
    }
    final draftId = _authoringEntityId(
      _authoringRequiredString(json, 'draft_id', maxBytes: 32),
      'draft_id',
    );
    final scriptModuleId = _authoringEntityId(
      _authoringRequiredString(json, 'script_module_id', maxBytes: 32),
      'script_module_id',
    );
    if (draftId != context.draftId ||
        scriptModuleId != context.scriptModuleId) {
      throw const FormatException(
        'authoring Story Draft response entity IDs disagree with its request',
      );
    }
    final draftKind = switch (json['draft_kind']) {
      'npc_draft' => AuthoringStoryDraftKind.npcDraft,
      'quest_draft' => AuthoringStoryDraftKind.questDraft,
      _ => throw const FormatException(
        'authoring Story Draft response kind is not supported',
      ),
    };
    if (draftKind != context.draftKind) {
      throw const FormatException(
        'authoring Story Draft response kind disagrees with its request',
      );
    }
    _authoringRequireStoryCandidateOwnership(project.project, context);
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    _authoringRequireRevision2CombinedGate(
      blocksBuild,
      diagnostics,
      'Story Draft applied response',
    );
    _authoringValidateBlocksBuild(blocksBuild, diagnostics);
    return AuthoringStoryDraftInsertApplied._(
      requestBindingSha256: requestBindingSha256,
      projectJson: projectJson,
      revision: revision,
      draftId: draftId,
      draftKind: draftKind,
      scriptModuleId: scriptModuleId,
      diagnostics: diagnostics,
      blocksBuild: blocksBuild,
    );
  }
}

final class AuthoringStoryDraftInsertRejected
    extends AuthoringStoryDraftInsertResult {
  const AuthoringStoryDraftInsertRejected._({
    required this.requestBindingSha256,
    required this.diagnostics,
  });

  @override
  final String requestBindingSha256;
  final List<AuthoringDiagnostic> diagnostics;

  factory AuthoringStoryDraftInsertRejected._fromJson(
    Map<String, Object?> json,
    String requestBindingSha256,
  ) {
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'request_binding_sha256',
      'diagnostics',
    }, 'Story Draft rejected response');
    if (json['ok'] != true || json['outcome'] != 'rejected') {
      throw const FormatException(
        'authoring Story Draft rejected response has an invalid discriminator',
      );
    }
    final diagnostics = _authoringDiagnostics(json);
    if (!diagnostics.any(
      (diagnostic) =>
          diagnostic.severity == AuthoringDiagnosticSeverity.error &&
          diagnostic.blocksBuild,
    )) {
      throw const FormatException(
        'authoring Story Draft rejection has no blocking error diagnostic',
      );
    }
    return AuthoringStoryDraftInsertRejected._(
      requestBindingSha256: requestBindingSha256,
      diagnostics: diagnostics,
    );
  }
}

final class _AuthoringStoryRequestContext {
  const _AuthoringStoryRequestContext({
    required this.baseProject,
    required this.projectId,
    required this.baseRevision,
    required this.draftId,
    required this.scriptModuleId,
    required this.displayName,
    required this.draftKind,
    required this.input,
    required this.moduleNamespace,
    required this.runtimeId,
    required this.generatorId,
    required this.generatorVersion,
  });

  final Map<String, Object?> baseProject;
  final String projectId;
  final int baseRevision;
  final String draftId;
  final String scriptModuleId;
  final String displayName;
  final AuthoringStoryDraftKind draftKind;
  final Map<String, Object?> input;
  final String moduleNamespace;
  final String runtimeId;
  final String generatorId;
  final int generatorVersion;
}

_AuthoringStoryRequestContext _authoringStoryRequestContext(
  String projectJson,
  String mutationJson,
) {
  final project = _authoringRequireCanonicalProjectJson(projectJson);
  if (project.schemaRevision != 2) {
    throw const FormatException(
      'authoring Story Draft base is not canonical schema revision 2',
    );
  }
  if (project.revision > _maxAuthoringStoryBaseRevision) {
    throw const FormatException(
      'authoring Story Draft base revision exceeds the signed wire contract',
    );
  }
  final mutation = _authoringDecodeDuplicateSafeObject(
    mutationJson,
    'Story Draft mutation',
  );
  _authoringExactFields(mutation, const {
    'expected_project_id',
    'expected_revision',
    'draft_id',
    'script_module_id',
    'display_name',
    'draft',
  }, 'Story Draft mutation');
  final expectedProjectId = _authoringEntityId(
    _authoringRequiredString(mutation, 'expected_project_id', maxBytes: 32),
    'expected_project_id',
  );
  final expectedRevision = _authoringRequiredInt(
    mutation,
    'expected_revision',
    max: _maxAuthoringStoryBaseRevision,
  );
  if (expectedProjectId != project.projectId ||
      expectedRevision != project.revision) {
    throw const FormatException(
      'authoring Story Draft mutation does not name its exact base project',
    );
  }
  final draftId = _authoringEntityId(
    _authoringRequiredString(mutation, 'draft_id', maxBytes: 32),
    'draft_id',
  );
  final scriptModuleId = _authoringEntityId(
    _authoringRequiredString(mutation, 'script_module_id', maxBytes: 32),
    'script_module_id',
  );
  if (draftId == '00000000000000000000000000000000' ||
      scriptModuleId == '00000000000000000000000000000000' ||
      draftId == scriptModuleId) {
    throw const FormatException(
      'authoring Story Draft mutation IDs must be distinct and non-zero',
    );
  }
  final displayName = _authoringRequiredString(
    mutation,
    'display_name',
    maxBytes: 256,
  );
  final draft = _authoringRequiredObject(
    mutation['draft'],
    'Story Draft mutation draft',
  );
  _authoringExactFields(draft, const {
    'kind',
    'input',
  }, 'Story Draft mutation draft');
  final input = _authoringRequiredObject(
    draft['input'],
    'Story Draft mutation input',
  );
  final (
    draftKind,
    moduleNamespace,
    runtimeId,
    generatorId,
    generatorVersion,
  ) = switch (draft['kind']) {
    'npc' => () {
      _authoringExactFields(input, const {
        'module_namespace',
        'unique_name',
        'parent_character_definition',
        'parent_ai_agent_config',
        'parent_spawn_definition',
      }, 'NPC Story Draft mutation input');
      final moduleNamespace = _authoringRequiredString(
        input,
        'module_namespace',
        maxBytes: 255,
      );
      _authoringDraftValidateModuleNamespace(moduleNamespace);
      return (
        AuthoringStoryDraftKind.npcDraft,
        moduleNamespace,
        _authoringRequiredString(input, 'unique_name', maxBytes: 128),
        'gore-authoring.logical-npc-clone-draft',
        1,
      );
    }(),
    'quest' => () {
      _authoringExactFields(input, const {
        'module_namespace',
        'technical_id',
        'text_helper',
        'parent_quest',
        'giver',
        'title',
        'description',
        'objective_title',
        'collision_catalog',
      }, 'Quest Story Draft mutation input');
      final moduleNamespace = _authoringRequiredString(
        input,
        'module_namespace',
        maxBytes: 255,
      );
      _authoringDraftValidateModuleNamespace(moduleNamespace);
      return (
        AuthoringStoryDraftKind.questDraft,
        moduleNamespace,
        _authoringRequiredString(input, 'technical_id', maxBytes: 128),
        'gore-authoring.draft-quest-skeleton',
        1,
      );
    }(),
    _ => throw const FormatException(
      'authoring Story Draft mutation kind is not supported',
    ),
  };
  return _AuthoringStoryRequestContext(
    baseProject: project.project,
    projectId: project.projectId,
    baseRevision: project.revision,
    draftId: draftId,
    scriptModuleId: scriptModuleId,
    displayName: displayName,
    draftKind: draftKind,
    input: input,
    moduleNamespace: moduleNamespace,
    runtimeId: runtimeId,
    generatorId: generatorId,
    generatorVersion: generatorVersion,
  );
}

void _authoringRequireStoryCandidateOwnership(
  Map<String, Object?> project,
  _AuthoringStoryRequestContext context,
) {
  _authoringRequireStoryCandidatePreservesBase(project, context);
  final entities = project['entities'] as Map<dynamic, dynamic>;
  final draftEntity = _authoringStoryCandidateEntity(
    entities,
    context.draftId,
    context.draftKind.wireName,
    'Draft',
  );
  if (draftEntity['display_name'] != context.displayName ||
      draftEntity['revision'] != 0) {
    throw const FormatException(
      'authoring Story Draft candidate Draft metadata disagrees with the request',
    );
  }
  final draftOrigin = _authoringRequiredObject(
    draftEntity['origin'],
    'Story Draft candidate Draft origin',
  );
  _authoringExactFields(draftOrigin, const {
    'type',
    'authored_runtime_id',
  }, 'Story Draft candidate Draft origin');
  if (draftOrigin['type'] != 'new' ||
      draftOrigin['authored_runtime_id'] != context.runtimeId) {
    throw const FormatException(
      'authoring Story Draft candidate Draft origin disagrees with the request',
    );
  }
  final draftPayload = _authoringRequiredObject(
    draftEntity['payload'],
    'Story Draft candidate Draft payload',
  );
  final draftData = _authoringRequiredObject(
    draftPayload['data'],
    'Story Draft candidate Draft data',
  );
  _authoringExactFields(draftData, const {
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  }, 'Story Draft candidate Draft data');
  if (draftData['generator_id'] != context.generatorId ||
      draftData['generator_version'] != context.generatorVersion) {
    throw const FormatException(
      'authoring Story Draft candidate Draft generator disagrees with the request',
    );
  }
  final expectedInput = <String, Object?>{
    'target': context.baseProject['target'],
    if (context.draftKind == AuthoringStoryDraftKind.questDraft)
      'quest_id': context.draftId,
    ...context.input,
  };
  if (!_authoringJsonDeepEquals(draftData['input'], expectedInput)) {
    throw const FormatException(
      'authoring Story Draft candidate Draft input disagrees with the request',
    );
  }
  _authoringRequireTypedStoryRef(
    draftData['script_module'],
    projectId: context.projectId,
    id: context.scriptModuleId,
    kind: 'script_module',
    context: 'Draft ScriptModule reference',
  );

  final moduleEntity = _authoringStoryCandidateEntity(
    entities,
    context.scriptModuleId,
    'script_module',
    'ScriptModule',
  );
  if (moduleEntity['display_name'] != context.moduleNamespace ||
      moduleEntity['revision'] != 0) {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule metadata is invalid',
    );
  }
  final moduleOrigin = _authoringRequiredObject(
    moduleEntity['origin'],
    'Story Draft candidate ScriptModule origin',
  );
  _authoringExactFields(moduleOrigin, const {
    'type',
    'generator_id',
    'generator_version',
    'owner',
  }, 'Story Draft candidate ScriptModule origin');
  if (moduleOrigin['type'] != 'generated' ||
      moduleOrigin['generator_id'] != context.generatorId ||
      moduleOrigin['generator_version'] != context.generatorVersion) {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule origin generator is invalid',
    );
  }
  _authoringRequireTypedStoryRef(
    moduleOrigin['owner'],
    projectId: context.projectId,
    id: context.draftId,
    kind: context.draftKind.wireName,
    context: 'ScriptModule origin owner',
  );
  final modulePayload = _authoringRequiredObject(
    moduleEntity['payload'],
    'Story Draft candidate ScriptModule payload',
  );
  final moduleData = _authoringRequiredObject(
    modulePayload['data'],
    'Story Draft candidate ScriptModule data',
  );
  _authoringExactFields(moduleData, const {
    'generator_id',
    'generator_version',
    'owner',
    'module_namespace',
    'module_relative_path',
    'source',
    'source_sha256',
    'input_fingerprint',
    'status',
  }, 'Story Draft candidate ScriptModule data');
  if (moduleData['generator_id'] != context.generatorId ||
      moduleData['generator_version'] != context.generatorVersion) {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule payload generator is invalid',
    );
  }
  if (moduleData['module_namespace'] != context.moduleNamespace ||
      moduleData['module_relative_path'] !=
          '${context.moduleNamespace.replaceAll('.', '/')}.as') {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule path is inconsistent',
    );
  }
  final source = _authoringRequiredString(
    moduleData,
    'source',
    maxBytes: _maxAuthoringDraftSourceBytes,
  );
  _authoringDraftVerifiedSourceSha256(moduleData, source);
  _authoringDraftSha256(moduleData, 'input_fingerprint');
  final status = _authoringRequiredObject(
    moduleData['status'],
    'Story Draft candidate ScriptModule status',
  );
  _authoringExactFields(status, const {
    'authoring',
    'runtime',
  }, 'Story Draft candidate ScriptModule status');
  if (status['authoring'] != 'offline_draft' ||
      status['runtime'] != 'runtime_unqualified') {
    throw const FormatException(
      'authoring Story Draft candidate ScriptModule status is invalid',
    );
  }
  _authoringRequireTypedStoryRef(
    moduleData['owner'],
    projectId: context.projectId,
    id: context.draftId,
    kind: context.draftKind.wireName,
    context: 'ScriptModule payload owner',
  );
}

void _authoringRequireStoryCandidatePreservesBase(
  Map<String, Object?> candidate,
  _AuthoringStoryRequestContext context,
) {
  for (final field in const <String>[
    'meta',
    'target',
    'authoring_locales',
    'asset_store',
  ]) {
    if (!_authoringJsonDeepEquals(
      candidate[field],
      context.baseProject[field],
    )) {
      throw FormatException(
        'authoring Story Draft candidate changed base field $field',
      );
    }
  }

  final baseEntities = context.baseProject['entities'];
  final candidateEntities = candidate['entities'];
  if (baseEntities is! Map || candidateEntities is! Map) {
    throw const FormatException(
      'authoring Story Draft base or candidate entities are not an object',
    );
  }
  if (baseEntities.containsKey(context.draftId) ||
      baseEntities.containsKey(context.scriptModuleId) ||
      candidateEntities.length != baseEntities.length + 2) {
    throw const FormatException(
      'authoring Story Draft candidate entity delta is not exactly two additions',
    );
  }
  for (final entry in baseEntities.entries) {
    if (entry.key is! String ||
        !candidateEntities.containsKey(entry.key) ||
        !_authoringJsonDeepEquals(candidateEntities[entry.key], entry.value)) {
      throw const FormatException(
        'authoring Story Draft candidate changed a preexisting entity',
      );
    }
  }
  for (final key in candidateEntities.keys) {
    if (key is! String ||
        (!baseEntities.containsKey(key) &&
            key != context.draftId &&
            key != context.scriptModuleId)) {
      throw const FormatException(
        'authoring Story Draft candidate added an unexpected entity',
      );
    }
  }
}

bool _authoringJsonDeepEquals(Object? left, Object? right, [int depth = 0]) {
  if (depth > 128) {
    throw const FormatException(
      'authoring Story Draft JSON exceeds the maximum nesting depth',
    );
  }
  if (left is Map && right is Map) {
    if (left.length != right.length) return false;
    for (final entry in left.entries) {
      if (entry.key is! String ||
          !right.containsKey(entry.key) ||
          !_authoringJsonDeepEquals(entry.value, right[entry.key], depth + 1)) {
        return false;
      }
    }
    return true;
  }
  if (left is List && right is List) {
    if (left.length != right.length) return false;
    for (var index = 0; index < left.length; index++) {
      if (!_authoringJsonDeepEquals(left[index], right[index], depth + 1)) {
        return false;
      }
    }
    return true;
  }
  if (left is num || right is num) {
    return left.runtimeType == right.runtimeType && left == right;
  }
  return left == right;
}

Map<String, Object?> _authoringStoryCandidateEntity(
  Map<dynamic, dynamic> entities,
  String id,
  String expectedKind,
  String context,
) {
  final entity = _authoringRequiredObject(
    entities[id],
    'Story Draft candidate $context entity',
  );
  _authoringExactFields(entity, const {
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, 'Story Draft candidate $context entity');
  if (entity['id'] != id) {
    throw FormatException(
      'authoring Story Draft candidate $context key and ID disagree',
    );
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    'Story Draft candidate $context payload',
  );
  _authoringExactFields(payload, const {
    'kind',
    'data',
  }, 'Story Draft candidate $context payload');
  if (payload['kind'] != expectedKind) {
    throw FormatException(
      'authoring Story Draft candidate $context kind disagrees with the response',
    );
  }
  return entity;
}

void _authoringRequireTypedStoryRef(
  Object? value, {
  required String projectId,
  required String id,
  required String kind,
  required String context,
}) {
  final ref = _authoringRequiredObject(value, 'Story Draft candidate $context');
  _authoringExactFields(ref, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'Story Draft candidate $context');
  if (ref['project_id'] != projectId ||
      ref['id'] != id ||
      ref['expected_kind'] != kind) {
    throw FormatException(
      'authoring Story Draft candidate $context is not exact',
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

void _authoringRequireRevision2CombinedGate(
  bool blocksBuild,
  List<AuthoringDiagnostic> diagnostics,
  String context,
) {
  if (!blocksBuild ||
      !diagnostics.any(
        (diagnostic) =>
            diagnostic.code == 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE' &&
            diagnostic.severity == AuthoringDiagnosticSeverity.error &&
            diagnostic.entity == null &&
            diagnostic.propertyPath == 'schema_revision' &&
            diagnostic.blocksBuild,
      )) {
    throw FormatException(
      'authoring $context is missing its blocking combined-validation gate',
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
    final storeFormat = object['store_format'];
    if (storeFormat is! int || storeFormat != 1) {
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
    final project = _authoringRequireCanonicalProjectJson(projectJson);
    final diagnostics = _authoringDiagnostics(json);
    final blocksBuild = _authoringRequiredBool(json, 'blocks_build');
    if (project.schemaRevision == 2) {
      _authoringRequireRevision2CombinedGate(
        blocksBuild,
        diagnostics,
        'revision-2 store response',
      );
    }
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
