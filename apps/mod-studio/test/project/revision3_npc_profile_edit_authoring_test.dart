import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_profile_edit_authoring.dart';

import '../support/revision3_npc_profile_edit_fixture.dart';

void main() {
  test('load starts exact seed and fresh catalog reads in parallel', () async {
    final fixture = Revision3NpcProfileTestFixture.create();
    final seed = Completer<AuthoringRevision3NpcProfileEditSeed>();
    final catalog = Completer<Revision3NpcCatalog>();
    var seedStarted = false;
    var catalogStarted = false;
    final service = Revision3NpcProfileEditAuthoringService(
      loadSeed:
          ({
            required npcId,
            required expectedNpcRevision,
            required expectedScriptModuleId,
            required expectedScriptModuleRevision,
            required expectedUniqueName,
            required expectedModuleNamespace,
            required expectedParentCharacterDefinition,
            required expectedParentAiAgentConfig,
            required expectedParentSpawnDefinition,
          }) {
            seedStarted = true;
            return seed.future;
          },
      loadCatalog: (gameRoot) {
        catalogStarted = true;
        return catalog.future;
      },
      publishTechnicalPlan: _unexpectedPublish,
    );

    final pending = service.load(
      index: fixture.index,
      npc: fixture.npc,
      gameRoot: r'C:\G1R',
    );
    await Future<void>.delayed(Duration.zero);
    expect(seedStarted, isTrue);
    expect(catalogStarted, isTrue);
    seed.complete(fixture.seed);
    catalog.complete(fixture.catalog());

    final checkpoint = await pending;
    expect(checkpoint.seed, same(fixture.seed));
    expect(checkpoint.currentArchetype.catalogId, revision3NpcProfileAsghanId);
  });

  test('alias ID plus name edit keeps Module regeneration disabled', () async {
    final fixture = Revision3NpcProfileTestFixture.create();
    final catalog = fixture.catalog(includeAlias: true);
    Revision3NpcProfileEditTechnicalPlan? publishedPlan;
    final service = _service(
      fixture,
      catalogs: <Revision3NpcCatalog>[catalog, catalog],
      publish: ({required gameRoot, required plan}) async {
        publishedPlan = plan;
        return _publication(plan);
      },
    );
    final checkpoint = await service.load(
      index: fixture.index,
      npc: fixture.npc,
      gameRoot: r'C:\G1R',
    );

    final publication = await service.publish(
      checkpoint: checkpoint,
      gameRoot: r'C:\G1R',
      displayName: 'Alias Renamed Guard',
      archetype: catalog.choice(revision3NpcProfileAliasId)!,
    );

    expect(publication.parentCatalogId, revision3NpcProfileAliasId);
    expect(publishedPlan!.nameChanged, isTrue);
    expect(publishedPlan!.archetypeChanged, isFalse);
    expect(publishedPlan!.moduleRegenerated, isFalse);
    expect(
      publishedPlan!.expectedCurrentParentTriple.sameBinding(
        publishedPlan!.expectedParentTriple,
      ),
      isTrue,
    );
  });

  test(
    'alias-only selection is a no-op and never refreshes or publishes',
    () async {
      final fixture = Revision3NpcProfileTestFixture.create();
      final catalog = fixture.catalog(includeAlias: true);
      var catalogReads = 0;
      var publications = 0;
      final service = Revision3NpcProfileEditAuthoringService(
        loadSeed: _seedLoader(fixture),
        loadCatalog: (_) async {
          catalogReads++;
          return catalog;
        },
        publishTechnicalPlan: ({required gameRoot, required plan}) async {
          publications++;
          return _publication(plan);
        },
      );
      final checkpoint = await service.load(
        index: fixture.index,
        npc: fixture.npc,
        gameRoot: r'C:\G1R',
      );

      await expectLater(
        service.publish(
          checkpoint: checkpoint,
          gameRoot: r'C:\G1R',
          displayName: fixture.seed.displayName,
          archetype: catalog.choice(revision3NpcProfileAliasId)!,
        ),
        throwsFormatException,
      );
      expect(catalogReads, 1);
      expect(publications, 0);
    },
  );

  test('archetype edit retains exact old/new triple evidence', () async {
    final fixture = Revision3NpcProfileTestFixture.create();
    final catalog = fixture.catalog();
    Revision3NpcProfileEditTechnicalPlan? publishedPlan;
    final service = _service(
      fixture,
      catalogs: <Revision3NpcCatalog>[catalog, catalog],
      publish: ({required gameRoot, required plan}) async {
        publishedPlan = plan;
        return _publication(plan);
      },
    );
    final checkpoint = await service.load(
      index: fixture.index,
      npc: fixture.npc,
      gameRoot: r'C:\G1R',
    );

    await service.publish(
      checkpoint: checkpoint,
      gameRoot: r'C:\G1R',
      displayName: fixture.seed.displayName,
      archetype: catalog.choice(revision3NpcProfileViperId)!,
    );

    expect(publishedPlan!.nameChanged, isFalse);
    expect(publishedPlan!.archetypeChanged, isTrue);
    expect(publishedPlan!.moduleRegenerated, isTrue);
    expect(
      publishedPlan!.expectedCurrentParentTriple.sameBinding(
        publishedPlan!.expectedParentTriple,
      ),
      isFalse,
    );
  });

  test('fresh catalog seal drift returns the replacement catalog', () async {
    final fixture = Revision3NpcProfileTestFixture.create();
    final initial = fixture.catalog();
    final fresh = fixture.catalog(storySealDigit: '3');
    final service = _service(
      fixture,
      catalogs: <Revision3NpcCatalog>[initial, fresh],
      publish: _unexpectedPublish,
    );
    final checkpoint = await service.load(
      index: fixture.index,
      npc: fixture.npc,
      gameRoot: r'C:\G1R',
    );

    await expectLater(
      service.publish(
        checkpoint: checkpoint,
        gameRoot: r'C:\G1R',
        displayName: 'Renamed Guard',
        archetype: initial.choice(revision3NpcProfileAsghanId)!,
      ),
      throwsA(
        isA<Revision3NpcProfileCatalogDriftException>().having(
          (error) => error.freshCatalog,
          'freshCatalog',
          same(fresh),
        ),
      ),
    );
  });
}

Revision3NpcProfileEditAuthoringService _service(
  Revision3NpcProfileTestFixture fixture, {
  required List<Revision3NpcCatalog> catalogs,
  required Revision3NpcProfileEditTechnicalPublisher publish,
}) {
  var catalogIndex = 0;
  return Revision3NpcProfileEditAuthoringService(
    loadSeed: _seedLoader(fixture),
    loadCatalog: (_) async => catalogs[catalogIndex++],
    publishTechnicalPlan: publish,
  );
}

Revision3NpcProfileEditSeedLoader _seedLoader(
  Revision3NpcProfileTestFixture fixture,
) =>
    ({
      required npcId,
      required expectedNpcRevision,
      required expectedScriptModuleId,
      required expectedScriptModuleRevision,
      required expectedUniqueName,
      required expectedModuleNamespace,
      required expectedParentCharacterDefinition,
      required expectedParentAiAgentConfig,
      required expectedParentSpawnDefinition,
    }) async => fixture.seed;

Future<Revision3NpcProfileEditPublication> _unexpectedPublish({
  required String gameRoot,
  required Revision3NpcProfileEditTechnicalPlan plan,
}) => throw StateError('publication was not expected');

Revision3NpcProfileEditPublication _publication(
  Revision3NpcProfileEditTechnicalPlan plan,
) => Revision3NpcProfileEditPublication(
  projectId: plan.projectId,
  projectRevision: plan.projectRevision + 1,
  npcId: plan.npcId,
  npcRevision: plan.expectedNpcRevision + 1,
  scriptModuleId: plan.scriptModuleId,
  scriptModuleRevision:
      plan.expectedScriptModuleRevision + (plan.moduleRegenerated ? 1 : 0),
  displayName: plan.displayName,
  previousParentCatalogId: plan.expectedParentCatalogId,
  parentCatalogId: plan.parentCatalogId,
  nameChanged: plan.nameChanged,
  archetypeChanged: plan.archetypeChanged,
  moduleRegenerated: plan.moduleRegenerated,
);
