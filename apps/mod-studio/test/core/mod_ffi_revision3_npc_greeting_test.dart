import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_npc_fixture.dart';
import '../support/revision3_npc_profile_edit_fixture.dart';

const _root = r'C:\Projects\NpcGreeting.goreproj';
const _lineId = '44444444444444444444444444444444';
const _localizationId = '55555555555555555555555555555555';
const _createdLineId = '66666666666666666666666666666666';
const _createdLocalizationId = '77777777777777777777777777777777';

void main() {
  test('Studio handshake requires the sorted NPC greeting command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_npc_greeting_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('replace request is canonical and exact-bound', () async {
    final basis = _basisProjectJson();
    final head = revision3NpcFixtureHead(basis);
    final binding = AuthoringRevision3NpcGreetingBindingV1(
      projectId: revision3NpcProfileProjectId,
      lineId: _lineId,
    );
    final request = AuthoringRevision3NpcGreetingRequestV1.forProject(
      expectedHead: head,
      currentProjectJson: basis,
      npcId: revision3NpcProfileNpcId,
      expectedNpcRevision: 0,
      intent: AuthoringRevision3NpcGreetingReplaceIntentV1(
        bindings: <AuthoringRevision3NpcGreetingBindingV1>[binding],
      ),
    );
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;
    expect(wire.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'npc_id',
      'expected_npc_revision',
      'intent',
    ]);
    final intent = (wire['intent']! as Map).cast<String, Object?>();
    expect(intent.keys, <String>['mode', 'bindings']);
    final encodedBinding = ((intent['bindings']! as List).single as Map)
        .cast<String, Object?>();
    expect(encodedBinding.keys, <String>['line']);
    expect(request.moduleId, revision3NpcProfileModuleId);
    expect(request.expectedModuleRevision, 0);
    expect(request.expectedGreetingCount, 0);

    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_prepare_revision3_npc_greeting_v1': _replaceResponse(
          basis,
          head,
          binding,
        ),
      },
    );
    final prepared = await ModFfi(core)
        .authoringStorePrepareRevision3NpcGreetingV1(
          root: _root,
          currentProjectJson: basis,
          request: request,
        );

    expect(prepared.mode, AuthoringRevision3NpcGreetingMode.replace);
    expect(prepared.greetingCount, 1);
    expect(prepared.npcRevision, 1);
    expect(prepared.moduleRevision, 0);
    expect(prepared.createdLineId, isNull);
    final call = core.calls.single;
    expect(call.command, 'authoring_store_prepare_revision3_npc_greeting_v1');
    expect(call.payload.keys, <String>[
      'current_project_json',
      'npc_greeting_request_json',
      'root',
    ]);
    expect(call.payload, isNot(contains('game_root')));
  });

  test('request rejects duplicate and no-op greeting lists', () {
    final basis = _basisProjectJson(withGreeting: true);
    final head = revision3NpcFixtureHead(basis);
    AuthoringRevision3NpcGreetingBindingV1 binding() =>
        AuthoringRevision3NpcGreetingBindingV1(
          projectId: revision3NpcProfileProjectId,
          lineId: _lineId,
        );
    expect(
      () => AuthoringRevision3NpcGreetingReplaceIntentV1(
        bindings: <AuthoringRevision3NpcGreetingBindingV1>[
          binding(),
          binding(),
        ],
      ),
      throwsFormatException,
    );
    expect(
      () => AuthoringRevision3NpcGreetingRequestV1.forProject(
        expectedHead: head,
        currentProjectJson: basis,
        npcId: revision3NpcProfileNpcId,
        expectedNpcRevision: 0,
        intent: AuthoringRevision3NpcGreetingReplaceIntentV1(
          bindings: <AuthoringRevision3NpcGreetingBindingV1>[binding()],
        ),
      ),
      throwsFormatException,
    );
  });

  test(
    'unchanged ScriptModule may already be at the signed revision limit',
    () {
      final project = (jsonDecode(_basisProjectJson()) as Map)
          .cast<String, Object?>();
      final entities = (project['entities']! as Map).cast<String, Object?>();
      final module = (entities[revision3NpcProfileModuleId]! as Map)
          .cast<String, Object?>();
      module['revision'] = 0x7fffffffffffffff;
      final basis = jsonEncode(project);
      final request = AuthoringRevision3NpcGreetingRequestV1.forProject(
        expectedHead: revision3NpcFixtureHead(basis),
        currentProjectJson: basis,
        npcId: revision3NpcProfileNpcId,
        expectedNpcRevision: 0,
        intent: AuthoringRevision3NpcGreetingReplaceIntentV1(
          bindings: <AuthoringRevision3NpcGreetingBindingV1>[
            AuthoringRevision3NpcGreetingBindingV1(
              projectId: revision3NpcProfileProjectId,
              lineId: _lineId,
            ),
          ],
        ),
      );

      expect(request.expectedModuleRevision, 0x7fffffffffffffff);
    },
  );

  test('preparation rejects a preserved ScriptModule revision change', () {
    final basis = _basisProjectJson();
    final head = revision3NpcFixtureHead(basis);
    final binding = AuthoringRevision3NpcGreetingBindingV1(
      projectId: revision3NpcProfileProjectId,
      lineId: _lineId,
    );
    final request = AuthoringRevision3NpcGreetingRequestV1.forProject(
      expectedHead: head,
      currentProjectJson: basis,
      npcId: revision3NpcProfileNpcId,
      expectedNpcRevision: 0,
      intent: AuthoringRevision3NpcGreetingReplaceIntentV1(
        bindings: <AuthoringRevision3NpcGreetingBindingV1>[binding],
      ),
    );
    final response = _replaceResponse(basis, head, binding);
    final candidate =
        jsonDecode(response['project_json']! as String) as Map<String, Object?>;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final module = (entities[revision3NpcProfileModuleId]! as Map)
        .cast<String, Object?>();
    module['revision'] = 1;
    response['project_json'] = jsonEncode(candidate);
    response['module_revision'] = 1;

    expect(
      () => AuthoringRevision3NpcGreetingPreparation.fromJson(
        response,
        currentProjectJson: basis,
        request: request,
      ),
      throwsFormatException,
    );

    final npcResponse = _replaceResponse(basis, head, binding);
    final npcCandidate =
        jsonDecode(npcResponse['project_json']! as String)
            as Map<String, Object?>;
    final npcEntities = (npcCandidate['entities']! as Map)
        .cast<String, Object?>();
    final npc = (npcEntities[revision3NpcProfileNpcId]! as Map)
        .cast<String, Object?>();
    npc['display_name'] = 'Unrequested rename';
    npcResponse['project_json'] = jsonEncode(npcCandidate);
    expect(
      () => AuthoringRevision3NpcGreetingPreparation.fromJson(
        npcResponse,
        currentProjectJson: basis,
        request: request,
      ),
      throwsFormatException,
    );
  });

  test('create-and-insert proves the embedded DialogLine delta atomically', () {
    final basis = _basisProjectJson();
    final head = revision3NpcFixtureHead(basis);
    final line = AuthoringRevision3DialogLineEntryRequestV1.forProject(
      expectedHead: head,
      currentProjectJson: basis,
      lineId: _createdLineId,
      lineDisplayName: 'New greeting',
      lineAuthoredIdentity: 'DIA_NEW_GREETING_LINE',
      speakerHint: 'Asghan',
      localization: AuthoringRevision3DialogLocalizationCreateIntentV1(
        localizationId: _createdLocalizationId,
        displayName: 'New greeting text',
        locId: 'DIA_NEW_GREETING',
        texts: const <String, String>{'de': 'Halt! Wer da?'},
      ),
    );
    final request = AuthoringRevision3NpcGreetingRequestV1.forProject(
      expectedHead: head,
      currentProjectJson: basis,
      npcId: revision3NpcProfileNpcId,
      expectedNpcRevision: 0,
      intent: AuthoringRevision3NpcGreetingCreateAndInsertIntentV1(
        index: 0,
        line: line,
      ),
    );
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;
    final intent = (wire['intent']! as Map).cast<String, Object?>();
    expect(intent.keys, <String>['mode', 'index', 'line']);

    final prepared = AuthoringRevision3NpcGreetingPreparation.fromJson(
      _createResponse(basis, head),
      currentProjectJson: basis,
      request: request,
    );
    expect(prepared.mode, AuthoringRevision3NpcGreetingMode.createAndInsert);
    expect(prepared.createdLineId, _createdLineId);
    expect(prepared.createdLocalizationId, _createdLocalizationId);
    expect(
      prepared.localizationAction,
      AuthoringRevision3DialogLocalizationAction.created,
    );
    expect(prepared.greetingCount, 1);
    expect(prepared.buildStatus, AuthoringRevision3DialogBuildStatus.blocked);
    expect(
      prepared.runtimeStatus,
      AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified,
    );
  });
}

String _basisProjectJson({bool withGreeting = false}) {
  final fixture = Revision3NpcProfileTestFixture.create();
  final project = jsonDecode(fixture.projectJson) as Map<String, Object?>;
  project['authoring_locales'] = <Object?>['de'];
  final entities = (project['entities']! as Map).cast<String, Object?>();
  entities[_localizationId] = _localizationEntity(
    id: _localizationId,
    displayName: 'Gate greeting text',
    authoredId: 'DIA_GATE_GREETING',
    text: 'Du kommst hier nicht durch.',
    revision: 2,
  );
  entities[_lineId] = _lineEntity(
    id: _lineId,
    localizationId: _localizationId,
    displayName: 'Gate greeting',
    authoredId: 'DIA_GATE_GREETING_LINE',
    revision: 3,
  );
  if (withGreeting) {
    final npc = (entities[revision3NpcProfileNpcId]! as Map)
        .cast<String, Object?>();
    final payload = (npc['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    data['greetings'] = <Object?>[_greetingBinding(_lineId)];
  }
  final ids = entities.keys.toList()..sort();
  project['entities'] = <String, Object?>{
    for (final id in ids) id: entities[id],
  };
  return jsonEncode(project);
}

Map<String, Object?> _replaceResponse(
  String basisJson,
  AuthoringWorkingHead basisHead,
  AuthoringRevision3NpcGreetingBindingV1 binding,
) {
  final candidate = jsonDecode(basisJson) as Map<String, Object?>;
  candidate['revision'] = (candidate['revision']! as int) + 1;
  final entities = (candidate['entities']! as Map).cast<String, Object?>();
  final npc = (entities[revision3NpcProfileNpcId]! as Map)
      .cast<String, Object?>();
  npc['revision'] = (npc['revision']! as int) + 1;
  final payload = (npc['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['greetings'] = <Object?>[_greetingBinding(binding.lineId)];
  final candidateJson = jsonEncode(candidate);
  return _response(
    basisHead: basisHead,
    candidateJson: candidateJson,
    mode: 'replace',
    createdLineId: null,
    createdLocalizationId: null,
    localizationAction: null,
  );
}

Map<String, Object?> _createResponse(
  String basisJson,
  AuthoringWorkingHead basisHead,
) {
  final candidate = jsonDecode(basisJson) as Map<String, Object?>;
  candidate['revision'] = (candidate['revision']! as int) + 1;
  final entities = (candidate['entities']! as Map).cast<String, Object?>();
  final npc = (entities[revision3NpcProfileNpcId]! as Map)
      .cast<String, Object?>();
  npc['revision'] = (npc['revision']! as int) + 1;
  final payload = (npc['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['greetings'] = <Object?>[_greetingBinding(_createdLineId)];
  entities[_createdLocalizationId] = _localizationEntity(
    id: _createdLocalizationId,
    displayName: 'New greeting text',
    authoredId: 'DIA_NEW_GREETING',
    text: 'Halt! Wer da?',
    revision: 0,
  );
  entities[_createdLineId] = _lineEntity(
    id: _createdLineId,
    localizationId: _createdLocalizationId,
    displayName: 'New greeting',
    authoredId: 'DIA_NEW_GREETING_LINE',
    revision: 0,
  );
  final ids = entities.keys.toList()..sort();
  candidate['entities'] = <String, Object?>{
    for (final id in ids) id: entities[id],
  };
  final candidateJson = jsonEncode(candidate);
  return _response(
    basisHead: basisHead,
    candidateJson: candidateJson,
    mode: 'create_and_insert',
    createdLineId: _createdLineId,
    createdLocalizationId: _createdLocalizationId,
    localizationAction: 'created',
  );
}

Map<String, Object?> _response({
  required AuthoringWorkingHead basisHead,
  required String candidateJson,
  required String mode,
  required String? createdLineId,
  required String? createdLocalizationId,
  required String? localizationAction,
}) {
  final candidate = jsonDecode(candidateJson) as Map<String, Object?>;
  return <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': basisHead.canonicalJson,
    'head_json': revision3NpcFixtureHead(candidateJson).canonicalJson,
    'project_json': candidateJson,
    'project_id': revision3NpcProfileProjectId,
    'revision': candidate['revision'],
    'npc_id': revision3NpcProfileNpcId,
    'npc_revision': 1,
    'module_id': revision3NpcProfileModuleId,
    'module_revision': 0,
    'mode': mode,
    'greeting_count': 1,
    'created_line_id': createdLineId,
    'created_localization_id': createdLocalizationId,
    'created_voice_slot_id': null,
    'localization_action': localizationAction,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'topic_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}

Map<String, Object?> _greetingBinding(String lineId) => <String, Object?>{
  'line': <String, Object?>{
    'project_id': revision3NpcProfileProjectId,
    'id': lineId,
    'expected_kind': 'dialog_line',
  },
};

Map<String, Object?> _localizationEntity({
  required String id,
  required String displayName,
  required String authoredId,
  required String text,
  required int revision,
}) => <String, Object?>{
  'id': id,
  'display_name': displayName,
  'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': authoredId},
  'revision': revision,
  'payload': <String, Object?>{
    'kind': 'localization_entry',
    'data': <String, Object?>{
      'loc_id': authoredId,
      'texts': <String, Object?>{'de': text},
    },
  },
};

Map<String, Object?> _lineEntity({
  required String id,
  required String localizationId,
  required String displayName,
  required String authoredId,
  required int revision,
}) => <String, Object?>{
  'id': id,
  'display_name': displayName,
  'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': authoredId},
  'revision': revision,
  'payload': <String, Object?>{
    'kind': 'dialog_line',
    'data': <String, Object?>{
      'localization': <String, Object?>{
        'project_id': revision3NpcProfileProjectId,
        'id': localizationId,
        'expected_kind': 'localization_entry',
      },
      'speaker_hint': 'Asghan',
      'voice_slots': <String, Object?>{},
    },
  },
};
