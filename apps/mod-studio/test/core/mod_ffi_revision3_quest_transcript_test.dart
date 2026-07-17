import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _root = r'C:\Projects\QuestTranscript.goreproj';
const _lineId = '44444444444444444444444444444444';
const _localizationId = '55555555555555555555555555555555';
const _createdLineId = '66666666666666666666666666666666';
const _createdLocalizationId = '77777777777777777777777777777777';

void main() {
  test('Studio handshake requires the sorted Quest transcript command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_quest_transcript_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('replace request is canonical, exact-bound and stable-slot aware', () {
    final basis = _basisProjectJson();
    final fixture = Revision3QuestOutlineFixture();
    final request = AuthoringRevision3QuestTranscriptRequestV1.forProject(
      expectedHead: fixture.head,
      currentProjectJson: basis,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      intent: AuthoringRevision3QuestTranscriptReplaceIntentV1(
        bindings: <AuthoringRevision3QuestTranscriptBindingV1>[
          AuthoringRevision3QuestTranscriptBindingV1(
            projectId: revision3QuestOutlineProjectId,
            lineId: _lineId,
            objectiveSlot: 1,
          ),
        ],
      ),
    );
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;
    expect(wire.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'quest_id',
      'expected_quest_revision',
      'intent',
    ]);
    final intent = (wire['intent']! as Map).cast<String, Object?>();
    expect(intent.keys, <String>['mode', 'bindings']);
    final binding = ((intent['bindings']! as List).single as Map)
        .cast<String, Object?>();
    expect(binding.keys, <String>['line', 'objective_slot']);
    expect(binding['objective_slot'], 1);
    expect(request.moduleId, revision3QuestOutlineModuleId);
    expect(request.expectedModuleRevision, fixture.moduleRevision);
  });

  test('request rejects duplicate lines, inactive slots and no-op order', () {
    final basis = _basisProjectJson();
    final fixture = Revision3QuestOutlineFixture();
    AuthoringRevision3QuestTranscriptBindingV1 binding(int? slot) =>
        AuthoringRevision3QuestTranscriptBindingV1(
          projectId: revision3QuestOutlineProjectId,
          lineId: _lineId,
          objectiveSlot: slot,
        );
    expect(
      () => AuthoringRevision3QuestTranscriptReplaceIntentV1(
        bindings: <AuthoringRevision3QuestTranscriptBindingV1>[
          binding(1),
          binding(2),
        ],
      ),
      throwsFormatException,
    );
    expect(
      () => AuthoringRevision3QuestTranscriptRequestV1.forProject(
        expectedHead: fixture.head,
        currentProjectJson: basis,
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        intent: AuthoringRevision3QuestTranscriptReplaceIntentV1(
          bindings: <AuthoringRevision3QuestTranscriptBindingV1>[binding(99)],
        ),
      ),
      throwsFormatException,
    );
  });

  test(
    'FFI accepts only the exact transcript delta and sends no game input',
    () async {
      final basis = _basisProjectJson();
      final fixture = Revision3QuestOutlineFixture();
      final binding = AuthoringRevision3QuestTranscriptBindingV1(
        projectId: revision3QuestOutlineProjectId,
        lineId: _lineId,
        objectiveSlot: 1,
      );
      final request = AuthoringRevision3QuestTranscriptRequestV1.forProject(
        expectedHead: fixture.head,
        currentProjectJson: basis,
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        intent: AuthoringRevision3QuestTranscriptReplaceIntentV1(
          bindings: <AuthoringRevision3QuestTranscriptBindingV1>[binding],
        ),
      );
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_quest_transcript_v1':
              _replaceResponse(basis, fixture, binding),
        },
      );

      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3QuestTranscriptV1(
            root: _root,
            currentProjectJson: basis,
            request: request,
          );

      expect(prepared.mode, AuthoringRevision3QuestTranscriptMode.replace);
      expect(prepared.transcriptCount, 1);
      expect(prepared.questRevision, fixture.questRevision + 1);
      expect(prepared.moduleRevision, fixture.moduleRevision);
      expect(prepared.createdLineId, isNull);
      final call = core.calls.single;
      expect(
        call.command,
        'authoring_store_prepare_revision3_quest_transcript_v1',
      );
      expect(call.payload.keys, <String>[
        'current_project_json',
        'quest_transcript_request_json',
        'root',
      ]);
      expect(call.payload, isNot(contains('game_root')));
    },
  );

  test('preparation rejects a preserved module revision change', () {
    final basis = _basisProjectJson();
    final fixture = Revision3QuestOutlineFixture();
    final binding = AuthoringRevision3QuestTranscriptBindingV1(
      projectId: revision3QuestOutlineProjectId,
      lineId: _lineId,
      objectiveSlot: 1,
    );
    final request = AuthoringRevision3QuestTranscriptRequestV1.forProject(
      expectedHead: fixture.head,
      currentProjectJson: basis,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      intent: AuthoringRevision3QuestTranscriptReplaceIntentV1(
        bindings: <AuthoringRevision3QuestTranscriptBindingV1>[binding],
      ),
    );
    final response = _replaceResponse(basis, fixture, binding);
    final candidate =
        jsonDecode(response['project_json']! as String) as Map<String, Object?>;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final module = (entities[revision3QuestOutlineModuleId]! as Map)
        .cast<String, Object?>();
    module['revision'] = fixture.moduleRevision + 1;
    response['project_json'] = jsonEncode(candidate);
    response['module_revision'] = fixture.moduleRevision + 1;

    expect(
      () => AuthoringRevision3QuestTranscriptPreparation.fromJson(
        response,
        currentProjectJson: basis,
        request: request,
      ),
      throwsFormatException,
    );
  });

  test('create-and-insert proves the embedded DialogLine delta atomically', () {
    final basis = _basisProjectJson();
    final fixture = Revision3QuestOutlineFixture();
    final line = AuthoringRevision3DialogLineEntryRequestV1.forProject(
      expectedHead: fixture.head,
      currentProjectJson: basis,
      lineId: _createdLineId,
      lineDisplayName: 'New warning',
      lineAuthoredIdentity: 'DIA_NEW_WARNING_LINE',
      speakerHint: 'Asghan',
      localization: AuthoringRevision3DialogLocalizationCreateIntentV1(
        localizationId: _createdLocalizationId,
        displayName: 'New warning text',
        locId: 'DIA_NEW_WARNING',
        texts: const <String, String>{'de': 'Bleib vom Tor weg.'},
      ),
    );
    final request = AuthoringRevision3QuestTranscriptRequestV1.forProject(
      expectedHead: fixture.head,
      currentProjectJson: basis,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      intent: AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1(
        index: 0,
        objectiveSlot: 2,
        line: line,
      ),
    );
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;
    final intent = (wire['intent']! as Map).cast<String, Object?>();
    expect(intent.keys, <String>['mode', 'index', 'objective_slot', 'line']);

    final prepared = AuthoringRevision3QuestTranscriptPreparation.fromJson(
      _createResponse(basis, fixture),
      currentProjectJson: basis,
      request: request,
    );

    expect(
      prepared.mode,
      AuthoringRevision3QuestTranscriptMode.createAndInsert,
    );
    expect(prepared.createdLineId, _createdLineId);
    expect(prepared.createdLocalizationId, _createdLocalizationId);
    expect(
      prepared.localizationAction,
      AuthoringRevision3DialogLocalizationAction.created,
    );
    expect(prepared.transcriptCount, 1);
  });
}

String _basisProjectJson() {
  final project =
      jsonDecode(Revision3QuestOutlineFixture().semanticProjectJson)
          as Map<String, Object?>;
  project['authoring_locales'] = <Object?>['de'];
  final entities = (project['entities']! as Map).cast<String, Object?>();
  entities[_localizationId] = <String, Object?>{
    'id': _localizationId,
    'display_name': 'Gate warning text',
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': 'DIA_GATE_WARNING',
    },
    'revision': 2,
    'payload': <String, Object?>{
      'kind': 'localization_entry',
      'data': <String, Object?>{
        'loc_id': 'DIA_GATE_WARNING',
        'texts': <String, Object?>{'de': 'Das Tor ist gesichert.'},
      },
    },
  };
  entities[_lineId] = <String, Object?>{
    'id': _lineId,
    'display_name': 'Gate warning',
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': 'DIA_GATE_WARNING_LINE',
    },
    'revision': 3,
    'payload': <String, Object?>{
      'kind': 'dialog_line',
      'data': <String, Object?>{
        'localization': <String, Object?>{
          'project_id': revision3QuestOutlineProjectId,
          'id': _localizationId,
          'expected_kind': 'localization_entry',
        },
        'speaker_hint': 'Asghan',
        'voice_slots': <String, Object?>{},
      },
    },
  };
  final ids = entities.keys.toList()..sort();
  project['entities'] = <String, Object?>{
    for (final id in ids) id: entities[id],
  };
  return jsonEncode(project);
}

Map<String, Object?> _replaceResponse(
  String basisJson,
  Revision3QuestOutlineFixture fixture,
  AuthoringRevision3QuestTranscriptBindingV1 binding,
) {
  final candidate = jsonDecode(basisJson) as Map<String, Object?>;
  candidate['revision'] = fixture.projectRevision + 1;
  final entities = (candidate['entities']! as Map).cast<String, Object?>();
  final quest = (entities[revision3QuestOutlineQuestId]! as Map)
      .cast<String, Object?>();
  quest['revision'] = fixture.questRevision + 1;
  final payload = (quest['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['transcript'] = <Object?>[
    <String, Object?>{
      'line': <String, Object?>{
        'project_id': binding.projectId,
        'id': binding.lineId,
        'expected_kind': 'dialog_line',
      },
      'objective_slot': binding.objectiveSlot,
    },
  ];
  return <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': fixture.head.canonicalJson,
    'head_json': manifestHead(4100, 'c').canonicalJson,
    'project_json': jsonEncode(candidate),
    'project_id': revision3QuestOutlineProjectId,
    'revision': fixture.projectRevision + 1,
    'quest_id': revision3QuestOutlineQuestId,
    'quest_revision': fixture.questRevision + 1,
    'module_id': revision3QuestOutlineModuleId,
    'module_revision': fixture.moduleRevision,
    'mode': 'replace',
    'transcript_count': 1,
    'created_line_id': null,
    'created_localization_id': null,
    'created_voice_slot_id': null,
    'localization_action': null,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'topic_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}

Map<String, Object?> _createResponse(
  String basisJson,
  Revision3QuestOutlineFixture fixture,
) {
  final candidate = jsonDecode(basisJson) as Map<String, Object?>;
  candidate['revision'] = fixture.projectRevision + 1;
  final entities = (candidate['entities']! as Map).cast<String, Object?>();
  final quest = (entities[revision3QuestOutlineQuestId]! as Map)
      .cast<String, Object?>();
  quest['revision'] = fixture.questRevision + 1;
  final payload = (quest['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['transcript'] = <Object?>[
    <String, Object?>{
      'line': <String, Object?>{
        'project_id': revision3QuestOutlineProjectId,
        'id': _createdLineId,
        'expected_kind': 'dialog_line',
      },
      'objective_slot': 2,
    },
  ];
  entities[_createdLocalizationId] = <String, Object?>{
    'id': _createdLocalizationId,
    'display_name': 'New warning text',
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': 'DIA_NEW_WARNING',
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'localization_entry',
      'data': <String, Object?>{
        'loc_id': 'DIA_NEW_WARNING',
        'texts': <String, Object?>{'de': 'Bleib vom Tor weg.'},
      },
    },
  };
  entities[_createdLineId] = <String, Object?>{
    'id': _createdLineId,
    'display_name': 'New warning',
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': 'DIA_NEW_WARNING_LINE',
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'dialog_line',
      'data': <String, Object?>{
        'localization': <String, Object?>{
          'project_id': revision3QuestOutlineProjectId,
          'id': _createdLocalizationId,
          'expected_kind': 'localization_entry',
        },
        'speaker_hint': 'Asghan',
        'voice_slots': <String, Object?>{},
      },
    },
  };
  final ids = entities.keys.toList()..sort();
  candidate['entities'] = <String, Object?>{
    for (final id in ids) id: entities[id],
  };
  return <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': fixture.head.canonicalJson,
    'head_json': manifestHead(4102, 'd').canonicalJson,
    'project_json': jsonEncode(candidate),
    'project_id': revision3QuestOutlineProjectId,
    'revision': fixture.projectRevision + 1,
    'quest_id': revision3QuestOutlineQuestId,
    'quest_revision': fixture.questRevision + 1,
    'module_id': revision3QuestOutlineModuleId,
    'module_revision': fixture.moduleRevision,
    'mode': 'create_and_insert',
    'transcript_count': 1,
    'created_line_id': _createdLineId,
    'created_localization_id': _createdLocalizationId,
    'created_voice_slot_id': null,
    'localization_action': 'created',
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'topic_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}
