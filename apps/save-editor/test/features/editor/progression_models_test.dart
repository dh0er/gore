import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';

void main() {
  test('ProgressionOverview parses inspect json', () {
    final overview = ProgressionOverview.fromJson({
      'status': 'ok',
      'questTotal': 707,
      'questStates': {'Available': 700, 'Running': 5, 'Succeeded': 2},
      'knowledgeCharacters': 12,
      'knowledgeEntries': 340,
      'memoryCharacters': 3,
      'memoryEvents': 1500,
      'writable': ['private.typed.setValue', 'private.typed.setAdd'],
    });
    expect(overview.status, 'ok');
    expect(overview.available, isTrue);
    expect(overview.questTotal, 707);
    expect(overview.questStates['Running'], 5);
    expect(overview.knowledgeCharacters, 12);
    expect(overview.memoryEvents, 1500);
    expect(overview.writable, contains('private.typed.setAdd'));

    final unavailable = ProgressionOverview.fromJson({'status': 'unavailable'});
    expect(unavailable.available, isFalse);
  });

  test('ProgressionQuestPage parses query json', () {
    final page = ProgressionQuestPage.fromJson({
      'total': 2,
      'offset': 0,
      'limit': 100,
      'stateCounts': {'Available': 1, 'Running': 1},
      'groupCounts': {'BanditsCamp': 1, 'OldCamp': 1},
      'quests': [
        {
          'questClass': '/Script/Angelscript.Quest_OldCamp_SLEEPER',
          'id': 'Quest_OldCamp_SLEEPER',
          'group': 'OldCamp',
          'name': 'SLEEPER',
          'currentState': 'EQuestState::Running',
          'statePath': [
            'QuestDataByClass',
            '{/Script/Angelscript.Quest_OldCamp_SLEEPER}',
            'CurrentState',
          ],
          'writable': true,
        },
      ],
    });
    expect(page.total, 2);
    expect(page.quests.single.group, 'OldCamp');
    expect(page.quests.single.currentState, 'EQuestState::Running');
    expect(page.quests.single.writable, isTrue);
    expect(page.quests.single.statePath, hasLength(3));
    // groupCounts parsing.
    expect(page.groupCounts['BanditsCamp'], 1);
    expect(page.groupCounts['OldCamp'], 1);
    // Defaults to empty when absent.
    final noGroups = ProgressionQuestPage.fromJson({
      'total': 0,
      'offset': 0,
      'limit': 100,
      'stateCounts': <String, Object?>{},
      'quests': <Object?>[],
    });
    expect(noGroups.groupCounts, isEmpty);
  });

  test('tutorial gate page reuses the quest page shape without a root row', () {
    final page = ProgressionQuestPage.fromJson({
      'section': 'tutorials',
      'total': 2,
      'offset': 0,
      'limit': 100,
      'quests': [
        {
          'questClass': '/Script/Angelscript.Quest_Tutorials_Tut_CombatBasics',
          'id': 'Quest_Tutorials_Tut_CombatBasics',
          'group': 'Tutorials',
          'name': 'Tut_CombatBasics',
          'currentState': 'EQuestState::Running',
          'statePath': [
            'QuestDataByClass',
            '{/Script/Angelscript.Quest_Tutorials_Tut_CombatBasics}',
            'CurrentState',
          ],
          'writable': true,
        },
        {
          'questClass': '/Script/Angelscript.Quest_Tutorials_Tut_Map',
          'id': 'Quest_Tutorials_Tut_Map',
          'group': 'Tutorials',
          'name': 'Tut_Map',
          'currentState': 'EQuestState::Available',
          'statePath': [
            'QuestDataByClass',
            '{/Script/Angelscript.Quest_Tutorials_Tut_Map}',
            'CurrentState',
          ],
          'writable': true,
        },
      ],
    });

    expect(page.quests, hasLength(2));
    expect(page.quests.map((quest) => quest.name), [
      'Tut_CombatBasics',
      'Tut_Map',
    ]);
    expect(page.quests.any((quest) => quest.id == 'Quest_Tutorials'), isFalse);
  });

  test('edit intents emit core edit json', () {
    final questEdit = QuestStateChange(
      statePath: const ['QuestDataByClass', '{X}', 'CurrentState'],
      state: 'EQuestState::Succeeded',
    );
    expect(questEdit.toEditJson(), {
      'path': 'private.typed.setValue',
      'value': {
        'path': ['QuestDataByClass', '{X}', 'CurrentState'],
        'value': 'EQuestState::Succeeded',
      },
    });

    final add = KnowledgeEntryEdit.add(
      character: 'Diego',
      entry: 'Voiceline_X',
    );
    expect(add.toEditJson(), {
      'path': 'private.knowledge.setEntry',
      'value': {'character': 'Diego', 'entry': 'Voiceline_X', 'present': true},
    });

    final remove = KnowledgeEntryEdit.remove(
      character: 'Diego',
      entry: 'Voiceline_X',
    );
    expect(remove.toEditJson(), {
      'path': 'private.knowledge.setEntry',
      'value': {'character': 'Diego', 'entry': 'Voiceline_X', 'present': false},
    });

    final removeEvent = MemoryEventEdit.remove(
      arrayPath: const [
        'LongTermMemoryByGlobalId',
        '{Hero}',
        'MemorizedEvents',
      ],
      index: 4,
    );
    expect(removeEvent.toEditJson(), {
      'path': 'private.typed.arrayRemove',
      'value': {
        'path': ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'],
        'index': 4,
      },
    });

    final duplicate = MemoryEventEdit.duplicate(
      arrayPath: const [
        'LongTermMemoryByGlobalId',
        '{Hero}',
        'MemorizedEvents',
      ],
      index: 4,
    );
    expect(duplicate.toEditJson()['path'], 'private.typed.arrayDuplicate');
    expect(
      MemoryEventEdit.fromEditJson(duplicate.toEditJson())?.isRemove,
      isFalse,
    );
  });

  test('knowledge and event pages parse', () {
    final entries = KnowledgeEntriesPage.fromJson({
      'character': 'OC_STT_Diego',
      'total': 2,
      'offset': 0,
      'limit': 200,
      'entries': ['A', 'B'],
      'setPath': [
        'CharacterKnowledgeByUniqueName',
        '{OC_STT_Diego}',
        'Knowledge',
      ],
    });
    expect(entries.entries, ['A', 'B']);
    expect(entries.setPath, hasLength(3));

    final events = MemoryEventsPage.fromJson({
      'character': 'Hero',
      'total': 1,
      'offset': 0,
      'limit': 100,
      'events': [
        {
          'index': 0,
          'tags': ['Memory.Quest.Started'],
          'timeSeconds': 12.5,
          'affected': 'Hero',
        },
      ],
      'arrayPath': ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'],
    });
    expect(events.events.single.tags, ['Memory.Quest.Started']);
    expect(events.events.single.timeSeconds, 12.5);
  });
}
