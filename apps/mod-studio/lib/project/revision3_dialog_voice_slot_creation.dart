part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3DialogVoiceSlotCreationRequestBytes = 64 * 1024;

enum AuthoringRevision3DialogVoiceSlotCreationBuildStatus { blocked }

enum AuthoringRevision3DialogVoiceSlotCreationRuntimeStatus {
  runtimeUnqualified,
}

enum AuthoringRevision3DialogVoiceSlotCreationTargetAuthority { notGranted }

enum AuthoringRevision3DialogVoiceSlotCreationPublicationStatus { notSupported }

/// Canonical project-only intent to create one exact empty managed slot for
/// one existing DialogLine locale binding.
final class AuthoringRevision3DialogVoiceSlotCreationRequestV1 {
  const AuthoringRevision3DialogVoiceSlotCreationRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.lineId,
    required this.expectedLineRevision,
    required this.localizationId,
    required this.expectedLocId,
    required this.locale,
    required this.slotId,
  });

  factory AuthoringRevision3DialogVoiceSlotCreationRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String lineId,
    required int expectedLineRevision,
    required String localizationId,
    required String expectedLocId,
    required String locale,
    required String slotId,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    if (current.revision >= _maxAuthoringRevision3VoiceAppliedRevision) {
      throw const FormatException(
        'revision-3 dialog Voice slot creation basis cannot advance its revision',
      );
    }
    return AuthoringRevision3DialogVoiceSlotCreationRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'line_id': lineId,
        'expected_line_revision': expectedLineRevision,
        'localization_id': localizationId,
        'expected_loc_id': expectedLocId,
        'locale': locale,
        'slot_id': slotId,
      }),
      currentProjectJson: currentProjectJson,
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String lineId;
  final int expectedLineRevision;
  final String localizationId;
  final String expectedLocId;
  final String locale;
  final String slotId;

  factory AuthoringRevision3DialogVoiceSlotCreationRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'dialogVoiceSlotCreationRequestJson',
        _maxAuthoringRevision3DialogVoiceSlotCreationRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 dialog Voice slot creation request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 dialog Voice slot creation request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'expected_line_revision',
      'localization_id',
      'expected_loc_id',
      'locale',
      'slot_id',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 dialog Voice slot creation request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'dialog Voice slot creation request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 dialog Voice slot creation request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 dialog Voice slot creation expected head',
        ),
      ),
    );
    final projectId = _authoringRevision3VoiceEntityId(
      request,
      'expected_project_id',
    );
    final lineId = _authoringRevision3VoiceEntityId(request, 'line_id');
    final localizationId = _authoringRevision3VoiceEntityId(
      request,
      'localization_id',
    );
    final slotId = _authoringRevision3VoiceEntityId(request, 'slot_id');
    if (<String>{lineId, localizationId, slotId}.length != 3) {
      throw const FormatException(
        'revision-3 dialog Voice slot creation entity IDs must be distinct',
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
        'revision-3 dialog Voice slot creation LocID is not safe',
      );
    }
    final parsed = AuthoringRevision3DialogVoiceSlotCreationRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: projectId,
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(
        _authoringRevision3VoiceGeneration(
          request['expected_target'],
          'dialog Voice slot creation request target',
        ),
      ),
      lineId: lineId,
      expectedLineRevision: _authoringRequiredInt(
        request,
        'expected_line_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      localizationId: localizationId,
      expectedLocId: locId,
      locale: locale,
      slotId: slotId,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    parsed._requireExactProjectBinding(current);
    return parsed;
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'revision-3 dialog Voice slot creation request does not bind the exact current project',
      );
    }
    _dialogVoiceSlotCreationRequireBasis(current.project, request: this);
  }
}

/// Strict prepare-only result. It grants no build, runtime, target,
/// deployment, save, media, or publication authority.
final class AuthoringRevision3DialogVoiceSlotCreationPreparation {
  const AuthoringRevision3DialogVoiceSlotCreationPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.targetResolution,
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
  final int lineRevision;
  final String localizationId;
  final int localizationRevision;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final Revision3ContentVoiceTargetResolution targetResolution;
  final AuthoringRevision3DialogVoiceSlotCreationBuildStatus buildStatus;
  final AuthoringRevision3DialogVoiceSlotCreationRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogVoiceSlotCreationTargetAuthority
  targetAuthority;
  final AuthoringRevision3DialogVoiceSlotCreationPublicationStatus
  publicationStatus;

  factory AuthoringRevision3DialogVoiceSlotCreationPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3DialogVoiceSlotCreationRequestV1 request,
  }) {
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _dialogVoiceSlotCreationRequireBasis(
      base.project,
      request: request,
    );
    request._requireExactProjectBinding(base);
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'project_id',
      'revision',
      'line_id',
      'line_revision',
      'localization_id',
      'localization_revision',
      'slot_id',
      'slot_revision',
      'locale',
      'loc_id',
      'target_resolution',
      'build_status',
      'runtime_status',
      'target_authority',
      'publication_status',
    }, 'revision-3 dialog Voice slot creation preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 dialog Voice slot creation response is not an unpublished preparation',
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
        'revision-3 dialog Voice slot creation response has an invalid head transition',
      );
    }
    final projectJson = _authoringRequiredString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _authoringRevision3VoiceEntityId(json, 'project_id');
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final lineId = _authoringRevision3VoiceEntityId(json, 'line_id');
    final lineRevision = _authoringRequiredInt(
      json,
      'line_revision',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final localizationId = _authoringRevision3VoiceEntityId(
      json,
      'localization_id',
    );
    final localizationRevision = _authoringRequiredInt(
      json,
      'localization_revision',
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final slotId = _authoringRevision3VoiceEntityId(json, 'slot_id');
    final slotRevision = _authoringRequiredInt(
      json,
      'slot_revision',
      max: _maxAuthoringRevision3VoiceBasisRevision,
    );
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(json, 'locale', maxBytes: 35),
    );
    final locId = _authoringRevision3VoiceString(
      json,
      'loc_id',
      maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
    );
    final targetResolution = Revision3ContentVoiceTargetResolution.parse(
      json['target_resolution'],
      'revision-3 dialog Voice slot creation target resolution',
    );
    if (projectId != request.expectedProjectId ||
        projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != request.expectedRevision + 1 ||
        revision != candidate.revision ||
        lineId != request.lineId ||
        lineRevision != request.expectedLineRevision + 1 ||
        localizationId != request.localizationId ||
        localizationRevision != basis.localizationRevision ||
        slotId != request.slotId ||
        slotRevision != 0 ||
        locale != request.locale ||
        locId != request.expectedLocId ||
        targetResolution != Revision3ContentVoiceTargetResolution.unresolved) {
      throw const FormatException(
        'revision-3 dialog Voice slot creation response disagrees with its exact request',
      );
    }
    _dialogVoiceSlotCreationRequireExactCandidate(
      base.project,
      candidate.project,
      request: request,
      lineRevision: lineRevision,
    );
    return AuthoringRevision3DialogVoiceSlotCreationPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      lineId: lineId,
      lineRevision: lineRevision,
      localizationId: localizationId,
      localizationRevision: localizationRevision,
      slotId: slotId,
      slotRevision: slotRevision,
      locale: locale,
      locId: locId,
      targetResolution: targetResolution,
      buildStatus: switch (json['build_status']) {
        'blocked' =>
          AuthoringRevision3DialogVoiceSlotCreationBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot creation response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogVoiceSlotCreationRuntimeStatus
              .runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot creation response grants unsupported runtime authority',
        ),
      },
      targetAuthority: switch (json['target_authority']) {
        'not_granted' =>
          AuthoringRevision3DialogVoiceSlotCreationTargetAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot creation response grants unsupported target authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3DialogVoiceSlotCreationPublicationStatus
              .notSupported,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot creation response grants unsupported publication authority',
        ),
      },
    );
  }
}

({int localizationRevision}) _dialogVoiceSlotCreationRequireBasis(
  Map<String, Object?> project, {
  required AuthoringRevision3DialogVoiceSlotCreationRequestV1 request,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 dialog Voice slot creation basis entities',
  );
  final line = _authoringRevision3VoiceEntity(
    entities,
    request.lineId,
    'dialog_line',
    'dialog Voice slot creation basis line',
  );
  if (_authoringRevision3VoiceRevision(line.entity) !=
      request.expectedLineRevision) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation line revision disagrees',
    );
  }
  _authoringRevision3VoiceExactOptionalFields(
    line.data,
    const {'localization', 'voice_slots'},
    const {'speaker_hint'},
    'dialog Voice slot creation basis DialogLine data',
  );
  final localizationRef = _authoringRevision3VoiceTypedRef(
    line.data['localization'],
    projectId: request.expectedProjectId,
    kind: 'localization_entry',
    context: 'dialog Voice slot creation line localization',
  );
  if (localizationRef.id != request.localizationId) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation localization differs from the exact line',
    );
  }
  final localization = _authoringRevision3VoiceEntity(
    entities,
    request.localizationId,
    'localization_entry',
    'dialog Voice slot creation basis localization',
  );
  _authoringExactFields(
    localization.data,
    const {'loc_id', 'texts'},
    'revision-3 dialog Voice slot creation LocalizationEntry data',
  );
  if (localization.data['loc_id'] != request.expectedLocId) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation localization identity disagrees',
    );
  }
  final locales = _authoringRevision3VoiceStringList(
    project['authoring_locales'],
    'dialog Voice slot creation authoring locales',
  );
  if (!locales.contains(request.locale)) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation locale is not authorable',
    );
  }
  final texts = _authoringRequiredObject(
    localization.data['texts'],
    'revision-3 dialog Voice slot creation localization texts',
  );
  final text = texts[request.locale];
  if (text is! String ||
      text.trim().isEmpty ||
      utf8.encode(text).length > _maxAuthoringRevision3VoiceTextBytes) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation locale has no bounded non-empty text',
    );
  }
  final voiceSlots = _authoringRequiredObject(
    line.data['voice_slots'],
    'revision-3 dialog Voice slot creation line slots',
  );
  if (voiceSlots.containsKey(request.locale)) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation locale already has a slot',
    );
  }
  if (entities.containsKey(request.slotId)) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation identity already exists',
    );
  }
  if (_dialogVoiceSlotCreationCountLocalRefs(
        entities,
        projectId: request.expectedProjectId,
        slotId: request.slotId,
      ) !=
      0) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation identity has a pre-existing backlink',
    );
  }
  return (
    localizationRevision: _authoringRevision3VoiceRevision(localization.entity),
  );
}

int _dialogVoiceSlotCreationCountLocalRefs(
  Map<String, Object?> entities, {
  required String projectId,
  required String slotId,
}) {
  var count = 0;
  void visit(Object? value, int depth) {
    if (depth > 128) {
      throw const FormatException(
        'revision-3 dialog Voice slot creation reference graph is too deeply nested',
      );
    }
    if (value is List) {
      for (final child in value) {
        visit(child, depth + 1);
      }
      return;
    }
    if (value is! Map) return;
    final object = _authoringRequiredObject(
      value,
      'revision-3 dialog Voice slot creation reference object',
    );
    if (object.containsKey('project_id') &&
        object.containsKey('id') &&
        object.containsKey('expected_kind') &&
        object['project_id'] == projectId &&
        object['id'] == slotId) {
      if (object.length != 3 || object['expected_kind'] != 'voice_slot') {
        throw const FormatException(
          'revision-3 dialog Voice slot creation has an invalid local backlink',
        );
      }
      count++;
      return;
    }
    for (final child in object.values) {
      visit(child, depth + 1);
    }
  }

  for (final entity in entities.values) {
    visit(entity, 0);
  }
  return count;
}

void _dialogVoiceSlotCreationRequireExactCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3DialogVoiceSlotCreationRequestV1 request,
  required int lineRevision,
}) {
  _dialogVoiceSlotCreationRequireBasis(base, request: request);
  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 dialog Voice slot creation expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 dialog Voice slot creation expected entities',
  );
  final line = _authoringRequiredObject(
    entities[request.lineId],
    'revision-3 dialog Voice slot creation expected line',
  );
  line['revision'] = lineRevision;
  final payload = _authoringRequiredObject(
    line['payload'],
    'revision-3 dialog Voice slot creation expected line payload',
  );
  final data = _authoringRequiredObject(
    payload['data'],
    'revision-3 dialog Voice slot creation expected line data',
  );
  final voiceSlots = _authoringRequiredObject(
    data['voice_slots'],
    'revision-3 dialog Voice slot creation expected line slots',
  );
  if (voiceSlots.containsKey(request.locale)) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation expected locale is occupied',
    );
  }
  voiceSlots[request.locale] = <String, Object?>{
    'project_id': request.expectedProjectId,
    'id': request.slotId,
    'expected_kind': 'voice_slot',
  };
  data['voice_slots'] = voiceSlots;
  payload['data'] = data;
  line['payload'] = payload;
  entities[request.lineId] = line;
  if (entities.containsKey(request.slotId)) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation expected identity is occupied',
    );
  }
  entities[request.slotId] = <String, Object?>{
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
        'candidates': <Object?>[],
      },
    },
  };
  expected['entities'] = entities;
  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'revision-3 dialog Voice slot creation candidate contains a non-exact project delta',
    );
  }
}
