import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

void main() {
  test('loads one exact visible Quest checkpoint', () async {
    final fixture = Revision3QuestOutlineFixture();
    final index = fixture.contentIndex();
    String? loadedQuest;
    int? loadedQuestRevision;
    String? loadedModule;
    int? loadedModuleRevision;
    final service = Revision3QuestTransitionsAuthoringService(
      loadSeed:
          ({
            required questId,
            required expectedQuestRevision,
            required expectedModuleId,
            required expectedModuleRevision,
          }) async {
            loadedQuest = questId;
            loadedQuestRevision = expectedQuestRevision;
            loadedModule = expectedModuleId;
            loadedModuleRevision = expectedModuleRevision;
            return _seed(fixture);
          },
      publishTechnicalPlan: ({required plan}) async =>
          throw UnimplementedError(),
    );

    final checkpoint = await service.load(
      index: index,
      quest: index.entityById(revision3QuestOutlineQuestId)!,
    );

    expect(loadedQuest, revision3QuestOutlineQuestId);
    expect(loadedQuestRevision, fixture.questRevision);
    expect(loadedModule, revision3QuestOutlineModuleId);
    expect(loadedModuleRevision, fixture.moduleRevision);
    expect(
      checkpoint.objectives.map((item) => item.title),
      fixture.objectiveTitles,
    );
    expect(checkpoint.objectives.map((item) => item.slot), [1, 2, 3]);
    expect(
      checkpoint.transitionPlan.canonicalJson,
      _seed(fixture).transitionPlan.canonicalJson,
    );
  });

  test(
    'sequential template keeps slots stable across presentation reorder',
    () {
      final basis = AuthoringRevision3QuestTransitionPlanV1.legacySeed(3);
      final reordered =
          Revision3QuestTransitionsAuthoringService.reorderObjectives(
            basis,
            const <int>[3, 1, 2],
          );
      final template =
          Revision3QuestTransitionsAuthoringService.sequentialTemplate(
            reordered,
          );

      expect(template.objectiveSlots, [1, 2, 3]);
      expect(template.objectiveOrder, [3, 1, 2]);
      expect(template.nextSlotOrdinal, 4);
      final rootStart = template.transitions.singleWhere(
        (transition) =>
            transition.node.kind ==
                AuthoringRevision3QuestTransitionNodeKind.root &&
            transition.edge == AuthoringRevision3QuestTransitionEdgeV1.start,
      );
      expect(rootStart.effects.single.target.slot, 3);
      final thirdSuccess = template.transitions.singleWhere(
        (transition) =>
            transition.node.slot == 3 &&
            transition.edge == AuthoringRevision3QuestTransitionEdgeV1.success,
      );
      expect(thirdSuccess.effects.single.target.slot, 1);
      final secondSuccess = template.transitions.singleWhere(
        (transition) =>
            transition.node.slot == 2 &&
            transition.edge == AuthoringRevision3QuestTransitionEdgeV1.success,
      );
      expect(secondSuccess.succeedsParent, isTrue);
      expect(secondSuccess.effects, isEmpty);
    },
  );

  test('canonical condition helpers sort and reject invalid expressions', () {
    final group = Revision3QuestTransitionsAuthoringService.conditionGroup([
      AuthoringRevision3QuestTransitionConditionAtomV1(
        node: AuthoringRevision3QuestTransitionNodeV1.objective(2),
        test: AuthoringRevision3QuestTransitionStateTestV1.completed,
        negated: false,
      ),
      const AuthoringRevision3QuestTransitionConditionAtomV1(
        node: AuthoringRevision3QuestTransitionNodeV1.root(),
        test: AuthoringRevision3QuestTransitionStateTestV1.started,
        negated: false,
      ),
    ]);
    expect(
      group.allOf.first.node.kind,
      AuthoringRevision3QuestTransitionNodeKind.root,
    );

    expect(
      () => Revision3QuestTransitionsAuthoringService.conditionGroup(const [
        AuthoringRevision3QuestTransitionConditionAtomV1(
          node: AuthoringRevision3QuestTransitionNodeV1.root(),
          test: AuthoringRevision3QuestTransitionStateTestV1.started,
          negated: false,
        ),
        AuthoringRevision3QuestTransitionConditionAtomV1(
          node: AuthoringRevision3QuestTransitionNodeV1.root(),
          test: AuthoringRevision3QuestTransitionStateTestV1.started,
          negated: true,
        ),
      ]),
      throwsFormatException,
    );
    expect(
      () => Revision3QuestTransitionsAuthoringService.predicate(
        List.generate(
          9,
          (_) => const [
            AuthoringRevision3QuestTransitionConditionAtomV1(
              node: AuthoringRevision3QuestTransitionNodeV1.root(),
              test: AuthoringRevision3QuestTransitionStateTestV1.started,
              negated: false,
            ),
          ],
        ),
      ),
      throwsFormatException,
    );
  });

  test(
    'publishes one exact technical plan and verifies callback result',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      Revision3QuestTransitionsEditTechnicalPlan? received;
      final service = _service(
        fixture,
        publish: (plan) {
          received = plan;
          return _publication(fixture, plan.transitionPlan.contentSeal);
        },
      );
      final index = fixture.contentIndex();
      final checkpoint = await service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
      );
      final plan = Revision3QuestTransitionsAuthoringService.sequentialTemplate(
        checkpoint.transitionPlan,
      );

      final publication = await service.publish(
        checkpoint: checkpoint,
        transitionPlan: plan,
      );

      expect(received?.questId, revision3QuestOutlineQuestId);
      expect(received?.expectedQuestRevision, fixture.questRevision);
      expect(received?.moduleId, revision3QuestOutlineModuleId);
      expect(received?.expectedModuleRevision, fixture.moduleRevision);
      expect(
        received?.expectedTransitionPlanSeal.sha256,
        checkpoint.seed.transitionPlanSeal.sha256,
      );
      expect(received?.transitionPlan.canonicalJson, plan.canonicalJson);
      expect(publication.projectRevision, fixture.projectRevision + 1);
    },
  );

  test(
    'rejects no-op, changed objective shape, and forged publication',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      var publishes = 0;
      final service = _service(
        fixture,
        publish: (plan) {
          publishes++;
          return Revision3QuestTransitionsEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: fixture.projectRevision + 2,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: fixture.questRevision + 1,
            moduleRevision: fixture.moduleRevision + 1,
            transitionPlanSeal: plan.transitionPlan.contentSeal,
          );
        },
      );
      final index = fixture.contentIndex();
      final checkpoint = await service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
      );

      await expectLater(
        service.publish(
          checkpoint: checkpoint,
          transitionPlan: checkpoint.transitionPlan,
        ),
        throwsFormatException,
      );
      expect(publishes, 0);
      expect(
        () => Revision3QuestTransitionsAuthoringService.validateEditablePlan(
          checkpoint,
          AuthoringRevision3QuestTransitionPlanV1.legacySeed(2),
        ),
        throwsFormatException,
      );
      await expectLater(
        service.publish(
          checkpoint: checkpoint,
          transitionPlan:
              Revision3QuestTransitionsAuthoringService.sequentialTemplate(
                checkpoint.transitionPlan,
              ),
        ),
        throwsA(isA<Revision3QuestTransitionsRequiresReopenException>()),
      );
    },
  );
}

AuthoringRevision3QuestTransitionsSeed _seed(
  Revision3QuestOutlineFixture fixture,
) => AuthoringRevision3QuestTransitionsSeed.forProject(
  currentProjectJson: fixture.projectJson,
  questId: revision3QuestOutlineQuestId,
  expectedQuestRevision: fixture.questRevision,
  expectedModuleId: revision3QuestOutlineModuleId,
  expectedModuleRevision: fixture.moduleRevision,
);

Revision3QuestTransitionsAuthoringService _service(
  Revision3QuestOutlineFixture fixture, {
  required Revision3QuestTransitionsEditPublication Function(
    Revision3QuestTransitionsEditTechnicalPlan plan,
  )
  publish,
}) => Revision3QuestTransitionsAuthoringService(
  loadSeed:
      ({
        required questId,
        required expectedQuestRevision,
        required expectedModuleId,
        required expectedModuleRevision,
      }) async => _seed(fixture),
  publishTechnicalPlan: ({required plan}) async => publish(plan),
);

Revision3QuestTransitionsEditPublication _publication(
  Revision3QuestOutlineFixture fixture,
  AuthoringDraftContentSeal seal,
) => Revision3QuestTransitionsEditPublication(
  projectId: revision3QuestOutlineProjectId,
  projectRevision: fixture.projectRevision + 1,
  questId: revision3QuestOutlineQuestId,
  moduleId: revision3QuestOutlineModuleId,
  questRevision: fixture.questRevision + 1,
  moduleRevision: fixture.moduleRevision + 1,
  transitionPlanSeal: seal,
);
