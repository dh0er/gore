part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3QuestTranscriptBindings = 256;
const _maxAuthoringRevision3QuestTranscriptRequestBytes = 512 * 1024;
const _maxAuthoringRevision3QuestObjectiveSlot = 0xffff;

enum AuthoringRevision3QuestTranscriptMode { replace, createAndInsert }

/// One exact project-local DialogLine placement in a Quest transcript.
/// Objective slots are stable semantic slots, never presentation indexes.
final class AuthoringRevision3QuestTranscriptBindingV1 {
  AuthoringRevision3QuestTranscriptBindingV1({
    required String projectId,
    required String lineId,
    required this.objectiveSlot,
  }) : projectId = _dialogEntityId(projectId, 'transcript project ID'),
       lineId = _dialogEntityId(lineId, 'transcript DialogLine ID') {
    final slot = objectiveSlot;
    if (slot != null &&
        (slot < 1 || slot > _maxAuthoringRevision3QuestObjectiveSlot)) {
      throw const FormatException(
        'revision-3 Quest transcript objective slot is outside the unsigned wire domain',
      );
    }
  }

  final String projectId;
  final String lineId;
  final int? objectiveSlot;

  Map<String, Object?> _toJson() => <String, Object?>{
    'line': <String, Object?>{
      'project_id': projectId,
      'id': lineId,
      'expected_kind': 'dialog_line',
    },
    'objective_slot': objectiveSlot,
  };
}

sealed class AuthoringRevision3QuestTranscriptIntentV1 {
  const AuthoringRevision3QuestTranscriptIntentV1();

  AuthoringRevision3QuestTranscriptMode get mode;
  Map<String, Object?> _toJson();
}

final class AuthoringRevision3QuestTranscriptReplaceIntentV1
    extends AuthoringRevision3QuestTranscriptIntentV1 {
  AuthoringRevision3QuestTranscriptReplaceIntentV1({
    required List<AuthoringRevision3QuestTranscriptBindingV1> bindings,
  }) : bindings = List.unmodifiable(bindings) {
    if (bindings.length > _maxAuthoringRevision3QuestTranscriptBindings) {
      throw const FormatException(
        'revision-3 Quest transcript exceeds the 256-line limit',
      );
    }
    if (bindings.map((binding) => binding.lineId).toSet().length !=
        bindings.length) {
      throw const FormatException(
        'revision-3 Quest transcript contains a DialogLine more than once',
      );
    }
  }

  @override
  AuthoringRevision3QuestTranscriptMode get mode =>
      AuthoringRevision3QuestTranscriptMode.replace;

  final List<AuthoringRevision3QuestTranscriptBindingV1> bindings;

  @override
  Map<String, Object?> _toJson() => <String, Object?>{
    'mode': 'replace',
    'bindings': [for (final binding in bindings) binding._toJson()],
  };
}

final class AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1
    extends AuthoringRevision3QuestTranscriptIntentV1 {
  AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1({
    required this.index,
    required this.objectiveSlot,
    required this.line,
  }) {
    if (index < 0 || index > _maxAuthoringRevision3QuestTranscriptBindings) {
      throw const FormatException(
        'revision-3 Quest transcript insertion index is outside the unsigned wire domain',
      );
    }
    final slot = objectiveSlot;
    if (slot != null &&
        (slot < 1 || slot > _maxAuthoringRevision3QuestObjectiveSlot)) {
      throw const FormatException(
        'revision-3 Quest transcript objective slot is outside the unsigned wire domain',
      );
    }
  }

  @override
  AuthoringRevision3QuestTranscriptMode get mode =>
      AuthoringRevision3QuestTranscriptMode.createAndInsert;

  final int index;
  final int? objectiveSlot;
  final AuthoringRevision3DialogLineEntryRequestV1 line;

  @override
  Map<String, Object?> _toJson() => <String, Object?>{
    'mode': 'create_and_insert',
    'index': index,
    'objective_slot': objectiveSlot,
    'line': jsonDecode(line.canonicalJson),
  };
}

/// Canonical exact-current request for a prepare-only Quest transcript edit.
final class AuthoringRevision3QuestTranscriptRequestV1 {
  const AuthoringRevision3QuestTranscriptRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.questId,
    required this.expectedQuestRevision,
    required this.intent,
    required this.moduleId,
    required this.expectedModuleRevision,
  });

  factory AuthoringRevision3QuestTranscriptRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String questId,
    required int expectedQuestRevision,
    required AuthoringRevision3QuestTranscriptIntentV1 intent,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3QuestTranscriptRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'quest_id': questId,
        'expected_quest_revision': expectedQuestRevision,
        'intent': intent._toJson(),
      }),
      currentProjectJson: currentProjectJson,
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String questId;
  final int expectedQuestRevision;
  final AuthoringRevision3QuestTranscriptIntentV1 intent;
  final String moduleId;
  final int expectedModuleRevision;

  factory AuthoringRevision3QuestTranscriptRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'questTranscriptRequestJson',
        _maxAuthoringRevision3QuestTranscriptRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 Quest transcript request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Quest transcript request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'quest_id',
      'expected_quest_revision',
      'intent',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Quest transcript request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'Quest transcript request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 Quest transcript request is not canonical',
      );
    }

    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final projectId = _dialogEntityId(
      _authoringRequiredString(request, 'expected_project_id', maxBytes: 32),
      'transcript expected project ID',
    );
    final questId = _dialogEntityId(
      _authoringRequiredString(request, 'quest_id', maxBytes: 32),
      'transcript Quest ID',
    );
    final intent = _questTranscriptIntent(
      request['intent'],
      currentProjectJson: currentProjectJson,
    );
    final basis = _questTranscriptRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    final parsed = AuthoringRevision3QuestTranscriptRequestV1._(
      canonicalJson: value,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            request['expected_head'],
            'revision-3 Quest transcript expected head',
          ),
        ),
      ),
      expectedProjectId: projectId,
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(
        _authoringRevision3VoiceGeneration(
          request['expected_target'],
          'Quest transcript request target',
        ),
      ),
      questId: questId,
      expectedQuestRevision: _authoringRequiredInt(
        request,
        'expected_quest_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      intent: intent,
      moduleId: basis.moduleId,
      expectedModuleRevision: basis.moduleRevision,
    );
    parsed._requireExactProjectBinding(current);
    return parsed;
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    final basis = _questTranscriptRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target']) ||
        expectedQuestRevision != basis.questRevision ||
        moduleId != basis.moduleId ||
        expectedModuleRevision != basis.moduleRevision) {
      throw const FormatException(
        'revision-3 Quest transcript request does not bind the exact current Quest',
      );
    }
    final activeSlots = basis.objectiveSlots.toSet();
    switch (intent) {
      case AuthoringRevision3QuestTranscriptReplaceIntentV1(:final bindings):
        _questTranscriptRequireBindings(
          current.project,
          projectId: current.projectId,
          bindings: bindings,
          activeObjectiveSlots: activeSlots,
        );
        if (_questTranscriptSameBindings(bindings, basis.transcript)) {
          throw const FormatException(
            'revision-3 Quest transcript replacement does not change the transcript',
          );
        }
      case AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1(
        :final index,
        :final objectiveSlot,
        :final line,
      ):
        if (basis.transcript.length >=
                _maxAuthoringRevision3QuestTranscriptBindings ||
            index > basis.transcript.length ||
            basis.transcript.any((binding) => binding.lineId == line.lineId) ||
            !_questTranscriptObjectiveSlotIsAllowed(
              objectiveSlot,
              activeSlots,
            ) ||
            line.expectedHead.canonicalJson != expectedHead.canonicalJson ||
            line.expectedProjectId != expectedProjectId ||
            line.expectedRevision != expectedRevision ||
            line.expectedTargetCanonicalJson != expectedTargetCanonicalJson) {
          throw const FormatException(
            'revision-3 Quest transcript insertion is stale or conflicts with the exact Quest',
          );
        }
        line._requireExactProjectBinding(current);
    }
  }
}

/// Strict unpublished candidate suitable only for managed fixed-head CAS.
final class AuthoringRevision3QuestTranscriptPreparation {
  const AuthoringRevision3QuestTranscriptPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.questId,
    required this.questRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.mode,
    required this.transcriptCount,
    required this.createdLineId,
    required this.createdLocalizationId,
    required this.createdVoiceSlotId,
    required this.localizationAction,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.topicAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String questId;
  final int questRevision;
  final String moduleId;
  final int moduleRevision;
  final AuthoringRevision3QuestTranscriptMode mode;
  final int transcriptCount;
  final String? createdLineId;
  final String? createdLocalizationId;
  final String? createdVoiceSlotId;
  final AuthoringRevision3DialogLocalizationAction? localizationAction;
  final AuthoringRevision3DialogBuildStatus buildStatus;
  final AuthoringRevision3DialogRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogTopicAuthority topicAuthority;
  final AuthoringRevision3DialogPublicationStatus publicationStatus;

  factory AuthoringRevision3QuestTranscriptPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3QuestTranscriptRequestV1 request,
  }) {
    final basis = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(basis);
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'project_id',
      'revision',
      'quest_id',
      'quest_revision',
      'module_id',
      'module_revision',
      'mode',
      'transcript_count',
      'created_line_id',
      'created_localization_id',
      'created_voice_slot_id',
      'localization_action',
      'build_status',
      'runtime_status',
      'topic_authority',
      'publication_status',
    }, 'revision-3 Quest transcript preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 Quest transcript response is not an unpublished preparation',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson ||
        head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'revision-3 Quest transcript response has an invalid head transition',
      );
    }
    final projectJson = _authoringRevision3ResponseString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _dialogEntityId(
      _authoringRevision3ResponseString(json, 'project_id', maxBytes: 32),
      'transcript response project ID',
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final questId = _dialogEntityId(
      _authoringRevision3ResponseString(json, 'quest_id', maxBytes: 32),
      'transcript response Quest ID',
    );
    final questRevision = _authoringRequiredInt(
      json,
      'quest_revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final moduleId = _dialogEntityId(
      _authoringRevision3ResponseString(json, 'module_id', maxBytes: 32),
      'transcript response module ID',
    );
    final moduleRevision = _authoringRequiredInt(
      json,
      'module_revision',
      max: _maxAuthoringStoryAppliedRevision,
    );
    final mode = switch (json['mode']) {
      'replace' => AuthoringRevision3QuestTranscriptMode.replace,
      'create_and_insert' =>
        AuthoringRevision3QuestTranscriptMode.createAndInsert,
      _ => throw const FormatException(
        'revision-3 Quest transcript response has an invalid mode',
      ),
    };
    final transcriptCount = _authoringRequiredInt(
      json,
      'transcript_count',
      max: _maxAuthoringRevision3QuestTranscriptBindings,
    );
    final createdLineId = _questTranscriptNullableResponseId(
      json,
      'created_line_id',
    );
    final createdLocalizationId = _questTranscriptNullableResponseId(
      json,
      'created_localization_id',
    );
    final createdVoiceSlotId = _questTranscriptNullableResponseId(
      json,
      'created_voice_slot_id',
    );
    final localizationAction = switch (json['localization_action']) {
      null => null,
      'created' => AuthoringRevision3DialogLocalizationAction.created,
      'reused_exact' => AuthoringRevision3DialogLocalizationAction.reusedExact,
      _ => throw const FormatException(
        'revision-3 Quest transcript response has an invalid localization action',
      ),
    };
    if (projectId != basis.projectId ||
        projectId != candidate.projectId ||
        revision != basis.revision + 1 ||
        revision != candidate.revision ||
        questId != request.questId ||
        questRevision != request.expectedQuestRevision + 1 ||
        moduleId != request.moduleId ||
        moduleRevision != request.expectedModuleRevision ||
        mode != request.intent.mode) {
      throw const FormatException(
        'revision-3 Quest transcript response identities or revisions disagree',
      );
    }
    switch (request.intent) {
      case AuthoringRevision3QuestTranscriptReplaceIntentV1(:final bindings):
        if (transcriptCount != bindings.length ||
            createdLineId != null ||
            createdLocalizationId != null ||
            createdVoiceSlotId != null ||
            localizationAction != null) {
          throw const FormatException(
            'revision-3 Quest transcript replacement response created entities',
          );
        }
      case AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1(
        :final line,
      ):
        final expectedAction =
            line.localization
                is AuthoringRevision3DialogLocalizationCreateIntentV1
            ? AuthoringRevision3DialogLocalizationAction.created
            : AuthoringRevision3DialogLocalizationAction.reusedExact;
        if (transcriptCount !=
                _questTranscriptRequireBasis(
                      basis.project,
                      projectId: basis.projectId,
                      questId: request.questId,
                    ).transcript.length +
                    1 ||
            createdLineId != line.lineId ||
            createdLocalizationId != line.localization.localizationId ||
            createdVoiceSlotId != line.voiceSlot?.slotId ||
            localizationAction != expectedAction) {
          throw const FormatException(
            'revision-3 Quest transcript insertion response disagrees with its line request',
          );
        }
    }
    _questTranscriptRequireExactCandidate(
      basis.project,
      candidate.project,
      request: request,
    );
    return AuthoringRevision3QuestTranscriptPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      questId: questId,
      questRevision: questRevision,
      moduleId: moduleId,
      moduleRevision: moduleRevision,
      mode: mode,
      transcriptCount: transcriptCount,
      createdLineId: createdLineId,
      createdLocalizationId: createdLocalizationId,
      createdVoiceSlotId: createdVoiceSlotId,
      localizationAction: localizationAction,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3DialogBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 Quest transcript response grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 Quest transcript response grants runtime authority',
        ),
      },
      topicAuthority: switch (json['topic_authority']) {
        'not_granted' => AuthoringRevision3DialogTopicAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 Quest transcript response grants topic authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3DialogPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 Quest transcript response grants native publication authority',
        ),
      },
    );
  }
}

AuthoringRevision3QuestTranscriptIntentV1 _questTranscriptIntent(
  Object? value, {
  required String currentProjectJson,
}) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 Quest transcript intent',
  );
  switch (json['mode']) {
    case 'replace':
      const fields = <String>['mode', 'bindings'];
      _authoringExactFields(json, fields.toSet(), 'Quest transcript replace');
      _authoringRevision3VoiceRequireFieldOrder(
        json,
        fields,
        'Quest transcript replace',
      );
      final raw = json['bindings'];
      if (raw is! List<Object?> ||
          raw.length > _maxAuthoringRevision3QuestTranscriptBindings) {
        throw const FormatException(
          'revision-3 Quest transcript bindings are not bounded',
        );
      }
      return AuthoringRevision3QuestTranscriptReplaceIntentV1(
        bindings: [for (final binding in raw) _questTranscriptBinding(binding)],
      );
    case 'create_and_insert':
      const fields = <String>['mode', 'index', 'objective_slot', 'line'];
      _authoringExactFields(json, fields.toSet(), 'Quest transcript insertion');
      _authoringRevision3VoiceRequireFieldOrder(
        json,
        fields,
        'Quest transcript insertion',
      );
      final line = AuthoringRevision3DialogLineEntryRequestV1.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            json['line'],
            'revision-3 Quest transcript inserted line',
          ),
        ),
        currentProjectJson: currentProjectJson,
      );
      return AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1(
        index: _authoringRequiredInt(
          json,
          'index',
          max: _maxAuthoringRevision3QuestTranscriptBindings,
        ),
        objectiveSlot: _questTranscriptNullableSlot(
          json['objective_slot'],
          'Quest transcript insertion objective slot',
        ),
        line: line,
      );
    default:
      throw const FormatException(
        'revision-3 Quest transcript intent has an invalid mode',
      );
  }
}

AuthoringRevision3QuestTranscriptBindingV1 _questTranscriptBinding(
  Object? value,
) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 Quest transcript binding',
  );
  const fields = <String>['line', 'objective_slot'];
  _authoringExactFields(json, fields.toSet(), 'Quest transcript binding');
  _authoringRevision3VoiceRequireFieldOrder(
    json,
    fields,
    'Quest transcript binding',
  );
  final line = _authoringRequiredObject(
    json['line'],
    'revision-3 Quest transcript line reference',
  );
  const refFields = <String>['project_id', 'id', 'expected_kind'];
  _authoringExactFields(line, refFields.toSet(), 'Quest transcript line ref');
  _authoringRevision3VoiceRequireFieldOrder(
    line,
    refFields,
    'Quest transcript line ref',
  );
  if (line['expected_kind'] != 'dialog_line') {
    throw const FormatException(
      'revision-3 Quest transcript reference is not a DialogLine',
    );
  }
  return AuthoringRevision3QuestTranscriptBindingV1(
    projectId: _authoringRequiredString(line, 'project_id', maxBytes: 32),
    lineId: _authoringRequiredString(line, 'id', maxBytes: 32),
    objectiveSlot: _questTranscriptNullableSlot(
      json['objective_slot'],
      'Quest transcript objective slot',
    ),
  );
}

int? _questTranscriptNullableSlot(Object? value, String context) {
  if (value == null) return null;
  if (value is! int ||
      value < 1 ||
      value > _maxAuthoringRevision3QuestObjectiveSlot) {
    throw FormatException('$context is outside the unsigned wire domain');
  }
  return value;
}

String? _questTranscriptNullableResponseId(
  Map<String, Object?> json,
  String field,
) {
  final value = json[field];
  if (value == null) return null;
  return _dialogEntityId(
    _authoringRevision3ResponseString(json, field, maxBytes: 32),
    'transcript response $field',
  );
}

({
  int questRevision,
  String moduleId,
  int moduleRevision,
  Set<int> objectiveSlots,
  List<AuthoringRevision3QuestTranscriptBindingV1> transcript,
})
_questTranscriptRequireBasis(
  Map<String, Object?> project, {
  required String projectId,
  required String questId,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Quest transcript basis entities',
  );
  final quest = _questOutlineEntity(entities, questId, 'quest_draft');
  final data = quest.data;
  final allowed = <String>{
    'generator_id',
    'generator_version',
    'input',
    'script_module',
    if (data.containsKey('transcript')) 'transcript',
  };
  _authoringExactFields(
    data,
    allowed,
    'revision-3 Quest transcript Quest data',
  );
  if (data['generator_id'] != _authoringRevision3QuestGeneratorId) {
    throw const FormatException(
      'revision-3 Quest transcript uses an unsupported Quest generator',
    );
  }
  final generatorVersion = _authoringRequiredInt(
    data,
    'generator_version',
    min: _authoringRevision3QuestGeneratorVersion,
    max: _authoringRevision3QuestGeneratorVersion,
  );
  final moduleRef = _authoringRequiredObject(
    data['script_module'],
    'revision-3 Quest transcript module reference',
  );
  _authoringExactFields(moduleRef, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'revision-3 Quest transcript module reference');
  final moduleId = _dialogEntityId(
    _authoringRequiredString(moduleRef, 'id', maxBytes: 32),
    'transcript module ID',
  );
  if (moduleRef['project_id'] != projectId ||
      moduleRef['expected_kind'] != 'script_module') {
    throw const FormatException(
      'revision-3 Quest transcript has a foreign or mistyped module reference',
    );
  }
  final module = _questOutlineEntity(entities, moduleId, 'script_module');
  final input = _authoringRequiredObject(
    data['input'],
    'revision-3 Quest transcript Quest input',
  );
  final objectiveSlots = <int>{};
  if (generatorVersion == _authoringRevision3QuestGeneratorVersion) {
    if (!input.containsKey('transition_plan')) {
      throw const FormatException(
        'revision-3 semantic Quest transcript has no transition plan',
      );
    }
    objectiveSlots.addAll(
      AuthoringRevision3QuestTransitionPlanV1.fromJson(
        input['transition_plan'],
      ).objectiveSlots,
    );
  }
  final rawTranscript = data['transcript'];
  final transcript = <AuthoringRevision3QuestTranscriptBindingV1>[];
  if (rawTranscript != null) {
    if (rawTranscript is! List<Object?> ||
        rawTranscript.isEmpty ||
        rawTranscript.length > _maxAuthoringRevision3QuestTranscriptBindings) {
      throw const FormatException(
        'revision-3 Quest transcript basis is not canonical or bounded',
      );
    }
    transcript.addAll(rawTranscript.map(_questTranscriptBinding));
  }
  _questTranscriptRequireBindings(
    project,
    projectId: projectId,
    bindings: transcript,
    activeObjectiveSlots: objectiveSlots,
  );
  return (
    questRevision: _authoringRequiredInt(
      quest.entity,
      'revision',
      max: _maxAuthoringStoryBaseRevision,
    ),
    moduleId: moduleId,
    moduleRevision: _authoringRequiredInt(
      module.entity,
      'revision',
      max: _maxAuthoringStoryBaseRevision,
    ),
    objectiveSlots: Set<int>.unmodifiable(objectiveSlots),
    transcript: List.unmodifiable(transcript),
  );
}

void _questTranscriptRequireBindings(
  Map<String, Object?> project, {
  required String projectId,
  required List<AuthoringRevision3QuestTranscriptBindingV1> bindings,
  required Set<int> activeObjectiveSlots,
}) {
  if (bindings.length > _maxAuthoringRevision3QuestTranscriptBindings ||
      bindings.map((binding) => binding.lineId).toSet().length !=
          bindings.length) {
    throw const FormatException(
      'revision-3 Quest transcript bindings are duplicated or exceed the limit',
    );
  }
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Quest transcript entities',
  );
  for (final binding in bindings) {
    if (binding.projectId != projectId ||
        !_questTranscriptObjectiveSlotIsAllowed(
          binding.objectiveSlot,
          activeObjectiveSlots,
        )) {
      throw const FormatException(
        'revision-3 Quest transcript binding is foreign or has an inactive objective slot',
      );
    }
    _questOutlineEntity(entities, binding.lineId, 'dialog_line');
  }
}

bool _questTranscriptObjectiveSlotIsAllowed(
  int? slot,
  Set<int> activeObjectiveSlots,
) => slot == null || activeObjectiveSlots.contains(slot);

bool _questTranscriptSameBindings(
  List<AuthoringRevision3QuestTranscriptBindingV1> left,
  List<AuthoringRevision3QuestTranscriptBindingV1> right,
) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    final a = left[index];
    final b = right[index];
    if (a.projectId != b.projectId ||
        a.lineId != b.lineId ||
        a.objectiveSlot != b.objectiveSlot) {
      return false;
    }
  }
  return true;
}

void _questTranscriptRequireExactCandidate(
  Map<String, Object?> basis,
  Map<String, Object?> candidate, {
  required AuthoringRevision3QuestTranscriptRequestV1 request,
}) {
  final basisFacts = _questTranscriptRequireBasis(
    basis,
    projectId: request.expectedProjectId,
    questId: request.questId,
  );
  final candidateFacts = _questTranscriptRequireBasis(
    candidate,
    projectId: request.expectedProjectId,
    questId: request.questId,
  );
  final expectedBindings = switch (request.intent) {
    AuthoringRevision3QuestTranscriptReplaceIntentV1(:final bindings) =>
      bindings,
    AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1(
      :final index,
      :final objectiveSlot,
      :final line,
    ) =>
      <AuthoringRevision3QuestTranscriptBindingV1>[
        ...basisFacts.transcript.take(index),
        AuthoringRevision3QuestTranscriptBindingV1(
          projectId: request.expectedProjectId,
          lineId: line.lineId,
          objectiveSlot: objectiveSlot,
        ),
        ...basisFacts.transcript.skip(index),
      ],
  };
  if (candidateFacts.questRevision != basisFacts.questRevision + 1 ||
      candidateFacts.moduleId != basisFacts.moduleId ||
      candidateFacts.moduleRevision != basisFacts.moduleRevision ||
      !_questTranscriptSameBindings(
        candidateFacts.transcript,
        expectedBindings,
      )) {
    throw const FormatException(
      'revision-3 Quest transcript candidate disagrees with the exact requested transcript',
    );
  }

  final normalized = _authoringRevision3VoiceCloneObject(
    candidate,
    'revision-3 Quest transcript normalized candidate',
  );
  final normalizedEntities = _authoringRequiredObject(
    normalized['entities'],
    'revision-3 Quest transcript normalized entities',
  );
  final basisEntities = _authoringRequiredObject(
    basis['entities'],
    'revision-3 Quest transcript basis entities',
  );
  normalizedEntities[request.questId] = jsonDecode(
    jsonEncode(
      _authoringRequiredObject(
        basisEntities[request.questId],
        'revision-3 Quest transcript basis Quest',
      ),
    ),
  );
  normalized['entities'] = normalizedEntities;
  switch (request.intent) {
    case AuthoringRevision3QuestTranscriptReplaceIntentV1():
      normalized['revision'] = basis['revision'];
      if (!_authoringJsonDeepEquals(normalized, basis)) {
        throw const FormatException(
          'revision-3 Quest transcript replacement changed a preserved field',
        );
      }
    case AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1(:final line):
      _dialogRequireExactCandidate(basis, normalized, request: line);
  }
}
