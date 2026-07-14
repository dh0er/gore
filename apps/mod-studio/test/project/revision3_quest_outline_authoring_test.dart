import 'package:flutter_test/flutter_test.dart';
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
    'edit input binds the exact selected Quest and owned module revisions',
    () {
      final index = Revision3QuestOutlineFixture().contentIndex();
      final quest = index.entityById(revision3QuestOutlineQuestId)!;
      final input = Revision3QuestOutlineEditInput.forQuest(
        index: index,
        quest: quest,
        displayName: 'Find Homer safely',
        title: 'Find Homer safely',
        objectiveTitles: const <String>[
          'Inspect the old gate',
          'Ask Asghan about Homer',
          'Report to Diego',
        ],
      );

      expect(input.questId, revision3QuestOutlineQuestId);
      expect(input.expectedQuestRevision, 4);
      expect(input.moduleId, revision3QuestOutlineModuleId);
      expect(input.expectedModuleRevision, 5);
      expect(() => input.objectiveTitles.clear(), throwsUnsupportedError);
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
}
