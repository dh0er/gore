import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_voice_slot_removal_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test('plan binds one exact intact empty unselected slot', () {
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(
        existingSlotGenerated: true,
        existingSlotTargetResolution: 'ambiguous',
      ),
    );

    final plan = Revision3DialogVoiceSlotRemovalTechnicalPlan.forCheckpoint(
      catalog: catalog,
      lineId: revision3VoiceContentLineId,
      locale: 'de',
    );

    expect(plan.expectedLineRevision, 2);
    expect(plan.localizationId, revision3VoiceContentLocalizationId);
    expect(plan.slotId, revision3VoiceContentSlotId);
    expect(plan.expectedSlotRevision, 1);
    expect(
      plan.targetResolution,
      Revision3ContentVoiceTargetResolution.ambiguous,
    );
  });

  test('plan rejects nonempty or selected slot', () {
    for (final index in <Revision3ContentIndex>[
      revision3VoiceContentIndexFixture(
        existingSlotGenerated: true,
        existingSlotCandidateCount: 1,
      ),
      revision3VoiceContentIndexFixture(
        existingSlotGenerated: true,
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      ),
    ]) {
      final catalog = Revision3VoiceCatalog.fromContentIndex(index);
      expect(
        () => Revision3DialogVoiceSlotRemovalTechnicalPlan.forCheckpoint(
          catalog: catalog,
          lineId: revision3VoiceContentLineId,
          locale: 'de',
        ),
        throwsA(isA<Revision3DialogVoiceSlotRemovalStaleCheckpointException>()),
      );
    }
  });

  test('plan rejects a slot outside the managed generated origin contract', () {
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingSlotGenerated: false),
    );

    expect(
      () => Revision3DialogVoiceSlotRemovalTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
      ),
      throwsA(isA<Revision3DialogVoiceSlotRemovalStaleCheckpointException>()),
    );
  });

  test('plan rejects a generated slot with any additional local backlink', () {
    final json = revision3VoiceContentIndexJsonFixture(
      existingSlotGenerated: true,
    );
    final entities = (json['entities']! as List).cast<Map<String, Object?>>();
    final localization = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentLocalizationId,
    );
    (localization['references']! as List).add(<String, Object?>{
      'role': 'origin_owner',
      'qualifier': null,
      'target': <String, Object?>{
        'project_id': revision3VoiceContentProjectId,
        'entity_id': revision3VoiceContentSlotId,
        'expected_kind': 'voice_slot',
      },
      'resolution': 'resolved',
    });
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      Revision3ContentIndex.fromJsonObject(json),
    );

    expect(
      catalog
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!
          .isRemovableGeneratedSlot,
      isFalse,
    );
    expect(
      () => Revision3DialogVoiceSlotRemovalTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
      ),
      throwsA(isA<Revision3DialogVoiceSlotRemovalStaleCheckpointException>()),
    );
  });

  test('service derives only from one fresh identical catalog', () async {
    final index = revision3VoiceContentIndexFixture(
      existingSlotGenerated: true,
      existingSlotTargetResolution: 'resolved',
    );
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(index);
    var loads = 0;
    Revision3DialogVoiceSlotRemovalTechnicalPlan? received;
    final service = Revision3DialogVoiceSlotRemovalAuthoringService(
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
    expect(
      result.removedTargetResolution,
      Revision3ContentVoiceTargetResolution.resolved,
    );
  });

  test('service rejects stale checkpoint without publishing', () async {
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingSlotGenerated: true),
    );
    var publications = 0;
    final service = Revision3DialogVoiceSlotRemovalAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(
        revision: checkpoint.projectRevision + 1,
        existingSlotGenerated: true,
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
      throwsA(isA<Revision3DialogVoiceSlotRemovalStaleCheckpointException>()),
    );
    expect(publications, 0);
  });

  test('service rejects a mismatched final publication receipt', () async {
    final index = revision3VoiceContentIndexFixture(
      existingSlotGenerated: true,
    );
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(index);
    final service = Revision3DialogVoiceSlotRemovalAuthoringService(
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
      throwsA(isA<Revision3DialogVoiceSlotRemovalRequiresReopenException>()),
    );
  });
}

Revision3DialogVoiceSlotRemovalPublication _publication({
  required String projectId,
  required int projectRevision,
  required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
}) => Revision3DialogVoiceSlotRemovalPublication(
  head: _publicationHead(),
  projectId: projectId,
  projectRevision: projectRevision,
  lineId: plan.lineId,
  lineRevision: plan.expectedLineRevision + 1,
  localizationId: plan.localizationId,
  slotId: plan.slotId,
  removedSlotRevision: plan.expectedSlotRevision,
  locale: plan.locale,
  locId: plan.locId,
  removedTargetResolution: plan.targetResolution,
);

AuthoringWorkingHead _publicationHead() =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': 1,
          'sha256': List<String>.filled(64, 'a').join(),
        },
      }),
    );
