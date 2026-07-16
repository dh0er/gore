import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_status_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test('technical plan binds one exact retained take status edit', () {
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingSlotCandidateCount: 2),
    );
    final line = catalog.line(revision3VoiceContentLineId)!;
    final slot = line.slotSummaryForLocale('de')!;
    final take = slot.candidates.first;

    final plan = Revision3VoiceTakeStatusTechnicalPlan.forCheckpoint(
      catalog: catalog,
      lineId: line.lineId,
      locale: 'de',
      takeId: take.id,
      desiredStatus: AuthoringRevision3VoiceTakeStatus.reviewed,
    );

    expect(plan.lineId, revision3VoiceContentLineId);
    expect(plan.localizationId, revision3VoiceContentLocalizationId);
    expect(plan.locId, 'GRD_263_ASGHAN_OPEN_INFO_06_02');
    expect(plan.locale, 'de');
    expect(plan.slotId, revision3VoiceContentSlotId);
    expect(plan.expectedSlotRevision, 1);
    expect(plan.takeId, take.id);
    expect(plan.expectedTakeRevision, 0);
    expect(plan.expectedStatus, AuthoringRevision3VoiceTakeStatus.recorded);
    expect(plan.desiredStatus, AuthoringRevision3VoiceTakeStatus.reviewed);
  });

  test('technical plan rejects no-op and selected take demotion', () {
    final recorded = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingSlotCandidateCount: 1),
    );
    final recordedTake = recorded
        .line(revision3VoiceContentLineId)!
        .slotSummaryForLocale('de')!
        .candidates
        .single;

    expect(
      () => Revision3VoiceTakeStatusTechnicalPlan.forCheckpoint(
        catalog: recorded,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        takeId: recordedTake.id,
        desiredStatus: AuthoringRevision3VoiceTakeStatus.recorded,
      ),
      throwsFormatException,
    );

    final selected = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      ),
    );
    final selectedTake = selected
        .line(revision3VoiceContentLineId)!
        .slotSummaryForLocale('de')!
        .candidates
        .single;
    expect(
      () => Revision3VoiceTakeStatusTechnicalPlan.forCheckpoint(
        catalog: selected,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        takeId: selectedTake.id,
        desiredStatus: AuthoringRevision3VoiceTakeStatus.reviewed,
      ),
      throwsA(isA<Revision3VoiceTakeStatusSelectedTakeException>()),
    );
  });

  test('service refreshes exact catalog before publishing status', () async {
    final index = revision3VoiceContentIndexFixture(
      existingSlotCandidateCount: 1,
    );
    final catalog = Revision3VoiceCatalog.fromContentIndex(index);
    final take = catalog
        .line(revision3VoiceContentLineId)!
        .slotSummaryForLocale('de')!
        .candidates
        .single;
    Revision3VoiceTakeStatusTechnicalPlan? received;
    final service = Revision3VoiceTakeStatusAuthoringService(
      loadContentIndex: () async => index,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            received = plan;
            return _matchingPublication(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );

    final publication = await service.publish(
      checkpoint: catalog,
      lineId: revision3VoiceContentLineId,
      locale: 'de',
      takeId: take.id,
      desiredStatus: AuthoringRevision3VoiceTakeStatus.approved,
    );

    expect(received, isNotNull);
    expect(publication.projectRevision, index.projectRevision + 1);
    expect(publication.takeId, take.id);
    expect(publication.takeRevision, take.revision + 1);
    expect(
      publication.previousStatus,
      AuthoringRevision3VoiceTakeStatus.recorded,
    );
    expect(publication.status, AuthoringRevision3VoiceTakeStatus.approved);
  });

  test('service rejects stale catalog without publishing', () async {
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(existingSlotCandidateCount: 1),
    );
    var publishCalls = 0;
    final service = Revision3VoiceTakeStatusAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(
        revision: checkpoint.projectRevision + 1,
        existingSlotCandidateCount: 1,
      ),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls += 1;
            return _matchingPublication(
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
        takeId: checkpoint
            .line(revision3VoiceContentLineId)!
            .slotSummaryForLocale('de')!
            .candidates
            .single
            .id,
        desiredStatus: AuthoringRevision3VoiceTakeStatus.approved,
      ),
      throwsA(isA<Revision3VoiceTakeStatusStaleCheckpointException>()),
    );
    expect(publishCalls, 0);
  });

  test('service requires reopen for a mismatched publication', () async {
    final index = revision3VoiceContentIndexFixture(
      existingSlotCandidateCount: 1,
    );
    final checkpoint = Revision3VoiceCatalog.fromContentIndex(index);
    final take = checkpoint
        .line(revision3VoiceContentLineId)!
        .slotSummaryForLocale('de')!
        .candidates
        .single;
    final service = Revision3VoiceTakeStatusAuthoringService(
      loadContentIndex: () async => index,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => _matchingPublication(
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
        takeId: take.id,
        desiredStatus: AuthoringRevision3VoiceTakeStatus.approved,
      ),
      throwsA(isA<Revision3VoiceTakeStatusRequiresReopenException>()),
    );
  });

  test('service maps content uncertainty to requires reopen', () async {
    final service = Revision3VoiceTakeStatusAuthoringService(
      loadContentIndex: () async =>
          throw const Revision3ContentRequiresReopenException(),
      publishTechnicalPlan: _unexpectedPublish,
    );

    await expectLater(
      service.loadCatalog(),
      throwsA(isA<Revision3VoiceTakeStatusRequiresReopenException>()),
    );
  });
}

Revision3VoiceTakeStatusPublication _matchingPublication({
  required String projectId,
  required int projectRevision,
  required Revision3VoiceTakeStatusTechnicalPlan plan,
}) => Revision3VoiceTakeStatusPublication(
  projectId: projectId,
  projectRevision: projectRevision,
  lineId: plan.lineId,
  localizationId: plan.localizationId,
  slotId: plan.slotId,
  slotRevision: plan.expectedSlotRevision,
  locale: plan.locale,
  locId: plan.locId,
  takeId: plan.takeId,
  takeRevision: plan.expectedTakeRevision + 1,
  previousStatus: plan.expectedStatus,
  status: plan.desiredStatus,
);

Future<Revision3VoiceTakeStatusPublication> _unexpectedPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeStatusTechnicalPlan plan,
}) => throw StateError('publisher must not be called');
