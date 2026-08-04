part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3QuestOutlineV2RequestJsonBytes = 32 * 1024;

enum AuthoringRevision3QuestOutlineBuildStatus { blocked }

enum AuthoringRevision3QuestOutlineRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3QuestOutlinePublicationStatus { notSupported }

final class AuthoringRevision3QuestOutlineObjectiveEditV2 {
  const AuthoringRevision3QuestOutlineObjectiveEditV2({
    required this.slot,
    required this.title,
  });

  final int slot;
  final String title;

  Map<String, Object?> toJson() => <String, Object?>{
    'slot': slot,
    'title': title,
  };
}

/// Stable-slot-aware outline request for an exact semantic (generator-v4)
/// Quest. The full active slot permutation and transition-plan seal are
/// derived from the exact project; callers cannot invent either authority.
final class AuthoringRevision3QuestOutlineEditRequestV2 {
  const AuthoringRevision3QuestOutlineEditRequestV2._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.questId,
    required this.expectedQuestRevision,
    required this.moduleId,
    required this.expectedModuleRevision,
    required this.expectedTransitionPlanSeal,
    required this.displayName,
    required this.questTitle,
    required this.objectives,
  });

  factory AuthoringRevision3QuestOutlineEditRequestV2.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required AuthoringDraftContentSeal expectedTransitionPlanSeal,
    required String displayName,
    required String questTitle,
    required List<AuthoringRevision3QuestOutlineObjectiveEditV2> objectives,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3QuestOutlineEditRequestV2.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'quest_id': questId,
        'expected_quest_revision': expectedQuestRevision,
        'expected_script_module_id': expectedModuleId,
        'expected_script_module_revision': expectedModuleRevision,
        'expected_transition_plan_seal': _questTransitionsSealJson(
          expectedTransitionPlanSeal,
        ),
        'display_name': displayName,
        'quest_title': questTitle,
        'objectives': [for (final objective in objectives) objective.toJson()],
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
  final String moduleId;
  final int expectedModuleRevision;
  final AuthoringDraftContentSeal expectedTransitionPlanSeal;
  final String displayName;
  final String questTitle;
  final List<AuthoringRevision3QuestOutlineObjectiveEditV2> objectives;

  factory AuthoringRevision3QuestOutlineEditRequestV2.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'questOutlineRequestJsonV2',
        _maxAuthoringRevision3QuestOutlineV2RequestJsonBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Quest outline-v2 request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'quest_id',
      'expected_quest_revision',
      'expected_script_module_id',
      'expected_script_module_revision',
      'expected_transition_plan_seal',
      'display_name',
      'quest_title',
      'objectives',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Quest outline-v2 request',
    );
    _questOutlineRequireFieldOrder(request, fields, 'v2 request');
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 request is not canonical',
      );
    }
    final rawObjectives = request['objectives'];
    if (rawObjectives is! List<Object?> ||
        rawObjectives.isEmpty ||
        rawObjectives.length > _maxAuthoringRevision3QuestObjectives) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 objectives must keep 1 to 8 slots',
      );
    }
    final objectives = <AuthoringRevision3QuestOutlineObjectiveEditV2>[];
    final slots = <int>{};
    final foldedTitles = <String>{};
    var totalTitleBytes = 0;
    for (final raw in rawObjectives) {
      final objective = _authoringRequiredObject(
        raw,
        'revision-3 Quest outline-v2 objective',
      );
      _authoringExactFields(objective, const {
        'slot',
        'title',
      }, 'revision-3 Quest outline-v2 objective');
      _questOutlineRequireFieldOrder(objective, const [
        'slot',
        'title',
      ], 'v2 objective');
      final slot = _authoringRequiredInt(
        objective,
        'slot',
        min: 1,
        max: _maxRevision3QuestTransitionObjectiveSlot,
      );
      if (!slots.add(slot)) {
        throw const FormatException(
          'authoring revision-3 Quest outline-v2 objective slots are duplicated',
        );
      }
      final title = _questOutlineLiteral(objective, 'title');
      totalTitleBytes += utf8.encode(title).length;
      if (!foldedTitles.add(title.toLowerCase())) {
        throw const FormatException(
          'authoring revision-3 Quest outline-v2 objective titles are duplicated',
        );
      }
      objectives.add(
        AuthoringRevision3QuestOutlineObjectiveEditV2(slot: slot, title: title),
      );
    }
    if (totalTitleBytes > _maxAuthoringRevision3QuestObjectiveTitlesBytes) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 objective titles exceed the total limit',
      );
    }
    final parsed = AuthoringRevision3QuestOutlineEditRequestV2._(
      canonicalJson: value,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            request['expected_head'],
            'revision-3 Quest outline-v2 expected head',
          ),
        ),
      ),
      expectedProjectId: _questOutlineEntityId(request, 'expected_project_id'),
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(
        _questOutlineGeneration(
          request['expected_target'],
          'v2 request target',
        ).json,
      ),
      questId: _questOutlineEntityId(request, 'quest_id'),
      expectedQuestRevision: _authoringRequiredInt(
        request,
        'expected_quest_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      moduleId: _questOutlineEntityId(request, 'expected_script_module_id'),
      expectedModuleRevision: _authoringRequiredInt(
        request,
        'expected_script_module_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      expectedTransitionPlanSeal: AuthoringDraftContentSeal.fromJson(
        _authoringRequiredObject(
          request['expected_transition_plan_seal'],
          'revision-3 Quest outline-v2 expected plan seal',
        ),
      ),
      displayName: _questOutlineDisplayName(request, 'display_name'),
      questTitle: _questOutlineLiteral(request, 'quest_title'),
      objectives: List.unmodifiable(objectives),
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
    final basis = _questTransitionsRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    final outline = _questOutlineRequireBasisPair(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target']) ||
        expectedQuestRevision != basis.questRevision ||
        moduleId != basis.moduleId ||
        expectedModuleRevision != basis.moduleRevision ||
        !_questTransitionsSameSeal(
          expectedTransitionPlanSeal,
          basis.transitionPlan.contentSeal,
        ) ||
        outline.questRevision != basis.questRevision ||
        outline.moduleId != basis.moduleId ||
        outline.moduleRevision != basis.moduleRevision ||
        objectives.length != basis.objectives.length) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 request does not bind the exact semantic Quest',
      );
    }
    final active = basis.transitionPlan.objectiveSlots.toSet();
    final requested = objectives.map((objective) => objective.slot).toSet();
    if (requested.length != objectives.length ||
        requested.length != active.length ||
        !requested.containsAll(active)) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 request changed the active objective slots',
      );
    }
    final unchangedObjectives =
        objectives.length == basis.objectives.length &&
        List.generate(
          objectives.length,
          (index) =>
              objectives[index].slot == basis.objectives[index].slot &&
              objectives[index].title == basis.objectives[index].title,
        ).every((same) => same);
    if (displayName == outline.displayName &&
        questTitle == outline.title &&
        unchangedObjectives) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 request does not change the outline',
      );
    }
  }
}

/// Strict unpublished semantic outline candidate. The response is rechecked
/// against both the old and new transition-plan seals before it can enter the
/// managed head-CAS publication lane.
final class AuthoringRevision3QuestOutlineEditPreparationV2 {
  const AuthoringRevision3QuestOutlineEditPreparationV2._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
    required this.transitionPlanSeal,
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
  final AuthoringDraftContentSeal transitionPlanSeal;
  final AuthoringRevision3QuestOutlineBuildStatus buildStatus;
  final AuthoringRevision3QuestOutlineRuntimeStatus runtimeStatus;
  final AuthoringRevision3QuestOutlinePublicationStatus publicationStatus;

  factory AuthoringRevision3QuestOutlineEditPreparationV2.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV2 request,
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
      'transition_plan_seal',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Quest outline-v2 preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 response is not an unpublished preparation',
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
        'authoring revision-3 Quest outline-v2 response changed or did not advance its head',
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
    final transitionPlanSeal = AuthoringDraftContentSeal.fromJson(
      _authoringRequiredObject(
        json['transition_plan_seal'],
        'revision-3 Quest outline-v2 response plan seal',
      ),
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
        'authoring revision-3 Quest outline-v2 response identities or revisions disagree',
      );
    }
    _questOutlineV2RequireExactDelta(
      base.project,
      candidate.project,
      request: request,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
      transitionPlanSeal: transitionPlanSeal,
    );
    return AuthoringRevision3QuestOutlineEditPreparationV2._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      questId: questId,
      moduleId: moduleId,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
      transitionPlanSeal: transitionPlanSeal,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3QuestOutlineBuildStatus.blocked,
        _ => throw const FormatException(
          'authoring revision-3 Quest outline-v2 response grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3QuestOutlineRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 Quest outline-v2 response grants runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3QuestOutlinePublicationStatus.notSupported,
        _ => throw const FormatException(
          'authoring revision-3 Quest outline-v2 response grants native publication authority',
        ),
      },
    );
  }
}

void _questOutlineV2RequireExactDelta(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3QuestOutlineEditRequestV2 request,
  required int questRevision,
  required int moduleRevision,
  required AuthoringDraftContentSeal transitionPlanSeal,
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
        'authoring revision-3 Quest outline-v2 candidate changed basis field $field',
      );
    }
  }
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 Quest outline-v2 basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'revision-3 Quest outline-v2 candidate entities',
  );
  if (baseEntities.length != candidateEntities.length ||
      !_sameQuestOutlineStrings(
        baseEntities.keys.toList(growable: false),
        candidateEntities.keys.toList(growable: false),
      )) {
    throw const FormatException(
      'authoring revision-3 Quest outline-v2 candidate changed the entity set',
    );
  }
  for (final entry in baseEntities.entries) {
    if (entry.key != request.questId &&
        entry.key != request.moduleId &&
        !_authoringJsonDeepEquals(entry.value, candidateEntities[entry.key])) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 candidate changed an unrelated entity',
      );
    }
  }

  final baseBasis = _questTransitionsRequireBasis(
    base,
    projectId: request.expectedProjectId,
    questId: request.questId,
  );
  final candidateBasis = _questTransitionsRequireBasis(
    candidate,
    projectId: request.expectedProjectId,
    questId: request.questId,
  );
  final requestedSlots = request.objectives
      .map((objective) => objective.slot)
      .toList(growable: false);
  if (!_questTransitionsSameInts(
        candidateBasis.transitionPlan.objectiveSlots,
        baseBasis.transitionPlan.objectiveSlots,
      ) ||
      candidateBasis.transitionPlan.nextSlotOrdinal !=
          baseBasis.transitionPlan.nextSlotOrdinal ||
      !_questTransitionsSameInts(
        candidateBasis.transitionPlan.objectiveOrder,
        requestedSlots,
      ) ||
      !_questTransitionsSameSeal(
        candidateBasis.transitionPlan.contentSeal,
        transitionPlanSeal,
      ) ||
      candidateBasis.objectives.length != request.objectives.length) {
    throw const FormatException(
      'authoring revision-3 Quest outline-v2 candidate changed the stable-slot contract',
    );
  }
  for (var index = 0; index < request.objectives.length; index++) {
    final requested = request.objectives[index];
    final actual = candidateBasis.objectives[index];
    if (requested.slot != actual.slot || requested.title != actual.title) {
      throw const FormatException(
        'authoring revision-3 Quest outline-v2 candidate objective order or title disagrees',
      );
    }
  }
  final expectedPlan = Map<String, Object?>.from(
    jsonDecode(jsonEncode(baseBasis.transitionPlan.toJson())) as Map,
  )..['objective_order'] = requestedSlots;
  if (!_authoringJsonDeepEquals(
    candidateBasis.transitionPlan.toJson(),
    expectedPlan,
  )) {
    throw const FormatException(
      'authoring revision-3 Quest outline-v2 candidate changed transition behavior',
    );
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
    'revision-3 Quest outline-v2 candidate input',
  );
  if (candidateQuest.entity['revision'] != questRevision ||
      candidateQuest.entity['display_name'] != request.displayName ||
      candidateInput['title'] != request.questTitle) {
    throw const FormatException(
      'authoring revision-3 Quest outline-v2 candidate disagrees with the requested outline',
    );
  }
  final normalizedQuest = _questOutlineClone(candidateQuest.entity);
  final baseQuestObject = _authoringRequiredObject(
    baseEntities[request.questId],
    'revision-3 Quest outline-v2 basis Quest',
  );
  normalizedQuest['revision'] = baseQuestObject['revision'];
  normalizedQuest['display_name'] = baseQuestObject['display_name'];
  final normalizedQuestData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedQuest['payload'],
      'normalized outline-v2 Quest payload',
    )['data'],
    'normalized outline-v2 Quest data',
  );
  final normalizedInput = _questOutlineMutableObject(
    normalizedQuestData['input'],
    'normalized outline-v2 Quest input',
  );
  final baseInput = _authoringRequiredObject(
    baseQuest.data['input'],
    'revision-3 Quest outline-v2 basis input',
  );
  for (final field in const [
    'title',
    'objective_title',
    'additional_objective_titles',
    'transition_plan',
  ]) {
    if (baseInput.containsKey(field)) {
      normalizedInput[field] = baseInput[field];
    } else {
      normalizedInput.remove(field);
    }
  }
  if (!_authoringJsonDeepEquals(normalizedQuest, baseQuestObject)) {
    throw const FormatException(
      'authoring revision-3 Quest outline-v2 candidate changed a preserved Quest field',
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
      'authoring revision-3 Quest outline-v2 candidate module revision is not exact',
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
  final fingerprint = _authoringRequiredString(
    candidateModule.data,
    'input_fingerprint',
    maxBytes: 64,
  );
  if (!_authoringSha256Pattern.hasMatch(sourceSha) ||
      !_authoringSha256Pattern.hasMatch(fingerprint) ||
      crypto.sha256.convert(utf8.encode(source)).toString() != sourceSha ||
      _authoringRevision3QuestInputFingerprint(candidateInput) != fingerprint) {
    throw const FormatException(
      'authoring revision-3 Quest outline-v2 candidate module seals disagree',
    );
  }
  final normalizedModule = _questOutlineClone(candidateModule.entity);
  final baseModuleObject = _authoringRequiredObject(
    baseEntities[request.moduleId],
    'revision-3 Quest outline-v2 basis module',
  );
  normalizedModule['revision'] = baseModuleObject['revision'];
  final normalizedModuleData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedModule['payload'],
      'normalized outline-v2 module payload',
    )['data'],
    'normalized outline-v2 module data',
  );
  for (final field in const ['source', 'source_sha256', 'input_fingerprint']) {
    normalizedModuleData[field] = baseModule.data[field];
  }
  if (!_authoringJsonDeepEquals(normalizedModule, baseModuleObject)) {
    throw const FormatException(
      'authoring revision-3 Quest outline-v2 candidate changed a preserved module field',
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
  final hasTranscript = quest.data.containsKey('transcript');
  _authoringExactFields(quest.data, <String>{
    'generator_id',
    'generator_version',
    'input',
    'script_module',
    if (hasTranscript) 'transcript',
  }, 'revision-3 Quest outline Quest data');
  if (quest.data['generator_id'] != _authoringRevision3QuestGeneratorId ||
      _authoringRequiredInt(
            quest.data,
            'generator_version',
            min: _authoringRevision3QuestGeneratorVersion,
            max: _authoringRevision3QuestGeneratorVersion,
          ) !=
          _authoringRevision3QuestGeneratorVersion) {
    throw const FormatException(
      'authoring revision-3 Quest outlines require generator version 4',
    );
  }
  final scriptRef = _authoringRequiredObject(
    quest.data['script_module'],
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
  if (module.data['generator_id'] != _authoringRevision3QuestGeneratorId ||
      module.data['generator_version'] !=
          _authoringRevision3QuestGeneratorVersion) {
    throw const FormatException(
      'authoring revision-3 Quest outline has an unsupported generated script',
    );
  }
  final input = _authoringRequiredObject(
    quest.data['input'],
    'revision-3 Quest outline Quest input',
  );
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
    if (input.containsKey('additional_objective_titles'))
      'additional_objective_titles',
    'transition_plan',
    'collision_catalog',
  }, 'revision-3 Quest outline Quest input');
  if (input['quest_id'] != questId ||
      jsonEncode(input['target']) != jsonEncode(project['target'])) {
    throw const FormatException(
      'authoring revision-3 Quest outline Quest identity or target is not exact',
    );
  }
  final firstObjective = _questOutlineLiteral(input, 'objective_title');
  final additional = input.containsKey('additional_objective_titles')
      ? _authoringRevision3QuestObjectiveTitleList(
          input['additional_objective_titles'],
          firstTitle: firstObjective,
          requireAdditional: true,
          context: 'outline basis',
        )
      : const <String>[];
  final titles = <String>[firstObjective, ...additional];
  final plan = AuthoringRevision3QuestTransitionPlanV1.fromJson(
    input['transition_plan'],
  );
  if (plan.objectiveOrder.length != titles.length) {
    throw const FormatException(
      'authoring revision-3 Quest transition plan does not cover every outline title',
    );
  }
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
    objectiveTitles: List<String>.unmodifiable(titles),
  );
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
