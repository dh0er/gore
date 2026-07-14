part of '../core/mod_ffi.dart';

/// Canonical request for a count-preserving edit of one exact-current Quest.
/// The module identity and all collision/source inputs are derived by native
/// code from the bound project and never cross this intent boundary.
final class AuthoringRevision3QuestOutlineEditRequestV1 {
  const AuthoringRevision3QuestOutlineEditRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.questId,
    required this.expectedQuestRevision,
    required this.displayName,
    required this.title,
    required this.objectiveTitles,
    required this.moduleId,
    required this.expectedModuleRevision,
  });

  factory AuthoringRevision3QuestOutlineEditRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String questId,
    required int expectedQuestRevision,
    required String displayName,
    required String title,
    required List<String> objectiveTitles,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3QuestOutlineEditRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'quest_id': questId,
        'expected_quest_revision': expectedQuestRevision,
        'display_name': displayName,
        'title': title,
        'objective_titles': objectiveTitles,
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
  final String displayName;
  final String title;
  final List<String> objectiveTitles;

  /// Derived from the exact bound Quest; deliberately absent from the wire.
  final String moduleId;
  final int expectedModuleRevision;

  factory AuthoringRevision3QuestOutlineEditRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'questOutlineRequestJson',
        _maxAuthoringRevision3QuestRequestJsonBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 Quest outline request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Quest outline request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'quest_id',
      'expected_quest_revision',
      'display_name',
      'title',
      'objective_titles',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Quest outline request',
    );
    _questOutlineRequireFieldOrder(request, fields, 'request');
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'authoring revision-3 Quest outline request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Quest outline expected head',
        ),
      ),
    );
    final expectedTarget = _questOutlineGeneration(
      request['expected_target'],
      'request target',
    );
    final objectiveTitles = _questOutlineObjectiveTitles(
      request['objective_titles'],
    );
    final parsed = AuthoringRevision3QuestOutlineEditRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: _questOutlineEntityId(request, 'expected_project_id'),
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(expectedTarget.json),
      questId: _questOutlineEntityId(request, 'quest_id'),
      expectedQuestRevision: _authoringRequiredInt(
        request,
        'expected_quest_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      displayName: _questOutlineDisplayName(request, 'display_name'),
      title: _questOutlineLiteral(request, 'title'),
      objectiveTitles: objectiveTitles,
      moduleId: '',
      expectedModuleRevision: 0,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final binding = _questOutlineRequireBasisPair(
      current.project,
      projectId: current.projectId,
      questId: parsed.questId,
    );
    final bound = AuthoringRevision3QuestOutlineEditRequestV1._(
      canonicalJson: parsed.canonicalJson,
      expectedHead: parsed.expectedHead,
      expectedProjectId: parsed.expectedProjectId,
      expectedRevision: parsed.expectedRevision,
      expectedTargetCanonicalJson: parsed.expectedTargetCanonicalJson,
      questId: parsed.questId,
      expectedQuestRevision: parsed.expectedQuestRevision,
      displayName: parsed.displayName,
      title: parsed.title,
      objectiveTitles: parsed.objectiveTitles,
      moduleId: binding.moduleId,
      expectedModuleRevision: binding.moduleRevision,
    );
    bound._requireExactProjectBinding(current);
    return bound;
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'authoring revision-3 Quest outline request does not bind the exact current project',
      );
    }
    final pair = _questOutlineRequireBasisPair(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    if (pair.questRevision != expectedQuestRevision ||
        pair.moduleId != moduleId ||
        pair.moduleRevision != expectedModuleRevision ||
        pair.objectiveTitles.length != objectiveTitles.length) {
      throw const FormatException(
        'authoring revision-3 Quest outline request does not bind the exact Quest/module pair',
      );
    }
    if (displayName == pair.displayName &&
        title == pair.title &&
        _sameQuestOutlineStrings(objectiveTitles, pair.objectiveTitles)) {
      throw const FormatException(
        'authoring revision-3 Quest outline request does not change the outline',
      );
    }
  }
}

enum AuthoringRevision3QuestOutlineBuildStatus { blocked }

enum AuthoringRevision3QuestOutlineRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3QuestOutlinePublicationStatus { notSupported }

/// Strict unpublished candidate. Besides the project revision, exactly one
/// Quest outline and its deterministic owned ScriptModule may change.
final class AuthoringRevision3QuestOutlineEditPreparation {
  const AuthoringRevision3QuestOutlineEditPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
  final AuthoringRevision3QuestOutlineBuildStatus buildStatus;
  final AuthoringRevision3QuestOutlineRuntimeStatus runtimeStatus;
  final AuthoringRevision3QuestOutlinePublicationStatus publicationStatus;

  factory AuthoringRevision3QuestOutlineEditPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV1 request,
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
      'quest_id',
      'module_id',
      'quest_revision',
      'module_revision',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Quest outline preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 Quest outline response is not an unpublished preparation',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Quest outline response changed its basis head',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Quest outline candidate did not advance its head',
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
    final projectId = _questOutlineEntityId(json, 'project_id');
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final questId = _questOutlineEntityId(json, 'quest_id');
    final moduleId = _questOutlineEntityId(json, 'module_id');
    final questRevision = _authoringRequiredInt(
      json,
      'quest_revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final moduleRevision = _authoringRequiredInt(
      json,
      'module_revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    if (projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != base.revision + 1 ||
        revision != candidate.revision ||
        questId != request.questId ||
        moduleId != request.moduleId ||
        questRevision != request.expectedQuestRevision + 1 ||
        moduleRevision != request.expectedModuleRevision + 1) {
      throw const FormatException(
        'authoring revision-3 Quest outline response identity or revisions disagree with the request',
      );
    }
    _questOutlineRequireExactDelta(
      base.project,
      candidate.project,
      request: request,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
    );
    return AuthoringRevision3QuestOutlineEditPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      questId: questId,
      moduleId: moduleId,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3QuestOutlineBuildStatus.blocked,
        _ => throw const FormatException(
          'authoring revision-3 Quest outline response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3QuestOutlineRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 Quest outline response grants unsupported runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3QuestOutlinePublicationStatus.notSupported,
        _ => throw const FormatException(
          'authoring revision-3 Quest outline response grants unsupported native publication authority',
        ),
      },
    );
  }
}

({
  int questRevision,
  String moduleId,
  int moduleRevision,
  String displayName,
  String title,
  List<String> objectiveTitles,
})
_questOutlineRequireBasisPair(
  Map<String, Object?> project, {
  required String projectId,
  required String questId,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Quest outline basis entities',
  );
  final quest = _questOutlineEntity(entities, questId, 'quest_draft');
  final questData = quest.data;
  _authoringExactFields(questData, const {
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  }, 'revision-3 Quest outline Quest data');
  final scriptRef = _authoringRequiredObject(
    questData['script_module'],
    'revision-3 Quest outline script reference',
  );
  _authoringExactFields(scriptRef, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'revision-3 Quest outline script reference');
  final moduleId = _questOutlineEntityId(scriptRef, 'id');
  if (scriptRef['project_id'] != projectId ||
      scriptRef['expected_kind'] != 'script_module') {
    throw const FormatException(
      'authoring revision-3 Quest outline has a foreign or mistyped module reference',
    );
  }
  final module = _questOutlineEntity(entities, moduleId, 'script_module');
  final input = _authoringRequiredObject(
    questData['input'],
    'revision-3 Quest outline Quest input',
  );
  final hasAdditional = input.containsKey('additional_objective_titles');
  _authoringExactFields(input, <String>{
    'target',
    'quest_id',
    'module_namespace',
    'technical_id',
    'text_helper',
    'parent_quest',
    'giver',
    'title',
    'description',
    'objective_title',
    if (hasAdditional) 'additional_objective_titles',
    'collision_catalog',
  }, 'revision-3 Quest outline Quest input');
  if (input['quest_id'] != questId ||
      jsonEncode(input['target']) != jsonEncode(project['target'])) {
    throw const FormatException(
      'authoring revision-3 Quest outline Quest identity or target is not exact',
    );
  }
  final firstObjective = _questOutlineLiteral(input, 'objective_title');
  final additional = hasAdditional
      ? _authoringRevision3QuestObjectiveTitleList(
          input['additional_objective_titles'],
          firstTitle: firstObjective,
          requireAdditional: true,
          context: 'outline basis',
        )
      : const <String>[];
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
    displayName: _questOutlineDisplayName(quest.entity, 'display_name'),
    title: _questOutlineLiteral(input, 'title'),
    objectiveTitles: List<String>.unmodifiable([firstObjective, ...additional]),
  );
}

void _questOutlineRequireExactDelta(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3QuestOutlineEditRequestV1 request,
  required int questRevision,
  required int moduleRevision,
}) {
  for (final field in const <String>[
    'format',
    'schema_revision',
    'project_id',
    'meta',
    'target',
    'authoring_locales',
    'asset_store',
  ]) {
    if (!_authoringJsonDeepEquals(base[field], candidate[field])) {
      throw FormatException(
        'authoring revision-3 Quest outline candidate changed basis field $field',
      );
    }
  }
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 Quest outline basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'revision-3 Quest outline candidate entities',
  );
  if (baseEntities.length != candidateEntities.length ||
      !_sameQuestOutlineStrings(
        baseEntities.keys.toList(growable: false),
        candidateEntities.keys.toList(growable: false),
      )) {
    throw const FormatException(
      'authoring revision-3 Quest outline candidate changed the entity set',
    );
  }
  for (final entry in baseEntities.entries) {
    if (entry.key != request.questId &&
        entry.key != request.moduleId &&
        !_authoringJsonDeepEquals(entry.value, candidateEntities[entry.key])) {
      throw const FormatException(
        'authoring revision-3 Quest outline candidate changed an unrelated entity',
      );
    }
  }

  final baseQuest = _questOutlineEntity(
    baseEntities,
    request.questId,
    'quest_draft',
  );
  final candidateQuest = _questOutlineEntity(
    candidateEntities,
    request.questId,
    'quest_draft',
  );
  final candidateInput = _authoringRequiredObject(
    candidateQuest.data['input'],
    'revision-3 Quest outline candidate input',
  );
  final candidateObjectives = <String>[
    _questOutlineLiteral(candidateInput, 'objective_title'),
    ...(candidateInput.containsKey('additional_objective_titles')
        ? _authoringRevision3QuestObjectiveTitleList(
            candidateInput['additional_objective_titles'],
            firstTitle: _questOutlineLiteral(candidateInput, 'objective_title'),
            requireAdditional: true,
            context: 'outline candidate',
          )
        : const <String>[]),
  ];
  if (candidateQuest.entity['revision'] != questRevision ||
      candidateQuest.entity['display_name'] != request.displayName ||
      candidateInput['title'] != request.title ||
      !_sameQuestOutlineStrings(candidateObjectives, request.objectiveTitles)) {
    throw const FormatException(
      'authoring revision-3 Quest outline candidate disagrees with the requested outline',
    );
  }
  final normalizedQuest = _questOutlineClone(candidateQuest.entity);
  final baseQuestObject = _authoringRequiredObject(
    baseEntities[request.questId],
    'revision-3 Quest outline basis Quest',
  );
  normalizedQuest['revision'] = baseQuestObject['revision'];
  normalizedQuest['display_name'] = baseQuestObject['display_name'];
  final normalizedQuestData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedQuest['payload'],
      'normalized Quest payload',
    )['data'],
    'normalized Quest data',
  );
  final normalizedInput = _questOutlineMutableObject(
    normalizedQuestData['input'],
    'normalized Quest input',
  );
  final baseInput = _authoringRequiredObject(
    baseQuest.data['input'],
    'basis Quest input',
  );
  normalizedInput['title'] = baseInput['title'];
  normalizedInput['objective_title'] = baseInput['objective_title'];
  if (baseInput.containsKey('additional_objective_titles')) {
    normalizedInput['additional_objective_titles'] =
        baseInput['additional_objective_titles'];
  } else {
    normalizedInput.remove('additional_objective_titles');
  }
  if (!_authoringJsonDeepEquals(normalizedQuest, baseQuestObject)) {
    throw const FormatException(
      'authoring revision-3 Quest outline candidate changed a non-outline Quest field',
    );
  }

  final baseModule = _questOutlineEntity(
    baseEntities,
    request.moduleId,
    'script_module',
  );
  final candidateModule = _questOutlineEntity(
    candidateEntities,
    request.moduleId,
    'script_module',
  );
  if (candidateModule.entity['revision'] != moduleRevision) {
    throw const FormatException(
      'authoring revision-3 Quest outline candidate module revision is not exact',
    );
  }
  final source = _authoringRequiredString(
    candidateModule.data,
    'source',
    maxBytes: _maxAuthoringDraftSourceBytes,
  );
  final sourceSha = _authoringRequiredString(
    candidateModule.data,
    'source_sha256',
    maxBytes: 64,
  );
  final inputFingerprint = _authoringRequiredString(
    candidateModule.data,
    'input_fingerprint',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(sourceSha) ||
      !_authoringSha256Pattern.hasMatch(inputFingerprint) ||
      crypto.sha256.convert(utf8.encode(source)).toString() != sourceSha ||
      _authoringRevision3QuestInputFingerprint(candidateInput) !=
          inputFingerprint) {
    throw const FormatException(
      'authoring revision-3 Quest outline candidate module seals disagree',
    );
  }
  final normalizedModule = _questOutlineClone(candidateModule.entity);
  final baseModuleObject = _authoringRequiredObject(
    baseEntities[request.moduleId],
    'revision-3 Quest outline basis module',
  );
  normalizedModule['revision'] = baseModuleObject['revision'];
  final normalizedModuleData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedModule['payload'],
      'normalized module payload',
    )['data'],
    'normalized module data',
  );
  for (final field in const <String>[
    'source',
    'source_sha256',
    'input_fingerprint',
  ]) {
    normalizedModuleData[field] = baseModule.data[field];
  }
  if (!_authoringJsonDeepEquals(normalizedModule, baseModuleObject)) {
    throw const FormatException(
      'authoring revision-3 Quest outline candidate changed a non-generated module field',
    );
  }
}

({Map<String, Object?> entity, Map<String, Object?> data}) _questOutlineEntity(
  Map<String, Object?> entities,
  String id,
  String kind,
) {
  final entity = _authoringRequiredObject(
    entities[id],
    'revision-3 Quest outline $kind entity',
  );
  _authoringExactFields(entity, const {
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, 'revision-3 Quest outline $kind entity');
  if (entity['id'] != id) {
    throw FormatException(
      'authoring revision-3 Quest outline $kind entity key and ID disagree',
    );
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 Quest outline $kind payload',
  );
  _authoringExactFields(payload, const {
    'kind',
    'data',
  }, 'revision-3 Quest outline $kind payload');
  if (payload['kind'] != kind) {
    throw FormatException(
      'authoring revision-3 Quest outline entity is not a $kind',
    );
  }
  return (
    entity: entity,
    data: _authoringRequiredObject(
      payload['data'],
      'revision-3 Quest outline $kind data',
    ),
  );
}

({Map<String, Object?> json, int byteLength, String sha256})
_questOutlineGeneration(Object? value, String context) {
  final generation = _authoringRequiredObject(
    value,
    'revision-3 Quest outline $context',
  );
  _authoringExactFields(generation, const {
    'executable',
  }, 'revision-3 Quest outline $context');
  final executable = _authoringRequiredObject(
    generation['executable'],
    'revision-3 Quest outline $context executable',
  );
  _authoringExactFields(executable, const {
    'byte_len',
    'sha256',
  }, 'revision-3 Quest outline $context executable');
  final sha = _authoringRequiredString(executable, 'sha256', maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(sha)) {
    throw const FormatException(
      'authoring revision-3 Quest outline target SHA-256 is invalid',
    );
  }
  return (
    json: generation,
    byteLength: _authoringRequiredInt(
      executable,
      'byte_len',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    ),
    sha256: sha,
  );
}

List<String> _questOutlineObjectiveTitles(Object? value) {
  if (value is! List<Object?> || value.isEmpty || value.length > 8) {
    throw const FormatException(
      'authoring revision-3 Quest outline objective list must keep 1 to 8 items',
    );
  }
  final first = value.first;
  if (first is! String) {
    throw const FormatException(
      'authoring revision-3 Quest outline objective 1 is not text',
    );
  }
  _authoringRevision3QuestValidateObjectiveTitle(first, 'outline objective 1');
  final additional = _authoringRevision3QuestObjectiveTitleList(
    value.skip(1).toList(growable: false),
    firstTitle: first,
    requireAdditional: false,
    context: 'outline request',
  );
  return List<String>.unmodifiable([first, ...additional]);
}

String _questOutlineDisplayName(Map<String, Object?> json, String field) {
  final value = _authoringRevision3QuestRequestString(json, field);
  if (value.trim().isEmpty ||
      value.trim() != value ||
      utf8.encode(value).length > 256 ||
      value.runes.any(
        (rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f),
      )) {
    throw const FormatException(
      'authoring revision-3 Quest outline display name is invalid',
    );
  }
  return value;
}

String _questOutlineLiteral(Map<String, Object?> json, String field) {
  final value = _authoringRevision3QuestRequestString(json, field);
  _authoringRevision3QuestValidateObjectiveTitle(value, 'outline $field');
  return value;
}

String _questOutlineEntityId(Map<String, Object?> json, String field) =>
    _authoringRevision3QuestEntityId(json, field);

void _questOutlineRequireFieldOrder(
  Map<String, Object?> json,
  List<String> expected,
  String context,
) {
  final actual = json.keys.toList(growable: false);
  if (!_sameQuestOutlineStrings(actual, expected)) {
    throw FormatException(
      'authoring revision-3 Quest outline $context has non-canonical field order',
    );
  }
}

bool _sameQuestOutlineStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

Map<String, Object?> _questOutlineClone(Map<String, Object?> value) =>
    Map<String, Object?>.from(jsonDecode(jsonEncode(value)) as Map);

Map<String, Object?> _questOutlineMutableObject(Object? value, String context) {
  if (value is! Map) {
    throw FormatException(
      'authoring revision-3 Quest outline $context is not an object',
    );
  }
  return value.cast<String, Object?>();
}
