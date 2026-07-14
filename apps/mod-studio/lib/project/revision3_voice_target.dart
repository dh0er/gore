part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceTargetRequestBytes = 64 * 1024;
// Native requires the ASCII LocID plus the literal `.ogg` suffix to fit the
// 1024-byte portable member-name bound.
const _maxAuthoringRevision3VoiceTargetLocIdBytes = 1020;
const _maxAuthoringRevision3VoiceTargetArchiveBytes = 255;
const _maxAuthoringRevision3VoiceTargetMemberBytes = 1024;
const _maxAuthoringRevision3VoiceTargetMatches = 512;

enum AuthoringRevision3VoiceTargetResolutionState {
  unresolved,
  ambiguous,
  resolved,
}

final class AuthoringRevision3VoiceTargetRequestV1 {
  const AuthoringRevision3VoiceTargetRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.lineId,
    required this.slotId,
    required this.locale,
    required this.expectedLocId,
  });

  factory AuthoringRevision3VoiceTargetRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String lineId,
    required String slotId,
    required String locale,
    required String expectedLocId,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3VoiceTargetRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'line_id': lineId,
        'slot_id': slotId,
        'locale': locale,
        'expected_loc_id': expectedLocId,
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
  final String locale;
  final String expectedLocId;

  factory AuthoringRevision3VoiceTargetRequestV1.fromCanonicalJson(
    String value,
  ) {
    try {
      _authoringRevision3RequestString(
        value,
        'voiceTargetRequestJson',
        _maxAuthoringRevision3VoiceTargetRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 Voice target request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Voice target request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'slot_id',
      'locale',
      'expected_loc_id',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Voice target request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'target request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 Voice target request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Voice target expected head',
        ),
      ),
    );
    final projectId = _authoringRevision3VoiceEntityId(
      request,
      'expected_project_id',
    );
    final lineId = _authoringRevision3VoiceEntityId(request, 'line_id');
    final slotId = _authoringRevision3VoiceEntityId(request, 'slot_id');
    if (lineId == slotId) {
      throw const FormatException(
        'revision-3 Voice target line and slot IDs must differ',
      );
    }
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(request, 'locale', maxBytes: 35),
    );
    final locId = _authoringRevision3VoiceString(
      request,
      'expected_loc_id',
      maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
    );
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(locId)) {
      throw const FormatException(
        'revision-3 Voice target LocID is not one safe archive basename stem',
      );
    }
    final target = _authoringRevision3VoiceGeneration(
      request['expected_target'],
      'target request generation',
    );
    return AuthoringRevision3VoiceTargetRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: projectId,
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(target),
      lineId: lineId,
      slotId: slotId,
      locale: locale,
      expectedLocId: locId,
    );
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'revision-3 Voice target request does not bind the exact current project',
      );
    }
  }
}

final class AuthoringRevision3VoiceTarget {
  const AuthoringRevision3VoiceTarget._({
    required this.archive,
    required this.member,
    required this.archiveByteLength,
    required this.archiveSha256,
    required this.memberUncompressedSize,
    required this.memberCrc32,
  });

  final String archive;
  final String member;
  final int archiveByteLength;
  final String archiveSha256;
  final int memberUncompressedSize;
  final int memberCrc32;

  String get deploymentKey =>
      '${archive.toLowerCase()}|${member.toLowerCase()}';

  factory AuthoringRevision3VoiceTarget._fromJson(
    Object? value, {
    required String locId,
    required String context,
  }) {
    final target = _authoringRequiredObject(value, context);
    _authoringExactFields(target, const <String>{
      'archive',
      'member',
      'operation',
      'archive_seal',
      'member_proof',
    }, context);
    final archive = _authoringRequiredString(
      target,
      'archive',
      maxBytes: _maxAuthoringRevision3VoiceTargetArchiveBytes,
    );
    final member = _authoringRequiredString(
      target,
      'member',
      maxBytes: _maxAuthoringRevision3VoiceTargetMemberBytes,
    );
    final expectedMemberBasename = '$locId.ogg';
    if (target['operation'] != 'replace' ||
        !_authoringRevision3VoiceTargetArchiveIsSafe(archive) ||
        !_authoringRevision3VoiceTargetMemberIsSafe(member) ||
        !_authoringRevision3VoiceTargetAsciiEqualsIgnoreCase(
          member.split('/').last,
          expectedMemberBasename,
        )) {
      throw FormatException('$context is not one safe existing-member target');
    }
    final archiveSeal = _authoringRequiredObject(
      target['archive_seal'],
      '$context archive seal',
    );
    _authoringExactFields(archiveSeal, const <String>{
      'byte_len',
      'sha256',
    }, '$context archive seal');
    final archiveByteLength = _authoringRequiredInt(
      archiveSeal,
      'byte_len',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final archiveSha256 = _authoringRequiredString(
      archiveSeal,
      'sha256',
      maxBytes: 64,
    );
    final proof = _authoringRequiredObject(
      target['member_proof'],
      '$context member proof',
    );
    _authoringExactFields(proof, const <String>{
      'state',
      'uncompressed_size',
      'crc32',
    }, '$context member proof');
    final memberUncompressedSize = _authoringRequiredInt(
      proof,
      'uncompressed_size',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final memberCrc32 = _authoringRequiredInt(proof, 'crc32', max: 0xffffffff);
    if (proof['state'] != 'present' ||
        !_authoringSha256Pattern.hasMatch(archiveSha256)) {
      throw FormatException('$context has invalid sealed member evidence');
    }
    return AuthoringRevision3VoiceTarget._(
      archive: archive,
      member: member,
      archiveByteLength: archiveByteLength,
      archiveSha256: archiveSha256,
      memberUncompressedSize: memberUncompressedSize,
      memberCrc32: memberCrc32,
    );
  }
}

final class AuthoringRevision3VoiceArchiveObservation {
  const AuthoringRevision3VoiceArchiveObservation._({
    required this.archive,
    required this.byteLength,
    required this.sha256,
  });

  final String archive;
  final int byteLength;
  final String sha256;

  factory AuthoringRevision3VoiceArchiveObservation._fromJson(Object? value) {
    final observation = _authoringRequiredObject(
      value,
      'revision-3 Voice target archive observation',
    );
    _authoringExactFields(observation, const <String>{
      'archive',
      'archive_seal',
    }, 'revision-3 Voice target archive observation');
    final archive = _authoringRequiredString(
      observation,
      'archive',
      maxBytes: _maxAuthoringRevision3VoiceTargetArchiveBytes,
    );
    final seal = _authoringRequiredObject(
      observation['archive_seal'],
      'revision-3 Voice target archive observation seal',
    );
    _authoringExactFields(seal, const <String>{
      'byte_len',
      'sha256',
    }, 'revision-3 Voice target archive observation seal');
    final sha256 = _authoringRequiredString(seal, 'sha256', maxBytes: 64);
    if (!_authoringRevision3VoiceTargetArchiveIsSafe(archive) ||
        !_authoringSha256Pattern.hasMatch(sha256)) {
      throw const FormatException(
        'revision-3 Voice target archive observation is invalid',
      );
    }
    return AuthoringRevision3VoiceArchiveObservation._(
      archive: archive,
      byteLength: _authoringRequiredInt(
        seal,
        'byte_len',
        min: 1,
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      sha256: sha256,
    );
  }
}

final class AuthoringRevision3VoiceTargetPreparation {
  AuthoringRevision3VoiceTargetPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.locale,
    required this.locId,
    required this.resolution,
    required List<AuthoringRevision3VoiceTarget> targets,
    required this.archiveObservation,
  }) : targets = List.unmodifiable(targets);

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String lineId;
  final String localizationId;
  final String slotId;
  final String locale;
  final String locId;
  final AuthoringRevision3VoiceTargetResolutionState resolution;
  final List<AuthoringRevision3VoiceTarget> targets;
  final AuthoringRevision3VoiceArchiveObservation? archiveObservation;

  int get matchCount => targets.length;
  AuthoringRevision3VoiceTarget? get resolvedTarget =>
      resolution == AuthoringRevision3VoiceTargetResolutionState.resolved
      ? targets.single
      : null;

  factory AuthoringRevision3VoiceTargetPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3VoiceTargetRequestV1 request,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'revision',
      'line_id',
      'localization_id',
      'slot_id',
      'locale',
      'loc_id',
      'resolution',
      'match_count',
      'target_resolution',
      'archive_observation',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Voice target preparation response');
    if (json['ok'] != true ||
        json['outcome'] != 'prepared_unpublished' ||
        json['build_status'] != 'blocked' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'revision-3 Voice target response escalates unpublished authority',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson ||
        head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'revision-3 Voice target response has an invalid head transition',
      );
    }
    final projectJson = _authoringRequiredString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(current);
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final lineId = _authoringEntityId(
      _authoringRequiredString(json, 'line_id', maxBytes: 32),
      'line_id',
    );
    final localizationId = _authoringEntityId(
      _authoringRequiredString(json, 'localization_id', maxBytes: 32),
      'localization_id',
    );
    final slotId = _authoringEntityId(
      _authoringRequiredString(json, 'slot_id', maxBytes: 32),
      'slot_id',
    );
    final locale = _authoringRevision3VoiceLocale(
      _authoringRequiredString(json, 'locale', maxBytes: 35),
    );
    final locId = _authoringRequiredString(
      json,
      'loc_id',
      maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
    );
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(locId) ||
        candidate.projectId != request.expectedProjectId ||
        candidate.revision != request.expectedRevision + 1 ||
        revision != candidate.revision ||
        lineId != request.lineId ||
        slotId != request.slotId ||
        locale != request.locale ||
        locId != request.expectedLocId) {
      throw const FormatException(
        'revision-3 Voice target response disagrees with its exact request',
      );
    }
    final parsedResolution = _authoringRevision3VoiceTargetResolution(
      json['target_resolution'],
      locId: locId,
      context: 'revision-3 Voice target response resolution',
    );
    final resolution = switch (json['resolution']) {
      'unresolved' => AuthoringRevision3VoiceTargetResolutionState.unresolved,
      'ambiguous' => AuthoringRevision3VoiceTargetResolutionState.ambiguous,
      'resolved' => AuthoringRevision3VoiceTargetResolutionState.resolved,
      _ => throw const FormatException(
        'revision-3 Voice target response has an unknown resolution',
      ),
    };
    final matchCount = _authoringRequiredInt(
      json,
      'match_count',
      max: _maxAuthoringRevision3VoiceTargetMatches,
    );
    if (resolution != parsedResolution.state ||
        matchCount != parsedResolution.targets.length) {
      throw const FormatException(
        'revision-3 Voice target response resolution cardinality disagrees',
      );
    }
    final archiveObservation = json['archive_observation'] == null
        ? null
        : AuthoringRevision3VoiceArchiveObservation._fromJson(
            json['archive_observation'],
          );
    if (archiveObservation == null && matchCount != 0) {
      throw const FormatException(
        'revision-3 Voice target matches lack an archive observation',
      );
    }
    for (final target in parsedResolution.targets) {
      if (archiveObservation == null ||
          target.archive != archiveObservation.archive ||
          target.archiveByteLength != archiveObservation.byteLength ||
          target.archiveSha256 != archiveObservation.sha256) {
        throw const FormatException(
          'revision-3 Voice target evidence disagrees with its archive observation',
        );
      }
    }
    _authoringRevision3VoiceRequireExactTargetCandidate(
      current.project,
      candidate.project,
      request: request,
      responseLocalizationId: localizationId,
      targetResolution: _authoringRequiredObject(
        json['target_resolution'],
        'revision-3 Voice target response resolution',
      ),
    );
    return AuthoringRevision3VoiceTargetPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: candidate.projectId,
      revision: revision,
      lineId: lineId,
      localizationId: localizationId,
      slotId: slotId,
      locale: locale,
      locId: locId,
      resolution: resolution,
      targets: parsedResolution.targets,
      archiveObservation: archiveObservation,
    );
  }
}

({
  AuthoringRevision3VoiceTargetResolutionState state,
  List<AuthoringRevision3VoiceTarget> targets,
})
_authoringRevision3VoiceTargetResolution(
  Object? value, {
  required String locId,
  required String context,
}) {
  final resolution = _authoringRequiredObject(value, context);
  final state = resolution['state'];
  final targets = <AuthoringRevision3VoiceTarget>[];
  switch (state) {
    case 'unresolved':
      _authoringExactFields(resolution, const {'state'}, context);
    case 'resolved':
      _authoringExactFields(resolution, const {'state', 'target'}, context);
      targets.add(
        AuthoringRevision3VoiceTarget._fromJson(
          resolution['target'],
          locId: locId,
          context: '$context target',
        ),
      );
    case 'ambiguous':
      _authoringExactFields(resolution, const {'state', 'candidates'}, context);
      final raw = resolution['candidates'];
      if (raw is! List ||
          raw.length < 2 ||
          raw.length > _maxAuthoringRevision3VoiceTargetMatches) {
        throw FormatException('$context has invalid ambiguous cardinality');
      }
      for (var index = 0; index < raw.length; index++) {
        targets.add(
          AuthoringRevision3VoiceTarget._fromJson(
            raw[index],
            locId: locId,
            context: '$context candidate $index',
          ),
        );
      }
    default:
      throw FormatException('$context has an unknown state');
  }
  final keys = <String>{};
  if (!targets.every((target) => keys.add(target.deploymentKey))) {
    throw FormatException('$context contains duplicate deployment targets');
  }
  return (
    state: switch (state) {
      'unresolved' => AuthoringRevision3VoiceTargetResolutionState.unresolved,
      'ambiguous' => AuthoringRevision3VoiceTargetResolutionState.ambiguous,
      'resolved' => AuthoringRevision3VoiceTargetResolutionState.resolved,
      _ => throw StateError('unreachable'),
    },
    targets: List.unmodifiable(targets),
  );
}

void _authoringRevision3VoiceRequireExactTargetCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3VoiceTargetRequestV1 request,
  required String responseLocalizationId,
  required Map<String, Object?> targetResolution,
}) {
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 Voice target basis entities',
  );
  final line = _authoringRevision3VoiceEntity(
    baseEntities,
    request.lineId,
    'dialog_line',
    'target basis line',
  );
  final localizationRef = _authoringRevision3VoiceTypedRef(
    line.data['localization'],
    projectId: request.expectedProjectId,
    kind: 'localization_entry',
    context: 'target line localization',
  );
  if (localizationRef.id != responseLocalizationId) {
    throw const FormatException(
      'revision-3 Voice target localization identity disagrees',
    );
  }
  final localization = _authoringRevision3VoiceEntity(
    baseEntities,
    localizationRef.id,
    'localization_entry',
    'target basis localization',
  );
  if (localization.data['loc_id'] != request.expectedLocId) {
    throw const FormatException(
      'revision-3 Voice target LocID disagrees with the exact line',
    );
  }
  final slots = _authoringRequiredObject(
    line.data['voice_slots'],
    'revision-3 Voice target basis line slots',
  );
  final slotRef = _authoringRevision3VoiceTypedRef(
    slots[request.locale],
    projectId: request.expectedProjectId,
    kind: 'voice_slot',
    context: 'target line slot',
  );
  if (slotRef.id != request.slotId) {
    throw const FormatException(
      'revision-3 Voice target slot differs from the exact line',
    );
  }
  _authoringRevision3VoiceValidateExistingSlot(
    baseEntities,
    projectId: request.expectedProjectId,
    lineId: request.lineId,
    slotId: request.slotId,
    locale: request.locale,
    locId: request.expectedLocId,
  );

  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 Voice target expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final expectedEntities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 Voice target expected entities',
  );
  final expectedSlot = _authoringRequiredObject(
    expectedEntities[request.slotId],
    'revision-3 Voice target expected slot',
  );
  final expectedPayload = _authoringRequiredObject(
    expectedSlot['payload'],
    'revision-3 Voice target expected slot payload',
  );
  final expectedData = _authoringRequiredObject(
    expectedPayload['data'],
    'revision-3 Voice target expected slot data',
  );
  expectedData['target_resolution'] = targetResolution;
  expectedPayload['data'] = expectedData;
  expectedSlot['payload'] = expectedPayload;
  expectedSlot['revision'] = _authoringRevision3VoiceIncrementRevision(
    expectedSlot,
  );
  expectedEntities[request.slotId] = expectedSlot;
  expected['entities'] = expectedEntities;
  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'revision-3 Voice target candidate contains a non-exact project delta',
    );
  }
}

/// Exact portable archive-basename stem accepted by the managed R3 Voice
/// target boundary.
///
/// The literal `.ogg` suffix is reserved by the native resolver, so the stem
/// is capped at 1020 ASCII bytes. Keeping this predicate public lets the
/// friendly catalog, technical plan, request, and response DTO share one
/// contract instead of admitting values that fail later at the FFI boundary.
bool authoringRevision3VoiceArchiveBasenameStemIsSafe(String value) {
  if (value.isEmpty ||
      value.length > _maxAuthoringRevision3VoiceTargetLocIdBytes ||
      value.trim() != value ||
      value == '.' ||
      value == '..' ||
      value.endsWith('.') ||
      value.endsWith(' ') ||
      value.contains('/') ||
      value.contains(r'\') ||
      value.codeUnits.any((unit) => unit > 0x7f) ||
      value.runes.any(_authoringRevision3VoiceControl) ||
      RegExp(r'[<>:"|?*]').hasMatch(value)) {
    return false;
  }
  return !_authoringRevision3VoiceTargetWindowsReservedName(value);
}

bool _authoringRevision3VoiceTargetArchiveIsSafe(String value) =>
    !value.contains('/') &&
    _authoringRevision3VoiceTargetPortablePathIsSafe(value) &&
    value.toLowerCase().endsWith('.zip');

bool _authoringRevision3VoiceTargetMemberIsSafe(String value) =>
    _authoringRevision3VoiceTargetPortablePathIsSafe(value) &&
    value.toLowerCase().endsWith('.ogg');

bool _authoringRevision3VoiceTargetAsciiEqualsIgnoreCase(
  String left,
  String right,
) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    final leftUnit = left.codeUnitAt(index);
    final rightUnit = right.codeUnitAt(index);
    if (leftUnit > 0x7f || rightUnit > 0x7f) return false;
    final foldedLeft = leftUnit >= 0x41 && leftUnit <= 0x5a
        ? leftUnit + 0x20
        : leftUnit;
    final foldedRight = rightUnit >= 0x41 && rightUnit <= 0x5a
        ? rightUnit + 0x20
        : rightUnit;
    if (foldedLeft != foldedRight) return false;
  }
  return true;
}

bool _authoringRevision3VoiceTargetPortablePathIsSafe(String value) {
  if (value.isEmpty ||
      utf8.encode(value).length >
          _maxAuthoringRevision3VoiceTargetMemberBytes ||
      value.startsWith('/') ||
      value.startsWith(r'\') ||
      value.contains(r'\') ||
      value.runes.any(_authoringRevision3VoiceControl)) {
    return false;
  }
  for (final segment in value.split('/')) {
    if (segment.isEmpty ||
        segment == '.' ||
        segment == '..' ||
        segment.contains(':') ||
        RegExp(r'[<>"|?*]').hasMatch(segment) ||
        segment.endsWith(' ') ||
        segment.endsWith('.') ||
        _authoringRevision3VoiceTargetWindowsReservedName(segment)) {
      return false;
    }
  }
  return true;
}

bool _authoringRevision3VoiceTargetWindowsReservedName(String segment) {
  final stem = segment.split('.').first.replaceFirst(RegExp(r'[ .]+$'), '');
  final folded = stem.toUpperCase();
  if (const {
    'CON',
    'PRN',
    'AUX',
    'NUL',
    r'CLOCK$',
    r'CONIN$',
    r'CONOUT$',
  }.contains(folded)) {
    return true;
  }
  return RegExp(r'^(COM|LPT)[1-9¹²³]$').hasMatch(folded);
}
