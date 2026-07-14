part of '../core/mod_ffi.dart';

const _maxRevision3QuestContextRequestJsonBytes = 32 * 1024;
const _maxRevision3QuestContextDescriptionBytes = 512;
const _maxRevision3QuestContextCatalogIdBytes = 256;

/// Exact-current private seed for the friendly existing-Quest context editor.
///
/// It is parsed inside the managed session from its already reopened canonical
/// project. Project JSON, selectors, source seals and generated source never
/// cross the UI boundary.
final class AuthoringRevision3QuestContextSeed {
  const AuthoringRevision3QuestContextSeed._({
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.questRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.description,
    required this.parentRuntimeClass,
    required this.parentCatalogLayer,
    required this.parentAuthoringSelector,
    required this.parentSourceSeal,
    required this.giverRuntimeUniqueName,
    required this.giverCatalogLayer,
    required this.giverAuthoringSelector,
    required this.giverSourceSeal,
  });

  factory AuthoringRevision3QuestContextSeed.forProject({
    required String currentProjectJson,
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _questContextRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    if (basis.questRevision != expectedQuestRevision ||
        basis.moduleId != expectedModuleId ||
        basis.moduleRevision != expectedModuleRevision ||
        basis.parentRuntimeClass != expectedParentRuntimeClass ||
        basis.giverRuntimeUniqueName != expectedGiverRuntimeUniqueName) {
      throw const FormatException(
        'revision-3 Quest context seed does not match the selected exact-current Quest',
      );
    }
    return AuthoringRevision3QuestContextSeed._(
      projectId: current.projectId,
      projectRevision: current.revision,
      questId: questId,
      questRevision: basis.questRevision,
      moduleId: basis.moduleId,
      moduleRevision: basis.moduleRevision,
      description: basis.description,
      parentRuntimeClass: basis.parentRuntimeClass,
      parentCatalogLayer: basis.parentCatalogLayer,
      parentAuthoringSelector: basis.parentAuthoringSelector,
      parentSourceSeal: basis.parentSourceSeal,
      giverRuntimeUniqueName: basis.giverRuntimeUniqueName,
      giverCatalogLayer: basis.giverCatalogLayer,
      giverAuthoringSelector: basis.giverAuthoringSelector,
      giverSourceSeal: basis.giverSourceSeal,
    );
  }

  final String projectId;
  final int projectRevision;
  final String questId;
  final int questRevision;
  final String moduleId;
  final int moduleRevision;
  final String description;

  /// Hidden join keys. UI surfaces must render catalog display labels only.
  final String parentRuntimeClass;
  final String parentCatalogLayer;
  final String parentAuthoringSelector;
  final AuthoringDraftContentSeal parentSourceSeal;
  final String giverRuntimeUniqueName;
  final String giverCatalogLayer;
  final String giverAuthoringSelector;
  final AuthoringDraftContentSeal giverSourceSeal;
}

/// Canonical, authority-minimal intent for one existing Quest context edit.
final class AuthoringRevision3QuestContextEditRequestV1 {
  const AuthoringRevision3QuestContextEditRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedStoryCatalogSeal,
    required this.questId,
    required this.expectedQuestRevision,
    required this.description,
    required this.parentCatalogId,
    required this.giverCatalogId,
    required this.moduleId,
    required this.expectedModuleRevision,
    required this.expectedParentRuntimeClass,
    required this.expectedParentCatalogLayer,
    required this.expectedParentAuthoringSelector,
    required this.expectedParentSourceSeal,
    required this.expectedGiverRuntimeUniqueName,
    required this.expectedGiverCatalogLayer,
    required this.expectedGiverAuthoringSelector,
    required this.expectedGiverSourceSeal,
  });

  factory AuthoringRevision3QuestContextEditRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required AuthoringDraftContentSeal expectedStoryCatalogSeal,
    required String questId,
    required int expectedQuestRevision,
    required String description,
    required String parentCatalogId,
    required String giverCatalogId,
    required String expectedParentRuntimeClass,
    required String expectedParentCatalogLayer,
    required String expectedParentAuthoringSelector,
    required AuthoringDraftContentSeal expectedParentSourceSeal,
    required String expectedGiverRuntimeUniqueName,
    required String expectedGiverCatalogLayer,
    required String expectedGiverAuthoringSelector,
    required AuthoringDraftContentSeal expectedGiverSourceSeal,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    return AuthoringRevision3QuestContextEditRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_story_catalog_seal': _questContextSealJson(
          expectedStoryCatalogSeal,
        ),
        'quest_id': questId,
        'expected_quest_revision': expectedQuestRevision,
        'description': description,
        'parent_catalog_id': parentCatalogId,
        'giver_catalog_id': giverCatalogId,
      }),
      currentProjectJson: currentProjectJson,
      expectedParentRuntimeClass: expectedParentRuntimeClass,
      expectedParentCatalogLayer: expectedParentCatalogLayer,
      expectedParentAuthoringSelector: expectedParentAuthoringSelector,
      expectedParentSourceSeal: expectedParentSourceSeal,
      expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
      expectedGiverCatalogLayer: expectedGiverCatalogLayer,
      expectedGiverAuthoringSelector: expectedGiverAuthoringSelector,
      expectedGiverSourceSeal: expectedGiverSourceSeal,
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final AuthoringDraftContentSeal expectedStoryCatalogSeal;
  final String questId;
  final int expectedQuestRevision;
  final String description;
  final String parentCatalogId;
  final String giverCatalogId;

  /// Derived from the exact bound Quest and deliberately absent from the wire.
  final String moduleId;
  final int expectedModuleRevision;
  final String expectedParentRuntimeClass;
  final String expectedParentCatalogLayer;
  final String expectedParentAuthoringSelector;
  final AuthoringDraftContentSeal expectedParentSourceSeal;
  final String expectedGiverRuntimeUniqueName;
  final String expectedGiverCatalogLayer;
  final String expectedGiverAuthoringSelector;
  final AuthoringDraftContentSeal expectedGiverSourceSeal;

  factory AuthoringRevision3QuestContextEditRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
    required String expectedParentRuntimeClass,
    required String expectedParentCatalogLayer,
    required String expectedParentAuthoringSelector,
    required AuthoringDraftContentSeal expectedParentSourceSeal,
    required String expectedGiverRuntimeUniqueName,
    required String expectedGiverCatalogLayer,
    required String expectedGiverAuthoringSelector,
    required AuthoringDraftContentSeal expectedGiverSourceSeal,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'questContextRequestJson',
        _maxRevision3QuestContextRequestJsonBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 Quest context request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Quest context request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_story_catalog_seal',
      'quest_id',
      'expected_quest_revision',
      'description',
      'parent_catalog_id',
      'giver_catalog_id',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Quest context request',
    );
    _questContextRequireFieldOrder(request, fields, 'request');
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'authoring revision-3 Quest context request is not canonical',
      );
    }
    final parsed = AuthoringRevision3QuestContextEditRequestV1._(
      canonicalJson: value,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            request['expected_head'],
            'revision-3 Quest context expected head',
          ),
        ),
      ),
      expectedProjectId: _questContextEntityId(request, 'expected_project_id'),
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      expectedStoryCatalogSeal: AuthoringDraftContentSeal.fromJson(
        _authoringRequiredObject(
          request['expected_story_catalog_seal'],
          'revision-3 Quest context Story catalog seal',
        ),
      ),
      questId: _questContextEntityId(request, 'quest_id'),
      expectedQuestRevision: _authoringRequiredInt(
        request,
        'expected_quest_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      description: _questContextDescription(request, 'description'),
      parentCatalogId: _questContextCatalogId(request, 'parent_catalog_id'),
      giverCatalogId: _questContextCatalogId(request, 'giver_catalog_id'),
      moduleId: '',
      expectedModuleRevision: 0,
      expectedParentRuntimeClass: expectedParentRuntimeClass,
      expectedParentCatalogLayer: expectedParentCatalogLayer,
      expectedParentAuthoringSelector: expectedParentAuthoringSelector,
      expectedParentSourceSeal: expectedParentSourceSeal,
      expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
      expectedGiverCatalogLayer: expectedGiverCatalogLayer,
      expectedGiverAuthoringSelector: expectedGiverAuthoringSelector,
      expectedGiverSourceSeal: expectedGiverSourceSeal,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _questContextRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: parsed.questId,
    );
    final bound = AuthoringRevision3QuestContextEditRequestV1._(
      canonicalJson: parsed.canonicalJson,
      expectedHead: parsed.expectedHead,
      expectedProjectId: parsed.expectedProjectId,
      expectedRevision: parsed.expectedRevision,
      expectedStoryCatalogSeal: parsed.expectedStoryCatalogSeal,
      questId: parsed.questId,
      expectedQuestRevision: parsed.expectedQuestRevision,
      description: parsed.description,
      parentCatalogId: parsed.parentCatalogId,
      giverCatalogId: parsed.giverCatalogId,
      moduleId: basis.moduleId,
      expectedModuleRevision: basis.moduleRevision,
      expectedParentRuntimeClass: parsed.expectedParentRuntimeClass,
      expectedParentCatalogLayer: parsed.expectedParentCatalogLayer,
      expectedParentAuthoringSelector: parsed.expectedParentAuthoringSelector,
      expectedParentSourceSeal: parsed.expectedParentSourceSeal,
      expectedGiverRuntimeUniqueName: parsed.expectedGiverRuntimeUniqueName,
      expectedGiverCatalogLayer: parsed.expectedGiverCatalogLayer,
      expectedGiverAuthoringSelector: parsed.expectedGiverAuthoringSelector,
      expectedGiverSourceSeal: parsed.expectedGiverSourceSeal,
    );
    bound._requireExactProjectBinding(current);
    return bound;
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision) {
      throw const FormatException(
        'authoring revision-3 Quest context request does not bind the exact current project',
      );
    }
    final basis = _questContextRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    if (basis.questRevision != expectedQuestRevision ||
        basis.moduleId != moduleId ||
        basis.moduleRevision != expectedModuleRevision) {
      throw const FormatException(
        'authoring revision-3 Quest context request does not bind the exact Quest/module pair',
      );
    }
  }
}

enum AuthoringRevision3QuestContextBuildStatus { blocked }

enum AuthoringRevision3QuestContextRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3QuestContextPublicationStatus { notSupported }

/// Strict unpublished context candidate. Only the selected Quest context and
/// its deterministic owned ScriptModule may change.
final class AuthoringRevision3QuestContextEditPreparation {
  const AuthoringRevision3QuestContextEditPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
    required this.storyCatalogSeal,
    required this.parentCatalogId,
    required this.giverCatalogId,
    required this.parentRuntimeClass,
    required this.parentCatalogLayer,
    required this.parentAuthoringSelector,
    required this.parentSourceSeal,
    required this.giverRuntimeUniqueName,
    required this.giverCatalogLayer,
    required this.giverAuthoringSelector,
    required this.giverSourceSeal,
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
  final AuthoringDraftContentSeal storyCatalogSeal;
  final String parentCatalogId;
  final String giverCatalogId;
  final String parentRuntimeClass;
  final String parentCatalogLayer;
  final String parentAuthoringSelector;
  final AuthoringDraftContentSeal parentSourceSeal;
  final String giverRuntimeUniqueName;
  final String giverCatalogLayer;
  final String giverAuthoringSelector;
  final AuthoringDraftContentSeal giverSourceSeal;
  final AuthoringRevision3QuestContextBuildStatus buildStatus;
  final AuthoringRevision3QuestContextRuntimeStatus runtimeStatus;
  final AuthoringRevision3QuestContextPublicationStatus publicationStatus;

  factory AuthoringRevision3QuestContextEditPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3QuestContextEditRequestV1 request,
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
      'story_catalog_seal',
      'parent_catalog_id',
      'giver_catalog_id',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Quest context preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 Quest context response is not an unpublished preparation',
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
        'authoring revision-3 Quest context response changed its basis head',
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
        'authoring revision-3 Quest context candidate did not advance its head',
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
    final projectId = _questContextEntityId(json, 'project_id');
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final questId = _questContextEntityId(json, 'quest_id');
    final moduleId = _questContextEntityId(json, 'module_id');
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
    final storyCatalogSeal = AuthoringDraftContentSeal.fromJson(
      _authoringRequiredObject(
        json['story_catalog_seal'],
        'revision-3 Quest context response Story catalog seal',
      ),
    );
    final parentCatalogId = _questContextCatalogId(json, 'parent_catalog_id');
    final giverCatalogId = _questContextCatalogId(json, 'giver_catalog_id');
    if (projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != base.revision + 1 ||
        revision != candidate.revision ||
        questId != request.questId ||
        moduleId != request.moduleId ||
        questRevision != request.expectedQuestRevision + 1 ||
        moduleRevision != request.expectedModuleRevision + 1 ||
        !_questContextSameSeal(
          storyCatalogSeal,
          request.expectedStoryCatalogSeal,
        ) ||
        parentCatalogId != request.parentCatalogId ||
        giverCatalogId != request.giverCatalogId) {
      throw const FormatException(
        'authoring revision-3 Quest context response identity or revisions disagree with the request',
      );
    }
    final changed = _questContextRequireExactDelta(
      base.project,
      candidate.project,
      request: request,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
    );
    if (changed.parentRuntimeClass != request.expectedParentRuntimeClass ||
        changed.parentCatalogLayer != request.expectedParentCatalogLayer ||
        changed.parentAuthoringSelector !=
            request.expectedParentAuthoringSelector ||
        !_questContextSameSeal(
          changed.parentSourceSeal,
          request.expectedParentSourceSeal,
        ) ||
        changed.giverRuntimeUniqueName !=
            request.expectedGiverRuntimeUniqueName ||
        changed.giverCatalogLayer != request.expectedGiverCatalogLayer ||
        changed.giverAuthoringSelector !=
            request.expectedGiverAuthoringSelector ||
        !_questContextSameSeal(
          changed.giverSourceSeal,
          request.expectedGiverSourceSeal,
        )) {
      throw const FormatException(
        'authoring revision-3 Quest context candidate changed a reviewed catalog binding',
      );
    }
    return AuthoringRevision3QuestContextEditPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      questId: questId,
      moduleId: moduleId,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
      storyCatalogSeal: storyCatalogSeal,
      parentCatalogId: parentCatalogId,
      giverCatalogId: giverCatalogId,
      parentRuntimeClass: changed.parentRuntimeClass,
      parentCatalogLayer: changed.parentCatalogLayer,
      parentAuthoringSelector: changed.parentAuthoringSelector,
      parentSourceSeal: changed.parentSourceSeal,
      giverRuntimeUniqueName: changed.giverRuntimeUniqueName,
      giverCatalogLayer: changed.giverCatalogLayer,
      giverAuthoringSelector: changed.giverAuthoringSelector,
      giverSourceSeal: changed.giverSourceSeal,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3QuestContextBuildStatus.blocked,
        _ => throw const FormatException(
          'authoring revision-3 Quest context response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3QuestContextRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 Quest context response grants unsupported runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3QuestContextPublicationStatus.notSupported,
        _ => throw const FormatException(
          'authoring revision-3 Quest context response grants unsupported native publication authority',
        ),
      },
    );
  }
}

({
  int questRevision,
  String moduleId,
  int moduleRevision,
  String description,
  String parentRuntimeClass,
  String parentCatalogLayer,
  String parentAuthoringSelector,
  AuthoringDraftContentSeal parentSourceSeal,
  String giverRuntimeUniqueName,
  String giverCatalogLayer,
  String giverAuthoringSelector,
  AuthoringDraftContentSeal giverSourceSeal,
})
_questContextRequireBasis(
  Map<String, Object?> project, {
  required String projectId,
  required String questId,
}) {
  final outline = _questOutlineRequireBasisPair(
    project,
    projectId: projectId,
    questId: questId,
    allowSemanticPlan: true,
  );
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Quest context basis entities',
  );
  final quest = _questOutlineEntity(entities, questId, 'quest_draft');
  final input = _authoringRequiredObject(
    quest.data['input'],
    'revision-3 Quest context basis input',
  );
  final description = _questContextDescription(input, 'description');
  final parent = _questContextResolvedValue(
    input['parent_quest'],
    projectTarget: project['target'],
    runtimeField: 'runtime_class',
    context: 'parent Quest',
  );
  final giver = _questContextResolvedValue(
    input['giver'],
    projectTarget: project['target'],
    runtimeField: 'runtime_unique_name',
    context: 'giver',
  );
  return (
    questRevision: outline.questRevision,
    moduleId: outline.moduleId,
    moduleRevision: outline.moduleRevision,
    description: description,
    parentRuntimeClass: parent.runtimeValue,
    parentCatalogLayer: parent.catalogLayer,
    parentAuthoringSelector: parent.authoringSelector,
    parentSourceSeal: parent.sourceSeal,
    giverRuntimeUniqueName: giver.runtimeValue,
    giverCatalogLayer: giver.catalogLayer,
    giverAuthoringSelector: giver.authoringSelector,
    giverSourceSeal: giver.sourceSeal,
  );
}

({
  String parentRuntimeClass,
  String parentCatalogLayer,
  String parentAuthoringSelector,
  AuthoringDraftContentSeal parentSourceSeal,
  String giverRuntimeUniqueName,
  String giverCatalogLayer,
  String giverAuthoringSelector,
  AuthoringDraftContentSeal giverSourceSeal,
})
_questContextRequireExactDelta(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3QuestContextEditRequestV1 request,
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
        'authoring revision-3 Quest context candidate changed basis field $field',
      );
    }
  }
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 Quest context basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'revision-3 Quest context candidate entities',
  );
  if (baseEntities.length != candidateEntities.length ||
      !_sameQuestOutlineStrings(
        baseEntities.keys.toList(growable: false),
        candidateEntities.keys.toList(growable: false),
      )) {
    throw const FormatException(
      'authoring revision-3 Quest context candidate changed the entity set',
    );
  }
  for (final entry in baseEntities.entries) {
    if (entry.key != request.questId &&
        entry.key != request.moduleId &&
        !_authoringJsonDeepEquals(entry.value, candidateEntities[entry.key])) {
      throw const FormatException(
        'authoring revision-3 Quest context candidate changed an unrelated entity',
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
  final baseInput = _authoringRequiredObject(
    baseQuest.data['input'],
    'revision-3 Quest context basis input',
  );
  final candidateInput = _authoringRequiredObject(
    candidateQuest.data['input'],
    'revision-3 Quest context candidate input',
  );
  final parent = _questContextResolvedValue(
    candidateInput['parent_quest'],
    projectTarget: candidate['target'],
    runtimeField: 'runtime_class',
    context: 'candidate parent Quest',
  );
  final giver = _questContextResolvedValue(
    candidateInput['giver'],
    projectTarget: candidate['target'],
    runtimeField: 'runtime_unique_name',
    context: 'candidate giver',
  );
  if (candidateQuest.entity['revision'] != questRevision ||
      candidateInput['description'] != request.description) {
    throw const FormatException(
      'authoring revision-3 Quest context candidate disagrees with the requested context',
    );
  }
  final contextChanged =
      baseInput['description'] != candidateInput['description'] ||
      !_authoringJsonDeepEquals(
        baseInput['parent_quest'],
        candidateInput['parent_quest'],
      ) ||
      !_authoringJsonDeepEquals(baseInput['giver'], candidateInput['giver']);
  if (!contextChanged) {
    throw const FormatException(
      'authoring revision-3 Quest context candidate made no context change',
    );
  }
  final normalizedQuest = _questOutlineClone(candidateQuest.entity);
  final baseQuestObject = _authoringRequiredObject(
    baseEntities[request.questId],
    'revision-3 Quest context basis Quest',
  );
  normalizedQuest['revision'] = baseQuestObject['revision'];
  final normalizedQuestData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedQuest['payload'],
      'normalized context Quest payload',
    )['data'],
    'normalized context Quest data',
  );
  final normalizedInput = _questOutlineMutableObject(
    normalizedQuestData['input'],
    'normalized context Quest input',
  );
  for (final field in const <String>['description', 'parent_quest', 'giver']) {
    normalizedInput[field] = baseInput[field];
  }
  if (!_authoringJsonDeepEquals(normalizedQuest, baseQuestObject)) {
    throw const FormatException(
      'authoring revision-3 Quest context candidate changed a non-context Quest field',
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
      'authoring revision-3 Quest context candidate module revision is not exact',
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
      'authoring revision-3 Quest context candidate module seals disagree',
    );
  }
  final normalizedModule = _questOutlineClone(candidateModule.entity);
  final baseModuleObject = _authoringRequiredObject(
    baseEntities[request.moduleId],
    'revision-3 Quest context basis module',
  );
  normalizedModule['revision'] = baseModuleObject['revision'];
  final normalizedModuleData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedModule['payload'],
      'normalized context module payload',
    )['data'],
    'normalized context module data',
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
      'authoring revision-3 Quest context candidate changed a non-generated module field',
    );
  }
  return (
    parentRuntimeClass: parent.runtimeValue,
    parentCatalogLayer: parent.catalogLayer,
    parentAuthoringSelector: parent.authoringSelector,
    parentSourceSeal: parent.sourceSeal,
    giverRuntimeUniqueName: giver.runtimeValue,
    giverCatalogLayer: giver.catalogLayer,
    giverAuthoringSelector: giver.authoringSelector,
    giverSourceSeal: giver.sourceSeal,
  );
}

({
  String runtimeValue,
  String catalogLayer,
  String authoringSelector,
  AuthoringDraftContentSeal sourceSeal,
})
_questContextResolvedValue(
  Object? value, {
  required Object? projectTarget,
  required String runtimeField,
  required String context,
}) {
  final resolved = _authoringRequiredObject(
    value,
    'revision-3 Quest context $context',
  );
  _authoringExactFields(resolved, <String>{
    'generation',
    'source_seal',
    'catalog_layer',
    'canonical_selector',
    runtimeField,
  }, 'revision-3 Quest context $context');
  if (!_authoringJsonDeepEquals(resolved['generation'], projectTarget)) {
    throw FormatException(
      'authoring revision-3 Quest context $context generation disagrees',
    );
  }
  final sourceSeal = AuthoringDraftContentSeal.fromJson(
    _authoringRequiredObject(
      resolved['source_seal'],
      'revision-3 Quest context $context source seal',
    ),
  );
  final catalogLayer = _authoringRequiredString(
    resolved,
    'catalog_layer',
    maxBytes: 1024,
  );
  final authoringSelector = _authoringRequiredString(
    resolved,
    'canonical_selector',
    maxBytes: 1024,
  );
  final runtimeValue = _authoringRequiredString(
    resolved,
    runtimeField,
    maxBytes: 1024,
  );
  return (
    runtimeValue: runtimeValue,
    catalogLayer: catalogLayer,
    authoringSelector: authoringSelector,
    sourceSeal: sourceSeal,
  );
}

String _questContextDescription(Map<String, Object?> json, String field) {
  final value = _authoringRequiredString(
    json,
    field,
    maxBytes: _maxRevision3QuestContextDescriptionBytes,
  );
  if (value.isEmpty ||
      value.trim() != value ||
      value.runes.any(
        (rune) => rune < 0x20 || rune > 0x7e || rune == 0x22 || rune == 0x5c,
      )) {
    throw const FormatException(
      'authoring revision-3 Quest context description is invalid',
    );
  }
  return value;
}

String _questContextCatalogId(Map<String, Object?> json, String field) {
  final value = _authoringRequiredString(
    json,
    field,
    maxBytes: _maxRevision3QuestContextCatalogIdBytes,
  );
  if (value.trim() != value ||
      value.runes.any((rune) => rune < 0x20 || rune == 0x7f)) {
    throw FormatException(
      'authoring revision-3 Quest context field $field is invalid',
    );
  }
  return value;
}

String _questContextEntityId(Map<String, Object?> json, String field) =>
    _authoringRevision3QuestEntityId(json, field);

Map<String, Object?> _questContextSealJson(AuthoringDraftContentSeal seal) =>
    <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};

bool _questContextSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

void _questContextRequireFieldOrder(
  Map<String, Object?> json,
  List<String> expected,
  String context,
) {
  final actual = json.keys.toList(growable: false);
  if (!_sameQuestOutlineStrings(actual, expected)) {
    throw FormatException(
      'authoring revision-3 Quest context $context has non-canonical field order',
    );
  }
}
