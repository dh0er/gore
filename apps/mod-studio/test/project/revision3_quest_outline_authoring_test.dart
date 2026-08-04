import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

void main() {
  test(
    'content projection retains immutable structured Quest outline facts',
    () {
      final index = Revision3QuestOutlineFixture().contentIndex();
      final quest = index.entityById(revision3QuestOutlineQuestId)!;
      final summary = quest.summary.questDraft!;

      expect(summary.technicalId, 'GORE_FIND_HOMER');
      expect(summary.title, 'Find Homer');
      expect(summary.objectiveTitles, <String>[
        'Ask Asghan about Homer',
        'Inspect the old gate',
        'Report the secured gate',
      ]);
      expect(summary.moduleNamespace, 'PROJECT.QUESTS.FINDHOMER');
      expect(summary.parentRuntimeClass, 'UQuest_SwampCamp_SCChapter2');
      expect(summary.giverRuntimeUniqueName, 'OM_GRD_Asghan_263');
      expect(
        () => summary.objectiveTitles.add('Injected'),
        throwsUnsupportedError,
      );
    },
  );

  test(
    'friendly validation rejects padding, unsafe text and count changes',
    () {
      expect(
        Revision3QuestOutlineEditInput.validateFields(
          displayName: ' Padded ',
          title: 'Safe title',
          objectiveTitles: const ['Safe objective'],
        ),
        isNotNull,
      );
      expect(
        Revision3QuestOutlineEditInput.validateFields(
          displayName: 'Safe name',
          title: 'Quoted "title"',
          objectiveTitles: const ['Safe objective'],
        ),
        contains('without quotes'),
      );
      expect(
        Revision3QuestOutlineEditInput.validateFields(
          displayName: 'Safe name',
          title: 'Safe title',
          objectiveTitles: const [],
        ),
        contains('between 1 and 8'),
      );
    },
  );

  test('edit binds exact entities, stable objective slots and plan seal', () {
    final fixture = Revision3QuestOutlineFixture();
    final index = fixture.contentIndex();
    final quest = index.entityById(revision3QuestOutlineQuestId)!;
    final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
      currentProjectJson: fixture.projectJson,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      expectedModuleId: revision3QuestOutlineModuleId,
      expectedModuleRevision: fixture.moduleRevision,
    );

    final input = Revision3QuestOutlineEditInput.forQuest(
      index: index,
      quest: quest,
      seed: seed,
      displayName: 'Find Homer safely',
      title: 'Secure the old gate',
      objectives: const [
        Revision3QuestOutlineObjectiveEdit(
          slot: 3,
          title: 'Report the secured gate',
        ),
        Revision3QuestOutlineObjectiveEdit(
          slot: 1,
          title: 'Ask Asghan about Homer',
        ),
        Revision3QuestOutlineObjectiveEdit(
          slot: 2,
          title: 'Inspect the old gate',
        ),
      ],
    );

    expect(input.questId, revision3QuestOutlineQuestId);
    expect(input.expectedQuestRevision, 4);
    expect(input.moduleId, revision3QuestOutlineModuleId);
    expect(input.expectedModuleRevision, 5);
    expect(input.objectiveSlots, [3, 1, 2]);
    expect(
      input.expectedTransitionPlanSeal.sha256,
      seed.transitionPlanSeal.sha256,
    );
    expect(() => input.objectiveSlots.add(4), throwsUnsupportedError);
    expect(() => input.objectiveTitles.clear(), throwsUnsupportedError);
    expect(
      () => Revision3QuestOutlineEditInput.forQuest(
        index: index,
        quest: quest,
        seed: seed,
        displayName: 'Find Homer safely',
        title: 'Secure the old gate',
        objectives: const [
          Revision3QuestOutlineObjectiveEdit(
            slot: 1,
            title: 'Ask Asghan about Homer',
          ),
          Revision3QuestOutlineObjectiveEdit(
            slot: 1,
            title: 'Inspect the old gate',
          ),
          Revision3QuestOutlineObjectiveEdit(
            slot: 3,
            title: 'Report the secured gate',
          ),
        ],
      ),
      throwsFormatException,
    );
  });

  test('single-objective current project omits optional extra titles', () {
    final fixture = Revision3QuestOutlineFixture(
      objectiveTitles: const <String>['Ask Asghan about Homer'],
    );

    final seed = AuthoringRevision3QuestTransitionsSeed.forProject(
      currentProjectJson: fixture.projectJson,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      expectedModuleId: revision3QuestOutlineModuleId,
      expectedModuleRevision: fixture.moduleRevision,
    );

    expect(seed.objectives.single.title, 'Ask Asghan about Homer');
  });
}
