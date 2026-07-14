import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/domain/quest_journal.dart';

void main() {
  const oldRoot = 'Quest_OldCamp';
  const oldChapter = 'Quest_OldCamp_OCCHAPTER1';
  const oldMain = 'Quest_OldCamp_OCCHAPTER1_BRINGLIST';
  const oldObjective =
      'Quest_OldCamp_OCCHAPTER1_BRINGLIST_BRINGLIST_OBJ_GETLIST';
  const oldInternal = 'Quest_OldCamp_OCCHAPTER1_BRINGLIST_BRINGLIST_MAP';
  const oldNamedBelowInternal =
      'Quest_OldCamp_OCCHAPTER1_BRINGLIST_BRINGLIST_MAP_MARKER';

  final labels = <String, String>{
    oldRoot: 'Old Camp structure',
    oldChapter: 'Chapter 1 structure',
    oldMain: 'Test of Faith',
    oldObjective: 'Collect the list',
    oldNamedBelowInternal: 'Map marker',
    'Quest_JESSE_PAYFORME': 'Pay for me',
    'Quest_NewCamp_NCCHAPTER1_DAMLURKER': 'The dam lurker',
    'Quest_SwampCamp_SCCHAPTER1_HARVEST': 'The weed harvest',
    'Quest_ValleyOfMines_FINDNEK': 'Find Nek',
    'Quest_OldMine_SUCCESS': 'Successful quest',
    'Quest_OldMine_FAILED': 'Failed quest',
    'Quest_OldMine_AVAILABLE': 'Future quest',
    'Quest_OldMine_NONE': 'Dormant quest',
    'Quest_Tutorials_Tut_Map': 'Map',
  };
  final descriptions = <String, String>{
    oldMain: 'Bring Diego the list.',
    'Quest_JESSE_PAYFORME': 'Help Jesse.',
    'Quest_NewCamp_NCCHAPTER1_DAMLURKER': 'Kill the lurker.',
    'Quest_SwampCamp_SCCHAPTER1_HARVEST': 'Harvest swampweed.',
    'Quest_ValleyOfMines_FINDNEK': 'Find the missing guard.',
    'Quest_OldMine_SUCCESS': 'Already done.',
    'Quest_OldMine_FAILED': 'Could not be done.',
    'Quest_OldMine_AVAILABLE': 'Not started.',
    'Quest_OldMine_NONE': 'Inactive.',
  };

  QuestJournal build(Iterable<ProgressionQuest> quests) => buildQuestJournal(
    quests,
    localizedLabel: (quest) => labels[quest.id],
    localizedDescription: (quest) => descriptions[quest.id],
    isJournalQuest: (quest) => descriptions.containsKey(quest.id),
  );

  test('skips root/chapter scaffolding and preserves named hierarchy', () {
    final journal = build([
      quest(oldRoot, 'OldCamp', 'Running'),
      quest(oldChapter, 'OldCamp', 'Running'),
      quest(oldMain, 'OldCamp', 'Running'),
      quest(oldObjective, 'OldCamp', 'Succeeded'),
      quest(oldInternal, 'OldCamp', 'Running'),
      quest(oldNamedBelowInternal, 'OldCamp', 'Running'),
    ]);

    final main = journal.roots.oldCamp.single;
    expect(main.quest.id, oldMain);
    expect(main.label, 'Test of Faith');
    expect(main.description, 'Bring Diego the list.');
    expect(main.children.map((node) => node.quest.id), [
      oldObjective,
      oldNamedBelowInternal,
    ]);
    expect(main.technicalDescendants.map((row) => row.id), [oldInternal]);
    expect(main.relatedQuests.map((row) => row.id), [
      oldMain,
      oldInternal,
      oldObjective,
      oldNamedBelowInternal,
    ]);
    expect(journal.flattenDepthFirst.map((node) => node.label), [
      'Test of Faith',
      'Collect the list',
      'Map marker',
    ]);
  });

  test('standalone named quest becomes a Colony root', () {
    final journal = build([
      quest('Quest_JESSE_PAYFORME', 'JESSE', 'EQuestState::Running'),
    ]);

    expect(journal.roots.colony.single.label, 'Pay for me');
    expect(journal.roots.countFor(QuestJournalSection.colony), 1);
  });

  test('Tutorials are returned separately and never enter journal roots', () {
    final root = quest('Quest_Tutorials', 'Tutorials', 'Running');
    final page = quest('Quest_Tutorials_Tut_Map', 'Tutorials', 'Succeeded');
    final journal = build([root, page]);

    expect(journal.roots.all, isEmpty);
    expect(journal.tutorials.map((row) => row.id), [root.id, page.id]);
  });

  test('classifies visible roots by state/group and exposes root counts', () {
    final journal = build([
      quest(oldMain, 'OldCamp', 'EQuestState::Running'),
      quest('Quest_NewCamp_NCCHAPTER1_DAMLURKER', 'NewCamp', 'Running'),
      quest('Quest_SwampCamp_SCCHAPTER1_HARVEST', 'SwampCamp', 'Running'),
      quest('Quest_ValleyOfMines_FINDNEK', 'ValleyOfMines', 'Running'),
      quest('Quest_OldMine_SUCCESS', 'OldMine', 'Succeeded'),
      quest('Quest_OldMine_FAILED', 'OldMine', 'EQuestState::Failed'),
      quest('Quest_OldMine_AVAILABLE', 'OldMine', 'Available'),
      quest('Quest_OldMine_NONE', 'OldMine', 'EQuestState::None'),
    ]);

    expect(journal.roots.oldCamp, hasLength(1));
    expect(journal.roots.newCamp, hasLength(1));
    expect(journal.roots.swampCamp, hasLength(1));
    expect(journal.roots.colony, hasLength(1));
    expect(journal.roots.completed.map((node) => node.label), [
      'Successful quest',
      'Failed quest',
    ]);
    expect(journal.roots.counts, {
      QuestJournalSection.oldCamp: 1,
      QuestJournalSection.newCamp: 1,
      QuestJournalSection.swampCamp: 1,
      QuestJournalSection.colony: 1,
      QuestJournalSection.completed: 2,
    });
    expect(
      journal.flattenDepthFirst.map((node) => node.label),
      isNot(contains(anyOf('Future quest', 'Dormant quest'))),
    );
  });

  test('search keeps a root when a named or technical descendant matches', () {
    final journal = build([
      quest(oldRoot, 'OldCamp', 'Running'),
      quest(oldChapter, 'OldCamp', 'Running'),
      quest(oldMain, 'OldCamp', 'Running'),
      quest(oldObjective, 'OldCamp', 'Succeeded'),
      quest(oldInternal, 'OldCamp', 'Running', name: 'Secret map node'),
    ]);

    expect(journal.search('collect the list').single.quest.id, oldMain);
    expect(journal.search('secret map node').single.quest.id, oldMain);
    expect(journal.search('does not exist'), isEmpty);
  });

  test('optional raw fallback keeps smoke-test data usable without loc', () {
    final journal = buildQuestJournal(
      [
        quest('Quest_OldCamp', 'OldCamp', 'Running'),
        quest('Quest_OldCamp_OCCHAPTER1', 'OldCamp', 'Running'),
        quest(
          'Quest_OldCamp_OCCHAPTER1_MAIN',
          'OldCamp',
          'Running',
          name: 'MAIN',
        ),
        quest(
          'Quest_OldCamp_OCCHAPTER1_MAIN_MAIN_OBJ_GO',
          'OldCamp',
          'Running',
          name: 'MAIN_OBJ_GO',
        ),
        quest(
          'Quest_OldCamp_OCCHAPTER1_MAIN_MAIN_MAP',
          'OldCamp',
          'Running',
          name: 'MAIN_MAP',
        ),
      ],
      localizedLabel: (_) => null,
      rawFallbackLabel: (quest) => 'Readable ${quest.name}',
      allowRawFallback: true,
    );

    expect(journal.roots.oldCamp.single.label, 'Readable MAIN');
    expect(journal.roots.oldCamp.single.children, isEmpty);
  });
}

ProgressionQuest quest(String id, String group, String state, {String? name}) =>
    ProgressionQuest(
      questClass: '/Script/Angelscript.$id',
      id: id,
      group: group,
      name: name ?? id.substring('Quest_'.length),
      currentState: state,
      statePath: ['QuestDataByClass', '{$id}', 'CurrentState'],
      writable: true,
    );
