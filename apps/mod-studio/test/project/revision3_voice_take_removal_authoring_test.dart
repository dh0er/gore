import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_removal_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test(
    'technical plan binds exact fresh candidate, revisions, and selection',
    () {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(
          existingSlotCandidateCount: 2,
          existingSlotHasSelectedTake: true,
        ),
      );
      final summary = catalog
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!;
      final selected = summary.candidates.first;

      final plan = Revision3VoiceTakeRemovalTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        takeId: selected.id,
      );

      expect(plan.localizationId, revision3VoiceContentLocalizationId);
      expect(plan.expectedSlotRevision, summary.slotRevision);
      expect(plan.expectedTakeRevision, selected.revision);
      expect(plan.expectedSelectedTakeId, selected.id);
      expect(plan.expectsSelectionCleared, isTrue);
      expect(plan.expectedRemainingCandidateCount, 1);
      expect(plan.expectedTakeEntityRemoved, isTrue);
    },
  );

  test(
    'technical plan counts shared candidate slots but not selection twice',
    () {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        Revision3ContentIndex.fromJsonObject(_sharedTakeIndex()),
      );
      final takeId = catalog
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!
          .candidates
          .single
          .id;

      final plan = Revision3VoiceTakeRemovalTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        takeId: takeId,
      );

      expect(catalog.candidateSlotUseCount(takeId), 2);
      expect(plan.expectsSelectionCleared, isTrue);
      expect(plan.expectedTakeEntityRemoved, isFalse);
    },
  );

  test('technical plan rejects a take absent from the exact slot', () {
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      ),
    );
    expect(
      () => Revision3VoiceTakeRemovalTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        takeId: '99999999999999999999999999999999',
      ),
      throwsA(isA<Revision3VoiceTakeRemovalStaleCheckpointException>()),
    );
  });

  test(
    'service reloads exact index once and publishes no-authority plan',
    () async {
      final index = revision3VoiceContentIndexFixture(
        existingSlotCandidateCount: 2,
        existingSlotHasSelectedTake: true,
      );
      final catalog = Revision3VoiceCatalog.fromContentIndex(index);
      final take = catalog
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!
          .candidates
          .first;
      var loads = 0;
      Revision3VoiceTakeRemovalTechnicalPlan? received;
      final service = Revision3VoiceTakeRemovalAuthoringService(
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
                takeEntityRemoved: true,
              );
            },
      );

      final result = await service.publish(
        checkpoint: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        takeId: take.id,
      );

      expect(loads, 1);
      expect(received?.takeId, take.id);
      expect(result.selectionCleared, isTrue);
      expect(result.remainingCandidateCount, 1);
    },
  );

  test('service rejects stale checkpoint without auto retry', () async {
    final initial = revision3VoiceContentIndexFixture(
      existingSlotCandidateCount: 2,
      existingSlotHasSelectedTake: true,
    );
    final catalog = Revision3VoiceCatalog.fromContentIndex(initial);
    final takeId = catalog
        .line(revision3VoiceContentLineId)!
        .slotSummaryForLocale('de')!
        .candidates
        .first
        .id;
    var publishCalls = 0;
    final service = Revision3VoiceTakeRemovalAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(
        revision: initial.projectRevision + 1,
        existingSlotCandidateCount: 2,
        existingSlotHasSelectedTake: true,
      ),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            return _publication(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              plan: plan,
              takeEntityRemoved: true,
            );
          },
    );

    await expectLater(
      service.publish(
        checkpoint: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        takeId: takeId,
      ),
      throwsA(isA<Revision3VoiceTakeRemovalStaleCheckpointException>()),
    );
    expect(publishCalls, 0);
  });

  test(
    'service treats mismatched managed receipt as requires-reopen',
    () async {
      final index = revision3VoiceContentIndexFixture(
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      );
      final catalog = Revision3VoiceCatalog.fromContentIndex(index);
      final takeId = catalog
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!
          .candidates
          .single
          .id;
      final service = Revision3VoiceTakeRemovalAuthoringService(
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
              takeEntityRemoved: true,
            ),
      );

      await expectLater(
        service.publish(
          checkpoint: catalog,
          lineId: revision3VoiceContentLineId,
          locale: 'de',
          takeId: takeId,
        ),
        throwsA(isA<Revision3VoiceTakeRemovalRequiresReopenException>()),
      );
    },
  );

  test(
    'service rejects only-wrong final/shared entity-removal flags',
    () async {
      final cases = <({Revision3ContentIndex index, bool forgedFlag})>[
        (
          index: revision3VoiceContentIndexFixture(
            existingSlotCandidateCount: 1,
            existingSlotHasSelectedTake: true,
          ),
          forgedFlag: false,
        ),
        (
          index: Revision3ContentIndex.fromJsonObject(_sharedTakeIndex()),
          forgedFlag: true,
        ),
      ];
      for (final testCase in cases) {
        final catalog = Revision3VoiceCatalog.fromContentIndex(testCase.index);
        final takeId = catalog
            .line(revision3VoiceContentLineId)!
            .slotSummaryForLocale('de')!
            .candidates
            .first
            .id;
        final service = Revision3VoiceTakeRemovalAuthoringService(
          loadContentIndex: () async => testCase.index,
          publishTechnicalPlan:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) async => _publication(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision + 1,
                plan: plan,
                takeEntityRemoved: testCase.forgedFlag,
              ),
        );

        await expectLater(
          service.publish(
            checkpoint: catalog,
            lineId: revision3VoiceContentLineId,
            locale: 'de',
            takeId: takeId,
          ),
          throwsA(isA<Revision3VoiceTakeRemovalRequiresReopenException>()),
        );
      }
    },
  );
}

Revision3VoiceTakeRemovalPublication _publication({
  required String projectId,
  required int projectRevision,
  required Revision3VoiceTakeRemovalTechnicalPlan plan,
  required bool takeEntityRemoved,
}) => Revision3VoiceTakeRemovalPublication(
  head: _publicationHead(),
  projectId: projectId,
  projectRevision: projectRevision,
  lineId: plan.lineId,
  localizationId: plan.localizationId,
  slotId: plan.slotId,
  slotRevision: plan.expectedSlotRevision + 1,
  locale: plan.locale,
  locId: plan.locId,
  takeId: plan.takeId,
  takeRevision: plan.expectedTakeRevision,
  previousSelectedTakeId: plan.expectedSelectedTakeId,
  selectionCleared: plan.expectsSelectionCleared,
  takeEntityRemoved: takeEntityRemoved,
  remainingCandidateCount: plan.expectedRemainingCandidateCount,
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

Map<String, Object?> _sharedTakeIndex() {
  final json = revision3VoiceContentIndexJsonFixture(
    existingSlotCandidateCount: 1,
    existingSlotHasSelectedTake: true,
    duplicateLine: true,
  );
  final entities = (json['entities']! as List).cast<Map<String, Object?>>();
  final duplicate = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentDuplicateLineId,
  );
  final duplicateSummary = (duplicate['summary']! as Map)
      .cast<String, Object?>();
  final duplicateData = (duplicateSummary['data']! as Map)
      .cast<String, Object?>();
  duplicateData['voice_slot_locales'] = <Object?>['de'];
  duplicateSummary['data'] = duplicateData;
  duplicate['summary'] = duplicateSummary;
  const secondSlotId = '77777777777777777777777777777777';
  (duplicate['references']! as List).add(<String, Object?>{
    'role': 'dialog_voice_slot',
    'qualifier': 'de',
    'target': <String, Object?>{
      'project_id': revision3VoiceContentProjectId,
      'entity_id': secondSlotId,
      'expected_kind': 'voice_slot',
    },
    'resolution': 'resolved',
  });
  entities.add(<String, Object?>{
    'id': secondSlotId,
    'kind': 'voice_slot',
    'display_name': 'Second German slot',
    'revision': 3,
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': 'second-slot',
    },
    'summary': <String, Object?>{
      'kind': 'voice_slot',
      'data': <String, Object?>{
        'locale': 'de',
        'target_resolution': 'unresolved',
        'candidate_count': 1,
        'has_selected_take': false,
      },
    },
    'references': <Object?>[
      <String, Object?>{
        'role': 'voice_candidate',
        'qualifier': null,
        'target': <String, Object?>{
          'project_id': revision3VoiceContentProjectId,
          'entity_id': '55000000000000000000000000000000',
          'expected_kind': 'voice_take',
        },
        'resolution': 'resolved',
      },
    ],
    'asset_references': <Object?>[],
  });
  entities.sort(
    (left, right) => (left['id']! as String).compareTo(right['id']! as String),
  );
  final counts = (json['entity_counts']! as Map).cast<String, Object?>();
  counts['voice_slot'] = 2;
  json['entity_counts'] = counts;
  json['entities'] = entities;
  return json;
}
