part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceTakeRemovalRequestBytes = 64 * 1024;

enum AuthoringRevision3VoiceTakeRemovalBuildStatus { blocked }

enum AuthoringRevision3VoiceTakeRemovalRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3VoiceTakeRemovalPublicationStatus { notSupported }

/// Canonical intent to detach one exact VoiceTake candidate from one exact
/// line/language VoiceSlot. It carries project-storage authority only.
final class AuthoringRevision3VoiceTakeRemovalRequestV1 {
  const AuthoringRevision3VoiceTakeRemovalRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.lineId,
    required this.localizationId,
    required this.expectedLocId,
    required this.locale,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.takeId,
    required this.expectedTakeRevision,
    required this.expectedSelectedTakeId,
  });

  factory AuthoringRevision3VoiceTakeRemovalRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String lineId,
    required String localizationId,
    required String expectedLocId,
    required String locale,
    required String slotId,
    required int expectedSlotRevision,
    required String takeId,
    required int expectedTakeRevision,
    required String? expectedSelectedTakeId,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    if (current.revision >= _maxAuthoringRevision3VoiceAppliedRevision) {
      throw const FormatException(
        'revision-3 Voice take removal basis cannot advance its revision',
      );
    }
    return AuthoringRevision3VoiceTakeRemovalRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'line_id': lineId,
        'localization_id': localizationId,
        'expected_loc_id': expectedLocId,
        'locale': locale,
        'slot_id': slotId,
        'expected_slot_revision': expectedSlotRevision,
        'take_id': takeId,
        'expected_take_revision': expectedTakeRevision,
        'expected_selected_take_id': expectedSelectedTakeId,
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
  final String localizationId;
  final String expectedLocId;
  final String locale;
  final String slotId;
  final int expectedSlotRevision;
  final String takeId;
  final int expectedTakeRevision;
  final String? expectedSelectedTakeId;

  factory AuthoringRevision3VoiceTakeRemovalRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'voiceTakeRemovalRequestJson',
        _maxAuthoringRevision3VoiceTakeRemovalRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 Voice take removal request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Voice take removal request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'localization_id',
      'expected_loc_id',
      'locale',
      'slot_id',
      'expected_slot_revision',
      'take_id',
      'expected_take_revision',
      'expected_selected_take_id',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Voice take removal request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'take removal request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 Voice take removal request is not canonical',
      );
    }

    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Voice take removal expected head',
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
    final takeId = _authoringRevision3VoiceEntityId(request, 'take_id');
    if (<String>{lineId, localizationId, slotId, takeId}.length != 4) {
      throw const FormatException(
        'revision-3 Voice take removal entity IDs must be distinct',
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
        'revision-3 Voice take removal LocID is not a safe archive basename stem',
      );
    }
    final parsed = AuthoringRevision3VoiceTakeRemovalRequestV1._(
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
          'take removal request target',
        ),
      ),
      lineId: lineId,
      localizationId: localizationId,
      expectedLocId: locId,
      locale: locale,
      slotId: slotId,
      expectedSlotRevision: _authoringRequiredInt(
        request,
        'expected_slot_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      takeId: takeId,
      expectedTakeRevision: _authoringRequiredInt(
        request,
        'expected_take_revision',
        max: _maxAuthoringRevision3VoiceAppliedRevision,
      ),
      expectedSelectedTakeId: _voiceTakeRemovalNullableEntityId(
        request,
        'expected_selected_take_id',
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
        'revision-3 Voice take removal request does not bind the exact current project',
      );
    }
    _voiceTakeRemovalRequireBasis(current.project, request: this);
  }
}

/// Strict prepare-only result. It grants no build, runtime, deployment, game,
/// save, source-artifact, or native publication authority.
final class AuthoringRevision3VoiceTakeRemovalPreparation {
  const AuthoringRevision3VoiceTakeRemovalPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.takeId,
    required this.takeRevision,
    required this.previousSelectedTakeId,
    required this.selectionCleared,
    required this.takeEntityRemoved,
    required this.remainingCandidateCount,
    required this.buildStatus,
    required this.runtimeStatus,
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
  final int slotRevision;
  final String locale;
  final String locId;
  final String takeId;
  final int takeRevision;
  final String? previousSelectedTakeId;
  final bool selectionCleared;
  final bool takeEntityRemoved;
  final int remainingCandidateCount;
  final AuthoringRevision3VoiceTakeRemovalBuildStatus buildStatus;
  final AuthoringRevision3VoiceTakeRemovalRuntimeStatus runtimeStatus;
  final AuthoringRevision3VoiceTakeRemovalPublicationStatus publicationStatus;

  factory AuthoringRevision3VoiceTakeRemovalPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRemovalRequestV1 request,
  }) {
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _voiceTakeRemovalRequireBasis(base.project, request: request);
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
      'localization_id',
      'slot_id',
      'slot_revision',
      'locale',
      'loc_id',
      'take_id',
      'take_revision',
      'previous_selected_take_id',
      'selection_cleared',
      'take_entity_removed',
      'remaining_candidate_count',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Voice take removal preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 Voice take removal response is not an unpublished preparation',
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
        'revision-3 Voice take removal response has an invalid head transition',
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
    final localizationId = _authoringRevision3VoiceEntityId(
      json,
      'localization_id',
    );
    final slotId = _authoringRevision3VoiceEntityId(json, 'slot_id');
    final slotRevision = _authoringRequiredInt(
      json,
      'slot_revision',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(json, 'locale', maxBytes: 35),
    );
    final locId = _authoringRevision3VoiceString(
      json,
      'loc_id',
      maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
    );
    final takeId = _authoringRevision3VoiceEntityId(json, 'take_id');
    final takeRevision = _authoringRequiredInt(
      json,
      'take_revision',
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final previousSelected = _voiceTakeRemovalNullableEntityId(
      json,
      'previous_selected_take_id',
    );
    final selectionCleared = _authoringRequiredBool(json, 'selection_cleared');
    final takeEntityRemoved = _authoringRequiredBool(
      json,
      'take_entity_removed',
    );
    final remainingCandidateCount = _authoringRequiredInt(
      json,
      'remaining_candidate_count',
      max: _maxAuthoringRevision3VoiceSlotCandidates,
    );
    if (projectId != request.expectedProjectId ||
        projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != request.expectedRevision + 1 ||
        revision != candidate.revision ||
        lineId != request.lineId ||
        localizationId != request.localizationId ||
        slotId != request.slotId ||
        slotRevision != request.expectedSlotRevision + 1 ||
        locale != request.locale ||
        locId != request.expectedLocId ||
        takeId != request.takeId ||
        takeRevision != request.expectedTakeRevision ||
        previousSelected != request.expectedSelectedTakeId ||
        selectionCleared != basis.selectionCleared ||
        takeEntityRemoved != basis.takeEntityShouldBeRemoved ||
        remainingCandidateCount != basis.remainingCandidateCount) {
      throw const FormatException(
        'revision-3 Voice take removal response disagrees with its exact request',
      );
    }
    _voiceTakeRemovalRequireExactCandidate(
      base.project,
      candidate.project,
      request: request,
      slotRevision: slotRevision,
      takeEntityRemoved: takeEntityRemoved,
    );
    return AuthoringRevision3VoiceTakeRemovalPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      lineId: lineId,
      localizationId: localizationId,
      slotId: slotId,
      slotRevision: slotRevision,
      locale: locale,
      locId: locId,
      takeId: takeId,
      takeRevision: takeRevision,
      previousSelectedTakeId: previousSelected,
      selectionCleared: selectionCleared,
      takeEntityRemoved: takeEntityRemoved,
      remainingCandidateCount: remainingCandidateCount,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3VoiceTakeRemovalBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 Voice take removal response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3VoiceTakeRemovalRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 Voice take removal response grants unsupported runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3VoiceTakeRemovalPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 Voice take removal response grants unsupported publication authority',
        ),
      },
    );
  }
}

String? _voiceTakeRemovalNullableEntityId(
  Map<String, Object?> json,
  String field,
) {
  if (!json.containsKey(field)) {
    throw FormatException(
      'revision-3 Voice take removal field $field is missing',
    );
  }
  if (json[field] == null) return null;
  return _authoringRevision3VoiceEntityId(json, field);
}

({
  int remainingCandidateCount,
  bool selectionCleared,
  bool takeEntityShouldBeRemoved,
})
_voiceTakeRemovalRequireBasis(
  Map<String, Object?> project, {
  required AuthoringRevision3VoiceTakeRemovalRequestV1 request,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Voice take removal basis entities',
  );
  final line = _authoringRevision3VoiceEntity(
    entities,
    request.lineId,
    'dialog_line',
    'take removal basis line',
  );
  _authoringRevision3VoiceExactOptionalFields(
    line.data,
    const {'localization', 'voice_slots'},
    const {'speaker_hint'},
    'take removal basis DialogLine data',
  );
  final localizationRef = _authoringRevision3VoiceTypedRef(
    line.data['localization'],
    projectId: request.expectedProjectId,
    kind: 'localization_entry',
    context: 'take removal line localization',
  );
  if (localizationRef.id != request.localizationId) {
    throw const FormatException(
      'revision-3 Voice take removal localization differs from the exact line',
    );
  }
  final localization = _authoringRevision3VoiceEntity(
    entities,
    request.localizationId,
    'localization_entry',
    'take removal basis localization',
  );
  _authoringExactFields(localization.data, const {
    'loc_id',
    'texts',
  }, 'revision-3 Voice take removal LocalizationEntry data');
  if (localization.data['loc_id'] != request.expectedLocId) {
    throw const FormatException(
      'revision-3 Voice take removal localization identity disagrees',
    );
  }
  final voiceSlots = _authoringRequiredObject(
    line.data['voice_slots'],
    'revision-3 Voice take removal line slots',
  );
  final slotRef = _authoringRevision3VoiceTypedRef(
    voiceSlots[request.locale],
    projectId: request.expectedProjectId,
    kind: 'voice_slot',
    context: 'take removal line slot',
  );
  if (slotRef.id != request.slotId) {
    throw const FormatException(
      'revision-3 Voice take removal slot differs from the exact line',
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
    'take removal basis slot',
  );
  if (_authoringRevision3VoiceRevision(slot.entity) !=
      request.expectedSlotRevision) {
    throw const FormatException(
      'revision-3 Voice take removal slot revision disagrees',
    );
  }
  final candidates = _authoringRevision3VoiceObjectList(
    slot.data['candidates'],
    'take removal candidates',
  );
  var candidateMatches = 0;
  for (final candidate in candidates) {
    final ref = _authoringRevision3VoiceTypedRef(
      candidate,
      projectId: request.expectedProjectId,
      kind: 'voice_take',
      context: 'take removal candidate',
    );
    if (ref.id == request.takeId) candidateMatches++;
  }
  if (candidateMatches != 1) {
    throw const FormatException(
      'revision-3 Voice take removal target is not one exact candidate',
    );
  }
  final take = _authoringRevision3VoiceEntity(
    entities,
    request.takeId,
    'voice_take',
    'take removal basis take',
  );
  if (_authoringRevision3VoiceRevision(take.entity) !=
          request.expectedTakeRevision ||
      take.data['locale'] != request.locale) {
    throw const FormatException(
      'revision-3 Voice take removal take revision or locale disagrees',
    );
  }
  final selectedValue = slot.data['selected'];
  final selected = selectedValue == null
      ? null
      : _authoringRevision3VoiceTypedRef(
          selectedValue,
          projectId: request.expectedProjectId,
          kind: 'voice_take',
          context: 'take removal current selection',
        ).id;
  if (selected != request.expectedSelectedTakeId) {
    throw const FormatException(
      'revision-3 Voice take removal current selection disagrees',
    );
  }

  final allRefs = _voiceTakeRemovalCountAllLocalTypedRefs(
    entities,
    projectId: request.expectedProjectId,
    takeId: request.takeId,
  );
  final slotRefs = _voiceTakeRemovalCountAllowedSlotRefs(
    entities,
    projectId: request.expectedProjectId,
    takeId: request.takeId,
  );
  if (allRefs != slotRefs) {
    throw const FormatException(
      'revision-3 Voice take removal take has a non-slot local backlink',
    );
  }
  final targetRefs = 1 + (selected == request.takeId ? 1 : 0);
  if (slotRefs < targetRefs) {
    throw const FormatException(
      'revision-3 Voice take removal reference accounting is inconsistent',
    );
  }
  return (
    remainingCandidateCount: candidates.length - 1,
    selectionCleared: selected == request.takeId,
    takeEntityShouldBeRemoved: slotRefs == targetRefs,
  );
}

int _voiceTakeRemovalCountAllLocalTypedRefs(
  Map<String, Object?> entities, {
  required String projectId,
  required String takeId,
}) {
  var count = 0;

  void visit(Object? value, int depth) {
    if (depth > 128) {
      throw const FormatException(
        'revision-3 Voice take removal reference graph is too deeply nested',
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
      'revision-3 Voice take removal reference object',
    );
    if (object.containsKey('project_id') &&
        object.containsKey('id') &&
        object.containsKey('expected_kind') &&
        object['project_id'] == projectId &&
        object['id'] == takeId) {
      if (object.length != 3 || object['expected_kind'] != 'voice_take') {
        throw const FormatException(
          'revision-3 Voice take removal has an invalid local backlink',
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

int _voiceTakeRemovalCountAllowedSlotRefs(
  Map<String, Object?> entities, {
  required String projectId,
  required String takeId,
}) {
  var count = 0;
  for (final entry in entities.entries) {
    final entity = _authoringRequiredObject(
      entry.value,
      'revision-3 Voice take removal backlink entity',
    );
    final payload = _authoringRequiredObject(
      entity['payload'],
      'revision-3 Voice take removal backlink payload',
    );
    if (payload['kind'] != 'voice_slot') continue;
    final data = _authoringRequiredObject(
      payload['data'],
      'revision-3 Voice take removal backlink VoiceSlot data',
    );
    final candidates = _authoringRevision3VoiceObjectList(
      data['candidates'],
      'take removal backlink candidates',
    );
    for (final candidate in candidates) {
      if (candidate['project_id'] == projectId && candidate['id'] == takeId) {
        _authoringRevision3VoiceTypedRef(
          candidate,
          projectId: projectId,
          kind: 'voice_take',
          context: 'take removal backlink candidate',
        );
        count++;
      }
    }
    final selected = data['selected'];
    if (selected != null) {
      final selectedRef = _authoringRequiredObject(
        selected,
        'revision-3 Voice take removal backlink selected take',
      );
      if (selectedRef['project_id'] == projectId &&
          selectedRef['id'] == takeId) {
        _authoringRevision3VoiceTypedRef(
          selectedRef,
          projectId: projectId,
          kind: 'voice_take',
          context: 'take removal backlink selected take',
        );
        count++;
      }
    }
  }
  return count;
}

void _voiceTakeRemovalRequireExactCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3VoiceTakeRemovalRequestV1 request,
  required int slotRevision,
  required bool takeEntityRemoved,
}) {
  _voiceTakeRemovalRequireBasis(base, request: request);
  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 Voice take removal expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 Voice take removal expected entities',
  );
  final slot = _authoringRequiredObject(
    entities[request.slotId],
    'revision-3 Voice take removal expected slot',
  );
  final payload = _authoringRequiredObject(
    slot['payload'],
    'revision-3 Voice take removal expected slot payload',
  );
  final data = _authoringRequiredObject(
    payload['data'],
    'revision-3 Voice take removal expected slot data',
  );
  final candidates = _authoringRevision3VoiceObjectList(
    data['candidates'],
    'take removal expected candidates',
  );
  var removed = 0;
  final survivors = <Map<String, Object?>>[];
  for (final candidate in candidates) {
    final ref = _authoringRevision3VoiceTypedRef(
      candidate,
      projectId: request.expectedProjectId,
      kind: 'voice_take',
      context: 'take removal expected candidate',
    );
    if (ref.id == request.takeId) {
      removed++;
    } else {
      survivors.add(candidate);
    }
  }
  if (removed != 1) {
    throw const FormatException(
      'revision-3 Voice take removal expected candidate delta is invalid',
    );
  }
  data['candidates'] = survivors;
  if (request.expectedSelectedTakeId == request.takeId) {
    data.remove('selected');
  }
  payload['data'] = data;
  slot['payload'] = payload;
  slot['revision'] = slotRevision;
  entities[request.slotId] = slot;
  if (takeEntityRemoved) entities.remove(request.takeId);
  expected['entities'] = entities;
  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'revision-3 Voice take removal candidate contains a non-exact project delta',
    );
  }
}
