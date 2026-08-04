import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_quest_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';

const _root = r'C:\Projects\QuestOutline.goreproj';

void main() {
  test('Studio handshake requires the sorted Quest outline command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_quest_outline_edit_v2'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('V2 request carries exact stable slots and transition-plan seal', () {
    final fixture = Revision3QuestOutlineFixture();
    final request = _semanticRequest(fixture);
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;

    expect(wire.keys, <String>[
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
    ]);
    expect(
      (wire['objectives']! as List).map(
        (objective) => (objective as Map)['slot'],
      ),
      [3, 1, 2],
    );
    expect(wire, isNot(contains('game_root')));
    expect(wire, isNot(contains('transition_plan')));
  });

  test('V2 request accepts canonical single-objective Quest input', () {
    final fixture = Revision3QuestOutlineFixture(
      objectiveTitles: const <String>['Ask Asghan about Homer'],
    );
    final seal = AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
      1,
    ).contentSeal;

    final request = AuthoringRevision3QuestOutlineEditRequestV2.forProject(
      expectedHead: fixture.head,
      currentProjectJson: fixture.projectJson,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      expectedModuleId: revision3QuestOutlineModuleId,
      expectedModuleRevision: fixture.moduleRevision,
      expectedTransitionPlanSeal: seal,
      displayName: 'Find Homer safely',
      questTitle: 'Secure the old gate',
      objectives: const <AuthoringRevision3QuestOutlineObjectiveEditV2>[
        AuthoringRevision3QuestOutlineObjectiveEditV2(
          slot: 1,
          title: 'Ask Asghan about Homer',
        ),
      ],
    );

    expect(request.objectives.single.slot, 1);
  });

  test('V2 FFI accepts only the exact slot-preserving candidate', () async {
    final fixture = Revision3QuestOutlineFixture();
    final request = _semanticRequest(fixture);
    final response = _semanticResponse(fixture, request);
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_store_prepare_revision3_quest_outline_edit_v2': response,
      },
    );

    final prepared = await ModFfi(core)
        .authoringStorePrepareRevision3QuestOutlineEditV2(
          root: _root,
          currentProjectJson: fixture.projectJson,
          request: request,
        );

    expect(prepared.revision, fixture.projectRevision + 1);
    expect(prepared.questRevision, fixture.questRevision + 1);
    expect(prepared.moduleRevision, fixture.moduleRevision + 1);
    expect(
      prepared.buildStatus,
      AuthoringRevision3QuestOutlineBuildStatus.blocked,
    );
    expect(
      prepared.runtimeStatus,
      AuthoringRevision3QuestOutlineRuntimeStatus.runtimeUnqualified,
    );
    expect(
      prepared.publicationStatus,
      AuthoringRevision3QuestOutlinePublicationStatus.notSupported,
    );
    expect(
      core.calls.single.command,
      'authoring_store_prepare_revision3_quest_outline_edit_v2',
    );

    final tampered = Map<String, Object?>.from(response);
    final candidate = (jsonDecode(tampered['project_json']! as String) as Map)
        .cast<String, Object?>();
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final quest = (entities[revision3QuestOutlineQuestId]! as Map)
        .cast<String, Object?>();
    final data = (((quest['payload']! as Map)['data']! as Map))
        .cast<String, Object?>();
    final input = (data['input']! as Map).cast<String, Object?>();
    final plan = (input['transition_plan']! as Map).cast<String, Object?>();
    plan['next_slot_ordinal'] = 99;
    tampered['project_json'] = jsonEncode(candidate);

    await expectLater(
      ModFfi(
        FakeGoreCoreFfiService(
          responses: {
            'authoring_store_prepare_revision3_quest_outline_edit_v2': tampered,
          },
        ),
      ).authoringStorePrepareRevision3QuestOutlineEditV2(
        root: _root,
        currentProjectJson: fixture.projectJson,
        request: request,
      ),
      throwsA(
        isA<ModFfiException>().having(
          (error) => error.code,
          'code',
          ModFfiException.malformedNativeResponseCode,
        ),
      ),
    );
  });
}

AuthoringRevision3QuestOutlineEditRequestV2 _semanticRequest(
  Revision3QuestOutlineFixture fixture,
) {
  final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
    currentProjectJson: fixture.projectJson,
    questId: revision3QuestOutlineQuestId,
    expectedQuestRevision: fixture.questRevision,
    expectedModuleId: revision3QuestOutlineModuleId,
    expectedModuleRevision: fixture.moduleRevision,
  );
  return AuthoringRevision3QuestOutlineEditRequestV2.forProject(
    expectedHead: fixture.head,
    currentProjectJson: fixture.projectJson,
    questId: revision3QuestOutlineQuestId,
    expectedQuestRevision: fixture.questRevision,
    expectedModuleId: revision3QuestOutlineModuleId,
    expectedModuleRevision: fixture.moduleRevision,
    expectedTransitionPlanSeal: seed.transitionPlanSeal,
    displayName: 'Find Homer safely',
    questTitle: 'Secure the old gate',
    objectives: const [
      AuthoringRevision3QuestOutlineObjectiveEditV2(
        slot: 3,
        title: 'Report the secured gate',
      ),
      AuthoringRevision3QuestOutlineObjectiveEditV2(
        slot: 1,
        title: 'Ask Asghan about Homer',
      ),
      AuthoringRevision3QuestOutlineObjectiveEditV2(
        slot: 2,
        title: 'Inspect the old gate',
      ),
    ],
  );
}

Map<String, Object?> _semanticResponse(
  Revision3QuestOutlineFixture fixture,
  AuthoringRevision3QuestOutlineEditRequestV2 request,
) {
  final candidate = (jsonDecode(fixture.projectJson) as Map)
      .cast<String, Object?>();
  candidate['revision'] = fixture.projectRevision + 1;
  final entities = (candidate['entities']! as Map).cast<String, Object?>();
  final quest = (entities[revision3QuestOutlineQuestId]! as Map)
      .cast<String, Object?>();
  quest['revision'] = fixture.questRevision + 1;
  quest['display_name'] = request.displayName;
  final questData = (((quest['payload']! as Map)['data']! as Map))
      .cast<String, Object?>();
  final input = (questData['input']! as Map).cast<String, Object?>();
  input['title'] = request.questTitle;
  input['objective_title'] = request.objectives.first.title;
  if (request.objectives.length > 1) {
    input['additional_objective_titles'] = [
      for (final objective in request.objectives.skip(1)) objective.title,
    ];
  } else {
    input.remove('additional_objective_titles');
  }
  final plan = (input['transition_plan']! as Map).cast<String, Object?>();
  plan['objective_order'] = [
    for (final objective in request.objectives) objective.slot,
  ];

  final module = (entities[revision3QuestOutlineModuleId]! as Map)
      .cast<String, Object?>();
  module['revision'] = fixture.moduleRevision + 1;
  final moduleData = ((((module['payload']! as Map)['data']! as Map)))
      .cast<String, Object?>();
  final source = '${moduleData['source']}\n// outline-v2 fixture';
  moduleData['source'] = source;
  moduleData['source_sha256'] = crypto.sha256
      .convert(utf8.encode(source))
      .toString();
  moduleData['input_fingerprint'] = revision3QuestInputFingerprint(input);
  final projectJson = jsonEncode(candidate);
  final planSeal = AuthoringRevision3QuestTransitionPlanV1.fromJson(
    plan,
  ).contentSeal;
  return <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': fixture.head.canonicalJson,
    'head_json': headFor(projectJson).canonicalJson,
    'project_json': projectJson,
    'project_id': revision3QuestOutlineProjectId,
    'revision': fixture.projectRevision + 1,
    'quest_id': revision3QuestOutlineQuestId,
    'module_id': revision3QuestOutlineModuleId,
    'quest_revision': fixture.questRevision + 1,
    'module_revision': fixture.moduleRevision + 1,
    'transition_plan_seal': <String, Object?>{
      'byte_len': planSeal.byteLength,
      'sha256': planSeal.sha256,
    },
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
}
