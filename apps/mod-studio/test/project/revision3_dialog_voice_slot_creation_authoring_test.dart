import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_voice_slot_creation_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test('plan binds an exact no-slot line and derives a hidden slot ID', () {
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingDeSlot: false),
    );

    final plan = Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint(
      catalog: catalog,
      lineId: revision3VoiceContentLineId,
      locale: 'de',
    );

    expect(plan.expectedLineRevision, 2);
    expect(plan.localizationId, revision3VoiceContentLocalizationId);
    expect(plan.expectedLocalizationRevision, 0);
    expect(plan.locId, 'GRD_263_ASGHAN_OPEN_INFO_06_02');
    expect(plan.locale, 'de');
    expect(plan.slotId, matches(RegExp(r'^[0-9a-f]{32}$')));
    expect(
      plan.slotId,
      isNot(
        anyOf(revision3VoiceContentLineId, revision3VoiceContentLocalizationId),
      ),
    );
  });

  test('plan is deterministic and probes an occupied candidate', () {
    final first = Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint(
      catalog: Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(existingDeSlot: false),
      ),
      lineId: revision3VoiceContentLineId,
      locale: 'de',
    );
    final repeated =
        Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint(
          catalog: Revision3VoiceCatalog.fromContentIndex(
            revision3VoiceContentIndexFixture(existingDeSlot: false),
          ),
          lineId: revision3VoiceContentLineId,
          locale: 'de',
        );
    final collided =
        Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint(
          catalog: Revision3VoiceCatalog.fromContentIndex(
            revision3VoiceContentIndexFixture(
              existingDeSlot: false,
              extraEntityIds: <String>[first.slotId],
            ),
          ),
          lineId: revision3VoiceContentLineId,
          locale: 'de',
        );

    expect(repeated.slotId, first.slotId);
    expect(collided.slotId, isNot(first.slotId));
  });

  test('plan rejects an existing slot or invalid locale', () {
    final existing = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingDeSlot: true),
    );
    final absent = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingDeSlot: false),
    );

    expect(
      () => Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint(
        catalog: existing,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
      ),
      throwsA(isA<Revision3DialogVoiceSlotCreationStaleCheckpointException>()),
    );
    expect(
      () => Revision3DialogVoiceSlotCreationTechnicalPlan.forCheckpoint(
        catalog: absent,
        lineId: revision3VoiceContentLineId,
        locale: 'DE',
      ),
      throwsA(isA<Revision3DialogVoiceSlotCreationStaleCheckpointException>()),
    );
  });

  test('service derives only from one fresh identical catalog', () async {
    final index = revision3VoiceContentIndexFixture(existingDeSlot: false);
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(index);
    var loads = 0;
    Revision3DialogVoiceSlotCreationTechnicalPlan? received;
    final service = Revision3DialogVoiceSlotCreationAuthoringService(
      loadContentIndex: () async {
        loads++;
        return index;
      },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            received = plan;
            return _publication(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );

    final result = await service.publish(
      checkpoint: checkpoint,
      lineId: revision3VoiceContentLineId,
      locale: 'de',
    );

    expect(loads, 1);
    expect(received?.expectedLineRevision, 2);
    expect(result.slotRevision, 0);
    expect(
      result.targetResolution,
      Revision3ContentVoiceTargetResolution.unresolved,
    );
  });

  test('service rejects stale checkpoint without publishing', () async {
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingDeSlot: false),
    );
    var publications = 0;
    final service = Revision3DialogVoiceSlotCreationAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(
        revision: checkpoint.projectRevision + 1,
        existingDeSlot: false,
      ),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publications++;
            return _publication(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );

    await expectLater(
      service.publish(
        checkpoint: checkpoint,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
      ),
      throwsA(isA<Revision3DialogVoiceSlotCreationStaleCheckpointException>()),
    );
    expect(publications, 0);
  });

  test('service rejects a mismatched final publication receipt', () async {
    final index = revision3VoiceContentIndexFixture(existingDeSlot: false);
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(index);
    final service = Revision3DialogVoiceSlotCreationAuthoringService(
      loadContentIndex: () async => index,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => _publication(
            projectId: expectedProjectId,
            projectRevision: expectedProjectRevision + 2,
            plan: plan,
          ),
    );

    await expectLater(
      service.publish(
        checkpoint: checkpoint,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
      ),
      throwsA(isA<Revision3DialogVoiceSlotCreationRequiresReopenException>()),
    );
  });
}

Revision3DialogVoiceSlotCreationPublication _publication({
  required String projectId,
  required int projectRevision,
  required Revision3DialogVoiceSlotCreationTechnicalPlan plan,
}) => Revision3DialogVoiceSlotCreationPublication(
  projectId: projectId,
  projectRevision: projectRevision,
  lineId: plan.lineId,
  lineRevision: plan.expectedLineRevision + 1,
  localizationId: plan.localizationId,
  localizationRevision: plan.expectedLocalizationRevision,
  slotId: plan.slotId,
  slotRevision: 0,
  locale: plan.locale,
  locId: plan.locId,
  targetResolution: Revision3ContentVoiceTargetResolution.unresolved,
);
