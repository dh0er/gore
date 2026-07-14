part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceRequestJsonBytes = 64 * 1024;
const _maxAuthoringRevision3VoiceBasisRevision = 0x7ffffffffffffffe;
const _maxAuthoringRevision3VoiceAppliedRevision = 0x7fffffffffffffff;
const _maxAuthoringRevision3VoiceTextBytes = 64 * 1024;
const _maxAuthoringRevision3VoiceDisplayNameBytes = 256;
const _maxAuthoringRevision3VoiceLogicalNameBytes = 1024;
const _maxAuthoringRevision3VoiceSlotCandidates = 1024;
const _authoringRevision3VoiceSlotGeneratorId = 'gore-authoring.voice-slot';
const _authoringRevision3VoiceSlotGeneratorVersion = 1;
const _authoringRevision3VoiceTakeImporterId = 'gore-authoring.ogg-import';
const _authoringRevision3VoiceZeroSha256 =
    '0000000000000000000000000000000000000000000000000000000000000000';

enum AuthoringRevision3VoiceTakeStatus { draft, recorded, reviewed, approved }

extension on AuthoringRevision3VoiceTakeStatus {
  String get wireName => switch (this) {
    AuthoringRevision3VoiceTakeStatus.draft => 'draft',
    AuthoringRevision3VoiceTakeStatus.recorded => 'recorded',
    AuthoringRevision3VoiceTakeStatus.reviewed => 'reviewed',
    AuthoringRevision3VoiceTakeStatus.approved => 'approved',
  };
}

/// Exact, bounded intent for importing one Ogg take into an existing revision-3 dialog line.
///
/// [forProject] derives project identity, revision, and target from the same canonical project
/// bytes later sent to native code. It cannot create archive-member or runtime-target authority.
final class AuthoringRevision3VoiceTakeRequestV1 {
  const AuthoringRevision3VoiceTakeRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.lineId,
    required this.slotId,
    required this.takeId,
    required this.locale,
    required this.text,
    required this.takeDisplayName,
    required this.logicalName,
    required this.status,
    required this.selectTake,
  });

  factory AuthoringRevision3VoiceTakeRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String lineId,
    required String slotId,
    required String takeId,
    required String locale,
    String? text,
    required String takeDisplayName,
    required String logicalName,
    required AuthoringRevision3VoiceTakeStatus status,
    bool selectTake = false,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3VoiceTakeRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'line_id': lineId,
        'slot_id': slotId,
        'take_id': takeId,
        'locale': locale,
        'text': ?text,
        'take_display_name': takeDisplayName,
        'logical_name': logicalName,
        'status': status.wireName,
        'select_take': selectTake,
      }),
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String lineId;
  final String slotId;
  final String takeId;
  final String locale;
  final String? text;
  final String takeDisplayName;
  final String logicalName;
  final AuthoringRevision3VoiceTakeStatus status;
  final bool selectTake;

  factory AuthoringRevision3VoiceTakeRequestV1.fromCanonicalJson(String value) {
    try {
      _authoringRevision3RequestString(
        value,
        'voiceRequestJson',
        _maxAuthoringRevision3VoiceRequestJsonBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 Voice request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Voice request',
    );
    final hasText = request.containsKey('text');
    final fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'slot_id',
      'take_id',
      'locale',
      if (hasText) 'text',
      'take_display_name',
      'logical_name',
      'status',
      'select_take',
    ];
    _authoringExactFields(request, fields.toSet(), 'revision-3 Voice request');
    _authoringRevision3VoiceRequireFieldOrder(request, fields, 'request');
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'authoring revision-3 Voice request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Voice expected head',
        ),
      ),
    );
    final expectedTarget = _authoringRevision3VoiceGeneration(
      request['expected_target'],
      'request target',
    );
    final lineId = _authoringRevision3VoiceEntityId(request, 'line_id');
    final slotId = _authoringRevision3VoiceEntityId(request, 'slot_id');
    final takeId = _authoringRevision3VoiceEntityId(request, 'take_id');
    if (slotId == takeId) {
      throw const FormatException(
        'authoring revision-3 Voice slot and take IDs must differ',
      );
    }
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(request, 'locale', maxBytes: 35),
    );
    final text = hasText
        ? _authoringRevision3VoiceString(
            request,
            'text',
            maxBytes: _maxAuthoringRevision3VoiceTextBytes,
            allowEmpty: true,
          )
        : null;
    if (text != null && (text.trim().isEmpty || text.contains('\u0000'))) {
      throw const FormatException(
        'authoring revision-3 Voice localized text is invalid',
      );
    }
    final takeDisplayName = _authoringRevision3VoiceString(
      request,
      'take_display_name',
      maxBytes: _maxAuthoringRevision3VoiceDisplayNameBytes,
      allowEmpty: true,
    );
    if (takeDisplayName.trim().isEmpty ||
        takeDisplayName.runes.any(_authoringRevision3VoiceControl)) {
      throw const FormatException(
        'authoring revision-3 Voice take display name is invalid',
      );
    }
    final logicalName = _authoringRevision3VoiceString(
      request,
      'logical_name',
      maxBytes: _maxAuthoringRevision3VoiceLogicalNameBytes,
      allowEmpty: true,
    );
    if (!_authoringRevision3VoiceLogicalNameIsSafe(logicalName)) {
      throw const FormatException(
        'authoring revision-3 Voice logical Ogg name is invalid',
      );
    }
    final status = _authoringRevision3VoiceStatus(request['status']);
    final selectTake = request['select_take'];
    if (selectTake is! bool) {
      throw const FormatException(
        'authoring revision-3 Voice select_take is not a bool',
      );
    }
    if (selectTake && status != AuthoringRevision3VoiceTakeStatus.approved) {
      throw const FormatException(
        'authoring revision-3 Voice cannot select an unapproved take',
      );
    }
    return AuthoringRevision3VoiceTakeRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: _authoringRevision3VoiceEntityId(
        request,
        'expected_project_id',
      ),
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(expectedTarget),
      lineId: lineId,
      slotId: slotId,
      takeId: takeId,
      locale: locale,
      text: text,
      takeDisplayName: takeDisplayName,
      logicalName: logicalName,
      status: status,
      selectTake: selectTake,
    );
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'authoring revision-3 Voice request does not bind the exact current project',
      );
    }
  }
}

enum AuthoringRevision3VoiceOggCodec { vorbis, opus }

enum AuthoringRevision3VoiceBuildStatus { blocked }

enum AuthoringRevision3VoiceRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3VoiceTargetAuthority { notGranted }

enum AuthoringRevision3VoiceNativePublicationStatus { notSupported }

final class AuthoringRevision3VoiceAsset {
  const AuthoringRevision3VoiceAsset._({
    required this.sha256,
    required this.byteLength,
    required this.logicalName,
  });

  final String sha256;
  final int byteLength;
  final String logicalName;
}

final class AuthoringRevision3VoiceOggMetadata {
  const AuthoringRevision3VoiceOggMetadata._({
    required this.codec,
    required this.channels,
    required this.sampleRate,
    required this.pages,
    required this.logicalStreams,
  });

  final AuthoringRevision3VoiceOggCodec codec;
  final int channels;
  final int sampleRate;
  final int pages;
  final int logicalStreams;
}

/// Strict native prepare-only result. Managed publication must still fully reopen [head] and win
/// its independent exact fixed-head byte CAS before exposing the project as current.
final class AuthoringRevision3VoiceTakePreparation {
  const AuthoringRevision3VoiceTakePreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.takeId,
    required this.locale,
    required this.takeStatus,
    required this.slotCreated,
    required this.selected,
    required this.asset,
    required this.ogg,
    required this.assetDeduplicated,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.targetAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String lineId;
  final String localizationId;
  final String slotId;
  final String takeId;
  final String locale;
  final AuthoringRevision3VoiceTakeStatus takeStatus;
  final bool slotCreated;
  final bool selected;
  final AuthoringRevision3VoiceAsset asset;
  final AuthoringRevision3VoiceOggMetadata ogg;
  final bool assetDeduplicated;
  final AuthoringRevision3VoiceBuildStatus buildStatus;
  final AuthoringRevision3VoiceRuntimeStatus runtimeStatus;
  final AuthoringRevision3VoiceTargetAuthority targetAuthority;
  final AuthoringRevision3VoiceNativePublicationStatus publicationStatus;

  factory AuthoringRevision3VoiceTakePreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRequestV1 request,
  }) {
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(base);
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'revision',
      'line_id',
      'localization_id',
      'slot_id',
      'take_id',
      'locale',
      'take_status',
      'slot_created',
      'selected',
      'asset',
      'ogg',
      'asset_deduplicated',
      'build_status',
      'runtime_status',
      'target_authority',
      'publication_status',
    }, 'revision-3 Voice preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 Voice preparation response is not prepared',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3VoiceString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Voice response basis head disagrees with its request',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3VoiceString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Voice candidate did not advance its head',
      );
    }
    final projectJson = _authoringRevision3VoiceString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    if (candidate.projectId != base.projectId ||
        revision != candidate.revision ||
        revision != base.revision + 1) {
      throw const FormatException(
        'authoring revision-3 Voice candidate identity or revision disagrees with its basis',
      );
    }
    final lineId = _authoringRevision3VoiceEntityId(json, 'line_id');
    final localizationId = _authoringRevision3VoiceEntityId(
      json,
      'localization_id',
    );
    final slotId = _authoringRevision3VoiceEntityId(json, 'slot_id');
    final takeId = _authoringRevision3VoiceEntityId(json, 'take_id');
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(json, 'locale', maxBytes: 35),
    );
    final takeStatus = _authoringRevision3VoiceStatus(json['take_status']);
    final slotCreated = _authoringRequiredBool(json, 'slot_created');
    final selected = _authoringRequiredBool(json, 'selected');
    if (lineId != request.lineId ||
        slotId != request.slotId ||
        takeId != request.takeId ||
        locale != request.locale ||
        takeStatus != request.status ||
        selected != request.selectTake) {
      throw const FormatException(
        'authoring revision-3 Voice response disagrees with its exact request',
      );
    }
    final asset = _authoringRevision3VoiceAsset(
      json['asset'],
      logicalName: request.logicalName,
    );
    final ogg = _authoringRevision3VoiceOgg(json['ogg']);
    final assetDeduplicated = _authoringRequiredBool(
      json,
      'asset_deduplicated',
    );
    _authoringRevision3VoiceRequireExactCandidate(
      base.project,
      candidate.project,
      request: request,
      responseLocalizationId: localizationId,
      responseSlotCreated: slotCreated,
      asset: asset,
      ogg: ogg,
    );
    return AuthoringRevision3VoiceTakePreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: candidate.projectId,
      revision: revision,
      lineId: lineId,
      localizationId: localizationId,
      slotId: slotId,
      takeId: takeId,
      locale: locale,
      takeStatus: takeStatus,
      slotCreated: slotCreated,
      selected: selected,
      asset: asset,
      ogg: ogg,
      assetDeduplicated: assetDeduplicated,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3VoiceBuildStatus.blocked,
        _ => throw const FormatException(
          'authoring revision-3 Voice response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3VoiceRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 Voice response grants unsupported runtime authority',
        ),
      },
      targetAuthority: switch (json['target_authority']) {
        'not_granted' => AuthoringRevision3VoiceTargetAuthority.notGranted,
        _ => throw const FormatException(
          'authoring revision-3 Voice response grants unsupported target authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3VoiceNativePublicationStatus.notSupported,
        _ => throw const FormatException(
          'authoring revision-3 Voice response grants unsupported native publication authority',
        ),
      },
    );
  }
}

String _authoringRevision3VoiceString(
  Map<String, Object?> json,
  String field, {
  required int maxBytes,
  bool allowEmpty = false,
}) {
  final value = json[field];
  if (value is! String || (!allowEmpty && value.isEmpty)) {
    throw FormatException(
      'authoring revision-3 Voice field $field is not a string',
    );
  }
  try {
    _authoringDraftRequestString(value, field, maxBytes);
  } on ArgumentError {
    throw FormatException(
      'authoring revision-3 Voice field $field is not bounded UTF-8',
    );
  }
  return value;
}

bool _authoringRevision3VoiceControl(int rune) =>
    rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);

bool _authoringRevision3VoiceLogicalNameIsSafe(String value) {
  if (value.trim() != value ||
      value.length <= 4 ||
      value.substring(value.length - 4).toLowerCase() != '.ogg') {
    return false;
  }
  const forbidden = <int>{
    0x22, // "
    0x2a, // *
    0x2f, // /
    0x3a, // :
    0x3c, // <
    0x3e, // >
    0x3f, // ?
    0x5c, // backslash
    0x7c, // |
  };
  if (value.runes.any(
    (rune) => _authoringRevision3VoiceControl(rune) || forbidden.contains(rune),
  )) {
    return false;
  }
  final stem = value.substring(0, value.length - 4);
  if (stem.isEmpty || stem == '.' || stem == '..') {
    return false;
  }
  final deviceStem = stem.split('.').first.toUpperCase();
  if (const {'CON', 'PRN', 'AUX', 'NUL'}.contains(deviceStem)) {
    return false;
  }
  return !RegExp(r'^(?:COM|LPT)[1-9]$').hasMatch(deviceStem);
}

String _authoringRevision3VoiceEntityId(
  Map<String, Object?> json,
  String field,
) {
  final id = _authoringEntityId(
    _authoringRevision3VoiceString(json, field, maxBytes: 32),
    field,
  );
  if (id == '00000000000000000000000000000000') {
    throw FormatException(
      'authoring revision-3 Voice field $field must not be zero',
    );
  }
  return id;
}

void _authoringRevision3VoiceRequireFieldOrder(
  Map<String, Object?> json,
  List<String> expected,
  String context,
) {
  final actual = json.keys.toList(growable: false);
  if (actual.length != expected.length) {
    throw FormatException(
      'authoring revision-3 Voice $context has an invalid field count',
    );
  }
  for (var index = 0; index < expected.length; index++) {
    if (actual[index] != expected[index]) {
      throw FormatException(
        'authoring revision-3 Voice $context has non-canonical field order',
      );
    }
  }
}

Map<String, Object?> _authoringRevision3VoiceGeneration(
  Object? value,
  String context,
) {
  final generation = _authoringRequiredObject(
    value,
    'revision-3 Voice $context generation',
  );
  _authoringExactFields(generation, const {
    'executable',
  }, 'revision-3 Voice $context generation');
  final executable = _authoringRequiredObject(
    generation['executable'],
    'revision-3 Voice $context executable seal',
  );
  _authoringExactFields(executable, const {
    'byte_len',
    'sha256',
  }, 'revision-3 Voice $context executable seal');
  _authoringRequiredInt(
    executable,
    'byte_len',
    min: 1,
    max: _maxAuthoringRevision3VoiceAppliedRevision,
  );
  final sha = _authoringRevision3VoiceString(
    executable,
    'sha256',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(sha) ||
      sha == _authoringRevision3VoiceZeroSha256) {
    throw FormatException(
      'authoring revision-3 Voice $context executable seal is invalid',
    );
  }
  return generation;
}

String _authoringRevision3VoiceLocale(String value) {
  if (value.length > 35 || value.codeUnits.any((unit) => unit > 0x7f)) {
    throw const FormatException(
      'authoring revision-3 Voice locale is not canonical ASCII',
    );
  }
  final segments = value.split('-');
  final language = segments.first;
  final lower = RegExp(r'^[a-z]{2,8}$');
  final alpha = RegExp(r'^[A-Za-z]+$');
  final alphaNumeric = RegExp(r'^[A-Za-z0-9]{1,8}$');
  if (!lower.hasMatch(language)) {
    throw const FormatException(
      'authoring revision-3 Voice locale language is invalid',
    );
  }
  final canonical = StringBuffer(language);
  for (var index = 1; index < segments.length; index++) {
    final segment = segments[index];
    if (!alphaNumeric.hasMatch(segment)) {
      throw const FormatException(
        'authoring revision-3 Voice locale segment is invalid',
      );
    }
    canonical.write('-');
    if (segment.length == 4 && alpha.hasMatch(segment)) {
      canonical.write(
        '${segment[0].toUpperCase()}${segment.substring(1).toLowerCase()}',
      );
    } else if (segment.length == 2 && alpha.hasMatch(segment)) {
      canonical.write(segment.toUpperCase());
    } else {
      canonical.write(segment.toLowerCase());
    }
  }
  if (canonical.toString() != value) {
    throw const FormatException(
      'authoring revision-3 Voice locale has non-canonical casing',
    );
  }
  return value;
}

AuthoringRevision3VoiceTakeStatus _authoringRevision3VoiceStatus(
  Object? value,
) => switch (value) {
  'draft' => AuthoringRevision3VoiceTakeStatus.draft,
  'recorded' => AuthoringRevision3VoiceTakeStatus.recorded,
  'reviewed' => AuthoringRevision3VoiceTakeStatus.reviewed,
  'approved' => AuthoringRevision3VoiceTakeStatus.approved,
  _ => throw const FormatException(
    'authoring revision-3 Voice take status is unsupported',
  ),
};

AuthoringRevision3VoiceAsset _authoringRevision3VoiceAsset(
  Object? value, {
  required String logicalName,
}) {
  final asset = _authoringRequiredObject(
    value,
    'revision-3 Voice response asset',
  );
  _authoringExactFields(asset, const {
    'sha256',
    'byte_len',
    'logical_name',
  }, 'revision-3 Voice response asset');
  final sha256 = _authoringRevision3VoiceString(asset, 'sha256', maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(sha256) ||
      sha256 == _authoringRevision3VoiceZeroSha256) {
    throw const FormatException(
      'authoring revision-3 Voice response asset seal is invalid',
    );
  }
  final returnedLogicalName = _authoringRevision3VoiceString(
    asset,
    'logical_name',
    maxBytes: _maxAuthoringRevision3VoiceLogicalNameBytes,
  );
  if (returnedLogicalName != logicalName) {
    throw const FormatException(
      'authoring revision-3 Voice response asset logical name disagrees',
    );
  }
  return AuthoringRevision3VoiceAsset._(
    sha256: sha256,
    byteLength: _authoringRequiredInt(
      asset,
      'byte_len',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    ),
    logicalName: returnedLogicalName,
  );
}

AuthoringRevision3VoiceOggMetadata _authoringRevision3VoiceOgg(Object? value) {
  final ogg = _authoringRequiredObject(
    value,
    'revision-3 Voice response Ogg metadata',
  );
  _authoringExactFields(ogg, const {
    'codec',
    'channels',
    'sample_rate',
    'pages',
    'logical_streams',
  }, 'revision-3 Voice response Ogg metadata');
  return AuthoringRevision3VoiceOggMetadata._(
    codec: switch (ogg['codec']) {
      'vorbis' => AuthoringRevision3VoiceOggCodec.vorbis,
      'opus' => AuthoringRevision3VoiceOggCodec.opus,
      _ => throw const FormatException(
        'authoring revision-3 Voice response Ogg codec is unsupported',
      ),
    },
    channels: _authoringRequiredInt(ogg, 'channels', min: 1, max: 0xff),
    sampleRate: _authoringRequiredInt(
      ogg,
      'sample_rate',
      min: 1,
      max: 0xffffffff,
    ),
    pages: _authoringRequiredInt(ogg, 'pages', min: 1, max: 0xffffffff),
    logicalStreams: _authoringRequiredInt(
      ogg,
      'logical_streams',
      min: 1,
      max: 0xffffffff,
    ),
  );
}

void _authoringRevision3VoiceRequireExactCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3VoiceTakeRequestV1 request,
  required String responseLocalizationId,
  required bool responseSlotCreated,
  required AuthoringRevision3VoiceAsset asset,
  required AuthoringRevision3VoiceOggMetadata ogg,
}) {
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 Voice basis entities',
  );
  final line = _authoringRevision3VoiceEntity(
    baseEntities,
    request.lineId,
    'dialog_line',
    'basis line',
  );
  _authoringRevision3VoiceExactOptionalFields(
    line.data,
    const {'localization', 'voice_slots'},
    const {'speaker_hint'},
    'basis DialogLine data',
  );
  final localizationRef = _authoringRevision3VoiceTypedRef(
    line.data['localization'],
    projectId: request.expectedProjectId,
    kind: 'localization_entry',
    context: 'line localization',
  );
  if (localizationRef.id == request.lineId ||
      localizationRef.id != responseLocalizationId) {
    throw const FormatException(
      'authoring revision-3 Voice localization identity disagrees',
    );
  }
  final localization = _authoringRevision3VoiceEntity(
    baseEntities,
    localizationRef.id,
    'localization_entry',
    'basis localization',
  );
  _authoringExactFields(localization.data, const {
    'loc_id',
    'texts',
  }, 'revision-3 Voice basis LocalizationEntry data');
  final locId = _authoringRevision3VoiceString(
    localization.data,
    'loc_id',
    maxBytes: 1024,
  );
  if (locId.runes.any(_authoringRevision3VoiceControl)) {
    throw const FormatException(
      'authoring revision-3 Voice basis localization ID is invalid',
    );
  }
  final voiceSlots = _authoringRequiredObject(
    line.data['voice_slots'],
    'revision-3 Voice basis line slots',
  );
  final existingSlotValue = voiceSlots[request.locale];
  final slotCreated = existingSlotValue == null;
  if (slotCreated != responseSlotCreated) {
    throw const FormatException(
      'authoring revision-3 Voice slot creation status disagrees with its basis',
    );
  }
  if (baseEntities.containsKey(request.takeId) ||
      (slotCreated && baseEntities.containsKey(request.slotId))) {
    throw const FormatException(
      'authoring revision-3 Voice request entity IDs collide with its basis',
    );
  }
  if (!slotCreated) {
    final slotRef = _authoringRevision3VoiceTypedRef(
      existingSlotValue,
      projectId: request.expectedProjectId,
      kind: 'voice_slot',
      context: 'existing line slot',
    );
    if (slotRef.id != request.slotId) {
      throw const FormatException(
        'authoring revision-3 Voice request slot differs from its basis line',
      );
    }
    _authoringRevision3VoiceValidateExistingSlot(
      baseEntities,
      projectId: request.expectedProjectId,
      lineId: request.lineId,
      slotId: request.slotId,
      locale: request.locale,
    );
  }

  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 Voice expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final locales = _authoringRevision3VoiceStringList(
    expected['authoring_locales'],
    'expected authoring locales',
  );
  if (!locales.contains(request.locale)) locales.add(request.locale);
  locales.sort();
  expected['authoring_locales'] = locales;
  final expectedEntities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 Voice expected entities',
  );

  final expectedLocalization = _authoringRequiredObject(
    expectedEntities[localizationRef.id],
    'revision-3 Voice expected localization',
  );
  final expectedLocalizationPayload = _authoringRequiredObject(
    expectedLocalization['payload'],
    'revision-3 Voice expected localization payload',
  );
  final expectedLocalizationData = _authoringRequiredObject(
    expectedLocalizationPayload['data'],
    'revision-3 Voice expected localization data',
  );
  final expectedTexts = _authoringRequiredObject(
    expectedLocalizationData['texts'],
    'revision-3 Voice expected localization texts',
  );
  if (request.text != null && expectedTexts[request.locale] != request.text) {
    expectedLocalization['revision'] =
        _authoringRevision3VoiceIncrementRevision(expectedLocalization);
    expectedTexts[request.locale] = request.text;
  }
  expectedLocalizationData['texts'] = expectedTexts;
  expectedLocalizationPayload['data'] = expectedLocalizationData;
  expectedLocalization['payload'] = expectedLocalizationPayload;
  expectedEntities[localizationRef.id] = expectedLocalization;

  final takeRef = <String, Object?>{
    'project_id': request.expectedProjectId,
    'id': request.takeId,
    'expected_kind': 'voice_take',
  };
  if (slotCreated) {
    final expectedLine = _authoringRequiredObject(
      expectedEntities[request.lineId],
      'revision-3 Voice expected line',
    );
    final expectedLinePayload = _authoringRequiredObject(
      expectedLine['payload'],
      'revision-3 Voice expected line payload',
    );
    final expectedLineData = _authoringRequiredObject(
      expectedLinePayload['data'],
      'revision-3 Voice expected line data',
    );
    final expectedSlots = _authoringRequiredObject(
      expectedLineData['voice_slots'],
      'revision-3 Voice expected line slots',
    );
    expectedSlots[request.locale] = <String, Object?>{
      'project_id': request.expectedProjectId,
      'id': request.slotId,
      'expected_kind': 'voice_slot',
    };
    expectedLineData['voice_slots'] = expectedSlots;
    expectedLinePayload['data'] = expectedLineData;
    expectedLine['payload'] = expectedLinePayload;
    expectedLine['revision'] = _authoringRevision3VoiceIncrementRevision(
      expectedLine,
    );
    expectedEntities[request.lineId] = expectedLine;
    expectedEntities[request.slotId] = <String, Object?>{
      'id': request.slotId,
      'display_name': 'Voice ${request.locale}',
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': _authoringRevision3VoiceSlotGeneratorId,
        'generator_version': _authoringRevision3VoiceSlotGeneratorVersion,
        'owner': <String, Object?>{
          'project_id': request.expectedProjectId,
          'id': request.lineId,
          'expected_kind': 'dialog_line',
        },
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': request.locale,
          'target_resolution': <String, Object?>{'state': 'unresolved'},
          'candidates': <Object?>[takeRef],
          if (request.selectTake) 'selected': takeRef,
        },
      },
    };
  } else {
    final expectedSlot = _authoringRequiredObject(
      expectedEntities[request.slotId],
      'revision-3 Voice expected existing slot',
    );
    final expectedSlotPayload = _authoringRequiredObject(
      expectedSlot['payload'],
      'revision-3 Voice expected existing slot payload',
    );
    final expectedSlotData = _authoringRequiredObject(
      expectedSlotPayload['data'],
      'revision-3 Voice expected existing slot data',
    );
    final candidates = _authoringRevision3VoiceObjectList(
      expectedSlotData['candidates'],
      'expected existing slot candidates',
    )..add(takeRef);
    expectedSlotData['candidates'] = candidates;
    if (request.selectTake) expectedSlotData['selected'] = takeRef;
    expectedSlotPayload['data'] = expectedSlotData;
    expectedSlot['payload'] = expectedSlotPayload;
    expectedSlot['revision'] = _authoringRevision3VoiceIncrementRevision(
      expectedSlot,
    );
    expectedEntities[request.slotId] = expectedSlot;
  }
  expectedEntities[request.takeId] = <String, Object?>{
    'id': request.takeId,
    'display_name': request.takeDisplayName,
    'origin': <String, Object?>{
      'type': 'imported',
      'importer': _authoringRevision3VoiceTakeImporterId,
      'source_seal': <String, Object?>{
        'byte_len': asset.byteLength,
        'sha256': asset.sha256,
      },
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'voice_take',
      'data': <String, Object?>{
        'locale': request.locale,
        'asset': <String, Object?>{
          'sha256': asset.sha256,
          'byte_len': asset.byteLength,
          'logical_name': asset.logicalName,
        },
        'ogg': <String, Object?>{
          'codec': switch (ogg.codec) {
            AuthoringRevision3VoiceOggCodec.vorbis => 'vorbis',
            AuthoringRevision3VoiceOggCodec.opus => 'opus',
          },
          'channels': ogg.channels,
          'sample_rate': ogg.sampleRate,
          'pages': ogg.pages,
          'logical_streams': ogg.logicalStreams,
        },
        'status': request.status.wireName,
      },
    },
  };
  expected['entities'] = expectedEntities;

  final expectedAssetStore = _authoringRequiredObject(
    expected['asset_store'],
    'revision-3 Voice expected asset store',
  );
  _authoringExactFields(expectedAssetStore, const {
    'assets',
  }, 'revision-3 Voice expected asset store');
  final expectedAssets = _authoringRequiredObject(
    expectedAssetStore['assets'],
    'revision-3 Voice expected assets',
  );
  final existingAsset = expectedAssets[asset.sha256];
  final expectedAssetMeta = <String, Object?>{
    'byte_len': asset.byteLength,
    'media_type': 'audio/ogg',
  };
  if (existingAsset != null &&
      !_authoringRevision3VoiceDeepEqual(existingAsset, expectedAssetMeta)) {
    throw const FormatException(
      'authoring revision-3 Voice asset metadata conflicts with its basis',
    );
  }
  expectedAssets[asset.sha256] = expectedAssetMeta;
  expectedAssetStore['assets'] = expectedAssets;
  expected['asset_store'] = expectedAssetStore;

  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'authoring revision-3 Voice candidate contains a non-exact project delta',
    );
  }
}

void _authoringRevision3VoiceValidateExistingSlot(
  Map<String, Object?> entities, {
  required String projectId,
  required String lineId,
  required String slotId,
  required String locale,
}) {
  final slot = _authoringRevision3VoiceEntity(
    entities,
    slotId,
    'voice_slot',
    'basis existing slot',
  );
  _authoringRevision3VoiceExactOptionalFields(
    slot.data,
    const {'locale', 'target_resolution', 'candidates'},
    const {'selected'},
    'basis existing VoiceSlot data',
  );
  if (slot.data['locale'] != locale) {
    throw const FormatException(
      'authoring revision-3 Voice existing slot locale disagrees',
    );
  }
  final resolution = _authoringRequiredObject(
    slot.data['target_resolution'],
    'revision-3 Voice existing slot target resolution',
  );
  if (!_authoringRevision3VoiceDeepEqual(resolution, const <String, Object?>{
    'state': 'unresolved',
  })) {
    throw const FormatException(
      'authoring revision-3 Voice existing slot lacks sealed target authority',
    );
  }
  final candidates = _authoringRevision3VoiceObjectList(
    slot.data['candidates'],
    'existing slot candidates',
  );
  if (candidates.length >= _maxAuthoringRevision3VoiceSlotCandidates) {
    throw const FormatException(
      'authoring revision-3 Voice existing slot candidate limit is exhausted',
    );
  }
  final candidateIds = <String>{};
  for (final candidate in candidates) {
    final ref = _authoringRevision3VoiceTypedRef(
      candidate,
      projectId: projectId,
      kind: 'voice_take',
      context: 'existing slot candidate',
    );
    if (!candidateIds.add(ref.id)) {
      throw const FormatException(
        'authoring revision-3 Voice existing slot candidates are duplicated',
      );
    }
    final take = _authoringRevision3VoiceEntity(
      entities,
      ref.id,
      'voice_take',
      'basis existing take',
    );
    _authoringExactFields(take.data, const {
      'locale',
      'asset',
      'ogg',
      'status',
    }, 'revision-3 Voice basis existing take data');
    if (take.data['locale'] != locale) {
      throw const FormatException(
        'authoring revision-3 Voice existing take locale disagrees',
      );
    }
  }
  final selected = slot.data['selected'];
  if (selected != null) {
    final ref = _authoringRevision3VoiceTypedRef(
      selected,
      projectId: projectId,
      kind: 'voice_take',
      context: 'existing slot selected take',
    );
    if (!candidateIds.contains(ref.id)) {
      throw const FormatException(
        'authoring revision-3 Voice selected take is not a candidate',
      );
    }
    final take = _authoringRevision3VoiceEntity(
      entities,
      ref.id,
      'voice_take',
      'basis selected take',
    );
    if (take.data['status'] != 'approved') {
      throw const FormatException(
        'authoring revision-3 Voice selected take is not approved',
      );
    }
  }
  for (final entry in entities.entries) {
    final entity = _authoringRequiredObject(
      entry.value,
      'revision-3 Voice basis entity',
    );
    final payload = _authoringRequiredObject(
      entity['payload'],
      'revision-3 Voice basis payload',
    );
    if (payload['kind'] != 'dialog_line') continue;
    final data = _authoringRequiredObject(
      payload['data'],
      'revision-3 Voice basis DialogLine data',
    );
    final slots = _authoringRequiredObject(
      data['voice_slots'],
      'revision-3 Voice basis DialogLine slots',
    );
    for (final owned in slots.entries) {
      final ref = _authoringRevision3VoiceTypedRef(
        owned.value,
        projectId: projectId,
        kind: 'voice_slot',
        context: 'basis slot ownership',
      );
      if (ref.id == slotId && (entry.key != lineId || owned.key != locale)) {
        throw const FormatException(
          'authoring revision-3 Voice existing slot is shared',
        );
      }
    }
  }
}

({Map<String, Object?> entity, Map<String, Object?> data})
_authoringRevision3VoiceEntity(
  Map<String, Object?> entities,
  String id,
  String kind,
  String context,
) {
  final entity = _authoringRequiredObject(
    entities[id],
    'revision-3 Voice $context entity',
  );
  _authoringExactFields(entity, const {
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, 'revision-3 Voice $context entity');
  if (entity['id'] != id) {
    throw FormatException('authoring revision-3 Voice $context ID disagrees');
  }
  _authoringRevision3VoiceRevision(entity);
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 Voice $context payload',
  );
  _authoringExactFields(payload, const {
    'kind',
    'data',
  }, 'revision-3 Voice $context payload');
  if (payload['kind'] != kind) {
    throw FormatException('authoring revision-3 Voice $context kind disagrees');
  }
  return (
    entity: entity,
    data: _authoringRequiredObject(
      payload['data'],
      'revision-3 Voice $context data',
    ),
  );
}

int _authoringRevision3VoiceRevision(Map<String, Object?> entity) =>
    _authoringRequiredInt(
      entity,
      'revision',
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );

int _authoringRevision3VoiceIncrementRevision(Map<String, Object?> entity) {
  final revision = _authoringRevision3VoiceRevision(entity);
  if (revision >= _maxAuthoringRevision3VoiceAppliedRevision) {
    throw const FormatException(
      'authoring revision-3 Voice entity revision cannot be incremented',
    );
  }
  return revision + 1;
}

({String id}) _authoringRevision3VoiceTypedRef(
  Object? value, {
  required String projectId,
  required String kind,
  required String context,
}) {
  final ref = _authoringRequiredObject(
    value,
    'revision-3 Voice $context reference',
  );
  _authoringExactFields(ref, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'revision-3 Voice $context reference');
  final id = _authoringRevision3VoiceEntityId(ref, 'id');
  if (ref['project_id'] != projectId || ref['expected_kind'] != kind) {
    throw FormatException(
      'authoring revision-3 Voice $context reference is not exact',
    );
  }
  return (id: id);
}

void _authoringRevision3VoiceExactOptionalFields(
  Map<String, Object?> object,
  Set<String> required,
  Set<String> optional,
  String context,
) {
  if (!required.every(object.containsKey) ||
      object.keys.any(
        (key) => !required.contains(key) && !optional.contains(key),
      )) {
    throw FormatException(
      'authoring revision-3 Voice $context has an invalid schema',
    );
  }
}

List<String> _authoringRevision3VoiceStringList(Object? value, String context) {
  if (value is! List) {
    throw FormatException('authoring revision-3 Voice $context is not a list');
  }
  final output = <String>[];
  for (final item in value) {
    if (item is! String) {
      throw FormatException(
        'authoring revision-3 Voice $context contains a non-string',
      );
    }
    output.add(_authoringRevision3VoiceLocale(item));
  }
  for (var index = 1; index < output.length; index++) {
    if (output[index - 1].compareTo(output[index]) >= 0) {
      throw FormatException(
        'authoring revision-3 Voice $context is not sorted and unique',
      );
    }
  }
  return output;
}

List<Map<String, Object?>> _authoringRevision3VoiceObjectList(
  Object? value,
  String context,
) {
  if (value is! List) {
    throw FormatException('authoring revision-3 Voice $context is not a list');
  }
  return value
      .map(
        (item) =>
            _authoringRequiredObject(item, 'revision-3 Voice $context item'),
      )
      .toList();
}

Map<String, Object?> _authoringRevision3VoiceCloneObject(
  Map<String, Object?> value,
  String context,
) => _authoringRequiredObject(jsonDecode(jsonEncode(value)), context);

bool _authoringRevision3VoiceDeepEqual(Object? left, Object? right) {
  if (left is Map && right is Map) {
    if (left.length != right.length) return false;
    for (final entry in left.entries) {
      if (!right.containsKey(entry.key) ||
          !_authoringRevision3VoiceDeepEqual(entry.value, right[entry.key])) {
        return false;
      }
    }
    return true;
  }
  if (left is List && right is List) {
    if (left.length != right.length) return false;
    for (var index = 0; index < left.length; index++) {
      if (!_authoringRevision3VoiceDeepEqual(left[index], right[index])) {
        return false;
      }
    }
    return true;
  }
  return left == right;
}

void _authoringRevision3VoicePrepareEnvelopePreflight(
  String command,
  String currentProjectJson,
  String gameRoot,
  String root,
  String source,
  String voiceRequestJson,
) {
  var encodedBytes =
      '{"command":"","payload":{"current_project_json":"","game_root":"","root":"","source":"","voice_request_json":""}}'
          .length +
      command.length;
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    currentProjectJson,
    'currentProjectJson',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    gameRoot,
    'gameRoot',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    root,
    'root',
    encodedBytes,
  );
  encodedBytes = _authoringAddEscapedJsonStringBytes(
    source,
    'source',
    encodedBytes,
  );
  _authoringAddEscapedJsonStringBytes(
    voiceRequestJson,
    'voiceRequestJson',
    encodedBytes,
  );
}
