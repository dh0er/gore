import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_context_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _currentParentId = 'g1r:quest-parent:swampcamp_scchapter2';
const _currentGiverId = 'g1r:npc:om_grd_asghan_263';

void main() {
  test(
    'load binds the exact stored runtime connections without defaults',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final service = _service(fixture: fixture, catalogs: [_catalog('a')]);

      final checkpoint = await service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        gameRoot: _gameRoot,
      );

      expect(checkpoint.seed.description, contains('missing worker'));
      expect(checkpoint.currentParent.catalogId, _currentParentId);
      expect(checkpoint.currentGiver.catalogId, _currentGiverId);
      expect(
        checkpoint.catalog.catalogSeal?.sha256,
        List.filled(64, 'a').join(),
      );
    },
  );

  test(
    'missing exact current mapping is unavailable, never replaceable',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final missingCurrent = Revision3QuestCatalog(
        parents: [
          Revision3QuestParentChoice(
            catalogId: revision3QuestContextParentCatalogId,
            displayName: 'Chapter Three',
            runtimeClass: revision3QuestContextParentRuntimeClass,
            catalogLayer: 'base-game.quest-parent.v1',
            authoringSelector: 'SwampCamp_SCChapter3',
            sourceSeal: _sourceSeal(11, '1'),
          ),
        ],
        givers: [
          Revision3QuestGiverChoice(
            catalogId: revision3QuestContextGiverCatalogId,
            displayName: 'Viper',
            runtimeUniqueName: revision3QuestContextGiverRuntimeUniqueName,
            catalogLayer: 'base-game.npc.v1',
            authoringSelector: revision3QuestContextGiverRuntimeUniqueName,
            sourceSeal: _sourceSeal(12, '2'),
          ),
        ],
        catalogSeal: _seal('a'),
        generationExecutableSeal: _generationSeal(),
      );

      await expectLater(
        _service(fixture: fixture, catalogs: [missingCurrent]).load(
          index: index,
          quest: index.entityById(revision3QuestOutlineQuestId)!,
          gameRoot: _gameRoot,
        ),
        throwsA(isA<Revision3QuestContextUnavailableException>()),
      );
    },
  );

  test(
    'load rejects a mismatched game generation before showing choices',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final valid = _catalog('a');
      final wrongGeneration = Revision3QuestCatalog(
        parents: valid.parents,
        givers: valid.givers,
        catalogSeal: valid.catalogSeal,
        generationExecutableSeal: _sourceSeal(171698176, '7'),
      );

      await expectLater(
        _service(fixture: fixture, catalogs: [wrongGeneration]).load(
          index: index,
          quest: index.entityById(revision3QuestOutlineQuestId)!,
          gameRoot: _gameRoot,
        ),
        throwsA(isA<Revision3QuestContextUnavailableException>()),
      );
    },
  );

  test('same runtime with wrong stored provenance is unavailable', () async {
    final fixture = Revision3QuestOutlineFixture();
    final index = fixture.contentIndex();
    final valid = _catalog('a');
    final wrongProvenance = Revision3QuestCatalog(
      parents: [
        Revision3QuestParentChoice(
          catalogId: _currentParentId,
          displayName: 'Chapter Two',
          runtimeClass: 'UQuest_SwampCamp_SCChapter2',
          catalogLayer: 'base-game.quest-parent.v1',
          authoringSelector: 'WrongSameRuntimeSelector',
          sourceSeal: _sourceSeal(11, '1'),
        ),
        valid.parents.last,
      ],
      givers: valid.givers,
      catalogSeal: valid.catalogSeal,
      generationExecutableSeal: valid.generationExecutableSeal,
    );

    await expectLater(
      _service(fixture: fixture, catalogs: [wrongProvenance]).load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        gameRoot: _gameRoot,
      ),
      throwsA(isA<Revision3QuestContextUnavailableException>()),
    );
  });

  test(
    'publish reloads sealed choices and sends only the reviewed binding',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final catalog = _catalog('a');
      Revision3QuestContextEditTechnicalPlan? received;
      final service = _service(
        fixture: fixture,
        catalogs: [catalog, _catalog('a')],
        publish: (plan) {
          received = plan;
          return _publication(fixture);
        },
      );
      final checkpoint = await service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        gameRoot: _gameRoot,
      );

      final publication = await service.publish(
        checkpoint: checkpoint,
        gameRoot: _gameRoot,
        description: 'Find Homer and report back safely.',
        parent: checkpoint.catalog.parent(
          revision3QuestContextParentCatalogId,
        )!,
        giver: checkpoint.catalog.giver(revision3QuestContextGiverCatalogId)!,
      );

      expect(publication.projectRevision, fixture.projectRevision + 1);
      expect(received?.questId, revision3QuestOutlineQuestId);
      expect(received?.moduleId, revision3QuestOutlineModuleId);
      expect(received?.parentCatalogId, revision3QuestContextParentCatalogId);
      expect(received?.giverCatalogId, revision3QuestContextGiverCatalogId);
      expect(
        received?.expectedParentRuntimeClass,
        revision3QuestContextParentRuntimeClass,
      );
      expect(
        received?.expectedGiverRuntimeUniqueName,
        revision3QuestContextGiverRuntimeUniqueName,
      );
    },
  );

  test('no-op is rejected before catalog reload or publication', () async {
    final fixture = Revision3QuestOutlineFixture();
    final index = fixture.contentIndex();
    var loads = 0;
    var publishes = 0;
    final service = Revision3QuestContextAuthoringService(
      loadSeed: _seedLoader(fixture),
      loadCatalog: (_) async {
        loads++;
        return _catalog('a');
      },
      publishTechnicalPlan: ({required gameRoot, required plan}) async {
        publishes++;
        return _publication(fixture);
      },
    );
    final checkpoint = await service.load(
      index: index,
      quest: index.entityById(revision3QuestOutlineQuestId)!,
      gameRoot: _gameRoot,
    );

    await expectLater(
      service.publish(
        checkpoint: checkpoint,
        gameRoot: _gameRoot,
        description: checkpoint.seed.description,
        parent: checkpoint.currentParent,
        giver: checkpoint.currentGiver,
      ),
      throwsFormatException,
    );
    expect(loads, 1);
    expect(publishes, 0);
  });

  test(
    'hotfix seal or selected binding drift requires explicit review',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final first = _catalog('a');
      final service = _service(
        fixture: fixture,
        catalogs: [first, _catalog('b')],
      );
      final checkpoint = await service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        gameRoot: _gameRoot,
      );

      await expectLater(
        service.publish(
          checkpoint: checkpoint,
          gameRoot: _gameRoot,
          description: 'Find Homer and report back safely.',
          parent: checkpoint.catalog.parent(
            revision3QuestContextParentCatalogId,
          )!,
          giver: checkpoint.catalog.giver(revision3QuestContextGiverCatalogId)!,
        ),
        throwsA(isA<Revision3QuestContextCatalogDriftException>()),
      );
    },
  );

  test(
    'hotfix losing the current mapping blocks instead of migrating',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final first = _catalog('a');
      final currentLost = Revision3QuestCatalog(
        parents: first.parents.where(
          (choice) => choice.catalogId != _currentParentId,
        ),
        givers: first.givers.where(
          (choice) => choice.catalogId != _currentGiverId,
        ),
        catalogSeal: _seal('b'),
        generationExecutableSeal: _generationSeal(),
      );
      final service = _service(
        fixture: fixture,
        catalogs: [first, currentLost],
      );
      final checkpoint = await service.load(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        gameRoot: _gameRoot,
      );

      await expectLater(
        service.publish(
          checkpoint: checkpoint,
          gameRoot: _gameRoot,
          description: 'Find Homer and report back safely.',
          parent: checkpoint.catalog.parent(
            revision3QuestContextParentCatalogId,
          )!,
          giver: checkpoint.catalog.giver(revision3QuestContextGiverCatalogId)!,
        ),
        throwsA(isA<Revision3QuestContextUnavailableException>()),
      );
    },
  );
}

Revision3QuestContextAuthoringService _service({
  required Revision3QuestOutlineFixture fixture,
  required List<Revision3QuestCatalog> catalogs,
  Revision3QuestContextEditPublication Function(
    Revision3QuestContextEditTechnicalPlan plan,
  )?
  publish,
}) {
  var catalogIndex = 0;
  return Revision3QuestContextAuthoringService(
    loadSeed: _seedLoader(fixture),
    loadCatalog: (_) async => catalogs[catalogIndex++],
    publishTechnicalPlan: ({required gameRoot, required plan}) async =>
        publish?.call(plan) ?? _publication(fixture),
  );
}

Revision3QuestContextSeedLoader _seedLoader(
  Revision3QuestOutlineFixture fixture,
) =>
    ({
      required questId,
      required expectedQuestRevision,
      required expectedModuleId,
      required expectedModuleRevision,
      required expectedParentRuntimeClass,
      required expectedGiverRuntimeUniqueName,
    }) async => AuthoringRevision3QuestContextSeed.forProject(
      currentProjectJson: fixture.projectJson,
      questId: questId,
      expectedQuestRevision: expectedQuestRevision,
      expectedModuleId: expectedModuleId,
      expectedModuleRevision: expectedModuleRevision,
      expectedParentRuntimeClass: expectedParentRuntimeClass,
      expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
    );

Revision3QuestCatalog _catalog(String sealDigit) => Revision3QuestCatalog(
  parents: [
    Revision3QuestParentChoice(
      catalogId: _currentParentId,
      displayName: 'Chapter Two',
      runtimeClass: 'UQuest_SwampCamp_SCChapter2',
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter2',
      sourceSeal: _sourceSeal(11, '1'),
    ),
    Revision3QuestParentChoice(
      catalogId: revision3QuestContextParentCatalogId,
      displayName: 'Chapter Three',
      runtimeClass: revision3QuestContextParentRuntimeClass,
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter3',
      sourceSeal: _sourceSeal(11, '1'),
    ),
  ],
  givers: [
    Revision3QuestGiverChoice(
      catalogId: _currentGiverId,
      displayName: 'Asghan',
      runtimeUniqueName: 'OM_GRD_Asghan_263',
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: 'OM_GRD_Asghan_263',
      sourceSeal: _sourceSeal(12, '2'),
    ),
    Revision3QuestGiverChoice(
      catalogId: revision3QuestContextGiverCatalogId,
      displayName: 'Viper',
      runtimeUniqueName: revision3QuestContextGiverRuntimeUniqueName,
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: revision3QuestContextGiverRuntimeUniqueName,
      sourceSeal: _sourceSeal(12, '2'),
    ),
  ],
  catalogSeal: _seal(sealDigit),
  generationExecutableSeal: _generationSeal(),
);

AuthoringDraftContentSeal _seal(String digit) =>
    AuthoringDraftContentSeal.fromJson({
      'byte_len': 2048,
      'sha256': List.filled(64, digit).join(),
    });

AuthoringDraftContentSeal _sourceSeal(int bytes, String digit) =>
    AuthoringDraftContentSeal.fromJson({
      'byte_len': bytes,
      'sha256': List.filled(64, digit).join(),
    });

AuthoringDraftContentSeal _generationSeal() =>
    AuthoringDraftContentSeal.fromJson({
      'byte_len': 171698176,
      'sha256': List.filled(64, 'a').join(),
    });

Revision3QuestContextEditPublication _publication(
  Revision3QuestOutlineFixture fixture,
) => Revision3QuestContextEditPublication(
  projectId: revision3QuestOutlineProjectId,
  projectRevision: fixture.projectRevision + 1,
  questId: revision3QuestOutlineQuestId,
  moduleId: revision3QuestOutlineModuleId,
  questRevision: fixture.questRevision + 1,
  moduleRevision: fixture.moduleRevision + 1,
);
