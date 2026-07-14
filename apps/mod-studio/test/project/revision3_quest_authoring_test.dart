import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';

const _projectId = '11111111111111111111111111111111';

void main() {
  test(
    'derives deterministic technical identities from project revision and intent',
    () {
      final input = Revision3QuestDraftAuthoringInput(
        parentCatalogId: 'chapter-one',
        giverCatalogId: 'asghan',
        title: '  Find Homer!  ',
        description: 'Ask around the old camp.',
        objectiveTitle: 'Speak to Asghan',
      );

      final first = Revision3QuestDraftTechnicalPlan.forCheckpoint(
        projectId: _projectId,
        projectRevision: 7,
        input: input,
      );
      final repeated = Revision3QuestDraftTechnicalPlan.forCheckpoint(
        projectId: _projectId,
        projectRevision: 7,
        input: input,
      );
      final nextRevision = Revision3QuestDraftTechnicalPlan.forCheckpoint(
        projectId: _projectId,
        projectRevision: 8,
        input: input,
      );

      expect(input.title, 'Find Homer!');
      expect(first.questId, repeated.questId);
      expect(first.scriptModuleId, repeated.scriptModuleId);
      expect(first.intent.moduleNamespace, repeated.intent.moduleNamespace);
      expect(first.questId, isNot(first.scriptModuleId));
      expect(first.questId, isNot(nextRevision.questId));
      expect(first.scriptModuleId, isNot(nextRevision.scriptModuleId));
      expect(
        first.intent.technicalId,
        matches(r'^GORE_FIND_HOMER_[0-9A-F]{10}$'),
      );
      expect(
        first.intent.moduleNamespace,
        matches(r'^GoreMods\.Quests\.FindHomer[0-9A-F]{10}$'),
      );
      expect(first.intent.parentCatalogId, 'chapter-one');
      expect(first.intent.giverCatalogId, 'asghan');
      expect(first.displayName, 'Find Homer!');
    },
  );

  test('rejects text the bounded Quest generators cannot represent', () {
    expect(
      () => Revision3QuestDraftAuthoringInput(
        parentCatalogId: 'chapter-one',
        giverCatalogId: 'asghan',
        title: 'Finde Hömer',
        description: 'Description',
        objectiveTitle: 'Objective',
      ),
      throwsFormatException,
    );
    expect(
      () => Revision3QuestDraftAuthoringInput(
        parentCatalogId: 'chapter-one',
        giverCatalogId: 'asghan',
        title: 'Quoted "Quest"',
        description: 'Description',
        objectiveTitle: 'Objective',
      ),
      throwsFormatException,
    );
    expect(
      () => Revision3QuestDraftTechnicalPlan.forCheckpoint(
        projectId: '00000000000000000000000000000000',
        projectRevision: 0,
        input: _input(),
      ),
      throwsFormatException,
    );
  });

  test(
    'keeps ordered objectives bounded, unique, immutable, and identity-bound',
    () {
      final input = Revision3QuestDraftAuthoringInput(
        parentCatalogId: 'chapter-one',
        giverCatalogId: 'asghan',
        title: 'Find Homer',
        description: 'Ask around the old camp.',
        objectiveTitle: 'Speak to Asghan',
        additionalObjectiveTitles: const <String>[
          'Inspect the old gate',
          'Report the secured gate',
        ],
      );
      expect(input.objectiveTitles, <String>[
        'Speak to Asghan',
        'Inspect the old gate',
        'Report the secured gate',
      ]);
      expect(input.additionalObjectiveTitles, <String>[
        'Inspect the old gate',
        'Report the secured gate',
      ]);
      expect(
        () => input.additionalObjectiveTitles.add('Mutate'),
        throwsUnsupportedError,
      );
      final multi = Revision3QuestDraftTechnicalPlan.forCheckpoint(
        projectId: _projectId,
        projectRevision: 7,
        input: input,
      );
      final single = Revision3QuestDraftTechnicalPlan.forCheckpoint(
        projectId: _projectId,
        projectRevision: 7,
        input: _input(),
      );
      expect(multi.questId, isNot(single.questId));
      expect(
        multi.intent.additionalObjectiveTitles,
        input.additionalObjectiveTitles,
      );

      expect(
        () => Revision3QuestDraftAuthoringInput(
          parentCatalogId: 'chapter-one',
          giverCatalogId: 'asghan',
          title: 'Duplicate',
          description: 'Description',
          objectiveTitle: 'Speak to Asghan',
          additionalObjectiveTitles: const <String>['speak to asghan'],
        ),
        throwsFormatException,
      );
      expect(
        () => Revision3QuestDraftAuthoringInput(
          parentCatalogId: 'chapter-one',
          giverCatalogId: 'asghan',
          title: 'Too many',
          description: 'Description',
          objectiveTitle: 'First',
          additionalObjectiveTitles: List<String>.generate(
            8,
            (index) => 'Objective ${index + 2}',
          ),
        ),
        throwsFormatException,
      );
    },
  );

  test('catalog projection requires nonempty unique picker identities', () {
    final parent = Revision3QuestCatalogChoice(
      catalogId: 'parent-one',
      displayName: 'Chapter One',
    );
    final giver = Revision3QuestCatalogChoice(
      catalogId: 'giver-one',
      displayName: 'Asghan',
    );
    final catalog = Revision3QuestCatalog(parents: [parent], givers: [giver]);

    expect(catalog.containsParent('parent-one'), isTrue);
    expect(catalog.containsGiver('giver-one'), isTrue);
    expect(
      () => Revision3QuestCatalog(parents: const [], givers: [giver]),
      throwsFormatException,
    );
    expect(
      () => Revision3QuestCatalog(parents: [parent, parent], givers: [giver]),
      throwsFormatException,
    );
  });
}

Revision3QuestDraftAuthoringInput _input() => Revision3QuestDraftAuthoringInput(
  parentCatalogId: 'chapter-one',
  giverCatalogId: 'asghan',
  title: 'Find Homer',
  description: 'Ask around the old camp.',
  objectiveTitle: 'Speak to Asghan',
);
