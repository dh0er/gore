part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceTakeSelectionRequestBytes = 64 * 1024;

enum AuthoringRevision3VoiceTakeSelectionBuildStatus { blocked }

enum AuthoringRevision3VoiceTakeSelectionRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3VoiceTakeSelectionPublicationStatus { notSupported }

/// Canonical intent to change only the selected take of one exact VoiceSlot.
///
/// All identities come from a freshly projected content checkpoint. In
/// particular, `null` means an explicit clear; it is never an omitted or
/// inferred first candidate.
final class AuthoringRevision3VoiceTakeSelectionRequestV1 {
  const AuthoringRevision3VoiceTakeSelectionRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.lineId,
    required this.slotId,
    required this.expectedSlotRevision,
    required this.locale,
    required this.expectedLocId,
    required this.expectedSelectedTakeId,
    required this.selectedTakeId,
  });

  factory AuthoringRevision3VoiceTakeSelectionRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String lineId,
    required String slotId,
    required int expectedSlotRevision,
    required String locale,
    required String expectedLocId,
    required String? expectedSelectedTakeId,
    required String? selectedTakeId,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3VoiceTakeSelectionRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'line_id': lineId,
        'slot_id': slotId,
        'expected_slot_revision': expectedSlotRevision,
        'locale': locale,
        'expected_loc_id': expectedLocId,
        'expected_selected_take_id': expectedSelectedTakeId,
        'selected_take_id': selectedTakeId,
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
  final String slotId;
  final int expectedSlotRevision;
  final String locale;
  final String expectedLocId;
  final String? expectedSelectedTakeId;
  final String? selectedTakeId;

  factory AuthoringRevision3VoiceTakeSelectionRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'voiceTakeSelectionRequestJson',
        _maxAuthoringRevision3VoiceTakeSelectionRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 Voice take selection request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Voice take selection request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'slot_id',
      'expected_slot_revision',
      'locale',
      'expected_loc_id',
      'expected_selected_take_id',
      'selected_take_id',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Voice take selection request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'take selection request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 Voice take selection request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Voice take selection expected head',
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
        'revision-3 Voice take selection line and slot IDs must differ',
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
        'revision-3 Voice take selection LocID is not a safe archive basename stem',
      );
    }
    final expectedSelected = _voiceTakeSelectionNullableEntityId(
      request,
      'expected_selected_take_id',
    );
    final selected = _voiceTakeSelectionNullableEntityId(
      request,
      'selected_take_id',
    );
    if (expectedSelected == selected) {
      throw const FormatException(
        'revision-3 Voice take selection request does not change the selection',
      );
    }
    final parsed = AuthoringRevision3VoiceTakeSelectionRequestV1._(
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
          'take selection request target',
        ),
      ),
      lineId: lineId,
      slotId: slotId,
      expectedSlotRevision: _authoringRequiredInt(
        request,
        'expected_slot_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      locale: locale,
      expectedLocId: locId,
      expectedSelectedTakeId: expectedSelected,
      selectedTakeId: selected,
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
        'revision-3 Voice take selection request does not bind the exact current project',
      );
    }
    _voiceTakeSelectionRequireBasis(current.project, request: this);
  }
}

/// Strict unpublished candidate whose only semantic delta is VoiceSlot.selected.
final class AuthoringRevision3VoiceTakeSelectionPreparation {
  const AuthoringRevision3VoiceTakeSelectionPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.lineId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.previousSelectedTakeId,
    required this.selectedTakeId,
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
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final String? previousSelectedTakeId;
  final String? selectedTakeId;
  final AuthoringRevision3VoiceTakeSelectionBuildStatus buildStatus;
  final AuthoringRevision3VoiceTakeSelectionRuntimeStatus runtimeStatus;
  final AuthoringRevision3VoiceTakeSelectionPublicationStatus publicationStatus;

  factory AuthoringRevision3VoiceTakeSelectionPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeSelectionRequestV1 request,
  }) {
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
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
      'slot_id',
      'slot_revision',
      'locale',
      'loc_id',
      'previous_selected_take_id',
      'selected_take_id',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Voice take selection preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 Voice take selection response is not an unpublished preparation',
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
    // WorkingHead seals a SnapshotManifest, not project_json. Only the exact
    // basis and a real head transition are valid comparisons here.
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson ||
        head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'revision-3 Voice take selection response has an invalid head transition',
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
    final previousSelected = _voiceTakeSelectionNullableEntityId(
      json,
      'previous_selected_take_id',
    );
    final selected = _voiceTakeSelectionNullableEntityId(
      json,
      'selected_take_id',
    );
    if (projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != base.revision + 1 ||
        revision != candidate.revision ||
        lineId != request.lineId ||
        slotId != request.slotId ||
        slotRevision != request.expectedSlotRevision + 1 ||
        locale != request.locale ||
        locId != request.expectedLocId ||
        previousSelected != request.expectedSelectedTakeId ||
        selected != request.selectedTakeId) {
      throw const FormatException(
        'revision-3 Voice take selection response disagrees with its exact request',
      );
    }
    _voiceTakeSelectionRequireExactCandidate(
      base.project,
      candidate.project,
      request: request,
      slotRevision: slotRevision,
    );
    return AuthoringRevision3VoiceTakeSelectionPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      lineId: lineId,
      slotId: slotId,
      slotRevision: slotRevision,
      locale: locale,
      locId: locId,
      previousSelectedTakeId: previousSelected,
      selectedTakeId: selected,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3VoiceTakeSelectionBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 Voice take selection response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3VoiceTakeSelectionRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 Voice take selection response grants unsupported runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3VoiceTakeSelectionPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 Voice take selection response grants unsupported publication authority',
        ),
      },
    );
  }
}

String? _voiceTakeSelectionNullableEntityId(
  Map<String, Object?> json,
  String field,
) {
  if (!json.containsKey(field)) {
    throw FormatException(
      'revision-3 Voice take selection field $field is missing',
    );
  }
  if (json[field] == null) return null;
  return _authoringRevision3VoiceEntityId(json, field);
}

void _voiceTakeSelectionRequireBasis(
  Map<String, Object?> project, {
  required AuthoringRevision3VoiceTakeSelectionRequestV1 request,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Voice take selection basis entities',
  );
  final line = _authoringRevision3VoiceEntity(
    entities,
    request.lineId,
    'dialog_line',
    'take selection basis line',
  );
  _authoringRevision3VoiceExactOptionalFields(
    line.data,
    const {'localization', 'voice_slots'},
    const {'speaker_hint'},
    'take selection basis DialogLine data',
  );
  final localizationRef = _authoringRevision3VoiceTypedRef(
    line.data['localization'],
    projectId: request.expectedProjectId,
    kind: 'localization_entry',
    context: 'take selection line localization',
  );
  final localization = _authoringRevision3VoiceEntity(
    entities,
    localizationRef.id,
    'localization_entry',
    'take selection basis localization',
  );
  _authoringExactFields(localization.data, const {
    'loc_id',
    'texts',
  }, 'revision-3 Voice take selection LocalizationEntry data');
  final locId = _authoringRevision3VoiceString(
    localization.data,
    'loc_id',
    maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
  );
  if (locId != request.expectedLocId ||
      !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId)) {
    throw const FormatException(
      'revision-3 Voice take selection localization identity disagrees',
    );
  }
  final voiceSlots = _authoringRequiredObject(
    line.data['voice_slots'],
    'revision-3 Voice take selection line slots',
  );
  final slotRef = _authoringRevision3VoiceTypedRef(
    voiceSlots[request.locale],
    projectId: request.expectedProjectId,
    kind: 'voice_slot',
    context: 'take selection line slot',
  );
  if (slotRef.id != request.slotId) {
    throw const FormatException(
      'revision-3 Voice take selection slot differs from the exact line',
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
    'take selection basis slot',
  );
  if (_authoringRevision3VoiceRevision(slot.entity) !=
      request.expectedSlotRevision) {
    throw const FormatException(
      'revision-3 Voice take selection slot revision disagrees',
    );
  }
  final candidates = _authoringRevision3VoiceObjectList(
    slot.data['candidates'],
    'take selection candidates',
  );
  final statuses = <String, AuthoringRevision3VoiceTakeStatus>{};
  for (final candidate in candidates) {
    final ref = _authoringRevision3VoiceTypedRef(
      candidate,
      projectId: request.expectedProjectId,
      kind: 'voice_take',
      context: 'take selection candidate',
    );
    final take = _authoringRevision3VoiceEntity(
      entities,
      ref.id,
      'voice_take',
      'take selection candidate',
    );
    if (take.data['locale'] != request.locale) {
      throw const FormatException(
        'revision-3 Voice take selection candidate locale disagrees',
      );
    }
    statuses[ref.id] = _authoringRevision3VoiceStatus(take.data['status']);
  }
  final selectedValue = slot.data['selected'];
  final basisSelected = selectedValue == null
      ? null
      : _authoringRevision3VoiceTypedRef(
          selectedValue,
          projectId: request.expectedProjectId,
          kind: 'voice_take',
          context: 'take selection current selection',
        ).id;
  if (basisSelected != request.expectedSelectedTakeId) {
    throw const FormatException(
      'revision-3 Voice take selection current selection disagrees',
    );
  }
  final requested = request.selectedTakeId;
  if (requested != null &&
      statuses[requested] != AuthoringRevision3VoiceTakeStatus.approved) {
    throw const FormatException(
      'revision-3 Voice take selection can select only an approved candidate',
    );
  }
}

void _voiceTakeSelectionRequireExactCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3VoiceTakeSelectionRequestV1 request,
  required int slotRevision,
}) {
  _voiceTakeSelectionRequireBasis(base, request: request);
  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 Voice take selection expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 Voice take selection expected entities',
  );
  final slot = _authoringRequiredObject(
    entities[request.slotId],
    'revision-3 Voice take selection expected slot',
  );
  final payload = _authoringRequiredObject(
    slot['payload'],
    'revision-3 Voice take selection expected slot payload',
  );
  final data = _authoringRequiredObject(
    payload['data'],
    'revision-3 Voice take selection expected slot data',
  );
  final selected = request.selectedTakeId;
  if (selected == null) {
    data.remove('selected');
  } else {
    data['selected'] = <String, Object?>{
      'project_id': request.expectedProjectId,
      'id': selected,
      'expected_kind': 'voice_take',
    };
  }
  payload['data'] = data;
  slot['payload'] = payload;
  slot['revision'] = slotRevision;
  entities[request.slotId] = slot;
  expected['entities'] = entities;
  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'revision-3 Voice take selection candidate contains a non-exact project delta',
    );
  }
}
