part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3StoryDraftRemovalRequestBytes = 32 * 1024;

enum AuthoringRevision3StoryDraftRemovalBuildStatus { blocked }

enum AuthoringRevision3StoryDraftRemovalRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3StoryDraftRemovalArtifactAuthority { notGranted }

enum AuthoringRevision3StoryDraftRemovalNativePublicationStatus { notSupported }

/// Exact canonical request for removing one managed Story Draft and only its
/// uniquely-owned generated ScriptModule.
final class AuthoringRevision3StoryDraftRemovalRequestV1 {
  const AuthoringRevision3StoryDraftRemovalRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.draftId,
    required this.draftKind,
    required this.expectedDraftRevision,
    required this.scriptModuleId,
    required this.expectedScriptModuleRevision,
  });

  factory AuthoringRevision3StoryDraftRemovalRequestV1.forProject({
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String draftId,
    required AuthoringStoryDraftKind draftKind,
    required int expectedDraftRevision,
    required String scriptModuleId,
    required int expectedScriptModuleRevision,
  }) {
    final project = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    if (project.revision >= _maxAuthoringSignedJsonInteger) {
      throw const FormatException(
        'revision-3 Story Draft removal basis cannot advance its revision',
      );
    }
    final target = _authoringRequiredObject(
      project.project['target'],
      'revision-3 Story Draft removal project target',
    );
    return AuthoringRevision3StoryDraftRemovalRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': project.projectId,
        'expected_revision': project.revision,
        'expected_target': target,
        'draft_id': draftId,
        'draft_kind': draftKind.wireName,
        'expected_draft_revision': expectedDraftRevision,
        'script_module_id': scriptModuleId,
        'expected_script_module_revision': expectedScriptModuleRevision,
      }),
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String draftId;
  final AuthoringStoryDraftKind draftKind;
  final int expectedDraftRevision;
  final String scriptModuleId;
  final int expectedScriptModuleRevision;

  factory AuthoringRevision3StoryDraftRemovalRequestV1.fromCanonicalJson(
    String value,
  ) {
    try {
      _authoringRevision3RequestString(
        value,
        'storyDraftRemovalRequestJson',
        _maxAuthoringRevision3StoryDraftRemovalRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 Story Draft removal request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Story Draft removal request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'draft_id',
      'draft_kind',
      'expected_draft_revision',
      'script_module_id',
      'expected_script_module_revision',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Story Draft removal request',
    );
    final actualFields = request.keys.toList(growable: false);
    for (var index = 0; index < fields.length; index++) {
      if (actualFields[index] != fields[index]) {
        throw const FormatException(
          'revision-3 Story Draft removal request has non-canonical field order',
        );
      }
    }
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 Story Draft removal request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 Story Draft removal expected head',
        ),
      ),
    );
    final expectedProjectId = _authoringEntityId(
      _authoringRequiredString(request, 'expected_project_id', maxBytes: 32),
      'expected_project_id',
    );
    final draftId = _authoringEntityId(
      _authoringRequiredString(request, 'draft_id', maxBytes: 32),
      'draft_id',
    );
    final scriptModuleId = _authoringEntityId(
      _authoringRequiredString(request, 'script_module_id', maxBytes: 32),
      'script_module_id',
    );
    if (draftId == scriptModuleId) {
      throw const FormatException(
        'revision-3 Story Draft removal entity IDs must be distinct',
      );
    }
    final draftKind = switch (request['draft_kind']) {
      'npc_draft' => AuthoringStoryDraftKind.npcDraft,
      'quest_draft' => AuthoringStoryDraftKind.questDraft,
      _ => throw const FormatException(
        'revision-3 Story Draft removal kind is unsupported',
      ),
    };
    final expectedTarget = _authoringRequiredObject(
      request['expected_target'],
      'revision-3 Story Draft removal expected target',
    );
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      expectedTarget,
      'revision-3 Story Draft removal expected target',
    );
    return AuthoringRevision3StoryDraftRemovalRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: expectedProjectId,
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringSignedJsonInteger - 1,
      ),
      expectedTargetCanonicalJson: jsonEncode(expectedTarget),
      draftId: draftId,
      draftKind: draftKind,
      expectedDraftRevision: _authoringRequiredInt(
        request,
        'expected_draft_revision',
        max: _maxAuthoringSignedJsonInteger,
      ),
      scriptModuleId: scriptModuleId,
      expectedScriptModuleRevision: _authoringRequiredInt(
        request,
        'expected_script_module_revision',
        max: _maxAuthoringSignedJsonInteger,
      ),
    );
  }

  void _requireMatchesProject(String currentProjectJson) {
    final project = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final target = _authoringRequiredObject(
      project.project['target'],
      'revision-3 Story Draft removal project target',
    );
    if (project.projectId != expectedProjectId ||
        project.revision != expectedRevision ||
        jsonEncode(target) != expectedTargetCanonicalJson) {
      throw const FormatException(
        'revision-3 Story Draft removal request disagrees with its exact project basis',
      );
    }
    _requireExactStoryDraftRemovalBasisPair(
      _authoringRequiredObject(
        project.project['entities'],
        'revision-3 Story Draft removal project entities',
      ),
      this,
    );
  }
}

final class AuthoringRevision3StoryDraftRemovalRemovedDraft {
  const AuthoringRevision3StoryDraftRemovalRemovedDraft._({
    required this.id,
    required this.kind,
    required this.revision,
  });

  final String id;
  final AuthoringStoryDraftKind kind;
  final int revision;
}

final class AuthoringRevision3StoryDraftRemovalRemovedScriptModule {
  const AuthoringRevision3StoryDraftRemovalRemovedScriptModule._({
    required this.id,
    required this.revision,
  });

  final String id;
  final int revision;
}

/// Strict prepare-only result. The candidate carries no publication authority;
/// the managed session must still fully reopen and fixed-head CAS publish it.
final class AuthoringRevision3StoryDraftRemovalPreparation {
  const AuthoringRevision3StoryDraftRemovalPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.removedDraft,
    required this.removedScriptModule,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.artifactAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final AuthoringRevision3StoryDraftRemovalRemovedDraft removedDraft;
  final AuthoringRevision3StoryDraftRemovalRemovedScriptModule
  removedScriptModule;
  final AuthoringRevision3StoryDraftRemovalBuildStatus buildStatus;
  final AuthoringRevision3StoryDraftRemovalRuntimeStatus runtimeStatus;
  final AuthoringRevision3StoryDraftRemovalArtifactAuthority artifactAuthority;
  final AuthoringRevision3StoryDraftRemovalNativePublicationStatus
  publicationStatus;

  factory AuthoringRevision3StoryDraftRemovalPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3StoryDraftRemovalRequestV1 request,
  }) {
    request._requireMatchesProject(currentProjectJson);
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'project_id',
      'revision',
      'removed',
      'build_status',
      'runtime_status',
      'artifact_authority',
      'publication_status',
    }, 'revision-3 Story Draft removal response');
    if (json['ok'] != true ||
        json['outcome'] != 'prepared_remove_unpublished') {
      throw const FormatException(
        'revision-3 Story Draft removal response is not an unpublished preparation',
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
        'revision-3 Story Draft removal response has an invalid head transition',
      );
    }
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final projectJson = _authoringRevision3ResponseString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringSignedJsonInteger,
    );
    if (projectId != request.expectedProjectId ||
        projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != request.expectedRevision + 1 ||
        revision != base.revision + 1 ||
        revision != candidate.revision) {
      throw const FormatException(
        'revision-3 Story Draft removal candidate identity or revision is invalid',
      );
    }

    final removed = _authoringRequiredObject(
      json['removed'],
      'revision-3 Story Draft removal removed pair',
    );
    _authoringExactFields(removed, const <String>{
      'draft',
      'script_module',
    }, 'revision-3 Story Draft removal removed pair');
    final removedDraft = _storyDraftRemovalRemovedDraft(removed['draft']);
    final removedScriptModule = _storyDraftRemovalRemovedScriptModule(
      removed['script_module'],
    );
    if (removedDraft.id != request.draftId ||
        removedDraft.kind != request.draftKind ||
        removedDraft.revision != request.expectedDraftRevision ||
        removedScriptModule.id != request.scriptModuleId ||
        removedScriptModule.revision != request.expectedScriptModuleRevision) {
      throw const FormatException(
        'revision-3 Story Draft removal receipt disagrees with its exact request',
      );
    }
    _requireExactStoryDraftRemovalCandidate(
      base: base.project,
      candidate: candidate.project,
      request: request,
    );
    return AuthoringRevision3StoryDraftRemovalPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      removedDraft: removedDraft,
      removedScriptModule: removedScriptModule,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3StoryDraftRemovalBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 Story Draft removal response has an unsupported build status',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3StoryDraftRemovalRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 Story Draft removal response has an unsupported runtime status',
        ),
      },
      artifactAuthority: switch (json['artifact_authority']) {
        'not_granted' =>
          AuthoringRevision3StoryDraftRemovalArtifactAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 Story Draft removal response grants unsupported artifact authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3StoryDraftRemovalNativePublicationStatus
              .notSupported,
        _ => throw const FormatException(
          'revision-3 Story Draft removal response grants unsupported publication authority',
        ),
      },
    );
  }
}

AuthoringRevision3StoryDraftRemovalRemovedDraft _storyDraftRemovalRemovedDraft(
  Object? value,
) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 Story Draft removal removed Draft',
  );
  _authoringExactFields(json, const <String>{
    'id',
    'kind',
    'revision',
  }, 'revision-3 Story Draft removal removed Draft');
  return AuthoringRevision3StoryDraftRemovalRemovedDraft._(
    id: _authoringEntityId(
      _authoringRequiredString(json, 'id', maxBytes: 32),
      'removed.draft.id',
    ),
    kind: switch (json['kind']) {
      'npc_draft' => AuthoringStoryDraftKind.npcDraft,
      'quest_draft' => AuthoringStoryDraftKind.questDraft,
      _ => throw const FormatException(
        'revision-3 Story Draft removal removed Draft kind is unsupported',
      ),
    },
    revision: _authoringRequiredInt(
      json,
      'revision',
      max: _maxAuthoringSignedJsonInteger,
    ),
  );
}

AuthoringRevision3StoryDraftRemovalRemovedScriptModule
_storyDraftRemovalRemovedScriptModule(Object? value) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 Story Draft removal removed ScriptModule',
  );
  _authoringExactFields(json, const <String>{
    'id',
    'kind',
    'revision',
  }, 'revision-3 Story Draft removal removed ScriptModule');
  if (json['kind'] != 'script_module') {
    throw const FormatException(
      'revision-3 Story Draft removal removed module kind is unsupported',
    );
  }
  return AuthoringRevision3StoryDraftRemovalRemovedScriptModule._(
    id: _authoringEntityId(
      _authoringRequiredString(json, 'id', maxBytes: 32),
      'removed.script_module.id',
    ),
    revision: _authoringRequiredInt(
      json,
      'revision',
      max: _maxAuthoringSignedJsonInteger,
    ),
  );
}

void _requireExactStoryDraftRemovalCandidate({
  required Map<String, Object?> base,
  required Map<String, Object?> candidate,
  required AuthoringRevision3StoryDraftRemovalRequestV1 request,
}) {
  for (final field in _authoringProjectTopLevelFields) {
    if (field == 'revision' || field == 'entities') continue;
    if (!_authoringJsonDeepEquals(base[field], candidate[field])) {
      throw FormatException(
        'revision-3 Story Draft removal candidate changed project field $field',
      );
    }
  }
  if (candidate['revision'] != request.expectedRevision + 1) {
    throw const FormatException(
      'revision-3 Story Draft removal candidate did not advance exactly once',
    );
  }
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 Story Draft removal basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'revision-3 Story Draft removal candidate entities',
  );
  if (baseEntities.length < 2 ||
      candidateEntities.length != baseEntities.length - 2 ||
      !baseEntities.containsKey(request.draftId) ||
      !baseEntities.containsKey(request.scriptModuleId) ||
      candidateEntities.containsKey(request.draftId) ||
      candidateEntities.containsKey(request.scriptModuleId)) {
    throw const FormatException(
      'revision-3 Story Draft removal candidate entity delta is not exactly two removals',
    );
  }
  _requireExactStoryDraftRemovalBasisPair(baseEntities, request);
  for (final entry in baseEntities.entries) {
    if (entry.key == request.draftId || entry.key == request.scriptModuleId) {
      continue;
    }
    if (!candidateEntities.containsKey(entry.key) ||
        !_authoringJsonDeepEquals(candidateEntities[entry.key], entry.value)) {
      throw const FormatException(
        'revision-3 Story Draft removal candidate changed another entity',
      );
    }
  }
  for (final key in candidateEntities.keys) {
    if (!baseEntities.containsKey(key) ||
        key == request.draftId ||
        key == request.scriptModuleId) {
      throw const FormatException(
        'revision-3 Story Draft removal candidate added or retained an unexpected entity',
      );
    }
  }
}

void _requireExactStoryDraftRemovalBasisPair(
  Map<String, Object?> entities,
  AuthoringRevision3StoryDraftRemovalRequestV1 request,
) {
  final draft = _storyDraftRemovalEntity(
    entities[request.draftId],
    request.draftId,
    'Draft',
  );
  final draftPayload = _authoringRequiredObject(
    draft['payload'],
    'revision-3 Story Draft removal Draft payload',
  );
  if (draft['revision'] != request.expectedDraftRevision ||
      draftPayload['kind'] != request.draftKind.wireName) {
    throw const FormatException(
      'revision-3 Story Draft removal basis Draft disagrees with the request',
    );
  }
  final draftData = _authoringRequiredObject(
    draftPayload['data'],
    'revision-3 Story Draft removal Draft data',
  );
  final draftGeneratorId = _authoringRequiredString(
    draftData,
    'generator_id',
    maxBytes: 256,
  );
  final draftGeneratorVersion = _authoringRequiredInt(
    draftData,
    'generator_version',
    max: _maxAuthoringSignedJsonInteger,
  );
  final expectedGenerator = switch (request.draftKind) {
    AuthoringStoryDraftKind.npcDraft => (
      id: _authoringRevision3NpcGeneratorId,
      version: _authoringRevision3NpcGeneratorVersion,
    ),
    AuthoringStoryDraftKind.questDraft => (
      id: _authoringRevision3QuestGeneratorId,
      version: _authoringRevision3QuestGeneratorVersion,
    ),
  };
  if (draftGeneratorId != expectedGenerator.id ||
      draftGeneratorVersion != expectedGenerator.version) {
    throw const FormatException(
      'revision-3 Story Draft removal basis uses an unsupported Draft generator',
    );
  }
  _authoringRequireTypedStoryRef(
    draftData['script_module'],
    projectId: request.expectedProjectId,
    id: request.scriptModuleId,
    kind: 'script_module',
    context: 'Story Draft removal Draft module',
  );

  final module = _storyDraftRemovalEntity(
    entities[request.scriptModuleId],
    request.scriptModuleId,
    'ScriptModule',
  );
  final modulePayload = _authoringRequiredObject(
    module['payload'],
    'revision-3 Story Draft removal ScriptModule payload',
  );
  if (module['revision'] != request.expectedScriptModuleRevision ||
      modulePayload['kind'] != 'script_module') {
    throw const FormatException(
      'revision-3 Story Draft removal basis ScriptModule disagrees with the request',
    );
  }
  final moduleOrigin = _authoringRequiredObject(
    module['origin'],
    'revision-3 Story Draft removal ScriptModule origin',
  );
  _authoringExactFields(moduleOrigin, const <String>{
    'type',
    'generator_id',
    'generator_version',
    'owner',
  }, 'revision-3 Story Draft removal ScriptModule origin');
  if (moduleOrigin['type'] != 'generated' ||
      moduleOrigin['generator_id'] != draftGeneratorId ||
      moduleOrigin['generator_version'] != draftGeneratorVersion) {
    throw const FormatException(
      'revision-3 Story Draft removal module origin does not match its Draft generator',
    );
  }
  _authoringRequireTypedStoryRef(
    moduleOrigin['owner'],
    projectId: request.expectedProjectId,
    id: request.draftId,
    kind: request.draftKind.wireName,
    context: 'Story Draft removal ScriptModule origin owner',
  );
  final moduleData = _authoringRequiredObject(
    modulePayload['data'],
    'revision-3 Story Draft removal ScriptModule data',
  );
  if (moduleData['generator_id'] != draftGeneratorId ||
      moduleData['generator_version'] != draftGeneratorVersion) {
    throw const FormatException(
      'revision-3 Story Draft removal module payload does not match its Draft generator',
    );
  }
  final status = _authoringRequiredObject(
    moduleData['status'],
    'revision-3 Story Draft removal ScriptModule status',
  );
  _authoringExactFields(status, const <String>{
    'authoring',
    'runtime',
  }, 'revision-3 Story Draft removal ScriptModule status');
  if (status['authoring'] != 'offline_draft' ||
      status['runtime'] != 'runtime_unqualified') {
    throw const FormatException(
      'revision-3 Story Draft removal module status is not a removable offline Draft',
    );
  }
  _authoringRequireTypedStoryRef(
    moduleData['owner'],
    projectId: request.expectedProjectId,
    id: request.draftId,
    kind: request.draftKind.wireName,
    context: 'Story Draft removal ScriptModule payload owner',
  );

  _requireExactStoryDraftRemovalReferenceClosure(entities, request);
}

void _requireExactStoryDraftRemovalReferenceClosure(
  Map<String, Object?> entities,
  AuthoringRevision3StoryDraftRemovalRequestV1 request,
) {
  var draftModuleEdges = 0;
  var moduleOwnerEdges = 0;

  bool isExactOutgoingDialogBinding(
    String sourceId,
    String targetId,
    String targetKind,
    List<Object> path,
  ) {
    if (sourceId != request.draftId ||
        targetId == request.draftId ||
        targetId == request.scriptModuleId ||
        targetKind != 'dialog_line') {
      return false;
    }
    final bindingField = switch (request.draftKind) {
      AuthoringStoryDraftKind.npcDraft => 'greetings',
      AuthoringStoryDraftKind.questDraft => 'transcript',
    };
    if (path.length != 5 ||
        path[0] != 'payload' ||
        path[1] != 'data' ||
        path[2] != bindingField ||
        path[3] is! int ||
        path[4] != 'line') {
      return false;
    }
    final rawTarget = entities[targetId];
    if (rawTarget is! Map) return false;
    final target = _authoringRequiredObject(
      rawTarget,
      'revision-3 Story Draft removal outgoing DialogLine target',
    );
    final payload = _authoringRequiredObject(
      target['payload'],
      'revision-3 Story Draft removal outgoing DialogLine payload',
    );
    return target['id'] == targetId && payload['kind'] == 'dialog_line';
  }

  void visit(Object? value, String sourceId, List<Object> path, int depth) {
    if (depth > 128) {
      throw const FormatException(
        'revision-3 Story Draft removal reference closure is too deeply nested',
      );
    }
    if (value is List) {
      for (var index = 0; index < value.length; index++) {
        visit(value[index], sourceId, <Object>[...path, index], depth + 1);
      }
      return;
    }
    if (value is! Map) return;
    final object = _authoringRequiredObject(
      value,
      'revision-3 Story Draft removal reference closure object',
    );
    if (object.length == 3 &&
        object.keys.toSet().containsAll(const <String>{
          'project_id',
          'id',
          'expected_kind',
        })) {
      final targetProject = object['project_id'];
      final targetId = object['id'];
      final targetKind = object['expected_kind'];
      if (targetProject is! String ||
          targetId is! String ||
          targetKind is! String) {
        throw const FormatException(
          'revision-3 Story Draft removal reference closure contains a malformed typed reference',
        );
      }
      if (targetProject != request.expectedProjectId) return;
      final sourceIsRemoved =
          sourceId == request.draftId || sourceId == request.scriptModuleId;
      final targetIsRemoved =
          targetId == request.draftId || targetId == request.scriptModuleId;
      if (!sourceIsRemoved && !targetIsRemoved) return;
      if (sourceId == request.draftId &&
          targetId == request.scriptModuleId &&
          targetKind == 'script_module') {
        draftModuleEdges++;
        return;
      }
      if (sourceId == request.scriptModuleId &&
          targetId == request.draftId &&
          targetKind == request.draftKind.wireName) {
        moduleOwnerEdges++;
        return;
      }
      if (isExactOutgoingDialogBinding(sourceId, targetId, targetKind, path)) {
        return;
      }
      throw const FormatException(
        'revision-3 Story Draft removal pair has an additional local reference',
      );
    }
    for (final entry in object.entries) {
      visit(entry.value, sourceId, <Object>[...path, entry.key], depth + 1);
    }
  }

  for (final entry in entities.entries) {
    visit(entry.value, entry.key, const <Object>[], 0);
  }
  if (draftModuleEdges != 1 || moduleOwnerEdges != 2) {
    throw const FormatException(
      'revision-3 Story Draft removal pair does not have the exact three-edge ownership closure',
    );
  }
}

Map<String, Object?> _storyDraftRemovalEntity(
  Object? value,
  String expectedId,
  String context,
) {
  final entity = _authoringRequiredObject(
    value,
    'revision-3 Story Draft removal $context entity',
  );
  _authoringExactFields(entity, const <String>{
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, 'revision-3 Story Draft removal $context entity');
  if (entity['id'] != expectedId) {
    throw FormatException(
      'revision-3 Story Draft removal $context key and ID disagree',
    );
  }
  _authoringRequiredInt(
    entity,
    'revision',
    max: _maxAuthoringSignedJsonInteger,
  );
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 Story Draft removal $context payload',
  );
  _authoringExactFields(payload, const <String>{
    'kind',
    'data',
  }, 'revision-3 Story Draft removal $context payload');
  return entity;
}
