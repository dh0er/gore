import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_quest_outline_fixture.dart';
import '../support/revision3_quest_fixture.dart';

const _root = r'C:\Projects\QuestTransitions.goreproj';

void main() {
  const canonicalWire =
      '{"objective_slots":[1],"objective_order":[1],"next_slot_ordinal":2,'
      '"transitions":[{"node":{"kind":"root"},"edge":"availability",'
      '"external_allowed":true},{"node":{"kind":"root"},"edge":"start",'
      '"external_allowed":true},{"node":{"kind":"objective","slot":1},'
      '"edge":"availability","external_allowed":true},{"node":'
      '{"kind":"objective","slot":1},"edge":"start","external_allowed":'
      'true},{"node":{"kind":"objective","slot":1},"edge":"success",'
      '"external_allowed":true,"succeeds_parent":true}]}';

  test('parses and preserves the canonical one-objective plan', () {
    final plan = AuthoringRevision3QuestTransitionPlanV1.fromJson(
      jsonDecode(canonicalWire),
    );

    expect(plan.objectiveSlots, [1]);
    expect(plan.objectiveOrder, [1]);
    expect(plan.nextSlotOrdinal, 2);
    expect(plan.transitions, hasLength(5));
    expect(plan.canonicalJson, canonicalWire);
    expect(plan.contentSeal.byteLength, 484);
    expect(
      plan.contentSeal.sha256,
      'fabcf2b6513300759fac3bcbac823254e52b59a970f3a7735087aad92a21f1e4',
    );
  });

  test('builds the default multi-objective plan in slot order', () {
    final plan = AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
      3,
    );

    expect(plan.objectiveSlots, [1, 2, 3]);
    expect(plan.objectiveOrder, [1, 2, 3]);
    expect(plan.nextSlotOrdinal, 4);
    expect(plan.transitions, hasLength(11));
    expect(
      plan.transitions.where((transition) => transition.succeedsParent),
      hasLength(1),
    );
    expect(plan.transitions.last.node.slot, 3);
    expect(plan.transitions.last.succeedsParent, isTrue);
  });

  test('keeps external triggers independent from predicates', () {
    final wire = jsonDecode(canonicalWire) as Map<String, Object?>;
    final transitions = wire['transitions']! as List<Object?>;
    final success = transitions.last! as Map<String, Object?>;
    success['predicate'] = {
      'any_of': [
        {
          'all_of': [
            {
              'node': {'kind': 'root'},
              'test': 'running',
              'negated': false,
            },
          ],
        },
      ],
    };
    final plan = AuthoringRevision3QuestTransitionPlanV1.fromJson(wire);

    expect(plan.transitions.last.externalAllowed, isTrue);
    expect(plan.transitions.last.predicate, isNotNull);
    expect(
      jsonDecode(plan.canonicalJson),
      jsonDecode(jsonEncode(plan.toJson())),
    );
  });

  test('rejects order that is not a full stable-slot permutation', () {
    expect(
      () => AuthoringRevision3QuestTransitionPlanV1.fromJson({
        'objective_slots': [1, 3],
        'objective_order': [1, 1],
        'next_slot_ordinal': 4,
        'transitions': [
          {
            'node': {'kind': 'root'},
            'edge': 'start',
            'external_allowed': true,
          },
        ],
      }),
      throwsFormatException,
    );
  });

  test('requires objective slot 1 to remain active', () {
    final plan = AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
      1,
    );

    expect(
      () => AuthoringRevision3QuestTransitionPlanV1(
        objectiveSlots: const <int>[2],
        objectiveOrder: const <int>[2],
        nextSlotOrdinal: 3,
        transitions: plan.transitions,
      ),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('slot 1 must remain active'),
        ),
      ),
    );
  });

  test('rejects conditions and effects targeting inactive objectives', () {
    expect(
      () => AuthoringRevision3QuestTransitionPlanV1.fromJson({
        'objective_slots': [1],
        'objective_order': [1],
        'next_slot_ordinal': 2,
        'transitions': [
          {
            'node': {'kind': 'root'},
            'edge': 'start',
            'external_allowed': false,
            'effects': [
              {
                'target': {'kind': 'objective', 'slot': 2},
                'effect': 'start',
              },
            ],
          },
        ],
      }),
      throwsFormatException,
    );
  });

  test('rejects noncanonical false succeeds_parent on input', () {
    expect(
      () => AuthoringRevision3QuestTransitionV1.fromJson({
        'node': {'kind': 'root'},
        'edge': 'success',
        'external_allowed': true,
        'succeeds_parent': false,
      }, context: 'test transition'),
      throwsFormatException,
    );
  });

  test('treats succeeds_parent as an implicit root success action', () {
    final plan = AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
      1,
    );
    final objectiveSuccess = plan.transitions.last;
    for (final terminal in <AuthoringRevision3QuestTransitionEffectKindV1>[
      AuthoringRevision3QuestTransitionEffectKindV1.succeed,
      AuthoringRevision3QuestTransitionEffectKindV1.fail,
    ]) {
      expect(
        () => AuthoringRevision3QuestTransitionV1(
          node: objectiveSuccess.node,
          edge: objectiveSuccess.edge,
          externalAllowed: true,
          effects: <AuthoringRevision3QuestTransitionEffectV1>[
            AuthoringRevision3QuestTransitionEffectV1(
              target: const AuthoringRevision3QuestTransitionNodeV1.root(),
              effect: terminal,
            ),
          ],
          succeedsParent: true,
        ),
        throwsFormatException,
      );
    }

    final transitions = <AuthoringRevision3QuestTransitionV1>[
      ...plan.transitions.take(2),
      AuthoringRevision3QuestTransitionV1(
        node: const AuthoringRevision3QuestTransitionNodeV1.root(),
        edge: AuthoringRevision3QuestTransitionEdgeV1.success,
        externalAllowed: true,
        effects: <AuthoringRevision3QuestTransitionEffectV1>[
          AuthoringRevision3QuestTransitionEffectV1(
            target: AuthoringRevision3QuestTransitionNodeV1.objective(1),
            effect: AuthoringRevision3QuestTransitionEffectKindV1.succeed,
          ),
        ],
      ),
      ...plan.transitions.skip(2),
    ];
    expect(
      () => AuthoringRevision3QuestTransitionPlanV1(
        objectiveSlots: plan.objectiveSlots,
        objectiveOrder: plan.objectiveOrder,
        nextSlotOrdinal: plan.nextSlotOrdinal,
        transitions: transitions,
      ),
      throwsFormatException,
    );
  });

  test('requires automatic terminal conditions to be provably disjoint', () {
    final plan = AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
      1,
    );
    const root = AuthoringRevision3QuestTransitionNodeV1.root();
    final objective = AuthoringRevision3QuestTransitionNodeV1.objective(1);
    AuthoringRevision3QuestTransitionPredicateV1 predicate(
      List<AuthoringRevision3QuestTransitionConditionAtomV1> atoms,
    ) => AuthoringRevision3QuestTransitionPredicateV1(
      anyOf: <AuthoringRevision3QuestTransitionConditionGroupV1>[
        AuthoringRevision3QuestTransitionConditionGroupV1(allOf: atoms),
      ],
    );

    final success = AuthoringRevision3QuestTransitionV1(
      node: objective,
      edge: AuthoringRevision3QuestTransitionEdgeV1.success,
      externalAllowed: true,
      predicate: predicate(<AuthoringRevision3QuestTransitionConditionAtomV1>[
        const AuthoringRevision3QuestTransitionConditionAtomV1(
          node: root,
          test: AuthoringRevision3QuestTransitionStateTestV1.running,
          negated: false,
        ),
      ]),
      succeedsParent: true,
    );
    final overlappingFailure = AuthoringRevision3QuestTransitionV1(
      node: objective,
      edge: AuthoringRevision3QuestTransitionEdgeV1.failure,
      externalAllowed: false,
      predicate: predicate(<AuthoringRevision3QuestTransitionConditionAtomV1>[
        const AuthoringRevision3QuestTransitionConditionAtomV1(
          node: root,
          test: AuthoringRevision3QuestTransitionStateTestV1.running,
          negated: false,
        ),
        AuthoringRevision3QuestTransitionConditionAtomV1(
          node: objective,
          test: AuthoringRevision3QuestTransitionStateTestV1.available,
          negated: false,
        ),
      ]),
    );
    expect(
      () => AuthoringRevision3QuestTransitionPlanV1(
        objectiveSlots: plan.objectiveSlots,
        objectiveOrder: plan.objectiveOrder,
        nextSlotOrdinal: plan.nextSlotOrdinal,
        transitions: <AuthoringRevision3QuestTransitionV1>[
          ...plan.transitions.take(plan.transitions.length - 1),
          success,
          overlappingFailure,
        ],
      ),
      throwsFormatException,
    );

    final disjointSuccess = AuthoringRevision3QuestTransitionV1(
      node: objective,
      edge: AuthoringRevision3QuestTransitionEdgeV1.success,
      externalAllowed: true,
      predicate: predicate(<AuthoringRevision3QuestTransitionConditionAtomV1>[
        const AuthoringRevision3QuestTransitionConditionAtomV1(
          node: root,
          test: AuthoringRevision3QuestTransitionStateTestV1.succeeded,
          negated: false,
        ),
      ]),
      succeedsParent: true,
    );
    final disjointFailure = AuthoringRevision3QuestTransitionV1(
      node: objective,
      edge: AuthoringRevision3QuestTransitionEdgeV1.failure,
      externalAllowed: false,
      predicate: predicate(<AuthoringRevision3QuestTransitionConditionAtomV1>[
        const AuthoringRevision3QuestTransitionConditionAtomV1(
          node: root,
          test: AuthoringRevision3QuestTransitionStateTestV1.failed,
          negated: false,
        ),
      ]),
    );
    expect(
      AuthoringRevision3QuestTransitionPlanV1(
        objectiveSlots: plan.objectiveSlots,
        objectiveOrder: plan.objectiveOrder,
        nextSlotOrdinal: plan.nextSlotOrdinal,
        transitions: <AuthoringRevision3QuestTransitionV1>[
          ...plan.transitions.take(plan.transitions.length - 1),
          disjointSuccess,
          disjointFailure,
        ],
      ).transitions,
      hasLength(plan.transitions.length + 1),
    );

    final rootSuccess = AuthoringRevision3QuestTransitionV1(
      node: root,
      edge: AuthoringRevision3QuestTransitionEdgeV1.success,
      externalAllowed: false,
      predicate: predicate(<AuthoringRevision3QuestTransitionConditionAtomV1>[
        AuthoringRevision3QuestTransitionConditionAtomV1(
          node: objective,
          test: AuthoringRevision3QuestTransitionStateTestV1.running,
          negated: false,
        ),
      ]),
    );
    final rootFailure = AuthoringRevision3QuestTransitionV1(
      node: root,
      edge: AuthoringRevision3QuestTransitionEdgeV1.failure,
      externalAllowed: false,
      predicate: predicate(<AuthoringRevision3QuestTransitionConditionAtomV1>[
        AuthoringRevision3QuestTransitionConditionAtomV1(
          node: objective,
          test: AuthoringRevision3QuestTransitionStateTestV1.running,
          negated: false,
        ),
      ]),
    );
    expect(
      () => AuthoringRevision3QuestTransitionPlanV1(
        objectiveSlots: plan.objectiveSlots,
        objectiveOrder: plan.objectiveOrder,
        nextSlotOrdinal: plan.nextSlotOrdinal,
        transitions: <AuthoringRevision3QuestTransitionV1>[
          ...plan.transitions.take(2),
          rootSuccess,
          rootFailure,
          ...plan.transitions.skip(2),
        ],
      ),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('provably disjoint'),
        ),
      ),
    );
  });

  test('reads retained v4 plan and rejects an exact plan no-op', () {
    final fixture = Revision3QuestOutlineFixture();
    final plan = AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
      3,
    );
    final projectJson = _projectJsonWithPlan(fixture, plan);
    final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
      currentProjectJson: projectJson,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      expectedModuleId: revision3QuestOutlineModuleId,
      expectedModuleRevision: fixture.moduleRevision,
    );

    expect(seed.transitionPlan.canonicalJson, plan.canonicalJson);
    expect(
      () => AuthoringRevision3QuestTransitionsEditRequestV1.forProject(
        expectedHead: fixture.head,
        currentProjectJson: projectJson,
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        transitionPlan: plan,
      ),
      throwsFormatException,
    );
  });

  test('Studio handshake requires the sorted Quest transitions command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_quest_transitions_edit_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('FFI sends the minimal wire and accepts an exact v4 delta', () async {
    final fixture = Revision3QuestOutlineFixture();
    final defaultPlan =
        AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(3);
    final plan = AuthoringRevision3QuestTransitionPlanV1(
      objectiveSlots: defaultPlan.objectiveSlots,
      objectiveOrder: const <int>[3, 1, 2],
      nextSlotOrdinal: defaultPlan.nextSlotOrdinal,
      transitions: defaultPlan.transitions,
    );
    final request = AuthoringRevision3QuestTransitionsEditRequestV1.forProject(
      expectedHead: fixture.head,
      currentProjectJson: fixture.projectJson,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      transitionPlan: plan,
    );
    final candidate = _projectJsonWithPlan(fixture, plan, advance: true);
    final response = <String, Object?>{
      'ok': true,
      'outcome': 'prepared_unpublished',
      'basis_head_json': fixture.head.canonicalJson,
      'head_json': manifestHead(5000, 'c').canonicalJson,
      'project_json': candidate,
      'project_id': revision3QuestOutlineProjectId,
      'revision': fixture.projectRevision + 1,
      'quest_id': revision3QuestOutlineQuestId,
      'module_id': revision3QuestOutlineModuleId,
      'quest_revision': fixture.questRevision + 1,
      'module_revision': fixture.moduleRevision + 1,
      'transition_plan_seal': <String, Object?>{
        'byte_len': plan.contentSeal.byteLength,
        'sha256': plan.contentSeal.sha256,
      },
      'build_status': 'blocked',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_supported',
    };
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_store_prepare_revision3_quest_transitions_edit_v1': response,
      },
    );

    final prepared = await ModFfi(core)
        .authoringStorePrepareRevision3QuestTransitionsEditV1(
          root: _root,
          currentProjectJson: fixture.projectJson,
          request: request,
        );

    expect(prepared.transitionPlanSeal.sha256, plan.contentSeal.sha256);
    expect(
      prepared.buildStatus,
      AuthoringRevision3QuestTransitionsBuildStatus.blocked,
    );
    expect(
      prepared.runtimeStatus,
      AuthoringRevision3QuestTransitionsRuntimeStatus.runtimeUnqualified,
    );
    expect(
      prepared.publicationStatus,
      AuthoringRevision3QuestTransitionsPublicationStatus.notSupported,
    );
    expect(core.calls.single.payload.keys, <String>[
      'current_project_json',
      'quest_transitions_request_json',
      'root',
    ]);
  });
}

String _projectJsonWithPlan(
  Revision3QuestOutlineFixture fixture,
  AuthoringRevision3QuestTransitionPlanV1 plan, {
  bool advance = false,
}) {
  final project = fixture.projectObject();
  if (advance) project['revision'] = fixture.projectRevision + 1;
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final quest = (entities[revision3QuestOutlineQuestId]! as Map)
      .cast<String, Object?>();
  final questPayload = (quest['payload']! as Map).cast<String, Object?>();
  final questData = (questPayload['data']! as Map).cast<String, Object?>();
  final input = (questData['input']! as Map).cast<String, Object?>();
  if (advance) quest['revision'] = fixture.questRevision + 1;
  questData['generator_version'] = 4;
  input['transition_plan'] = plan.toJson();

  final module = (entities[revision3QuestOutlineModuleId]! as Map)
      .cast<String, Object?>();
  final moduleOrigin = (module['origin']! as Map).cast<String, Object?>();
  final modulePayload = (module['payload']! as Map).cast<String, Object?>();
  final moduleData = (modulePayload['data']! as Map).cast<String, Object?>();
  if (advance) module['revision'] = fixture.moduleRevision + 1;
  moduleOrigin['generator_version'] = 4;
  moduleData['generator_version'] = 4;
  moduleData['input_fingerprint'] = revision3QuestInputFingerprint(input);
  return jsonEncode(project);
}
