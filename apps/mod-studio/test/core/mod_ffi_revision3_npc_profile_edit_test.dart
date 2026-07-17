import 'dart:collection';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_npc_fixture.dart';

const _projectId = '00000000000000000000000000000003';
const _npcId = '00000000000000000000000000000081';
const _moduleId = '00000000000000000000000000000082';
const _asghanId = 'g1r:npc:om_grd_asghan_263';
const _viperId = 'g1r:npc:om_stt_viper_302';
const _aliasId = 'g1r:npc:asghan_alias_263';
const _executableByteLength = 171698176;
const _executableSha256 =
    'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5';
const _signedMax = 0x7fffffffffffffff;

void main() {
  test('required command handshake includes NPC profile preparation', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_npc_profile_edit_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'name-only candidate preserves the complete generated Module entity',
    () {
      final basis = _basis();
      final request = _request(
        basis,
        displayName: 'Renamed Guard',
        currentId: _asghanId,
        nextId: _asghanId,
        currentTriple: basis.asghanTriple,
        nextTriple: basis.asghanTriple,
        archetypeChanged: false,
      );
      final candidate = _candidate(
        basis,
        displayName: request.displayName,
        input: basis.asghanInput,
        regenerateModule: false,
      );

      final prepared = AuthoringRevision3NpcProfileEditPreparation.fromJson(
        _response(request, candidate),
        currentProjectJson: basis.projectJson,
        request: request,
      );

      expect(prepared.nameChanged, isTrue);
      expect(prepared.archetypeChanged, isFalse);
      expect(prepared.scriptModuleRevision, 0);
      final baseModule = _entity(basis.projectJson, _moduleId);
      final nextModule = _entity(candidate.projectJson, _moduleId);
      expect(nextModule, baseModule);
    },
  );

  test(
    'archetype edit requires the reviewed triple and regenerated Module',
    () {
      final basis = _basis();
      final request = _request(
        basis,
        displayName: basis.displayName,
        currentId: _asghanId,
        nextId: _viperId,
        currentTriple: basis.asghanTriple,
        nextTriple: basis.viperTriple,
        archetypeChanged: true,
      );
      final candidate = _candidate(
        basis,
        displayName: request.displayName,
        input: basis.viperInput,
        regenerateModule: true,
      );

      final prepared = AuthoringRevision3NpcProfileEditPreparation.fromJson(
        _response(request, candidate),
        currentProjectJson: basis.projectJson,
        request: request,
      );

      expect(prepared.nameChanged, isFalse);
      expect(prepared.archetypeChanged, isTrue);
      expect(prepared.moduleRegenerated, isTrue);
      expect(prepared.scriptModuleRevision, 1);
    },
  );

  test('profile edits preserve existing NPC greeting bindings exactly', () {
    final basis = _basis(withGreetings: true);
    final request = _request(
      basis,
      displayName: basis.displayName,
      currentId: _asghanId,
      nextId: _viperId,
      currentTriple: basis.asghanTriple,
      nextTriple: basis.viperTriple,
      archetypeChanged: true,
    );
    final intact = _candidate(
      basis,
      displayName: request.displayName,
      input: basis.viperInput,
      regenerateModule: true,
    );

    final prepared = AuthoringRevision3NpcProfileEditPreparation.fromJson(
      _response(request, intact),
      currentProjectJson: basis.projectJson,
      request: request,
    );
    expect(prepared.archetypeChanged, isTrue);
    expect(
      _npcData(intact.projectJson)['greetings'],
      _npcData(basis.projectJson)['greetings'],
    );

    final dropped = _candidate(
      basis,
      displayName: request.displayName,
      input: basis.viperInput,
      regenerateModule: true,
      dropGreetings: true,
    );
    expect(
      () => AuthoringRevision3NpcProfileEditPreparation.fromJson(
        _response(request, dropped),
        currentProjectJson: basis.projectJson,
        request: request,
      ),
      throwsFormatException,
    );
  });

  test('wrapper sends the exact bounded NPC profile request', () async {
    final basis = _basis();
    final request = _request(
      basis,
      displayName: 'Renamed Guard',
      currentId: _asghanId,
      nextId: _asghanId,
      currentTriple: basis.asghanTriple,
      nextTriple: basis.asghanTriple,
      archetypeChanged: false,
    );
    final candidate = _candidate(
      basis,
      displayName: request.displayName,
      input: basis.asghanInput,
      regenerateModule: false,
    );
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_prepare_revision3_npc_profile_edit_v1': _response(
          request,
          candidate,
        ),
      },
    );

    final prepared = await ModFfi(core)
        .authoringStorePrepareRevision3NpcProfileEditV1(
          root: r'C:\Mods\Managed NPC.goreproj',
          gameRoot: r'D:\Games\Gothic Remake',
          currentProjectJson: basis.projectJson,
          request: request,
        );

    expect(prepared.head.canonicalJson, candidate.head.canonicalJson);
    expect(core.calls, hasLength(1));
    expect(core.calls.single.payload, <String, Object?>{
      'current_project_json': basis.projectJson,
      'game_root': r'D:\Games\Gothic Remake',
      'npc_profile_request_json': request.canonicalJson,
      'root': r'C:\Mods\Managed NPC.goreproj',
    });
  });

  test('alias ID with the same triple remains a name-only edit', () {
    final basis = _basis();
    final request = _request(
      basis,
      displayName: 'Alias Renamed Guard',
      currentId: _asghanId,
      nextId: _aliasId,
      currentTriple: basis.asghanTriple,
      nextTriple: basis.asghanTriple,
      archetypeChanged: false,
    );
    final candidate = _candidate(
      basis,
      displayName: request.displayName,
      input: basis.asghanInput,
      regenerateModule: false,
    );

    final prepared = AuthoringRevision3NpcProfileEditPreparation.fromJson(
      _response(request, candidate),
      currentProjectJson: basis.projectJson,
      request: request,
    );

    expect(prepared.parentCatalogId, _aliasId);
    expect(prepared.archetypeChanged, isFalse);
    expect(prepared.moduleRegenerated, isFalse);
  });

  test('alias-only selection and forged delta are rejected locally', () {
    final basis = _basis();
    expect(
      () => _request(
        basis,
        displayName: basis.displayName,
        currentId: _asghanId,
        nextId: _aliasId,
        currentTriple: basis.asghanTriple,
        nextTriple: basis.asghanTriple,
        archetypeChanged: false,
      ),
      throwsFormatException,
    );

    final request = _request(
      basis,
      displayName: 'Renamed Guard',
      currentId: _asghanId,
      nextId: _asghanId,
      currentTriple: basis.asghanTriple,
      nextTriple: basis.asghanTriple,
      archetypeChanged: false,
    );
    final candidate = _candidate(
      basis,
      displayName: request.displayName,
      input: basis.asghanInput,
      regenerateModule: false,
      mutateModule: true,
    );
    expect(
      () => AuthoringRevision3NpcProfileEditPreparation.fromJson(
        _response(request, candidate),
        currentProjectJson: basis.projectJson,
        request: request,
      ),
      throwsFormatException,
    );
  });

  test(
    'signed-max Module revision is accepted only when Module stays byte-exact',
    () {
      final basis = _basis(moduleRevision: _signedMax);
      final nameOnly = _request(
        basis,
        displayName: 'Signed Max Rename',
        currentId: _asghanId,
        nextId: _asghanId,
        currentTriple: basis.asghanTriple,
        nextTriple: basis.asghanTriple,
        archetypeChanged: false,
      );
      final candidate = _candidate(
        basis,
        displayName: nameOnly.displayName,
        input: basis.asghanInput,
        regenerateModule: false,
      );

      final prepared = AuthoringRevision3NpcProfileEditPreparation.fromJson(
        _response(nameOnly, candidate),
        currentProjectJson: basis.projectJson,
        request: nameOnly,
      );
      expect(prepared.scriptModuleRevision, _signedMax);
      expect(
        _entity(candidate.projectJson, _moduleId),
        _entity(basis.projectJson, _moduleId),
      );

      expect(
        () => _request(
          basis,
          displayName: basis.displayName,
          currentId: _asghanId,
          nextId: _viperId,
          currentTriple: basis.asghanTriple,
          nextTriple: basis.viperTriple,
          archetypeChanged: true,
        ),
        throwsFormatException,
      );
    },
  );
}

typedef _Basis = ({
  String projectJson,
  AuthoringWorkingHead head,
  String displayName,
  Map<String, Object?> asghanInput,
  Map<String, Object?> viperInput,
  AuthoringRevision3NpcProfileParentTripleExpectation asghanTriple,
  AuthoringRevision3NpcProfileParentTripleExpectation viperTriple,
  int moduleRevision,
});

typedef _Candidate = ({String projectJson, AuthoringWorkingHead head});

_Basis _basis({int moduleRevision = 0, bool withGreetings = false}) {
  final empty = _emptyProjectJson();
  final emptyHead = revision3NpcFixtureHead(empty);
  final asghanRequest = _draftRequest(
    projectJson: empty,
    head: emptyHead,
    parentCatalogId: _asghanId,
  );
  final created = Revision3NpcFixture.fromBasis(
    basisHead: emptyHead,
    basisProjectJson: empty,
    request: asghanRequest,
  );
  var projectJson = created.candidateProjectJson;
  var head = created.candidateHead;
  if (moduleRevision != 0 || withGreetings) {
    final project = _map(jsonDecode(projectJson));
    final entities = _map(project['entities']);
    if (moduleRevision != 0) {
      final module = _map(entities[_moduleId]);
      module['revision'] = moduleRevision;
    }
    if (withGreetings) {
      final npc = _map(entities[_npcId]);
      final payload = _map(npc['payload']);
      final data = _map(payload['data']);
      data['greetings'] = <Object?>[
        <String, Object?>{
          'line': <String, Object?>{
            'project_id': _projectId,
            'entity_id': '00000000000000000000000000000091',
            'expected_kind': 'dialog_line',
          },
        },
      ];
    }
    projectJson = jsonEncode(project);
    head = revision3NpcFixtureHead(projectJson);
  }
  final target = ((jsonDecode(empty) as Map)['target'] as Map)
      .cast<String, Object?>();
  final viperInput = revision3NpcFixtureInput(
    request: _draftRequest(
      projectJson: empty,
      head: emptyHead,
      parentCatalogId: _viperId,
    ),
    target: target,
  );
  final asghanInput = _npcInput(projectJson);
  return (
    projectJson: projectJson,
    head: head,
    displayName: asghanRequest.displayName,
    asghanInput: asghanInput,
    viperInput: viperInput,
    asghanTriple: _triple(asghanInput),
    viperTriple: _triple(viperInput),
    moduleRevision: moduleRevision,
  );
}

AuthoringRevision3NpcDraftRequestV1 _draftRequest({
  required String projectJson,
  required AuthoringWorkingHead head,
  required String parentCatalogId,
}) => AuthoringRevision3NpcDraftRequestV1.forProject(
  expectedHead: head,
  currentProjectJson: projectJson,
  npcId: _npcId,
  scriptModuleId: _moduleId,
  displayName: 'Managed Guard',
  intent: AuthoringRevision3NpcDraftIntentV1(
    moduleNamespace: 'GoreMods.Npcs.ManagedGuard',
    uniqueName: 'GoreManagedGuard',
    parentCatalogId: parentCatalogId,
  ),
);

AuthoringRevision3NpcProfileEditRequestV1 _request(
  _Basis basis, {
  required String displayName,
  required String currentId,
  required String nextId,
  required AuthoringRevision3NpcProfileParentTripleExpectation currentTriple,
  required AuthoringRevision3NpcProfileParentTripleExpectation nextTriple,
  required bool archetypeChanged,
}) {
  final seed = AuthoringRevision3NpcProfileEditSeed.forProject(
    head: basis.head,
    currentProjectJson: basis.projectJson,
    npcId: _npcId,
    expectedNpcRevision: 0,
    expectedScriptModuleId: _moduleId,
    expectedScriptModuleRevision: basis.moduleRevision,
    expectedUniqueName: 'GoreManagedGuard',
    expectedModuleNamespace: 'GoreMods.Npcs.ManagedGuard',
    expectedParentCharacterDefinition:
        currentTriple.characterDefinition.runtimeClass,
    expectedParentAiAgentConfig: currentTriple.aiAgentConfig.runtimeClass,
    expectedParentSpawnDefinition: currentTriple.spawnDefinition.runtimeClass,
  );
  return AuthoringRevision3NpcProfileEditRequestV1.forProject(
    expectedHead: basis.head,
    currentProjectJson: basis.projectJson,
    seed: seed,
    expectedStoryCatalogSeal: _seal('1'),
    expectedNpcCatalogSeal: _seal('2'),
    expectedParentCatalogId: currentId,
    expectedCurrentParentTriple: currentTriple,
    displayName: displayName,
    parentCatalogId: nextId,
    expectedParentTriple: nextTriple,
    expectedArchetypeChanged: archetypeChanged,
    expectedModuleRegenerated: archetypeChanged,
  );
}

_Candidate _candidate(
  _Basis basis, {
  required String displayName,
  required Map<String, Object?> input,
  required bool regenerateModule,
  bool mutateModule = false,
  bool dropGreetings = false,
}) {
  final project = (jsonDecode(basis.projectJson) as Map)
      .cast<String, Object?>();
  final entities = SplayTreeMap<String, Object?>.from(
    (project['entities']! as Map).cast<String, Object?>(),
  );
  final npc = _cloneMap(entities[_npcId]);
  npc['display_name'] = displayName;
  npc['revision'] = 1;
  if (dropGreetings) {
    final payload = _map(npc['payload']);
    final data = _map(payload['data']);
    data.remove('greetings');
  }
  if (regenerateModule) {
    final payload = _map(npc['payload']);
    final data = _map(payload['data']);
    data['input'] = input;
    payload['data'] = data;
    npc['payload'] = payload;
  }
  entities[_npcId] = npc;
  if (regenerateModule) {
    final request = _draftRequest(
      projectJson: _emptyProjectJson(),
      head: revision3NpcFixtureHead(_emptyProjectJson()),
      parentCatalogId: _viperId,
    );
    final module = revision3NpcFixtureModuleEntity(
      projectId: _projectId,
      request: request,
      input: input,
    );
    module['revision'] = basis.moduleRevision + 1;
    entities[_moduleId] = module;
  } else if (mutateModule) {
    final module = _cloneMap(entities[_moduleId]);
    module['display_name'] = 'Forged.Module';
    entities[_moduleId] = module;
  }
  project['revision'] = 2;
  project['entities'] = entities;
  final projectJson = jsonEncode(project);
  return (projectJson: projectJson, head: revision3NpcFixtureHead(projectJson));
}

Map<String, Object?> _response(
  AuthoringRevision3NpcProfileEditRequestV1 request,
  _Candidate candidate,
) => <String, Object?>{
  'ok': true,
  'outcome': 'prepared_unpublished',
  'basis_head_json': request.expectedHead.canonicalJson,
  'head_json': candidate.head.canonicalJson,
  'project_json': candidate.projectJson,
  'project_id': request.expectedProjectId,
  'revision': request.expectedRevision + 1,
  'npc_id': request.npcId,
  'npc_revision': request.expectedNpcRevision + 1,
  'script_module_id': request.scriptModuleId,
  'script_module_revision':
      request.expectedScriptModuleRevision +
      (request.expectsModuleRegenerated ? 1 : 0),
  'display_name': request.displayName,
  'previous_parent_catalog_id': request.expectedParentCatalogId,
  'parent_catalog_id': request.parentCatalogId,
  'story_catalog_seal': _sealJson(request.expectedStoryCatalogSeal),
  'npc_catalog_seal': _sealJson(request.expectedNpcCatalogSeal),
  'name_changed': request.expectsNameChanged,
  'archetype_changed': request.expectsArchetypeChanged,
  'module_regenerated': request.expectsModuleRegenerated,
  'build_status': 'blocked',
  'runtime_status': 'runtime_unqualified',
  'catalog_authority': 'not_granted',
  'collision_authority': 'not_granted',
  'publication_status': 'not_supported',
};

AuthoringRevision3NpcProfileParentTripleExpectation _triple(
  Map<String, Object?> input,
) => AuthoringRevision3NpcProfileParentTripleExpectation(
  characterDefinition: _parent(input, 'parent_character_definition'),
  aiAgentConfig: _parent(input, 'parent_ai_agent_config'),
  spawnDefinition: _parent(input, 'parent_spawn_definition'),
);

AuthoringRevision3NpcProfileParentExpectation _parent(
  Map<String, Object?> input,
  String field,
) {
  final parent = _map(input[field]);
  final seal = _map(parent['source_seal']);
  return AuthoringRevision3NpcProfileParentExpectation(
    catalogLayer: parent['catalog_layer']! as String,
    authoringSelector: parent['canonical_selector']! as String,
    runtimeClass: parent['runtime_class']! as String,
    sourceSeal: AuthoringDraftContentSeal.fromJson(seal),
  );
}

Map<String, Object?> _npcInput(String projectJson) {
  final npc = _entity(projectJson, _npcId);
  return _map(_map(_map(npc['payload'])['data'])['input']);
}

Map<String, Object?> _npcData(String projectJson) {
  final npc = _entity(projectJson, _npcId);
  return _map(_map(npc['payload'])['data']);
}

Map<String, Object?> _entity(String projectJson, String id) =>
    _map(_map((jsonDecode(projectJson) as Map)['entities'])[id]);

Map<String, Object?> _cloneMap(Object? value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Map<String, Object?> _map(Object? value) =>
    (value as Map).cast<String, Object?>();

AuthoringDraftContentSeal _seal(String digit) =>
    AuthoringDraftContentSeal.fromJson(<String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, digit).join(),
    });

Map<String, Object?> _sealJson(AuthoringDraftContentSeal seal) =>
    <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};

String _emptyProjectJson() => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': _projectId,
  'revision': 0,
  'meta': <String, Object?>{
    'name': 'R3 NPC profile adapter',
    'version': '1.0.0',
    'author': 'tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': _executableByteLength,
      'sha256': _executableSha256,
    },
  },
  'authoring_locales': <Object?>[],
  'entities': SplayTreeMap<String, Object?>(),
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});
