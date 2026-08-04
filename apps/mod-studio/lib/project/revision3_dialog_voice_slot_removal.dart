part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3DialogVoiceSlotRemovalRequestBytes = 64 * 1024;

enum AuthoringRevision3DialogVoiceSlotRemovalBuildStatus { blocked }

enum AuthoringRevision3DialogVoiceSlotRemovalRuntimeStatus {
  runtimeUnqualified,
}

enum AuthoringRevision3DialogVoiceSlotRemovalTargetAuthority { notGranted }

enum AuthoringRevision3DialogVoiceSlotRemovalPublicationStatus { notSupported }

/// Canonical project-only intent to remove one exact, empty, unselected slot
/// from one exact DialogLine locale binding.
final class AuthoringRevision3DialogVoiceSlotRemovalRequestV1 {
  const AuthoringRevision3DialogVoiceSlotRemovalRequestV1._({
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
    required this.expectedSlotRevision,
  });

  factory AuthoringRevision3DialogVoiceSlotRemovalRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String lineId,
    required int expectedLineRevision,
    required String localizationId,
    required String expectedLocId,
    required String locale,
    required String slotId,
    required int expectedSlotRevision,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    if (current.revision >= _maxAuthoringRevision3VoiceAppliedRevision) {
      throw const FormatException(
        'revision-3 dialog Voice slot removal basis cannot advance its revision',
      );
    }
    return AuthoringRevision3DialogVoiceSlotRemovalRequestV1.fromCanonicalJson(
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
        'expected_slot_revision': expectedSlotRevision,
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
  final int expectedSlotRevision;

  factory AuthoringRevision3DialogVoiceSlotRemovalRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'dialogVoiceSlotRemovalRequestJson',
        _maxAuthoringRevision3DialogVoiceSlotRemovalRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 dialog Voice slot removal request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 dialog Voice slot removal request',
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
      'expected_slot_revision',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 dialog Voice slot removal request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'dialog Voice slot removal request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 dialog Voice slot removal request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 dialog Voice slot removal expected head',
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
        'revision-3 dialog Voice slot removal entity IDs must be distinct',
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
        'revision-3 dialog Voice slot removal LocID is not safe',
      );
    }
    final parsed = AuthoringRevision3DialogVoiceSlotRemovalRequestV1._(
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
          'dialog Voice slot removal request target',
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
      expectedSlotRevision: _authoringRequiredInt(
        request,
        'expected_slot_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
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
        'revision-3 dialog Voice slot removal request does not bind the exact current project',
      );
    }
    _dialogVoiceSlotRemovalRequireBasis(current.project, request: this);
  }
}

/// Strict prepare-only result. It grants no build, runtime, target,
/// deployment, save, media, or publication authority.
final class AuthoringRevision3DialogVoiceSlotRemovalPreparation {
  const AuthoringRevision3DialogVoiceSlotRemovalPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.slotId,
    required this.removedSlotRevision,
    required this.locale,
    required this.locId,
    required this.removedTargetResolution,
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
  final String slotId;
  final int removedSlotRevision;
  final String locale;
  final String locId;
  final Revision3ContentVoiceTargetResolution removedTargetResolution;
  final AuthoringRevision3DialogVoiceSlotRemovalBuildStatus buildStatus;
  final AuthoringRevision3DialogVoiceSlotRemovalRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogVoiceSlotRemovalTargetAuthority targetAuthority;
  final AuthoringRevision3DialogVoiceSlotRemovalPublicationStatus
  publicationStatus;

  factory AuthoringRevision3DialogVoiceSlotRemovalPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3DialogVoiceSlotRemovalRequestV1 request,
  }) {
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _dialogVoiceSlotRemovalRequireBasis(
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
      'slot_id',
      'removed_slot_revision',
      'locale',
      'loc_id',
      'removed_target_resolution',
      'build_status',
      'runtime_status',
      'target_authority',
      'publication_status',
    }, 'revision-3 dialog Voice slot removal preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 dialog Voice slot removal response is not an unpublished preparation',
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
        'revision-3 dialog Voice slot removal response has an invalid head transition',
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
    final slotId = _authoringRevision3VoiceEntityId(json, 'slot_id');
    final removedSlotRevision = _authoringRequiredInt(
      json,
      'removed_slot_revision',
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
    final removedTargetResolution = Revision3ContentVoiceTargetResolution.parse(
      json['removed_target_resolution'],
      'revision-3 dialog Voice slot removal target resolution',
    );
    if (projectId != request.expectedProjectId ||
        projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != request.expectedRevision + 1 ||
        revision != candidate.revision ||
        lineId != request.lineId ||
        lineRevision != request.expectedLineRevision + 1 ||
        localizationId != request.localizationId ||
        slotId != request.slotId ||
        removedSlotRevision != request.expectedSlotRevision ||
        locale != request.locale ||
        locId != request.expectedLocId ||
        removedTargetResolution != basis.targetResolution) {
      throw const FormatException(
        'revision-3 dialog Voice slot removal response disagrees with its exact request',
      );
    }
    _dialogVoiceSlotRemovalRequireExactCandidate(
      base.project,
      candidate.project,
      request: request,
      lineRevision: lineRevision,
    );
    return AuthoringRevision3DialogVoiceSlotRemovalPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      lineId: lineId,
      lineRevision: lineRevision,
      localizationId: localizationId,
      slotId: slotId,
      removedSlotRevision: removedSlotRevision,
      locale: locale,
      locId: locId,
      removedTargetResolution: removedTargetResolution,
      buildStatus: switch (json['build_status']) {
        'blocked' =>
          AuthoringRevision3DialogVoiceSlotRemovalBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot removal response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogVoiceSlotRemovalRuntimeStatus
              .runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot removal response grants unsupported runtime authority',
        ),
      },
      targetAuthority: switch (json['target_authority']) {
        'not_granted' =>
          AuthoringRevision3DialogVoiceSlotRemovalTargetAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot removal response grants unsupported target authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3DialogVoiceSlotRemovalPublicationStatus
              .notSupported,
        _ => throw const FormatException(
          'revision-3 dialog Voice slot removal response grants unsupported publication authority',
        ),
      },
    );
  }
}

({Revision3ContentVoiceTargetResolution targetResolution})
_dialogVoiceSlotRemovalRequireBasis(
  Map<String, Object?> project, {
  required AuthoringRevision3DialogVoiceSlotRemovalRequestV1 request,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 dialog Voice slot removal basis entities',
  );
  final line = _authoringRevision3VoiceEntity(
    entities,
    request.lineId,
    'dialog_line',
    'dialog Voice slot removal basis line',
  );
  if (_authoringRevision3VoiceRevision(line.entity) !=
      request.expectedLineRevision) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal line revision disagrees',
    );
  }
  _authoringRevision3VoiceExactOptionalFields(
    line.data,
    const {'localization', 'voice_slots'},
    const {'speaker_hint'},
    'dialog Voice slot removal basis DialogLine data',
  );
  final localizationRef = _authoringRevision3VoiceTypedRef(
    line.data['localization'],
    projectId: request.expectedProjectId,
    kind: 'localization_entry',
    context: 'dialog Voice slot removal line localization',
  );
  if (localizationRef.id != request.localizationId) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal localization differs from the exact line',
    );
  }
  final localization = _authoringRevision3VoiceEntity(
    entities,
    request.localizationId,
    'localization_entry',
    'dialog Voice slot removal basis localization',
  );
  _authoringExactFields(
    localization.data,
    const {'loc_id', 'texts'},
    'revision-3 dialog Voice slot removal LocalizationEntry data',
  );
  if (localization.data['loc_id'] != request.expectedLocId) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal localization identity disagrees',
    );
  }
  final voiceSlots = _authoringRequiredObject(
    line.data['voice_slots'],
    'revision-3 dialog Voice slot removal line slots',
  );
  final slotRef = _authoringRevision3VoiceTypedRef(
    voiceSlots[request.locale],
    projectId: request.expectedProjectId,
    kind: 'voice_slot',
    context: 'dialog Voice slot removal line slot',
  );
  if (slotRef.id != request.slotId) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal slot differs from the exact line',
    );
  }
  _authoringRevision3VoiceValidateExistingSlot(
    entities,
    projectId: request.expectedProjectId,
    lineId: request.lineId,
    slotId: request.slotId,
    locale: request.locale,
    locId: request.expectedLocId,
  );
  final slot = _authoringRevision3VoiceEntity(
    entities,
    request.slotId,
    'voice_slot',
    'dialog Voice slot removal basis slot',
  );
  if (_authoringRevision3VoiceRevision(slot.entity) !=
      request.expectedSlotRevision) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal slot revision disagrees',
    );
  }
  final slotOrigin = _authoringRequiredObject(
    slot.entity['origin'],
    'revision-3 dialog Voice slot removal slot origin',
  );
  _authoringExactFields(slotOrigin, const {
    'type',
    'generator_id',
    'generator_version',
    'owner',
  }, 'revision-3 dialog Voice slot removal slot origin');
  final slotOwner = _authoringRevision3VoiceTypedRef(
    slotOrigin['owner'],
    projectId: request.expectedProjectId,
    kind: 'dialog_line',
    context: 'revision-3 dialog Voice slot removal slot owner',
  );
  if (slotOrigin['type'] != 'generated' ||
      slotOrigin['generator_id'] != _authoringRevision3VoiceSlotGeneratorId ||
      slotOrigin['generator_version'] !=
          _authoringRevision3VoiceSlotGeneratorVersion ||
      slotOwner.id != request.lineId) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal requires the exact managed generated slot origin',
    );
  }
  final candidates = _authoringRevision3VoiceObjectList(
    slot.data['candidates'],
    'dialog Voice slot removal candidates',
  );
  if (candidates.isNotEmpty || slot.data['selected'] != null) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal requires an empty unselected slot',
    );
  }
  if (_dialogVoiceSlotRemovalCountLocalRefs(
        entities,
        projectId: request.expectedProjectId,
        slotId: request.slotId,
      ) !=
      1) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal slot is not uniquely owned',
    );
  }
  final parsedResolution = _authoringRevision3VoiceTargetResolution(
    slot.data['target_resolution'],
    locId: request.expectedLocId,
    context: 'revision-3 dialog Voice slot removal target resolution',
  );
  return (
    targetResolution: switch (parsedResolution.state) {
      AuthoringRevision3VoiceTargetResolutionState.unresolved =>
        Revision3ContentVoiceTargetResolution.unresolved,
      AuthoringRevision3VoiceTargetResolutionState.ambiguous =>
        Revision3ContentVoiceTargetResolution.ambiguous,
      AuthoringRevision3VoiceTargetResolutionState.resolved =>
        Revision3ContentVoiceTargetResolution.resolved,
    },
  );
}

int _dialogVoiceSlotRemovalCountLocalRefs(
  Map<String, Object?> entities, {
  required String projectId,
  required String slotId,
}) {
  var count = 0;
  void visit(Object? value, int depth) {
    if (depth > 128) {
      throw const FormatException(
        'revision-3 dialog Voice slot removal reference graph is too deeply nested',
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
      'revision-3 dialog Voice slot removal reference object',
    );
    if (object.containsKey('project_id') &&
        object.containsKey('id') &&
        object.containsKey('expected_kind') &&
        object['project_id'] == projectId &&
        object['id'] == slotId) {
      if (object.length != 3 || object['expected_kind'] != 'voice_slot') {
        throw const FormatException(
          'revision-3 dialog Voice slot removal has an invalid local backlink',
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

void _dialogVoiceSlotRemovalRequireExactCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3DialogVoiceSlotRemovalRequestV1 request,
  required int lineRevision,
}) {
  _dialogVoiceSlotRemovalRequireBasis(base, request: request);
  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 dialog Voice slot removal expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 dialog Voice slot removal expected entities',
  );
  final line = _authoringRequiredObject(
    entities[request.lineId],
    'revision-3 dialog Voice slot removal expected line',
  );
  line['revision'] = lineRevision;
  final payload = _authoringRequiredObject(
    line['payload'],
    'revision-3 dialog Voice slot removal expected line payload',
  );
  final data = _authoringRequiredObject(
    payload['data'],
    'revision-3 dialog Voice slot removal expected line data',
  );
  final voiceSlots = _authoringRequiredObject(
    data['voice_slots'],
    'revision-3 dialog Voice slot removal expected line slots',
  );
  if (voiceSlots.remove(request.locale) == null ||
      entities.remove(request.slotId) == null) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal expected delta is missing its slot',
    );
  }
  data['voice_slots'] = voiceSlots;
  payload['data'] = data;
  line['payload'] = payload;
  entities[request.lineId] = line;
  expected['entities'] = entities;
  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'revision-3 dialog Voice slot removal candidate contains a non-exact project delta',
    );
  }
}
