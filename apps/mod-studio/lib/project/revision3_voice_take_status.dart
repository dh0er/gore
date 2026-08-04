part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceTakeStatusRequestBytes = 64 * 1024;
const _maxAuthoringRevision3VoiceTakeStatusUnchangedRevision =
    0x7fffffffffffffff;

enum AuthoringRevision3VoiceTakeStatusBuildStatus { blocked }

enum AuthoringRevision3VoiceTakeStatusRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3VoiceTakeStatusPublicationStatus { notSupported }

/// Exact-current intent to change only one retained VoiceTake review status.
///
/// The request is bound to the project, line, localization, slot, and take
/// revisions that were inspected by the caller. It carries no filesystem,
/// media, build, runtime, or publication authority.
final class AuthoringRevision3VoiceTakeStatusRequestV1 {
  const AuthoringRevision3VoiceTakeStatusRequestV1._({
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
    required this.expectedStatus,
    required this.desiredStatus,
  });

  factory AuthoringRevision3VoiceTakeStatusRequestV1.forProject({
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
    required AuthoringRevision3VoiceTakeStatus expectedStatus,
    required AuthoringRevision3VoiceTakeStatus desiredStatus,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3VoiceTakeStatusRequestV1.fromCanonicalJson(
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
        'expected_status': expectedStatus.wireName,
        'desired_status': desiredStatus.wireName,
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
  final AuthoringRevision3VoiceTakeStatus expectedStatus;
  final AuthoringRevision3VoiceTakeStatus desiredStatus;

  factory AuthoringRevision3VoiceTakeStatusRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'voiceTakeStatusRequestJson',
        _maxAuthoringRevision3VoiceTakeStatusRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 Voice take status request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Voice take status request',
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
      'expected_status',
      'desired_status',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Voice take status request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'take status request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 Voice take status request is not canonical',
      );
    }

    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Voice take status expected head',
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
        'revision-3 Voice take status entity IDs must be distinct',
      );
    }
    final locId = _authoringRevision3VoiceString(
      request,
      'expected_loc_id',
      maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
    );
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(locId)) {
      throw const FormatException(
        'revision-3 Voice take status LocID is not a safe archive basename stem',
      );
    }
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(request, 'locale', maxBytes: 35),
    );
    final expectedStatus = _authoringRevision3VoiceStatus(
      request['expected_status'],
    );
    final desiredStatus = _authoringRevision3VoiceStatus(
      request['desired_status'],
    );
    if (expectedStatus == desiredStatus) {
      throw const FormatException(
        'revision-3 Voice take status request does not change the take',
      );
    }

    final parsed = AuthoringRevision3VoiceTakeStatusRequestV1._(
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
          'take status request target',
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
        max: _maxAuthoringRevision3VoiceTakeStatusUnchangedRevision,
      ),
      takeId: takeId,
      expectedTakeRevision: _authoringRequiredInt(
        request,
        'expected_take_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      expectedStatus: expectedStatus,
      desiredStatus: desiredStatus,
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
        'revision-3 Voice take status request does not bind the exact current project',
      );
    }
    _voiceTakeStatusRequireBasis(current.project, request: this);
  }
}

/// Strict unpublished candidate whose sole semantic change is one take status.
final class AuthoringRevision3VoiceTakeStatusPreparation {
  const AuthoringRevision3VoiceTakeStatusPreparation._({
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
    required this.previousStatus,
    required this.status,
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
  final AuthoringRevision3VoiceTakeStatus previousStatus;
  final AuthoringRevision3VoiceTakeStatus status;
  final AuthoringRevision3VoiceTakeStatusBuildStatus buildStatus;
  final AuthoringRevision3VoiceTakeStatusRuntimeStatus runtimeStatus;
  final AuthoringRevision3VoiceTakeStatusPublicationStatus publicationStatus;

  factory AuthoringRevision3VoiceTakeStatusPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeStatusRequestV1 request,
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
      'localization_id',
      'slot_id',
      'slot_revision',
      'locale',
      'loc_id',
      'take_id',
      'take_revision',
      'previous_status',
      'status',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Voice take status preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 Voice take status response is not an unpublished preparation',
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
        'revision-3 Voice take status response has an invalid head transition',
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
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final previousStatus = _authoringRevision3VoiceStatus(
      json['previous_status'],
    );
    final status = _authoringRevision3VoiceStatus(json['status']);
    if (projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != base.revision + 1 ||
        revision != candidate.revision ||
        lineId != request.lineId ||
        localizationId != request.localizationId ||
        slotId != request.slotId ||
        slotRevision != request.expectedSlotRevision ||
        locale != request.locale ||
        locId != request.expectedLocId ||
        takeId != request.takeId ||
        takeRevision != request.expectedTakeRevision + 1 ||
        previousStatus != request.expectedStatus ||
        status != request.desiredStatus) {
      throw const FormatException(
        'revision-3 Voice take status response disagrees with its exact request',
      );
    }
    _voiceTakeStatusRequireExactCandidate(
      base.project,
      candidate.project,
      request: request,
      takeRevision: takeRevision,
    );
    return AuthoringRevision3VoiceTakeStatusPreparation._(
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
      previousStatus: previousStatus,
      status: status,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3VoiceTakeStatusBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 Voice take status response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3VoiceTakeStatusRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 Voice take status response grants unsupported runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3VoiceTakeStatusPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 Voice take status response grants unsupported publication authority',
        ),
      },
    );
  }
}

void _voiceTakeStatusRequireBasis(
  Map<String, Object?> project, {
  required AuthoringRevision3VoiceTakeStatusRequestV1 request,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Voice take status basis entities',
  );
  final line = _authoringRevision3VoiceEntity(
    entities,
    request.lineId,
    'dialog_line',
    'take status basis line',
  );
  _authoringRevision3VoiceExactOptionalFields(
    line.data,
    const {'localization', 'voice_slots'},
    const {'speaker_hint'},
    'take status basis DialogLine data',
  );
  final localizationRef = _authoringRevision3VoiceTypedRef(
    line.data['localization'],
    projectId: request.expectedProjectId,
    kind: 'localization_entry',
    context: 'take status line localization',
  );
  if (localizationRef.id != request.localizationId) {
    throw const FormatException(
      'revision-3 Voice take status localization differs from the exact line',
    );
  }
  final localization = _authoringRevision3VoiceEntity(
    entities,
    request.localizationId,
    'localization_entry',
    'take status basis localization',
  );
  _authoringExactFields(localization.data, const {
    'loc_id',
    'texts',
  }, 'revision-3 Voice take status LocalizationEntry data');
  final locId = _authoringRevision3VoiceString(
    localization.data,
    'loc_id',
    maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
  );
  if (locId != request.expectedLocId ||
      !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId)) {
    throw const FormatException(
      'revision-3 Voice take status localization identity disagrees',
    );
  }
  final voiceSlots = _authoringRequiredObject(
    line.data['voice_slots'],
    'revision-3 Voice take status line slots',
  );
  final slotRef = _authoringRevision3VoiceTypedRef(
    voiceSlots[request.locale],
    projectId: request.expectedProjectId,
    kind: 'voice_slot',
    context: 'take status line slot',
  );
  if (slotRef.id != request.slotId) {
    throw const FormatException(
      'revision-3 Voice take status slot differs from the exact line',
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
    'take status basis slot',
  );
  if (_authoringRevision3VoiceRevision(slot.entity) !=
      request.expectedSlotRevision) {
    throw const FormatException(
      'revision-3 Voice take status slot revision disagrees',
    );
  }
  final candidates = _authoringRevision3VoiceObjectList(
    slot.data['candidates'],
    'take status candidates',
  );
  var targetCandidateCount = 0;
  for (final candidate in candidates) {
    final ref = _authoringRevision3VoiceTypedRef(
      candidate,
      projectId: request.expectedProjectId,
      kind: 'voice_take',
      context: 'take status candidate',
    );
    if (ref.id == request.takeId) targetCandidateCount++;
  }
  if (targetCandidateCount != 1) {
    throw const FormatException(
      'revision-3 Voice take status take is not one exact slot candidate',
    );
  }

  var targetOwnerCount = 0;
  String? targetOwner;
  for (final entry in entities.entries) {
    final entity = _authoringRequiredObject(
      entry.value,
      'revision-3 Voice take status basis entity',
    );
    final payload = _authoringRequiredObject(
      entity['payload'],
      'revision-3 Voice take status basis payload',
    );
    if (payload['kind'] != 'voice_slot') continue;
    final data = _authoringRequiredObject(
      payload['data'],
      'revision-3 Voice take status basis VoiceSlot data',
    );
    final ownedCandidates = _authoringRevision3VoiceObjectList(
      data['candidates'],
      'take status ownership candidates',
    );
    for (final owned in ownedCandidates) {
      _authoringExactFields(owned, const {
        'project_id',
        'id',
        'expected_kind',
      }, 'revision-3 Voice take status ownership candidate');
      if (owned['project_id'] == request.expectedProjectId &&
          owned['expected_kind'] == 'voice_take' &&
          owned['id'] == request.takeId) {
        targetOwnerCount++;
        targetOwner = entry.key;
      }
    }
  }
  if (targetOwnerCount != 1 || targetOwner != request.slotId) {
    throw const FormatException(
      'revision-3 Voice take status take is shared by another slot',
    );
  }

  final take = _authoringRevision3VoiceEntity(
    entities,
    request.takeId,
    'voice_take',
    'take status basis take',
  );
  if (_authoringRevision3VoiceRevision(take.entity) !=
      request.expectedTakeRevision) {
    throw const FormatException(
      'revision-3 Voice take status take revision disagrees',
    );
  }
  if (take.data['locale'] != request.locale ||
      _authoringRevision3VoiceStatus(take.data['status']) !=
          request.expectedStatus) {
    throw const FormatException(
      'revision-3 Voice take status current take disagrees',
    );
  }
  final selectedValue = slot.data['selected'];
  final selectedTakeId = selectedValue == null
      ? null
      : _authoringRevision3VoiceTypedRef(
          selectedValue,
          projectId: request.expectedProjectId,
          kind: 'voice_take',
          context: 'take status current selection',
        ).id;
  if (selectedTakeId == request.takeId &&
      request.desiredStatus != AuthoringRevision3VoiceTakeStatus.approved) {
    throw const FormatException(
      'revision-3 Voice take status cannot make the selected take unapproved',
    );
  }
}

void _voiceTakeStatusRequireExactCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3VoiceTakeStatusRequestV1 request,
  required int takeRevision,
}) {
  _voiceTakeStatusRequireBasis(base, request: request);
  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 Voice take status expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 Voice take status expected entities',
  );
  final take = _authoringRequiredObject(
    entities[request.takeId],
    'revision-3 Voice take status expected take',
  );
  final payload = _authoringRequiredObject(
    take['payload'],
    'revision-3 Voice take status expected take payload',
  );
  final data = _authoringRequiredObject(
    payload['data'],
    'revision-3 Voice take status expected take data',
  );
  data['status'] = request.desiredStatus.wireName;
  payload['data'] = data;
  take['payload'] = payload;
  take['revision'] = takeRevision;
  entities[request.takeId] = take;
  expected['entities'] = entities;
  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'revision-3 Voice take status candidate contains a non-exact project delta',
    );
  }
}
