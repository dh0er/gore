part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3NpcProfileEditRequestBytes = 32 * 1024;

enum AuthoringRevision3NpcProfileEditBuildStatus { blocked }

enum AuthoringRevision3NpcProfileEditRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3NpcProfileEditCatalogAuthority { notGranted }

enum AuthoringRevision3NpcProfileEditCollisionAuthority { notGranted }

enum AuthoringRevision3NpcProfileEditPublicationStatus { notSupported }

/// Non-wire catalog evidence retained beside the bounded request. Native owns
/// catalog selection, while Dart uses the same exact evidence to reject a
/// response whose generated parent bindings do not match the reviewed row.
final class AuthoringRevision3NpcProfileParentExpectation {
  const AuthoringRevision3NpcProfileParentExpectation({
    required this.catalogLayer,
    required this.authoringSelector,
    required this.runtimeClass,
    required this.sourceSeal,
  });

  final String catalogLayer;
  final String authoringSelector;
  final String runtimeClass;
  final AuthoringDraftContentSeal sourceSeal;

  bool sameBinding(AuthoringRevision3NpcProfileParentExpectation other) =>
      catalogLayer == other.catalogLayer &&
      authoringSelector == other.authoringSelector &&
      runtimeClass == other.runtimeClass &&
      _npcProfileEditSameSeal(sourceSeal, other.sourceSeal);
}

final class AuthoringRevision3NpcProfileParentTripleExpectation {
  const AuthoringRevision3NpcProfileParentTripleExpectation({
    required this.characterDefinition,
    required this.aiAgentConfig,
    required this.spawnDefinition,
  });

  final AuthoringRevision3NpcProfileParentExpectation characterDefinition;
  final AuthoringRevision3NpcProfileParentExpectation aiAgentConfig;
  final AuthoringRevision3NpcProfileParentExpectation spawnDefinition;

  bool sameBinding(AuthoringRevision3NpcProfileParentTripleExpectation other) =>
      characterDefinition.sameBinding(other.characterDefinition) &&
      aiAgentConfig.sameBinding(other.aiAgentConfig) &&
      spawnDefinition.sameBinding(other.spawnDefinition);
}

/// Exact existing NPC/module input derived locally from one canonical project.
/// It performs no native call and grants no catalog, mutation, or publication
/// authority.
final class AuthoringRevision3NpcProfileEditSeed {
  const AuthoringRevision3NpcProfileEditSeed._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.targetCanonicalJson,
    required this.npcId,
    required this.npcRevision,
    required this.scriptModuleId,
    required this.scriptModuleRevision,
    required this.displayName,
    required this.moduleNamespace,
    required this.uniqueName,
    required this.parentCharacterDefinition,
    required this.parentAiAgentConfig,
    required this.parentSpawnDefinition,
    required this.inputCanonicalJson,
    required this.inputSeal,
    required this.moduleSourceSeal,
    required this.moduleInputFingerprint,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final String targetCanonicalJson;
  final String npcId;
  final int npcRevision;
  final String scriptModuleId;
  final int scriptModuleRevision;
  final String displayName;
  final String moduleNamespace;
  final String uniqueName;
  final AuthoringRevision3NpcInspectionParent parentCharacterDefinition;
  final AuthoringRevision3NpcInspectionParent parentAiAgentConfig;
  final AuthoringRevision3NpcInspectionParent parentSpawnDefinition;
  final String inputCanonicalJson;
  final AuthoringDraftContentSeal inputSeal;
  final AuthoringDraftContentSeal moduleSourceSeal;
  final String moduleInputFingerprint;

  factory AuthoringRevision3NpcProfileEditSeed.forProject({
    required AuthoringWorkingHead head,
    required String currentProjectJson,
    required String npcId,
    required int expectedNpcRevision,
    required String expectedScriptModuleId,
    required int expectedScriptModuleRevision,
    required String expectedUniqueName,
    required String expectedModuleNamespace,
    required String expectedParentCharacterDefinition,
    required String expectedParentAiAgentConfig,
    required String expectedParentSpawnDefinition,
  }) {
    final seed = AuthoringRevision3NpcProfileEditSeed._fromProject(
      head: head,
      currentProjectJson: currentProjectJson,
      npcId: npcId,
      expectedNpcRevision: expectedNpcRevision,
      expectedScriptModuleId: expectedScriptModuleId,
      expectedScriptModuleRevision: expectedScriptModuleRevision,
    );
    if (seed.uniqueName != expectedUniqueName ||
        seed.moduleNamespace != expectedModuleNamespace ||
        seed.parentCharacterDefinition.runtimeClass !=
            expectedParentCharacterDefinition ||
        seed.parentAiAgentConfig.runtimeClass != expectedParentAiAgentConfig ||
        seed.parentSpawnDefinition.runtimeClass !=
            expectedParentSpawnDefinition) {
      throw const FormatException(
        'revision-3 NPC profile seed disagrees with the selected content projection',
      );
    }
    return seed;
  }

  factory AuthoringRevision3NpcProfileEditSeed._fromProject({
    required AuthoringWorkingHead head,
    required String currentProjectJson,
    required String npcId,
    required int expectedNpcRevision,
    required String expectedScriptModuleId,
    required int expectedScriptModuleRevision,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    _npcProfileEditRequireHeadBindsProject(
      head,
      currentProjectJson,
      'NPC profile seed',
    );
    final pair = _npcProfileEditParsePair(
      current.project,
      projectId: current.projectId,
      npcId: npcId,
      expectedNpcRevision: expectedNpcRevision,
      scriptModuleId: expectedScriptModuleId,
      expectedScriptModuleRevision: expectedScriptModuleRevision,
    );
    return AuthoringRevision3NpcProfileEditSeed._(
      head: head,
      projectId: current.projectId,
      projectRevision: current.revision,
      targetCanonicalJson: jsonEncode(current.project['target']),
      npcId: npcId,
      npcRevision: pair.npcRevision,
      scriptModuleId: expectedScriptModuleId,
      scriptModuleRevision: pair.moduleRevision,
      displayName: pair.displayName,
      moduleNamespace: pair.input.moduleNamespace,
      uniqueName: pair.input.uniqueName,
      parentCharacterDefinition: pair.input.parentCharacterDefinition,
      parentAiAgentConfig: pair.input.parentAiAgentConfig,
      parentSpawnDefinition: pair.input.parentSpawnDefinition,
      inputCanonicalJson: pair.input.canonicalJson,
      inputSeal: _npcProfileEditBytesSeal(pair.input.canonicalJson),
      moduleSourceSeal: _npcProfileEditBytesSeal(pair.source),
      moduleInputFingerprint: pair.inputFingerprint,
    );
  }
}

/// Canonical exact-basis request for one bounded name/archetype edit.
final class AuthoringRevision3NpcProfileEditRequestV1 {
  const AuthoringRevision3NpcProfileEditRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.expectedStoryCatalogSeal,
    required this.expectedNpcCatalogSeal,
    required this.npcId,
    required this.expectedNpcRevision,
    required this.scriptModuleId,
    required this.expectedScriptModuleRevision,
    required this.expectedParentCatalogId,
    required this.displayName,
    required this.parentCatalogId,
    required this.basisSeed,
    required this.expectedCurrentParentTriple,
    required this.expectedParentTriple,
    required this.expectsArchetypeChanged,
    required this.expectsModuleRegenerated,
  });

  factory AuthoringRevision3NpcProfileEditRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required AuthoringRevision3NpcProfileEditSeed seed,
    required AuthoringDraftContentSeal expectedStoryCatalogSeal,
    required AuthoringDraftContentSeal expectedNpcCatalogSeal,
    required String expectedParentCatalogId,
    required String displayName,
    required String parentCatalogId,
    required AuthoringRevision3NpcProfileParentTripleExpectation
    expectedCurrentParentTriple,
    required AuthoringRevision3NpcProfileParentTripleExpectation
    expectedParentTriple,
    required bool expectedArchetypeChanged,
    required bool expectedModuleRegenerated,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    if (seed.head.canonicalJson != expectedHead.canonicalJson ||
        seed.projectId != current.projectId ||
        seed.projectRevision != current.revision ||
        seed.targetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'revision-3 NPC profile seed is stale for the current project',
      );
    }
    return AuthoringRevision3NpcProfileEditRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'expected_story_catalog_seal': _npcProfileEditSealJson(
          expectedStoryCatalogSeal,
        ),
        'expected_npc_catalog_seal': _npcProfileEditSealJson(
          expectedNpcCatalogSeal,
        ),
        'npc_id': seed.npcId,
        'expected_npc_revision': seed.npcRevision,
        'script_module_id': seed.scriptModuleId,
        'expected_script_module_revision': seed.scriptModuleRevision,
        'expected_parent_catalog_id': expectedParentCatalogId,
        'display_name': displayName,
        'parent_catalog_id': parentCatalogId,
      }),
      currentProjectJson: currentProjectJson,
      expectedCurrentParentTriple: expectedCurrentParentTriple,
      expectedParentTriple: expectedParentTriple,
      expectedArchetypeChanged: expectedArchetypeChanged,
      expectedModuleRegenerated: expectedModuleRegenerated,
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final AuthoringDraftContentSeal expectedStoryCatalogSeal;
  final AuthoringDraftContentSeal expectedNpcCatalogSeal;
  final String npcId;
  final int expectedNpcRevision;
  final String scriptModuleId;
  final int expectedScriptModuleRevision;
  final String expectedParentCatalogId;
  final String displayName;
  final String parentCatalogId;
  final AuthoringRevision3NpcProfileEditSeed basisSeed;
  final AuthoringRevision3NpcProfileParentTripleExpectation
  expectedCurrentParentTriple;
  final AuthoringRevision3NpcProfileParentTripleExpectation
  expectedParentTriple;
  final bool expectsArchetypeChanged;
  final bool expectsModuleRegenerated;

  bool get expectsNameChanged => displayName != basisSeed.displayName;

  factory AuthoringRevision3NpcProfileEditRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
    required AuthoringRevision3NpcProfileParentTripleExpectation
    expectedCurrentParentTriple,
    required AuthoringRevision3NpcProfileParentTripleExpectation
    expectedParentTriple,
    required bool expectedArchetypeChanged,
    required bool expectedModuleRegenerated,
  }) {
    if (expectedModuleRegenerated != expectedArchetypeChanged ||
        expectedArchetypeChanged !=
            !expectedCurrentParentTriple.sameBinding(expectedParentTriple)) {
      throw const FormatException(
        'revision-3 NPC profile regeneration expectation is inconsistent',
      );
    }
    try {
      _authoringRevision3RequestString(
        value,
        'npcProfileEditRequestJson',
        _maxAuthoringRevision3NpcProfileEditRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 NPC profile request is not bounded UTF-8',
      );
    }
    final json = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 NPC profile request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'expected_story_catalog_seal',
      'expected_npc_catalog_seal',
      'npc_id',
      'expected_npc_revision',
      'script_module_id',
      'expected_script_module_revision',
      'expected_parent_catalog_id',
      'display_name',
      'parent_catalog_id',
    ];
    _authoringExactFields(
      json,
      fields.toSet(),
      'revision-3 NPC profile request',
    );
    _authoringRevision3NpcRequireFieldOrder(
      json,
      fields,
      'profile edit request',
    );
    if (jsonEncode(json) != value) {
      throw const FormatException(
        'revision-3 NPC profile request is not canonical',
      );
    }
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          json['expected_head'],
          'revision-3 NPC profile expected head',
        ),
      ),
    );
    _npcProfileEditRequireHeadBindsProject(
      expectedHead,
      currentProjectJson,
      'NPC profile request',
    );
    final expectedProjectId = _authoringRevision3NpcEntityId(
      json,
      'expected_project_id',
    );
    final expectedRevision = _authoringRequiredInt(
      json,
      'expected_revision',
      max: _maxAuthoringRevision3NpcBasisRevision,
    );
    final target = _authoringRevision3NpcGeneration(
      json['expected_target'],
      'NPC profile request target',
    );
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        jsonEncode(target.json) != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'revision-3 NPC profile request does not bind the exact current project',
      );
    }
    final npcId = _authoringRevision3NpcEntityId(json, 'npc_id');
    final npcRevision = _authoringRequiredInt(
      json,
      'expected_npc_revision',
      max: _maxAuthoringRevision3NpcBasisRevision,
    );
    final moduleId = _authoringRevision3NpcEntityId(json, 'script_module_id');
    final moduleRevision = _authoringRequiredInt(
      json,
      'expected_script_module_revision',
      max: expectedModuleRegenerated
          ? _maxAuthoringRevision3NpcBasisRevision
          : _maxAuthoringRevision3NpcAppliedRevision,
    );
    if (npcId == moduleId) {
      throw const FormatException(
        'revision-3 NPC profile request entity IDs must be distinct',
      );
    }
    final currentParent = _npcProfileEditCatalogId(
      json,
      'expected_parent_catalog_id',
    );
    final nextParent = _npcProfileEditCatalogId(json, 'parent_catalog_id');
    final displayName = _authoringRevision3NpcDisplayName(json, 'display_name');
    final seed = AuthoringRevision3NpcProfileEditSeed._fromProject(
      head: expectedHead,
      currentProjectJson: currentProjectJson,
      npcId: npcId,
      expectedNpcRevision: npcRevision,
      expectedScriptModuleId: moduleId,
      expectedScriptModuleRevision: moduleRevision,
    );
    _npcProfileEditRequireParentTriple(seed, expectedCurrentParentTriple);
    if (displayName == seed.displayName && !expectedArchetypeChanged) {
      throw const FormatException(
        'revision-3 NPC profile request does not change the profile',
      );
    }
    return AuthoringRevision3NpcProfileEditRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: expectedProjectId,
      expectedRevision: expectedRevision,
      expectedTargetCanonicalJson: jsonEncode(target.json),
      expectedStoryCatalogSeal: _npcProfileEditSeal(
        json['expected_story_catalog_seal'],
        'Story catalog',
      ),
      expectedNpcCatalogSeal: _npcProfileEditSeal(
        json['expected_npc_catalog_seal'],
        'NPC catalog',
      ),
      npcId: npcId,
      expectedNpcRevision: npcRevision,
      scriptModuleId: moduleId,
      expectedScriptModuleRevision: moduleRevision,
      expectedParentCatalogId: currentParent,
      displayName: displayName,
      parentCatalogId: nextParent,
      basisSeed: seed,
      expectedCurrentParentTriple: expectedCurrentParentTriple,
      expectedParentTriple: expectedParentTriple,
      expectsArchetypeChanged: expectedArchetypeChanged,
      expectsModuleRegenerated: expectedModuleRegenerated,
    );
  }
}

/// Strict prepare-only result. Publication remains exclusively managed by the
/// serialized session after a full candidate reopen.
final class AuthoringRevision3NpcProfileEditPreparation {
  const AuthoringRevision3NpcProfileEditPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.npcId,
    required this.npcRevision,
    required this.scriptModuleId,
    required this.scriptModuleRevision,
    required this.displayName,
    required this.previousParentCatalogId,
    required this.parentCatalogId,
    required this.storyCatalogSeal,
    required this.npcCatalogSeal,
    required this.nameChanged,
    required this.archetypeChanged,
    required this.moduleRegenerated,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.catalogAuthority,
    required this.collisionAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String npcId;
  final int npcRevision;
  final String scriptModuleId;
  final int scriptModuleRevision;
  final String displayName;
  final String previousParentCatalogId;
  final String parentCatalogId;
  final AuthoringDraftContentSeal storyCatalogSeal;
  final AuthoringDraftContentSeal npcCatalogSeal;
  final bool nameChanged;
  final bool archetypeChanged;
  final bool moduleRegenerated;
  final AuthoringRevision3NpcProfileEditBuildStatus buildStatus;
  final AuthoringRevision3NpcProfileEditRuntimeStatus runtimeStatus;
  final AuthoringRevision3NpcProfileEditCatalogAuthority catalogAuthority;
  final AuthoringRevision3NpcProfileEditCollisionAuthority collisionAuthority;
  final AuthoringRevision3NpcProfileEditPublicationStatus publicationStatus;

  factory AuthoringRevision3NpcProfileEditPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3NpcProfileEditRequestV1 request,
  }) {
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
      'script_module_id',
      'script_module_revision',
      'display_name',
      'previous_parent_catalog_id',
      'parent_catalog_id',
      'story_catalog_seal',
      'npc_catalog_seal',
      'name_changed',
      'archetype_changed',
      'module_regenerated',
      'build_status',
      'runtime_status',
      'catalog_authority',
      'collision_authority',
      'publication_status',
    }, 'revision-3 NPC profile preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 NPC profile response is not an unpublished preparation',
      );
    }
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _npcProfileEditString(
        json,
        'basis_head_json',
        _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson) {
      throw const FormatException(
        'revision-3 NPC profile response basis head disagrees',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _npcProfileEditString(json, 'head_json', _maxAuthoringHeadJsonBytes),
    );
    final projectJson = _npcProfileEditString(
      json,
      'project_json',
      _maxAuthoringProjectJsonBytes,
    );
    _npcProfileEditRequireHeadBindsProject(
      head,
      projectJson,
      'NPC profile candidate',
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _authoringRevision3NpcEntityId(json, 'project_id');
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringRevision3NpcAppliedRevision,
    );
    final npcId = _authoringRevision3NpcEntityId(json, 'npc_id');
    final npcRevision = _authoringRequiredInt(
      json,
      'npc_revision',
      min: 1,
      max: _maxAuthoringRevision3NpcAppliedRevision,
    );
    final moduleId = _authoringRevision3NpcEntityId(json, 'script_module_id');
    final moduleRevision = _authoringRequiredInt(
      json,
      'script_module_revision',
      max: _maxAuthoringRevision3NpcAppliedRevision,
    );
    final displayName = _authoringRevision3NpcDisplayName(json, 'display_name');
    final previousParent = _npcProfileEditCatalogId(
      json,
      'previous_parent_catalog_id',
    );
    final parent = _npcProfileEditCatalogId(json, 'parent_catalog_id');
    final storySeal = _npcProfileEditSeal(
      json['story_catalog_seal'],
      'response Story catalog',
    );
    final npcSeal = _npcProfileEditSeal(
      json['npc_catalog_seal'],
      'response NPC catalog',
    );
    final nameChanged = _npcProfileEditBool(json, 'name_changed');
    final archetypeChanged = _npcProfileEditBool(json, 'archetype_changed');
    final moduleRegenerated = _npcProfileEditBool(json, 'module_regenerated');
    if (projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != base.revision + 1 ||
        revision != candidate.revision ||
        npcId != request.npcId ||
        npcRevision != request.expectedNpcRevision + 1 ||
        moduleId != request.scriptModuleId ||
        moduleRevision !=
            request.expectedScriptModuleRevision +
                (request.expectsModuleRegenerated ? 1 : 0) ||
        displayName != request.displayName ||
        previousParent != request.expectedParentCatalogId ||
        parent != request.parentCatalogId ||
        !_npcProfileEditSameSeal(storySeal, request.expectedStoryCatalogSeal) ||
        !_npcProfileEditSameSeal(npcSeal, request.expectedNpcCatalogSeal) ||
        nameChanged != request.expectsNameChanged ||
        archetypeChanged != request.expectsArchetypeChanged ||
        moduleRegenerated != request.expectsModuleRegenerated) {
      throw const FormatException(
        'revision-3 NPC profile response disagrees with its exact request',
      );
    }
    _npcProfileEditRequireExactDelta(
      base.project,
      candidate.project,
      request: request,
    );
    return AuthoringRevision3NpcProfileEditPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      npcId: npcId,
      npcRevision: npcRevision,
      scriptModuleId: moduleId,
      scriptModuleRevision: moduleRevision,
      displayName: displayName,
      previousParentCatalogId: previousParent,
      parentCatalogId: parent,
      storyCatalogSeal: storySeal,
      npcCatalogSeal: npcSeal,
      nameChanged: nameChanged,
      archetypeChanged: archetypeChanged,
      moduleRegenerated: moduleRegenerated,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3NpcProfileEditBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 NPC profile response grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3NpcProfileEditRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 NPC profile response grants runtime authority',
        ),
      },
      catalogAuthority: switch (json['catalog_authority']) {
        'not_granted' =>
          AuthoringRevision3NpcProfileEditCatalogAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 NPC profile response grants catalog authority',
        ),
      },
      collisionAuthority: switch (json['collision_authority']) {
        'not_granted' =>
          AuthoringRevision3NpcProfileEditCollisionAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 NPC profile response grants collision authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3NpcProfileEditPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 NPC profile response grants native publication authority',
        ),
      },
    );
  }
}

typedef _NpcProfileEditPair = ({
  Map<String, Object?> npcEntity,
  Map<String, Object?> npcData,
  Map<String, Object?> moduleEntity,
  Map<String, Object?> moduleData,
  int npcRevision,
  int moduleRevision,
  String displayName,
  AuthoringRevision3NpcInspectionInput input,
  String source,
  String inputFingerprint,
});

_NpcProfileEditPair _npcProfileEditParsePair(
  Map<String, Object?> project, {
  required String projectId,
  required String npcId,
  required int expectedNpcRevision,
  required String scriptModuleId,
  required int expectedScriptModuleRevision,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 NPC profile entities',
  );
  final npc = _authoringRevision3NpcEntity(entities, npcId, 'npc_draft');
  final npcRevision = _authoringRequiredInt(
    npc.entity,
    'revision',
    max: _maxAuthoringRevision3NpcAppliedRevision,
  );
  final displayName = _authoringRevision3NpcDisplayName(
    npc.entity,
    'display_name',
  );
  final npcOrigin = _authoringRequiredObject(
    npc.entity['origin'],
    'revision-3 NPC profile origin',
  );
  _authoringExactFields(npcOrigin, const <String>{
    'type',
    'authored_runtime_id',
  }, 'revision-3 NPC profile origin');
  _authoringExactFields(npc.data, const <String>{
    'generator_id',
    'generator_version',
    'input',
    'script_module',
  }, 'revision-3 NPC profile data');
  _authoringRevision3NpcRequireGenerator(npc.data, 'profile NPC data');
  final input = AuthoringRevision3NpcInspectionInput._fromJson(
    npc.data['input'],
  );
  if (npcOrigin['type'] != 'new' ||
      npcOrigin['authored_runtime_id'] != input.uniqueName) {
    throw const FormatException('revision-3 NPC profile origin is unsupported');
  }
  _authoringRevision3NpcTypedRef(
    npc.data['script_module'],
    projectId: projectId,
    id: scriptModuleId,
    kind: 'script_module',
    context: 'profile ScriptModule',
  );
  final module = _authoringRevision3NpcEntity(
    entities,
    scriptModuleId,
    'script_module',
  );
  final moduleRevision = _authoringRequiredInt(
    module.entity,
    'revision',
    max: _maxAuthoringRevision3NpcAppliedRevision,
  );
  final moduleOrigin = _authoringRequiredObject(
    module.entity['origin'],
    'revision-3 NPC profile module origin',
  );
  _authoringExactFields(moduleOrigin, const <String>{
    'type',
    'generator_id',
    'generator_version',
    'owner',
  }, 'revision-3 NPC profile module origin');
  if (moduleOrigin['type'] != 'generated') {
    throw const FormatException(
      'revision-3 NPC profile module origin is unsupported',
    );
  }
  _authoringRevision3NpcRequireGenerator(moduleOrigin, 'profile module origin');
  _authoringRevision3NpcTypedRef(
    moduleOrigin['owner'],
    projectId: projectId,
    id: npcId,
    kind: 'npc_draft',
    context: 'profile module origin owner',
  );
  final generated = AuthoringRevision3NpcInspectionGeneratedModule._fromJson(
    module.data,
    input,
  );
  if (module.entity['display_name'] != input.moduleNamespace ||
      generated.owner.projectId != projectId ||
      generated.owner.id != npcId) {
    throw const FormatException(
      'revision-3 NPC profile module binding is invalid',
    );
  }
  if (npcRevision != expectedNpcRevision ||
      moduleRevision != expectedScriptModuleRevision) {
    throw const FormatException(
      'revision-3 NPC profile entity revisions are stale',
    );
  }
  return (
    npcEntity: npc.entity,
    npcData: npc.data,
    moduleEntity: module.entity,
    moduleData: module.data,
    npcRevision: npcRevision,
    moduleRevision: moduleRevision,
    displayName: displayName,
    input: input,
    source: generated.source,
    inputFingerprint: generated.inputFingerprint,
  );
}

void _npcProfileEditRequireExactDelta(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3NpcProfileEditRequestV1 request,
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
        'revision-3 NPC profile candidate changed basis field $field',
      );
    }
  }
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 NPC profile basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'revision-3 NPC profile candidate entities',
  );
  if (candidateEntities.length != baseEntities.length ||
      !candidateEntities.keys.toSet().containsAll(baseEntities.keys)) {
    throw const FormatException(
      'revision-3 NPC profile candidate changed the entity set',
    );
  }
  for (final entry in baseEntities.entries) {
    if (entry.key == request.npcId || entry.key == request.scriptModuleId) {
      continue;
    }
    if (!_authoringJsonDeepEquals(candidateEntities[entry.key], entry.value)) {
      throw const FormatException(
        'revision-3 NPC profile candidate changed an unrelated entity',
      );
    }
  }
  final basePair = _npcProfileEditParsePair(
    base,
    projectId: request.expectedProjectId,
    npcId: request.npcId,
    expectedNpcRevision: request.expectedNpcRevision,
    scriptModuleId: request.scriptModuleId,
    expectedScriptModuleRevision: request.expectedScriptModuleRevision,
  );
  final candidatePair = _npcProfileEditParsePair(
    candidate,
    projectId: request.expectedProjectId,
    npcId: request.npcId,
    expectedNpcRevision: request.expectedNpcRevision + 1,
    scriptModuleId: request.scriptModuleId,
    expectedScriptModuleRevision:
        request.expectedScriptModuleRevision +
        (request.expectsModuleRegenerated ? 1 : 0),
  );
  if (candidatePair.displayName != request.displayName ||
      candidatePair.input.moduleNamespace != basePair.input.moduleNamespace ||
      candidatePair.input.uniqueName != basePair.input.uniqueName ||
      !_authoringJsonDeepEquals(
        basePair.npcEntity['origin'],
        candidatePair.npcEntity['origin'],
      )) {
    throw const FormatException(
      'revision-3 NPC profile candidate changed stable NPC identity',
    );
  }
  _npcProfileEditRequireParentTriple(
    AuthoringRevision3NpcProfileEditSeed._(
      head: request.expectedHead,
      projectId: request.expectedProjectId,
      projectRevision: request.expectedRevision,
      targetCanonicalJson: request.expectedTargetCanonicalJson,
      npcId: request.npcId,
      npcRevision: candidatePair.npcRevision,
      scriptModuleId: request.scriptModuleId,
      scriptModuleRevision: candidatePair.moduleRevision,
      displayName: candidatePair.displayName,
      moduleNamespace: candidatePair.input.moduleNamespace,
      uniqueName: candidatePair.input.uniqueName,
      parentCharacterDefinition: candidatePair.input.parentCharacterDefinition,
      parentAiAgentConfig: candidatePair.input.parentAiAgentConfig,
      parentSpawnDefinition: candidatePair.input.parentSpawnDefinition,
      inputCanonicalJson: candidatePair.input.canonicalJson,
      inputSeal: _npcProfileEditBytesSeal(candidatePair.input.canonicalJson),
      moduleSourceSeal: _npcProfileEditBytesSeal(candidatePair.source),
      moduleInputFingerprint: candidatePair.inputFingerprint,
    ),
    request.expectedParentTriple,
  );
  if (!request.expectsArchetypeChanged) {
    if (!_authoringJsonDeepEquals(basePair.npcData, candidatePair.npcData) ||
        !_authoringJsonDeepEquals(
          basePair.moduleEntity,
          candidatePair.moduleEntity,
        )) {
      throw const FormatException(
        'revision-3 NPC name-only edit changed generated content',
      );
    }
    return;
  }
  final baseInput = jsonDecode(basePair.input.canonicalJson);
  final candidateInput = jsonDecode(candidatePair.input.canonicalJson);
  if (baseInput is! Map<String, Object?> ||
      candidateInput is! Map<String, Object?>) {
    throw const FormatException('revision-3 NPC profile input is invalid');
  }
  for (final field in const <String>[
    'target',
    'module_namespace',
    'unique_name',
  ]) {
    if (!_authoringJsonDeepEquals(baseInput[field], candidateInput[field])) {
      throw FormatException(
        'revision-3 NPC archetype edit changed stable input field $field',
      );
    }
  }
  if (!_authoringJsonDeepEquals(
        basePair.moduleEntity['origin'],
        candidatePair.moduleEntity['origin'],
      ) ||
      basePair.moduleEntity['display_name'] !=
          candidatePair.moduleEntity['display_name']) {
    throw const FormatException(
      'revision-3 NPC archetype edit changed stable module identity',
    );
  }
}

void _npcProfileEditRequireParentTriple(
  AuthoringRevision3NpcProfileEditSeed seed,
  AuthoringRevision3NpcProfileParentTripleExpectation expected,
) {
  if (!_npcProfileEditParentMatches(
        seed.parentCharacterDefinition,
        expected.characterDefinition,
      ) ||
      !_npcProfileEditParentMatches(
        seed.parentAiAgentConfig,
        expected.aiAgentConfig,
      ) ||
      !_npcProfileEditParentMatches(
        seed.parentSpawnDefinition,
        expected.spawnDefinition,
      )) {
    throw const FormatException(
      'revision-3 NPC profile parent triple is not the exact catalog selection',
    );
  }
}

bool _npcProfileEditParentMatches(
  AuthoringRevision3NpcInspectionParent actual,
  AuthoringRevision3NpcProfileParentExpectation expected,
) =>
    actual.catalogLayer == expected.catalogLayer &&
    actual.canonicalSelector == expected.authoringSelector &&
    actual.runtimeClass == expected.runtimeClass &&
    _npcProfileEditSameSeal(actual.sourceSeal, expected.sourceSeal);

void _npcProfileEditRequireHeadBindsProject(
  AuthoringWorkingHead head,
  String projectJson,
  String context,
) {
  final bytes = utf8.encode(projectJson);
  if (head.snapshotByteLength != bytes.length ||
      head.snapshotSha256 != crypto.sha256.convert(bytes).toString()) {
    throw FormatException('$context head does not seal its project bytes');
  }
}

AuthoringDraftContentSeal _npcProfileEditBytesSeal(String value) {
  final bytes = utf8.encode(value);
  return AuthoringDraftContentSeal.fromJson(<String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  });
}

AuthoringDraftContentSeal _npcProfileEditSeal(Object? value, String context) =>
    _npcInspectionSeal(value, 'revision-3 NPC profile $context');

Map<String, Object?> _npcProfileEditSealJson(AuthoringDraftContentSeal seal) =>
    <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};

bool _npcProfileEditSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

String _npcProfileEditCatalogId(Map<String, Object?> json, String field) {
  final value = _npcProfileEditString(
    json,
    field,
    _maxAuthoringRevision3NpcCatalogIdBytes,
  );
  if (!_authoringRevision3NpcCatalogIdPattern.hasMatch(value)) {
    throw FormatException('revision-3 NPC profile $field is not canonical');
  }
  return value;
}

String _npcProfileEditString(
  Map<String, Object?> json,
  String field,
  int maxBytes,
) => _authoringRevision3NpcString(json, field, maxBytes: maxBytes);

bool _npcProfileEditBool(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! bool) {
    throw FormatException('revision-3 NPC profile $field is not boolean');
  }
  return value;
}
