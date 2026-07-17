part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceTakePreviewRequestBytes = 64 * 1024;
const _authoringRevision3VoiceTakePreviewLeaf = 'preview.ogg';
const _authoringRevision3VoiceTakePreviewAuthority =
    'exact_current_managed_cas_voice_take_v1';
const _authoringRevision3VoiceTakePreviewLifecycle =
    'native_opaque_cleanup_capability_v1';
const _authoringRevision3VoiceTakePreviewRegistrationAuthority =
    'native_owned_ephemeral_temp_capability_v1';
final _authoringRevision3VoiceTakePreviewRootBasenamePattern = RegExp(
  r'^gore-mod-studio-voice-preview-[0-9a-f]{64}$',
);
final _authoringRevision3VoiceTakePreviewCleanupTokenPattern = RegExp(
  r'^[0-9a-f]{64}$',
);

/// Exact immutable asset identity selected by a fresh managed ContentIndex.
final class AuthoringRevision3VoiceTakePreviewExpectedAsset {
  const AuthoringRevision3VoiceTakePreviewExpectedAsset({
    required this.sha256,
    required this.byteLength,
    required this.logicalName,
  });

  final String sha256;
  final int byteLength;
  final String logicalName;

  Map<String, Object?> get _canonicalObject => <String, Object?>{
    'sha256': sha256,
    'byte_len': byteLength,
    'logical_name': logicalName,
  };
}

/// Canonical exact-current request for copying one verified managed CAS Voice
/// asset into a native-created, native-owned ephemeral preview capability.
final class AuthoringRevision3VoiceTakePreviewRequestV1 {
  const AuthoringRevision3VoiceTakePreviewRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.lineId,
    required this.expectedLineRevision,
    required this.localizationId,
    required this.expectedLocalizationRevision,
    required this.expectedLocId,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.locale,
    required this.takeId,
    required this.expectedTakeRevision,
    required this.expectedAsset,
  });

  factory AuthoringRevision3VoiceTakePreviewRequestV1({
    required AuthoringWorkingHead expectedHead,
    required String expectedProjectId,
    required int expectedRevision,
    required String lineId,
    required int expectedLineRevision,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
    required String slotId,
    required int expectedSlotRevision,
    required String locale,
    required String takeId,
    required int expectedTakeRevision,
    required AuthoringRevision3VoiceTakePreviewExpectedAsset expectedAsset,
  }) => AuthoringRevision3VoiceTakePreviewRequestV1.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'expected_head': jsonDecode(expectedHead.canonicalJson),
      'expected_project_id': expectedProjectId,
      'expected_revision': expectedRevision,
      'line_id': lineId,
      'expected_line_revision': expectedLineRevision,
      'localization_id': localizationId,
      'expected_localization_revision': expectedLocalizationRevision,
      'expected_loc_id': expectedLocId,
      'slot_id': slotId,
      'expected_slot_revision': expectedSlotRevision,
      'locale': locale,
      'take_id': takeId,
      'expected_take_revision': expectedTakeRevision,
      'expected_asset': expectedAsset._canonicalObject,
    }),
  );

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String lineId;
  final int expectedLineRevision;
  final String localizationId;
  final int expectedLocalizationRevision;
  final String expectedLocId;
  final String slotId;
  final int expectedSlotRevision;
  final String locale;
  final String takeId;
  final int expectedTakeRevision;
  final AuthoringRevision3VoiceTakePreviewExpectedAsset expectedAsset;

  factory AuthoringRevision3VoiceTakePreviewRequestV1.fromCanonicalJson(
    String value,
  ) {
    try {
      _authoringRevision3RequestString(
        value,
        'voiceTakePreviewRequestJson',
        _maxAuthoringRevision3VoiceTakePreviewRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 Voice take preview request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Voice take preview request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'line_id',
      'expected_line_revision',
      'localization_id',
      'expected_localization_revision',
      'expected_loc_id',
      'slot_id',
      'expected_slot_revision',
      'locale',
      'take_id',
      'expected_take_revision',
      'expected_asset',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Voice take preview request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'take preview request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 Voice take preview request is not canonical',
      );
    }

    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Voice take preview expected head',
        ),
      ),
    );
    final expectedProjectId = _authoringRevision3VoiceEntityId(
      request,
      'expected_project_id',
    );
    final lineId = _authoringRevision3VoiceEntityId(request, 'line_id');
    final localizationId = _authoringRevision3VoiceEntityId(
      request,
      'localization_id',
    );
    final slotId = _authoringRevision3VoiceEntityId(request, 'slot_id');
    final takeId = _authoringRevision3VoiceEntityId(request, 'take_id');
    if (<String>{lineId, localizationId, slotId, takeId}.length != 4) {
      throw const FormatException(
        'revision-3 Voice take preview entity IDs must be distinct',
      );
    }
    final expectedLocId = _authoringRevision3VoiceString(
      request,
      'expected_loc_id',
      maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
    );
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(expectedLocId)) {
      throw const FormatException(
        'revision-3 Voice take preview LocID is not a safe archive basename stem',
      );
    }
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(request, 'locale', maxBytes: 35),
    );
    final asset = _authoringRequiredObject(
      request['expected_asset'],
      'revision-3 Voice take preview expected asset',
    );
    const assetFields = <String>['sha256', 'byte_len', 'logical_name'];
    _authoringExactFields(
      asset,
      assetFields.toSet(),
      'revision-3 Voice take preview expected asset',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      asset,
      assetFields,
      'take preview expected asset',
    );
    final sha256 = _authoringRevision3VoiceString(
      asset,
      'sha256',
      maxBytes: 64,
    );
    final logicalName = _authoringRevision3VoiceString(
      asset,
      'logical_name',
      maxBytes: _maxAuthoringRevision3VoiceLogicalNameBytes,
    );
    if (!_authoringSha256Pattern.hasMatch(sha256) ||
        sha256 == _authoringRevision3VoiceZeroSha256 ||
        !_authoringRevision3VoiceLogicalNameIsSafe(logicalName)) {
      throw const FormatException(
        'revision-3 Voice take preview expected asset is invalid',
      );
    }

    return AuthoringRevision3VoiceTakePreviewRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: expectedProjectId,
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      lineId: lineId,
      expectedLineRevision: _authoringRequiredInt(
        request,
        'expected_line_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      localizationId: localizationId,
      expectedLocalizationRevision: _authoringRequiredInt(
        request,
        'expected_localization_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      expectedLocId: expectedLocId,
      slotId: slotId,
      expectedSlotRevision: _authoringRequiredInt(
        request,
        'expected_slot_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      locale: locale,
      takeId: takeId,
      expectedTakeRevision: _authoringRequiredInt(
        request,
        'expected_take_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      expectedAsset: AuthoringRevision3VoiceTakePreviewExpectedAsset(
        sha256: sha256,
        byteLength: _authoringRequiredInt(
          asset,
          'byte_len',
          min: 1,
          max: _maxVoiceOggBytes,
        ),
        logicalName: logicalName,
      ),
    );
  }
}

/// Opaque native lifetime capability registered before any managed CAS read.
final class AuthoringRevision3VoiceTakePreviewRegistration {
  const AuthoringRevision3VoiceTakePreviewRegistration._({
    required this.cleanupToken,
    required this.previewRoot,
    required this.previewPath,
    required this.previewLeaf,
  });

  final String cleanupToken;
  final String previewRoot;
  final String previewPath;
  final String previewLeaf;

  factory AuthoringRevision3VoiceTakePreviewRegistration.fromJson(
    Map<String, Object?> json,
  ) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'cleanup_token',
      'preview_root',
      'preview_path',
      'preview_leaf',
      'preview_authority',
      'preview_lifecycle',
      'project_write_status',
      'game_write_status',
      'save_write_status',
      'build_status',
      'deployment_status',
      'runtime_status',
    }, 'revision-3 Voice take preview registration response');
    if (json['ok'] != true ||
        json['outcome'] != 'preview_capability_registered' ||
        json['preview_leaf'] != _authoringRevision3VoiceTakePreviewLeaf ||
        json['preview_authority'] !=
            _authoringRevision3VoiceTakePreviewRegistrationAuthority ||
        json['preview_lifecycle'] !=
            _authoringRevision3VoiceTakePreviewLifecycle ||
        json['project_write_status'] != 'not_performed' ||
        json['game_write_status'] != 'not_performed' ||
        json['save_write_status'] != 'not_performed' ||
        json['build_status'] != 'not_performed' ||
        json['deployment_status'] != 'not_performed' ||
        json['runtime_status'] != 'not_qualified') {
      throw const FormatException(
        'revision-3 Voice preview registration grants invalid authority',
      );
    }
    final cleanupToken = _authoringRequiredString(
      json,
      'cleanup_token',
      maxBytes: 64,
    );
    final previewPath = _authoringRequiredString(
      json,
      'preview_path',
      maxBytes: _maxAuthoringStorePathBytes,
    );
    final previewRoot = _authoringRequiredString(
      json,
      'preview_root',
      maxBytes: _maxAuthoringStorePathBytes,
    );
    final expectedPath = p.join(
      previewRoot,
      _authoringRevision3VoiceTakePreviewLeaf,
    );
    if (!_authoringRevision3VoiceTakePreviewCleanupTokenPattern.hasMatch(
          cleanupToken,
        ) ||
        !p.isAbsolute(previewRoot) ||
        !p.isAbsolute(previewPath) ||
        !_authoringRevision3VoiceTakePreviewRootBasenamePattern.hasMatch(
          p.basename(previewRoot),
        ) ||
        !p.equals(p.normalize(previewPath), p.normalize(expectedPath))) {
      throw const FormatException(
        'revision-3 Voice preview registration is not capability-bound',
      );
    }
    return AuthoringRevision3VoiceTakePreviewRegistration._(
      cleanupToken: cleanupToken,
      previewRoot: previewRoot,
      previewPath: previewPath,
      previewLeaf: _authoringRevision3VoiceTakePreviewLeaf,
    );
  }
}

/// Strict success receipt for one read-only exact-current CAS copy.
final class AuthoringRevision3VoiceTakePreviewMaterialization {
  const AuthoringRevision3VoiceTakePreviewMaterialization._({
    required this.basisHead,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.locId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.takeId,
    required this.takeRevision,
    required this.asset,
    required this.status,
    required this.ogg,
    required this.previewPath,
    required this.previewLeaf,
    required this.cleanupToken,
  });

  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final int lineRevision;
  final String localizationId;
  final int localizationRevision;
  final String locId;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String takeId;
  final int takeRevision;
  final AuthoringRevision3VoiceAsset asset;
  final AuthoringRevision3VoiceTakeStatus status;
  final AuthoringRevision3VoiceOggMetadata ogg;
  final String previewPath;
  final String previewLeaf;
  final String cleanupToken;

  factory AuthoringRevision3VoiceTakePreviewMaterialization.fromJson(
    Map<String, Object?> json, {
    required String previewRoot,
    required String cleanupToken,
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'project_id',
      'project_revision',
      'line_id',
      'line_revision',
      'localization_id',
      'localization_revision',
      'loc_id',
      'slot_id',
      'slot_revision',
      'locale',
      'take_id',
      'take_revision',
      'asset',
      'status',
      'ogg',
      'preview_path',
      'preview_leaf',
      'preview_authority',
      'cleanup_token',
      'preview_lifecycle',
      'project_write_status',
      'game_write_status',
      'save_write_status',
      'build_status',
      'deployment_status',
      'runtime_status',
    }, 'revision-3 Voice take preview response');
    if (json['ok'] != true ||
        json['outcome'] != 'preview_ready' ||
        json['preview_leaf'] != _authoringRevision3VoiceTakePreviewLeaf ||
        json['preview_authority'] !=
            _authoringRevision3VoiceTakePreviewAuthority ||
        json['preview_lifecycle'] !=
            _authoringRevision3VoiceTakePreviewLifecycle ||
        json['project_write_status'] != 'not_performed' ||
        json['game_write_status'] != 'not_performed' ||
        json['save_write_status'] != 'not_performed' ||
        json['build_status'] != 'not_performed' ||
        json['deployment_status'] != 'not_performed' ||
        json['runtime_status'] != 'not_qualified') {
      throw const FormatException(
        'revision-3 Voice take preview response grants invalid authority',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final asset = _authoringRevision3VoicePreviewAsset(
      json['asset'],
      request.expectedAsset,
    );
    final ogg = _authoringRevision3VoiceOgg(json['ogg']);
    final status = _authoringRevision3VoiceStatus(json['status']);
    final previewPath = _authoringRequiredString(
      json,
      'preview_path',
      maxBytes: _maxAuthoringStorePathBytes,
    );
    final responseCleanupToken = _authoringRequiredString(
      json,
      'cleanup_token',
      maxBytes: 64,
    );
    final expectedPath = p.join(
      previewRoot,
      _authoringRevision3VoiceTakePreviewLeaf,
    );
    if (!_authoringRevision3VoiceTakePreviewCleanupTokenPattern.hasMatch(
          responseCleanupToken,
        ) ||
        responseCleanupToken != cleanupToken ||
        !p.isAbsolute(previewPath) ||
        !p.equals(p.normalize(previewPath), p.normalize(expectedPath))) {
      throw const FormatException(
        'revision-3 Voice take preview response escaped its capability root',
      );
    }

    final result = AuthoringRevision3VoiceTakePreviewMaterialization._(
      basisHead: basisHead,
      projectId: _authoringRevision3VoiceEntityId(json, 'project_id'),
      projectRevision: _authoringRequiredInt(
        json,
        'project_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      lineId: _authoringRevision3VoiceEntityId(json, 'line_id'),
      lineRevision: _authoringRequiredInt(
        json,
        'line_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      localizationId: _authoringRevision3VoiceEntityId(json, 'localization_id'),
      localizationRevision: _authoringRequiredInt(
        json,
        'localization_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      locId: _authoringRevision3VoiceString(
        json,
        'loc_id',
        maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
      ),
      slotId: _authoringRevision3VoiceEntityId(json, 'slot_id'),
      slotRevision: _authoringRequiredInt(
        json,
        'slot_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      locale: _authoringRevision3VoiceLocale(
        _authoringRevision3VoiceString(json, 'locale', maxBytes: 35),
      ),
      takeId: _authoringRevision3VoiceEntityId(json, 'take_id'),
      takeRevision: _authoringRequiredInt(
        json,
        'take_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      asset: asset,
      status: status,
      ogg: ogg,
      previewPath: previewPath,
      previewLeaf: _authoringRevision3VoiceTakePreviewLeaf,
      cleanupToken: responseCleanupToken,
    );
    if (result.basisHead.canonicalJson != request.expectedHead.canonicalJson ||
        result.projectId != request.expectedProjectId ||
        result.projectRevision != request.expectedRevision ||
        result.lineId != request.lineId ||
        result.lineRevision != request.expectedLineRevision ||
        result.localizationId != request.localizationId ||
        result.localizationRevision != request.expectedLocalizationRevision ||
        result.locId != request.expectedLocId ||
        result.slotId != request.slotId ||
        result.slotRevision != request.expectedSlotRevision ||
        result.locale != request.locale ||
        result.takeId != request.takeId ||
        result.takeRevision != request.expectedTakeRevision ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(result.locId)) {
      throw const FormatException(
        'revision-3 Voice take preview response disagrees with its exact request',
      );
    }
    return result;
  }
}

AuthoringRevision3VoiceAsset _authoringRevision3VoicePreviewAsset(
  Object? value,
  AuthoringRevision3VoiceTakePreviewExpectedAsset expected,
) {
  final asset = _authoringRequiredObject(
    value,
    'revision-3 Voice take preview response asset',
  );
  _authoringExactFields(asset, const {
    'sha256',
    'byte_len',
    'logical_name',
  }, 'revision-3 Voice take preview response asset');
  final sha256 = _authoringRevision3VoiceString(asset, 'sha256', maxBytes: 64);
  final byteLength = _authoringRequiredInt(
    asset,
    'byte_len',
    min: 1,
    max: _maxVoiceOggBytes,
  );
  final logicalName = _authoringRevision3VoiceString(
    asset,
    'logical_name',
    maxBytes: _maxAuthoringRevision3VoiceLogicalNameBytes,
  );
  if (sha256 != expected.sha256 ||
      byteLength != expected.byteLength ||
      logicalName != expected.logicalName) {
    throw const FormatException(
      'revision-3 Voice take preview response asset disagrees',
    );
  }
  return AuthoringRevision3VoiceAsset._(
    sha256: sha256,
    byteLength: byteLength,
    logicalName: logicalName,
  );
}
