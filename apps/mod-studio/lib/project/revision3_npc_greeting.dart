part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3NpcGreetingBindings = 256;
const _maxAuthoringRevision3NpcGreetingRequestBytes = 512 * 1024;

enum AuthoringRevision3NpcGreetingMode { replace, createAndInsert }

/// One exact project-local DialogLine placement in an NPC greeting list.
final class AuthoringRevision3NpcGreetingBindingV1 {
  AuthoringRevision3NpcGreetingBindingV1({
    required String projectId,
    required String lineId,
  }) : projectId = _dialogEntityId(projectId, 'NPC greeting project ID'),
       lineId = _dialogEntityId(lineId, 'NPC greeting DialogLine ID');

  final String projectId;
  final String lineId;

  Map<String, Object?> _toJson() => <String, Object?>{
    'line': <String, Object?>{
      'project_id': projectId,
      'id': lineId,
      'expected_kind': 'dialog_line',
    },
  };
}

sealed class AuthoringRevision3NpcGreetingIntentV1 {
  const AuthoringRevision3NpcGreetingIntentV1();

  AuthoringRevision3NpcGreetingMode get mode;
  Map<String, Object?> _toJson();
}

final class AuthoringRevision3NpcGreetingReplaceIntentV1
    extends AuthoringRevision3NpcGreetingIntentV1 {
  AuthoringRevision3NpcGreetingReplaceIntentV1({
    required List<AuthoringRevision3NpcGreetingBindingV1> bindings,
  }) : bindings = List.unmodifiable(bindings) {
    if (bindings.length > _maxAuthoringRevision3NpcGreetingBindings) {
      throw const FormatException(
        'revision-3 NPC greeting list exceeds the 256-line limit',
      );
    }
    if (bindings.map((binding) => binding.lineId).toSet().length !=
        bindings.length) {
      throw const FormatException(
        'revision-3 NPC greeting list contains a DialogLine more than once',
      );
    }
  }

  @override
  AuthoringRevision3NpcGreetingMode get mode =>
      AuthoringRevision3NpcGreetingMode.replace;

  final List<AuthoringRevision3NpcGreetingBindingV1> bindings;

  @override
  Map<String, Object?> _toJson() => <String, Object?>{
    'mode': 'replace',
    'bindings': [for (final binding in bindings) binding._toJson()],
  };
}

final class AuthoringRevision3NpcGreetingCreateAndInsertIntentV1
    extends AuthoringRevision3NpcGreetingIntentV1 {
  AuthoringRevision3NpcGreetingCreateAndInsertIntentV1({
    required this.index,
    required this.line,
  }) {
    if (index < 0 || index > _maxAuthoringRevision3NpcGreetingBindings) {
      throw const FormatException(
        'revision-3 NPC greeting insertion index is outside its wire domain',
      );
    }
  }

  @override
  AuthoringRevision3NpcGreetingMode get mode =>
      AuthoringRevision3NpcGreetingMode.createAndInsert;

  final int index;
  final AuthoringRevision3DialogLineEntryRequestV1 line;

  @override
  Map<String, Object?> _toJson() => <String, Object?>{
    'mode': 'create_and_insert',
    'index': index,
    'line': jsonDecode(line.canonicalJson),
  };
}

/// Canonical exact-current request for a prepare-only NPC greeting edit.
final class AuthoringRevision3NpcGreetingRequestV1 {
  const AuthoringRevision3NpcGreetingRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.npcId,
    required this.expectedNpcRevision,
    required this.intent,
    required this.moduleId,
    required this.expectedModuleRevision,
    required this.expectedGreetingCount,
  });

  factory AuthoringRevision3NpcGreetingRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String npcId,
    required int expectedNpcRevision,
    required AuthoringRevision3NpcGreetingIntentV1 intent,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3NpcGreetingRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'npc_id': npcId,
        'expected_npc_revision': expectedNpcRevision,
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
  final String npcId;
  final int expectedNpcRevision;
  final AuthoringRevision3NpcGreetingIntentV1 intent;
  final String moduleId;
  final int expectedModuleRevision;
  final int expectedGreetingCount;

  factory AuthoringRevision3NpcGreetingRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'npcGreetingRequestJson',
        _maxAuthoringRevision3NpcGreetingRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 NPC greeting request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 NPC greeting request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'npc_id',
      'expected_npc_revision',
      'intent',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 NPC greeting request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'NPC greeting request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 NPC greeting request is not canonical',
      );
    }

    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final projectId = _dialogEntityId(
      _authoringRequiredString(request, 'expected_project_id', maxBytes: 32),
      'NPC greeting expected project ID',
    );
    final npcId = _dialogEntityId(
      _authoringRequiredString(request, 'npc_id', maxBytes: 32),
      'NPC greeting NPC ID',
    );
    final intent = _npcGreetingIntent(
      request['intent'],
      currentProjectJson: currentProjectJson,
    );
    final basis = _npcGreetingRequireBasis(
      current.project,
      projectId: current.projectId,
      npcId: npcId,
    );
    final parsed = AuthoringRevision3NpcGreetingRequestV1._(
      canonicalJson: value,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            request['expected_head'],
            'revision-3 NPC greeting expected head',
          ),
        ),
      ),
      expectedProjectId: projectId,
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringRevision3NpcBasisRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(
        _authoringRevision3VoiceGeneration(
          request['expected_target'],
          'NPC greeting request target',
        ),
      ),
      npcId: npcId,
      expectedNpcRevision: _authoringRequiredInt(
        request,
        'expected_npc_revision',
        max: _maxAuthoringRevision3NpcBasisRevision,
      ),
      intent: intent,
      moduleId: basis.moduleId,
      expectedModuleRevision: basis.moduleRevision,
      expectedGreetingCount: basis.greetings.length,
    );
    parsed._requireExactProjectBinding(current);
    return parsed;
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    final basis = _npcGreetingRequireBasis(
      current.project,
      projectId: current.projectId,
      npcId: npcId,
    );
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target']) ||
        expectedNpcRevision != basis.npcRevision ||
        moduleId != basis.moduleId ||
        expectedModuleRevision != basis.moduleRevision) {
      throw const FormatException(
        'revision-3 NPC greeting request does not bind the exact current NPC',
      );
    }
    switch (intent) {
      case AuthoringRevision3NpcGreetingReplaceIntentV1(:final bindings):
        _npcGreetingRequireBindings(
          current.project,
          projectId: current.projectId,
          bindings: bindings,
        );
        if (_npcGreetingSameBindings(bindings, basis.greetings)) {
          throw const FormatException(
            'revision-3 NPC greeting replacement does not change the list',
          );
        }
      case AuthoringRevision3NpcGreetingCreateAndInsertIntentV1(
        :final index,
        :final line,
      ):
        if (basis.greetings.length >=
                _maxAuthoringRevision3NpcGreetingBindings ||
            index > basis.greetings.length ||
            basis.greetings.any((binding) => binding.lineId == line.lineId) ||
            line.expectedHead.canonicalJson != expectedHead.canonicalJson ||
            line.expectedProjectId != expectedProjectId ||
            line.expectedRevision != expectedRevision ||
            line.expectedTargetCanonicalJson != expectedTargetCanonicalJson) {
          throw const FormatException(
            'revision-3 NPC greeting insertion is stale or conflicts with the exact NPC',
          );
        }
        line._requireExactProjectBinding(current);
    }
  }
}

/// Strict unpublished candidate suitable only for managed fixed-head CAS.
final class AuthoringRevision3NpcGreetingPreparation {
  const AuthoringRevision3NpcGreetingPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.npcId,
    required this.npcRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.mode,
    required this.greetingCount,
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
  final String npcId;
  final int npcRevision;
  final String moduleId;
  final int moduleRevision;
  final AuthoringRevision3NpcGreetingMode mode;
  final int greetingCount;
  final String? createdLineId;
  final String? createdLocalizationId;
  final String? createdVoiceSlotId;
  final AuthoringRevision3DialogLocalizationAction? localizationAction;
  final AuthoringRevision3DialogBuildStatus buildStatus;
  final AuthoringRevision3DialogRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogTopicAuthority topicAuthority;
  final AuthoringRevision3DialogPublicationStatus publicationStatus;

  factory AuthoringRevision3NpcGreetingPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3NpcGreetingRequestV1 request,
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
      'npc_id',
      'npc_revision',
      'module_id',
      'module_revision',
      'mode',
      'greeting_count',
      'created_line_id',
      'created_localization_id',
      'created_voice_slot_id',
      'localization_action',
      'build_status',
      'runtime_status',
      'topic_authority',
      'publication_status',
    }, 'revision-3 NPC greeting preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 NPC greeting response is not an unpublished preparation',
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
        'revision-3 NPC greeting response has an invalid head transition',
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
      'NPC greeting response project ID',
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringRevision3NpcAppliedRevision,
    );
    final npcId = _dialogEntityId(
      _authoringRevision3ResponseString(json, 'npc_id', maxBytes: 32),
      'NPC greeting response NPC ID',
    );
    final npcRevision = _authoringRequiredInt(
      json,
      'npc_revision',
      min: 1,
      max: _maxAuthoringRevision3NpcAppliedRevision,
    );
    final moduleId = _dialogEntityId(
      _authoringRevision3ResponseString(json, 'module_id', maxBytes: 32),
      'NPC greeting response module ID',
    );
    final moduleRevision = _authoringRequiredInt(
      json,
      'module_revision',
      max: _maxAuthoringRevision3NpcAppliedRevision,
    );
    final mode = switch (json['mode']) {
      'replace' => AuthoringRevision3NpcGreetingMode.replace,
      'create_and_insert' => AuthoringRevision3NpcGreetingMode.createAndInsert,
      _ => throw const FormatException(
        'revision-3 NPC greeting response has an invalid mode',
      ),
    };
    final greetingCount = _authoringRequiredInt(
      json,
      'greeting_count',
      max: _maxAuthoringRevision3NpcGreetingBindings,
    );
    final createdLineId = _npcGreetingNullableResponseId(
      json,
      'created_line_id',
    );
    final createdLocalizationId = _npcGreetingNullableResponseId(
      json,
      'created_localization_id',
    );
    final createdVoiceSlotId = _npcGreetingNullableResponseId(
      json,
      'created_voice_slot_id',
    );
    final localizationAction = switch (json['localization_action']) {
      null => null,
      'created' => AuthoringRevision3DialogLocalizationAction.created,
      'reused_exact' => AuthoringRevision3DialogLocalizationAction.reusedExact,
      _ => throw const FormatException(
        'revision-3 NPC greeting response has an invalid localization action',
      ),
    };
    if (projectId != basis.projectId ||
        projectId != candidate.projectId ||
        revision != basis.revision + 1 ||
        revision != candidate.revision ||
        npcId != request.npcId ||
        npcRevision != request.expectedNpcRevision + 1 ||
        moduleId != request.moduleId ||
        moduleRevision != request.expectedModuleRevision ||
        mode != request.intent.mode) {
      throw const FormatException(
        'revision-3 NPC greeting response identities or revisions disagree',
      );
    }
    switch (request.intent) {
      case AuthoringRevision3NpcGreetingReplaceIntentV1(:final bindings):
        if (greetingCount != bindings.length ||
            createdLineId != null ||
            createdLocalizationId != null ||
            createdVoiceSlotId != null ||
            localizationAction != null) {
          throw const FormatException(
            'revision-3 NPC greeting replacement response created entities',
          );
        }
      case AuthoringRevision3NpcGreetingCreateAndInsertIntentV1(:final line):
        final expectedAction =
            line.localization
                is AuthoringRevision3DialogLocalizationCreateIntentV1
            ? AuthoringRevision3DialogLocalizationAction.created
            : AuthoringRevision3DialogLocalizationAction.reusedExact;
        if (greetingCount !=
                _npcGreetingRequireBasis(
                      basis.project,
                      projectId: basis.projectId,
                      npcId: request.npcId,
                    ).greetings.length +
                    1 ||
            createdLineId != line.lineId ||
            createdLocalizationId != line.localization.localizationId ||
            createdVoiceSlotId != line.voiceSlot?.slotId ||
            localizationAction != expectedAction) {
          throw const FormatException(
            'revision-3 NPC greeting insertion response disagrees with its line request',
          );
        }
    }
    _npcGreetingRequireExactCandidate(
      basis.project,
      candidate.project,
      request: request,
    );
    return AuthoringRevision3NpcGreetingPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      npcId: npcId,
      npcRevision: npcRevision,
      moduleId: moduleId,
      moduleRevision: moduleRevision,
      mode: mode,
      greetingCount: greetingCount,
      createdLineId: createdLineId,
      createdLocalizationId: createdLocalizationId,
      createdVoiceSlotId: createdVoiceSlotId,
      localizationAction: localizationAction,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3DialogBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 NPC greeting response grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 NPC greeting response grants runtime authority',
        ),
      },
      topicAuthority: switch (json['topic_authority']) {
        'not_granted' => AuthoringRevision3DialogTopicAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 NPC greeting response grants topic authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3DialogPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 NPC greeting response grants native publication authority',
        ),
      },
    );
  }
}

AuthoringRevision3NpcGreetingIntentV1 _npcGreetingIntent(
  Object? value, {
  required String currentProjectJson,
}) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 NPC greeting intent',
  );
  switch (json['mode']) {
    case 'replace':
      const fields = <String>['mode', 'bindings'];
      _authoringExactFields(json, fields.toSet(), 'NPC greeting replace');
      _authoringRevision3VoiceRequireFieldOrder(
        json,
        fields,
        'NPC greeting replace',
      );
      final raw = json['bindings'];
      if (raw is! List<Object?> ||
          raw.length > _maxAuthoringRevision3NpcGreetingBindings) {
        throw const FormatException(
          'revision-3 NPC greeting bindings are not bounded',
        );
      }
      return AuthoringRevision3NpcGreetingReplaceIntentV1(
        bindings: [for (final binding in raw) _npcGreetingBinding(binding)],
      );
    case 'create_and_insert':
      const fields = <String>['mode', 'index', 'line'];
      _authoringExactFields(json, fields.toSet(), 'NPC greeting insertion');
      _authoringRevision3VoiceRequireFieldOrder(
        json,
        fields,
        'NPC greeting insertion',
      );
      final line = AuthoringRevision3DialogLineEntryRequestV1.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            json['line'],
            'revision-3 NPC greeting inserted line',
          ),
        ),
        currentProjectJson: currentProjectJson,
      );
      return AuthoringRevision3NpcGreetingCreateAndInsertIntentV1(
        index: _authoringRequiredInt(
          json,
          'index',
          max: _maxAuthoringRevision3NpcGreetingBindings,
        ),
        line: line,
      );
    default:
      throw const FormatException(
        'revision-3 NPC greeting intent has an invalid mode',
      );
  }
}

AuthoringRevision3NpcGreetingBindingV1 _npcGreetingBinding(Object? value) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 NPC greeting binding',
  );
  const fields = <String>['line'];
  _authoringExactFields(json, fields.toSet(), 'NPC greeting binding');
  _authoringRevision3VoiceRequireFieldOrder(
    json,
    fields,
    'NPC greeting binding',
  );
  final line = _authoringRequiredObject(
    json['line'],
    'revision-3 NPC greeting line reference',
  );
  const refFields = <String>['project_id', 'id', 'expected_kind'];
  _authoringExactFields(line, refFields.toSet(), 'NPC greeting line ref');
  _authoringRevision3VoiceRequireFieldOrder(
    line,
    refFields,
    'NPC greeting line ref',
  );
  if (line['expected_kind'] != 'dialog_line') {
    throw const FormatException(
      'revision-3 NPC greeting reference is not a DialogLine',
    );
  }
  return AuthoringRevision3NpcGreetingBindingV1(
    projectId: _authoringRequiredString(line, 'project_id', maxBytes: 32),
    lineId: _authoringRequiredString(line, 'id', maxBytes: 32),
  );
}

String? _npcGreetingNullableResponseId(
  Map<String, Object?> json,
  String field,
) {
  final value = json[field];
  if (value == null) return null;
  return _dialogEntityId(
    _authoringRevision3ResponseString(json, field, maxBytes: 32),
    'NPC greeting response $field',
  );
}

({
  int npcRevision,
  String moduleId,
  int moduleRevision,
  List<AuthoringRevision3NpcGreetingBindingV1> greetings,
})
_npcGreetingRequireBasis(
  Map<String, Object?> project, {
  required String projectId,
  required String npcId,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 NPC greeting basis entities',
  );
  final npc = _authoringRevision3NpcEntity(entities, npcId, 'npc_draft');
  final data = npc.data;
  _authoringExactFields(data, <String>{
    'generator_id',
    'generator_version',
    'input',
    'script_module',
    if (data.containsKey('greetings')) 'greetings',
  }, 'revision-3 NPC greeting NPC data');
  _authoringRevision3NpcRequireGenerator(data, 'greeting NPC data');
  final moduleRef = _authoringRequiredObject(
    data['script_module'],
    'revision-3 NPC greeting module reference',
  );
  _authoringExactFields(moduleRef, const <String>{
    'project_id',
    'id',
    'expected_kind',
  }, 'revision-3 NPC greeting module reference');
  final moduleId = _dialogEntityId(
    _authoringRequiredString(moduleRef, 'id', maxBytes: 32),
    'NPC greeting module ID',
  );
  if (moduleRef['project_id'] != projectId ||
      moduleRef['expected_kind'] != 'script_module') {
    throw const FormatException(
      'revision-3 NPC greeting has a foreign or mistyped module reference',
    );
  }
  final module = _authoringRevision3NpcEntity(
    entities,
    moduleId,
    'script_module',
  );
  final rawGreetings = data['greetings'];
  final greetings = <AuthoringRevision3NpcGreetingBindingV1>[];
  if (rawGreetings != null) {
    if (rawGreetings is! List<Object?> ||
        rawGreetings.isEmpty ||
        rawGreetings.length > _maxAuthoringRevision3NpcGreetingBindings) {
      throw const FormatException(
        'revision-3 NPC greeting basis is not canonical or bounded',
      );
    }
    greetings.addAll(rawGreetings.map(_npcGreetingBinding));
  }
  _npcGreetingRequireBindings(
    project,
    projectId: projectId,
    bindings: greetings,
  );
  return (
    npcRevision: _authoringRequiredInt(
      npc.entity,
      'revision',
      max: _maxAuthoringRevision3NpcBasisRevision,
    ),
    moduleId: moduleId,
    moduleRevision: _authoringRequiredInt(
      module.entity,
      'revision',
      max: _maxAuthoringRevision3NpcAppliedRevision,
    ),
    greetings: List.unmodifiable(greetings),
  );
}

void _npcGreetingRequireBindings(
  Map<String, Object?> project, {
  required String projectId,
  required List<AuthoringRevision3NpcGreetingBindingV1> bindings,
}) {
  if (bindings.length > _maxAuthoringRevision3NpcGreetingBindings ||
      bindings.map((binding) => binding.lineId).toSet().length !=
          bindings.length) {
    throw const FormatException(
      'revision-3 NPC greeting bindings are duplicated or exceed the limit',
    );
  }
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 NPC greeting entities',
  );
  for (final binding in bindings) {
    if (binding.projectId != projectId) {
      throw const FormatException(
        'revision-3 NPC greeting binding belongs to another project',
      );
    }
    _authoringRevision3NpcEntity(entities, binding.lineId, 'dialog_line');
  }
}

bool _npcGreetingSameBindings(
  List<AuthoringRevision3NpcGreetingBindingV1> left,
  List<AuthoringRevision3NpcGreetingBindingV1> right,
) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    final a = left[index];
    final b = right[index];
    if (a.projectId != b.projectId || a.lineId != b.lineId) return false;
  }
  return true;
}

void _npcGreetingRequireExactCandidate(
  Map<String, Object?> basis,
  Map<String, Object?> candidate, {
  required AuthoringRevision3NpcGreetingRequestV1 request,
}) {
  final basisFacts = _npcGreetingRequireBasis(
    basis,
    projectId: request.expectedProjectId,
    npcId: request.npcId,
  );
  final candidateFacts = _npcGreetingRequireBasis(
    candidate,
    projectId: request.expectedProjectId,
    npcId: request.npcId,
  );
  final expectedBindings = switch (request.intent) {
    AuthoringRevision3NpcGreetingReplaceIntentV1(:final bindings) => bindings,
    AuthoringRevision3NpcGreetingCreateAndInsertIntentV1(
      :final index,
      :final line,
    ) =>
      <AuthoringRevision3NpcGreetingBindingV1>[
        ...basisFacts.greetings.take(index),
        AuthoringRevision3NpcGreetingBindingV1(
          projectId: request.expectedProjectId,
          lineId: line.lineId,
        ),
        ...basisFacts.greetings.skip(index),
      ],
  };
  if (candidateFacts.npcRevision != basisFacts.npcRevision + 1 ||
      candidateFacts.moduleId != basisFacts.moduleId ||
      candidateFacts.moduleRevision != basisFacts.moduleRevision ||
      !_npcGreetingSameBindings(candidateFacts.greetings, expectedBindings)) {
    throw const FormatException(
      'revision-3 NPC greeting candidate disagrees with the exact requested list',
    );
  }

  final normalized = _authoringRevision3VoiceCloneObject(
    candidate,
    'revision-3 NPC greeting normalized candidate',
  );
  final normalizedEntities = _authoringRequiredObject(
    normalized['entities'],
    'revision-3 NPC greeting normalized entities',
  );
  final basisEntities = _authoringRequiredObject(
    basis['entities'],
    'revision-3 NPC greeting basis entities',
  );
  final basisNpc = _authoringRequiredObject(
    basisEntities[request.npcId],
    'revision-3 NPC greeting basis NPC',
  );
  final candidateNpc = _authoringRevision3VoiceCloneObject(
    _authoringRequiredObject(
      normalizedEntities[request.npcId],
      'revision-3 NPC greeting candidate NPC',
    ),
    'revision-3 NPC greeting normalized NPC',
  );
  candidateNpc['revision'] = basisNpc['revision'];
  final candidatePayload = _authoringRequiredObject(
    candidateNpc['payload'],
    'revision-3 NPC greeting candidate payload',
  );
  final candidateData = _authoringRequiredObject(
    candidatePayload['data'],
    'revision-3 NPC greeting candidate data',
  );
  final basisPayload = _authoringRequiredObject(
    basisNpc['payload'],
    'revision-3 NPC greeting basis payload',
  );
  final basisData = _authoringRequiredObject(
    basisPayload['data'],
    'revision-3 NPC greeting basis data',
  );
  if (basisData.containsKey('greetings')) {
    candidateData['greetings'] = jsonDecode(jsonEncode(basisData['greetings']));
  } else {
    candidateData.remove('greetings');
  }
  candidatePayload['data'] = candidateData;
  candidateNpc['payload'] = candidatePayload;
  if (!_authoringJsonDeepEquals(candidateNpc, basisNpc)) {
    throw const FormatException(
      'revision-3 NPC greeting candidate changed a preserved NPC field',
    );
  }
  normalizedEntities[request.npcId] = jsonDecode(jsonEncode(basisNpc));
  normalized['entities'] = normalizedEntities;
  switch (request.intent) {
    case AuthoringRevision3NpcGreetingReplaceIntentV1():
      normalized['revision'] = basis['revision'];
      if (!_authoringJsonDeepEquals(normalized, basis)) {
        throw const FormatException(
          'revision-3 NPC greeting replacement changed a preserved field',
        );
      }
    case AuthoringRevision3NpcGreetingCreateAndInsertIntentV1(:final line):
      _dialogRequireExactCandidate(basis, normalized, request: line);
  }
}
