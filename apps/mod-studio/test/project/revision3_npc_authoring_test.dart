import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';

const _projectId = '11111111111111111111111111111111';

void main() {
  test('derives deterministic hidden identities from the exact checkpoint', () {
    final input = Revision3NpcDraftAuthoringInput(
      parentCatalogId: 'g1r:npc:om_grd_asghan_263',
      displayName: '  North Gate Guard  ',
    );

    final first = Revision3NpcDraftTechnicalPlan.forCheckpoint(
      projectId: _projectId,
      projectRevision: 7,
      input: input,
    );
    final repeated = Revision3NpcDraftTechnicalPlan.forCheckpoint(
      projectId: _projectId,
      projectRevision: 7,
      input: input,
    );
    final nextRevision = Revision3NpcDraftTechnicalPlan.forCheckpoint(
      projectId: _projectId,
      projectRevision: 8,
      input: input,
    );

    expect(input.displayName, 'North Gate Guard');
    expect(first.npcId, repeated.npcId);
    expect(first.scriptModuleId, repeated.scriptModuleId);
    expect(first.npcId, isNot(first.scriptModuleId));
    expect(first.npcId, isNot(nextRevision.npcId));
    expect(first.scriptModuleId, isNot(nextRevision.scriptModuleId));
    expect(
      first.intent.uniqueName,
      matches(r'^GORE_NORTH_GATE_GUARD_[0-9A-F]{10}$'),
    );
    expect(
      first.intent.moduleNamespace,
      matches(r'^GoreMods\.Npcs\.NorthGateGuard[0-9A-F]{10}$'),
    );
    expect(first.intent.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
    expect(first.displayName, 'North Gate Guard');
  });

  test('accepts friendly Unicode but keeps generated identifiers portable', () {
    final input = Revision3NpcDraftAuthoringInput(
      parentCatalogId: 'g1r:npc:oc_grd_viper_253',
      displayName: 'Wächterin am Tor',
    );
    final plan = Revision3NpcDraftTechnicalPlan.forCheckpoint(
      projectId: _projectId,
      projectRevision: 0,
      input: input,
    );

    expect(input.displayName, 'Wächterin am Tor');
    expect(plan.intent.uniqueName, matches(r'^[A-Z][A-Z0-9_]*$'));
    expect(plan.intent.moduleNamespace, matches(r'^[A-Za-z0-9.]+$'));
  });

  test('rejects invalid friendly input and exact checkpoint identities', () {
    expect(
      () => Revision3NpcDraftAuthoringInput(
        parentCatalogId: 'g1r:npc:oc_grd_viper_253',
        displayName: 'Guard\nInjected',
      ),
      throwsFormatException,
    );
    expect(
      () => Revision3NpcDraftAuthoringInput(
        parentCatalogId: 'g1r:npc:oc_grd_viper_253',
        displayName: 'Guard\u0085Injected',
      ),
      throwsFormatException,
    );
    expect(
      () => Revision3NpcDraftAuthoringInput(
        parentCatalogId: ' g1r:npc:oc_grd_viper_253',
        displayName: 'Guard',
      ),
      throwsFormatException,
    );
    expect(
      () => Revision3NpcDraftTechnicalPlan.forCheckpoint(
        projectId: '00000000000000000000000000000000',
        projectRevision: 0,
        input: _input(),
      ),
      throwsFormatException,
    );
  });

  test('catalog projection requires nonempty unique qualified choices', () {
    final choice = Revision3NpcCatalogChoice(
      catalogId: 'g1r:npc:om_grd_asghan_263',
      displayName: 'Asghan',
    );
    final catalog = Revision3NpcCatalog(choices: [choice]);

    expect(catalog.contains('g1r:npc:om_grd_asghan_263'), isTrue);
    expect(catalog.choice('g1r:npc:om_grd_asghan_263')?.displayName, 'Asghan');
    expect(() => Revision3NpcCatalog(choices: const []), throwsFormatException);
    expect(
      () => Revision3NpcCatalog(choices: [choice, choice]),
      throwsFormatException,
    );
  });
}

Revision3NpcDraftAuthoringInput _input() => Revision3NpcDraftAuthoringInput(
  parentCatalogId: 'g1r:npc:om_grd_asghan_263',
  displayName: 'Gate Guard',
);
